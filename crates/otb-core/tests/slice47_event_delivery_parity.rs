//! Parity port of the event-delivery pure gates (slice 47, RED).
//!
//! Mirrors representative cases from:
//! - `src/bot/events/telegram-event-delivery.ts`
//!   (`getLowerCaseErrorMessage` lower-casing,
//!   `isMessageNotModifiedError` tolerated-edit classifier,
//!   `isMessageToDeleteMissingError` tolerated-delete classifier,
//!   `TelegramEventDelivery` draft-id numbering 1, 2, ... + reset)
//! - `tests/bot/events/telegram-event-delivery.test.ts` (tolerated edit +
//!   missing deletes, 429/other failures NOT tolerated, draft ids
//!   `[1, 2, 1]` across reset)
//! - `src/bot/events/background-notice-delivery.ts` (`formatShortSessionId`
//!   short id, `getBackgroundSessionLabel` title branch,
//!   `formatBackgroundSessionNotification` kind -> message mapping)
//! - `tests/bot/events/background-notice-delivery.test.ts` (each of the
//!   three kinds sends; title label used)
//! - `src/bot/events/failed-tool-event.ts` (`buildFailedToolEvent`
//!   synthetic lost-call event)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::event_delivery_gates::*;
use std::collections::BTreeMap;

// --- lower_case_error_message (01-04) ---

#[test]
fn slice47_01_uppercase_message_lowercased() {
    assert_eq!(
        lower_case_error_message("Bad Request: MESSAGE IS NOT MODIFIED"),
        "bad request: message is not modified"
    );
}

#[test]
fn slice47_02_mixed_case_with_numbers_lowercased() {
    assert_eq!(
        lower_case_error_message("Too Many Requests: Retry After 3"),
        "too many requests: retry after 3"
    );
}

#[test]
fn slice47_03_already_lowercase_unchanged() {
    assert_eq!(
        lower_case_error_message("message to delete not found"),
        "message to delete not found"
    );
}

#[test]
fn slice47_04_empty_stays_empty() {
    assert_eq!(lower_case_error_message(""), "");
}

// --- is_message_not_modified_message (05-14) ---

#[test]
fn slice47_05_exact_phrase_is_tolerated() {
    assert!(is_message_not_modified_message("message is not modified"));
}

#[test]
fn slice47_06_bad_request_prefix_is_tolerated() {
    assert!(is_message_not_modified_message(
        "Bad Request: message is not modified"
    ));
}

#[test]
fn slice47_07_uppercase_phrase_is_tolerated() {
    assert!(is_message_not_modified_message("MESSAGE IS NOT MODIFIED"));
}

#[test]
fn slice47_08_telegram_long_form_is_tolerated() {
    assert!(is_message_not_modified_message(
        "Bad Request: message is not modified: specified new message content and reply markup are exactly the same",
    ));
}

#[test]
fn slice47_09_delete_missing_is_not_not_modified() {
    assert!(!is_message_not_modified_message(
        "Bad Request: message to delete not found"
    ));
}

#[test]
fn slice47_10_identifier_missing_is_not_not_modified() {
    assert!(!is_message_not_modified_message(
        "Bad Request: message identifier is not specified"
    ));
}

#[test]
fn slice47_11_empty_is_not_not_modified() {
    assert!(!is_message_not_modified_message(""));
}

#[test]
fn slice47_12_modified_without_not_is_not_tolerated() {
    assert!(!is_message_not_modified_message(
        "Bad Request: message is modified"
    ));
}

#[test]
fn slice47_13_chat_not_found_is_not_tolerated() {
    assert!(!is_message_not_modified_message("Bad Request: chat not found"));
}

#[test]
fn slice47_14_rate_limit_is_not_tolerated() {
    assert!(!is_message_not_modified_message(
        "Too Many Requests: retry after 3"
    ));
}

// --- is_message_to_delete_missing_message (15-24) ---

#[test]
fn slice47_15_delete_not_found_is_tolerated() {
    assert!(is_message_to_delete_missing_message(
        "message to delete not found"
    ));
}

#[test]
fn slice47_16_delete_not_found_with_prefix_is_tolerated() {
    assert!(is_message_to_delete_missing_message(
        "Bad Request: message to delete not found"
    ));
}

#[test]
fn slice47_17_delete_not_found_uppercase_is_tolerated() {
    assert!(is_message_to_delete_missing_message(
        "MESSAGE TO DELETE NOT FOUND"
    ));
}

#[test]
fn slice47_18_identifier_missing_is_tolerated() {
    assert!(is_message_to_delete_missing_message(
        "message identifier is not specified"
    ));
}

#[test]
fn slice47_19_identifier_missing_with_prefix_is_tolerated() {
    assert!(is_message_to_delete_missing_message(
        "Bad Request: message identifier is not specified"
    ));
}

#[test]
fn slice47_20_identifier_missing_uppercase_is_tolerated() {
    assert!(is_message_to_delete_missing_message(
        "MESSAGE IDENTIFIER IS NOT SPECIFIED"
    ));
}

#[test]
fn slice47_21_not_modified_is_not_delete_missing() {
    assert!(!is_message_to_delete_missing_message(
        "Bad Request: message is not modified"
    ));
}

#[test]
fn slice47_22_empty_is_not_delete_missing() {
    assert!(!is_message_to_delete_missing_message(""));
}

#[test]
fn slice47_23_other_not_found_is_not_tolerated() {
    assert!(!is_message_to_delete_missing_message(
        "Bad Request: message to edit not found"
    ));
}

#[test]
fn slice47_24_rate_limit_is_not_delete_missing() {
    assert!(!is_message_to_delete_missing_message(
        "Too Many Requests: retry after 3"
    ));
}

// --- DraftIdSequence (25-28) ---

