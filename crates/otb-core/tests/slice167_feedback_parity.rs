//! Slice 167 parity: callback feedback gates.
//!
//! Mirrors the pure TS contract in
//! `tests/bot/callbacks/feedback.test.ts` (the callback feedback flow over
//! `src/bot/callbacks/feedback.ts`):
//! - the 200-char answer budget (`CALLBACK_ANSWER_MAX_LENGTH`)
//! - the over-budget head-cut (long answers are cut to 200 chars; short
//!   and exactly-200 answers pass through)
//! - the answer payload shapes (notify toasts, alert modals, switched
//!   answers empty, cancel answers toast the cancelled text)
//! - the rejection gate (failure/cancel swallow a stale answer rejection;
//!   notify/alert/switched propagate it)
//! - the answer-first effect orders (switched answers, replies, deletes;
//!   cancelMenu answers then deletes; cancelPrompt answers then edits)
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `context_inline_gates` / `callback_health_gates` (the same 200-char
//!   truncation atoms; this slice owns the feedback-flow copies)
//! - grammy delivery, `.catch` wiring, `t()` lookups, `logger.warn`
//!   emission, answer-before-mutation await timing (callers)

use otb_core::feedback_gates::*;

// ---- budget constants (2) ----

#[test]
fn feedback_budget_value_parity() {
    assert_eq!(CALLBACK_ANSWER_MAX_LENGTH, 200);
}

#[test]
fn feedback_cancelled_key_value_parity() {
    assert_eq!(CANCELLED_TOAST_KEY, "common.cancelled");
}

// ---- truncation gate (2) ----

#[test]
fn feedback_truncation_gate_parity() {
    assert!(!should_truncate_answer(0));
    assert!(!should_truncate_answer(199));
    assert!(!should_truncate_answer(200));
    assert!(should_truncate_answer(201));
    assert!(should_truncate_answer(300));
}

#[test]
fn feedback_truncated_length_matches_budget_parity() {
    // TS: the 300-char name is cut to exactly 200 chars.
    let long = "x".repeat(300);
    assert_eq!(truncate_answer_text(&long).chars().count(), 200);
}

// ---- truncation projection (3) ----

#[test]
fn feedback_short_answer_passes_through_parity() {
    assert_eq!(truncate_answer_text("ok"), "ok");
}

#[test]
fn feedback_exact_budget_answer_passes_through_parity() {
    let exact = "y".repeat(200);
    assert_eq!(truncate_answer_text(&exact), exact);
}

#[test]
fn feedback_long_answer_head_cut_parity() {
    let long = "x".repeat(300);
    let cut = truncate_answer_text(&long);
    assert_eq!(cut, "x".repeat(200));
}

// ---- payload shapes (4) ----

#[test]
fn feedback_notify_toasts_text_parity() {
    assert_eq!(
        notify_answer_payload("cancelled"),
        AnswerPayload::Toast("cancelled".to_string())
    );
}

#[test]
fn feedback_alert_modals_text_parity() {
    assert_eq!(
        alert_answer_payload("empty"),
        AnswerPayload::Modal("empty".to_string())
    );
}

#[test]
fn feedback_switched_answers_empty_parity() {
    // Rule 1: the toast is deliberately empty; the message is the feedback.
    assert_eq!(switched_answer_payload(), AnswerPayload::Empty);
}

#[test]
fn feedback_cancel_toasts_cancelled_parity() {
    assert_eq!(
        cancel_answer_payload("cancelled"),
        AnswerPayload::Toast("cancelled".to_string())
    );
}

// ---- rejection gate (2) ----

#[test]
fn feedback_failure_and_cancel_swallow_rejection_parity() {
    // TS: `failure` swallows a rejected answer instead of throwing.
    assert!(should_swallow_answer_rejection(FeedbackKind::Failure));
    assert!(should_swallow_answer_rejection(FeedbackKind::CancelMenu));
    assert!(should_swallow_answer_rejection(FeedbackKind::CancelPrompt));
}

#[test]
fn feedback_notify_alert_switched_propagate_rejection_parity() {
    // TS: `notify` does not swallow a rejected answer.
    assert!(!should_swallow_answer_rejection(FeedbackKind::Notify));
    assert!(!should_swallow_answer_rejection(FeedbackKind::Alert));
    assert!(!should_swallow_answer_rejection(FeedbackKind::Switched));
}

// ---- effect orders (3) ----

#[test]
fn feedback_switched_order_parity() {
    // TS: switched answers without text, sends the keyboard, then drops
    // the menu (`["answer", "reply", "delete"]`).
    assert_eq!(
        feedback_effect_order(FeedbackKind::Switched),
        &[
            FeedbackEffect::Answer,
            FeedbackEffect::Reply,
            FeedbackEffect::Delete
        ]
    );
}

#[test]
fn feedback_cancel_menu_order_parity() {
    // TS: cancelMenu answers before deleting the menu message.
    assert_eq!(
        feedback_effect_order(FeedbackKind::CancelMenu),
        &[FeedbackEffect::Answer, FeedbackEffect::Delete]
    );
}

#[test]
fn feedback_cancel_prompt_order_parity() {
    // TS: cancelPrompt answers before replacing the prompt text.
    assert_eq!(
        feedback_effect_order(FeedbackKind::CancelPrompt),
        &[FeedbackEffect::Answer, FeedbackEffect::Edit]
    );
}

#[test]
fn feedback_notify_only_answers_parity() {
    assert_eq!(
        feedback_effect_order(FeedbackKind::Notify),
        &[FeedbackEffect::Answer]
    );
    assert_eq!(
        feedback_effect_order(FeedbackKind::Alert),
        &[FeedbackEffect::Answer]
    );
    assert_eq!(
        feedback_effect_order(FeedbackKind::Failure),
        &[FeedbackEffect::Answer]
    );
}
