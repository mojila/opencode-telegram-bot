//! Stores / runtime / opencode-client pure subset (slice 30b, RED).
//!
//! Rust port of the pure validation/state logic behind:
//! - `src/app/stores/settings-store.ts` (`getResponseStreamingMode`,
//!   `getTtsMode`, `getPromptQueueMode`/`setPromptQueueMode` branching,
//!   `INITIAL_SETTINGS_PRESET` key/type validation, preset-wins rules)
//! - `src/app/types/scheduled-task.ts` (`cloneScheduledTask` agent backfill,
//!   cron `enabled` default)
//! - `src/app/stores/scheduled-task-store.ts` (`toggleScheduledTask` noop
//!   guard, `activationPending` marker)
//! - `src/app/services/scheduled-task-session-ignore-service.ts` (24h TTL,
//!   match + freshness gate)
//! - `src/app/services/scheduled-task-next-run-service.ts` (`isTaskDue`
//!   timestamp comparison)
//! - `src/runtime/mode.ts` (`normalizeMode`, `parseModeFromArgv`,
//!   `hasInvalidModeSyntax`, `resolveRuntimeMode`)
//! - `src/runtime/container.ts` (`isEnabledFlag`, `isContainerRuntime`)
//! - `src/app/bootstrap/app-container.ts` (heartbeat `5000ms` / every 6
//!   ticks log gate)
//! - `src/runtime/paths.ts` (`resolveAppHome` override/mode branching,
//!   settings-path join)
//! - `src/opencode/server-target.ts` (`describeOpencodeServerUrl`,
//!   switched-vs-configured resolution)
//! - `src/opencode/client.ts` (`PROBE_ROUTES`, basic-auth header,
//!   `classifyProbe`, probe URL join)
//! - `src/app/bootstrap/start-bot-app.ts` (fatal `401/404` token codes,
//!   retryable `5xx`, `60s` retry cap, exponential startup delay)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/stores_runtime_parity.rs` assert the TS
//! contract and MUST FAIL with `unimplemented!()` panics until the Rust port
//! lands (step 2).
//!
//! No FS reads/writes, no `process.env`, no network/fetch, no timers live
//! here — only pure predicates, classifiers, and small value transforms.

use std::path::Path;

// ---------------------------------------------------------------------------
// Settings: response-streaming + TTS normalization
// (`getResponseStreamingMode`, `getTtsMode`)
// ---------------------------------------------------------------------------

/// `"draft"` stays `"draft"`, everything else is `"edit"`
/// (TS: `responseStreamingMode === "draft" ? "draft" : "edit"`).
pub fn normalize_response_streaming_mode(raw: Option<&str>) -> &'static str {
    match raw {
        Some("draft") => "draft",
        _ => "edit",
    }
}

/// Stored TTS mode or `"off"` when missing/invalid
/// (TS: `currentSettings.ttsMode ?? "off"`, valid: `off/all/auto`).
pub fn normalize_tts_mode(raw: Option<&str>) -> &'static str {
    match raw {
        Some("off") => "off",
        Some("all") => "all",
        Some("auto") => "auto",
        _ => "off",
    }
}

// ---------------------------------------------------------------------------
// Settings: prompt-queue resolution
// (`getPromptQueueMode` / `setPromptQueueMode` branching by server version)
// ---------------------------------------------------------------------------

/// Effective queue mode: V1 reads the released `promptQueueEnabled` boolean
/// (`true` -> `"queue"`, else `"off"`); V2 reads its own `promptQueueMode`
/// (`"steer"` by default) and ignores the boolean.
pub fn resolve_prompt_queue_mode<'a>(
    server_version: &str,
    prompt_queue_enabled: Option<bool>,
    prompt_queue_mode: Option<&'a str>,
) -> &'a str {
    if server_version == "v2" {
        return prompt_queue_mode.unwrap_or("steer");
    }
    match prompt_queue_enabled {
        Some(true) => "queue",
        _ => "off",
    }
}

/// What `setPromptQueueMode` writes to `promptQueueEnabled`: V1 maps
/// `mode != "off"` to `Some(bool)`, V2 leaves the boolean alone (`None`).
pub fn map_set_prompt_queue_enabled(server_version: &str, mode: &str) -> Option<bool> {
    if server_version == "v2" {
        return None;
    }
    Some(mode != "off")
}

/// What `setPromptQueueMode` writes to `promptQueueMode`: V2 stores the mode,
/// V1 leaves its own mode slot alone (`None`).
pub fn map_set_prompt_queue_mode<'a>(server_version: &str, mode: &'a str) -> Option<&'a str> {
    if server_version == "v2" {
        return Some(mode);
    }
    None
}

