//! Server command dispatch gates (slice 104, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/commands/opencode-start-command.ts`
//!   (`health?.data?.healthy` already-running gate; the unavailable-server
//!   resolution dispatch (`refuse` -> error reply, `answers` ->
//!   already-running reply, `connected` -> connected reply, anything else
//!   spawns and posts the starting message); the spawned-process `!pid`
//!   start-error gate; the ready-wait `elapsed < maxWaitMs` poll bound and
//!   the `!ready` started-not-ready gate; the `notifyReady` /
//!   `notifyUnavailable` lifecycle reason strings)
//! - `src/bot/commands/opencode-stop-command.ts` (`STOP_REASON`
//!   `"opencode_stop"`, the 5000ms kill budget, the `!localTarget`
//!   remote-configured gate, the `!pid` not-running gate, the `!stopped`
//!   stop-error gate)
//! - `src/bot/commands/reload-command.ts` (`formatReloadResult` key
//!   selection: `success` -> `reload.success`, `failed` with an error ->
//!   `reload.failed_with_error`, anything else -> `reload.failed`; the
//!   `success && modelChanged` model-views refresh gate; the `success`
//!   late-catalog watch gate)
//!
//! Already ported elsewhere and NOT repeated here:
//! - `handover_restart_gates::classify_unavailable_resolution` (the
//!   `refuse` / `answers`+`connected` adopt / spawn kind mapping behind the
//!   start dispatch; this slice owns the command's finer `answers` vs
//!   `connected` reply split), `AUTO_RESTART_READY_TIMEOUT_MS` /
//!   `AUTO_RESTART_READY_POLL_INTERVAL_MS` /
//!   `AUTO_RESTART_HEALTH_CHECK_TIMEOUT_MS` (the same 10000/500/3000 budgets
//!   the start command waits with; wait bounds arrive as explicit ms args).
//! - `local_process_gates` (`resolveLocalOpencodeTarget`, pid parsing),
//!   `server_target_gates` (`getActiveOpencodeServer`, health-cause
//!   reporting), `callback_health_gates` (`classifyFailedHealthCheck`).
//! - `ConfigReloadResult` construction/timeout racing
//!   (`src/app/services/config-reload-service.ts`); the result kind and the
//!   error presence arrive as explicit `ReloadResultKind` / `has_error`
//!   snapshots.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`reply`, `editBotText`, status messages),
//!   `opencodeClient.global.health` probes with `AbortController` timeouts,
//!   `resolveUnavailableOpencodeServer` / `startLocalOpencodeServer` /
//!   `findServerPid` / `killServerProcess` / `clearStandServerPid`,
//!   `reloadOpencodeConfig`, `refreshModelViews` /
//!   `refreshModelViewsAfterLateCatalogSettle`, `safeBackgroundTask`,
//!   `Date.now()` clocks (arrive as explicit `elapsed_ms`), `t(...)`
//!   lookups (reply keys travel as literal key strings in the caller),
//!   `explainFailedHealthCheck`, `logger`.
//!
//! Deviations: none intended. `pid` presence arrives as `has_pid`,
//! resolution kinds arrive as the raw `kind` string from
//! `resolveUnavailableOpencodeServer`, and health arrives as an explicit
//! `healthy` bool (the TS `health?.data?.healthy` optional chain stays with
//! the caller).
//!
//! Step 2 (GREEN): pure port landed. The parity tests in
//! `crates/otb-core/tests/slice104_server_command_parity.rs` assert this
//! contract.

// ---------------------------------------------------------------------------
// /opencode_start (`src/bot/commands/opencode-start-command.ts`)
// ---------------------------------------------------------------------------

/// Lifecycle reason posted when the start finds a healthy server already up
/// (TS: `notifyReady("opencode_start_already_running")`).
pub const START_NOTIFY_ALREADY_RUNNING: &str = "opencode_start_already_running";

/// Lifecycle reason posted when the start finds nothing listening
/// (TS: `notifyUnavailable("opencode_start_not_running")`).
pub const START_NOTIFY_NOT_RUNNING: &str = "opencode_start_not_running";

/// Lifecycle reason posted when the start adopts the registered server
/// (TS: `notifyReady("opencode_start_connected")`).
pub const START_NOTIFY_CONNECTED: &str = "opencode_start_connected";

/// Lifecycle reason posted after a spawned server answers healthy
/// (TS: `notifyReady("opencode_start_success")`).
pub const START_NOTIFY_SUCCESS: &str = "opencode_start_success";

