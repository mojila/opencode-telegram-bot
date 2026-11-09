//! Slice 168 parity: prompt-attachment cancel flow gates.
//!
//! Mirrors the pure TS contract in
//! `tests/bot/callbacks/prompt-attachment-callback-handler.test.ts` (the
//! cancel flow over
//! `src/bot/callbacks/prompt-attachment-callback-handler.ts`):
//! - exact dispatch data (`attach:cancel`; foreign or missing data is not
//!   this flow — the handler returns `false` with no effects)
//! - the cancelled key (`attachment.cancelled`, resolved via `cancelPrompt`)
//! - the cancel effect order (clear the attachment, clear the waiting mode,
//!   answer, then edit)
//! - the keyboard drop (the edit carries no `reply_markup` so the button
//!   cannot be pressed twice)
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `router_middleware_gates` (the `ATTACHMENT_CANCEL_CALLBACK`
//!   dispatch atom; this slice owns the cancel-flow copies)
//! - `feedback_gates` (the 200-char truncation and answer-then-edit order
//!   behind `cancelPrompt`; this slice owns the flow copies)
//! - `attach_foreground_gates` (the attachment store atoms)
//! - locale strings for `attachment.cancelled` (i18n owners)
//! - grammy delivery, `promptAttachment` / `interactionManager` effects,
//!   `t()` lookups, `logger.debug` emission (callers)

use otb_core::prompt_attachment_cancel_gates::*;

// ---- dispatch data (2) ----

#[test]
fn prompt_attachment_cancel_data_value_parity() {
    assert_eq!(ATTACHMENT_CANCEL_DATA, "attach:cancel");
}

#[test]
fn prompt_attachment_cancelled_key_value_parity() {
    assert_eq!(ATTACHMENT_CANCELLED_KEY, "attachment.cancelled");
}

// ---- dispatch guard (2) ----

#[test]
fn prompt_attachment_cancel_guard_matches_only_cancel_parity() {
    assert!(handles_prompt_attachment_cancel(Some("attach:cancel")));
    assert!(!handles_prompt_attachment_cancel(Some(
        "ls:download:/repo/a.ts"
    )));
    assert!(!handles_prompt_attachment_cancel(Some("attach:other")));
    assert!(!handles_prompt_attachment_cancel(None));
    assert!(!handles_prompt_attachment_cancel(Some("")));
}

#[test]
fn prompt_attachment_cancel_foreign_data_returns_no_effects_parity() {
    // TS: a foreign callback returns `false` and never answers.
    assert_eq!(
        decide_prompt_attachment_cancel(Some("ls:download:/repo/a.ts")),
        PromptAttachmentCancelAction::Ignore
    );
    assert_eq!(
        decide_prompt_attachment_cancel(None),
        PromptAttachmentCancelAction::Ignore
    );
    assert_eq!(
        decide_prompt_attachment_cancel(Some("")),
        PromptAttachmentCancelAction::Ignore
    );
}

// ---- flow decision (2) ----

#[test]
fn prompt_attachment_cancel_decision_accepts_cancel_parity() {
    // TS: the cancel callback clears state and returns `true`.
    assert_eq!(
        decide_prompt_attachment_cancel(Some("attach:cancel")),
        PromptAttachmentCancelAction::Cancel
    );
}

#[test]
fn prompt_attachment_cancel_cancel_clears_before_answering_parity() {
    // Clearing precedes the answer/edit: the attachment clear is first,
    // the waiting-mode clear is second.
    let order = prompt_attachment_cancel_effect_order();
    assert_eq!(order[0], PromptAttachmentCancelEffect::ClearAttachment);
    assert_eq!(order[1], PromptAttachmentCancelEffect::ClearWaitingMode);
}

// ---- effect order (2) ----

#[test]
fn prompt_attachment_cancel_effect_order_parity() {
    // TS: `promptAttachment.clear`, `interactionManager.clear`, then
    // `cancelPrompt` (answer, then edit).
    use PromptAttachmentCancelEffect::*;
    assert_eq!(
        prompt_attachment_cancel_effect_order(),
        &[ClearAttachment, ClearWaitingMode, Answer, Edit]
    );
}

#[test]
fn prompt_attachment_cancel_edit_drops_keyboard_parity() {
    // TS: the edit is a single argument — no `reply_markup` carried over,
    // so the button cannot be pressed twice.
    assert!(!prompt_attachment_cancel_edit_carries_keyboard());
}