// ---------------------------------------------------------------------------
// Settings: INITIAL_SETTINGS_PRESET validation
// (`applyInitialSettingsPreset` key/type gates)
// ---------------------------------------------------------------------------

/// True for the ten keys `applyInitialSettingsPreset` accepts.
pub fn is_known_preset_key(key: &str) -> bool {
    matches!(
        key,
        "ttsMode"
            | "compactOutputMode"
            | "deleteCompactProgressOnFinish"
            | "showThinkingContent"
            | "showAssistantRunFooter"
            | "pinnedDashboardEnabled"
            | "showBackgroundSessionNotifications"
            | "responseStreamingMode"
            | "sendDiffFileAttachments"
            | "promptQueueEnabled"
    )
}

/// True for `off` / `all` / `auto` (TS: `VALID_TTS_MODES`).
pub fn is_valid_preset_tts_value(value: &str) -> bool {
    matches!(value, "off" | "all" | "auto")
}

/// True for `edit` / `draft` (TS: `VALID_STREAMING_MODES`).
pub fn is_valid_preset_streaming_value(value: &str) -> bool {
    matches!(value, "edit" | "draft")
}

// ---------------------------------------------------------------------------
// Scheduled tasks: clone backfill + toggle guard
// (`cloneScheduledTask`, `toggleScheduledTask`)
// ---------------------------------------------------------------------------

/// Backfill for tasks persisted before the `agent` field existed
/// (TS: `task.agent ?? "build"`).
pub fn backfill_task_agent<'a>(agent: Option<&'a str>) -> &'a str {
    agent.unwrap_or("build")
}

/// Default for legacy cron tasks persisted without `enabled`
/// (TS: `enabled ?? true` inside `cloneScheduledTask`).
pub fn default_task_enabled(enabled: Option<bool>) -> bool {
    enabled.unwrap_or(true)
}

/// True when `toggleScheduledTask` returns `null` without writing:
/// non-`cron` kinds never toggle, and an already-matching state is a noop
/// (TS: `task.kind !== "cron" || (task.enabled !== false) === enabled`).
pub fn cron_toggle_is_noop(kind: &str, current_enabled: bool, requested: bool) -> bool {
    kind != "cron" || current_enabled == requested
}

/// `activationPending` marker written on enable
/// (TS: `activationPending: enabled || undefined`).
pub fn activation_pending_after_enable(enabled: bool) -> Option<bool> {
    if enabled {
        Some(true)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Scheduled-task session ignores: 24h TTL
// (`scheduled-task-session-ignore-service.ts`)
// ---------------------------------------------------------------------------

/// Ignore time-to-live: 24h in ms.
pub const SESSION_IGNORE_TTL_MS: i64 = 24 * 60 * 60 * 1000;

/// True when `now - createdAt < TTL` (TS: `isFreshIgnore`).
pub fn is_fresh_ignore(created_at_ms: i64, now_ms: i64) -> bool {
    now_ms.saturating_sub(created_at_ms) < SESSION_IGNORE_TTL_MS
}

/// True only when the session id matches AND the ignore is still fresh.
pub fn should_ignore_session(session_matches: bool, fresh: bool) -> bool {
    session_matches && fresh
}

/// True when `nextRunAt` parses and is at/below `now`
/// (TS: `isTaskDue`; `None`/missing never due).
pub fn is_task_due_ms(next_run_ms: Option<i64>, now_ms: i64) -> bool {
    matches!(next_run_ms, Some(t) if t <= now_ms)
}

// ---------------------------------------------------------------------------
// Runtime mode (`src/runtime/mode.ts`)
// ---------------------------------------------------------------------------

/// `"installed"` / `"sources"` or `None` (TS: `normalizeMode`).
pub fn normalize_runtime_mode(raw: &str) -> Option<&'static str> {
    match raw {
        "installed" => Some("installed"),
        "sources" => Some("sources"),
        _ => None,
    }
}

/// Last `--mode` / `--mode=` value wins; invalid values collapse to `None`
/// (TS: `parseModeFromArgv`).
pub fn parse_mode_from_argv(argv: &[&str]) -> Option<&'static str> {
    let mut mode: Option<&'static str> = None;
    let mut index = 0;
    while index < argv.len() {
        let token = argv[index];
        if token.is_empty() {
            index += 1;
            continue;
        }
        if token == "--mode" {
            let value = argv.get(index + 1).copied().unwrap_or("");
            if value.is_empty() || value.starts_with('-') {
                return None;
            }
            mode = normalize_runtime_mode(value);
            index += 2;
            continue;
        }
        if let Some(value) = token.strip_prefix("--mode=") {
            mode = normalize_runtime_mode(value);
        }
        index += 1;
    }
    mode
}

