//! Logging init: `tracing-subscriber` EnvFilter (T2).
//!
//! Node parity (`src/utils/logger.ts`): `LOG_LEVEL` is one of
//! `debug|info|warn|error` (default `info`); unknown values fall back to
//! `info`.

use tracing_subscriber::EnvFilter;

/// Build the filter directive for a normalized level string.
pub fn filter_for_level(log_level: &str) -> String {
    let normalized = log_level.trim().to_ascii_lowercase();
    let level = match normalized.as_str() {
        "debug" | "info" | "warn" | "error" => normalized,
        _ => crate::config::DEFAULT_LOG_LEVEL.to_string(),
    };
    format!("otb_bot={level},otb_core={level}")
}

/// Initialize global tracing with an `EnvFilter`.
///
/// `RUST_LOG` wins when set (standard `tracing-subscriber` behavior);
/// otherwise the app `LOG_LEVEL` applies to the `otb_bot`/`otb_core`
/// targets. Safe to call twice (tests, re-init): extra calls are ignored.
pub fn init_logging(log_level: &str) {
    let directive = filter_for_level(log_level);
    let filter =
        EnvFilter::try_new(std::env::var("RUST_LOG").unwrap_or(directive)).unwrap_or_else(|_| {
            EnvFilter::new(format!(
                "otb_bot={},otb_core={}",
                crate::config::DEFAULT_LOG_LEVEL,
                crate::config::DEFAULT_LOG_LEVEL
            ))
        });
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init();
}
