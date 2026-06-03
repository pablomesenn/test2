use crate::algorithms::Policy;
use crate::firestore::{AuthMode, RouteConfigDoc};
use globset::{Glob, GlobMatcher};

/// Configuración efectiva calculada para un request específico.
/// Es **inmutable** y se pasa por valor al handler, evitando mutar
/// el estado global del cache (bug que tenía la versión anterior).
#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    /// Dominio al que pertenece la petición (host del request o del upstream).
    pub domain: String,
    /// URL completa del origen al que se debe hacer fetch.
    pub origin_url: String,
    /// Política de reemplazo a aplicar en el bucket del dominio.
    pub policy: Policy,
    /// Cuota máxima en bytes para el bucket del dominio.
    pub max_size_bytes: u64,
    /// TTL en segundos para esta entrada específica.
    /// `None` significa "sin expiración".
    pub ttl_seconds: Option<u64>,
    /// Si esta extensión/recurso es cacheable según la regla aplicada.
    pub cacheable: bool,
    /// Modo de autenticación exigido para esta ruta.
    pub auth_mode: AuthMode,
    /// Pattern que matcheó (útil para logs/diagnóstico).
    pub matched_pattern: Option<String>,
}

impl EffectiveConfig {
    /// Configuración por defecto cuando no hay regla en Firestore.
    /// Útil para fallback local o pruebas sin Firebase.
    pub fn default_for(
        domain: &str,
        origin_url: &str,
        policy: Policy,
        max_size_bytes: u64,
    ) -> Self {
        Self {
            domain: domain.to_string(),
            origin_url: origin_url.to_string(),
            policy,
            max_size_bytes,
            ttl_seconds: None,
            cacheable: true,
            auth_mode: AuthMode::None,
            matched_pattern: None,
        }
    }
}

/// Una regla pre-compilada para matching eficiente.
pub struct CompiledRoute {
    pub doc: RouteConfigDoc,
    pub matcher: GlobMatcher,
}

impl CompiledRoute {
    pub fn compile(doc: RouteConfigDoc) -> Result<Self, String> {
        let glob = Glob::new(&doc.pattern)
            .map_err(|e| format!("pattern inválido '{}': {}", doc.pattern, e))?;
        Ok(Self {
            matcher: glob.compile_matcher(),
            doc,
        })
    }

    pub fn matches(&self, path: &str) -> bool {
        self.matcher.is_match(path)
    }
}

/// Resuelve la configuración efectiva para una petición específica.
///
/// Estrategia:
/// 1. Buscar entre las rutas del dominio cuál hace match con el path.
/// 2. Aplicar la primera regla que matchee (orden por precedencia: pattern más largo gana).
/// 3. Aplicar overrides por extensión (fileRules).
/// 4. Si nada matchea, retornar default.
pub struct RouteResolver;

impl RouteResolver {
    /// Calcula la configuración efectiva para un request.
    ///
    /// `routes`: rutas configuradas para el dominio (típicamente cargadas de Firestore).
    /// `path`: path relativo de la petición (ej: `/assets/main.css`).
    /// `domain`: host del request.
    /// `fallback`: configuración a usar si ninguna regla matchea.
    pub fn resolve(
        routes: &[CompiledRoute],
        domain: &str,
        path: &str,
        fallback: EffectiveConfig,
    ) -> EffectiveConfig {
        // Filtrar rutas que matchean y ordenar por especificidad (pattern más largo gana).
        // Esto evita que `/*` gane sobre `/assets/*`.
        let mut matching: Vec<&CompiledRoute> = routes
            .iter()
            .filter(|r| r.doc.enabled && r.doc.domain == domain && r.matches(path))
            .collect();
        matching.sort_by_key(|r| std::cmp::Reverse(r.doc.pattern.len()));

        let best = match matching.first() {
            Some(r) => r,
            None => return fallback,
        };

        let doc = &best.doc;
        let policy =
            Policy::try_from_str(&doc.cache_policy).unwrap_or_else(|| fallback.policy.clone());

        // Determinar TTL y cacheable según extensión (si aplica).
        let extension = Self::extract_extension(path);
        let (ttl, cacheable) = match extension.as_deref() {
            Some(ext) => {
                let rule = doc
                    .file_rules
                    .iter()
                    .find(|r| r.extension.eq_ignore_ascii_case(ext));
                match rule {
                    Some(r) => (Some(r.ttl_seconds), r.cacheable),
                    None => (doc.default_ttl_seconds, true),
                }
            }
            None => (doc.default_ttl_seconds, true),
        };

        // Construir URL al origen: base + path tal cual.
        // Esto soporta los 4 casos del spec: REST API, Apache, sitio HTTP externo, sitio HTTPS externo.
        let origin_url = Self::join_origin(&doc.origin_base_url, path);

        EffectiveConfig {
            domain: domain.to_string(),
            origin_url,
            policy,
            max_size_bytes: doc.cache_max_size_bytes,
            ttl_seconds: ttl,
            cacheable,
            auth_mode: doc.auth_mode.clone(),
            matched_pattern: Some(doc.pattern.clone()),
        }
    }

