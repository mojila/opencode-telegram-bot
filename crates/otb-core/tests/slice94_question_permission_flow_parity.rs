//! Slice 94 parity: question + permission callback flow-state gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/question-callback-handler.ts`
//! (`getCallbackMessageId` numeric-`message_id` gate, the inactive
//! question gate, the active-message mismatch gate, the
//! settling-from-Telegram swallow gate, the `NaN`-or-current-index
//! mismatch gate, the select-with-`NaN`-option invalid gate, the
//! `answerStep` request-ID-plus-active drop gate and last-question
//! submit-vs-show-next branch, the `handleQuestionTextAnswer` no-text /
//! not-waiting / single-with-custom / single-last / multiple /
//! single-next decision, the cancel-poll missing-context reject gate,
//! the `finishDismissal` already-closed / settled / gone / accepted /
//! failed map) and
//! `src/bot/callbacks/permission-callback-handler.ts`
//! (`getCallbackMessageId` numeric-`message_id` gate, the inactive
//! gate, the active-message mismatch gate, the empty-`requestIDs`
//! gate, the bad-action reject gate, the missing-directory/chat
//! context gate, the null-message-or-`markSending` swallow gate, the
//! reject-on-V2 settles-session gate, the failed `failSending`
//! apply-vs-warning map, the end-session-prompts gate).
//! `parse_question_callback` / `is_question_callback` /
//! `is_last_question` / `is_permission_reply_value` /
//! `classify_permission_callback` / `is_permission_callback` /
//! `is_permission_request_not_found` and the `question:` /
//! `permission:` data constants already live in
//! `question_permission_inline_gates` and are NOT re-tested here;
//! manager reads arrive here as explicit bools/enums.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `question_permission_flow_gates` is ported (only the constant-value
//! tests pass).

use otb_core::question_permission_flow_gates::*;

// ---- toast-key constants (5) ----

#[test]
fn question_inactive_key_value_parity() {
    assert_eq!(QUESTION_INACTIVE_TOAST_KEY, "question.inactive_callback");
}

#[test]
fn question_processing_key_value_parity() {
    assert_eq!(
        QUESTION_PROCESSING_TOAST_KEY,
        "question.processing_error_callback"
    );
}

#[test]
fn permission_inactive_key_value_parity() {
    assert_eq!(PERMISSION_INACTIVE_TOAST_KEY, "permission.inactive_callback");
}

#[test]
fn permission_processing_key_value_parity() {
    assert_eq!(
        PERMISSION_PROCESSING_TOAST_KEY,
        "permission.processing_error_callback"
    );
}

#[test]
fn permission_no_request_key_value_parity() {
    assert_eq!(
        PERMISSION_NO_REQUEST_TOAST_KEY,
        "permission.no_active_request_callback"
    );
}

// ---- callback message-id resolve (6) ----

#[test]
fn question_callback_message_id_some_parity() {
    assert_eq!(question_callback_message_id(true, Some(7)), Some(7));
}

#[test]
fn question_callback_message_id_no_message_parity() {
    assert_eq!(question_callback_message_id(false, Some(7)), None);
}

#[test]
fn question_callback_message_id_missing_id_parity() {
    assert_eq!(question_callback_message_id(true, None), None);
}

#[test]
fn permission_callback_message_id_some_parity() {
    assert_eq!(permission_callback_message_id(true, Some(9)), Some(9));
}

#[test]
fn permission_callback_message_id_no_message_parity() {
    assert_eq!(permission_callback_message_id(false, Some(9)), None);
}

#[test]
fn permission_callback_message_id_missing_id_parity() {
    assert_eq!(permission_callback_message_id(true, None), None);
}

// ---- question active / message / settling gates (12) ----

#[test]
fn rejects_question_when_inactive_parity() {
    assert!(should_reject_question_when_inactive(false));
}

#[test]
fn keeps_question_when_active_parity() {
    assert!(!should_reject_question_when_inactive(true));
}

#[test]
fn question_message_match_equal_parity() {
    assert!(!is_question_message_mismatch(Some(7), Some(7)));
}

#[test]
fn question_message_mismatch_differ_parity() {
    assert!(is_question_message_mismatch(Some(7), Some(8)));
}

#[test]
fn question_message_mismatch_missing_parity() {
    assert!(is_question_message_mismatch(None, Some(7)));
    assert!(is_question_message_mismatch(Some(7), None));
    assert!(is_question_message_mismatch(None, None));
}

#[test]
fn swallows_question_while_settling_parity() {
    assert!(should_swallow_question_settling(true));
}

#[test]
fn proceeds_question_when_not_settling_parity() {
    assert!(!should_swallow_question_settling(false));
}

#[test]
fn question_index_match_current_parity() {
    assert!(!is_question_index_mismatch(Some(2), 2));
}

#[test]
fn question_index_mismatch_other_parity() {
    assert!(is_question_index_mismatch(Some(2), 3));
}

#[test]
fn question_index_mismatch_nan_parity() {
    assert!(is_question_index_mismatch(None, 2));
}

#[test]
fn question_option_invalid_select_missing_parity() {
    assert!(is_question_option_invalid(true, None));
}

#[test]
fn question_option_valid_cases_parity() {
    assert!(!is_question_option_invalid(true, Some(0)));
    assert!(!is_question_option_invalid(false, None));
}

// ---- answerStep gates (4) ----

#[test]
fn drops_answer_step_on_request_mismatch_or_inactive_parity() {
    assert!(should_drop_answer_step(false, true));
    assert!(should_drop_answer_step(true, false));
}

#[test]
fn keeps_answer_step_when_match_and_active_parity() {
    assert!(!should_drop_answer_step(true, true));
}

