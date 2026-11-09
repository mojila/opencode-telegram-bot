//! Slice 83 parity: scheduled-task callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/scheduled-task-callback-handler.ts`
//! (`handleTaskCallback` exact `task:retry-schedule` / `task:cancel`
//! dispatch, `isTaskInteraction` kind gate, `isTaskCallbackActive`
//! schedule/preview/prompt message-id membership, the combined inactive
//! gate, the stale orphaned-interaction clear gate,
//! `handleTaskListCallback` `tasklist:` dispatch guard,
//! `parseTaskListMetadata` kind/flow/stage/message-id/task-id gates, the
//! `list`-stage open gate, the `detail`-stage + task-id-equality
//! toggle/delete gates, the `deleteMessageIfPresent` chat-plus-numeric-id
//! gate, the retry-schedule gate, `buildTaskInteractionMetadata`, and
//! `truncatePromptForDetails` with `TASK_DETAIL_PROMPT_BYTE_BUDGET`).
//! Menu constants (`TASK_RETRY_SCHEDULE_CALLBACK`, `TASKLIST_*`),
//! id builds, and id parses already live in `recent_scheduled_gates`
//! and are NOT re-tested here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `task_callback_gates` is ported (only the constant-value test passes).

use otb_core::recent_scheduled_gates::{
    TASKLIST_OPEN_PREFIX, TASK_CANCEL_CALLBACK, TASK_RETRY_SCHEDULE_CALLBACK,
};
use otb_core::task_callback_gates::*;

fn flow() -> TaskFlowMessageIds {
    TaskFlowMessageIds {
        schedule_request_message_id: Some(11),
        preview_message_id: Some(12),
        prompt_request_message_id: Some(13),
    }
}

fn snapshot(
    kind: &str,
    flow_name: Option<&str>,
    stage: Option<&str>,
    message_id: Option<i64>,
    task_id: Option<&str>,
) -> TaskListSnapshot {
    TaskListSnapshot {
        kind: kind.to_string(),
        flow: flow_name.map(str::to_string),
        stage: stage.map(str::to_string),
        message_id,
        task_id: task_id.map(str::to_string),
    }
}

// ---- constant ----

#[test]
fn task_detail_prompt_byte_budget_value_parity() {
    assert_eq!(TASK_DETAIL_PROMPT_BYTE_BUDGET, 3400);
}

// ---- task-creation dispatch ----

#[test]
fn handles_retry_exact_parity() {
    assert!(handles_task_callback(Some(TASK_RETRY_SCHEDULE_CALLBACK)));
}

#[test]
fn handles_cancel_exact_parity() {
    assert!(handles_task_callback(Some(TASK_CANCEL_CALLBACK)));
}

#[test]
fn handles_rejects_suffix_and_other_prefixes_parity() {
    assert!(!handles_task_callback(Some("task:cancel:x")));
    assert!(!handles_task_callback(Some("task:retry-schedule:x")));
    assert!(!handles_task_callback(Some("tasklist:cancel")));
    assert!(!handles_task_callback(Some("task:")));
}

#[test]
fn handles_rejects_none_and_empty_parity() {
    assert!(!handles_task_callback(None));
    assert!(!handles_task_callback(Some("")));
}

// ---- task interaction kind ----

#[test]
fn task_interaction_kind_parity() {
    assert!(is_task_interaction(Some("task")));
}

#[test]
fn task_interaction_rejects_other_kinds_parity() {
    assert!(!is_task_interaction(Some("custom")));
    assert!(!is_task_interaction(Some("inline")));
    assert!(!is_task_interaction(None));
}

// ---- callback liveness ----

#[test]
fn task_active_on_schedule_slot_parity() {
    assert!(is_task_callback_active(&flow(), 11));
}

#[test]
fn task_active_on_preview_slot_parity() {
    assert!(is_task_callback_active(&flow(), 12));
}

#[test]
fn task_active_on_prompt_slot_parity() {
    assert!(is_task_callback_active(&flow(), 13));
}

#[test]
fn task_inactive_on_unknown_message_parity() {
    assert!(!is_task_callback_active(&flow(), 99));
    assert!(!is_task_callback_active(&flow(), 0));
}

// ---- combined task inactive gate ----

#[test]
fn task_inactive_when_no_flow_state_parity() {
    assert!(should_answer_task_inactive(false, true, true, true));
    assert!(should_answer_task_inactive(false, false, false, false));
}

