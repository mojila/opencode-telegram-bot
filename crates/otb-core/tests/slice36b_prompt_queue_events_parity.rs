//! Parity port of prompt-queue-dispatch remainder + event-router remainder
//! + session-status remainder + busy/idle remainder (slice 36b, RED).
//!
//! Mirrors:
//! - `tests/bot/handlers/prompt-queue-dispatch.test.ts` (queue/full/button/
//!   command gates, arrival tickets, hand-over, dispatch guards)
//! - `tests/bot/handlers/prompt-queue-dispatch-v2.test.ts` (v2 inbox gate,
//!   steer-vs-queue delivery, full-queue refuse, delivered-first run gate)
//! - `tests/app/services/event-router.test.ts` (heartbeat, session-id
//!   priority, attached-busy marking, goal.changed gate, foreground arg)
//! - `tests/bot/events/session-runtime-state.test.ts` (session keys,
//!   thinking ids, background-subagent gate, delivered-text strip)
//! - `tests/app/services/busy-reconciliation-service.test.ts` (terminal
//!   status, grace period, throttle, active-stream clear gate)
//! - `tests/app/services/run-control-service.test.ts` (foreground busy
//!   OR, busy directories)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::prompt_queue_events::*;

// --- can_queue_media_prompt (4) ---

#[test]
fn slice36b_01_media_voice_queues_when_enabled() {
    assert!(can_queue_media_prompt(true, true, false, false, false));
}

#[test]
fn slice36b_02_media_photo_or_document_queues_when_enabled() {
    assert!(can_queue_media_prompt(true, false, false, true, false));
    assert!(can_queue_media_prompt(true, false, false, false, true));
    assert!(can_queue_media_prompt(true, false, true, false, false));
}

#[test]
fn slice36b_03_media_plain_text_does_not_queue_as_media() {
    assert!(!can_queue_media_prompt(true, false, false, false, false));
}

#[test]
fn slice36b_04_media_never_queues_when_setting_off() {
    assert!(!can_queue_media_prompt(false, true, false, false, false));
    assert!(!can_queue_media_prompt(false, false, false, true, true));
}

// --- uses_opencode_inbox + delivery (4) ---

#[test]
fn slice36b_05_v2_uses_opencode_inbox() {
    assert!(uses_opencode_inbox("v2"));
}

#[test]
fn slice36b_06_v1_does_not_use_opencode_inbox() {
    assert!(!uses_opencode_inbox("v1"));
    assert!(!uses_opencode_inbox(""));
}

#[test]
fn slice36b_07_steer_mode_selects_steer_delivery() {
    assert_eq!(select_inbox_delivery("steer"), InboxDelivery::Steer);
}

#[test]
fn slice36b_08_queue_and_off_modes_select_queue_delivery() {
    assert_eq!(select_inbox_delivery("queue"), InboxDelivery::Queue);
    assert_eq!(select_inbox_delivery("off"), InboxDelivery::Queue);
}

// --- arrival ticket + hand-over (4) ---

#[test]
fn slice36b_09_ticket_only_while_message_would_wait() {
    assert!(should_take_arrival_ticket(true, true, true));
}

#[test]
fn slice36b_10_ticket_not_taken_when_idle_or_disabled() {
    assert!(!should_take_arrival_ticket(true, true, false));
    assert!(!should_take_arrival_ticket(true, false, true));
    assert!(!should_take_arrival_ticket(false, true, true));
}

#[test]
fn slice36b_11_prepared_prompt_hands_over_after_detach() {
    assert!(should_hand_over_prepared(true, true));
}

#[test]
fn slice36b_12_prepared_prompt_keeps_normal_path_without_detach() {
    assert!(!should_hand_over_prepared(false, true));
    assert!(!should_hand_over_prepared(true, false));
}

// --- dispatch block guard (4) ---

#[test]
fn slice36b_13_dispatch_runs_when_idle_with_work() {
    assert!(!is_dispatch_blocked(false, 1, true, true, false));
}

