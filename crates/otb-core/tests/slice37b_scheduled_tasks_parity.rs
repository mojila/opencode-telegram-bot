//! Parity port of scheduled-task scheduler-core / store-pure / run-once remainder
//! (slice 37b, RED).
//!
//! Mirrors representative `it`s from:
//! - `tests/app/services/scheduled-task-runtime-service.test.ts`
//!   (64-char prompt preview, elapsed-ms, timer clamp, schedule/overdue/
//!   activation gates, running-interrupted recovery, disabled-cron nulling,
//!   cron recompute, once backfill/invalid-runAt, success/error next-run
//!   resolution, once removal, flush/start guards)
//! - `tests/app/services/scheduled-task-executor-service.test.ts` remainder
//!   (response-text join, timeout regex/prefix, execution-timeout message/ms,
//!   docs-URL normalization, finish-reason, tool-calls await, empty/idle/
//!   startup caps, session-active, interactive-session match)
//! - `tests/app/services/scheduled-task-next-run-service.test.ts` remainder
//!   (once null gate, disabled-cron not-due gate)
//! - `tests/app/services/scheduled-task-schedule-parser-service.test.ts`
//!   remainder (fenced-first order, brace slice, error passthrough,
//!   summary/kind/cron/runAt gates, model/variant inclusion)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::scheduled_tasks::*;

// --- runtime prompt / elapsed (runtime-service: normalize/calculate) ---

#[test]
fn slice37b_01_short_prompt_passes_through() {
    assert_eq!(normalize_task_prompt("Send report"), "Send report");
}

#[test]
fn slice37b_02_prompt_collapses_whitespace() {
    assert_eq!(normalize_task_prompt("Send  report\nnow"), "Send report now");
}

#[test]
fn slice37b_03_long_prompt_truncates_to_64_with_ellipsis() {
    let long = "x".repeat(100);
    let out = normalize_task_prompt(&long);
    assert_eq!(out.len(), TASK_DESCRIPTION_PREVIEW_LENGTH + 3);
    assert!(out.ends_with("..."));
}

#[test]
fn slice37b_04_elapsed_ms_is_difference() {
    assert_eq!(calculate_elapsed_ms(Some(1000), Some(61_000)), 60_000);
}

#[test]
fn slice37b_05_elapsed_ms_zero_when_missing() {
    assert_eq!(calculate_elapsed_ms(None, Some(1000)), 0);
    assert_eq!(calculate_elapsed_ms(Some(1000), None), 0);
}

// --- timer clamp + schedule gates ---

#[test]
fn slice37b_06_max_timer_delay_const() {
    assert_eq!(MAX_TIMER_DELAY_MS, 2_147_483_647);
}

#[test]
fn slice37b_07_clamp_caps_large_delay() {
    assert_eq!(clamp_timer_delay(9_999_999_999), MAX_TIMER_DELAY_MS);
}

#[test]
fn slice37b_08_clamp_keeps_small_delay() {
    assert_eq!(clamp_timer_delay(5000), 5000);
}

#[test]
fn slice37b_09_disabled_cron_does_not_schedule() {
    assert!(!should_schedule_task("cron", false, true));
    assert!(!should_schedule_task("cron", false, false));
}

#[test]
fn slice37b_10_enabled_cron_with_next_run_schedules() {
    assert!(should_schedule_task("cron", true, true));
    assert!(!should_schedule_task("cron", true, false));
}

#[test]
fn slice37b_11_once_with_next_run_schedules() {
    assert!(should_schedule_task("once", true, true));
    assert!(!should_schedule_task("once", true, false));
}

#[test]
fn slice37b_12_overdue_is_non_positive_delay() {
    assert!(is_overdue(0));
    assert!(is_overdue(-1));
    assert!(!is_overdue(1));
}

#[test]
fn slice37b_13_overdue_activation_defers_to_refresh() {
    assert!(overdue_cron_activation_defers(true, true, true));
    assert!(!overdue_cron_activation_defers(true, false, true));
    assert!(!overdue_cron_activation_defers(false, true, true));
    assert!(!overdue_cron_activation_defers(true, true, false));
}

// --- startup recovery decisions ---

#[test]
fn slice37b_14_running_status_marked_interrupted() {
    assert!(should_mark_running_interrupted("running"));
    assert!(!should_mark_running_interrupted("success"));
    assert!(!should_mark_running_interrupted("idle"));
}

#[test]
fn slice37b_15_restart_error_const() {
    assert_eq!(
        RESTART_INTERRUPTED_ERROR,
        "Interrupted by bot restart during scheduled task execution."
    );
}

#[test]
fn slice37b_16_disabled_cron_next_run_nulled() {
    assert!(should_null_disabled_cron_next_run("cron", false));
    assert!(!should_null_disabled_cron_next_run("cron", true));
    assert!(!should_null_disabled_cron_next_run("once", false));
}

