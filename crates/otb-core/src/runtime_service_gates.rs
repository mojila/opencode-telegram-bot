//! Runtime service / health-port / node-version / logger-level gates (slice 96, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/runtime/node-version.ts` (`LOWEST_SUPPORTED_MAJOR` 22,
//!   `MINIMUM_MINOR_BY_MAJOR` `{22: 14, 23: 6}`,
//!   `SUPPORTED_NODE_VERSIONS`, `getUnsupportedNodeVersionMessage`
//!   major/minor parse + minimum-minor gate + two-line message)
//! - `src/runtime/health-port.ts` (`BOT_HEALTH_PORT_ENV`,
//!   `CONTAINER_DEFAULT_HEALTH_PORT` 3100, `MAX_PORT` 65535,
//!   `resolveHealthPort` trim / `^\d+$` / `<= 65535` gate with the
//!   container-aware default)
//! - `src/runtime/service/env.ts` (`SERVICE_CHILD_ENV_KEY`,
//!   `SERVICE_STATE_PATH_ENV_KEY`, `buildServiceChildEnv` spread +
//!   overwrite, `isServiceChildProcess` `=== "1"` check,
//!   `getServiceStateFilePathFromEnv` blank gate)
//! - `src/runtime/service/manager.ts` pure subset
//!   (`SERVICE_STATE_FILE_NAME`, `PROCESS_EXIT_POLL_MS` 100,
//!   `DEFAULT_STOP_TIMEOUT_MS` 5000, `sanitizeTimestampForFile`
//!   colon + first-`T` rewrite, `isValidServiceState` pid/mode gates)
//! - `src/utils/logger.ts` pure subset (`DEFAULT_LOG_LEVEL` `"info"`,
//!   `DEFAULT_LOG_RETENTION` 10, `LOG_LEVELS` debug < info < warn < error,
//!   `normalizeLogLevel` membership fallback, `parsePositiveInteger`
//!   `parseInt` / `<= 0` fallback)
//! - `src/cli/failure-exit.ts` (`LOG_FLUSH_TIMEOUT_MS` 1000,
//!   `EXIT_RUNTIME_ERROR` 1)
//!
//! Already ported elsewhere and NOT repeated here:
//! - `src/runtime/mode.ts`, `src/runtime/container.ts`,
//!   `src/runtime/paths.ts` (see `stores_runtime`).
//! - `src/cli/args.ts` (`parseCliArgs`; see `bot_bootstrap_gates`).
//! - `LOG_LEVELS`-driven `shouldLog` against live `process.env`,
//!   `formatPrefix`/`withPrefix` timestamp rendering, file logging,
//!   `safeBackgroundTask`, `handleCliFailure` flushing/exit, and all
//!   `fs`/process/network I/O stay with the caller.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `process.env` / `process.versions.node` reads (env input arrives as
//!   explicit `Option<&str>` args), `fs` state-file reads/writes,
//!   `process.kill` liveness, `fetch` health probes, `setTimeout` flush
//!   races, `process.exit`.
//!
//! Deviations: none intended. `Number.parseInt(value, 10)` leading-digit
//! semantics and the `String.replace("T", "_")` first-only rewrite are
//! mirrored; parity fixtures use ASCII inputs where JS and Rust agree.
//! `path.join` stays with the caller: this module owns the log file
//! *name* only, so assertions hold on every OS.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice96_runtime_service_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Lowest Node.js major the bot supports (TS: `LOWEST_SUPPORTED_MAJOR`).
pub const LOWEST_SUPPORTED_NODE_MAJOR: u64 = 22;

/// Human-readable supported range (TS: `SUPPORTED_NODE_VERSIONS`).
pub const SUPPORTED_NODE_VERSIONS: &str = "22.14+, 23.6+, or 24+";

/// Env key for the health endpoint port (TS: `BOT_HEALTH_PORT_ENV`).
pub const BOT_HEALTH_PORT_ENV: &str = "BOT_HEALTH_PORT";

/// Health port default inside a container (TS: `CONTAINER_DEFAULT_HEALTH_PORT`).
pub const CONTAINER_DEFAULT_HEALTH_PORT: u16 = 3100;

/// Largest valid TCP port (TS: `MAX_PORT`).
pub const MAX_PORT: u32 = 65535;

