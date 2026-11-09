//! Daemon/side-by-side guard (T10, structure only).
//!
//! The Rust binary runs next to the live TS bot (PID 8971) and must never
//! disturb it by default:
//! - DB: isolated path only (`OTB_DB_PATH`, else
//!   `<app_home>/.otb-bot-test.db`); live `settings.json` / `opencode.db`
//!   are never written.
//! - Health: port `0` (disabled) unless `BOT_HEALTH_PORT` is explicitly
//!   set; binds loopback only.
//! - Polling: opt-in only (`OTB_ENABLE_POLLING=1`); the live
//!   `TELEGRAM_BOT_TOKEN` is read only for that opt-in poll loop.
//!
//! Pure predicates only. Env reads, binds, file writes, and polling stay
//! with the caller (`store`, `health`, `telegram`).

use std::path::Path;

/// Live state files the Rust side must never write.
pub const LIVE_STATE_FILES: &[&str] = &["settings.json", "opencode.db"];

/// Whether `db_path` is the isolated cache DB (never a live path).
pub fn db_path_is_isolated(db_path: &Path) -> bool {
    db_path.file_name().and_then(|name| name.to_str())
        == Some(crate::store::ISOLATED_DB_FILE)
}

/// Whether a DB path targets a live state file that must stay untouched.
pub fn db_path_is_live(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| LIVE_STATE_FILES.contains(&name))
}

/// Health bind is side-by-side safe when disabled (`0`) or explicitly
/// opted in via `BOT_HEALTH_PORT` (TS default: off, 3100 in a container).
pub fn health_bind_is_safe(port: u16, health_env_explicit: bool) -> bool {
    port == 0 || health_env_explicit
}

/// Polling is safe unless it would run against the live bot token.
pub fn polling_guard_is_safe(polling_enabled: bool, uses_live_token: bool) -> bool {
    !(polling_enabled && uses_live_token)
}

/// One side-by-side verdict over the three default-off surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideBySideReport {
    pub db_isolated: bool,
    pub health_safe: bool,
    pub polling_safe: bool,
}

/// `true` only when every surface is side-by-side safe.
pub fn side_by_side_safe(report: &SideBySideReport) -> bool {
    report.db_isolated && report.health_safe && report.polling_safe
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn scaffold_defaults_are_side_by_side_safe() {
        let isolated = PathBuf::from("/tmp/app-home").join(crate::store::ISOLATED_DB_FILE);
        assert!(db_path_is_isolated(&isolated));
        assert!(!db_path_is_live(&isolated));
        assert!(!db_path_is_live(&PathBuf::from("/tmp/app-home/custom.db")));
        assert!(db_path_is_live(&PathBuf::from("/live/settings.json")));
        assert!(db_path_is_live(&PathBuf::from("/live/opencode.db")));

        assert!(health_bind_is_safe(0, false));
        assert!(!health_bind_is_safe(3100, false));
        assert!(health_bind_is_safe(3100, true));

        assert!(polling_guard_is_safe(false, false));
        assert!(polling_guard_is_safe(true, false));
        assert!(!polling_guard_is_safe(true, true));

        let report = SideBySideReport {
            db_isolated: true,
            health_safe: true,
            polling_safe: true,
        };
        assert!(side_by_side_safe(&report));
        let unsafe_report = SideBySideReport {
            db_isolated: false,
            ..report
        };
        assert!(!side_by_side_safe(&unsafe_report));
    }
}
