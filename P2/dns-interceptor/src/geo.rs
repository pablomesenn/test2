use maxminddb::Reader;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::IpAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeoError {
    #[error("no se pudo abrir la base de datos GeoIP: {0}")]
    DbOpen(#[from] maxminddb::MaxMindDBError),
}

// Schema genérico compatible con DB-IP y MaxMind
#[derive(Deserialize, Debug)]
struct DbIpCountry {
    country: Option<CountryRecord>,
}

#[derive(Deserialize, Debug)]
struct CountryRecord {
    iso_code: Option<String>,
}

pub struct GeoResolver {
    reader: Reader<Vec<u8>>,
    zone_map: HashMap<String, String>,
    default_zone: String,
}

impl GeoResolver {
    pub fn new(
        db_path: &str,
        zone_map: HashMap<String, String>,
        default_zone: String,
    ) -> Result<Self, GeoError> {
        let reader = Reader::open_readfile(db_path)?;
        Ok(Self { reader, zone_map, default_zone })
    }

    /// Dado una IP, devuelve el código ISO del país (ej: "CR", "US")
    pub fn country_for_ip(&self, ip: IpAddr) -> Option<String> {
        let record: DbIpCountry = self.reader.lookup(ip).ok()?;
        let code = record.country?.iso_code?.to_uppercase();
        Some(code)
    }

    /// Dado una IP, devuelve la IP del zonal cache que corresponde
    pub fn resolve_zone(&self, ip: IpAddr) -> String {
        match self.country_for_ip(ip) {
            Some(code) => {
                self.zone_map
                    .get(&code)
                    .cloned()
                    .unwrap_or_else(|| self.default_zone.clone())
            }
            None => self.default_zone.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_zone_map() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("US".to_string(), "10.0.0.1".to_string());
        m.insert("CR".to_string(), "10.0.0.2".to_string());
        m
    }

    #[test]
    fn test_zone_map_known_country() {
        let map = make_zone_map();
        assert_eq!(map.get("US").unwrap(), "10.0.0.1");
        assert_eq!(map.get("CR").unwrap(), "10.0.0.2");
    }

    #[test]
    fn test_zone_map_unknown_falls_back() {
        let map = make_zone_map();
        let default = "10.0.0.99".to_string();
        let result = map.get("XX").cloned().unwrap_or(default.clone());
        assert_eq!(result, default);
    }

    #[test]
    fn test_zone_map_case_insensitive_by_convention() {
        // El resolver siempre normaliza a uppercase antes de buscar
        let map = make_zone_map();
        let code = "cr".to_uppercase();
        assert_eq!(map.get(&code).unwrap(), "10.0.0.2");
    }

    #[test]
    fn test_resolve_zone_with_real_db() {
        // Esta prueba solo corre si existe el archivo de DB-IP
        let db_path = "./geoip/dbip-country-lite.mmdb";
        if !std::path::Path::new(db_path).exists() {
            println!("SKIP: base de datos DB-IP no encontrada en {}", db_path);
            return;
        }

        let mut map = HashMap::new();
        map.insert("US".to_string(), "10.0.0.1".to_string());

        let resolver = GeoResolver::new(db_path, map, "10.0.0.99".to_string()).unwrap();

        // 8.8.8.8 es de Google — debe ser US
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        let country = resolver.country_for_ip(ip);
        assert_eq!(country, Some("US".to_string()));

        let zone = resolver.resolve_zone(ip);
        assert_eq!(zone, "10.0.0.1");
    }

    #[test]
    fn test_resolve_zone_unknown_ip_gets_default() {
        let db_path = "./geoip/dbip-country-lite.mmdb";
        if !std::path::Path::new(db_path).exists() {
            println!("SKIP: base de datos DB-IP no encontrada en {}", db_path);
            return;
        }

        let map = HashMap::new(); // ningún país configurado
        let resolver = GeoResolver::new(db_path, map, "10.0.0.99".to_string()).unwrap();

        // Loopback no tiene país → debe ir al default
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let zone = resolver.resolve_zone(ip);
        assert_eq!(zone, "10.0.0.99");
    }

    #[test]
    fn test_resolve_zone_cr_ip() {
        let mut map = HashMap::new();
        map.insert("CR".to_string(), "10.0.0.2".to_string());
        map.insert("US".to_string(), "10.0.0.1".to_string());

        let resolver = GeoResolver::new(
            "./geoip/dbip-country-lite.mmdb",
            map,
            "10.0.0.99".to_string(),
        ).unwrap();

        // IP pública de ICE Costa Rica
        let ip: IpAddr = "190.10.10.1".parse().unwrap();
        let zone = resolver.resolve_zone(ip);
        assert_eq!(zone, "10.0.0.2");
    }

    #[test]
    fn test_resolve_zone_us_ip() {
        let mut map = HashMap::new();
        map.insert("CR".to_string(), "10.0.0.2".to_string());
        map.insert("US".to_string(), "10.0.0.1".to_string());

        let resolver = GeoResolver::new(
            "./geoip/dbip-country-lite.mmdb",
            map,
            "10.0.0.99".to_string(),
        ).unwrap();

        // IP pública de Google US
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        let zone = resolver.resolve_zone(ip);
        assert_eq!(zone, "10.0.0.1");
    }
}
