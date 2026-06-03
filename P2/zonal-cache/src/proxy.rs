use crate::auth::{AuthError, AuthManager};
use crate::cache::DiskCache;
use crate::route_resolver::EffectiveConfig;
use reqwest::Client;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("error de autenticación: {0}")]
    Auth(#[from] AuthError),
    #[error("error de caché: {0}")]
    Cache(#[from] crate::cache::CacheError),
    #[error("error de red: {0}")]
    Network(String),
    #[error("URL inválida: {0}")]
    InvalidUrl(String),
}

/// Respuesta del proxy. Body es Vec<u8> para soportar binarios.
#[derive(Debug)]
pub struct ProxyResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub from_cache: bool,
    /// Edad de la entrada en cache (segundos). 0 si MISS.
    pub age_secs: u64,
    /// Política aplicada (para header X-Cache-Policy).
    pub policy: String,
    /// TTL configurado para esta entrada (None si no aplica).
    pub ttl_seconds: Option<u64>,
    /// Pattern que matcheó (None si no había regla).
    pub matched_pattern: Option<String>,
}

/// Decide si una respuesta es cacheable según la regla efectiva y la respuesta del origen.
///
/// La regla manda primero (`cfg.cacheable`); luego se filtra por status code.
/// Estados cacheables siguen el espíritu de RFC 7234 sin entrar a su complejidad completa.
pub fn is_cacheable_response(cfg: &EffectiveConfig, status: u16) -> bool {
    if !cfg.cacheable {
        return false;
    }
    // 200, 203, 204, 206, 300, 301, 410: cacheables por defecto en HTTP.
    matches!(status, 200 | 203 | 204 | 206 | 300 | 301 | 410)
}

/// Construye la cache key a partir de la URL del origen.
/// Se normaliza para evitar duplicados por mayúsculas o trailing slash.
pub fn build_cache_key(url: &str) -> String {
    url.to_lowercase().trim_end_matches('/').to_string()
}

/// Valida la URL parseándola con `url::Url`. Esto rechaza esquemas exóticos,
/// URLs sin host, y otras malformaciones (mejor que el viejo `starts_with`).
pub fn validate_url(url: &str) -> Result<(), ProxyError> {
    let parsed = Url::parse(url).map_err(|e| ProxyError::InvalidUrl(e.to_string()))?;
    match parsed.scheme() {
        "http" | "https" => {
            if parsed.host_str().is_none() {
                return Err(ProxyError::InvalidUrl("URL sin host".into()));
            }
            Ok(())
        }
        other => Err(ProxyError::InvalidUrl(format!(
            "esquema no soportado: {}",
            other
        ))),
    }
}

/// Hace fetch al origen. Devuelve body como Vec<u8> para preservar binarios.
pub async fn fetch_from_origin(
    client: &Client,
    url: &str,
) -> Result<(u16, String, Vec<u8>), ProxyError> {
    let response = client
        .get(url)
        .header("User-Agent", "ZonalCache/0.1")
        .send()
        .await
        .map_err(|e| ProxyError::Network(e.to_string()))?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let body = response
        .bytes()
        .await
        .map_err(|e| ProxyError::Network(e.to_string()))?
        .to_vec();

    Ok((status, content_type, body))
}

