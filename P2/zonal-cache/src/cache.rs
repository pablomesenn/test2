use crate::algorithms::{CacheEntry, EvictionSelector, Policy};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("error de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("entrada no encontrada: {0}")]
    NotFound(String),
    #[error("entrada expirada: {0}")]
    Expired(String),
    #[error("caché lleno y no se pudo evictar (dominio {0})")]
    Full(String),
    #[error("recurso excede el tamaño máximo del dominio")]
    EntryTooLarge,
}

/// Metadatos persistidos en memoria por cada entrada.
/// El cuerpo binario vive en disco bajo `cache_dir/<domain_hash>/<key_hash>`.
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    pub entry: CacheEntry,
    pub content_type: String,
    pub status_code: u16,
}

/// Bucket por dominio: tiene su propia cuota, política y conjunto de entries.
/// Esto es lo que el spec exige: "tamaño máximo por dominio no puede sobrepasar
/// el valor definido en Firebase".
#[derive(Debug)]
pub struct DomainBucket {
    pub domain: String,
    pub max_size_bytes: u64,
    pub current_size_bytes: u64,
    pub policy: Policy,
    /// key → metadata
    pub entries: HashMap<String, CacheMetadata>,
}

impl DomainBucket {
    pub fn new(domain: &str, max_size_bytes: u64, policy: Policy) -> Self {
        Self {
            domain: domain.to_string(),
            max_size_bytes,
            current_size_bytes: 0,
            policy,
            entries: HashMap::new(),
        }
    }
}

/// Mantener un hash por dominio evita problemas con caracteres inválidos
/// en nombres de archivo (`:`, `/`, `?`, etc.) y caps de longitud del FS.
pub struct DiskCache {
    pub cache_dir: PathBuf,
    /// Cuota por defecto si el dominio no tiene una configurada.
    pub default_max_size_bytes: u64,
    /// Política por defecto.
    pub default_policy: Policy,
    /// Buckets por dominio.
    pub buckets: HashMap<String, DomainBucket>,
}

impl DiskCache {
    pub fn new(cache_dir: &str, default_max_size_bytes: u64, default_policy: Policy) -> Self {
        std::fs::create_dir_all(cache_dir).ok();
        Self {
            cache_dir: PathBuf::from(cache_dir),
            default_max_size_bytes,
            default_policy,
            buckets: HashMap::new(),
        }
    }

    fn now_ts() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn hash_short(s: &str) -> String {
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        let full = hex::encode(h.finalize());
        full[..16].to_string()
    }

    fn hash_full(s: &str) -> String {
        let mut h = Sha256::new();
        h.update(s.as_bytes());
        hex::encode(h.finalize())
    }

    fn domain_dir(&self, domain: &str) -> PathBuf {
        self.cache_dir.join(Self::hash_short(domain))
    }

    fn key_path(&self, domain: &str, key: &str) -> PathBuf {
        self.domain_dir(domain).join(Self::hash_full(key))
    }

    /// Asegura que el bucket del dominio existe con la config indicada.
    /// Si ya existe, **no sobrescribe** la config (eso lo decide quien llame
    /// `set_domain_config` explícitamente).
    pub fn ensure_bucket(&mut self, domain: &str, max_size_bytes: u64, policy: Policy) {
        self.buckets
            .entry(domain.to_string())
            .or_insert_with(|| DomainBucket::new(domain, max_size_bytes, policy));
    }

    /// Actualiza la configuración (cuota + política) del bucket de un dominio.
    /// Si la nueva cuota es menor que el tamaño actual, evicta hasta encajar.
    pub async fn set_domain_config(
        &mut self,
        domain: &str,
        max_size_bytes: u64,
        policy: Policy,
    ) -> Result<(), CacheError> {
        let bucket = self
            .buckets
            .entry(domain.to_string())
            .or_insert_with(|| DomainBucket::new(domain, max_size_bytes, policy.clone()));
        bucket.max_size_bytes = max_size_bytes;
        bucket.policy = policy;

        // Si shrink, evictar hasta caber
        while bucket.current_size_bytes > bucket.max_size_bytes {
            let entries_vec: Vec<CacheEntry> =
                bucket.entries.values().map(|m| m.entry.clone()).collect();
            let victim_key = EvictionSelector::select_victim(&entries_vec, &bucket.policy)
                .map(|s| s.to_string());
            match victim_key {
                Some(k) => {
                    Self::remove_entry_internal(bucket, &self.cache_dir, &k).await?;
                }
                None => break,
            }
        }
        Ok(())
    }

