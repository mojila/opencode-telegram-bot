//! Slice 119 parity: CLI entry pure remainder (RED).
//!
//! Mirrors the pure TS contract in `src/cli.ts`:
//! - `EXIT_SUCCESS` 0, `EXIT_RUNTIME_ERROR` 1, `EXIT_INVALID_ARGS` 2
//! - `CLI_USAGE` (start/status/stop/config commands + the three notes)
//! - `CLI_MESSAGES` daemon/unknown-service/cleanup/start/status/stop texts
//! - `formatServiceCleanupMessage` stale/invalid/null gates
//! - `linePid` / `lineStartedAt` / `lineUptimeSec` / `lineLogFile` /
//!   `lineAppHome` formatters + `Math.max(0, floor(delta / 1000))` uptime
//!   clamp + the five-line `formatServiceDetails` join
//! - `runStartCommand` daemon guard (`daemon && mode !== "installed"`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! `src/cli/args.ts` (`bot_bootstrap_gates`), `src/cli/failure-exit.ts`
//! (`runtime_service_gates`), runtime mode/paths/node-version
//! (`stores_runtime`, `bot_bootstrap_gates`, `runtime_service_gates`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `cli_entry_gates` is ported (only the constant-value tests pass).

use otb_core::cli_entry_gates::*;

// ---- constants (4) ----

#[test]
fn exit_codes_value_parity() {
    assert_eq!(EXIT_SUCCESS, 0);
    assert_eq!(EXIT_RUNTIME_ERROR, 1);
    assert_eq!(EXIT_INVALID_ARGS, 2);
}

#[test]
fn cli_usage_value_parity() {
    assert!(CLI_USAGE.contains("opencode-telegram [start] [--daemon] [--mode installed]"));
    assert!(CLI_USAGE.contains("opencode-telegram status"));
    assert!(CLI_USAGE.contains("opencode-telegram stop"));
    assert!(CLI_USAGE.contains("opencode-telegram config"));
    assert!(CLI_USAGE.contains("No command defaults to start"));
    assert!(CLI_USAGE.contains("start runs in foreground by default"));
    assert!(CLI_USAGE.contains("--daemon is supported for the installed runtime only"));
}

#[test]
fn cli_static_texts_value_parity() {
    assert_eq!(
        CLI_DAEMON_REQUIRES_INSTALLED,
        "Daemon mode is supported only for the installed runtime. Use `opencode-telegram start` for foreground source runs."
    );
    assert_eq!(CLI_UNKNOWN_SERVICE_ERROR, "Unknown service error.");
    assert_eq!(CLI_CLEANUP_STALE, "Removed stale daemon state file.");
    assert_eq!(CLI_CLEANUP_INVALID, "Removed invalid daemon state file.");
}

#[test]
fn cli_lifecycle_texts_value_parity() {
    assert_eq!(CLI_START_SUCCESS, "OpenCode Telegram Bot daemon started.");
    assert_eq!(
        CLI_START_ALREADY_RUNNING,
        "OpenCode Telegram Bot daemon is already running."
    );
    assert_eq!(CLI_STATUS_RUNNING, "Service status: running");
    assert_eq!(CLI_STATUS_STOPPED, "Service status: stopped");
    assert_eq!(CLI_STOP_SUCCESS, "OpenCode Telegram Bot daemon stopped.");
    assert_eq!(
        CLI_STOP_ALREADY_STOPPED,
        "OpenCode Telegram Bot daemon is not running."
    );
}

// ---- cleanup reason (2) ----

#[test]
fn parses_cleanup_reason_parity() {
    assert_eq!(
        parse_service_cleanup_reason("stale"),
        Some(ServiceCleanupReason::Stale)
    );
    assert_eq!(
        parse_service_cleanup_reason("invalid"),
        Some(ServiceCleanupReason::Invalid)
    );
    assert_eq!(parse_service_cleanup_reason("other"), None);
    assert_eq!(parse_service_cleanup_reason(""), None);
}

#[test]
fn cleanup_message_gate_parity() {
    assert_eq!(
        format_service_cleanup_message(Some(ServiceCleanupReason::Stale)),
        Some("Removed stale daemon state file.".to_string())
    );
    assert_eq!(
        format_service_cleanup_message(Some(ServiceCleanupReason::Invalid)),
        Some("Removed invalid daemon state file.".to_string())
    );
    assert_eq!(format_service_cleanup_message(None), None);
}

// ---- line formatters (5) ----

#[test]
fn line_pid_parity() {
    assert_eq!(line_pid(42), "PID: 42");
}

#[test]
fn line_started_at_parity() {
    assert_eq!(
        line_started_at("2026-01-01T00:00:00.000Z"),
        "Started at: 2026-01-01T00:00:00.000Z"
    );
}

#[test]
fn line_uptime_sec_parity() {
    assert_eq!(line_uptime_sec(0), "Uptime: 0 sec");
    assert_eq!(line_uptime_sec(61), "Uptime: 61 sec");
}

#[test]
fn line_paths_parity() {
    assert_eq!(line_log_file("/tmp/bot.log"), "Log file: /tmp/bot.log");
    assert_eq!(line_app_home("/home/u/.otb"), "App home: /home/u/.otb");
}

#[test]
fn uptime_clamp_parity() {
    assert_eq!(compute_uptime_sec(65_000, 0), 65);
    assert_eq!(compute_uptime_sec(1_500, 0), 1);
    assert_eq!(compute_uptime_sec(0, 10_000), 0);
}

// ---- details + daemon guard (2) ----

#[test]
fn service_details_join_parity() {
    assert_eq!(
        format_service_details(7, "2026-01-01T00:00:00.000Z", 9, "/tmp/bot.log", "/home/u/.otb"),
        "PID: 7\nStarted at: 2026-01-01T00:00:00.000Z\nUptime: 9 sec\nLog file: /tmp/bot.log\nApp home: /home/u/.otb"
    );
}

#[test]
fn daemon_guard_parity() {
    assert!(daemon_requires_installed("sources", true));
    assert!(!daemon_requires_installed("installed", true));
    assert!(!daemon_requires_installed("sources", false));
    assert!(!daemon_requires_installed("installed", false));
}
