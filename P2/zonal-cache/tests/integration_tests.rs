//! Tests de integración del flujo proxy → cache → origen.

use mockito::{Matcher, Server};
use reqwest::Client;
use std::env;
use uuid::Uuid;
use zonal_cache::algorithms::Policy;
use zonal_cache::auth::AuthManager;
use zonal_cache::cache::DiskCache;
use zonal_cache::firestore::AuthMode;
use zonal_cache::proxy::handle_proxy_request;
use zonal_cache::route_resolver::EffectiveConfig;

fn temp_cache() -> DiskCache {
    let dir = env::temp_dir().join(format!("zc_it_{}", Uuid::new_v4()));
    DiskCache::new(dir.to_str().unwrap(), 1024 * 1024, Policy::Lru)
}

fn cfg(
    origin: &str,
    domain: &str,
    auth_mode: AuthMode,
    cacheable: bool,
    ttl: Option<u64>,
) -> EffectiveConfig {
    EffectiveConfig {
        domain: domain.to_string(),
        origin_url: origin.to_string(),
        policy: Policy::Lru,
        max_size_bytes: 1024 * 1024,
        ttl_seconds: ttl,
        cacheable,
        auth_mode,
        matched_pattern: Some("/*".to_string()),
    }
}

#[tokio::test]
async fn test_miss_then_hit() {
    let mut server = Server::new_async().await;
    // Origen responde una sola vez con "hello".
    let m = server
        .mock("GET", "/page")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("hello")
        .expect(1) // se exige UNA sola llamada al origen
        .create_async()
        .await;

    let url = format!("{}/page", server.url());
    let c = cfg(&url, "ex.com", AuthMode::None, true, None);
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    // 1) MISS
    let r1 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(r1.status, 200);
    assert_eq!(r1.body, b"hello");
    assert!(!r1.from_cache);

    // 2) HIT
    let r2 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(r2.body, b"hello");
    assert!(r2.from_cache);

    m.assert_async().await; // verifica que solo se llamó UNA vez al origen
}

#[tokio::test]
async fn test_ttl_expiration_triggers_refetch() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("GET", "/ttl")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("v1")
        .expect_at_least(2)
        .create_async()
        .await;

    let url = format!("{}/ttl", server.url());
    let c = cfg(&url, "ex.com", AuthMode::None, true, Some(0)); // TTL = 0 → expira casi inmediato
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    // Primera llamada: MISS, se guarda con expires_at = now + 0.
    let _ = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();

    // Esperar a que expire.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    // Segunda llamada: entrada expirada → refetch al origen.
    let r2 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(!r2.from_cache, "tras expirar, debe ser MISS");
    m.assert_async().await;
}

#[tokio::test]
async fn test_api_key_auth_required() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/private")
        .with_status(200)
        .with_body("secret")
        .expect(0) // origen NO debe ser llamado si falla auth
        .create_async()
        .await;

    let url = format!("{}/private", server.url());
    let c = cfg(&url, "ex.com", AuthMode::ApiKey, true, None);
    let mut auth = AuthManager::new(3600);
    auth.register_api_key("good-key", "user1");
    let mut cache = temp_cache();
    let client = Client::new();

    // Sin API key → falla.
    let r1 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client).await;
    assert!(r1.is_err());

    // API key inválida → falla.
    let r2 = handle_proxy_request(&c, Some("bad-key"), None, &auth, &mut cache, &client).await;
    assert!(r2.is_err());

    // API key válida → pasa.
    let r3 = handle_proxy_request(&c, Some("good-key"), None, &auth, &mut cache, &client).await;
    assert!(r3.is_ok());
}

#[tokio::test]
async fn test_user_password_session_auth() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/user-area")
        .with_status(200)
        .with_body("welcome")
        .create_async()
        .await;

    let url = format!("{}/user-area", server.url());
    let c = cfg(&url, "ex.com", AuthMode::UserPassword, true, None);
    let mut auth = AuthManager::new(3600);
    auth.register_user("alice", "wonderland");
    let token = auth.login("alice", "wonderland").unwrap();
    let mut cache = temp_cache();
    let client = Client::new();

    // Token correcto → pasa.
    let r_ok = handle_proxy_request(&c, None, Some(&token), &auth, &mut cache, &client).await;
    assert!(r_ok.is_ok());

    // Sin token → falla.
    let r_no = handle_proxy_request(&c, None, None, &auth, &mut cache, &client).await;
    assert!(r_no.is_err());

    // API key en vez de sesión cuando el modo es USER_PASSWORD → falla.
    auth_with_api_key_should_fail_user_password_route(&c, &mut auth, &mut cache, &client).await;
}

