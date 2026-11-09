//! Slice 109 parity: V2 client pure gates (RED).
//!
//! Mirrors the pure TS contract in `src/opencode/v2/client.ts`
//! (`HEARTBEAT_INTERVAL_MS`, `MESSAGE_PAGE_SIZE`, `MAX_MESSAGE_PAGES`,
//! `SESSION_PAGE_SIZE`, `COMPLETION_POLL_INTERVAL_MS`,
//! `COMPLETION_POLL_ATTEMPTS`, `SERVER_IDENTITY_TIMEOUT_MS`,
//! `isNotFound` declared/undeclared branches, `isUnsupportedGoalError`
//! command/rpc/404 branches, `parseModelRef` split, `listMessages`
//! `min(limit * 3, 200)` cap, message/session cursor break gates,
//! `admitPrompt` system preface + `"queue"` default, `runCommand`
//! skill `text || "/" + command`, `heartbeat:<now>` id, unknown
//! question/permission and no-completed-reply messages,
//! `applySelection` skip/agent/model gates, `goal.get`
//! `output ?? null` gate). Mapper bodies (`v2_mappers`) and stream
//! translation (`v2_event_translator_gates`) are NOT re-tested here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `v2_client_gates` is ported (only the constant-value tests pass).

use otb_core::v2_client_gates::*;

// ---- constants (7) ----

#[test]
fn heartbeat_interval_value_parity() {
    assert_eq!(HEARTBEAT_INTERVAL_MS, 10_000);
}

#[test]
fn message_page_size_value_parity() {
    assert_eq!(MESSAGE_PAGE_SIZE, 200);
}

#[test]
fn max_message_pages_value_parity() {
    assert_eq!(MAX_MESSAGE_PAGES, 50);
}

#[test]
fn session_page_size_value_parity() {
    assert_eq!(SESSION_PAGE_SIZE, 100);
}

#[test]
fn completion_poll_interval_value_parity() {
    assert_eq!(COMPLETION_POLL_INTERVAL_MS, 250);
}

#[test]
fn completion_poll_attempts_value_parity() {
    assert_eq!(COMPLETION_POLL_ATTEMPTS, 40);
}

#[test]
fn server_identity_timeout_value_parity() {
    assert_eq!(SERVER_IDENTITY_TIMEOUT_MS, 2_000);
}

// ---- isNotFound (3) ----

#[test]
fn not_found_declared_name_parity() {
    assert!(is_not_found_name("SessionNotFoundError"));
    assert!(!is_not_found_name("ClientError"));
}

#[test]
fn not_found_client_404_parity() {
    assert!(is_not_found_client_status(
        "ClientError",
        "UnexpectedStatus",
        404
    ));
}

#[test]
fn not_found_client_non_404_drops_parity() {
    assert!(!is_not_found_client_status(
        "ClientError",
        "UnexpectedStatus",
        500
    ));
    assert!(!is_not_found_client_status(
        "Other",
        "UnexpectedStatus",
        404
    ));
}

// ---- unsupported goal (4) ----

#[test]
fn goal_command_not_found_parity() {
    assert!(is_goal_command_not_found("CommandNotFoundError", ""));
    assert!(is_goal_command_not_found("Other", "CommandNotFoundError"));
    assert!(is_goal_command_not_found("MyCommandNotFoundError", ""));
}

#[test]
fn goal_command_other_names_drop_parity() {
    assert!(!is_goal_command_not_found("ClientError", ""));
}

#[test]
fn goal_rpc_unavailable_parity() {
    assert!(is_goal_rpc_unavailable("rpc.method_not_found"));
    assert!(is_goal_rpc_unavailable("rpc.unavailable"));
    assert!(!is_goal_rpc_unavailable("rpc.ok"));
}

#[test]
fn goal_404_rpc_only_parity() {
    assert!(is_goal_404_for_rpc(
        true,
        "ClientError",
        "UnexpectedStatus",
        404
    ));
    assert!(!is_goal_404_for_rpc(
        false,
        "ClientError",
        "UnexpectedStatus",
        404
    ));
    assert!(!is_goal_404_for_rpc(
        true,
        "ClientError",
        "UnexpectedStatus",
        500
    ));
}

// ---- parseModelRef (3) ----

#[test]
fn model_ref_splits_provider_and_id_parity() {
    assert_eq!(
        parse_model_ref(Some("openai/gpt")),
        Some(("openai".to_string(), "gpt".to_string()))
    );
}

