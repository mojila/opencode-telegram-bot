//! CLI entry pure remainder gates (slice 119, RED).
//!
//! Rust port of the pure remainder in `src/cli.ts` (the installed-runtime
//! entry; `src/index.ts` is the source-mode entry with no exports):
//! - `EXIT_SUCCESS` 0, `EXIT_RUNTIME_ERROR` 1, `EXIT_INVALID_ARGS` 2
//! - `CLI_USAGE` (usage text with the start/status/stop/config commands and
//!   the no-command-defaults-to-start + foreground + `--daemon`-installed
//!   notes)
//! - `CLI_MESSAGES` (`daemonRequiresInstalled`, `unknownServiceError`,
//!   `cleanupStale`, `cleanupInvalid`, `startSuccess`, `startAlreadyRunning`,
//!   `statusRunning`, `statusStopped`, `stopSuccess`, `stopAlreadyStopped`,
//!   `linePid`, `lineStartedAt`, `lineUptimeSec`, `lineLogFile`,
//!   `lineAppHome`)
//! - `formatServiceCleanupMessage` (`"stale"` -> stale text, `"invalid"` ->
//!   invalid text, `null` -> `null`)
//! - `formatServiceDetails` (`linePid` / `lineStartedAt` / `lineUptimeSec` /
//!   `lineLogFile` / `lineAppHome` joined by `"\n"`, uptime via
//!   `Math.max(0, Math.floor((Date.now() - Date.parse(startedAt)) / 1000))`)
//! - `runStartCommand` daemon guard (`daemon && mode !== "installed"` ->
//!   `daemonRequiresInstalled`)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `process.stdout` / `process.stderr` writes (`writeStdout`,
//!   `writeStderr`, `printUsage`, `writeOptionalLine`), `process.exitCode`,
//!   `Date.now()` clocks (arrive as explicit `now_ms` args),
//!   `Date.parse` input arrives as an already-parsed epoch-ms arg so tests
//!   stay deterministic, service-manager / runtime-bootstrap / logger I/O,
//!   `runCli` dispatch, `handleCliFailure` flushing.
//!
//! Already ported elsewhere and NOT repeated here:
//! - `src/cli/args.ts` (`parseCliArgs`; see `bot_bootstrap_gates`).
//! - `src/cli/failure-exit.ts` (`LOG_FLUSH_TIMEOUT_MS`, `EXIT_RUNTIME_ERROR`
//!   code; see `runtime_service_gates`).
//! - `src/runtime/mode.ts` (`resolveRuntimeMode`; see `stores_runtime` /
//!   `bot_bootstrap_gates`), `src/runtime/node-version.ts` and
//!   `src/runtime/paths.ts` (see `runtime_service_gates` / `stores_runtime`).
//!
//! Deviations: none intended. `lineUptimeSec` takes the already-clamped
//! non-negative seconds (the `Math.max(0, ...)` clamp lives in
//! `compute_uptime_sec`); `format_service_details` takes the same
//! precomputed `uptime_sec` so `Date.now()` never enters this module.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice119_cli_entry_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Success exit code (TS: `EXIT_SUCCESS` in `src/cli.ts`).
pub const EXIT_SUCCESS: i32 = 0;

/// Runtime-failure exit code (TS: `EXIT_RUNTIME_ERROR`).
pub const EXIT_RUNTIME_ERROR: i32 = 1;

/// Bad-arguments exit code (TS: `EXIT_INVALID_ARGS`).
pub const EXIT_INVALID_ARGS: i32 = 2;

/// CLI usage text (TS: `CLI_USAGE`).
pub const CLI_USAGE: &str = "Usage:\n  opencode-telegram [start] [--daemon] [--mode installed]\n  opencode-telegram status\n  opencode-telegram stop\n  opencode-telegram config\n\nNotes:\n  - No command defaults to start\n  - start runs in foreground by default\n  - --daemon is supported for the installed runtime only";

/// Daemon needs the installed runtime (TS: `CLI_MESSAGES.daemonRequiresInstalled`).
pub const CLI_DAEMON_REQUIRES_INSTALLED: &str = "Daemon mode is supported only for the installed runtime. Use `opencode-telegram start` for foreground source runs.";

/// Fallback service error (TS: `CLI_MESSAGES.unknownServiceError`).
pub const CLI_UNKNOWN_SERVICE_ERROR: &str = "Unknown service error.";

/// Stale state-file cleanup notice (TS: `CLI_MESSAGES.cleanupStale`).
pub const CLI_CLEANUP_STALE: &str = "Removed stale daemon state file.";