async fn auth_with_api_key_should_fail_user_password_route(
    c: &EffectiveConfig,
    auth: &mut AuthManager,
    cache: &mut DiskCache,
    client: &Client,
) {
    auth.register_api_key("k", "user");
    let r = handle_proxy_request(c, Some("k"), None, auth, cache, client).await;
    assert!(
        r.is_err(),
        "API key no debe valer en una ruta USER_PASSWORD"
    );
}

#[tokio::test]
async fn test_non_cacheable_rule_skips_storage() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("GET", "/uncacheable")
        .with_status(200)
        .with_body("dynamic")
        .expect_at_least(2)
        .create_async()
        .await;

    let url = format!("{}/uncacheable", server.url());
    let c = cfg(&url, "ex.com", AuthMode::None, false, None); // cacheable = false
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    let r1 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    let r2 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(!r1.from_cache);
    assert!(!r2.from_cache, "no cacheable → siempre MISS");
    m.assert_async().await;
}

#[tokio::test]
async fn test_binary_payload_preserved() {
    let mut server = Server::new_async().await;
    let payload: Vec<u8> = vec![0, 1, 2, 0xff, 0xfe, 0x80, 0x81, 0x00, 0x42];
    let _m = server
        .mock("GET", "/img.png")
        .with_status(200)
        .with_header("content-type", "image/png")
        .with_body(payload.clone())
        .create_async()
        .await;

    let url = format!("{}/img.png", server.url());
    let c = cfg(&url, "ex.com", AuthMode::None, true, None);
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    let r1 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(r1.body, payload);
    assert_eq!(r1.content_type, "image/png");

    // Segunda llamada desde cache: bytes idénticos.
    let r2 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(r2.from_cache);
    assert_eq!(r2.body, payload);
}

#[tokio::test]
async fn test_origin_5xx_not_cached() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/err")
        .with_status(500)
        .with_body("oops")
        .expect_at_least(2)
        .create_async()
        .await;

    let url = format!("{}/err", server.url());
    let c = cfg(&url, "ex.com", AuthMode::None, true, None);
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    let r1 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(r1.status, 500);
    let r2 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(!r2.from_cache, "5xx no debe cachearse");
}

#[tokio::test]
async fn test_origin_404_not_cached() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/missing")
        .with_status(404)
        .with_body("nope")
        .expect_at_least(2)
        .create_async()
        .await;

    let url = format!("{}/missing", server.url());
    let c = cfg(&url, "ex.com", AuthMode::None, true, None);
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    let _ = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    let r2 = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert!(!r2.from_cache);
}

#[tokio::test]
async fn test_two_domains_independent_caches() {
    // Mismo path, distintos dominios → entradas independientes.
    let mut s1 = Server::new_async().await;
    let mut s2 = Server::new_async().await;
    let _m1 = s1
        .mock("GET", "/")
        .with_status(200)
        .with_body("one")
        .create_async()
        .await;
    let _m2 = s2
        .mock("GET", "/")
        .with_status(200)
        .with_body("two")
        .create_async()
        .await;

    let c1 = cfg(
        &format!("{}/", s1.url()),
        "a.com",
        AuthMode::None,
        true,
        None,
    );
    let c2 = cfg(
        &format!("{}/", s2.url()),
        "b.com",
        AuthMode::None,
        true,
        None,
    );
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    let r1 = handle_proxy_request(&c1, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    let r2 = handle_proxy_request(&c2, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(r1.body, b"one");
    assert_eq!(r2.body, b"two");

    // Cada uno desde su propio bucket.
    let stats = cache.stats();
    assert_eq!(stats.by_domain.len(), 2);
}

#[tokio::test]
async fn test_query_with_matcher() {
    // Validar que pasamos query strings al origen correctamente.
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", Matcher::Regex(r"^/api\?q=.*$".to_string()))
        .with_status(200)
        .with_body("ok")
        .create_async()
        .await;

    let url = format!("{}/api?q=hello", server.url());
    let c = cfg(&url, "ex.com", AuthMode::None, true, None);
    let auth = AuthManager::new(3600);
    let mut cache = temp_cache();
    let client = Client::new();

    let r = handle_proxy_request(&c, None, None, &auth, &mut cache, &client)
        .await
        .unwrap();
    assert_eq!(r.status, 200);
}
