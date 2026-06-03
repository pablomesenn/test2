use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use url::Url;
use zonal_cache::algorithms::Policy;
use zonal_cache::auth::AuthManager;
use zonal_cache::cache::DiskCache;
use zonal_cache::config::Config;
use zonal_cache::firestore::FirestoreClient;
use zonal_cache::proxy::{self, handle_proxy_request};
use zonal_cache::route_resolver::{CompiledRoute, EffectiveConfig, RouteResolver};

type BoxedBody = Full<Bytes>;
type SharedCache = Arc<Mutex<DiskCache>>;
type SharedAuth = Arc<Mutex<AuthManager>>;
type SharedClient = Arc<Client>;
type SharedFirestore = Arc<FirestoreClient>;

/// Cache en memoria de las rutas leídas de Firestore.
/// Evita pegarle a Firestore en cada request.
struct RouteCache {
    routes: Vec<CompiledRoute>,
    fetched_at: u64,
    ttl_secs: u64,
}

impl RouteCache {
    fn new(ttl_secs: u64) -> Self {
        Self {
            routes: vec![],
            fetched_at: 0,
            ttl_secs,
        }
    }
    fn is_stale(&self, now: u64) -> bool {
        now.saturating_sub(self.fetched_at) >= self.ttl_secs
    }
}

type SharedRoutes = Arc<Mutex<RouteCache>>;

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn string_body(s: impl Into<String>) -> BoxedBody {
    Full::new(Bytes::from(s.into()))
}

fn bytes_body(b: Vec<u8>) -> BoxedBody {
    Full::new(Bytes::from(b))
}

fn json_response(status: StatusCode, value: serde_json::Value) -> Response<BoxedBody> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(string_body(value.to_string()))
        .unwrap()
}

fn text_response(status: StatusCode, msg: impl Into<String>) -> Response<BoxedBody> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(string_body(msg))
        .unwrap()
}

/// Refresca el cache de rutas si está vencido. Solo bloquea Firestore una vez por TTL.
async fn refresh_routes_if_stale(domain: &str, routes: SharedRoutes, firestore: SharedFirestore) {
    let now = now_ts();
    let needs_refresh = {
        let guard = routes.lock().await;
        guard.is_stale(now)
    };
    if !needs_refresh || !firestore.is_connected() {
        return;
    }
    match firestore.list_routes_for_domain(domain).await {
        Ok(docs) => {
            let compiled: Vec<CompiledRoute> = docs
                .into_iter()
                .filter_map(|d| match CompiledRoute::compile(d) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        warn!("Ruta inválida ignorada: {}", e);
                        None
                    }
                })
                .collect();
            let mut guard = routes.lock().await;
            guard.routes = compiled;
            guard.fetched_at = now;
            info!(
                "Rutas recargadas desde Firestore para {}: {} reglas",
                domain,
                guard.routes.len()
            );
        }
        Err(e) => warn!("No se pudieron recargar rutas para {}: {}", domain, e),
    }
}

/// Extrae el dominio sobre el que aplica el request:
/// - Si vino el header `host`, lo usa (típico cuando el cliente entra al cache directamente).
/// - Si no, intenta sacarlo del query `url=`.
/// - Como último recurso, usa el `fallback_domain` del config.
fn resolve_request_domain(
    host_header: Option<&str>,
    url_param: Option<&str>,
    fallback: &str,
) -> String {
    if let Some(host) = host_header {
        // Quitar puerto si viene.
        let clean = host.split(':').next().unwrap_or(host).trim();
        if !clean.is_empty() && clean != "localhost" && clean != "127.0.0.1" {
            return clean.to_string();
        }
    }
    if let Some(u) = url_param {
        if let Ok(parsed) = Url::parse(u) {
            if let Some(h) = parsed.host_str() {
                return h.to_string();
            }
        }
    }
    fallback.to_string()
}