/// Service state file name (TS: `SERVICE_STATE_FILE_NAME`).
pub const SERVICE_STATE_FILE_NAME: &str = "bot-service.json";

/// Exit-poll interval for service shutdown (TS: `PROCESS_EXIT_POLL_MS`).
pub const PROCESS_EXIT_POLL_MS: u64 = 100;

/// Default service stop timeout (TS: `DEFAULT_STOP_TIMEOUT_MS`).
pub const DEFAULT_STOP_TIMEOUT_MS: u64 = 5000;

/// Env key marking a service child (TS: `SERVICE_CHILD_ENV_KEY`).
pub const SERVICE_CHILD_ENV_KEY: &str = "OPENCODE_TELEGRAM_SERVICE_CHILD";

/// Env key carrying the state file path (TS: `SERVICE_STATE_PATH_ENV_KEY`).
pub const SERVICE_STATE_PATH_ENV_KEY: &str = "OPENCODE_TELEGRAM_SERVICE_STATE_PATH";

/// Service-child marker value (TS: the `"1"` literal).
pub const SERVICE_CHILD_ENV_VALUE: &str = "1";

/// Fallback log level (TS: `DEFAULT_LOG_LEVEL`).
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// Fallback log retention count (TS: `DEFAULT_LOG_RETENTION`).
pub const DEFAULT_LOG_RETENTION: u32 = 10;

/// Logger flush race budget for CLI failures (TS: `LOG_FLUSH_TIMEOUT_MS`).
pub const LOG_FLUSH_TIMEOUT_MS: u64 = 1000;

/// Process exit code for CLI failures (TS: `EXIT_RUNTIME_ERROR`).
pub const EXIT_RUNTIME_ERROR: i32 = 1;

/// Mirror of `Number.parseInt(text, 10)`.
///
/// Skips leading ASCII whitespace, consumes an optional `+`/`-` sign and
/// the leading run of ASCII digits, and stops at the first non-digit.
/// Returns `None` when there are no digits (JS `NaN`).
fn js_parse_int(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let mut negative = false;
    if index < bytes.len() && (bytes[index] == b'+' || bytes[index] == b'-') {
        negative = bytes[index] == b'-';
        index += 1;
    }
    let start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if start == index {
        return None;
    }
    let digits = &text[start..index];
    let magnitude: i64 = digits.parse().ok()?;
    Some(if negative { -magnitude } else { magnitude })
}

/// Error message when `version` is too old, else `None`
/// (TS: `getUnsupportedNodeVersionMessage`).
pub fn get_unsupported_node_version_message(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let raw_major = parts.next().unwrap_or("");
    let raw_minor = parts.next().unwrap_or("");
    let major = js_parse_int(raw_major)?;
    let minor = js_parse_int(raw_minor)?;

    if major >= LOWEST_SUPPORTED_NODE_MAJOR as i64 {
        let minimum_minor: Option<i64> = match major {
            22 => Some(14),
            23 => Some(6),
            _ => None,
        };
        match minimum_minor {
            None => return None,
            Some(minimum) if minor >= minimum => return None,
            _ => {}
        }
    }

    Some(
        [
            format!(
                "OpenCode Telegram Bot requires Node.js {}, but the current version is v{}.",
                SUPPORTED_NODE_VERSIONS, version
            ),
            "Update Node.js and try again: https://nodejs.org".to_string(),
        ]
        .join("\n"),
    )
}

/// Health endpoint port; `0` means off (TS: `resolveHealthPort`).
pub fn resolve_health_port(raw: Option<&str>, in_container: bool) -> u16 {
    let default_port: u16 = if in_container {
        CONTAINER_DEFAULT_HEALTH_PORT
    } else {
        0
    };
    let value = raw.map(str::trim).unwrap_or("");
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return default_port;
    }
    let port: u32 = value.parse().unwrap_or(MAX_PORT + 1);
    if port <= MAX_PORT {
        port as u16
    } else {
        default_port
    }
}

/// `true` when the env value marks a service child
/// (TS: `isServiceChildProcess`, env value passed explicitly).
pub fn is_service_child_process(env_value: Option<&str>) -> bool {
    env_value == Some(SERVICE_CHILD_ENV_VALUE)
}

