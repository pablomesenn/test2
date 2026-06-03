use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: String,
    pub geoip_path: String,
    pub upstream_dns: String,
    // Mapa de país ISO-3166 alpha-2 → IP del zonal cache
    // Ejemplo: "CR" -> "10.0.0.1", "US" -> "10.0.0.2"
    pub zone_map: HashMap<String, String>,
    // Dominios que interceptamos (los demás se resuelven normal)
    pub managed_domains: Vec<String>,
    // IP del zonal cache por defecto si no hay match de país
    pub default_zone: String,
}

impl Config {
    pub fn from_env() -> Self {
        let zone_map = Self::parse_zone_map(
            &env::var("ZONE_MAP").unwrap_or_default()
        );

        let managed_domains = env::var("MANAGED_DOMAINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            listen_addr: env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:53".to_string()),
            geoip_path: env::var("GEOIP_PATH")
                .unwrap_or_else(|_| "./geoip/GeoLite2-Country.mmdb".to_string()),
            upstream_dns: env::var("UPSTREAM_DNS")
                .unwrap_or_else(|_| "8.8.8.8:53".to_string()),
            default_zone: env::var("DEFAULT_ZONE")
                .unwrap_or_else(|_| "10.0.0.1".to_string()),
            zone_map,
            managed_domains,
        }
    }

    // Parsea "CR=10.0.0.1,US=10.0.0.2" → HashMap
    fn parse_zone_map(raw: &str) -> HashMap<String, String> {
        raw.split(',')
            .filter_map(|pair| {
                let mut parts = pair.trim().splitn(2, '=');
                let country = parts.next()?.trim().to_uppercase();
                let ip = parts.next()?.trim().to_string();
                if country.is_empty() || ip.is_empty() {
                    None
                } else {
                    Some((country, ip))
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_zone_map_valid() {
        let map = Config::parse_zone_map("CR=10.0.0.1,US=10.0.0.2,DE=10.0.0.3");
        assert_eq!(map.get("CR"), Some(&"10.0.0.1".to_string()));
        assert_eq!(map.get("US"), Some(&"10.0.0.2".to_string()));
        assert_eq!(map.get("DE"), Some(&"10.0.0.3".to_string()));
    }

    #[test]
    fn test_parse_zone_map_empty() {
        let map = Config::parse_zone_map("");
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_zone_map_ignores_malformed() {
        let map = Config::parse_zone_map("CR=10.0.0.1,MALFORMED,US=10.0.0.2");
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("CR"));
        assert!(map.contains_key("US"));
    }

    #[test]
    fn test_parse_zone_map_normalizes_case() {
        let map = Config::parse_zone_map("cr=10.0.0.1,us=10.0.0.2");
        assert!(map.contains_key("CR"));
        assert!(map.contains_key("US"));
    }
}
