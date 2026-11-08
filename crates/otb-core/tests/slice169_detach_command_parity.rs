//! Slice 169 parity: detach-command gates.
//!
//! Mirrors the pure TS contract in
//! `tests/bot/commands/detach.test.ts` (the detach flow over
//! `src/bot/commands/detach-command.ts`):
//! - the detach reason (`detach_command`, passed to the attached-session
//!   detach, the run clear, and the interaction reset)
//! - the guard order (missing project replies `detach.project_not_selected`
//!   with no effects; otherwise a missing session replies
//!   `detach.no_active_session` with no effects)
//! - the teardown order (fetch the OpenCode title, hand the waiting queue
//!   over to the session, detach/clear local state, clear the pinned
//!   message, init the keyboard, refresh the context limit, then reply)
//! - the handover-before-clear gate (waiting messages stay with the session
//!   so the clear below finds nothing to withdraw)
//! - the keyboard gate (the success reply carries `reply_markup` only when
//!   the keyboard manager returns one)
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `formatters::extra_formatters::format_session_title` (the empty-title
//!   -> default-session-title fallback behind the success text)
//! - `handover_restart_gates` (the handover delivery atoms; this slice owns
//!   the detach call-site order)
//! - `command_definitions_gates` (the `detach` command definition atom)
//! - grammy delivery, `t()` lookups, `logger` emission, pinned/keyboard
//!   manager I/O, session-object plumbing (callers)

use otb_core::detach_command_gates::*;

// ---- reason + keys (3) ----

#[test]
fn detach_reason_value_parity() {
    assert_eq!(DETACH_REASON, "detach_command");
}

#[test]
fn detach_guard_key_values_parity() {
    assert_eq!(
        DETACH_PROJECT_NOT_SELECTED_KEY,
        "detach.project_not_selected"
    );
    assert_eq!(DETACH_NO_ACTIVE_SESSION_KEY, "detach.no_active_session");
}

#[test]
fn detach_reply_key_values_parity() {
    assert_eq!(DETACH_SUCCESS_KEY, "detach.success");
    assert_eq!(DETACH_ERROR_KEY, "detach.error");
}

// ---- precondition decision (3) ----

#[test]
fn detach_missing_project_short_circuits_first_parity() {
    // TS: the project guard runs before the session guard, so a missing
    // project wins even when a session is selected.
    assert_eq!(
        decide_detach_precondition(false, true),
        DetachPrecondition::NeedsProject
    );
    assert_eq!(
        decide_detach_precondition(false, false),
        DetachPrecondition::NeedsProject
    );
}

#[test]
fn detach_missing_session_needs_session_parity() {
    // TS: with a project but no session the handler replies the no-op
    // message and performs no teardown.
    assert_eq!(
        decide_detach_precondition(true, false),
        DetachPrecondition::NeedsSession
    );
}

#[test]
fn detach_ready_when_project_and_session_selected_parity() {
    assert_eq!(
        decide_detach_precondition(true, true),
        DetachPrecondition::Ready
    );
}

// ---- teardown order (3) ----

#[test]
fn detach_handover_runs_before_clear_parity() {
    // TS: `handOverPromptQueue` is awaited before `clearSession`, so what
    // waits for the running turn stays with the session.
    let order = detach_effect_order();
    let handover = order
        .iter()
        .position(|e| *e == DetachEffect::HandoverQueue)
        .expect("handover in order");
    let clear = order
        .iter()
        .position(|e| *e == DetachEffect::ClearSession)
        .expect("clear in order");
    assert!(handover < clear);
}

#[test]
fn detach_title_fetch_runs_before_clear_parity() {
    // TS: `fetchSessionTitle` is awaited before `clearSession`, so the
    // success text names the session OpenCode knows.
    let order = detach_effect_order();
    let fetch = order
        .iter()
        .position(|e| *e == DetachEffect::FetchTitle)
        .expect("fetch in order");
    let clear = order
        .iter()
        .position(|e| *e == DetachEffect::ClearSession)
        .expect("clear in order");
    assert!(fetch < clear);
}

#[test]
fn detach_full_effect_order_parity() {
    // TS: fetch title, hand the queue over, detach + clear local state,
    // clear the pinned message, init the keyboard, refresh the context
    // limit, read the keyboard, then reply.
    use DetachEffect::*;
    assert_eq!(
        detach_effect_order(),
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
    );
}

// ---- conditional gates (4) ----

#[test]
fn detach_pinned_clear_needs_initialization_parity() {
    // TS: the pinned message is cleared only when the manager is
    // initialized; failures are swallowed by the caller.
    assert!(should_clear_pinned(true));
    assert!(!should_clear_pinned(false));
}

#[test]
fn detach_keyboard_init_needs_chat_parity() {
    // TS: the keyboard manager is initialized only when `ctx.chat` is
    // present.
    assert!(should_initialize_keyboard(true));
    assert!(!should_initialize_keyboard(false));
}

#[test]
fn detach_context_refresh_precedes_keyboard_read_parity() {
    // TS: `refreshContextLimit` -> `getContextLimit` -> `updateContext` ->
    // `getKeyboard` (the update runs before the keyboard is read).
    let order = detach_effect_order();
    let update = order
        .iter()
        .position(|e| *e == DetachEffect::UpdateContext)
        .expect("update in order");
    let read = order
        .iter()
        .position(|e| *e == DetachEffect::ReadKeyboard)
        .expect("read in order");
    assert!(update < read);
}

#[test]
fn detach_success_reply_carries_keyboard_only_when_present_parity() {
    // TS: the success reply spreads `{ reply_markup: keyboard }` only when
    // the keyboard manager returns one.
    assert!(detach_success_carries_keyboard(true));
    assert!(!detach_success_carries_keyboard(false));
}

// ---- context reset atom (1) ----

#[test]
fn detach_cleared_used_tokens_value_parity() {
    // TS: `updateContext(0, contextLimit)` resets the used-token count.
    assert_eq!(DETACH_CLEARED_USED_TOKENS, 0);
}