#[test]
fn slice36b_14_dispatch_blocked_while_in_flight_or_busy() {
    assert!(is_dispatch_blocked(true, 2, true, true, false));
    assert!(is_dispatch_blocked(false, 1, true, true, true));
}

#[test]
fn slice36b_15_dispatch_blocked_when_queue_empty() {
    assert!(is_dispatch_blocked(false, 0, true, true, false));
}

#[test]
fn slice36b_16_dispatch_blocked_without_deps_or_context() {
    assert!(is_dispatch_blocked(false, 1, false, true, false));
    assert!(is_dispatch_blocked(false, 1, true, false, false));
}

// --- extract_event_session_id (4) ---

#[test]
fn slice36b_17_direct_session_id_wins() {
    assert_eq!(
        extract_event_session_id(Some("s-1"), Some("s-2"), Some("s-3")),
        Some("s-1".to_string())
    );
}

#[test]
fn slice36b_18_info_session_id_is_fallback() {
    assert_eq!(
        extract_event_session_id(None, Some("s-2"), Some("s-3")),
        Some("s-2".to_string())
    );
}

#[test]
fn slice36b_19_part_session_id_is_last_resort() {
    assert_eq!(
        extract_event_session_id(None, None, Some("s-3")),
        Some("s-3".to_string())
    );
}

#[test]
fn slice36b_20_no_session_when_event_carries_none() {
    assert_eq!(extract_event_session_id(None, None, None), None);
}

// --- should_mark_attached_busy (6) ---

#[test]
fn slice36b_21_status_busy_marks_attached_busy() {
    assert!(should_mark_attached_busy("session.status", None, false, true));
}

#[test]
fn slice36b_22_status_idle_does_not_mark_busy() {
    assert!(!should_mark_attached_busy("session.status", None, false, false));
}

#[test]
fn slice36b_23_assistant_update_without_completion_marks_busy() {
    assert!(should_mark_attached_busy(
        "message.updated",
        Some("assistant"),
        false,
        false
    ));
}

#[test]
fn slice36b_24_completed_or_user_update_does_not_mark_busy() {
    assert!(!should_mark_attached_busy(
        "message.updated",
        Some("assistant"),
        true,
        false
    ));
    assert!(!should_mark_attached_busy(
        "message.updated",
        Some("user"),
        false,
        false
    ));
}

#[test]
fn slice36b_25_part_and_question_marks_busy() {
    assert!(should_mark_attached_busy(
        "message.part.delta",
        None,
        false,
        false
    ));
    assert!(should_mark_attached_busy(
        "message.part.updated",
        None,
        false,
        false
    ));
    assert!(should_mark_attached_busy("question.asked", None, false, false));
    assert!(should_mark_attached_busy(
        "permission.asked",
        None,
        false,
        false
    ));
}

#[test]
fn slice36b_26_unrelated_event_does_not_mark_busy() {
    assert!(!should_mark_attached_busy("session.idle", None, false, false));
    assert!(!should_mark_attached_busy(
        "server.heartbeat",
        None,
        false,
        false
    ));
}

// --- goal.changed + background arg (3) ---

#[test]
fn slice36b_27_goal_changed_with_session_applies() {
    assert!(should_apply_goal_changed("goal.changed", true));
}

#[test]
fn slice36b_28_goal_changed_without_session_or_other_type_skips() {
    assert!(!should_apply_goal_changed("goal.changed", false));
    assert!(!should_apply_goal_changed("session.idle", true));
}

#[test]
fn slice36b_29_background_arg_only_for_foreground_session() {
    assert_eq!(
        background_observe_session(true, Some("session-1")),
        Some("session-1".to_string())
    );
    assert_eq!(background_observe_session(false, Some("session-2")), None);
    assert_eq!(background_observe_session(true, None), None);
}

// --- session keys + thinking id (3) ---

#[test]
fn slice36b_30_session_key_joins_with_colon() {
    assert_eq!(session_key("session-1", "call-1"), "session-1:call-1");
}

