//! Question + permission callback flow-state gates (slice 94, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/question-callback-handler.ts`
//!   (`getCallbackMessageId` numeric-`message_id` gate; the inactive
//!   question gate; the active-message mismatch gate; the
//!   settling-from-Telegram swallow gate; the `NaN`-or-current-index
//!   mismatch gate; the select-with-`NaN`-option invalid gate;
//!   `answerStep`'s request-ID-plus-active drop gate and its
//!   last-question submit-vs-show-next branch;
//!   `handleQuestionTextAnswer`'s no-text / not-waiting-for-custom /
//!   single-with-custom-already / single-last-submit / multiple-reshow /
//!   single-next decision; `handleCancelPoll`'s missing-request-ID /
//!   directory / chat-ID reject gate; `finishDismissal`'s
//!   already-closed / settled-answered / gone-cancelled /
//!   accepted-or-settled-cancelled / failed-warning map)
//! - `src/bot/callbacks/permission-callback-handler.ts`
//!   (`getCallbackMessageId` numeric-`message_id` gate; the inactive
//!   permission gate; the active-message mismatch gate; the empty
//!   `requestIDs` gate; the bad-action reject gate; the missing
//!   directory / chat-ID context gate; the null-message-or-`markSending`
//!   swallow gate; the reject-on-V2 settles-session gate; the failed
//!   `failSending` apply-vs-warning map; the accepted-plus-reject-plus-
//!   session end-session-prompts gate)
//!
//! Already ported elsewhere and NOT repeated here:
//! - `question_permission_inline_gates::parse_question_callback`,
//!   `is_question_callback`, `is_last_question`,
//!   `is_permission_reply_value`, `classify_permission_callback`,
//!   `is_permission_callback`, `is_permission_request_not_found`
//!   (the `question:` / `permission:` dispatches, action shapes,
//!   last-question predicate, reply-value and not-found matchers behind
//!   the flow below; this slice takes their outcomes as explicit
//!   bools/enums).
//! - `question_permission_inline_gates::{QUESTION_CALLBACK_PREFIX,
//!   PERMISSION_CALLBACK_*}` (the callback-data constants behind the
//!   dispatch above; this slice owns the toast-key literals the flow
//!   answers with).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, alert/cancel/failure toasts).
//! - `questionManager` active/message/settling/index/selection/waiting
//!   reads, `permissionManager` active/message/requests/`markSending` /
//!   `failSending` / `endPrompt` / `endSessionPrompts` writes,
//!   `submitPollAnswers`, `showNextQuestion`, `showCurrentQuestion`,
//!   `updateQuestionMessage`, `syncQuestionInteractionState`,
//!   `applyPermissionPromptChanges`, `showPermissionDeliveryWarning`,
//!   `opencodeClient.question.reject` / `permission.reply`.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `messageId` values are `i64` (`typeof ===
//! `"number"` in TS), a missing message or a non-numeric `message_id`
//! reads as `None`, and `NaN` indices arrive as `None` (TS `parseInt`
//! failures stay with the caller).

/// Toast key answered when a question tap lands on no live poll.
pub const QUESTION_INACTIVE_TOAST_KEY: &str = "question.inactive_callback";
/// Toast key answered when a question tap cannot be understood.
pub const QUESTION_PROCESSING_TOAST_KEY: &str = "question.processing_error_callback";
/// Toast key answered when a permission tap lands on no live request.
pub const PERMISSION_INACTIVE_TOAST_KEY: &str = "permission.inactive_callback";
/// Toast key answered when a permission tap cannot be understood.
pub const PERMISSION_PROCESSING_TOAST_KEY: &str = "permission.processing_error_callback";
/// Toast key answered when a permission tap has no directory/chat to reply in.
pub const PERMISSION_NO_REQUEST_TOAST_KEY: &str = "permission.no_active_request_callback";

/// `answerStep` branch (TS: `isLastQuestion` submit vs delete-and-next).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerStep {
    SubmitPoll,
    ShowNext,
}

/// `handleQuestionTextAnswer` branch (TS: the no-text early return, the
/// not-waiting guard, the single-with-custom guard, the single-last
/// submit, the multiple reshow, the single-next fallthrough).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionTextDecision {
    Ignored,
    NeedCustomButton,
    AlreadyAnswered,
    SubmitPoll,
    ReshowCurrent,
    ShowNext,
}

