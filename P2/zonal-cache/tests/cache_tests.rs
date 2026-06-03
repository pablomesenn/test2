//! Tests de integración del Zonal Cache.
//!
//! Cubren flujos end-to-end que los unit tests no pueden cubrir porque
//! necesitan múltiples módulos trabajando juntos o un servidor HTTP real
//! (levantado con mockito).
//!
//! Para correr solo estos tests:
//!   cargo test --test cache_tests
//!
//! Para correr con output visible:
//!   cargo test --test cache_tests -- --nocapture

use mockito::Server;
use reqwest::Client;
use std::env;
use uuid::Uuid;
use zonal_cache::algorithms::Policy;
use zonal_cache::auth::AuthManager;
use zonal_cache::cache::DiskCache;
use zonal_cache::firestore::AuthMode;
use zonal_cache::proxy::{
    build_cache_key, fetch_from_origin, handle_proxy_request, is_cacheable_response, validate_url,
};
use zonal_cache::route_resolver::{CompiledRoute, EffectiveConfig, RouteResolver};
use zonal_cache::firestore::{FileRule, RouteConfigDoc};

// ─────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────

/// Crea un DiskCache temporal con nombre único para aislar cada test.
fn temp_cache(label: &str, max_bytes: u64, policy: Policy) -> DiskCache {
    let dir = env::temp_dir().join(format!("zc_integ_{}_{}", label, Uuid::new_v4()));
    DiskCache::new(dir.to_str().unwrap(), max_bytes, policy)
}

/// AuthManager listo con un usuario y una API key pre-registrados.
fn make_auth() -> AuthManager {
    let mut auth = AuthManager::new(3600);
    auth.register_user("alice", "secret123");
    auth.register_api_key("api-key-valid", "alice");
    auth
}

/// EffectiveConfig mínima para pruebas de proxy.
fn eff(
    origin_url: &str,
    auth_mode: AuthMode,
    cacheable: bool,
    ttl: Option<u64>,
) -> EffectiveConfig {
    EffectiveConfig {
        domain: "test.local".into(),
        origin_url: origin_url.into(),
        policy: Policy::Lru,
        max_size_bytes: 10 * 1024 * 1024,
        ttl_seconds: ttl,
        cacheable,
        auth_mode,
        matched_pattern: Some("/*".into()),
    }
}

/// Construye un RouteConfigDoc para el route_resolver.
fn make_route(
    domain: &str,
    pattern: &str,
    origin: &str,
    auth_mode: AuthMode,
    file_rules: Vec<FileRule>,
    ttl: Option<u64>,
) -> RouteConfigDoc {
    RouteConfigDoc {
        domain: domain.to_string(),
        pattern: pattern.to_string(),
        origin_base_url: origin.to_string(),
        cache_max_size_bytes: 5 * 1024 * 1024,
        cache_policy: "LRU".to_string(),
        default_ttl_seconds: ttl,
        file_rules,
        auth_mode,
        enabled: true,
    }
}

// ─────────────────────────────────────────────
//  1. Flujo proxy completo con mockito
// ─────────────────────────────────────────────

/// MISS → fetch al origen → respuesta al cliente → entrada guardada en disco.
#[tokio::test]
async fn test_proxy_cache_miss_fetches_origin_and_stores() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/page")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<h1>Hola</h1>")
        .create_async()
        .await;

    let url = format!("{}/page", server.url());
    let auth = make_auth();
    let mut cache = temp_cache("miss_fetch", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff(&url, AuthMode::None, true, None);

    let resp = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();

    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"<h1>Hola</h1>");
    assert!(!resp.from_cache, "Primer request debe ser MISS");

    let key = build_cache_key(&url);
    assert!(
        cache.contains("test.local", &key),
        "Entrada debe quedar en caché tras MISS"
    );

    mock.assert_async().await;
}

/// Segunda llamada con la misma URL → HIT desde disco, el origen no recibe más requests.
#[tokio::test]
async fn test_proxy_cache_hit_on_second_request() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/cached")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("cached content")
        .expect(1)
        .create_async()
        .await;

    let url = format!("{}/cached", server.url());
    let auth = make_auth();
    let mut cache = temp_cache("hit_second", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff(&url, AuthMode::None, true, None);

    let r1 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(!r1.from_cache);

    let r2 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(r2.from_cache, "Segunda llamada debe ser HIT");
    assert_eq!(r2.body, b"cached content");

    mock.assert_async().await;
}