#[allow(clippy::too_many_arguments)]
async fn handle_request(
    req: Request<Incoming>,
    cache: SharedCache,
    auth: SharedAuth,
    client: SharedClient,
    firestore: SharedFirestore,
    routes: SharedRoutes,
    cfg: Arc<Config>,
) -> Result<Response<BoxedBody>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let headers = req.headers().clone();
    info!("Request: {} {}", method, path);

    // ----- Health -----
    if path == "/health" {
        let fs_status = if firestore.is_connected() {
            "connected"
        } else {
            "disconnected"
        };
        return Ok(json_response(
            StatusCode::OK,
            serde_json::json!({
                "status": "ok",
                "firestore": fs_status,
            }),
        ));
    }

    // ----- Stats -----
    if path == "/cache/stats" {
        let cache_guard = cache.lock().await;
        let s = cache_guard.stats();
        let by_domain: Vec<_> = s
            .by_domain
            .iter()
            .map(|d| {
                serde_json::json!({
                    "domain": d.domain,
                    "entries": d.entry_count,
                    "current_size_bytes": d.current_size_bytes,
                    "max_size_bytes": d.max_size_bytes,
                    "policy": d.policy,
                })
            })
            .collect();
        return Ok(json_response(
            StatusCode::OK,
            serde_json::json!({
                "total_entries": s.total_entries,
                "total_size_bytes": s.total_size_bytes,
                "default_max_size_bytes": s.default_max_size_bytes,
                "default_policy": s.default_policy,
                "by_domain": by_domain,
            }),
        ));
    }

    // ----- Reload de cache-config global -----
    if method == hyper::Method::GET && path == "/cache/reload" {
        match firestore.get_cache_config().await {
            Ok(config_doc) => {
                let mut cache_guard = cache.lock().await;
                cache_guard.default_max_size_bytes = config_doc.max_size_bytes;
                cache_guard.default_policy = Policy::from_str(&config_doc.policy);
                info!(
                    "Config global recargada: {} bytes, política {}",
                    config_doc.max_size_bytes, config_doc.policy
                );
                return Ok(json_response(
                    StatusCode::OK,
                    serde_json::json!({
                        "status": "reloaded",
                        "max_size_bytes": config_doc.max_size_bytes,
                        "policy": config_doc.policy,
                        "ttl_seconds": config_doc.ttl_seconds,
                    }),
                ));
            }
            Err(e) => {
                return Ok(text_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("Error: {}", e),
                ))
            }
        }
    }

    // ----- Login -----
    if method == hyper::Method::POST && path == "/auth/login" {
        let body_bytes = req
            .into_body()
            .collect()
            .await
            .unwrap_or_default()
            .to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes);

        let json: serde_json::Value = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(_) => return Ok(text_response(StatusCode::BAD_REQUEST, "Invalid JSON")),
        };

        let user = json.get("user").and_then(|v| v.as_str());
        let pass = json.get("password").and_then(|v| v.as_str());
        let (user, pass) = match (user, pass) {
            (Some(u), Some(p)) => (u, p),
            _ => {
                return Ok(text_response(
                    StatusCode::BAD_REQUEST,
                    "Faltan campos 'user' o 'password'",
                ))
            }
        };

        let mut auth_guard = auth.lock().await;
        match auth_guard.login(user, pass) {
            Ok(token) => Ok(json_response(
                StatusCode::OK,
                serde_json::json!({"token": token}),
            )),
            Err(_) => Ok(text_response(
                StatusCode::UNAUTHORIZED,
                "Invalid credentials",
            )),
        }
    }
    // ----- Logout -----
    else if method == hyper::Method::POST && path == "/auth/logout" {
        let token = headers
            .get("x-session-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if token.is_empty() {
            return Ok(text_response(
                StatusCode::BAD_REQUEST,
                "Falta x-session-token",
            ));
        }
        let mut auth_guard = auth.lock().await;
        auth_guard.logout(token);
        Ok(json_response(
            StatusCode::OK,
            serde_json::json!({"status": "logged_out"}),
        ))
    }
    // ----- Proxy -----
    else if method == hyper::Method::GET && path == "/proxy" {
        // El endpoint /proxy?url=... existe para pruebas directas con curl.
        // En producción, el DNS Interceptor redirige al cache y entra por host header.
        let url_param = query
            .split('&')
            .find_map(|p| p.strip_prefix("url="))
            .map(urlencoding_decode);

        let host_header = headers.get("host").and_then(|v| v.to_str().ok());
        let domain =
            resolve_request_domain(host_header, url_param.as_deref(), &cfg.fallback_domain);

        // Refrescar rutas del dominio si está stale.
        refresh_routes_if_stale(&domain, routes.clone(), firestore.clone()).await;

        // Construir EffectiveConfig:
        //   - Si vino ?url=, usamos esa URL como origen (modo "proxy directo" para testing).
        //   - Si no, usamos el path y dejamos que RouteResolver arme la URL al origen.
        let effective = {
            let routes_guard = routes.lock().await;
            let fallback_cfg = match &url_param {
                Some(u) => EffectiveConfig::default_for(
                    &domain,
                    u,
                    default_policy_parsed(&cfg),
                    cfg.max_cache_size_bytes,
                ),
                None => {
                    // Sin URL explícita y sin reglas: no podemos saber adónde ir.
                    // Devolvemos error temprano.
                    if routes_guard.routes.is_empty() {
                        return Ok(text_response(
                            StatusCode::BAD_REQUEST,
                            "Falta parámetro url o ruta configurada en Firestore",
                        ));
                    }
                    EffectiveConfig::default_for(
                        &domain,
                        "",
                        default_policy_parsed(&cfg),
                        cfg.max_cache_size_bytes,
                    )
                }
            };

            RouteResolver::resolve(&routes_guard.routes, &domain, &path, fallback_cfg)
        };

        // Validar que tenemos una URL de origen utilizable.
        if effective.origin_url.is_empty() {
            return Ok(text_response(
                StatusCode::BAD_REQUEST,
                "No se pudo determinar URL de origen",
            ));
        }

        // Aplicar config del dominio al bucket del cache **sin** afectar otros buckets.
        {
            let mut cache_guard = cache.lock().await;
            if let Err(e) = cache_guard
                .set_domain_config(
                    &effective.domain,
                    effective.max_size_bytes,
                    effective.policy.clone(),
                )
                .await
            {
                warn!(
                    "No se pudo ajustar cuota del dominio {}: {}",
                    effective.domain, e
                );
            }
        }

        // Extraer credenciales del request.
        let api_key = headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let session_token = headers
            .get("x-session-token")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        // Si la regla exige API key y Firestore tiene la API key registrada, sincronizar AuthManager local.
        // Esto se hace ANTES de tomar locks largos para no bloquear concurrencia.
        if let Some(ref key) = api_key {
            if firestore.is_connected() {
                match firestore.get_api_key(key).await {
                    Ok(doc) if doc.active => {
                        let mut a = auth.lock().await;
                        a.register_api_key(key, &doc.user_id);
                    }
                    Ok(_) => {
                        // API key existe en Firestore pero está inactiva → rechazar inmediatamente.
                        return Ok(text_response(StatusCode::UNAUTHORIZED, "API key inactiva"));
                    }
                    Err(_) => {
                        // No está en Firestore: el AuthManager local decide si la conoce.
                    }
                }
            }
        }

        // Ahora tomar locks cortos y delegar al proxy.
        let response = {
            let auth_guard = auth.lock().await;
            let mut cache_guard = cache.lock().await;
            handle_proxy_request(
                &effective,
                api_key.as_deref(),
                session_token.as_deref(),
                &auth_guard,
                &mut cache_guard,
                &client,
            )
            .await
        };

        match response {
            Ok(pr) => {
                info!(
                    "Proxy {} → {} ({} bytes, cache={})",
                    effective.origin_url,
                    pr.status,
                    pr.body.len(),
                    pr.from_cache
                );

                let mut builder = Response::builder()
                    .status(pr.status)
                    .header("content-type", &pr.content_type)
                    .header("x-cache", if pr.from_cache { "HIT" } else { "MISS" })
                    .header("x-cache-policy", &pr.policy)
                    .header("x-cache-domain", &effective.domain);

                if let Some(ttl) = pr.ttl_seconds {
                    builder = builder.header("x-cache-ttl", ttl.to_string());
                }
                if pr.from_cache {
                    builder = builder.header("x-cache-age", pr.age_secs.to_string());
                }
                if let Some(p) = &pr.matched_pattern {
                    builder = builder.header("x-cache-pattern", p);
                }
                builder = builder.header("x-origin", &effective.origin_url);

                Ok(builder.body(bytes_body(pr.body)).unwrap())
            }
            Err(e) => {
                warn!("Proxy error: {}", e);
                let status = match &e {
                    proxy::ProxyError::Auth(_) => StatusCode::UNAUTHORIZED,
                    proxy::ProxyError::InvalidUrl(_) => StatusCode::BAD_REQUEST,
                    _ => StatusCode::BAD_GATEWAY,
                };
                Ok(text_response(status, format!("Error: {}", e)))
            }
        }
    } else {
        Ok(text_response(
            StatusCode::NOT_FOUND,
            "Endpoint no encontrado",
        ))
    }
}