#[test]
fn slice47_25_first_draft_id_is_one() {
    assert_eq!(DraftIdSequence::new().next_draft_id(), 1);
}

#[test]
fn slice47_26_draft_ids_increment_in_sequence() {
    let mut seq = DraftIdSequence::new();
    assert_eq!(seq.next_draft_id(), 1);
    assert_eq!(seq.next_draft_id(), 2);
    assert_eq!(seq.next_draft_id(), 3);
}

#[test]
fn slice47_27_reset_returns_to_one() {
    let mut seq = DraftIdSequence::new();
    assert_eq!(seq.next_draft_id(), 1);
    assert_eq!(seq.next_draft_id(), 2);
    seq.reset();
    assert_eq!(seq.next_draft_id(), 1);
}

#[test]
fn slice47_28_sequences_are_independent() {
    let mut first = DraftIdSequence::new();
    let mut second = DraftIdSequence::new();
    assert_eq!(first.next_draft_id(), 1);
    assert_eq!(first.next_draft_id(), 2);
    assert_eq!(second.next_draft_id(), 1);
}

// --- format_short_session_id (29-34) ---

#[test]
fn slice47_29_empty_stays_empty() {
    assert_eq!(format_short_session_id(""), "");
}

#[test]
fn slice47_30_short_id_kept_whole() {
    assert_eq!(format_short_session_id("abc"), "abc");
}

#[test]
fn slice47_31_exactly_eight_kept_whole() {
    assert_eq!(format_short_session_id("12345678"), "12345678");
}

#[test]
fn slice47_32_nine_chars_truncated_to_eight() {
    assert_eq!(format_short_session_id("session-1"), "session-");
}

#[test]
fn slice47_33_long_id_truncated_to_eight() {
    assert_eq!(format_short_session_id("ses_abcdef123456"), "ses_abcd");
}

#[test]
fn slice47_34_uuid_truncated_to_eight() {
    assert_eq!(
        format_short_session_id("550e8400-e29b-41d4-a716-446655440000"),
        "550e8400"
    );
}

// --- select_background_session_title (35-39) ---

#[test]
fn slice47_35_plain_title_selected() {
    assert_eq!(
        select_background_session_title(Some("Other")),
        Some("Other".to_string())
    );
}

#[test]
fn slice47_36_padded_title_trimmed() {
    assert_eq!(
        select_background_session_title(Some("  Other  ")),
        Some("Other".to_string())
    );
}

#[test]
fn slice47_37_whitespace_only_title_is_none() {
    assert_eq!(select_background_session_title(Some("   ")), None);
}

#[test]
fn slice47_38_empty_title_is_none() {
    assert_eq!(select_background_session_title(Some("")), None);
}

#[test]
fn slice47_39_missing_title_is_none() {
    assert_eq!(select_background_session_title(None), None);
}

// --- background_notice_message_key (40-42) ---

#[test]
fn slice47_40_assistant_response_key() {
    assert_eq!(
        background_notice_message_key(BackgroundNoticeKind::AssistantResponse),
        "background.assistant_response"
    );
}

#[test]
fn slice47_41_question_asked_key() {
    assert_eq!(
        background_notice_message_key(BackgroundNoticeKind::QuestionAsked),
        "background.question_asked"
    );
}

#[test]
fn slice47_42_permission_asked_key() {
    assert_eq!(
        background_notice_message_key(BackgroundNoticeKind::PermissionAsked),
        "background.permission_asked"
    );
}

// --- build_failed_tool_event (43-49) ---

fn lost_call() -> FailedToolCall {
    FailedToolCall {
        session_id: "ses-1".to_string(),
        message_id: "msg-1".to_string(),
        call_id: "call-7".to_string(),
        tool: "write".to_string(),
        state_start: Some(100),
        input: None,
        metadata: None,
    }
}

#[test]
fn slice47_43_id_marks_call_as_lost() {
    assert_eq!(build_failed_tool_event(&lost_call(), 200).id, "call-7:lost");
}

#[test]
fn slice47_44_type_status_and_empty_error() {
    let event = build_failed_tool_event(&lost_call(), 200);
    assert_eq!(event.event_type, "message.part.updated");
    assert_eq!(event.status, "error");
    assert_eq!(event.error, "");
}

#[test]
fn slice47_45_session_message_call_tool_pass_through() {
    let event = build_failed_tool_event(&lost_call(), 200);
    assert_eq!(event.session_id, "ses-1");
    assert_eq!(event.message_id, "msg-1");
    assert_eq!(event.call_id, "call-7");
    assert_eq!(event.tool, "write");
}

#[test]
fn slice47_46_start_comes_from_tool_state_end_is_now() {
    let event = build_failed_tool_event(&lost_call(), 200);
    assert_eq!(event.start, 100);
    assert_eq!(event.end, 200);
}

#[test]
fn slice47_47_start_falls_back_to_now_without_state_time() {
    let mut call = lost_call();
    call.state_start = None;
    let event = build_failed_tool_event(&call, 200);
    assert_eq!(event.start, 200);
    assert_eq!(event.end, 200);
}

#[test]
fn slice47_48_missing_input_defaults_to_empty() {
    let event = build_failed_tool_event(&lost_call(), 200);
    assert!(event.input.is_empty());
    assert!(!event.has_metadata);
}

#[test]
fn slice47_49_provided_input_and_metadata_carried_over() {
    let mut call = lost_call();
    call.input = Some(BTreeMap::from([("path".to_string(), "a.ts".to_string())]));
    call.metadata = Some(BTreeMap::from([("source".to_string(), "bot".to_string())]));
    let event = build_failed_tool_event(&call, 200);
    assert_eq!(event.input.get("path"), Some(&"a.ts".to_string()));
    assert!(event.has_metadata);
}