    pub fn contains(&self, domain: &str, key: &str) -> bool {
        self.buckets
            .get(domain)
            .map(|b| b.entries.contains_key(key))
            .unwrap_or(false)
    }

    /// Obtiene una entrada. Verifica expiración y la elimina si está expirada.
    /// Devuelve (body, content_type, status, age_secs).
    pub async fn get(
        &mut self,
        domain: &str,
        key: &str,
    ) -> Result<(Vec<u8>, String, u16, u64), CacheError> {
        let now = Self::now_ts();
        let bucket = self
            .buckets
            .get_mut(domain)
            .ok_or_else(|| CacheError::NotFound(format!("{}:{}", domain, key)))?;

        // Snapshot mínimo para decidir si expiró antes de mutar.
        let (expired, content_type, status_code, age) = {
            let meta = bucket
                .entries
                .get(key)
                .ok_or_else(|| CacheError::NotFound(format!("{}:{}", domain, key)))?;
            (
                meta.entry.is_expired(now),
                meta.content_type.clone(),
                meta.status_code,
                meta.entry.age_secs(now),
            )
        };

        if expired {
            // Eliminar la entrada expirada antes de devolver el error.
            Self::remove_entry_internal(bucket, &self.cache_dir, key)
                .await
                .ok();
            return Err(CacheError::Expired(format!("{}:{}", domain, key)));
        }

        // Actualizar metadata de uso (LRU/LFU)
        if let Some(meta) = bucket.entries.get_mut(key) {
            meta.entry.last_used = now;
            meta.entry.frequency += 1;
        }

        let path = self.key_path(domain, key);
        let data = fs::read(&path).await?;
        Ok((data, content_type, status_code, age))
    }

    /// Inserta una entrada en el bucket del dominio. Si excede la cuota del
    /// bucket, evicta según la política del bucket. Si la entrada por sí sola
    /// excede `max_size_bytes`, retorna `EntryTooLarge`.
    pub async fn put(
        &mut self,
        domain: &str,
        key: &str,
        data: &[u8],
        content_type: &str,
        status_code: u16,
        ttl_seconds: Option<u64>,
    ) -> Result<(), CacheError> {
        let size = data.len() as u64;
        let now = Self::now_ts();

        // Crear bucket si no existe (con defaults globales).
        let bucket = self.buckets.entry(domain.to_string()).or_insert_with(|| {
            DomainBucket::new(
                domain,
                self.default_max_size_bytes,
                self.default_policy.clone(),
            )
        });

        if size > bucket.max_size_bytes {
            return Err(CacheError::EntryTooLarge);
        }

        // Reemplazo de entrada existente: descontar tamaño viejo.
        if let Some(meta) = bucket.entries.get(key) {
            let old_size = meta.entry.size_bytes;
            let path = self
                .cache_dir
                .join(Self::hash_short(domain))
                .join(Self::hash_full(key));
            fs::remove_file(&path).await.ok();
            bucket.entries.remove(key);
            bucket.current_size_bytes = bucket.current_size_bytes.saturating_sub(old_size);
        }

        // Evictar hasta hacer espacio.
        while bucket.current_size_bytes + size > bucket.max_size_bytes {
            let entries_vec: Vec<CacheEntry> =
                bucket.entries.values().map(|m| m.entry.clone()).collect();
            let victim_key = EvictionSelector::select_victim(&entries_vec, &bucket.policy)
                .map(|s| s.to_string())
                .ok_or_else(|| CacheError::Full(domain.to_string()))?;
            Self::remove_entry_internal(bucket, &self.cache_dir, &victim_key).await?;
        }

        // Asegurar directorio del dominio.
        let dom_dir = self.cache_dir.join(Self::hash_short(domain));
        fs::create_dir_all(&dom_dir).await?;

        // Escribir cuerpo binario.
        let path = dom_dir.join(Self::hash_full(key));
        let mut file = fs::File::create(&path).await?;
        file.write_all(data).await?;

        let expires_at = ttl_seconds.map(|t| now + t);

        bucket.entries.insert(
            key.to_string(),
            CacheMetadata {
                entry: CacheEntry {
                    key: key.to_string(),
                    domain: domain.to_string(),
                    size_bytes: size,
                    frequency: 1,
                    last_used: now,
                    inserted_at: now,
                    expires_at,
                },
                content_type: content_type.to_string(),
                status_code,
            },
        );
        bucket.current_size_bytes += size;

        Ok(())
    }

