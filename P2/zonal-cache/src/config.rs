use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: String,
    pub cache_dir: String,
    pub max_cache_size_bytes: u64,
    pub default_policy: String,
    pub firebase_project_id: String,
    pub node_api_url: String,
    pub bootstrap_api_keys: Vec<(String, String)>, // (key, user_id)
    /// TTL en segundos para el cache en memoria de las rutas leídas de Firestore.
    /// Evita pegarle a Firestore en cada request.
    pub route_cache_ttl_secs: u64,
    /// Dominio default cuando se usa el endpoint /proxy?url=... y no se puede
    /// inferir un dominio gestionado.
    pub fallback_domain: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            listen_addr: env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            cache_dir: env::var("CACHE_DIR").unwrap_or_else(|_| "./cache_data".to_string()),
            max_cache_size_bytes: env::var("CACHE_MAX_SIZE_BYTES")
                .unwrap_or_else(|_| "104857600".to_string())
                .parse()
                .unwrap_or(104857600),
            default_policy: env::var("CACHE_POLICY").unwrap_or_else(|_| "LRU".to_string()),
            firebase_project_id: env::var("FIREBASE_PROJECT_ID")
                .unwrap_or_else(|_| "ic7602-p2".to_string()),
            node_api_url: env::var("NODE_API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            bootstrap_api_keys: Self::parse_api_keys(
                &env::var("BOOTSTRAP_API_KEYS").unwrap_or_default(),
            ),
            route_cache_ttl_secs: env::var("ROUTE_CACHE_TTL_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            fallback_domain: env::var("FALLBACK_DOMAIN").unwrap_or_else(|_| "default".to_string()),
        }
    }

    // Parsea "key1:user1,key2:user2" → Vec<(key, user_id)>
    fn parse_api_keys(raw: &str) -> Vec<(String, String)> {
        raw.split(',')
            .filter_map(|pair| {
                let mut parts = pair.trim().splitn(2, ':');
                let key = parts.next()?.trim().to_string();
                let user = parts.next()?.trim().to_string();
                if key.is_empty() || user.is_empty() {
                    None
                } else {
                    Some((key, user))
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_loads() {
        let config = Config::from_env();
        assert!(!config.listen_addr.is_empty());
        assert!(!config.cache_dir.is_empty());
        assert!(config.max_cache_size_bytes > 0);
        assert!(!config.default_policy.is_empty());
        assert!(config.route_cache_ttl_secs > 0);
    }

    #[test]
    fn test_valid_default_policy() {
        let config = Config::from_env();
        let valid = ["LRU", "LFU", "FIFO", "MRU", "RANDOM"];
        assert!(valid.contains(&config.default_policy.as_str()));
    }

    #[test]
    fn test_parse_api_keys_valid() {
        let keys = Config::parse_api_keys("key1:user1,key2:user2");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], ("key1".to_string(), "user1".to_string()));
        assert_eq!(keys[1], ("key2".to_string(), "user2".to_string()));
    }

    #[test]
    fn test_parse_api_keys_empty() {
        assert!(Config::parse_api_keys("").is_empty());
    }

    #[test]
    fn test_parse_api_keys_ignores_malformed() {
        let keys = Config::parse_api_keys("key1:user1,malformed,key2:user2");
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_parse_api_keys_handles_whitespace() {
        let keys = Config::parse_api_keys(" k1 : u1 , k2:u2 ");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], ("k1".to_string(), "u1".to_string()));
    }
}