#[test]
fn slice37b_17_cron_recompute_when_missing_or_invalid() {
    assert!(should_recompute_cron_next_run(false, false, false));
    assert!(should_recompute_cron_next_run(true, false, false));
    assert!(should_recompute_cron_next_run(true, true, true));
    assert!(!should_recompute_cron_next_run(true, true, false));
}

#[test]
fn slice37b_18_once_backfill_only_for_idle_without_next_run() {
    assert!(should_backfill_once_next_run(false, "idle"));
    assert!(!should_backfill_once_next_run(true, "idle"));
    assert!(!should_backfill_once_next_run(false, "success"));
}

#[test]
fn slice37b_19_invalid_once_run_at_gate() {
    assert!(is_invalid_once_run_at(false, false));
    assert!(is_invalid_once_run_at(true, false));
    assert!(!is_invalid_once_run_at(true, true));
}

// --- success / error next-run resolution + once removal ---

#[test]
fn slice37b_20_success_disabled_cron_is_null() {
    assert_eq!(next_run_after_success("cron", false, false), "null");
}

#[test]
fn slice37b_21_success_activation_pending_keeps() {
    assert_eq!(next_run_after_success("cron", true, true), "keep");
}

#[test]
fn slice37b_22_success_cron_computes_next() {
    assert_eq!(next_run_after_success("cron", true, false), "computed");
}

#[test]
fn slice37b_23_error_once_is_null() {
    assert_eq!(next_run_after_error("once", true, false), "null");
}

#[test]
fn slice37b_24_error_disabled_cron_is_null() {
    assert_eq!(next_run_after_error("cron", false, false), "null");
}

#[test]
fn slice37b_25_error_activation_pending_keeps() {
    assert_eq!(next_run_after_error("cron", true, true), "keep");
}

#[test]
fn slice37b_26_once_removed_after_success() {
    assert!(should_remove_task_after_success("once"));
    assert!(!should_remove_task_after_success("cron"));
}

// --- flush / start guards ---

#[test]
fn slice37b_27_flush_skipped_when_blocked() {
    assert!(should_skip_flush(true, true, true, false, false));
    assert!(should_skip_flush(false, false, true, false, false));
    assert!(should_skip_flush(false, true, false, false, false));
    assert!(should_skip_flush(false, true, true, true, false));
    assert!(should_skip_flush(false, true, true, false, true));
}

#[test]
fn slice37b_28_flush_proceeds_when_ready() {
    assert!(!should_skip_flush(false, true, true, false, false));
}

#[test]
fn slice37b_29_start_blocked_reasons() {
    assert_eq!(
        start_execution_blocked(true, true, true),
        Some("already_running")
    );
    assert_eq!(start_execution_blocked(false, false, true), Some("missing"));
    assert_eq!(start_execution_blocked(false, true, false), Some("not_due"));
}

#[test]
fn slice37b_30_start_allowed_when_ready() {
    assert_eq!(start_execution_blocked(false, true, true), None);
}

// --- store pure remainder ---

#[test]
fn slice37b_31_toggle_disable_nulls_next_run() {
    assert!(cron_toggle_next_run_is_null(false));
    assert!(!cron_toggle_next_run_is_null(true));
}

#[test]
fn slice37b_32_remove_changed_gate() {
    assert!(store_remove_changed(2, 1));
    assert!(!store_remove_changed(1, 1));
}

// --- run-once gates (next-run-service remainder) ---

#[test]
fn slice37b_33_once_null_when_past_or_missing() {
    assert!(once_next_run_is_null(Some(1000), 2000));
    assert!(once_next_run_is_null(Some(2000), 2000));
    assert!(once_next_run_is_null(None, 2000));
}

#[test]
fn slice37b_34_once_present_when_future() {
    assert!(!once_next_run_is_null(Some(3000), 2000));
}

#[test]
fn slice37b_35_disabled_cron_never_due() {
    assert!(!is_cron_task_due(false, Some(1000), 2000));
    assert!(!is_cron_task_due(false, None, 2000));
}

#[test]
fn slice37b_36_enabled_cron_due_gate() {
    assert!(is_cron_task_due(true, Some(1000), 2000));
    assert!(is_cron_task_due(true, Some(2000), 2000));
    assert!(!is_cron_task_due(true, Some(3000), 2000));
    assert!(!is_cron_task_due(true, None, 2000));
}

// --- executor pure remainder ---

#[test]
fn slice37b_37_collect_response_text_joins_non_ignored() {
    assert_eq!(
        collect_response_text(&[("text", "hello ", false), ("text", "world", false)]),
        "hello world"
    );
    assert_eq!(
        collect_response_text(&[("text", "keep", false), ("text", "skip", true)]),
        "keep"
    );
    assert_eq!(
        collect_response_text(&[("tool", "x", false), ("text", "  hi  ", false)]),
        "hi"
    );
}

#[test]
fn slice37b_38_timeout_regex_gate() {
    assert!(is_timeout_error_message("Request timed out after 300000ms"));
    assert!(is_timeout_error_message("Model request TIMEOUT"));
    assert!(is_timeout_error_message("deadline exceeded"));
    assert!(is_timeout_error_message("request aborted"));
    assert!(!is_timeout_error_message("create failed"));
}

