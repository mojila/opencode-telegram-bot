//! Health endpoint: axum `GET /health` (+ `/health/live`) on 127.0.0.1 (T3).
//!
//! Node parity (`src/app/services/health-server-service.ts`,
//! `src/runtime/health-port.ts`, `src/runtime/container.ts`,
//! `.env.example`):
//! - Env `BOT_HEALTH_PORT`; empty/invalid/out-of-range means unset: on by
//!   default (3100) in a container, off (0) elsewhere. Explicit `0` disables.
//! - Binds the loopback interface only; port `0` skips serving entirely.
//! - A failed listen logs a warning and the bot runs on.
//! - `/health/live` is the Docker `HEALTHCHECK` probe path.
//! - `/health/ready` and the OpenCode dependency check stay Node-only until
//!   the OpenCode client lands; that route is absent here on purpose.
//! - No ISO `timestamp` field yet: no date-time crate is wired in. It arrives
//!   with the full payload (ready check) later.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

/// Env var carrying the health port (TS: `BOT_HEALTH_PORT_ENV`).
pub const BOT_HEALTH_PORT_ENV: &str = "BOT_HEALTH_PORT";

/// Env flag forcing container runtime (TS: `OPENCODE_TELEGRAM_CONTAINER`).
pub const CONTAINER_ENV: &str = "OPENCODE_TELEGRAM_CONTAINER";

/// Loopback host the health endpoint binds (TS: `HEALTH_HOST`).
pub const HEALTH_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// Shared handler state: bot version plus the serve start time.
#[derive(Debug, Clone)]
struct HealthState {
    version: String,
    started: Instant,
}

/// `GET /health` (+ `/health/live`): process liveness with the bot version.
async fn health(State(state): State<Arc<HealthState>>) -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "version": state.version,
        "uptimeSeconds": state.started.elapsed().as_secs(),
    }))
}

/// Router serving `GET /health` and `GET /health/live`.
pub fn router(version: String) -> Router {
    let state = Arc::new(HealthState {
        version,
        started: Instant::now(),
    });
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(health))
        .with_state(state)
}

/// Whether this process runs in a container (TS: `isContainerRuntime`).
pub fn is_container_runtime() -> bool {
    let flag = std::env::var(CONTAINER_ENV).ok();
    otb_core::stores_runtime::is_container_runtime(
        flag.as_deref(),
        std::path::Path::new("/.dockerenv").exists(),
    )
}

/// Health port from the environment (TS: `config.health.port`).
pub fn resolve_health_port_from_env() -> u16 {
    let raw = std::env::var(BOT_HEALTH_PORT_ENV).ok();
    otb_core::types_config::resolve_health_port(raw.as_deref(), is_container_runtime())
}

/// Serve on `127.0.0.1:port`. A failed bind logs and returns so the bot runs
/// on (TS: `HealthServer.start` catch path).
pub async fn serve(port: u16, version: &str) {
    let addr = SocketAddr::new(IpAddr::V4(HEALTH_HOST), port);
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::warn!(
                "[Health] Health server could not listen on {addr}; continuing without it: {err}"
            );
            return;
        }
    };
    tracing::info!("[Health] Health server listening on {addr} (/health, /health/live)");
    if let Err(err) = axum::serve(listener, router(version.to_string())).await {
        tracing::error!("[Health] Health server error: {err}");
    }
}

/// Spawn the health server as a background task; `None` when disabled (port
/// `0`). Never blocks the caller.
pub fn spawn(port: u16, version: String) -> Option<tokio::task::JoinHandle<()>> {
    if port == 0 {
        tracing::info!("[Health] Health server disabled (BOT_HEALTH_PORT is not set or 0)");
        return None;
    }
    Some(tokio::spawn(async move { serve(port, &version).await }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_port_spawns_nothing() {
        assert!(spawn(0, "test".to_string()).is_none());
    }

    /// Live probe on an OS-assigned ephemeral port: never the live/container
    /// port, never a fixed test port that could collide.
    #[tokio::test]
    async fn health_and_live_return_status_and_version() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral health port");
        let port = listener.local_addr().expect("read bound port").port();
        let server =
            tokio::spawn(
                async move { axum::serve(listener, router("test-version".to_string())).await },
            );

        for path in ["/health", "/health/live"] {
            let body: Value = reqwest::get(format!("http://127.0.0.1:{port}{path}"))
                .await
                .expect("health GET")
                .json()
                .await
                .expect("health JSON");
            assert_eq!(body["status"], "healthy");
            assert_eq!(body["version"], "test-version");
        }

        server.abort();
    }
}
