//! Session / health-server / document-extractor / config-reload gates (slice 97, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/session-service.ts` (`setCurrentSession` same-id
//!   keep gate vs different-id reset, `fetchSessionTitle` fetched-vs-error
//!   fallback; `clearSession` always resets — no branch to port)
//! - `src/app/services/run-control-service.ts` (`isForegroundBusy` OR gate,
//!   `getBusyDirectories` dedupe of foreground dirs plus the busy attached
//!   directory, `reconcileForegroundBusyState` early-return gate)
//! - `src/app/services/health-server-service.ts` (`HEALTH_HOST`,
//!   `OPENCODE_CHECK_TIMEOUT_MS`, `CLOSE_CONNECTIONS_TIMEOUT_MS`, the
//!   `method !== "GET"` 405 gate, the hand-rolled `split("?")[0]` path gate,
//!   `/health/live` / `/health/ready` / `/health` / 404 dispatch,
//!   ready 200-vs-503 and payload healthy-vs-degraded status, and
//!   `getUptimeSeconds` null-start floor math)
//! - `src/app/services/document-extractor-service.ts`
//!   (`REQUEST_TIMEOUT_MS`, `isDocExtractorConfigured` `Boolean(url)` gate,
//!   bearer auth header gate, timeout / HTTP-error / missing-text messages)
//! - `src/app/services/config-reload-service.ts`
//!   (`CONFIG_RELOAD_TIMEOUT_MS`, `reloadInFlight ?? startReload` join gate,
//!   success `modelChanged` inversion, timeout clears only the current
//!   operation)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - settings-store reads/writes (`getSettingsSession`,
//!   `setCurrentSession`, `clearSession`), `withdrawPromptQueue`,
//!   `promptAttachment.clear`.
//! - `foregroundSessionState` / `attachManager` snapshots, `reconcileBusyStateNow`
//!   directory probes, `checkOpencodeHealth` network checks, `http` listen/close,
//!   `setTimeout` races, `Date.now()` clocks (arrive as explicit `now_ms` args).
//! - `fetch` document extraction, `FormData`/`Blob` bodies, `AbortController`
//!   timeouts, `opencodeV2Client.location.reload()` and model-catalog refresh.
//!
//! Deviations: none intended. `String.length` parity fixtures use ASCII-only
//! inputs where JS UTF-16 code units and Rust `char`s coincide. `uptime_seconds`
//! mirrors `Math.floor((now - start) / 1000)`; both args are whole millis so
//! Rust truncating division agrees on `now >= start` (parity fixtures never
//! run the clock backwards). `split_health_path` treats `None` like TS
//! `req.url ?? "/"`.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice97_session_health_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Loopback host the health endpoint binds (TS: `HEALTH_HOST`).
pub const HEALTH_HOST: &str = "127.0.0.1";

/// OpenCode check budget inside a health probe (TS: `OPENCODE_CHECK_TIMEOUT_MS`).
pub const OPENCODE_CHECK_TIMEOUT_MS: u64 = 3000;

/// Forced connection close budget on health-server stop
/// (TS: `CLOSE_CONNECTIONS_TIMEOUT_MS`).
pub const CLOSE_CONNECTIONS_TIMEOUT_MS: u64 = 2000;

/// Document-extractor fetch budget (TS: `REQUEST_TIMEOUT_MS`).
pub const DOC_EXTRACTOR_REQUEST_TIMEOUT_MS: u64 = 60_000;

/// Config-reload wait budget; only the wait ends (TS: `CONFIG_RELOAD_TIMEOUT_MS`).
pub const CONFIG_RELOAD_TIMEOUT_MS: u64 = 60_000;

/// Thrown when no document-extractor URL is configured (TS: `extractDocument` guard).
pub const DOC_EXTRACTOR_NOT_CONFIGURED_MESSAGE: &str =
    "Document extractor is not configured: DOC_EXTRACTOR_URL is required";

/// Thrown when the extractor answer carries no text field (TS: `extractDocument` guard).
pub const DOC_EXTRACTOR_MISSING_TEXT_MESSAGE: &str =
    "Document extractor API response does not contain a text field";

/// Health-endpoint dispatch (TS: `HealthServer.handleRequest` branches).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthRoute {
    Live,
    Ready,
    Root,
    NotFound,
    MethodNotAllowed,
}

/// Config-reload outcome (TS: `ConfigReloadResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigReloadResult {
    Success { model_changed: bool },
    Failed { error: Option<String> },
    Timeout,
}

/// `true` when switching sessions must drop the previous session's queue
/// and attachment (TS: `getSettingsSession()?.id !== sessionInfo.id`).
pub fn should_reset_on_session_set(previous_id: Option<&str>, next_id: &str) -> bool {
    previous_id.is_none_or(|prev| prev != next_id)
}

/// Title to show: the freshly fetched one unless the fetch failed
/// (TS: `if (!error && data) return data.title; return session.title`).
pub fn resolve_session_title(fetched: Option<&str>, failed: bool, fallback: &str) -> String {
    if !failed {
        if let Some(title) = fetched {
            return title.to_string();
        }
    }
    fallback.to_string()
}