// Decodificador URL-encoded simple para el query string del endpoint /proxy.
// No usamos `url::form_urlencoded` directamente para que la firma del parser
// quede explícita y testeable.
fn urlencoding_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8 as char);
                        i += 3;
                    }
                    _ => {
                        out.push('%');
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

/// Helper: parsea la política default del Config a la enum Policy.
fn default_policy_parsed(cfg: &Config) -> Policy {
    Policy::from_str(&cfg.default_policy)
}

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("zonal_cache=info".parse().unwrap()),
        )
        .init();

    let cfg = Arc::new(Config::from_env());
    info!("Iniciando Zonal Cache en {}", cfg.listen_addr);

    let firestore = Arc::new(FirestoreClient::new(&cfg.firebase_project_id).await);

    // Cargar config global inicial desde Firestore (defaults).
    let (initial_policy, initial_max_size) = if firestore.is_connected() {
        match firestore.get_cache_config().await {
            Ok(c) => {
                info!(
                    "Config inicial desde Firestore: {} bytes, política {}",
                    c.max_size_bytes, c.policy
                );
                (c.policy, c.max_size_bytes)
            }
            Err(e) => {
                warn!("Falló cache-config inicial: {}; usando defaults locales", e);
                (cfg.default_policy.clone(), cfg.max_cache_size_bytes)
            }
        }
    } else {
        warn!("Firestore no disponible; usando defaults locales");
        (cfg.default_policy.clone(), cfg.max_cache_size_bytes)
    };

    let policy = Policy::from_str(&initial_policy);
    let cache = Arc::new(Mutex::new(DiskCache::new(
        &cfg.cache_dir,
        initial_max_size,
        policy,
    )));

    let mut auth_manager = AuthManager::new(3600);
    for (key, user) in &cfg.bootstrap_api_keys {
        auth_manager.register_api_key(key, user);
        info!("API key de arranque registrada para usuario: {}", user);
    }
    let auth = Arc::new(Mutex::new(auth_manager));

    let client = Arc::new(
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Error creando HTTP client"),
    );

    let routes = Arc::new(Mutex::new(RouteCache::new(cfg.route_cache_ttl_secs)));

    let addr: SocketAddr = cfg.listen_addr.parse().unwrap_or_else(|e| {
        error!("Dirección inválida: {}", e);
        std::process::exit(1);
    });
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            error!("No se pudo hacer bind en {}: {}", addr, e);
            std::process::exit(1);
        });

    info!("Escuchando en {}", addr);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                warn!("Accept transitorio: {}", e);
                continue;
            }
        };

        let cache_c = cache.clone();
        let auth_c = auth.clone();
        let client_c = client.clone();
        let fs_c = firestore.clone();
        let routes_c = routes.clone();
        let cfg_c = cfg.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| {
                handle_request(
                    req,
                    cache_c.clone(),
                    auth_c.clone(),
                    client_c.clone(),
                    fs_c.clone(),
                    routes_c.clone(),
                    cfg_c.clone(),
                )
            });
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                error!("Connection error: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding_decode_basic() {
        assert_eq!(urlencoding_decode("hello"), "hello");
        assert_eq!(urlencoding_decode("hello%20world"), "hello world");
        assert_eq!(urlencoding_decode("a+b"), "a b");
        assert_eq!(
            urlencoding_decode("https%3A%2F%2Fx.com%2Fy"),
            "https://x.com/y"
        );
    }

    #[test]
    fn test_urlencoding_decode_malformed_keeps_percent() {
        assert_eq!(urlencoding_decode("a%ZZb"), "a%ZZb");
        assert_eq!(urlencoding_decode("a%"), "a%");
    }

    #[test]
    fn test_resolve_request_domain_prefers_host_header() {
        let d = resolve_request_domain(Some("ex.com:8080"), Some("http://other.com/x"), "fallback");
        assert_eq!(d, "ex.com");
    }

    #[test]
    fn test_resolve_request_domain_ignores_localhost() {
        let d = resolve_request_domain(Some("localhost"), Some("http://real.com/x"), "fallback");
        assert_eq!(d, "real.com");
    }

    #[test]
    fn test_resolve_request_domain_uses_url_param() {
        let d = resolve_request_domain(None, Some("http://from-url.com/path"), "fallback");
        assert_eq!(d, "from-url.com");
    }

    #[test]
    fn test_resolve_request_domain_falls_back() {
        let d = resolve_request_domain(None, None, "default");
        assert_eq!(d, "default");
    }
}
