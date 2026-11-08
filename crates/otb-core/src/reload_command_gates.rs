//! Reload-command gates (slice 171, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/bot/commands/reload.test.ts` (the reload flow over
//! `src/bot/commands/reload-command.ts`):
//! - the reloading key (`reload.reloading`, the initial `ctx.reply`)
//! - the result key (`reload.success`, `reload.failed_with_error` when a
//!   failure carries an error, `reload.failed` for a bare failure, a timeout,
//!   or a thrown reload)
//! - the model-view refresh gate (only a successful reload that replaced the
//!   selected model refreshes the model views immediately)
//! - the late-catalog settle gate (only a successful reload schedules
//!   `refreshModelViewsAfterLateCatalogSettle`; failures and throws never do)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/bot/commands/reload-command.ts` lines 18-26 (`formatReloadResult`:
//!   success -> `reload.success`, failed with error ->
//!   `reload.failed_with_error`, everything else -> `reload.failed`), 28-54
//!   (`reloadAndReport`: refresh views on `success && modelChanged`, always
//!   schedule the late-catalog settle on success, catch maps to the bare
//!   failure line, then `editBotText`), 61-71 (`reloadCommand`: reply the
//!   reloading line, then run the reload in the background).
//! - `src/app/services/config-reload-service.ts` lines 11-14
//!   (`ConfigReloadResult`: `success { modelChanged }`, `failed { error }`,
//!   `timeout`).
//! - `tests/bot/commands/reload.test.ts` lines 47-62 (success edits the
//!   reloading message into success with no immediate refresh but one late
//!   settle), 64-73 (success with `modelChanged` redraws the model views),
//!   75-86 (failure with error shows `failed_with_error` and schedules no
//!   late settle), 88-101 (bare failure and timeout both show the bare
//!   failure line), 103-111 (a thrown reload ends in the bare failure line),
//!   113-121 (two reloads each edit their own status message).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`ctx.reply`, `editBotText`, `safeBackgroundTask`
//!   delivery, `setMyCommands` plumbing).
//! - `reloadOpencodeConfig` I/O (server reload, catalog refresh, timeout
//!   join) and `refreshModelViews` /
//!   `refreshModelViewsAfterLateCatalogSettle` delivery.
//! - per-message plumbing (`statusMessage.message_id` -> `editBotText`;
//!   test lines 113-121 are two calls with different ids, owned by the
//!   caller).
//! - `t(key, params)` resolution (keys travel here as literal key strings)
//!   and the `logger.error` emission.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - locale strings for the `reload.*` keys (i18n owners).
//! - `config-reload-service` timeout/join/in-flight atoms (service owner;
//!   this slice takes the settled `ConfigReloadResult` as input).
//!
//! Deviations: none intended. The thrown reload (TS `catch`) and the
//! `timeout` kind both read as the bare failure key, matching
//! `formatReloadResult` falling through to `t("reload.failed")`; the
//! nullable `error` maps to `Option<&str>` with `Some("")` treated as absent
//! per the `Boolean(command && ...)` precedent.
//!
//! Step 2 (GREEN): pure sync logic ported from the TS contract asserted
//! by the parity tests in
//! `crates/otb-core/tests/slice171_reload_command_parity.rs`.

/// Initial status reply key (TS: `ctx.reply(t("reload.reloading"))`).
pub const RELOAD_RELOADING_KEY: &str = "reload.reloading";

/// Success result key (TS: `t("reload.success")`).
pub const RELOAD_SUCCESS_KEY: &str = "reload.success";

/// Bare failure result key (TS: `t("reload.failed")` — bare `failed`,
/// `timeout`, or a thrown reload).
pub const RELOAD_FAILED_KEY: &str = "reload.failed";

/// Failure-with-error result key (TS: `t("reload.failed_with_error",
/// { error })`).
pub const RELOAD_FAILED_WITH_ERROR_KEY: &str = "reload.failed_with_error";

/// Settled reload outcome (TS: `ConfigReloadResult` plus the `catch` in
/// `reloadAndReport`, which maps a thrown reload to the bare failure line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadCompletion<'a> {
    /// `reloadOpencodeConfig` resolved `{ kind: "success", modelChanged }`.
    Success { model_changed: bool },
    /// `reloadOpencodeConfig` resolved `{ kind: "failed", error }`.
    /// `None` (and `Some("")`) is the bare failure line from test lines
    /// 88-93.
    Failed { error: Option<&'a str> },
    /// `reloadOpencodeConfig` resolved `{ kind: "timeout" }` (test lines
    /// 96-101).
    Timeout,
    /// `reloadOpencodeConfig` rejected (test lines 103-111: `catch` maps to
    /// the bare failure line).
    Thrown,
}

/// Result key for one settled reload (TS: `formatReloadResult` — success is
/// `reload.success`, a failure with an error is `reload.failed_with_error`,
/// every other outcome is the bare `reload.failed`).
pub fn reload_result_key(completion: ReloadCompletion<'_>) -> &'static str {
    match completion {
        ReloadCompletion::Success { .. } => RELOAD_SUCCESS_KEY,
        ReloadCompletion::Failed { error } => match error {
            Some(message) if !message.is_empty() => RELOAD_FAILED_WITH_ERROR_KEY,
            _ => RELOAD_FAILED_KEY,
        },
        ReloadCompletion::Timeout | ReloadCompletion::Thrown => RELOAD_FAILED_KEY,
    }
}

/// Immediate model-view refresh gate (TS: `if (result.kind === "success") {
/// if (result.modelChanged) await refreshModelViews(deps) }` — only a
/// successful reload that replaced the selected model refreshes now; test
/// lines 47-62 assert no refresh when `modelChanged` is false, lines 64-73
/// assert one refresh when true, failures never refresh).
pub fn should_refresh_model_views(completion: ReloadCompletion<'_>) -> bool {
    matches!(
        completion,
        ReloadCompletion::Success { model_changed: true }
    )
}

/// Late-catalog settle gate (TS: `refreshModelViewsAfterLateCatalogSettle`
/// runs inside the same `result.kind === "success"` branch, whatever
/// `modelChanged` is; test line 61 asserts it on success without a model
/// change, test line 85 asserts it never runs on failure, and the thrown
/// path in `catch` never reaches it).
pub fn should_schedule_late_catalog_settle(completion: ReloadCompletion<'_>) -> bool {
    matches!(completion, ReloadCompletion::Success { .. })
}