    pub async fn remove(&mut self, domain: &str, key: &str) -> Result<(), CacheError> {
        if let Some(bucket) = self.buckets.get_mut(domain) {
            Self::remove_entry_internal(bucket, &self.cache_dir, key).await?;
        }
        Ok(())
    }

    /// Helper interno: elimina una entrada del bucket dado y borra su archivo.
    async fn remove_entry_internal(
        bucket: &mut DomainBucket,
        cache_dir: &Path,
        key: &str,
    ) -> Result<(), CacheError> {
        if let Some(meta) = bucket.entries.remove(key) {
            bucket.current_size_bytes = bucket
                .current_size_bytes
                .saturating_sub(meta.entry.size_bytes);
            let dom_dir = cache_dir.join(Self::hash_short(&bucket.domain));
            let path = dom_dir.join(Self::hash_full(key));
            fs::remove_file(&path).await.ok();
        }
        Ok(())
    }

    /// Estadísticas globales: agrega todos los buckets.
    pub fn stats(&self) -> CacheStats {
        let mut total_entries = 0;
        let mut total_size = 0u64;
        let mut by_domain = Vec::new();
        for (dom, b) in &self.buckets {
            total_entries += b.entries.len();
            total_size += b.current_size_bytes;
            by_domain.push(DomainStats {
                domain: dom.clone(),
                entry_count: b.entries.len(),
                current_size_bytes: b.current_size_bytes,
                max_size_bytes: b.max_size_bytes,
                policy: b.policy.as_str().to_string(),
            });
        }
        CacheStats {
            total_entries,
            total_size_bytes: total_size,
            default_max_size_bytes: self.default_max_size_bytes,
            default_policy: self.default_policy.as_str().to_string(),
            by_domain,
        }
    }
}

#[derive(Debug)]
pub struct DomainStats {
    pub domain: String,
    pub entry_count: usize,
    pub current_size_bytes: u64,
    pub max_size_bytes: u64,
    pub policy: String,
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub default_max_size_bytes: u64,
    pub default_policy: String,
    pub by_domain: Vec<DomainStats>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use uuid::Uuid;

    fn temp_cache(name: &str, max_size: u64, policy: Policy) -> DiskCache {
        let unique_id = Uuid::new_v4().to_string();
        let dir = env::temp_dir().join(format!("zonal_cache_test_{}_{}", name, unique_id));
        DiskCache::new(dir.to_str().unwrap(), max_size, policy)
    }

    #[tokio::test]
    async fn test_put_and_get_basic() {
        let mut cache = temp_cache("put_get", 1024, Policy::Lru);
        cache
            .put(
                "example.com",
                "key1",
                b"hello world",
                "text/plain",
                200,
                None,
            )
            .await
            .unwrap();
        let (data, ct, status, age) = cache.get("example.com", "key1").await.unwrap();
        assert_eq!(data, b"hello world");
        assert_eq!(ct, "text/plain");
        assert_eq!(status, 200);
        assert!(age < 5);
    }

    #[tokio::test]
    async fn test_contains_per_domain() {
        let mut cache = temp_cache("contains", 1024, Policy::Lru);
        cache
            .put("a.com", "k", b"data", "text/plain", 200, None)
            .await
            .unwrap();
        assert!(cache.contains("a.com", "k"));
        // Mismo key, otro dominio: no debe colisionar.
        assert!(!cache.contains("b.com", "k"));
    }

    #[tokio::test]
    async fn test_remove() {
        let mut cache = temp_cache("remove", 1024, Policy::Lru);
        cache
            .put("a.com", "k", b"data", "text/plain", 200, None)
            .await
            .unwrap();
        cache.remove("a.com", "k").await.unwrap();
        assert!(!cache.contains("a.com", "k"));
    }