/// Respuesta binaria (simula PNG): el round-trip debe ser fiel byte a byte.
#[tokio::test]
async fn test_proxy_binary_response_roundtrip() {
    let binary_payload: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0xFF, 0x00];

    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/image.png")
        .with_status(200)
        .with_header("content-type", "image/png")
        .with_body(binary_payload.clone())
        .create_async()
        .await;

    let url = format!("{}/image.png", server.url());
    let auth = make_auth();
    let mut cache = temp_cache("binary_rt", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff(&url, AuthMode::None, true, None);

    let r1 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(r1.body, binary_payload);
    assert_eq!(r1.content_type, "image/png");

    let r2 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(r2.from_cache);
    assert_eq!(r2.body, binary_payload, "Binario debe sobrevivir el round-trip");

    mock.assert_async().await;
}

/// Respuestas con status 404 y 500 NO deben cachearse.
#[tokio::test]
async fn test_proxy_error_responses_not_cached() {
    for status in [404u16, 500u16] {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/error")
            .with_status(status as usize)
            .with_header("content-type", "text/plain")
            .with_body("error")
            .expect(2)
            .create_async()
            .await;

        let url = format!("{}/error", server.url());
        let auth = make_auth();
        let mut cache = temp_cache(&format!("no_cache_{}", status), 10 * 1024 * 1024, Policy::Lru);
        let client = Client::new();
        let cfg = eff(&url, AuthMode::None, true, None);

        let r1 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
            .await
            .unwrap();
        assert_eq!(r1.status, status);
        assert!(!r1.from_cache);

        let r2 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
            .await
            .unwrap();
        assert!(!r2.from_cache, "status {} no debe cachearse", status);

        mock.assert_async().await;
    }
}

/// TTL expirado → la entrada se descarta y se re-fetchea del origen.
#[tokio::test]
async fn test_proxy_expired_ttl_refetches_origin() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/volatile")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("fresh")
        .expect(2)
        .create_async()
        .await;

    let url = format!("{}/volatile", server.url());
    let auth = make_auth();
    let mut cache = temp_cache("ttl_refetch", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff(&url, AuthMode::None, true, Some(0)); // TTL = 0

    let r1 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(!r1.from_cache);

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    let r2 = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(!r2.from_cache, "Entrada expirada debe producir MISS");

    mock.assert_async().await;
}

// ─────────────────────────────────────────────
//  2. Autenticación (integración proxy + auth)
// ─────────────────────────────────────────────

#[tokio::test]
async fn test_proxy_auth_none_allows_anonymous() {
    let mut server = Server::new_async().await;
    server
        .mock("GET", "/public")
        .with_status(200)
        .with_body("ok")
        .create_async()
        .await;

    let url = format!("{}/public", server.url());
    let auth = make_auth();
    let mut cache = temp_cache("auth_none", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff(&url, AuthMode::None, true, None);

    let r = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client).await;
    assert!(r.is_ok(), "Auth::None debe permitir acceso anónimo");
}