/// Settled-while-sending kind behind `finishDismissal` (TS:
/// `getSettledWhileSending()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettledKind {
    Answered,
    Cancelled,
}

/// Dismissal send outcome behind `finishDismissal` (TS: the
/// `question.reject` `"accepted"` / `"gone"` / `"failed"` result).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DismissalResult {
    Accepted,
    Gone,
    Failed,
}

/// `finishDismissal` branch (TS: already-closed early return, settled
/// answers/cancels, accepted-or-settled-cancelled close, failed
/// warning).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishDismissal {
    AlreadyClosed,
    SettledAnswered,
    SettledCancelled,
    CloseCancelled,
    ShowWarning,
}

/// Failed-permission finish branch (TS: `failSending` change with an
/// outcome applies prompt changes, a change without one warns, no
/// change leaves the prompt as it is).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishPermission {
    ApplyChanges,
    ShowWarning,
    ClosePrompts,
    NoChange,
}

/// Question callback message-id resolve (TS: `getCallbackMessageId` in
/// `question-callback-handler.ts`: no message or no numeric
/// `message_id` reads as `None`).
pub fn question_callback_message_id(
    has_message_with_id: bool,
    message_id: Option<i64>,
) -> Option<i64> {
    if has_message_with_id {
        message_id
    } else {
        None
    }
}

/// Permission callback message-id resolve (TS: `getCallbackMessageId`
/// in `permission-callback-handler.ts`: no message or no numeric
/// `message_id` reads as `None`).
pub fn permission_callback_message_id(
    has_message_with_id: bool,
    message_id: Option<i64>,
) -> Option<i64> {
    if has_message_with_id {
        message_id
    } else {
        None
    }
}

/// Question inactive gate (TS: `!questionManager.isActive()` answers
/// `question.inactive_callback`).
pub fn should_reject_question_when_inactive(is_active: bool) -> bool {
    !is_active
}

/// Question active-message mismatch gate (TS:
/// `!questionManager.isActiveMessage(callbackMessageId)` answers
/// `question.inactive_callback`).
pub fn is_question_message_mismatch(
    active_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (active_message_id, callback_message_id) {
        (Some(active), Some(callback)) => active != callback,
        _ => true,
    }
}

/// Question settling swallow gate (TS:
/// `questionManager.isSettlingFromTelegram()` answers empty and
/// returns: the tap changes nothing while answers dismiss).
pub fn should_swallow_question_settling(is_settling: bool) -> bool {
    is_settling
}

/// Question index mismatch gate (TS: `Number.isNaN(questionIndex) ||
/// questionIndex !== questionManager.getCurrentIndex()` answers
/// `question.inactive_callback`; `NaN` arrives as `None`).
pub fn is_question_index_mismatch(question_index: Option<i64>, current_index: i64) -> bool {
    match question_index {
        Some(index) => index != current_index,
        None => true,
    }
}

/// Question select-option invalid gate (TS: `action === "select"` with
/// `Number.isNaN(optionIndex)` answers
/// `question.processing_error_callback`; a missing option arrives as
/// `None`).
pub fn is_question_option_invalid(is_select_action: bool, option_index: Option<i64>) -> bool {
    is_select_action && option_index.is_none()
}

/// `answerStep` drop gate (TS: `questionManager.getRequestID() !==
/// requestID || !questionManager.isActive()` leaves a poll the tap
/// race already ended as it is).
pub fn should_drop_answer_step(request_id_matches: bool, is_active: bool) -> bool {
    !request_id_matches || !is_active
}

/// `answerStep` branch (TS: the last question submits the whole poll,
/// any earlier one gives way to the next question; lastness arrives
/// from `is_last_question`, owned by slice 58).
pub fn decide_answer_step(is_last: bool) -> AnswerStep {
    if is_last {
        AnswerStep::SubmitPoll
    } else {
        AnswerStep::ShowNext
    }
}

/// `handleQuestionTextAnswer` decision (TS: no text is ignored, a tap
/// not waiting for custom input is told to use the custom button, a
/// single-select question holding a custom answer is already answered,
/// a single last question submits, a multiple question is re-shown
/// below the text, otherwise the next question shows).
pub fn decide_question_text_answer(
    has_text: bool,
    waiting_for_custom: bool,
    multiple: bool,
    has_custom_answer: bool,
    is_last: bool,
) -> QuestionTextDecision {
    if !has_text {
        return QuestionTextDecision::Ignored;
    }
    if !waiting_for_custom {
        return QuestionTextDecision::NeedCustomButton;
    }
    if !multiple && has_custom_answer {
        return QuestionTextDecision::AlreadyAnswered;
    }
    if !multiple && is_last {
        return QuestionTextDecision::SubmitPoll;
    }
    if multiple {
        return QuestionTextDecision::ReshowCurrent;
    }
    QuestionTextDecision::ShowNext
}

