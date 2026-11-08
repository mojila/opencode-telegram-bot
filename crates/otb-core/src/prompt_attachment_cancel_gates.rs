//! Prompt-attachment cancel gates (slice 168, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/bot/callbacks/prompt-attachment-callback-handler.test.ts`
//! (the cancel flow over
//! `src/bot/callbacks/prompt-attachment-callback-handler.ts`):
//! - exact dispatch data (`attach:cancel`; anything else, including missing
//!   data, is not this flow and the handler returns `false` with no effects)
//! - the cancelled key (`attachment.cancelled`, resolved via `cancelPrompt`)
//! - the cancel effect order (clear the attachment, clear the waiting mode,
//!   answer, then edit — the edit carries a single argument so no
//!   `reply_markup` is carried over and the button cannot be pressed twice)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/bot/callbacks/prompt-attachment-callback-handler.ts` lines 7-8
//!   (`ATTACHMENT_CANCEL_CALLBACK`), 11-27 (`handlePromptAttachmentCancel`:
//!   exact-match guard, `promptAttachment.clear("user_cancelled")`,
//!   `interactionManager.clear("attachment_cancelled")`,
//!   `cancelPrompt(ctx, "attachment.cancelled")`).
//! - `src/bot/callbacks/feedback.ts` lines 74-78 (`cancelPrompt` answers
//!   then edits).
//! - `tests/bot/callbacks/prompt-attachment-callback-handler.test.ts`
//!   lines 39-44 (foreign data returns `false` with no answer), 46-59
//!   (cancel clears the attachment and the waiting mode, returns `true`),
//!   61-72 (the edit drops the keyboard: a single argument).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`).
//! - `promptAttachment` read/write, `interactionManager` snapshot/clear.
//! - `t("attachment.cancelled")` resolution (arrives here as the literal
//!   key string) and the `logger.debug` emission.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `router_middleware_gates::ATTACHMENT_CANCEL_CALLBACK` and
//!   `is_attachment_cancel_callback` (the dispatch atom; the data string is
//!   repeated below only as the dispatch table this flow matches, per the
//!   slice 166 precedent).
//! - `feedback_gates` (the 200-char truncation behind `cancelPrompt` and
//!   the answer-then-edit order; repeated below only as this flow's full
//!   effect order, per the slice 167 precedent).
//! - locale strings for `attachment.cancelled` (i18n owners).
//! - `attach_foreground_gates` (the `promptAttachment` set/get/clear store
//!   atoms behind the clear effect).
//!
//! Deviations: none intended. The single-argument `editMessageText` maps to
//! [`prompt_attachment_cancel_edit_carries_keyboard`] returning `false`.
//!
//! Step 2 (GREEN): pure sync logic ported from the TS contract asserted
//! by the parity tests in
//! `crates/otb-core/tests/slice168_prompt_attachment_cancel_parity.rs`.

/// Exact cancel callback data (TS: `ATTACHMENT_CANCEL_CALLBACK`).
///
/// Canonical atom: `router_middleware_gates::ATTACHMENT_CANCEL_CALLBACK`;
/// repeated here as this flow's dispatch table.
pub const ATTACHMENT_CANCEL_DATA: &str = "attach:cancel";

/// Cancelled key resolved through `cancelPrompt` (TS: `"attachment.cancelled"`).
pub const ATTACHMENT_CANCELLED_KEY: &str = "attachment.cancelled";

/// Outcome of one prompt-attachment-cancel callback (TS: the `true`/`false`
/// return plus which effects the handler performs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAttachmentCancelAction {
    /// Not this flow (TS: `handlePromptAttachmentCancel` returns `false`;
    /// no answer, no state change — e.g. `"ls:download:/repo/a.ts"`).
    Ignore,
    /// Cancel accepted (TS: returns `true`; clears the attachment and the
    /// waiting mode, then runs `cancelPrompt`).
    Cancel,
}

/// One delivered effect in the cancel flow (TS: the manager clears plus the
/// awaited grammy calls inside `cancelPrompt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAttachmentCancelEffect {
    /// `promptAttachment.clear("user_cancelled")`.
    ClearAttachment,
    /// `interactionManager.clear("attachment_cancelled")`.
    ClearWaitingMode,
    /// `cancelPrompt` answers the callback query.
    Answer,
    /// `cancelPrompt` edits the prompt text (single argument: no
    /// `reply_markup` carried over).
    Edit,
}

/// Dispatch guard (TS: `ctx.callbackQuery?.data !== ATTACHMENT_CANCEL_CALLBACK`
/// returns `false`).
pub fn handles_prompt_attachment_cancel(data: Option<&str>) -> bool {
    data == Some(ATTACHMENT_CANCEL_DATA)
}

/// Cancel-flow decision (TS: the dispatch composition in
/// `handlePromptAttachmentCancel`).
pub fn decide_prompt_attachment_cancel(data: Option<&str>) -> PromptAttachmentCancelAction {
    if handles_prompt_attachment_cancel(data) {
        PromptAttachmentCancelAction::Cancel
    } else {
        PromptAttachmentCancelAction::Ignore
    }
}

/// Cancel effect order (TS: `promptAttachment.clear`, then
/// `interactionManager.clear`, then `cancelPrompt`'s answer-then-edit).
pub fn prompt_attachment_cancel_effect_order() -> &'static [PromptAttachmentCancelEffect] {
    use PromptAttachmentCancelEffect::*;
    &[ClearAttachment, ClearWaitingMode, Answer, Edit]
}

/// Keyboard-carry gate (TS: the cancel `editMessageText` is called with a
/// single argument, so no `reply_markup` is carried over and the cancel
/// button cannot be pressed twice).
pub fn prompt_attachment_cancel_edit_carries_keyboard() -> bool {
    false
}