#[tokio::test]
async fn test_proxy_auth_api_key_valid_allows() {
    let mut server = Server::new_async().await;
    server
        .mock("GET", "/private")
        .with_status(200)
        .with_body("secret")
        .create_async()
        .await;

    let url = format!("{}/private", server.url());
    let auth = make_auth();
    let mut cache = temp_cache("auth_key_ok", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff(&url, AuthMode::ApiKey, true, None);

    let r = handle_proxy_request(&cfg, Some("api-key-valid"), None, &auth, &mut cache, &client).await;
    assert!(r.is_ok());
    assert_eq!(r.unwrap().status, 200);
}

#[tokio::test]
async fn test_proxy_auth_api_key_invalid_rejects() {
    let auth = make_auth();
    let mut cache = temp_cache("auth_key_bad", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff("http://localhost:19999/x", AuthMode::ApiKey, true, None);

    let r = handle_proxy_request(&cfg, Some("wrong-key"), None, &auth, &mut cache, &client).await;
    assert!(r.is_err(), "Key inválida debe rechazarse");
}

#[tokio::test]
async fn test_proxy_auth_api_key_missing_rejects() {
    let auth = make_auth();
    let mut cache = temp_cache("auth_key_missing", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff("http://localhost:19999/x", AuthMode::ApiKey, true, None);

    let r = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client).await;
    assert!(r.is_err(), "Ausencia de x-api-key debe rechazarse");
}

#[tokio::test]
async fn test_proxy_auth_user_password_valid_session_allows() {
    let mut server = Server::new_async().await;
    server
        .mock("GET", "/dashboard")
        .with_status(200)
        .with_body("panel")
        .create_async()
        .await;

    let url = format!("{}/dashboard", server.url());
    let mut auth = make_auth();
    let token = auth.login("alice", "secret123").unwrap();
    let mut cache = temp_cache("auth_sess_ok", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff(&url, AuthMode::UserPassword, true, None);

    let r = handle_proxy_request(&cfg, None, Some(&token), &auth, &mut cache, &client).await;
    assert!(r.is_ok());
}

#[tokio::test]
async fn test_proxy_auth_user_password_invalid_session_rejects() {
    let auth = make_auth();
    let mut cache = temp_cache("auth_sess_bad", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff("http://localhost:19999/x", AuthMode::UserPassword, true, None);

    let r = handle_proxy_request(&cfg, None, Some("fake-token"), &auth, &mut cache, &client).await;
    assert!(r.is_err());
}

/// No se puede usar session-token en una ruta que exige x-api-key.
#[tokio::test]
async fn test_proxy_auth_wrong_mode_session_for_api_key_route_rejects() {
    let mut auth = make_auth();
    let token = auth.login("alice", "secret123").unwrap();
    let mut cache = temp_cache("wrong_mode", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff("http://localhost:19999/x", AuthMode::ApiKey, true, None);

    let r = handle_proxy_request(&cfg, None, Some(&token), &auth, &mut cache, &client).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_proxy_invalid_scheme_fails_before_auth() {
    let auth = make_auth();
    let mut cache = temp_cache("bad_scheme", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();
    let cfg = eff("ftp://example.com/file", AuthMode::None, true, None);

    let r = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client).await;
    assert!(r.is_err());
}

// ─────────────────────────────────────────────
//  3. Auth manager — flujos de integración
// ─────────────────────────────────────────────

#[tokio::test]
async fn test_auth_full_session_lifecycle() {
    let mut auth = make_auth();

    let token = auth.login("alice", "secret123").expect("login debe funcionar");
    assert!(!token.is_empty());

    let user = auth.validate_session(&token).expect("sesión debe ser válida");
    assert_eq!(user, "alice");

    auth.logout(&token);
    assert!(
        auth.validate_session(&token).is_err(),
        "Tras logout el token debe ser inválido"
    );
}

#[tokio::test]
async fn test_auth_runtime_api_key_registration() {
    let mut auth = AuthManager::new(3600);
    auth.register_api_key("runtime-key-xyz", "bob");

    assert_eq!(auth.validate_api_key("runtime-key-xyz").unwrap(), "bob");
    assert!(auth.validate_api_key("other-key").is_err());
}

#[tokio::test]
async fn test_auth_cleanup_only_removes_expired_sessions() {
    let mut auth_short = AuthManager::new(0);
    auth_short.register_user("u", "p");
    let expired_token = auth_short.login("u", "p").unwrap();

    let mut auth_long = AuthManager::new(3600);
    auth_long.register_user("u", "p");
    let valid_token = auth_long.login("u", "p").unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    auth_short.cleanup_sessions();
    auth_long.cleanup_sessions();

    assert!(
        auth_short.validate_session(&expired_token).is_err(),
        "Sesión expirada debe eliminarse tras cleanup"
    );
    assert!(
        auth_long.validate_session(&valid_token).is_ok(),
        "Sesión vigente no debe eliminarse"
    );
}

// ─────────────────────────────────────────────
//  4. Cache multi-dominio — cuotas independientes
// ─────────────────────────────────────────────

#[tokio::test]
async fn test_cache_multidomain_independent_quotas() {
    let mut cache = temp_cache("multidomain", 1024 * 1024, Policy::Lru);

    cache.set_domain_config("big.com", 1024, Policy::Lru).await.unwrap();
    cache.set_domain_config("tiny.com", 10, Policy::Lru).await.unwrap();

    cache.put("big.com", "k", &vec![0u8; 512], "app/octet", 200, None).await.unwrap();
    cache.put("tiny.com", "k", b"hi", "text/plain", 200, None).await.unwrap();

    let stats = cache.stats();
    let big  = stats.by_domain.iter().find(|d| d.domain == "big.com").unwrap();
    let tiny = stats.by_domain.iter().find(|d| d.domain == "tiny.com").unwrap();

    assert_eq!(big.current_size_bytes, 512);
    assert_eq!(tiny.current_size_bytes, 2);
    assert!(tiny.current_size_bytes <= tiny.max_size_bytes);
}

#[tokio::test]
async fn test_cache_shrink_quota_evicts_until_fits() {
    let mut cache = temp_cache("shrink_mid", 1024 * 1024, Policy::Lru);
    cache.set_domain_config("d.com", 30, Policy::Lru).await.unwrap();

    cache.put("d.com", "a", b"0123456789", "t/p", 200, None).await.unwrap();
    cache.put("d.com", "b", b"0123456789", "t/p", 200, None).await.unwrap();
    cache.put("d.com", "c", b"0123456789", "t/p", 200, None).await.unwrap();

    let s1 = cache.stats();
    assert_eq!(s1.total_size_bytes, 30);

    cache.set_domain_config("d.com", 15, Policy::Lru).await.unwrap();
    let s2 = cache.stats();
    assert!(s2.total_size_bytes <= 15, "Tras shrink el tamaño debe ser ≤ 15");
}

// ─────────────────────────────────────────────
//  5. Evicción por política — FIX: cuota 14 bytes
//
//  La condición del while en put() es: current + size > max
//  Con cuota=15 y entradas de 5 bytes: 10+5=15, NOT > 15 → nunca evicta.
//  Con cuota=14: 10+5=15 > 14 → sí evicta al insertar la tercera entrada.
//
//  Para LRU/MRU/LFU: los timestamps se asignan en el mismo nanosegundo
//  cuando el test corre rápido. Se fijan explícitamente via bucket.entries
//  para garantizar un orden determinista.
// ─────────────────────────────────────────────

/// LRU: evicta el elemento con last_used más antiguo.
#[tokio::test]
async fn test_cache_eviction_lru_integration() {
    // cuota=14 → 10+5 > 14 → la tercera inserción fuerza evicción
    let mut cache = temp_cache("lru_integ", 14, Policy::Lru);

    cache.put("d.com", "first",  b"AAAAA", "t/p", 200, None).await.unwrap();
    cache.put("d.com", "second", b"BBBBB", "t/p", 200, None).await.unwrap();

    // Fijar last_used explícitamente para un orden determinista:
    // first=200 (más reciente), second=100 (más antiguo → víctima LRU)
    if let Some(b) = cache.buckets.get_mut("d.com") {
        if let Some(m) = b.entries.get_mut("first")  { m.entry.last_used = 200; }
        if let Some(m) = b.entries.get_mut("second") { m.entry.last_used = 100; }
    }

    // Insertar tercero → debe evictar "second" (last_used más bajo)
    cache.put("d.com", "third", b"CCCCC", "t/p", 200, None).await.unwrap();

    assert!(cache.contains("d.com", "first"),   "first tiene last_used=200, debe quedar");
    assert!(!cache.contains("d.com", "second"), "second tiene last_used=100, debe ser evictado (LRU)");
    assert!(cache.contains("d.com", "third"),   "third recién insertado debe quedar");
}

/// LFU: evicta el elemento con menor frecuencia de acceso.
#[tokio::test]
async fn test_cache_eviction_lfu_integration() {
    let mut cache = temp_cache("lfu_integ", 14, Policy::Lfu);

    cache.put("d.com", "rare",    b"AAAAA", "t/p", 200, None).await.unwrap();
    cache.put("d.com", "popular", b"BBBBB", "t/p", 200, None).await.unwrap();

    // Fijar frecuencias explícitamente: rare=1, popular=10
    if let Some(b) = cache.buckets.get_mut("d.com") {
        if let Some(m) = b.entries.get_mut("rare")    { m.entry.frequency = 1;  }
        if let Some(m) = b.entries.get_mut("popular") { m.entry.frequency = 10; }
    }

    // Insertar tercero → debe evictar "rare" (frequency más baja)
    cache.put("d.com", "new", b"CCCCC", "t/p", 200, None).await.unwrap();

    assert!(!cache.contains("d.com", "rare"),   "LFU debe evictar rare (freq=1)");
    assert!(cache.contains("d.com", "popular"), "popular tiene freq=10, debe quedar");
    assert!(cache.contains("d.com", "new"));
}

/// FIFO: evicta el elemento insertado primero (inserted_at más antiguo).
#[tokio::test]
async fn test_cache_eviction_fifo_integration() {
    let mut cache = temp_cache("fifo_integ", 14, Policy::Fifo);

    cache.put("d.com", "oldest", b"AAAAA", "t/p", 200, None).await.unwrap();
    cache.put("d.com", "middle", b"BBBBB", "t/p", 200, None).await.unwrap();

    // Fijar inserted_at para garantizar orden
    if let Some(b) = cache.buckets.get_mut("d.com") {
        if let Some(m) = b.entries.get_mut("oldest") { m.entry.inserted_at = 1; }
        if let Some(m) = b.entries.get_mut("middle") { m.entry.inserted_at = 2; }
    }

    // Tocar "oldest" — en FIFO no debe importar el uso reciente
    let _ = cache.get("d.com", "oldest").await.unwrap();

    // Insertar tercero → debe evictar "oldest" (inserted_at más bajo)
    cache.put("d.com", "newest", b"CCCCC", "t/p", 200, None).await.unwrap();

    assert!(!cache.contains("d.com", "oldest"), "FIFO debe evictar el más antiguo insertado");
    assert!(cache.contains("d.com", "newest"));
}

/// MRU: evicta el elemento con last_used más reciente.
#[tokio::test]
async fn test_cache_eviction_mru_integration() {
    let mut cache = temp_cache("mru_integ", 14, Policy::Mru);

    cache.put("d.com", "a", b"AAAAA", "t/p", 200, None).await.unwrap();
    cache.put("d.com", "b", b"BBBBB", "t/p", 200, None).await.unwrap();

    // Fijar last_used: b=999 (más reciente → víctima MRU), a=1 (más antiguo → sobrevive)
    if let Some(bkt) = cache.buckets.get_mut("d.com") {
        if let Some(m) = bkt.entries.get_mut("a") { m.entry.last_used = 1;   }
        if let Some(m) = bkt.entries.get_mut("b") { m.entry.last_used = 999; }
    }

    // Insertar tercero → debe evictar "b" (last_used más alto)
    cache.put("d.com", "c", b"CCCCC", "t/p", 200, None).await.unwrap();

    assert!(!cache.contains("d.com", "b"), "MRU debe evictar b (last_used=999)");
    assert!(cache.contains("d.com", "a"), "a tiene last_used=1, debe quedar");
    assert!(cache.contains("d.com", "c"));
}

/// Random: con cuota que obliga evicción, siempre queda exactamente 1 entrada menos.
#[tokio::test]
async fn test_cache_eviction_random_removes_one_entry() {
    // cuota=14: al insertar la tercera (10+5=15 > 14) se evicta 1
    let mut cache = temp_cache("random_integ", 14, Policy::Random);

    cache.put("d.com", "x", b"AAAAA", "t/p", 200, None).await.unwrap();
    cache.put("d.com", "y", b"BBBBB", "t/p", 200, None).await.unwrap();
    // Al insertar "z" se debe evictar alguna de las dos anteriores
    cache.put("d.com", "z", b"CCCCC", "t/p", 200, None).await.unwrap();

    let stats = cache.stats();
    let b = stats.by_domain.iter().find(|d| d.domain == "d.com").unwrap();
    assert_eq!(b.entry_count, 2,    "Random debe dejar exactamente 2 entradas");
    assert_eq!(b.current_size_bytes, 10, "Random debe dejar exactamente 10 bytes");

    // La entrada "z" siempre debe estar (la víctima es una de las previas)
    assert!(cache.contains("d.com", "z"), "La entrada recién insertada debe quedar");
}

// ─────────────────────────────────────────────
//  6. Route resolver + proxy (integración completa)
// ─────────────────────────────────────────────

#[tokio::test]
async fn test_route_resolver_wildcard_with_proxy() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/assets/style.css")
        .with_status(200)
        .with_header("content-type", "text/css")
        .with_body("body { color: red; }")
        .create_async()
        .await;

    let origin_base = server.url();
    let route = make_route("ex.com", "/assets/*", &origin_base, AuthMode::None, vec![], Some(3600));
    let compiled = vec![CompiledRoute::compile(route).unwrap()];
    let fallback = EffectiveConfig::default_for("ex.com", "", Policy::Lru, 10 * 1024 * 1024);

    let cfg = RouteResolver::resolve(&compiled, "ex.com", "/assets/style.css", fallback);

    assert_eq!(cfg.matched_pattern, Some("/assets/*".to_string()));
    assert!(cfg.origin_url.ends_with("/assets/style.css"));
    assert_eq!(cfg.ttl_seconds, Some(3600));

    let auth = make_auth();
    let mut cache = temp_cache("resolver_proxy", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();

    let resp = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();

    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"body { color: red; }");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_route_resolver_specific_beats_wildcard() {
    let mut server_specific = Server::new_async().await;
    let mock_specific = server_specific
        .mock("GET", "/api/v1/items")
        .with_status(200)
        .with_body("items from specific")
        .create_async()
        .await;

    let routes = vec![
        CompiledRoute::compile(make_route(
            "ex.com", "/*", "http://general-origin", AuthMode::None, vec![], None,
        )).unwrap(),
        CompiledRoute::compile(make_route(
            "ex.com", "/api/v1/items", &server_specific.url(), AuthMode::None, vec![], None,
        )).unwrap(),
    ];
    let fallback = EffectiveConfig::default_for("ex.com", "", Policy::Lru, 10 * 1024 * 1024);

    let cfg = RouteResolver::resolve(&routes, "ex.com", "/api/v1/items", fallback);
    assert_eq!(cfg.matched_pattern, Some("/api/v1/items".to_string()));

    let auth = make_auth();
    let mut cache = temp_cache("specific_wins", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();

    let resp = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(resp.body, b"items from specific");
    mock_specific.assert_async().await;
}

#[tokio::test]
async fn test_route_resolver_file_rules_override_ttl_and_cacheability() {
    let file_rules = vec![
        FileRule { extension: "css".into(),  ttl_seconds: 7200, cacheable: true  },
        FileRule { extension: "json".into(), ttl_seconds: 30,   cacheable: false },
    ];
    let route = make_route("ex.com", "/static/*", "http://origin", AuthMode::None, file_rules, Some(300));
    let compiled = vec![CompiledRoute::compile(route).unwrap()];
    let fallback = EffectiveConfig::default_for("ex.com", "", Policy::Lru, 10 * 1024 * 1024);

    let css_cfg  = RouteResolver::resolve(&compiled, "ex.com", "/static/app.css",  fallback.clone());
    let json_cfg = RouteResolver::resolve(&compiled, "ex.com", "/static/data.json", fallback.clone());
    let svg_cfg  = RouteResolver::resolve(&compiled, "ex.com", "/static/icon.svg",  fallback);

    assert_eq!(css_cfg.ttl_seconds,  Some(7200), "CSS debe tener TTL de su file_rule");
    assert!(css_cfg.cacheable,  "CSS debe ser cacheable");

    assert_eq!(json_cfg.ttl_seconds, Some(30), "JSON debe tener TTL de su file_rule");
    assert!(!json_cfg.cacheable, "JSON no debe ser cacheable");

    assert_eq!(svg_cfg.ttl_seconds,  Some(300), "SVG sin regla usa TTL default");
    assert!(svg_cfg.cacheable);
}

#[tokio::test]
async fn test_route_resolver_auth_mode_propagated_to_proxy() {
    let route = make_route("ex.com", "/private/*", "http://localhost:19999", AuthMode::ApiKey, vec![], None);
    let compiled = vec![CompiledRoute::compile(route).unwrap()];
    let fallback = EffectiveConfig::default_for("ex.com", "", Policy::Lru, 10 * 1024 * 1024);

    let cfg = RouteResolver::resolve(&compiled, "ex.com", "/private/data", fallback);
    assert_eq!(cfg.auth_mode, AuthMode::ApiKey);

    let auth = make_auth();
    let mut cache = temp_cache("auth_propagated", 10 * 1024 * 1024, Policy::Lru);
    let client = Client::new();

    let r = handle_proxy_request(&cfg, None, None, &auth, &mut cache, &client).await;
    assert!(r.is_err(), "Ruta con ApiKey debe rechazar acceso sin credencial");
}

#[tokio::test]
async fn test_route_resolver_no_match_returns_fallback() {
    let route = make_route("ex.com", "/api/*", "http://api", AuthMode::None, vec![], None);
    let compiled = vec![CompiledRoute::compile(route).unwrap()];
    let fallback = EffectiveConfig::default_for("ex.com", "http://fallback", Policy::Lru, 10 * 1024 * 1024);

    let cfg = RouteResolver::resolve(&compiled, "ex.com", "/other/path", fallback);
    assert_eq!(cfg.matched_pattern, None);
    assert_eq!(cfg.origin_url, "http://fallback");
}

// ─────────────────────────────────────────────
//  7. fetch_from_origin — integración directa
// ─────────────────────────────────────────────

#[tokio::test]
async fn test_fetch_from_origin_basic() {
    let mut server = Server::new_async().await;
    server
        .mock("GET", "/data")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"ok":true}"#)
        .create_async()
        .await;

    let client = Client::new();
    let url = format!("{}/data", server.url());
    let (status, ct, body) = fetch_from_origin(&client, &url).await.unwrap();

    assert_eq!(status, 200);
    assert!(ct.contains("application/json"));
    assert_eq!(body, br#"{"ok":true}"#);
}

#[tokio::test]
async fn test_fetch_from_origin_connection_refused() {
    let client = Client::new();
    let r = fetch_from_origin(&client, "http://127.0.0.1:19998/x").await;
    assert!(r.is_err(), "Conexión rechazada debe devolver error de red");
}

// ─────────────────────────────────────────────
//  8. is_cacheable_response — tabla de verdad
// ─────────────────────────────────────────────

#[test]
fn test_is_cacheable_response_truth_table() {
    let cacheable_cfg = eff("http://x/y", AuthMode::None, true,  None);
    let blocked_cfg   = eff("http://x/y", AuthMode::None, false, None);

    for status in [200u16, 203, 204, 206, 300, 301, 410] {
        assert!(
            is_cacheable_response(&cacheable_cfg, status),
            "status {} debe ser cacheable cuando la regla lo permite", status
        );
        assert!(
            !is_cacheable_response(&blocked_cfg, status),
            "status {} NO debe cachearse cuando la regla lo bloquea", status
        );
    }

    for status in [400u16, 401, 403, 404, 500, 502, 503] {
        assert!(
            !is_cacheable_response(&cacheable_cfg, status),
            "status {} nunca debe cachearse", status
        );
    }
}

// ─────────────────────────────────────────────
//  9. validate_url — casos límite
// ─────────────────────────────────────────────

#[test]
fn test_validate_url_edge_cases() {
    assert!(validate_url("http://example.com").is_ok());
    assert!(validate_url("https://sub.domain.io/path?q=1&r=2").is_ok());
    assert!(validate_url("http://192.168.1.1:8080/api").is_ok());

    assert!(validate_url("ftp://x.com").is_err());
    assert!(validate_url("file:///etc/passwd").is_err());
    assert!(validate_url("javascript:alert(1)").is_err());
    assert!(validate_url("http://").is_err());
    assert!(validate_url("").is_err());
    assert!(validate_url("not-a-url").is_err());
    assert!(validate_url("//no-scheme.com").is_err());
}