//! Switch-presentation / model-views / safe-background-task gates (slice 110, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/project-switch-service.ts` (`currentModel.variant ||
//!   "default"` fallback before `formatVariantForButton`, initial
//!   `tokensUsed: 0`, `trackBackgroundSessions && ensureEventSubscription`
//!   gate)
//! - `src/bot/services/project-switch-presentation.ts` (`if (ctx.chat)`
//!   initialize gate)
//! - `src/bot/services/attach-presentation.ts` (`preparePinnedSession`
//!   same-session-with-message gate, message-with-null-session restore gate,
//!   `forceFullRestore` reload gate, `showPinnedSession` context-info gate,
//!   `syncAttachState` initialized gate)
//! - `src/bot/services/model-views.ts` (`isInitialized` keyboard-update gate)
//! - `src/utils/safe-background-task.ts` (`[safeBackgroundTask]` prefix,
//!   task-failed and hook-failed message builders, `if (!hook) return` gate)
//!
//! Already ported elsewhere and NOT re-tested here:
//! `formatVariantForButton` / `formatVariantForDisplay` (`agent_model_variant_gates`
//! owner), `getAttachBusyStatus` (`attach_foreground_gates` owner),
//! restore/reconnect orchestration gates (`attach_restore_gates` owner).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `Context`/`Api` I/O, `keyboardManager` / `pinnedMessageManager`
//!   mutation and reads (snapshots arrive as explicit bools/ids),
//!   `refreshContextLimit` / `prepareRestore` / `prepareSessionChange` /
//!   `showPreparedSession` / `clear` awaits, `resolveProjectAgent` /
//!   `getStoredModel` reads, `watchLateModelCatalogChanges` waits,
//!   `logger` calls, `Promise` plumbing in `safeBackgroundTask`.
//!
//! Deviations: none intended. JS truthiness is mirrored explicitly:
//! `None` and `""` count as missing for variant (whitespace-only strings
//! count as present, like TS `||`); a `None` pinned session never equals
//! the current id (TS `null === string` is `false`).
//!
//! Step 2 (GREEN): pure ports of the TS gates below.

/// Log prefix for background-task failures
/// (TS: `` `[safeBackgroundTask]` `` in `safe-background-task.ts`).
pub const SAFE_BACKGROUND_TASK_PREFIX: &str = "[safeBackgroundTask]";

/// Pinned-dashboard action for `preparePinnedSession`
/// (TS: same-session early return vs `prepareRestore` vs
/// `prepareSessionChange` in `attach-presentation.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparePinnedAction {
    SameSession,
    Restore,
    Change,
}

/// `"[safeBackgroundTask] {task} failed:"`
/// (TS: `` `[safeBackgroundTask] ${taskName} failed:` ``).
pub fn safe_background_task_error_message(task_name: &str) -> String {
    format!("{SAFE_BACKGROUND_TASK_PREFIX} {task_name} failed:")
}

/// `"[safeBackgroundTask] {task}: {hook} failed:"`
/// (TS: `` `[safeBackgroundTask] ${taskName}: ${hookName} failed:` ``).
pub fn safe_background_hook_error_message(task_name: &str, hook_name: &str) -> String {
    format!("{SAFE_BACKGROUND_TASK_PREFIX} {task_name}: {hook_name} failed:")
}

/// `true` when the optional hook runs (TS: `if (!hook) return;`).
pub fn should_run_hook(has_hook: bool) -> bool {
    has_hook
}

/// Variant id used for the switch keyboard
/// (TS: `currentModel.variant || "default"` in `project-switch-service.ts`).
pub fn resolve_switch_variant(variant: Option<&str>) -> &str {
    match variant {
        Some(v) if !v.is_empty() => v,
        _ => "default",
    }
}

/// Initial used-tokens for a fresh switch dashboard
/// (TS: `tokensUsed: 0` in `project-switch-service.ts`).
pub fn switch_initial_tokens_used() -> u64 {
    0
}

/// `true` when the switch follows a registered server stream
/// (TS: `if (config.bot.trackBackgroundSessions && options.ensureEventSubscription)`).
pub fn should_ensure_event_subscription(track_background: bool, has_ensure: bool) -> bool {
    track_background && has_ensure
}

/// `true` when the switch keyboard is initialized
/// (TS: `if (ctx.chat)` in `project-switch-presentation.ts`).
pub fn should_initialize_switch_keyboard(has_chat: bool) -> bool {
    has_chat
}

/// `true` when the model button follows the stored model
/// (TS: `if (deps.keyboardManager.isInitialized())` in `model-views.ts`).
pub fn should_update_model_views_keyboard(is_initialized: bool) -> bool {
    is_initialized
}

/// `true` when the attach state is synced to the dashboard
/// (TS: `if (!pinnedMessageManager.isInitialized()) return;` in
/// `attach-presentation.ts`).
pub fn should_sync_attach_state(is_initialized: bool) -> bool {
    is_initialized
}

/// `true` when the pinned session is the session being attached
/// (TS: `pinnedState.sessionId === session.id`).
pub fn is_same_pinned_session(pinned_session_id: Option<&str>, current_session_id: &str) -> bool {
    pinned_session_id == Some(current_session_id)
}

/// Dashboard action for the attach prepare step (TS: same-session-with-message
/// early return, else message-with-null-session restore, else change).
pub fn select_prepare_pinned_action(
    is_same_session: bool,
    has_message: bool,
    session_is_null: bool,
) -> PreparePinnedAction {
    if is_same_session && has_message {
        PreparePinnedAction::SameSession
    } else if has_message && session_is_null {
        PreparePinnedAction::Restore
    } else {
        PreparePinnedAction::Change
    }
}

/// `true` when the same-session path reloads values from history
/// (TS: `if (forceFullRestore)` in `attach-presentation.ts`).
pub fn should_full_restore_on_same_session(force_full_restore: bool) -> bool {
    force_full_restore
}

/// `true` when the keyboard context follows the dashboard values
/// (TS: `if (contextInfo)` in `attach-presentation.ts`).
pub fn should_update_keyboard_context(has_context_info: bool) -> bool {
    has_context_info
}
