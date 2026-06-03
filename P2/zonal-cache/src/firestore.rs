use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfigDoc {
    pub max_size_bytes: u64,
    pub ttl_seconds: u64,
    pub policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfigDoc {
    pub owner_uid: String,
    pub verified: bool,
    pub zone: String,
    pub cache_policy: Option<String>,
    pub cache_ttl: Option<u64>,
    pub max_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyDoc {
    pub user_id: String,
    pub active: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDoc {
    pub email: String,
    pub api_keys: Vec<String>,
    pub created_at: u64,
}

/// Regla por extensión de archivo (css, js, html, png...).
/// Permite override de TTL y declarar si el tipo es cacheable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRule {
    /// Sin punto: "css", "js", "png", "html", "json".
    pub extension: String,
    pub ttl_seconds: u64,
    pub cacheable: bool,
}

/// Modos de autenticación soportados para una ruta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthMode {
    #[default]
    None,
    /// Requiere encabezado `x-api-key`.
    ApiKey,
    /// Requiere sesión de usuario/contraseña (cookie o `x-session-token`).
    UserPassword,
}

/// Configuración de una ruta dentro de un dominio.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteConfigDoc {
    pub domain: String,
    pub pattern: String,
    pub origin_base_url: String,
    pub cache_max_size_bytes: u64,
    pub cache_policy: String,
    #[serde(default)]
    pub default_ttl_seconds: Option<u64>,
    #[serde(default)]
    pub file_rules: Vec<FileRule>,
    #[serde(default)]
    pub auth_mode: AuthMode,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub struct FirestoreClient {
    #[allow(dead_code)]
    project_id: String,
    db: Option<firestore::FirestoreDb>,
}

impl FirestoreClient {
    pub async fn new(project_id: &str) -> Self {
        match firestore::FirestoreDb::new(project_id).await {
            Ok(db) => {
                info!("Conexión a Firestore establecida: {}", project_id);
                Self {
                    project_id: project_id.to_string(),
                    db: Some(db),
                }
            }
            Err(e) => {
                error!("No se pudo conectar a Firestore: {}", e);
                Self {
                    project_id: project_id.to_string(),
                    db: None,
                }
            }
        }
    }

    pub async fn get_cache_config(&self) -> Result<CacheConfigDoc, String> {
        let db = self.db.as_ref().ok_or("Firestore no disponible")?;

        let doc: Option<CacheConfigDoc> = db
            .fluent()
            .select()
            .by_id_in("cache-config")
            .obj()
            .one("default")
            .await
            .map_err(|e| format!("Error leyendo cache-config: {}", e))?;

        doc.ok_or_else(|| "Documento 'default' no encontrado en cache-config".to_string())
    }

    pub async fn get_domain_config(&self, domain: &str) -> Result<DomainConfigDoc, String> {
        let db = self.db.as_ref().ok_or("Firestore no disponible")?;

        let doc: Option<DomainConfigDoc> = db
            .fluent()
            .select()
            .by_id_in("domains")
            .obj()
            .one(domain)
            .await
            .map_err(|e| format!("Error leyendo domain {}: {}", domain, e))?;

        doc.ok_or_else(|| format!("Dominio '{}' no encontrado", domain))
    }

    pub async fn get_api_key(&self, api_key: &str) -> Result<ApiKeyDoc, String> {
        let db = self.db.as_ref().ok_or("Firestore no disponible")?;

        let doc: Option<ApiKeyDoc> = db
            .fluent()
            .select()
            .by_id_in("api-keys")
            .obj()
            .one(api_key)
            .await
            .map_err(|e| format!("Error leyendo api-key: {}", e))?;

        doc.ok_or_else(|| "API key no encontrada".to_string())
    }

    /// Carga todas las rutas configuradas para un dominio.
    /// El RouteResolver es quien decide cuál aplica al path concreto.
    pub async fn list_routes_for_domain(
        &self,
        domain: &str,
    ) -> Result<Vec<RouteConfigDoc>, String> {
        let db = self.db.as_ref().ok_or("Firestore no disponible")?;

        let docs: Vec<RouteConfigDoc> = db
            .fluent()
            .select()
            .from("routes")
            .filter(|q| q.for_all([q.field("domain").eq(domain), q.field("enabled").eq(true)]))
            .obj()
            .query()
            .await
            .map_err(|e| format!("Error listando routes para {}: {}", domain, e))?;

        Ok(docs)
    }

    pub fn is_connected(&self) -> bool {
        self.db.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_serialization() {
        let config = CacheConfigDoc {
            max_size_bytes: 1000,
            ttl_seconds: 3600,
            policy: "LRU".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: CacheConfigDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.policy, "LRU");
        assert_eq!(deserialized.max_size_bytes, 1000);
    }

    #[test]
    fn test_domain_config_serialization() {
        let config = DomainConfigDoc {
            owner_uid: "user123".to_string(),
            verified: true,
            zone: "us-central1".to_string(),
            cache_policy: Some("LFU".to_string()),
            cache_ttl: Some(7200),
            max_size_bytes: Some(5242880),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DomainConfigDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.owner_uid, "user123");
        assert!(deserialized.verified);
        assert_eq!(deserialized.cache_policy, Some("LFU".to_string()));
    }

    #[test]
    fn test_api_key_serialization() {
        let doc = ApiKeyDoc {
            user_id: "admin".to_string(),
            active: true,
            created_at: 0,
        };
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: ApiKeyDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, "admin");
        assert!(deserialized.active);
    }

    #[test]
    fn test_route_config_camel_case_serialization() {
        let json = r#"{
            "domain": "example.com",
            "pattern": "/assets/*",
            "originBaseUrl": "https://origin.example.com",
            "cacheMaxSizeBytes": 1048576,
            "cachePolicy": "LRU",
            "defaultTtlSeconds": 600,
            "fileRules": [
                { "extension": "css", "ttl_seconds": 3600, "cacheable": true }
            ],
            "authMode": "API_KEY",
            "enabled": true
        }"#;
        let parsed: RouteConfigDoc = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.domain, "example.com");
        assert_eq!(parsed.pattern, "/assets/*");
        assert_eq!(parsed.cache_max_size_bytes, 1048576);
        assert_eq!(parsed.auth_mode, AuthMode::ApiKey);
        assert_eq!(parsed.file_rules.len(), 1);
        assert_eq!(parsed.file_rules[0].extension, "css");
    }

    #[test]
    fn test_auth_mode_default_is_none() {
        let json = r#"{
            "domain": "x.com",
            "pattern": "/*",
            "originBaseUrl": "https://x.com",
            "cacheMaxSizeBytes": 100,
            "cachePolicy": "LRU"
        }"#;
        let parsed: RouteConfigDoc = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.auth_mode, AuthMode::None);
        assert!(parsed.enabled, "enabled default debe ser true");
    }

    #[test]
    fn test_auth_mode_user_password() {
        let json = r#""USER_PASSWORD""#;
        let parsed: AuthMode = serde_json::from_str(json).unwrap();
        assert_eq!(parsed, AuthMode::UserPassword);
    }

    #[test]
    fn test_cache_config_default_policy_valid() {
        let config = CacheConfigDoc {
            max_size_bytes: 104857600,
            ttl_seconds: 3600,
            policy: "LRU".to_string(),
        };
        let valid = ["LRU", "LFU", "FIFO", "MRU", "RANDOM"];
        assert!(valid.contains(&config.policy.as_str()));
    }
}