/// True for a bare/malformed/unknown `--mode` flag
/// (TS: `hasInvalidModeSyntax`).
pub fn has_invalid_mode_syntax(argv: &[&str]) -> bool {
    let mut index = 0;
    while index < argv.len() {
        let token = argv[index];
        if token.is_empty() {
            index += 1;
            continue;
        }
        if token == "--mode" {
            match argv.get(index + 1) {
                Some(value)
                    if !value.is_empty()
                        && !value.starts_with('-')
                        && normalize_runtime_mode(value).is_some() =>
                {
                    index += 2;
                    continue;
                }
                _ => return true,
            }
        }
        if let Some(value) = token.strip_prefix("--mode=") {
            if normalize_runtime_mode(value).is_none() {
                return true;
            }
        }
        index += 1;
    }
    false
}

/// Outcome of `resolveRuntimeMode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeModeResolve {
    pub mode: String,
    pub error: Option<String>,
}

/// Explicit wins; invalid argv syntax keeps the default plus the
/// `"Invalid value for --mode. Expected sources|installed"` error;
/// otherwise the argv mode or the default with no error.
pub fn resolve_runtime_mode(
    explicit: Option<&str>,
    argv: &[&str],
    default: &str,
) -> RuntimeModeResolve {
    if let Some(mode) = explicit {
        if !mode.is_empty() {
            return RuntimeModeResolve {
                mode: mode.to_string(),
                error: None,
            };
        }
    }
    if has_invalid_mode_syntax(argv) {
        return RuntimeModeResolve {
            mode: default.to_string(),
            error: Some("Invalid value for --mode. Expected sources|installed".to_string()),
        };
    }
    RuntimeModeResolve {
        mode: parse_mode_from_argv(argv).unwrap_or(default).to_string(),
        error: None,
    }
}

// ---------------------------------------------------------------------------
// Container + heartbeat (`src/runtime/container.ts`, `app-container.ts`)
// ---------------------------------------------------------------------------

/// Flag gate: empty/`0`/`false`/`no` (case-insensitive, trimmed) are off,
/// everything else non-empty is on (TS: `isEnabledFlag`).
pub fn is_enabled_flag(raw: Option<&str>) -> bool {
    match raw {
        None => false,
        Some(value) => {
            let normalized = value.trim().to_lowercase();
            !normalized.is_empty()
                && normalized != "0"
                && normalized != "false"
                && normalized != "no"
        }
    }
}

/// Container when the env flag is on, else when `/.dockerenv` exists
/// (TS: `isContainerRuntime`).
pub fn is_container_runtime(env_flag: Option<&str>, dockerenv_exists: bool) -> bool {
    if is_enabled_flag(env_flag) {
        return true;
    }
    dockerenv_exists
}

/// Heartbeat cadence: every 5000ms.
pub const HEARTBEAT_INTERVAL_MS: u64 = 5000;

/// Heartbeat log cadence: every 6th tick (30s).
pub const HEARTBEAT_LOG_EVERY_TICKS: u64 = 6;

/// True on every 6th positive tick (TS: `counter % 6 === 0`).
pub fn heartbeat_should_log(counter: u64) -> bool {
    counter % HEARTBEAT_LOG_EVERY_TICKS == 0
}

// ---------------------------------------------------------------------------
// Paths (`src/runtime/paths.ts`)
// ---------------------------------------------------------------------------

/// Home resolution: non-blank `OPENCODE_TELEGRAM_HOME` wins, else `cwd` in
/// `sources` mode, else the installed home (TS: `resolveAppHome`).
pub fn resolve_app_home(
    mode: &str,
    home_override: Option<&str>,
    cwd: &str,
    installed_home: &str,
) -> String {
    if let Some(raw) = home_override {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let path = Path::new(trimmed);
            if path.is_absolute() {
                return trimmed.to_string();
            }
            return Path::new(cwd)
                .join(trimmed)
                .to_string_lossy()
                .into_owned();
        }
    }
    if mode == "sources" {
        cwd.to_string()
    } else {
        installed_home.to_string()
    }
}

// ---------------------------------------------------------------------------
// OpenCode server target (`src/opencode/server-target.ts`)
// ---------------------------------------------------------------------------