/// Invalid state-file cleanup notice (TS: `CLI_MESSAGES.cleanupInvalid`).
pub const CLI_CLEANUP_INVALID: &str = "Removed invalid daemon state file.";

/// Daemon start success (TS: `CLI_MESSAGES.startSuccess`).
pub const CLI_START_SUCCESS: &str = "OpenCode Telegram Bot daemon started.";

/// Daemon already running (TS: `CLI_MESSAGES.startAlreadyRunning`).
pub const CLI_START_ALREADY_RUNNING: &str = "OpenCode Telegram Bot daemon is already running.";

/// Service status lines (TS: `CLI_MESSAGES.statusRunning` / `statusStopped`).
pub const CLI_STATUS_RUNNING: &str = "Service status: running";
/// Service stopped line (TS: `CLI_MESSAGES.statusStopped`).
pub const CLI_STATUS_STOPPED: &str = "Service status: stopped";

/// Daemon stop success (TS: `CLI_MESSAGES.stopSuccess`).
pub const CLI_STOP_SUCCESS: &str = "OpenCode Telegram Bot daemon stopped.";

/// Daemon already stopped (TS: `CLI_MESSAGES.stopAlreadyStopped`).
pub const CLI_STOP_ALREADY_STOPPED: &str = "OpenCode Telegram Bot daemon is not running.";

/// State-file cleanup cause (TS: `cleanupReason` in `formatServiceCleanupMessage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceCleanupReason {
    Stale,
    Invalid,
}

/// Parse a cleanup reason (`"stale"` / `"invalid"`, else `None`).
pub fn parse_service_cleanup_reason(raw: &str) -> Option<ServiceCleanupReason> {
    match raw {
        "stale" => Some(ServiceCleanupReason::Stale),
        "invalid" => Some(ServiceCleanupReason::Invalid),
        _ => None,
    }
}

/// Cleanup notice for a reason, or `None` for `None` (TS: `formatServiceCleanupMessage`).
pub fn format_service_cleanup_message(
    reason: Option<ServiceCleanupReason>,
) -> Option<String> {
    match reason {
        Some(ServiceCleanupReason::Stale) => Some(CLI_CLEANUP_STALE.to_string()),
        Some(ServiceCleanupReason::Invalid) => Some(CLI_CLEANUP_INVALID.to_string()),
        None => None,
    }
}

/// `PID: {pid}` (TS: `CLI_MESSAGES.linePid`).
pub fn line_pid(pid: u32) -> String {
    format!("PID: {pid}")
}

/// `Started at: {startedAt}` (TS: `CLI_MESSAGES.lineStartedAt`).
pub fn line_started_at(started_at: &str) -> String {
    format!("Started at: {started_at}")
}

/// `Uptime: {seconds} sec` (TS: `CLI_MESSAGES.lineUptimeSec`; pass the
/// already-clamped value from [`compute_uptime_sec`]).
pub fn line_uptime_sec(seconds: i64) -> String {
    format!("Uptime: {seconds} sec")
}

/// `Log file: {path}` (TS: `CLI_MESSAGES.lineLogFile`).
pub fn line_log_file(file_path: &str) -> String {
    format!("Log file: {file_path}")
}

/// `App home: {appHome}` (TS: `CLI_MESSAGES.lineAppHome`).
pub fn line_app_home(app_home: &str) -> String {
    format!("App home: {app_home}")
}

/// Clamp an uptime delta to whole non-negative seconds
/// (TS: `Math.max(0, Math.floor((nowMs - startedAtMs) / 1000))` inside
/// `formatServiceDetails`).
pub fn compute_uptime_sec(now_ms: i64, started_at_ms: i64) -> i64 {
    (now_ms - started_at_ms).div_euclid(1000).max(0)
}

/// Five detail lines joined by `"\n"` (TS: `formatServiceDetails`).
pub fn format_service_details(
    pid: u32,
    started_at: &str,
    uptime_sec: i64,
    log_file_path: &str,
    app_home: &str,
) -> String {
    [
        line_pid(pid),
        line_started_at(started_at),
        line_uptime_sec(uptime_sec),
        line_log_file(log_file_path),
        line_app_home(app_home),
    ]
    .join("\n")
}

/// Whether `--daemon` must be rejected for this mode
/// (TS: `daemon && modeResult.mode !== "installed"` in `runStartCommand`).
pub fn daemon_requires_installed(mode: &str, daemon: bool) -> bool {
    daemon && mode != "installed"
}