/// Cancel-poll context reject gate (TS: `!requestID || !directory ||
/// chatId === undefined` warns that the Cancel did not get through).
pub fn should_reject_dismissal_context(
    has_request_id: bool,
    has_directory: bool,
    has_chat_id: bool,
) -> bool {
    !has_request_id || !has_directory || !has_chat_id
}

/// `finishDismissal` map (TS: a dismissal for a poll something else
/// already ended is left as it is; a poll settled as answered or
/// cancelled outside closes that way; an accepted dismissal or a
/// settled-cancelled one closes cancelled; a failed one warns and
/// stays answerable).
pub fn decide_finish_dismissal(
    still_dismissing_same_request: bool,
    settled: Option<SettledKind>,
    result: DismissalResult,
) -> FinishDismissal {
    if !still_dismissing_same_request {
        return FinishDismissal::AlreadyClosed;
    }
    if settled == Some(SettledKind::Answered) {
        return FinishDismissal::SettledAnswered;
    }
    if result == DismissalResult::Gone {
        return FinishDismissal::SettledCancelled;
    }
    if result == DismissalResult::Accepted || settled == Some(SettledKind::Cancelled) {
        return FinishDismissal::CloseCancelled;
    }
    FinishDismissal::ShowWarning
}

/// Permission inactive gate (TS: `!permissionManager.isActive()`
/// clears and answers `permission.inactive_callback`).
pub fn should_reject_permission_when_inactive(is_active: bool) -> bool {
    !is_active
}

/// Permission active-message mismatch gate (TS:
/// `!permissionManager.isActiveMessage(callbackMessageId)` answers
/// `permission.inactive_callback`).
pub fn is_permission_message_mismatch(
    active_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (active_message_id, callback_message_id) {
        (Some(active), Some(callback)) => active != callback,
        _ => true,
    }
}

/// Permission request-list gate (TS:
/// `permissionManager.getRequestIDs(callbackMessageId).length === 0`
/// answers `permission.inactive_callback`).
pub fn has_permission_requests(count: usize) -> bool {
    count > 0
}

/// Permission bad-action reject gate (TS: a missing action or one
/// `isPermissionReply` rejects answers
/// `permission.processing_error_callback`; reply-value parsing lives
/// in slice 58 and arrives here as a bool).
pub fn should_reject_permission_on_bad_action(action_valid: bool) -> bool {
    !action_valid
}

/// Permission runtime-context gate (TS: `!directory || !chatId` clears
/// and answers `permission.no_active_request_callback`).
pub fn is_permission_runtime_context_valid(has_directory: bool, has_chat_id: bool) -> bool {
    has_directory && has_chat_id
}

/// Permission sending swallow gate (TS: `callbackMessageId === null
/// || !permissionManager.markSending(...)` answers empty and returns:
/// a second tap changes nothing while the first answer goes out).
pub fn should_swallow_permission_sending(
    has_callback_message: bool,
    mark_sending_ok: bool,
) -> bool {
    !has_callback_message || !mark_sending_ok
}

/// Reject settles-session gate (TS: on V2 a reject settles every
/// pending request of the session).
pub fn settles_session_on_reject(reply_is_reject: bool, server_is_v2: bool) -> bool {
    reply_is_reject && server_is_v2
}

/// Failed-permission finish map (TS: `failSending` with an outcome
/// applies prompt changes, a change without one shows the delivery
/// warning, no change leaves the prompt as it is).
pub fn decide_failed_permission_finish(
    has_fail_change: bool,
    change_has_outcome: bool,
) -> FinishPermission {
    if !has_fail_change {
        return FinishPermission::NoChange;
    }
    if change_has_outcome {
        return FinishPermission::ApplyChanges;
    }
    FinishPermission::ShowWarning
}

/// End-session-prompts gate (TS: an accepted answer that settles the
/// session also ends that session's other prompts with a reject).
pub fn should_end_session_prompts(
    accepted_count: usize,
    settles_session: bool,
    has_session_id: bool,
) -> bool {
    accepted_count > 0 && settles_session && has_session_id
}