/// Address for the log without credentials; unparseable input falls back to
/// `"the configured OPENCODE_API_URL"`
/// (TS: `describeOpencodeServerUrl`).
pub fn describe_server_url(url: &str) -> String {
    const FALLBACK: &str = "the configured OPENCODE_API_URL";
    let (protocol, rest) = match url.split_once("://") {
        Some(parts) if !parts.0.is_empty() => parts,
        _ => return FALLBACK.to_string(),
    };
    let split_at = rest
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(rest.len());
    let (authority, after) = rest.split_at(split_at);
    let host = match authority.rsplit_once('@') {
        Some((_, host)) => host,
        None => authority,
    };
    if host.is_empty() {
        return FALLBACK.to_string();
    }
    let path = after
        .split(|c| c == '?' || c == '#')
        .next()
        .unwrap_or("");
    let path = if path == "/" { "" } else { path };
    format!("{protocol}://{host}{path}")
}

/// Which stored server the next request targets: `Some(url)` switches away
/// from the configured URL, `None` (or a URL equal to the configured one)
/// stays on the configured server (TS: `switchOpencodeServer` mapping).
pub fn resolve_switched_server(
    new_url: Option<&str>,
    configured_url: &str,
) -> Option<String> {
    match new_url {
        Some(url) if url != configured_url => Some(url.to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// OpenCode client (`src/opencode/client.ts`)
// ---------------------------------------------------------------------------

/// Probe path per API version: `v1` -> `"global/health"`,
/// `v2` -> `"api/info"` (TS: `PROBE_ROUTES`).
pub fn probe_route(version: &str) -> &'static str {
    if version == "v2" {
        "api/info"
    } else {
        "global/health"
    }
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64_ALPHABET[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_ALPHABET[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// `Basic base64(user:password)` header, or `None` when no password is set
/// (TS: `getAuth`; empty password means no header).
pub fn build_basic_auth(username: Option<&str>, password: Option<&str>) -> Option<String> {
    match password {
        Some(secret) if !secret.is_empty() => {
            let credentials = format!("{}:{}", username.unwrap_or(""), secret);
            Some(format!("Basic {}", base64_encode(credentials.as_bytes())))
        }
        _ => None,
    }
}

/// Probe outcome from the HTTP status + body gate
/// (TS: `probeOpencodeServer`: `401` -> `"unauthorized"`;
/// ok + JSON + version -> `"found"`; else `"none"`).
pub fn classify_probe(status: u16, content_type_json: bool, has_version: bool, ok: bool) -> &'static str {
    if status == 401 {
        return "unauthorized";
    }
    if !ok || !content_type_json {
        return "none";
    }
    if has_version {
        "found"
    } else {
        "none"
    }
}

/// `base + "/" + route` with exactly one separator
/// (TS: `baseUrl.endsWith("/") ? baseUrl : baseUrl + "/"` + route).
pub fn join_probe_url(base: &str, route: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{route}")
    } else {
        format!("{base}/{route}")
    }
}

// ---------------------------------------------------------------------------
// Bootstrap Telegram retry (`src/app/bootstrap/start-bot-app.ts`)
// ---------------------------------------------------------------------------

/// Fatal token codes that are never retried: `401` / `404`
/// (TS: `TELEGRAM_FATAL_TOKEN_CODES`).
pub fn is_fatal_token_code(code: Option<u16>) -> bool {
    matches!(code, Some(401) | Some(404))
}

/// Server codes retried at startup: `500-599`
/// (TS: `isTelegramStartupServerError`).
pub fn is_retryable_server_code(code: Option<u16>) -> bool {
    matches!(code, Some(500..=599))
}

/// Retry cap: `min(requested, 60_000)` (TS: `TELEGRAM_STARTUP_RETRY_CAP_MS`).
pub fn capped_retry_delay_ms(requested_ms: u64) -> u64 {
    requested_ms.min(60_000)
}

/// Startup delay: `min(max(base * 2^(attempt-1), retry_after), cap)`
/// with `base = 1000`, `cap = 60_000`
/// (TS: `nextTelegramStartupRetryDelayMs`).
pub fn next_startup_retry_delay_ms(attempt: u32, retry_after_ms: Option<u64>) -> u64 {
    const BASE_MS: u64 = 1000;
    const CAP_MS: u64 = 60_000;
    let shift = attempt.saturating_sub(1).min(16);
    let exponential_ms = BASE_MS.saturating_mul(1u64 << shift).min(CAP_MS);
    match retry_after_ms {
        None => exponential_ms,
        Some(retry_after) => exponential_ms.max(retry_after).min(CAP_MS),
    }
}