    #[tokio::test]
    async fn test_size_tracking_per_domain() {
        let mut cache = temp_cache("size", 1024, Policy::Lru);
        cache
            .put("a.com", "k1", b"hello", "text/plain", 200, None)
            .await
            .unwrap();
        cache
            .put("a.com", "k2", b"world!", "text/plain", 200, None)
            .await
            .unwrap();
        cache
            .put("b.com", "k1", b"xx", "text/plain", 200, None)
            .await
            .unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_size_bytes, 13);
        let a_stats = stats
            .by_domain
            .iter()
            .find(|d| d.domain == "a.com")
            .unwrap();
        let b_stats = stats
            .by_domain
            .iter()
            .find(|d| d.domain == "b.com")
            .unwrap();
        assert_eq!(a_stats.current_size_bytes, 11);
        assert_eq!(b_stats.current_size_bytes, 2);
    }

    #[tokio::test]
    async fn test_eviction_lru() {
        let mut cache = temp_cache("evict_lru", 10, Policy::Lru);
        cache
            .put("d.com", "a", b"12345", "text/plain", 200, None)
            .await
            .unwrap();
        // Tocar "a" para que sea más reciente.
        let _ = cache.get("d.com", "a").await.unwrap();
        cache
            .put("d.com", "b", b"67890", "text/plain", 200, None)
            .await
            .unwrap();

        // Insertar tercero: debería evictar "b" (más viejo en last_used).
        cache
            .put("d.com", "c", b"abcde", "text/plain", 200, None)
            .await
            .unwrap();

        assert!(
            cache.contains("d.com", "a"),
            "a debió quedar (era el más recientemente usado)"
        );
        assert!(cache.contains("d.com", "c"));
        let stats = cache.stats();
        assert_eq!(stats.total_size_bytes, 10);
    }

    #[tokio::test]
    async fn test_eviction_fifo() {
        let mut cache = temp_cache("evict_fifo", 10, Policy::Fifo);
        cache
            .put("d.com", "first", b"12345", "text/plain", 200, None)
            .await
            .unwrap();
        if let Some(b) = cache.buckets.get_mut("d.com") {
            if let Some(m) = b.entries.get_mut("first") {
                m.entry.inserted_at = 1;
            }
        }
        cache
            .put("d.com", "second", b"67890", "text/plain", 200, None)
            .await
            .unwrap();
        if let Some(b) = cache.buckets.get_mut("d.com") {
            if let Some(m) = b.entries.get_mut("second") {
                m.entry.inserted_at = 2;
            }
        }
        // Tocar "first" — no debe importar en FIFO.
        let _ = cache.get("d.com", "first").await.unwrap();

        cache
            .put("d.com", "third", b"abcde", "text/plain", 200, None)
            .await
            .unwrap();

        assert!(
            !cache.contains("d.com", "first"),
            "FIFO debe evictar el primero insertado"
        );
        assert!(cache.contains("d.com", "third"));
    }

    #[tokio::test]
    async fn test_eviction_lfu() {
        let mut cache = temp_cache("evict_lfu", 10, Policy::Lfu);
        cache
            .put("d.com", "a", b"12345", "text/plain", 200, None)
            .await
            .unwrap();
        // a se usa varias veces
        for _ in 0..5 {
            let _ = cache.get("d.com", "a").await.unwrap();
        }
        cache
            .put("d.com", "b", b"67890", "text/plain", 200, None)
            .await
            .unwrap();
        // b apenas se usa al insertarse.

        cache
            .put("d.com", "c", b"abcde", "text/plain", 200, None)
            .await
            .unwrap();

        assert!(
            cache.contains("d.com", "a"),
            "LFU debe conservar a (más frecuente)"
        );
        assert!(
            !cache.contains("d.com", "b"),
            "LFU debe evictar b (menos frecuente)"
        );
    }

    #[tokio::test]
    async fn test_eviction_mru() {
        let mut cache = temp_cache("evict_mru", 10, Policy::Mru);
        cache
            .put("d.com", "a", b"12345", "text/plain", 200, None)
            .await
            .unwrap();
        cache
            .put("d.com", "b", b"67890", "text/plain", 200, None)
            .await
            .unwrap();
        // Tocar "b" — ahora es el más recientemente usado.
        let _ = cache.get("d.com", "b").await.unwrap();
        cache
            .put("d.com", "c", b"abcde", "text/plain", 200, None)
            .await
            .unwrap();

        // MRU evicta el más recientemente usado al momento de la evicción.
        // Tras get("b"), "b" era MRU; al insertar "c", se evictó "b".
        assert!(!cache.contains("d.com", "b"), "MRU debe evictar b");
        assert!(cache.contains("d.com", "a"));
    }

    #[tokio::test]
    async fn test_per_domain_quotas_are_independent() {
        let mut cache = temp_cache("quotas", 100, Policy::Lru);
        cache
            .set_domain_config("small.com", 5, Policy::Lru)
            .await
            .unwrap();
        cache
            .set_domain_config("big.com", 100, Policy::Lru)
            .await
            .unwrap();

        // small.com solo aguanta 5 bytes.
        cache
            .put("small.com", "a", b"12345", "text/plain", 200, None)
            .await
            .unwrap();
        cache
            .put("small.com", "b", b"67890", "text/plain", 200, None)
            .await
            .unwrap();
        // big.com no se ve afectado.
        cache
            .put("big.com", "x", b"hello world", "text/plain", 200, None)
            .await
            .unwrap();

        let stats = cache.stats();
        let small = stats
            .by_domain
            .iter()
            .find(|d| d.domain == "small.com")
            .unwrap();
        let big = stats
            .by_domain
            .iter()
            .find(|d| d.domain == "big.com")
            .unwrap();
        assert!(small.current_size_bytes <= 5);
        assert_eq!(big.current_size_bytes, 11);
    }

    #[tokio::test]
    async fn test_entry_too_large_for_domain() {
        let mut cache = temp_cache("too_large", 100, Policy::Lru);
        cache
            .set_domain_config("tiny.com", 5, Policy::Lru)
            .await
            .unwrap();
        let res = cache
            .put(
                "tiny.com",
                "k",
                b"this is way too big",
                "text/plain",
                200,
                None,
            )
            .await;
        assert!(matches!(res, Err(CacheError::EntryTooLarge)));
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let mut cache = temp_cache("ttl", 1024, Policy::Lru);
        // TTL = 0 → expira en el mismo segundo.
        cache
            .put("d.com", "k", b"data", "text/plain", 200, Some(0))
            .await
            .unwrap();
        // Esperar un segundo para garantizar que now > expires_at.
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        let res = cache.get("d.com", "k").await;
        assert!(matches!(res, Err(CacheError::Expired(_))));
        // Tras intentar leer, debe haberse evictado.
        assert!(!cache.contains("d.com", "k"));
    }

    #[tokio::test]
    async fn test_ttl_none_never_expires() {
        let mut cache = temp_cache("ttl_none", 1024, Policy::Lru);
        cache
            .put("d.com", "k", b"data", "text/plain", 200, None)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(cache.get("d.com", "k").await.is_ok());
    }

    #[tokio::test]
    async fn test_replace_existing_key() {
        let mut cache = temp_cache("replace", 1024, Policy::Lru);
        cache
            .put("d.com", "k", b"old", "text/plain", 200, None)
            .await
            .unwrap();
        let stats = cache.stats();
        assert_eq!(stats.total_size_bytes, 3);

        cache
            .put("d.com", "k", b"new_data", "text/plain", 200, None)
            .await
            .unwrap();
        let stats = cache.stats();
        assert_eq!(stats.total_size_bytes, 8);
        let (body, _, _, _) = cache.get("d.com", "k").await.unwrap();
        assert_eq!(body, b"new_data");
    }

    #[tokio::test]
    async fn test_binary_data_roundtrip() {
        // Bytes no-UTF8 deben sobrevivir el round-trip (PNG, JS minificado, etc.).
        let mut cache = temp_cache("binary", 1024, Policy::Lru);
        let payload = vec![0u8, 1, 2, 0xff, 0xfe, 0xfd, 0x80, 0x81];
        cache
            .put("d.com", "k", &payload, "image/png", 200, None)
            .await
            .unwrap();
        let (data, ct, status, _) = cache.get("d.com", "k").await.unwrap();
        assert_eq!(data, payload);
        assert_eq!(ct, "image/png");
        assert_eq!(status, 200);
    }

    #[tokio::test]
    async fn test_shrink_quota_evicts() {
        let mut cache = temp_cache("shrink", 1024, Policy::Lru);
        cache
            .put("d.com", "a", b"12345", "text/plain", 200, None)
            .await
            .unwrap();
        cache
            .put("d.com", "b", b"67890", "text/plain", 200, None)
            .await
            .unwrap();
        let stats = cache.stats();
        assert_eq!(stats.total_size_bytes, 10);

        // Achicar cuota a 5 — debe evictar lo necesario.
        cache
            .set_domain_config("d.com", 5, Policy::Lru)
            .await
            .unwrap();
        let stats = cache.stats();
        assert!(stats.total_size_bytes <= 5);
    }
}
