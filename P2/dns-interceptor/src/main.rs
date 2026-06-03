mod config;
mod dns;
mod geo;

use config::Config;
use dns::{build_a_response, is_managed, parse_query};
use geo::GeoResolver;
use std::net::{Ipv4Addr, UdpSocket};
use std::str::FromStr;
use tracing::{error, info, warn};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("dns_interceptor=info".parse().unwrap()),
        )
        .init();

    let config = Config::from_env();
    info!("Iniciando DNS Interceptor en {}", config.listen_addr);
    info!("Dominios gestionados: {:?}", config.managed_domains);
    info!("Zonas configuradas: {:?}", config.zone_map);

    let resolver = match GeoResolver::new(
        &config.geoip_path,
        config.zone_map.clone(),
        config.default_zone.clone(),
    ) {
        Ok(r) => r,
        Err(e) => {
            error!("No se pudo inicializar GeoResolver: {}", e);
            std::process::exit(1);
        }
    };

    let socket = UdpSocket::bind(&config.listen_addr).unwrap_or_else(|e| {
        error!("No se pudo hacer bind en {}: {}", config.listen_addr, e);
        std::process::exit(1);
    });

    info!("Escuchando en UDP {}", config.listen_addr);

    let mut buf = [0u8; 512];

    loop {
        let (len, src) = match socket.recv_from(&mut buf) {
            Ok(v) => v,
            Err(e) => {
                error!("Error recibiendo paquete: {}", e);
                continue;
            }
        };

        let query_buf = &buf[..len];

        let query = match parse_query(query_buf) {
            Ok(q) => q,
            Err(e) => {
                warn!("Query inválida de {}: {}", src, e);
                continue;
            }
        };

        info!("Query de {} → '{}'", src, query.name);

        if is_managed(&query.name, &config.managed_domains) {
            let zone_ip = resolver.resolve_zone(src.ip());
            info!(
                "  → dominio gestionado, zona: {} (IP cliente: {})",
                zone_ip, src.ip()
            );

            let ip = match Ipv4Addr::from_str(&zone_ip) {
                Ok(ip) => ip,
                Err(_) => {
                    error!("IP de zona inválida: {}", zone_ip);
                    continue;
                }
            };

            let response = build_a_response(&query, ip);
            if let Err(e) = socket.send_to(&response, src) {
                error!("Error enviando respuesta a {}: {}", src, e);
            }
        } else {
            // Forwarding al upstream DNS
            info!("  → forwarding a {}", config.upstream_dns);
            match forward_to_upstream(query_buf, &config.upstream_dns) {
                Ok(resp) => {
                    if let Err(e) = socket.send_to(&resp, src) {
                        error!("Error enviando respuesta forwarded a {}: {}", src, e);
                    }
                }
                Err(e) => {
                    error!("Error en forwarding DNS: {}", e);
                }
            }
        }
    }
}

fn forward_to_upstream(query: &[u8], upstream: &str) -> std::io::Result<Vec<u8>> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_read_timeout(Some(std::time::Duration::from_secs(3)))?;
    sock.send_to(query, upstream)?;
    let mut buf = [0u8; 512];
    let (len, _) = sock.recv_from(&mut buf)?;
    Ok(buf[..len].to_vec())
}