#[test]
fn model_ref_misses_without_separator_parity() {
    assert_eq!(parse_model_ref(Some("gpt")), None);
    assert_eq!(parse_model_ref(Some("/gpt")), None);
    assert_eq!(parse_model_ref(None), None);
}

#[test]
fn model_ref_misses_when_blank_parity() {
    assert_eq!(parse_model_ref(Some("")), None);
}

// ---- paging (4) ----

#[test]
fn message_page_limit_triples_small_requests_parity() {
    assert_eq!(message_page_limit(10), 30);
}

#[test]
fn message_page_limit_caps_at_page_size_parity() {
    assert_eq!(message_page_limit(100), 200);
    assert_eq!(message_page_limit(200), 200);
}

#[test]
fn message_pages_break_without_next_or_empty_parity() {
    assert!(should_break_message_pages(false, false));
    assert!(should_break_message_pages(true, true));
    assert!(!should_break_message_pages(true, false));
}

#[test]
fn session_list_breaks_on_seen_or_empty_parity() {
    assert!(should_break_session_list(false, false, false));
    assert!(should_break_session_list(true, true, false));
    assert!(should_break_session_list(true, false, true));
    assert!(!should_break_session_list(true, false, false));
}

// ---- prompt text / delivery / skill (4) ----

#[test]
fn system_preface_joins_with_blank_line_parity() {
    assert_eq!(
        resolve_prompt_text(Some("sys"), "hi"),
        "sys\n\nhi".to_string()
    );
}

#[test]
fn missing_system_passes_text_through_parity() {
    assert_eq!(resolve_prompt_text(None, "hi"), "hi".to_string());
    assert_eq!(resolve_prompt_text(Some(""), "hi"), "hi".to_string());
}

#[test]
fn delivery_defaults_to_queue_parity() {
    assert_eq!(default_delivery(None), "queue");
    assert_eq!(default_delivery(Some("steer")), "steer");
}

#[test]
fn skill_text_falls_back_to_command_parity() {
    assert_eq!(skill_prompt_text("goal", "do it"), "do it".to_string());
    assert_eq!(skill_prompt_text("goal", ""), "/goal".to_string());
}

// ---- ids / messages (4) ----

#[test]
fn heartbeat_id_uses_now_parity() {
    assert_eq!(heartbeat_event_id(42), "heartbeat:42".to_string());
}

#[test]
fn unknown_question_message_parity() {
    assert_eq!(
        unknown_question_message("q1"),
        "Unknown question request: q1".to_string()
    );
}

#[test]
fn unknown_permission_message_parity() {
    assert_eq!(
        unknown_permission_message("p1"),
        "Unknown permission request: p1".to_string()
    );
}

#[test]
fn no_completed_reply_message_parity() {
    assert_eq!(
        no_completed_reply_message("ses-1"),
        "No completed assistant reply in session ses-1".to_string()
    );
}

// ---- applySelection (4) ----

#[test]
fn selection_skips_without_agent_and_model_parity() {
    assert!(should_skip_apply_selection(false, false));
    assert!(!should_skip_apply_selection(true, false));
    assert!(!should_skip_apply_selection(false, true));
}

#[test]
fn agent_switches_on_difference_parity() {
    assert!(should_switch_agent(Some("build"), Some("plan")));
    assert!(!should_switch_agent(Some("build"), Some("build")));
    assert!(!should_switch_agent(Some("build"), None));
    assert!(!should_switch_agent(Some("build"), Some("")));
}

#[test]
fn model_switches_on_id_or_provider_difference_parity() {
    assert!(should_switch_model(
        Some("a"),
        Some("p"),
        None,
        true,
        "b",
        "p",
        None
    ));
    assert!(should_switch_model(
        Some("a"),
        Some("p"),
        None,
        true,
        "a",
        "q",
        None
    ));
    assert!(!should_switch_model(
        Some("a"),
        Some("p"),
        None,
        false,
        "b",
        "q",
        None
    ));
}

#[test]
fn model_switches_on_defined_variant_difference_parity() {
    assert!(should_switch_model(
        Some("a"),
        Some("p"),
        Some("v1"),
        true,
        "a",
        "p",
        Some("v2")
    ));
    assert!(!should_switch_model(
        Some("a"),
        Some("p"),
        Some("v1"),
        true,
        "a",
        "p",
        None
    ));
    assert!(!should_switch_model(
        Some("a"),
        Some("p"),
        Some("v1"),
        true,
        "a",
        "p",
        Some("v1")
    ));
}

// ---- goal output (1) ----

#[test]
fn goal_missing_output_is_null_parity() {
    assert!(goal_output_is_null(false));
    assert!(!goal_output_is_null(true));
}