/// Procesa un request completo del proxy:
///   1) valida autenticación según el `auth_mode` de la regla,
///   2) consulta el caché del dominio,
///   3) si MISS, hace fetch al origen,
///   4) si la respuesta es cacheable, la persiste.
///
/// Importante: NO muta el cache global. Solo el bucket del dominio se actualiza,
/// y la cuota/política del bucket se ajusta vía `set_domain_config` ANTES de llamar.
pub async fn handle_proxy_request(
    cfg: &EffectiveConfig,
    api_key: Option<&str>,
    session_token: Option<&str>,
    auth: &AuthManager,
    cache: &mut DiskCache,
    client: &Client,
) -> Result<ProxyResponse, ProxyError> {
    // 1. Validar la URL del origen (vino del RouteResolver).
    validate_url(&cfg.origin_url)?;

    // 2. Validar autenticación según el modo exigido por la regla.
    auth.validate_for_mode(&cfg.auth_mode, api_key, session_token)
        .map_err(ProxyError::Auth)?;

    // 3. Buscar en caché.
    let key = build_cache_key(&cfg.origin_url);
    if cache.contains(&cfg.domain, &key) {
        match cache.get(&cfg.domain, &key).await {
            Ok((body, content_type, status, age)) => {
                return Ok(ProxyResponse {
                    status,
                    content_type,
                    body,
                    from_cache: true,
                    age_secs: age,
                    policy: cfg.policy.as_str().to_string(),
                    ttl_seconds: cfg.ttl_seconds,
                    matched_pattern: cfg.matched_pattern.clone(),
                });
            }
            Err(crate::cache::CacheError::Expired(_)) => {
                // Expiró: caer al fetch del origen.
            }
            Err(e) => return Err(ProxyError::Cache(e)),
        }
    }

    // 4. MISS o expirado: fetch al origen.
    let (status, content_type, body) = fetch_from_origin(client, &cfg.origin_url).await?;

    // 5. Guardar en caché si la respuesta es cacheable según la regla.
    if is_cacheable_response(cfg, status) {
        // Si la entrada es demasiado grande para el bucket, lo logueamos pero
        // igual respondemos al cliente con el contenido del origen.
        if let Err(e) = cache
            .put(
                &cfg.domain,
                &key,
                &body,
                &content_type,
                status,
                cfg.ttl_seconds,
            )
            .await
        {
            tracing::warn!("No se pudo cachear {} en {}: {}", key, cfg.domain, e);
        }
    }

    Ok(ProxyResponse {
        status,
        content_type,
        body,
        from_cache: false,
        age_secs: 0,
        policy: cfg.policy.as_str().to_string(),
        ttl_seconds: cfg.ttl_seconds,
        matched_pattern: cfg.matched_pattern.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::Policy;
    use crate::auth::AuthManager;
    use crate::cache::DiskCache;
    use crate::firestore::AuthMode;
    use std::env;
    use uuid::Uuid;

    fn make_auth() -> AuthManager {
        let mut auth = AuthManager::new(3600);
        auth.register_api_key("valid-key", "user1");
        auth.register_user("user1", "pass123");
        auth
    }

    fn temp_cache() -> DiskCache {
        let dir = env::temp_dir().join(format!("zc_proxy_test_{}", Uuid::new_v4()));
        DiskCache::new(dir.to_str().unwrap(), 1024 * 1024, Policy::Lru)
    }

    fn cfg_for(
        origin: &str,
        auth_mode: AuthMode,
        cacheable: bool,
        ttl: Option<u64>,
    ) -> EffectiveConfig {
        EffectiveConfig {
            domain: "ex.com".into(),
            origin_url: origin.into(),
            policy: Policy::Lru,
            max_size_bytes: 1024 * 1024,
            ttl_seconds: ttl,
            cacheable,
            auth_mode,
            matched_pattern: Some("/*".into()),
        }
    }

    #[test]
    fn test_is_cacheable_200_with_rule_allowing() {
        let c = cfg_for("http://x/y", AuthMode::None, true, None);
        assert!(is_cacheable_response(&c, 200));
        assert!(is_cacheable_response(&c, 301));
        assert!(!is_cacheable_response(&c, 500));
        assert!(!is_cacheable_response(&c, 404));
    }

    #[test]
    fn test_is_cacheable_blocked_by_rule() {
        let c = cfg_for("http://x/y", AuthMode::None, false, None);
        assert!(!is_cacheable_response(&c, 200));
    }

    #[test]
    fn test_build_cache_key_normalization() {
        assert_eq!(build_cache_key("HTTP://Ex.com/page/"), "http://ex.com/page");
        assert_eq!(build_cache_key("http://x/"), "http://x");
    }

    #[test]
    fn test_validate_url_http_https_ok() {
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("https://example.com/path?q=1").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_other_schemes() {
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_validate_url_rejects_garbage() {
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("").is_err());
        assert!(validate_url("http://").is_err());
    }

    #[tokio::test]
    async fn test_handle_request_auth_none_no_origin_call_when_url_invalid() {
        // Si la URL es inválida, falla antes incluso de tocar auth.
        let auth = make_auth();
        let mut cache = temp_cache();
        let client = Client::new();
        let cfg = cfg_for("ftp://x.com/y", AuthMode::None, true, None);
        let res = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client).await;
        assert!(matches!(res, Err(ProxyError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn test_handle_request_auth_api_key_missing_rejects() {
        let auth = make_auth();
        let mut cache = temp_cache();
        let client = Client::new();
        let cfg = cfg_for("http://localhost:9/x", AuthMode::ApiKey, true, None);
        let res = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client).await;
        assert!(matches!(res, Err(ProxyError::Auth(_))));
    }

    #[tokio::test]
    async fn test_handle_request_auth_api_key_invalid_rejects() {
        let auth = make_auth();
        let mut cache = temp_cache();
        let client = Client::new();
        let cfg = cfg_for("http://localhost:9/x", AuthMode::ApiKey, true, None);
        let res =
            handle_proxy_request(&cfg, Some("wrong-key"), None, &auth, &mut cache, &client).await;
        assert!(matches!(res, Err(ProxyError::Auth(_))));
    }
}
