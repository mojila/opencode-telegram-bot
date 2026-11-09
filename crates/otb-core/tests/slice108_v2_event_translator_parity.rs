//! Slice 108 parity: V2 event-translator pure gates (RED).
//!
//! Mirrors the pure TS contract in `src/opencode/v2/events.ts`
//! (`createV2EventTranslator` + `translateGoalChanged`:
//! goal-changed emit gates and `?? null` fallback, `idleEvents` id pair
//! and interrupted/reason mark, `backgroundKey` shell/session/null rule
//! and the success-stays-running buffer gate, missing-tool-call drop,
//! inbox-delivery user/non-goal-internal filter and `${inboxID}:text`
//! part, `${messageID}:updated` / `${part.id}:${time}` / `${id}:error`
//! / `:text:` / `:reasoning:` / delta part ids, `location ??
//! remembered` directory attribution and remember gate, connect
//! tool-clear + `restarted` flag gate, execution-end
//! interrupted/failed mapping).
//! Mapper bodies (`toV1ToolName`, `toV1Question`, `isGoalInternalMetadata`
//! shape) live in `v2_mappers` / `event_transport_gates` and are NOT
//! re-tested here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `v2_event_translator_gates` is ported (only the constant-value tests pass).

use otb_core::v2_event_translator_gates::*;

// ---- constants (12) ----

#[test]
fn goal_changed_rpc_type_value_parity() {
    assert_eq!(V2_GOAL_CHANGED_RPC_TYPE, "rpc.opencode.goal.changed");
}

#[test]
fn goal_changed_event_type_value_parity() {
    assert_eq!(V1_GOAL_CHANGED_EVENT_TYPE, "goal.changed");
}

#[test]
fn session_status_event_type_value_parity() {
    assert_eq!(V1_SESSION_STATUS_EVENT_TYPE, "session.status");
}

#[test]
fn session_idle_event_type_value_parity() {
    assert_eq!(V1_SESSION_IDLE_EVENT_TYPE, "session.idle");
}

#[test]
fn message_updated_event_type_value_parity() {
    assert_eq!(V1_MESSAGE_UPDATED_EVENT_TYPE, "message.updated");
}

#[test]
fn part_updated_event_type_value_parity() {
    assert_eq!(V1_PART_UPDATED_EVENT_TYPE, "message.part.updated");
}

#[test]
fn part_delta_event_type_value_parity() {
    assert_eq!(V1_PART_DELTA_EVENT_TYPE, "message.part.delta");
}

#[test]
fn session_error_event_type_value_parity() {
    assert_eq!(V1_SESSION_ERROR_EVENT_TYPE, "session.error");
}

#[test]
fn session_compacted_event_type_value_parity() {
    assert_eq!(V1_SESSION_COMPACTED_EVENT_TYPE, "session.compacted");
}

#[test]
fn permission_asked_event_type_value_parity() {
    assert_eq!(V1_PERMISSION_ASKED_EVENT_TYPE, "permission.asked");
}

#[test]
fn question_asked_event_type_value_parity() {
    assert_eq!(V1_QUESTION_ASKED_EVENT_TYPE, "question.asked");
}

#[test]
fn question_rejected_event_type_value_parity() {
    assert_eq!(V1_QUESTION_REJECTED_EVENT_TYPE, "question.rejected");
}

// ---- should_emit_goal_changed (2) ----

#[test]
fn goal_changed_emits_for_string_session_parity() {
    assert!(should_emit_goal_changed(true, true));
}

#[test]
fn goal_changed_drops_other_type_or_session_parity() {
    assert!(!should_emit_goal_changed(false, true));
    assert!(!should_emit_goal_changed(true, false));
}

// ---- goal_changed_goal_is_null (1) ----

#[test]
fn goal_changed_missing_goal_is_null_parity() {
    assert!(goal_changed_goal_is_null(false));
    assert!(!goal_changed_goal_is_null(true));
}

// ---- idle id pair (2) ----

#[test]
fn idle_status_id_appends_status_parity() {
    assert_eq!(idle_status_event_id("e1"), "e1:status".to_string());
}

#[test]
fn idle_id_appends_idle_parity() {
    assert_eq!(idle_event_id("e1"), "e1:idle".to_string());
    assert_eq!(session_error_event_id("e9"), "e9:error".to_string());
}

// ---- assistant / part ids (2) ----

#[test]
fn assistant_updated_id_appends_updated_parity() {
    assert_eq!(assistant_updated_event_id("m1"), "m1:updated".to_string());
}