#[test]
fn task_inactive_when_not_task_interaction_parity() {
    assert!(should_answer_task_inactive(true, false, true, true));
}

#[test]
fn task_inactive_when_no_callback_message_parity() {
    assert!(should_answer_task_inactive(true, true, false, true));
}

#[test]
fn task_inactive_when_message_not_owned_parity() {
    assert!(should_answer_task_inactive(true, true, true, false));
}

#[test]
fn task_active_when_all_good_parity() {
    assert!(!should_answer_task_inactive(true, true, true, true));
}

// ---- stale clear gate ----

#[test]
fn stale_cleared_when_orphaned_interaction_parity() {
    assert!(should_clear_stale_task_interaction(false, true));
}

#[test]
fn stale_kept_when_flow_alive_parity() {
    assert!(!should_clear_stale_task_interaction(true, true));
}

#[test]
fn stale_kept_without_interaction_parity() {
    assert!(!should_clear_stale_task_interaction(false, false));
    assert!(!should_clear_stale_task_interaction(true, false));
}

// ---- tasklist dispatch ----

#[test]
fn handles_tasklist_open_parity() {
    assert!(handles_tasklist_callback(Some(&format!(
        "{TASKLIST_OPEN_PREFIX}t1"
    ))));
}

#[test]
fn handles_tasklist_cancel_parity() {
    assert!(handles_tasklist_callback(Some("tasklist:cancel")));
}

#[test]
fn handles_tasklist_bare_prefix_parity() {
    // TS checks only `startsWith("tasklist:")`, so the bare prefix handles.
    assert!(handles_tasklist_callback(Some("tasklist:")));
}

#[test]
fn handles_tasklist_rejects_none_and_empty_parity() {
    assert!(!handles_tasklist_callback(None));
    assert!(!handles_tasklist_callback(Some("")));
}

#[test]
fn handles_tasklist_rejects_other_prefixes_parity() {
    assert!(!handles_tasklist_callback(Some("task:cancel")));
    assert!(!handles_tasklist_callback(Some("recent:0")));
    assert!(!handles_tasklist_callback(Some("tasklist")));
}

// ---- tasklist metadata parse ----

#[test]
fn tasklist_metadata_none_is_none_parity() {
    assert_eq!(parse_task_list_metadata(None), None);
}

#[test]
fn tasklist_metadata_wrong_kind_is_none_parity() {
    let s = snapshot("inline", Some("tasklist"), Some("list"), Some(7), None);
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
}

#[test]
fn tasklist_metadata_wrong_flow_is_none_parity() {
    let s = snapshot(
        "custom",
        Some("session-search"),
        Some("list"),
        Some(7),
        None,
    );
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
    let s = snapshot("custom", None, Some("list"), Some(7), None);
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
}

#[test]
fn tasklist_metadata_missing_message_id_is_none_parity() {
    let s = snapshot("custom", Some("tasklist"), Some("list"), None, None);
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
}

#[test]
fn tasklist_metadata_list_valid_parity() {
    let s = snapshot("custom", Some("tasklist"), Some("list"), Some(7), None);
    assert_eq!(
        parse_task_list_metadata(Some(&s)),
        Some(TaskListMetadata::List { message_id: 7 })
    );
}

#[test]
fn tasklist_metadata_detail_valid_parity() {
    let s = snapshot(
        "custom",
        Some("tasklist"),
        Some("detail"),
        Some(7),
        Some("t1"),
    );
    assert_eq!(
        parse_task_list_metadata(Some(&s)),
        Some(TaskListMetadata::Detail {
            message_id: 7,
            task_id: "t1".to_string(),
        })
    );
}

#[test]
fn tasklist_metadata_detail_missing_task_id_is_none_parity() {
    let s = snapshot("custom", Some("tasklist"), Some("detail"), Some(7), None);
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
    let s = snapshot(
        "custom",
        Some("tasklist"),
        Some("detail"),
        Some(7),
        Some(""),
    );
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
}

#[test]
fn tasklist_metadata_bad_stage_is_none_parity() {
    let s = snapshot(
        "custom",
        Some("tasklist"),
        Some("input"),
        Some(7),
        Some("t1"),
    );
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
    let s = snapshot("custom", Some("tasklist"), None, Some(7), Some("t1"));
    assert_eq!(parse_task_list_metadata(Some(&s)), None);
}