/// What the start command does with an unavailable-server resolution
/// (TS: the `resolution.kind` dispatch in `opencodeStartCommand`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartResolutionAction {
    /// `refuse`: reply with the generic error.
    Refuse,
    /// `answers`: the registered server answered; reply already-running.
    AlreadyRunning,
    /// `connected`: adopted the registered server on another port.
    Connected,
    /// Anything else: spawn a local server and post the starting message.
    Spawn,
}

/// Dispatch a resolution kind to its start action (TS: `if (kind ===
/// "refuse") ... if (kind === "answers") ... if (kind === "connected") ...
/// else spawn`).
pub fn classify_start_resolution(kind: &str) -> StartResolutionAction {
    match kind {
        "refuse" => StartResolutionAction::Refuse,
        "answers" => StartResolutionAction::AlreadyRunning,
        "connected" => StartResolutionAction::Connected,
        _ => StartResolutionAction::Spawn,
    }
}

/// `true` when the start ends early because a server already answers
/// healthy (TS: `if (health?.data?.healthy)`).
pub fn should_report_start_already_running(healthy: bool) -> bool {
    healthy
}

/// `true` when the spawned process has no pid and the start reports a
/// start error instead of waiting (TS: `if (!pid)`).
pub fn should_report_start_spawn_error(has_pid: bool) -> bool {
    !has_pid
}

/// `true` while the ready poll keeps waiting (TS: `Date.now() - startTime
/// < maxWaitMs` in `waitForServerReady`).
pub fn should_keep_waiting_for_ready(elapsed_ms: u64, max_wait_ms: u64) -> bool {
    elapsed_ms < max_wait_ms
}

/// `true` when the spawned server never became ready and the start reports
/// started-not-ready instead of success (TS: `if (!ready)`).
pub fn should_report_start_not_ready(ready: bool) -> bool {
    !ready
}

// ---------------------------------------------------------------------------
// /opencode_stop (`src/bot/commands/opencode-stop-command.ts`)
// ---------------------------------------------------------------------------

/// Reason threading every stop release (`resetRuntimeStreams`,
/// `foregroundSessionState.clearAll`, queue/handover withdraws,
/// `resetInteractions`, `notifyUnavailable`) (TS: `STOP_REASON`).
pub const STOP_REASON: &str = "opencode_stop";

/// Budget for the SIGTERM/SIGKILL escalation
/// (TS: `killServerProcess(pid, 5000)`).
pub const STOP_KILL_TIMEOUT_MS: u64 = 5_000;

/// `true` when the stop ends early because the active server is not a local
/// target (TS: `if (!localTarget)`).
pub fn should_report_stop_remote(has_local_target: bool) -> bool {
    !has_local_target
}

/// `true` when the stop ends early because no server pid listens on the
/// target port (TS: `if (!pid)`).
pub fn should_report_stop_not_running(has_pid: bool) -> bool {
    !has_pid
}

/// `true` when the kill did not stop the process and the stop reports a
/// stop error instead of success (TS: `if (!stopped)`).
pub fn should_report_stop_error(stopped: bool) -> bool {
    !stopped
}

// ---------------------------------------------------------------------------
// /reload (`src/bot/commands/reload-command.ts`)
// ---------------------------------------------------------------------------

/// Config-reload outcome kinds (TS: `ConfigReloadResult["kind"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadResultKind {
    Success,
    Failed,
    Timeout,
}

/// i18n key for the reload report line (TS: `formatReloadResult`).
pub fn reload_result_message_key(kind: ReloadResultKind, has_error: bool) -> &'static str {
    match kind {
        ReloadResultKind::Success => "reload.success",
        ReloadResultKind::Failed if has_error => "reload.failed_with_error",
        _ => "reload.failed",
    }
}

/// `true` when a successful reload redraws the model views immediately
/// (TS: `if (result.kind === "success") { if (result.modelChanged)
/// refreshModelViews(...) }`).
pub fn should_refresh_model_views_after_reload(
    kind: ReloadResultKind,
    model_changed: bool,
) -> bool {
    kind == ReloadResultKind::Success && model_changed
}

/// `true` when a successful reload watches for late catalog changes
/// (TS: `refreshModelViewsAfterLateCatalogSettle(...)` inside the same
/// `kind === "success"` branch).
pub fn should_watch_late_catalog_after_reload(kind: ReloadResultKind) -> bool {
    kind == ReloadResultKind::Success
}
