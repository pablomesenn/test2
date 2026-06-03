use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Policy {
    Lru,
    Lfu,
    Fifo,
    Mru,
    Random,
}

impl Policy {
    pub fn try_from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "LRU" => Some(Policy::Lru),
            "LFU" => Some(Policy::Lfu),
            "FIFO" => Some(Policy::Fifo),
            "MRU" => Some(Policy::Mru),
            "RANDOM" => Some(Policy::Random),
            _ => None,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        Self::try_from_str(s).unwrap_or(Policy::Lru)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Policy::Lru => "LRU",
            Policy::Lfu => "LFU",
            Policy::Fifo => "FIFO",
            Policy::Mru => "MRU",
            Policy::Random => "RANDOM",
        }
    }
}

/// Registro de metadatos de una entrada en caché.
///
/// `expires_at = None` significa "sin expiración" (cachear hasta evicción).
/// `domain` permite agrupar entries para cuotas por host.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub domain: String,
    pub size_bytes: u64,
    pub frequency: u64,          // para LFU
    pub last_used: u64,          // timestamp para LRU/MRU
    pub inserted_at: u64,        // timestamp para FIFO
    pub expires_at: Option<u64>, // timestamp UNIX en que expira; None = nunca
}

impl CacheEntry {
    /// Devuelve true si la entrada ya expiró según `now`.
    pub fn is_expired(&self, now: u64) -> bool {
        match self.expires_at {
            Some(exp) => now >= exp,
            None => false,
        }
    }

    /// Edad en segundos (0 si no aplica).
    pub fn age_secs(&self, now: u64) -> u64 {
        now.saturating_sub(self.inserted_at)
    }
}

/// Selector de víctima según la política configurada.
/// No tiene estado: opera puramente sobre el slice de entries que se le pase.
pub struct EvictionSelector;