// ---- stage gates ----

#[test]
fn tasklist_list_stage_parity() {
    assert!(is_tasklist_list_stage(Some("list")));
    assert!(!is_tasklist_list_stage(Some("detail")));
    assert!(!is_tasklist_list_stage(None));
}

#[test]
fn tasklist_detail_stage_parity() {
    assert!(is_tasklist_detail_stage(Some("detail")));
    assert!(!is_tasklist_detail_stage(Some("list")));
    assert!(!is_tasklist_detail_stage(None));
}

// ---- combined tasklist inactive gate ----

#[test]
fn tasklist_inactive_when_no_metadata_parity() {
    assert!(should_answer_tasklist_inactive(false, true, true));
}

#[test]
fn tasklist_inactive_when_no_callback_message_parity() {
    assert!(should_answer_tasklist_inactive(true, false, true));
    assert!(should_answer_tasklist_inactive(true, false, false));
}

#[test]
fn tasklist_inactive_on_message_mismatch_parity() {
    assert!(should_answer_tasklist_inactive(true, true, false));
}

#[test]
fn tasklist_active_when_all_good_parity() {
    assert!(!should_answer_tasklist_inactive(true, true, true));
}

// ---- toggle / delete gates ----

#[test]
fn tasklist_toggle_allowed_parity() {
    assert!(should_allow_tasklist_toggle(true, true));
}

#[test]
fn tasklist_toggle_blocked_parity() {
    assert!(!should_allow_tasklist_toggle(false, true));
    assert!(!should_allow_tasklist_toggle(true, false));
    assert!(!should_allow_tasklist_toggle(false, false));
}

#[test]
fn tasklist_delete_allowed_parity() {
    assert!(should_allow_tasklist_delete(true, true));
}

#[test]
fn tasklist_delete_blocked_parity() {
    assert!(!should_allow_tasklist_delete(false, true));
    assert!(!should_allow_tasklist_delete(true, false));
}

// ---- delete-if-present gate ----

#[test]
fn delete_message_sent_when_chat_and_numeric_parity() {
    assert!(should_delete_message_if_present(true, true));
}

#[test]
fn delete_message_skipped_parity() {
    assert!(!should_delete_message_if_present(false, true));
    assert!(!should_delete_message_if_present(true, false));
    assert!(!should_delete_message_if_present(false, false));
}

// ---- retry-schedule gate ----

#[test]
fn retry_schedule_allowed_parity() {
    assert!(should_retry_task_schedule(true, true));
}

#[test]
fn retry_schedule_blocked_parity() {
    assert!(!should_retry_task_schedule(false, true));
    assert!(!should_retry_task_schedule(true, false));
    assert!(!should_retry_task_schedule(false, false));
}

// ---- metadata build ----

#[test]
fn build_task_interaction_metadata_parity() {
    assert_eq!(
        build_task_interaction_metadata("awaiting_schedule", "p1", "/w/p1", None),
        TaskInteractionMetadata {
            flow: "task".to_string(),
            stage: "awaiting_schedule".to_string(),
            project_id: "p1".to_string(),
            project_worktree: "/w/p1".to_string(),
            preview_message_id: None,
        }
    );
    assert_eq!(
        build_task_interaction_metadata("awaiting_prompt", "p1", "/w/p1", Some(12),)
            .preview_message_id,
        Some(12)
    );
}

// ---- prompt truncation ----

#[test]
fn truncate_short_prompt_passthrough_parity() {
    assert_eq!(truncate_prompt_for_details("hello"), "hello");
    assert_eq!(truncate_prompt_for_details(""), "");
}

#[test]
fn truncate_long_prompt_cut_parity() {
    let long = "a".repeat(4000);
    let out = truncate_prompt_for_details(&long);
    assert!(out.ends_with("..."));
    assert!(out.len() <= TASK_DETAIL_PROMPT_BYTE_BUDGET);
}

#[test]
fn truncate_multibyte_safe_parity() {
    // 2000 × 2-byte chars = 4000 bytes, over budget: cut keeps a char
    // boundary and the "..." suffix within budget.
    let text = "é".repeat(2000);
    let out = truncate_prompt_for_details(&text);
    assert!(out.ends_with("..."));
    assert!(out.len() <= TASK_DETAIL_PROMPT_BYTE_BUDGET);
    assert!(out.is_char_boundary(out.len()));
}