    /// Extrae la extensión (sin punto) del último segmento del path.
    /// Devuelve None si no hay extensión.
    pub fn extract_extension(path: &str) -> Option<String> {
        let last_segment = path.rsplit('/').next()?;
        let dot_idx = last_segment.rfind('.')?;
        // Evitar "archivos ocultos" como ".gitignore" → no es una extensión real.
        if dot_idx == 0 {
            return None;
        }
        Some(last_segment[dot_idx + 1..].to_ascii_lowercase())
    }

    /// Une `base` y `path` evitando doble slash o slash faltante.
    fn join_origin(base: &str, path: &str) -> String {
        let base_trimmed = base.trim_end_matches('/');
        if path.starts_with('/') {
            format!("{}{}", base_trimmed, path)
        } else {
            format!("{}/{}", base_trimmed, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firestore::FileRule;

    fn route(domain: &str, pattern: &str, origin: &str) -> RouteConfigDoc {
        RouteConfigDoc {
            domain: domain.to_string(),
            pattern: pattern.to_string(),
            origin_base_url: origin.to_string(),
            cache_max_size_bytes: 1024 * 1024,
            cache_policy: "LRU".to_string(),
            default_ttl_seconds: Some(300),
            file_rules: vec![],
            auth_mode: AuthMode::None,
            enabled: true,
        }
    }

    fn fallback() -> EffectiveConfig {
        EffectiveConfig::default_for("fallback.com", "http://fallback", Policy::Lru, 1024)
    }

    fn compile(doc: RouteConfigDoc) -> CompiledRoute {
        CompiledRoute::compile(doc).expect("pattern válido")
    }

    #[test]
    fn test_extract_extension_basic() {
        assert_eq!(
            RouteResolver::extract_extension("/a/b/file.css"),
            Some("css".to_string())
        );
        assert_eq!(
            RouteResolver::extract_extension("/a/b/file.HTML"),
            Some("html".to_string())
        );
        assert_eq!(RouteResolver::extract_extension("/no-ext"), None);
        assert_eq!(RouteResolver::extract_extension("/.hidden"), None);
        assert_eq!(RouteResolver::extract_extension("/a.b/file"), None);
    }

    #[test]
    fn test_exact_pattern_match() {
        let routes = vec![compile(route(
            "ex.com",
            "/api/items",
            "http://java-api:8080",
        ))];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/api/items", fallback());
        assert_eq!(cfg.matched_pattern, Some("/api/items".to_string()));
        assert_eq!(cfg.origin_url, "http://java-api:8080/api/items");
    }

    #[test]
    fn test_wildcard_match() {
        let routes = vec![compile(route("ex.com", "/assets/*", "http://apache"))];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/assets/style.css", fallback());
        assert_eq!(cfg.matched_pattern, Some("/assets/*".to_string()));
        assert_eq!(cfg.origin_url, "http://apache/assets/style.css");
    }

    #[test]
    fn test_no_match_returns_fallback() {
        let routes = vec![compile(route("ex.com", "/api/*", "http://java"))];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/other/page", fallback());
        assert_eq!(cfg.matched_pattern, None);
        assert_eq!(cfg.domain, "fallback.com");
    }

    #[test]
    fn test_longest_pattern_wins() {
        // Spec: "se deben soportar wildcards" + reglas más específicas ganan.
        let routes = vec![
            compile(route("ex.com", "/*", "http://general")),
            compile(route("ex.com", "/api/v1/users", "http://users-api")),
            compile(route("ex.com", "/api/*", "http://api")),
        ];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/api/v1/users", fallback());
        assert_eq!(cfg.matched_pattern, Some("/api/v1/users".to_string()));
        assert_eq!(cfg.origin_url, "http://users-api/api/v1/users");
    }

    #[test]
    fn test_other_domain_ignored() {
        let routes = vec![compile(route("other.com", "/*", "http://other"))];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/anything", fallback());
        assert_eq!(cfg.matched_pattern, None);
    }

    #[test]
    fn test_disabled_route_ignored() {
        let mut r = route("ex.com", "/api/*", "http://api");
        r.enabled = false;
        let routes = vec![compile(r)];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/api/x", fallback());
        assert_eq!(cfg.matched_pattern, None);
    }

    #[test]
    fn test_file_rules_override_ttl() {
        let mut r = route("ex.com", "/assets/*", "http://apache");
        r.default_ttl_seconds = Some(60);
        r.file_rules = vec![
            FileRule {
                extension: "css".into(),
                ttl_seconds: 3600,
                cacheable: true,
            },
            FileRule {
                extension: "json".into(),
                ttl_seconds: 10,
                cacheable: false,
            },
        ];
        let routes = vec![compile(r)];

        let css_cfg = RouteResolver::resolve(&routes, "ex.com", "/assets/style.css", fallback());
        assert_eq!(css_cfg.ttl_seconds, Some(3600));
        assert!(css_cfg.cacheable);

        let json_cfg = RouteResolver::resolve(&routes, "ex.com", "/assets/data.json", fallback());
        assert_eq!(json_cfg.ttl_seconds, Some(10));
        assert!(!json_cfg.cacheable);

        // Extensión sin regla específica: cae a default_ttl_seconds
        let other_cfg = RouteResolver::resolve(&routes, "ex.com", "/assets/file.svg", fallback());
        assert_eq!(other_cfg.ttl_seconds, Some(60));
    }

    #[test]
    fn test_policy_from_route() {
        let mut r = route("ex.com", "/*", "http://o");
        r.cache_policy = "MRU".to_string();
        let routes = vec![compile(r)];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/x", fallback());
        assert_eq!(cfg.policy, Policy::Mru);
    }

    #[test]
    fn test_invalid_policy_in_route_falls_back() {
        let mut r = route("ex.com", "/*", "http://o");
        r.cache_policy = "BANANA".to_string();
        let routes = vec![compile(r)];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/x", fallback());
        // fallback().policy es Lru
        assert_eq!(cfg.policy, Policy::Lru);
    }

    #[test]
    fn test_auth_mode_propagated() {
        let mut r = route("ex.com", "/private/*", "http://o");
        r.auth_mode = AuthMode::ApiKey;
        let routes = vec![compile(r)];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/private/x", fallback());
        assert_eq!(cfg.auth_mode, AuthMode::ApiKey);
    }

    #[test]
    fn test_origin_join_no_double_slash() {
        let r = route("ex.com", "/*", "http://origin/");
        let routes = vec![compile(r)];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/path", fallback());
        assert_eq!(cfg.origin_url, "http://origin/path");
    }

    #[test]
    fn test_https_origin_works() {
        let r = route("ex.com", "/*", "https://external.example");
        let routes = vec![compile(r)];
        let cfg = RouteResolver::resolve(&routes, "ex.com", "/api/data", fallback());
        assert_eq!(cfg.origin_url, "https://external.example/api/data");
    }

    #[test]
    fn test_compile_invalid_pattern_errors() {
        let r = route("ex.com", "[invalid", "http://o");
        assert!(CompiledRoute::compile(r).is_err());
    }
}