/// Foreground is busy when either session state or the attach holds it
/// (TS: `foregroundSessionState.isBusy() || attachManager.isBusy()`).
pub fn is_foreground_busy(fg_busy: bool, attach_busy: bool) -> bool {
    fg_busy || attach_busy
}

/// `false` skips the reconcile pass entirely
/// (TS: `if (!isForegroundBusy(deps)) return;`).
pub fn should_reconcile_busy(is_busy: bool) -> bool {
    is_busy
}

/// Busy directories, foreground first, attached appended once when busy
/// (TS: `getBusyDirectories` insertion-ordered `Set`).
pub fn collect_busy_directories(
    fg_dirs: &[String],
    attached_dir: Option<&str>,
    attached_busy: bool,
) -> Vec<String> {
    let mut directories: Vec<String> = Vec::with_capacity(fg_dirs.len() + 1);
    for dir in fg_dirs {
        if !directories.iter().any(|existing| existing == dir) {
            directories.push(dir.clone());
        }
    }
    if attached_busy {
        if let Some(dir) = attached_dir {
            if !directories.iter().any(|existing| existing == dir) {
                directories.push(dir.to_string());
            }
        }
    }
    directories
}

/// Only `GET` is served (TS: `if (req.method !== "GET")` 405 gate).
pub fn is_health_method_allowed(method: &str) -> bool {
    method == "GET"
}

/// Dispatch a request line to its route; `None` reads as `"/"` and both
/// `"/"` and `"/health"` serve the root payload
/// (TS: `(req.url ?? "/").split("?")[0]` + branch chain).
pub fn resolve_health_route(method: &str, url: Option<&str>) -> HealthRoute {
    if method != "GET" {
        return HealthRoute::MethodNotAllowed;
    }
    let raw = url.unwrap_or("/");
    let path = raw.split('?').next().unwrap_or("/");
    match path {
        "/health/live" => HealthRoute::Live,
        "/health/ready" => HealthRoute::Ready,
        "/health" | "/" => HealthRoute::Root,
        _ => HealthRoute::NotFound,
    }
}

/// Ready status code: healthy reads live, degraded does not
/// (TS: `payload.status === "healthy" ? 200 : 503`).
pub fn health_ready_status(healthy: bool) -> u16 {
    if healthy { 200 } else { 503 }
}

/// Payload status field (TS: `opencode.healthy ? "healthy" : "degraded"`).
pub fn health_payload_status(healthy: bool) -> &'static str {
    if healthy { "healthy" } else { "degraded" }
}

/// Whole seconds since start, `0` without a start
/// (TS: `getUptimeSeconds`: `startTimeMs === null ? 0 : floor(...)`).
pub fn uptime_seconds(start_ms: Option<i64>, now_ms: i64) -> i64 {
    match start_ms {
        None => 0,
        Some(start) => (now_ms - start) / 1000,
    }
}

/// `true` when a document-extractor URL is configured
/// (TS: `Boolean(config.docExtractor.apiUrl)`).
pub fn is_doc_extractor_configured(api_url: Option<&str>) -> bool {
    matches!(api_url, Some(url) if !url.is_empty())
}

/// Bearer header for the extractor call, or `None` without a key
/// (TS: `if (config.docExtractor.apiKey) headers["Authorization"] = ...`).
pub fn doc_extractor_auth_header(api_key: Option<&str>) -> Option<String> {
    match api_key {
        Some(key) if !key.is_empty() => Some(format!("Bearer {key}")),
        _ => None,
    }
}

/// Timeout error text (TS: `` `Document extractor request timed out after ...` ``).
pub fn doc_extractor_timeout_message() -> String {
    format!(
        "Document extractor request timed out after {DOC_EXTRACTOR_REQUEST_TIMEOUT_MS}ms"
    )
}

/// HTTP-error text preferring the body over the status text
/// (TS: `` `... HTTP ${status}: ${errorBody || statusText}` ``).
pub fn doc_extractor_http_error(status: u16, body: &str, status_text: &str) -> String {
    let detail = if body.is_empty() { status_text } else { body };
    format!("Document extractor API returned HTTP {status}: {detail}")
}

/// Validated extractor text, or the missing-text message
/// (TS: `typeof data.text !== "string"` guard).
pub fn extracted_text_or_error(text: Option<&str>) -> Result<String, &'static str> {
    match text {
        Some(value) => Ok(value.to_string()),
        None => Err(DOC_EXTRACTOR_MISSING_TEXT_MESSAGE),
    }
}

/// `true` when the call joins the reload already in flight
/// (TS: `reloadInFlight ?? startReload()`).
pub fn should_join_inflight(has_inflight: bool) -> bool {
    has_inflight
}

/// Success outcome flag (TS: `modelChanged: !isSameModel(before, after)`).
pub fn reload_success_model_changed(before_eq_after: bool) -> bool {
    !before_eq_after
}

/// A timeout detaches only the operation that timed out
/// (TS: `if (reloadInFlight === operation) reloadInFlight = null`).
pub fn should_clear_inflight_on_timeout(result_is_timeout: bool, is_current: bool) -> bool {
    result_is_timeout && is_current
}