#[test]
fn part_updated_id_appends_time_parity() {
    assert_eq!(part_updated_event_id("p1", 7), "p1:7".to_string());
}

// ---- background_key (3) ----

#[test]
fn background_key_prefers_shell_parity() {
    assert_eq!(
        background_key(Some("s1"), Some("sess")),
        Some("shell:s1".to_string())
    );
}

#[test]
fn background_key_falls_back_to_session_parity() {
    assert_eq!(
        background_key(None, Some("sess")),
        Some("session:sess".to_string())
    );
    assert_eq!(
        background_key(Some(""), Some("sess")),
        Some("session:sess".to_string())
    );
}

#[test]
fn background_key_null_without_ids_parity() {
    assert_eq!(background_key(None, None), None);
    assert_eq!(background_key(Some(""), Some("")), None);
}

// ---- should_buffer_background_call (2) ----

#[test]
fn success_running_with_key_buffers_parity() {
    assert!(should_buffer_background_call(true, true, true));
}

#[test]
fn non_success_or_done_or_keyless_completes_parity() {
    assert!(!should_buffer_background_call(false, true, true));
    assert!(!should_buffer_background_call(true, false, true));
    assert!(!should_buffer_background_call(true, true, false));
}

// ---- should_drop_missing_tool_call (1) ----

#[test]
fn missing_tool_call_drops_parity() {
    assert!(should_drop_missing_tool_call(false));
    assert!(!should_drop_missing_tool_call(true));
}

// ---- should_emit_inbox_delivery (2) ----

#[test]
fn inbox_user_non_goal_item_emits_parity() {
    assert!(should_emit_inbox_delivery(true, true, false));
}

#[test]
fn inbox_missing_non_user_or_goal_item_drops_parity() {
    assert!(!should_emit_inbox_delivery(false, true, false));
    assert!(!should_emit_inbox_delivery(true, false, false));
    assert!(!should_emit_inbox_delivery(true, true, true));
}

// ---- inbox / step / delta part ids (3) ----

#[test]
fn inbox_text_part_id_parity() {
    assert_eq!(inbox_text_part_id("in1"), "in1:text".to_string());
}

#[test]
fn step_text_and_reasoning_ids_parity() {
    assert_eq!(step_text_part_id("m1", 2), "m1:text:2".to_string());
    assert_eq!(
        step_reasoning_part_id("m1", 3),
        "m1:reasoning:3".to_string()
    );
}

#[test]
fn delta_part_id_uses_kind_parity() {
    assert_eq!(delta_part_id("m1", "text", 1), "m1:text:1".to_string());
    assert_eq!(
        delta_part_id("m1", "reasoning", 1),
        "m1:reasoning:1".to_string()
    );
}

// ---- resolve_translator_directory (2) ----

#[test]
fn translator_location_wins_when_present_parity() {
    assert_eq!(
        resolve_translator_directory(Some("C:/repo"), Some("/old")),
        Some("C:/repo".to_string())
    );
}

#[test]
fn translator_falls_back_to_remembered_parity() {
    assert_eq!(
        resolve_translator_directory(None, Some("/old")),
        Some("/old".to_string())
    );
    assert_eq!(
        resolve_translator_directory(Some(""), Some("/old")),
        Some("/old".to_string())
    );
    assert_eq!(resolve_translator_directory(None, None), None);
}

// ---- should_remember_directory (1) ----

#[test]
fn remember_needs_session_and_directory_parity() {
    assert!(should_remember_directory(true, Some("C:/repo")));
    assert!(!should_remember_directory(false, Some("C:/repo")));
    assert!(!should_remember_directory(true, None));
    assert!(!should_remember_directory(true, Some("")));
}

// ---- connect gates (2) ----

#[test]
fn connect_clears_tools_parity() {
    assert!(should_clear_tools_on_connect(true));
    assert!(!should_clear_tools_on_connect(false));
}

#[test]
fn restarted_flag_present_only_when_boolean_parity() {
    assert!(!has_restarted_flag(None));
    assert!(has_restarted_flag(Some(true)));
    assert!(has_restarted_flag(Some(false)));
}

// ---- execution-end mapping (2) ----

#[test]
fn execution_interrupted_marks_idle_parity() {
    assert!(execution_end_is_interrupted(true));
    assert!(!execution_end_is_interrupted(false));
}

#[test]
fn execution_failed_takes_error_path_parity() {
    assert!(execution_end_is_failed(true));
    assert!(!execution_end_is_failed(false));
}