#[test]
fn slice36b_31_session_key_keeps_sessions_apart() {
    assert_ne!(
        session_key("session-1", "call-1"),
        session_key("session-2", "call-1")
    );
}

#[test]
fn slice36b_32_thinking_stream_id_prefixes_message() {
    assert_eq!(thinking_stream_id("message-1"), "thinking:message-1");
}

// --- background subagent + delivered text (6) ---

#[test]
fn slice36b_33_running_background_subagent_stays_followed() {
    assert!(is_running_background_subagent(true, "running"));
    assert!(is_running_background_subagent(true, "pending"));
}

#[test]
fn slice36b_34_foreground_or_done_subagent_not_background_running() {
    assert!(!is_running_background_subagent(false, "running"));
    assert!(!is_running_background_subagent(true, "completed"));
}

#[test]
fn slice36b_35_strip_without_delivery_returns_whole_text() {
    assert_eq!(strip_delivered_text(None, "Analysis"), "Analysis");
}

#[test]
fn slice36b_36_strip_removes_delivered_prefix() {
    assert_eq!(
        strip_delivered_text(Some("Analysis"), "Analysis\n\nMore"),
        "\n\nMore"
    );
}

#[test]
fn slice36b_37_strip_empty_remainder_collapses_to_empty() {
    assert_eq!(strip_delivered_text(Some("Analysis"), "Analysis\n\n"), "");
}

#[test]
fn slice36b_38_strip_rewritten_reply_keeps_whole_text() {
    assert_eq!(
        strip_delivered_text(Some("Analysis"), "Rewritten"),
        "Rewritten"
    );
}

// --- terminal status + grace + throttle (6) ---

#[test]
fn slice36b_39_missing_and_idle_and_error_are_terminal() {
    assert!(is_terminal_session_status(None));
    assert!(is_terminal_session_status(Some("idle")));
    assert!(is_terminal_session_status(Some("error")));
}

#[test]
fn slice36b_40_busy_status_is_not_terminal() {
    assert!(!is_terminal_session_status(Some("busy")));
}

#[test]
fn slice36b_41_fresh_busy_mark_stays_inside_grace() {
    assert!(is_within_foreground_grace(10_000, 11_000));
}

#[test]
fn slice36b_42_old_busy_mark_leaves_grace() {
    assert!(!is_within_foreground_grace(10_000, 13_000));
    assert!(!is_within_foreground_grace(10_000, 12_000));
}

#[test]
fn slice36b_43_throttle_skips_rapid_reconcile() {
    assert!(should_throttle_reconcile(Some(10_000), 10_001));
    assert!(!should_throttle_reconcile(Some(7_000), 10_001));
    assert!(!should_throttle_reconcile(None, 10_001));
}

#[test]
fn slice36b_44_throttle_boundary_allows_reconcile_at_ten_seconds() {
    assert!(!should_throttle_reconcile(Some(0), 10_000));
}

// --- foreground busy + clear gate + directories (4) ---

#[test]
fn slice36b_45_foreground_busy_is_either_side() {
    assert!(is_foreground_busy(true, false));
    assert!(is_foreground_busy(false, true));
    assert!(is_foreground_busy(true, true));
    assert!(!is_foreground_busy(false, false));
}

#[test]
fn slice36b_46_stale_busy_clears_only_when_terminal_and_idle_stream() {
    assert!(should_clear_stale_busy(true, false, false));
    assert!(!should_clear_stale_busy(false, false, false));
    assert!(!should_clear_stale_busy(true, true, false));
    assert!(!should_clear_stale_busy(true, false, true));
}

#[test]
fn slice36b_47_busy_directories_dedupe() {
    assert_eq!(
        dedupe_busy_directories(&["D:/repo", "D:/repo", "D:/other"]),
        vec!["D:/repo".to_string(), "D:/other".to_string()]
    );
}

#[test]
fn slice36b_48_busy_directories_empty_without_busy() {
    assert!(dedupe_busy_directories(&[]).is_empty());
}
