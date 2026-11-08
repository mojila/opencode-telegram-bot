//! Feedback gates (slice 167, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/bot/callbacks/feedback.test.ts` (the callback feedback contract
//! over `src/bot/callbacks/feedback.ts`):
//! - `CALLBACK_ANSWER_MAX_LENGTH` (200; Telegram rejects longer answers)
//! - `resolveAnswerText` head-cut (over-budget resolved text is cut to the
//!   first 200 chars and warns; at-or-under budget passes through)
//! - answer payload shapes (`notify` toasts `{ text }`, `alert` modals
//!   `{ text, show_alert: true }`, `switched` answers with no text)
//! - rejection handling (`failure` / `cancelMenu` / `cancelPrompt` swallow
//!   an `answerCallbackQuery` rejection; `notify` / `alert` / `switched`
//!   propagate it)
//! - answer-first effect order (`switched` answers, then replies, then
//!   deletes; `cancelMenu` answers then deletes; `cancelPrompt` answers
//!   then edits)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/bot/callbacks/feedback.ts` lines 20-37 (`CALLBACK_ANSWER_MAX_LENGTH`,
//!   `resolveAnswerText`), 40-52 (`notify` / `alert` / `failure`), 58-78
//!   (`switched` / `cancelMenu` / `cancelPrompt`).
//! - `tests/bot/callbacks/feedback.test.ts` lines 29-56 (toast, modal,
//!   interpolation), 58-70 (failure swallows, notify propagates), 72-135
//!   (`switched` / `cancelMenu` / `cancelPrompt` answer-first orders),
//!   137-148 (over-budget answer is cut to 200 chars and warns).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `reply`,
//!   `editMessageText`, `deleteMessage` delivery and `.catch` wiring).
//! - `t(key, params)` resolution (arrives here as already-resolved text;
//!   the `common.cancelled` key travels as a literal key string).
//! - `logger.warn` truncation warning (the caller warns when
//!   [`should_truncate_answer`] is true).
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `context_inline_gates::CALLBACK_ANSWER_MAX_LENGTH`,
//!   `truncate_callback_answer`, `should_truncate_callback_answer` (the
//!   same 200-char head-cut behind `resolveAnswerText`; repeated below
//!   only as this flow's dispatch table, per the slice 166 precedent).
//! - `callback_health_gates` truncation predicates (same budget).
//! - locale strings for `common.cancelled` and other answer keys (i18n
//!   owners).
//!
//! Deviations: none intended. `String.length` (UTF-16 code units in TS)
//! vs `chars().count()` here coincide on the ASCII-only parity fixtures;
//! `text.slice(0, 200)` maps to a `chars().take(200)` head-cut. `messageId`
//! shapes are not involved in this flow.
//!
//! Step 2 (GREEN): pure sync logic ported from the TS contract asserted
//! by the parity tests in
//! `crates/otb-core/tests/slice167_feedback_parity.rs`.

/// Telegram callback-answer budget (TS: `CALLBACK_ANSWER_MAX_LENGTH`).
pub const CALLBACK_ANSWER_MAX_LENGTH: usize = 200;

/// Cancel toast key (TS: `resolveAnswerText("common.cancelled")` in
/// `cancelMenu` / `cancelPrompt`).
pub const CANCELLED_TOAST_KEY: &str = "common.cancelled";

/// One feedback flow (TS: the exported helper behind the test).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackKind {
    Notify,
    Alert,
    Failure,
    Switched,
    CancelMenu,
    CancelPrompt,
}

/// One delivered effect in a feedback flow (TS: the awaited grammy calls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackEffect {
    Answer,
    Reply,
    Edit,
    Delete,
}

/// Answer payload shape (TS: the `answerCallbackQuery` argument).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerPayload {
    /// Toast at the top of the screen (TS: `{ text }`).
    Toast(String),
    /// Modal the user dismisses (TS: `{ text, show_alert: true }`).
    Modal(String),
    /// Empty confirmation (TS: `answerCallbackQuery()` with no args).
    Empty,
}

/// Over-budget gate (TS: `text.length > CALLBACK_ANSWER_MAX_LENGTH` in
/// `resolveAnswerText`; the caller warns when this is true).
pub fn should_truncate_answer(char_count: usize) -> bool {
    char_count > CALLBACK_ANSWER_MAX_LENGTH
}

/// Over-budget head-cut (TS: `text.slice(0, CALLBACK_ANSWER_MAX_LENGTH)`;
/// at-or-under budget passes through).
pub fn truncate_answer_text(resolved: &str) -> String {
    if resolved.chars().count() <= CALLBACK_ANSWER_MAX_LENGTH {
        return resolved.to_string();
    }
    resolved.chars().take(CALLBACK_ANSWER_MAX_LENGTH).collect()
}

/// Toast payload (TS: `notify` answers `{ text }`).
pub fn notify_answer_payload(resolved_text: &str) -> AnswerPayload {
    AnswerPayload::Toast(truncate_answer_text(resolved_text))
}

/// Modal payload (TS: `alert` answers `{ text, show_alert: true }`).
pub fn alert_answer_payload(resolved_text: &str) -> AnswerPayload {
    AnswerPayload::Modal(truncate_answer_text(resolved_text))
}

/// Empty confirmation payload (TS: `switched` answers with no text; the
/// message below is the confirmation).
pub fn switched_answer_payload() -> AnswerPayload {
    AnswerPayload::Empty
}

/// Cancel toast payload (TS: `cancelMenu` / `cancelPrompt` answer
/// `{ text: resolveAnswerText("common.cancelled") }`).
pub fn cancel_answer_payload(resolved_cancelled: &str) -> AnswerPayload {
    AnswerPayload::Toast(truncate_answer_text(resolved_cancelled))
}

/// Rejection gate (TS: `failure` / `cancelMenu` / `cancelPrompt` wire
/// `.catch(() => {})` on the answer so a stale `query is too old`
/// rejection never throws; `notify` / `alert` / the `switched` answer
/// propagate it).
pub fn should_swallow_answer_rejection(kind: FeedbackKind) -> bool {
    matches!(
        kind,
        FeedbackKind::Failure | FeedbackKind::CancelMenu | FeedbackKind::CancelPrompt
    )
}

/// Answer-first effect order (TS: the await sequences in `switched` /
/// `cancelMenu` / `cancelPrompt`; notifies/alerts/failures only answer).
pub fn feedback_effect_order(kind: FeedbackKind) -> &'static [FeedbackEffect] {
    match kind {
        FeedbackKind::Switched => &[
            FeedbackEffect::Answer,
            FeedbackEffect::Reply,
            FeedbackEffect::Delete,
        ],
        FeedbackKind::CancelMenu => &[FeedbackEffect::Answer, FeedbackEffect::Delete],
        FeedbackKind::CancelPrompt => &[FeedbackEffect::Answer, FeedbackEffect::Edit],
        FeedbackKind::Notify | FeedbackKind::Alert | FeedbackKind::Failure => {
            &[FeedbackEffect::Answer]
        }
    }
}