impl EvictionSelector {
    /// Dado un conjunto de entries, devuelve la key de la víctima a evictar.
    /// Devuelve `None` si el slice está vacío.
    pub fn select_victim<'a>(entries: &'a [CacheEntry], policy: &Policy) -> Option<&'a str> {
        if entries.is_empty() {
            return None;
        }

        match policy {
            Policy::Lru => entries
                .iter()
                .min_by_key(|e| e.last_used)
                .map(|e| e.key.as_str()),

            Policy::Mru => entries
                .iter()
                .max_by_key(|e| e.last_used)
                .map(|e| e.key.as_str()),

            Policy::Lfu => entries
                .iter()
                .min_by_key(|e| e.frequency)
                .map(|e| e.key.as_str()),

            Policy::Fifo => entries
                .iter()
                .min_by_key(|e| e.inserted_at)
                .map(|e| e.key.as_str()),

            Policy::Random => {
                let idx = rand::thread_rng().gen_range(0..entries.len());
                Some(entries[idx].key.as_str())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entries() -> Vec<CacheEntry> {
        vec![
            CacheEntry {
                key: "a".into(),
                domain: "d".into(),
                size_bytes: 100,
                frequency: 5,
                last_used: 100,
                inserted_at: 1,
                expires_at: None,
            },
            CacheEntry {
                key: "b".into(),
                domain: "d".into(),
                size_bytes: 200,
                frequency: 1,
                last_used: 200,
                inserted_at: 2,
                expires_at: None,
            },
            CacheEntry {
                key: "c".into(),
                domain: "d".into(),
                size_bytes: 150,
                frequency: 3,
                last_used: 50,
                inserted_at: 3,
                expires_at: None,
            },
        ]
    }

    #[test]
    fn test_try_from_str_valid() {
        assert_eq!(Policy::try_from_str("LRU"), Some(Policy::Lru));
        assert_eq!(Policy::try_from_str("LFU"), Some(Policy::Lfu));
        assert_eq!(Policy::try_from_str("FIFO"), Some(Policy::Fifo));
        assert_eq!(Policy::try_from_str("MRU"), Some(Policy::Mru));
        assert_eq!(Policy::try_from_str("RANDOM"), Some(Policy::Random));
    }

    #[test]
    fn test_try_from_str_invalid_returns_none() {
        assert_eq!(Policy::try_from_str("invalid"), None);
        assert_eq!(Policy::try_from_str(""), None);
    }

    #[test]
    fn test_from_str_falls_back_to_lru() {
        // Compat: la versión vieja debe seguir cayendo a LRU.
        assert_eq!(Policy::from_str("invalid"), Policy::Lru);
        assert_eq!(Policy::from_str("LFU"), Policy::Lfu);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(Policy::try_from_str("lru"), Some(Policy::Lru));
        assert_eq!(Policy::try_from_str("Lfu"), Some(Policy::Lfu));
        assert_eq!(Policy::try_from_str("fIfO"), Some(Policy::Fifo));
    }

    #[test]
    fn test_as_str_roundtrip() {
        for p in [
            Policy::Lru,
            Policy::Lfu,
            Policy::Fifo,
            Policy::Mru,
            Policy::Random,
        ] {
            let s = p.as_str();
            assert_eq!(Policy::try_from_str(s), Some(p));
        }
    }

    #[test]
    fn test_lru_selects_least_recently_used() {
        let entries = make_entries();
        // last_used: a=100, b=200, c=50 → víctima: c
        let victim = EvictionSelector::select_victim(&entries, &Policy::Lru);
        assert_eq!(victim, Some("c"));
    }

    #[test]
    fn test_mru_selects_most_recently_used() {
        let entries = make_entries();
        // last_used: a=100, b=200, c=50 → víctima: b
        let victim = EvictionSelector::select_victim(&entries, &Policy::Mru);
        assert_eq!(victim, Some("b"));
    }

    #[test]
    fn test_lfu_selects_least_frequently_used() {
        let entries = make_entries();
        // frequency: a=5, b=1, c=3 → víctima: b
        let victim = EvictionSelector::select_victim(&entries, &Policy::Lfu);
        assert_eq!(victim, Some("b"));
    }

    #[test]
    fn test_fifo_selects_oldest_inserted() {
        let entries = make_entries();
        // inserted_at: a=1, b=2, c=3 → víctima: a
        let victim = EvictionSelector::select_victim(&entries, &Policy::Fifo);
        assert_eq!(victim, Some("a"));
    }

    #[test]
    fn test_random_selects_some_existing_entry() {
        let entries = make_entries();
        let victim = EvictionSelector::select_victim(&entries, &Policy::Random);
        assert!(victim.is_some());
        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&victim.unwrap()));
    }

    #[test]
    fn test_empty_entries_returns_none() {
        let entries: Vec<CacheEntry> = vec![];
        for policy in [
            Policy::Lru,
            Policy::Lfu,
            Policy::Fifo,
            Policy::Mru,
            Policy::Random,
        ] {
            assert!(EvictionSelector::select_victim(&entries, &policy).is_none());
        }
    }

    #[test]
    fn test_single_entry_always_selected() {
        let entries = vec![CacheEntry {
            key: "solo".into(),
            domain: "d".into(),
            size_bytes: 100,
            frequency: 1,
            last_used: 1,
            inserted_at: 1,
            expires_at: None,
        }];
        for policy in [
            Policy::Lru,
            Policy::Lfu,
            Policy::Fifo,
            Policy::Mru,
            Policy::Random,
        ] {
            let victim = EvictionSelector::select_victim(&entries, &policy);
            assert_eq!(victim, Some("solo"));
        }
    }

    #[test]
    fn test_is_expired_with_none_never_expires() {
        let e = CacheEntry {
            key: "k".into(),
            domain: "d".into(),
            size_bytes: 0,
            frequency: 0,
            last_used: 0,
            inserted_at: 0,
            expires_at: None,
        };
        assert!(!e.is_expired(u64::MAX));
    }

    #[test]
    fn test_is_expired_before_and_after() {
        let e = CacheEntry {
            key: "k".into(),
            domain: "d".into(),
            size_bytes: 0,
            frequency: 0,
            last_used: 0,
            inserted_at: 100,
            expires_at: Some(200),
        };
        assert!(!e.is_expired(199));
        assert!(e.is_expired(200));
        assert!(e.is_expired(500));
    }

    #[test]
    fn test_age_secs_basic() {
        let e = CacheEntry {
            key: "k".into(),
            domain: "d".into(),
            size_bytes: 0,
            frequency: 0,
            last_used: 0,
            inserted_at: 100,
            expires_at: None,
        };
        assert_eq!(e.age_secs(150), 50);
        assert_eq!(e.age_secs(100), 0);
        // saturación: now < inserted_at no debe panic
        assert_eq!(e.age_secs(50), 0);
    }
}