#[test]
fn submits_poll_when_last_parity() {
    assert_eq!(decide_answer_step(true), AnswerStep::SubmitPoll);
}

#[test]
fn shows_next_when_not_last_parity() {
    assert_eq!(decide_answer_step(false), AnswerStep::ShowNext);
}

// ---- question text-answer decision (6) ----

#[test]
fn text_answer_ignored_without_text_parity() {
    assert_eq!(
        decide_question_text_answer(false, true, false, false, true),
        QuestionTextDecision::Ignored
    );
}

#[test]
fn text_answer_needs_custom_button_when_not_waiting_parity() {
    assert_eq!(
        decide_question_text_answer(true, false, false, false, true),
        QuestionTextDecision::NeedCustomButton
    );
}

#[test]
fn text_answer_already_answered_single_with_custom_parity() {
    assert_eq!(
        decide_question_text_answer(true, true, false, true, true),
        QuestionTextDecision::AlreadyAnswered
    );
}

#[test]
fn text_answer_submits_single_last_without_custom_parity() {
    assert_eq!(
        decide_question_text_answer(true, true, false, false, true),
        QuestionTextDecision::SubmitPoll
    );
}

#[test]
fn text_answer_reshows_current_when_multiple_parity() {
    assert_eq!(
        decide_question_text_answer(true, true, true, false, false),
        QuestionTextDecision::ReshowCurrent
    );
}

#[test]
fn text_answer_shows_next_single_non_last_parity() {
    assert_eq!(
        decide_question_text_answer(true, true, false, false, false),
        QuestionTextDecision::ShowNext
    );
}

// ---- cancel-poll context + finishDismissal (7) ----

#[test]
fn rejects_dismissal_context_when_missing_parity() {
    assert!(should_reject_dismissal_context(false, true, true));
    assert!(should_reject_dismissal_context(true, false, true));
    assert!(should_reject_dismissal_context(true, true, false));
}

#[test]
fn keeps_dismissal_context_when_full_parity() {
    assert!(!should_reject_dismissal_context(true, true, true));
}

#[test]
fn finish_dismissal_already_closed_parity() {
    assert_eq!(
        decide_finish_dismissal(false, None, DismissalResult::Accepted),
        FinishDismissal::AlreadyClosed
    );
}

#[test]
fn finish_dismissal_settled_answered_parity() {
    assert_eq!(
        decide_finish_dismissal(
            true,
            Some(SettledKind::Answered),
            DismissalResult::Failed
        ),
        FinishDismissal::SettledAnswered
    );
}

#[test]
fn finish_dismissal_gone_maps_settled_cancelled_parity() {
    assert_eq!(
        decide_finish_dismissal(true, None, DismissalResult::Gone),
        FinishDismissal::SettledCancelled
    );
}

#[test]
fn finish_dismissal_accepted_or_settled_cancelled_close_parity() {
    assert_eq!(
        decide_finish_dismissal(true, None, DismissalResult::Accepted),
        FinishDismissal::CloseCancelled
    );
    assert_eq!(
        decide_finish_dismissal(
            true,
            Some(SettledKind::Cancelled),
            DismissalResult::Failed
        ),
        FinishDismissal::CloseCancelled
    );
}

#[test]
fn finish_dismissal_failed_without_settlement_warns_parity() {
    assert_eq!(
        decide_finish_dismissal(true, None, DismissalResult::Failed),
        FinishDismissal::ShowWarning
    );
}

// ---- permission flow gates (10) ----

#[test]
fn rejects_permission_when_inactive_parity() {
    assert!(should_reject_permission_when_inactive(false));
    assert!(!should_reject_permission_when_inactive(true));
}

#[test]
fn permission_message_match_equal_parity() {
    assert!(!is_permission_message_mismatch(Some(4), Some(4)));
}

#[test]
fn permission_message_mismatch_parity() {
    assert!(is_permission_message_mismatch(Some(4), Some(5)));
    assert!(is_permission_message_mismatch(None, Some(4)));
    assert!(is_permission_message_mismatch(Some(4), None));
}

#[test]
fn permission_requests_gate_parity() {
    assert!(has_permission_requests(2));
    assert!(!has_permission_requests(0));
}

#[test]
fn permission_bad_action_gate_parity() {
    assert!(should_reject_permission_on_bad_action(false));
    assert!(!should_reject_permission_on_bad_action(true));
}

#[test]
fn permission_runtime_context_gate_parity() {
    assert!(is_permission_runtime_context_valid(true, true));
    assert!(!is_permission_runtime_context_valid(false, true));
    assert!(!is_permission_runtime_context_valid(true, false));
}

#[test]
fn swallows_permission_while_sending_parity() {
    assert!(should_swallow_permission_sending(false, true));
    assert!(should_swallow_permission_sending(true, false));
    assert!(!should_swallow_permission_sending(true, true));
}

#[test]
fn settles_session_on_reject_v2_parity() {
    assert!(settles_session_on_reject(true, true));
    assert!(!settles_session_on_reject(true, false));
    assert!(!settles_session_on_reject(false, true));
    assert!(!settles_session_on_reject(false, false));
}

#[test]
fn failed_permission_finish_map_parity() {
    assert_eq!(
        decide_failed_permission_finish(true, true),
        FinishPermission::ApplyChanges
    );
    assert_eq!(
        decide_failed_permission_finish(true, false),
        FinishPermission::ShowWarning
    );
    assert_eq!(
        decide_failed_permission_finish(false, false),
        FinishPermission::NoChange
    );
}

#[test]
fn end_session_prompts_gate_parity() {
    assert!(should_end_session_prompts(2, true, true));
    assert!(!should_end_session_prompts(0, true, true));
    assert!(!should_end_session_prompts(2, false, true));
    assert!(!should_end_session_prompts(2, true, false));
}
