//! Expone los módulos como librería para que los tests de integración en
//! `tests/*.rs` puedan importarlos. El binario `zonal-cache` (src/main.rs)
//! sigue siendo el entry point en producción.

pub mod algorithms;
pub mod auth;
pub mod cache;
pub mod config;
pub mod firestore;
pub mod proxy;
pub mod route_resolver;