/// State file path, or `None` when missing/blank
/// (TS: `getServiceStateFilePathFromEnv`, env value passed explicitly).
pub fn get_service_state_path_from_env(raw: Option<&str>) -> Option<String> {
    let value = raw?;
    if value.trim().is_empty() {
        return None;
    }
    Some(value.to_string())
}

fn upsert_entry(entries: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| k == key) {
        slot.1 = value.to_string();
    } else {
        entries.push((key.to_string(), value.to_string()));
    }
}

/// Child env entries: every base entry plus the child marker and state
/// path, which overwrite same-named base keys
/// (TS: `buildServiceChildEnv` spread).
pub fn build_service_child_entries(
    base: &[(&str, &str)],
    state_file_path: &str,
) -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = base
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    upsert_entry(&mut entries, SERVICE_CHILD_ENV_KEY, SERVICE_CHILD_ENV_VALUE);
    upsert_entry(&mut entries, SERVICE_STATE_PATH_ENV_KEY, state_file_path);
    entries
}

/// Make a timestamp file-safe (TS: `sanitizeTimestampForFile`: every `:`
/// becomes `-`, then the first `T` becomes `_`).
pub fn sanitize_timestamp_for_file(timestamp: &str) -> String {
    timestamp.replace(':', "-").replacen('T', "_", 1)
}

/// `bot-service-<sanitized-timestamp19>.log` file name
/// (TS: `createServiceLogFilePath` without the directory join, which
/// stays with the caller so tests hold on every OS).
pub fn service_log_file_name(timestamp19: &str) -> String {
    format!(
        "bot-service-{}.log",
        sanitize_timestamp_for_file(timestamp19)
    )
}

/// Minimal service-state shape (TS: `BotServiceState` fields read by
/// `isValidServiceState`; `pid` is `f64` to mirror the JS `number`
/// integer check).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServiceStateShape {
    pub pid: Option<f64>,
    pub started_at: Option<String>,
    pub log_file_path: Option<String>,
    pub mode: Option<String>,
}

/// Integer `pid > 0` plus non-empty `startedAt`/`logFilePath` and
/// `mode === "daemon"` (TS: `isValidServiceState`).
pub fn is_valid_service_state(state: &ServiceStateShape) -> bool {
    let pid_ok = match state.pid {
        Some(pid) if pid.is_finite() && pid.fract() == 0.0 && pid > 0.0 => true,
        _ => false,
    };
    if !pid_ok {
        return false;
    }
    let started_ok = state.started_at.as_deref().is_some_and(|s| !s.is_empty());
    if !started_ok {
        return false;
    }
    let log_ok = state
        .log_file_path
        .as_deref()
        .is_some_and(|s| !s.is_empty());
    if !log_ok {
        return false;
    }
    state.mode.as_deref() == Some("daemon")
}

/// Known level kept, anything else falls back to `"info"`
/// (TS: `normalizeLogLevel`).
pub fn normalize_log_level(value: &str) -> &'static str {
    match value {
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        _ => DEFAULT_LOG_LEVEL,
    }
}

/// `parseInt` leading digits when `> 0`, else the default
/// (TS: `parsePositiveInteger`).
pub fn parse_positive_integer(raw: Option<&str>, default_value: u64) -> u64 {
    let value = match raw {
        None => return default_value,
        Some(v) => v,
    };
    if value.is_empty() {
        return default_value;
    }
    match js_parse_int(value) {
        Some(parsed) if parsed > 0 => parsed as u64,
        _ => default_value,
    }
}

/// Severity rank: debug 0, info 1, warn 2, error 3; `None` when unknown
/// (TS: `LOG_LEVELS`).
pub fn log_level_rank(level: &str) -> Option<u8> {
    match level {
        "debug" => Some(0),
        "info" => Some(1),
        "warn" => Some(2),
        "error" => Some(3),
        _ => None,
    }
}

/// `true` when `level` meets the configured floor
/// (TS: `shouldLog`: `LOG_LEVELS[level] >= LOG_LEVELS[configured]`).
pub fn should_log(level: &str, configured: &str) -> bool {
    match (log_level_rank(level), log_level_rank(configured)) {
        (Some(left), Some(right)) => left >= right,
        _ => false,
    }
}
