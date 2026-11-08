//! Detach-command gates (slice 169, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/bot/commands/detach.test.ts` (the detach flow over
//! `src/bot/commands/detach-command.ts`):
//! - the detach reason (`detach_command`, passed to the attached-session
//!   detach, the run clear, and the interaction reset)
//! - the guard order (missing project first, then missing session; guard
//!   paths reply with no teardown effects)
//! - the teardown order (fetch the OpenCode title, hand the waiting queue
//!   over to the session so the clear below finds nothing to withdraw,
//!   detach/clear local state, clear the pinned message, init the keyboard,
//!   refresh the context limit, read the keyboard, then reply success)
//! - the conditional steps (pinned clear only when initialized, keyboard
//!   init only when `ctx.chat` is present)
//! - the success-reply keyboard gate (`reply_markup` only when the keyboard
//!   manager returns one) and the used-token reset (`updateContext(0, …)`)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/bot/commands/detach-command.ts` lines 33-43 (project guard, then
//!   session guard), 45 (`fetchSessionTitle`), 49-55 (`handOverPromptQueue`
//!   before `detachAttachedSession` / `clearPromptResponseMode` /
//!   `markIdle` / `clearRun` / `resetInteractions` / `clearSession`, all
//!   under the `"detach_command"` reason), 57-63 (pinned clear when
//!   initialized), 65-73 (keyboard init, context refresh, `updateContext(0,
//!   contextLimit)`, keyboard read), 79-81 (success reply with keyboard),
//!   82-85 (error reply).
//! - `tests/bot/commands/detach.test.ts` lines 109-120 (handover before
//!   clear), 122-144 (full teardown effects, update-before-read, success
//!   reply with `reply_markup`), 146-163 (idle session, same behavior),
//!   165-184 (title fetched before clear), 186-196 (unnamed session named
//!   via the default title), 198-208 (no session: no-op, no effects),
//!   210-219 (no project: no-op, no effects).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`ctx.reply`, keyboard delivery).
//! - `getCurrentProject` / `getCurrentSession` reads, `fetchSessionTitle`
//!   (OpenCode), `handOverPromptQueue`, `detachAttachedSession`,
//!   `clearPromptResponseMode`, `markIdle`, `clearRun`, `resetInteractions`,
//!   `clearSession`, pinned/keyboard manager I/O.
//! - `t(key, params)` resolution (keys travel here as literal key strings)
//!   and the `logger` emission.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `formatters::extra_formatters::format_session_title` (the empty-title
//!   -> default-session-title fallback behind the success text; TS test
//!   lines 186-196).
//! - `handover_restart_gates` (the handover delivery atoms; repeated below
//!   only as this flow's call-site order, per the slice 167 precedent).
//! - `command_definitions_gates::CommandDefinition` for `"detach"` (the
//!   command definition atom).
//! - locale strings for the `detach.*` keys (i18n owners).
//!
//! Deviations: none intended. The `"detach_command"` reason travels as a
//! literal; the nullable keyboard maps to
//! [`detach_success_carries_keyboard`].
//!
//! Step 2 (GREEN): pure sync logic ported from the TS contract asserted
//! by the parity tests in
//! `crates/otb-core/tests/slice169_detach_command_parity.rs`.

/// Reason passed to the attached-session detach, the run clear, and the
/// interaction reset (TS: `"detach_command"`).
pub const DETACH_REASON: &str = "detach_command";

/// No-project guard reply key (TS: `t("detach.project_not_selected")`).
pub const DETACH_PROJECT_NOT_SELECTED_KEY: &str = "detach.project_not_selected";

/// No-session guard reply key (TS: `t("detach.no_active_session")`).
pub const DETACH_NO_ACTIVE_SESSION_KEY: &str = "detach.no_active_session";

/// Success reply key (TS: `t("detach.success", { title })`).
pub const DETACH_SUCCESS_KEY: &str = "detach.success";

/// Failure reply key (TS: `t("detach.error")`).
pub const DETACH_ERROR_KEY: &str = "detach.error";

/// Used-token count after a detach (TS: `updateContext(0, contextLimit)`).
pub const DETACH_CLEARED_USED_TOKENS: u32 = 0;

/// Detach entry precondition (TS: the project guard, then the session
/// guard; guard paths reply with no teardown effects).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetachPrecondition {
    /// No project selected (TS: replies `detach.project_not_selected`).
    NeedsProject,
    /// Project selected but no session (TS: replies
    /// `detach.no_active_session`).
    NeedsSession,
    /// Project and session selected (TS: runs the teardown order).
    Ready,
}

/// One delivered effect in the detach flow (TS: the awaited calls in
/// `detachCommand`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetachEffect {
    /// `fetchSessionTitle(currentSession)` (naming precedes the clear).
    FetchTitle,
    /// `handOverPromptQueue(currentSession)` (waiting messages stay with
    /// the session so the clear below finds nothing to withdraw).
    HandoverQueue,
    /// `detachAttachedSession("detach_command", deps)`.
    DetachAttached,
    /// `clearPromptResponseMode(session.id)`.
    ClearPromptMode,
    /// `foregroundSessionState.markIdle(session.id)`.
    MarkIdle,
    /// `assistantRunState.clearRun(session.id, "detach_command")`.
    ClearRun,
    /// `resetInteractions("detach_command")`.
    ResetInteractions,
    /// `clearSession()`.
    ClearSession,
    /// `pinnedMessageManager.clear()` (only when initialized; failures are
    /// swallowed by the caller).
    ClearPinned,
    /// `keyboardManager.initialize(api, chat.id)` (only when `ctx.chat`
    /// is present).
    InitializeKeyboard,
    /// `pinnedMessageManager.refreshContextLimit()`.
    RefreshContextLimit,
    /// `keyboardManager.updateContext(0, contextLimit)`.
    UpdateContext,
    /// `keyboardManager.getKeyboard()`.
    ReadKeyboard,
    /// `ctx.reply(success, …)`.
    Reply,
}

/// Entry decision (TS: the project guard runs first, then the session
/// guard).
pub fn decide_detach_precondition(has_project: bool, has_session: bool) -> DetachPrecondition {
    if !has_project {
        DetachPrecondition::NeedsProject
    } else if !has_session {
        DetachPrecondition::NeedsSession
    } else {
        DetachPrecondition::Ready
    }
}

/// Teardown order for the ready path (TS: fetch title, hand the queue over,
/// detach/clear local state, clear pinned, init keyboard, refresh context,
/// read keyboard, reply).
pub fn detach_effect_order() -> &'static [DetachEffect] {
    use DetachEffect::*;
    &[
        FetchTitle,
        HandoverQueue,
        DetachAttached,
        ClearPromptMode,
        MarkIdle,
        ClearRun,
        ResetInteractions,
        ClearSession,
        ClearPinned,
        InitializeKeyboard,
        RefreshContextLimit,
        UpdateContext,
        ReadKeyboard,
        Reply,
    ]
}

/// Pinned-clear gate (TS: the pinned message is cleared only when
/// `pinnedMessageManager.isInitialized()`).
pub fn should_clear_pinned(pinned_initialized: bool) -> bool {
    pinned_initialized
}

/// Keyboard-init gate (TS: the keyboard manager is initialized only when
/// `ctx.chat` is present).
pub fn should_initialize_keyboard(has_chat: bool) -> bool {
    has_chat
}

/// Success-reply keyboard gate (TS: the success reply spreads
/// `{ reply_markup: keyboard }` only when `getKeyboard()` returns one).
pub fn detach_success_carries_keyboard(has_keyboard: bool) -> bool {
    has_keyboard
}