#[test]
fn slice37b_39_bot_timeout_prefix_gate() {
    assert!(is_bot_execution_timeout_message(
        "Scheduled task exceeded bot execution timeout after 120 minutes."
    ));
    assert!(!is_bot_execution_timeout_message("Request timed out"));
}

#[test]
fn slice37b_40_execution_timeout_message_and_ms() {
    assert_eq!(
        create_execution_timeout_message(120),
        "Scheduled task exceeded bot execution timeout after 120 minutes."
    );
    assert_eq!(execution_timeout_ms(120), 120 * 60 * 1000);
    assert_eq!(EXECUTION_POLL_INTERVAL_MS, 2000);
    assert_eq!(MAX_IDLE_POLLS_WITHOUT_RESULT, 3);
    assert_eq!(MAX_STARTUP_POLLS_WITHOUT_ACTIVITY, 45);
}

#[test]
fn slice37b_41_normalize_error_appends_docs_once() {
    let out = normalize_scheduled_task_error("Request timed out after 300000ms");
    assert!(out.contains(MODELS_DOCS_URL));
    let passthrough = "Request timed out. Check OpenCode model timeout settings: https://opencode.ai/docs/config/#models";
    assert_eq!(normalize_scheduled_task_error(passthrough), passthrough);
}

#[test]
fn slice37b_42_finish_reason_prefers_step_finish() {
    assert_eq!(
        finish_reason_from_parts(Some("stop"), Some("other")),
        Some("stop".to_string())
    );
    assert_eq!(
        finish_reason_from_parts(None, Some("stop")),
        Some("stop".to_string())
    );
    assert_eq!(finish_reason_from_parts(None, None), None);
    assert_eq!(finish_reason_from_parts(Some("   "), Some("  ")), None);
}

#[test]
fn slice37b_43_tool_calls_awaits_final_reply() {
    assert!(assistant_completed_is_awaiting_tool(true, Some("tool-calls")));
    assert!(!assistant_completed_is_awaiting_tool(true, Some("stop")));
    assert!(!assistant_completed_is_awaiting_tool(false, Some("tool-calls")));
}

#[test]
fn slice37b_44_empty_idle_startup_caps() {
    assert_eq!(MAX_COMPLETED_EMPTY_RESULT_RECHECKS, 3);
    assert!(should_throw_empty_after_rechecks(4));
    assert!(!should_throw_empty_after_rechecks(3));
    assert!(should_throw_idle_timeout(3));
    assert!(!should_throw_idle_timeout(2));
    assert!(should_throw_startup_timeout(45));
    assert!(!should_throw_startup_timeout(44));
    assert_eq!(COMPLETED_EMPTY_RESULT_RECHECK_INTERVAL_MS, 500);
}

#[test]
fn slice37b_45_session_active_and_interactive_match() {
    assert!(session_is_active(true, "busy"));
    assert!(!session_is_active(true, "idle"));
    assert!(!session_is_active(false, "busy"));
    assert!(interactive_request_matches("session-1", "session-1"));
    assert!(!interactive_request_matches("other", "session-1"));
    assert_eq!(SCHEDULED_TASK_SESSION_TITLE, "Scheduled task run");
    assert_eq!(
        INTERACTIVE_PERMISSION_REJECT_MESSAGE,
        "Scheduled task cannot continue because it requires interactive permission."
    );
}

// --- schedule-parser pure remainder ---

#[test]
fn slice37b_46_json_candidate_and_error_gates() {
    assert!(fenced_candidate_is_first(true));
    assert!(!fenced_candidate_is_first(false));
    assert!(brace_slice_present(0, 10));
    assert!(!brace_slice_present(-1, 10));
    assert!(!brace_slice_present(5, 5));
    assert!(is_schedule_error_payload(true, true));
    assert!(!is_schedule_error_payload(true, false));
    assert!(!is_schedule_error_payload(false, true));
}

#[test]
fn slice37b_47_schedule_validation_and_prompt_gates() {
    assert!(is_valid_schedule_summary(Some("Every 5 minutes")));
    assert!(!is_valid_schedule_summary(Some("   ")));
    assert!(!is_valid_schedule_summary(None));
    assert!(is_valid_schedule_kind("cron"));
    assert!(is_valid_schedule_kind("once"));
    assert!(!is_valid_schedule_kind("banana"));
    assert!(schedule_cron_missing(None));
    assert!(schedule_cron_missing(Some("   ")));
    assert!(!schedule_cron_missing(Some("*/5 * * * *")));
    assert!(schedule_run_at_invalid(false));
    assert!(!schedule_run_at_invalid(true));
    assert!(should_include_model_in_prompt(true, true));
    assert!(!should_include_model_in_prompt(true, false));
    assert!(!should_include_model_in_prompt(false, true));
    assert!(should_include_variant_in_prompt(true));
    assert!(!should_include_variant_in_prompt(false));
}
