//! Slice 101 parity: event-handler dispatch gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/events/handlers/handler-context.ts` (`SUBAGENT_STREAM_PREFIX`,
//! `SESSION_MESSAGE_MAX_LENGTH`, reply-keyboard initialized gate,
//! `formatSessionMessage` trim / unknown-error fallback / truncation),
//! `src/bot/events/handlers/run-close.ts` (`closeForegroundRun` footer
//! state + identity gates),
//! `src/bot/events/handlers/dashboard-handler.ts` (dashboard initialized
//! gate, zero-token intermediate skip, `contextLimit > 0` keyboard gate,
//! compact diff routing),
//! `src/bot/events/handlers/goal-handler.ts` (snapshot initialized gate,
//! notice destination/foreground gate),
//! `src/bot/events/handlers/interaction-handler.ts` (`getFollowedSessionId`
//! subagent-to-root gate, released-poll generation drop, in-flight ending
//! drop, `startQuestions` wait branch, replacing-poll gate, question-error
//! dispatch, settled waiting/sending/cancelled branches, stale-permission
//! drop, question-kind permission wait, waiting-request kind dispatch,
//! session-run-ended empty-changes skip) and
//! `src/bot/events/handlers/assistant-response-handler.ts` (inbox-pickup
//! queue-close, inbox-run start and quote-echo branches).
//!
//! Already covered elsewhere and NOT re-tested here:
//! assistant/thinking rendering, streaming internals, poll/permission menus,
//! and the `summaryAggregator` subscription plumbing itself.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `event_handler_gates` is ported (only the constant-value tests pass).

use otb_core::event_handler_gates::*;

// ---- constants (4) ----

#[test]
fn subagent_stream_prefix_value_parity() {
    assert_eq!(SUBAGENT_STREAM_PREFIX, "🧩");
}

#[test]
fn session_message_max_length_value_parity() {
    assert_eq!(SESSION_MESSAGE_MAX_LENGTH, 3500);
}

#[test]
fn unknown_error_key_value_parity() {
    assert_eq!(SESSION_MESSAGE_UNKNOWN_ERROR_KEY, "common.unknown_error");
}

#[test]
fn truncation_suffix_value_parity() {
    assert_eq!(SESSION_MESSAGE_TRUNCATION_SUFFIX, "...");
}

// ---- formatSessionMessage (5) ----

#[test]
fn session_message_trims_parity() {
    assert_eq!(
        format_session_message("  hello  ", "oops"),
        "hello".to_string()
    );
}

#[test]
fn session_message_empty_uses_fallback_parity() {
    assert_eq!(format_session_message("", "oops"), "oops".to_string());
}

#[test]
fn session_message_blank_uses_fallback_parity() {
    assert_eq!(
        format_session_message("   \n  ", "oops"),
        "oops".to_string()
    );
}

#[test]
fn session_message_truncates_over_limit_parity() {
    let long = "a".repeat(SESSION_MESSAGE_MAX_LENGTH + 1);
    let cut = format_session_message(&long, "oops");
    assert_eq!(cut.len(), SESSION_MESSAGE_MAX_LENGTH);
    assert!(cut.ends_with("..."));
    let longer = "a".repeat(SESSION_MESSAGE_MAX_LENGTH + 100);
    assert_eq!(
        format_session_message(&longer, "oops").len(),
        SESSION_MESSAGE_MAX_LENGTH
    );
}

#[test]
fn session_message_at_limit_passes_through_parity() {
    let exact = "b".repeat(SESSION_MESSAGE_MAX_LENGTH);
    assert_eq!(format_session_message(&exact, "oops"), exact);
    assert_eq!(format_session_message("hi", "oops"), "hi".to_string());
}

// ---- getReplyKeyboard (2) ----

#[test]
fn reply_keyboard_when_initialized_parity() {
    assert!(should_return_reply_keyboard(true));
}

#[test]
fn reply_keyboard_when_uninitialized_parity() {
    assert!(!should_return_reply_keyboard(false));
}

// ---- getFollowedSessionId (3) ----

#[test]
fn followed_session_direct_parity() {
    assert_eq!(resolve_followed_session(false, "s1", "root"), "s1");
}

#[test]
fn followed_session_subagent_uses_root_parity() {
    assert_eq!(resolve_followed_session(true, "sub", "root"), "root");
}

#[test]
fn followed_session_ignores_root_when_direct_parity() {
    assert_eq!(resolve_followed_session(false, "s1", "other"), "s1");
}

// ---- closeForegroundRun footer (6) ----

#[test]
fn run_footer_sends_when_complete_parity() {
    assert!(should_send_run_footer(true, true, false, true, true, true));
}

#[test]
fn run_footer_skips_when_hidden_parity() {
    assert!(!should_send_run_footer(
        false, true, false, true, true, true
    ));
}

#[test]
fn run_footer_skips_without_response_parity() {
    assert!(!should_send_run_footer(
        true, false, false, true, true, true
    ));
}

#[test]
fn run_footer_skips_when_suppressed_parity() {
    assert!(!should_send_run_footer(true, true, true, true, true, true));
}

#[test]
fn run_footer_skips_without_agent_parity() {
    assert!(!should_send_run_footer(
        true, true, false, false, true, true
    ));
}

#[test]
fn run_footer_skips_without_provider_or_model_parity() {
    assert!(!should_send_run_footer(
        true, true, false, true, false, true
    ));
    assert!(!should_send_run_footer(
        true, true, false, true, true, false
    ));
}

// ---- dashboard tokens (6) ----

#[test]
fn dashboard_handles_only_when_initialized_parity() {
    assert!(should_handle_dashboard_update(true));
    assert!(!should_handle_dashboard_update(false));
}

#[test]
fn dashboard_skips_zero_intermediate_parity() {
    assert!(should_skip_zero_token_update(false, 0));
}

#[test]
fn dashboard_keeps_completed_zero_parity() {
    assert!(!should_skip_zero_token_update(true, 0));
}

#[test]
fn dashboard_keeps_nonzero_intermediate_parity() {
    assert!(!should_skip_zero_token_update(false, 12));
}

#[test]
fn dashboard_updates_keyboard_only_with_limit_parity() {
    assert!(should_update_keyboard_context(200_000));
    assert!(!should_update_keyboard_context(0));
}

#[test]
fn dashboard_routes_diff_to_compact_parity() {
    assert!(should_route_session_diff_to_compact(true));
    assert!(!should_route_session_diff_to_compact(false));
}

// ---- goal (4) ----

#[test]
fn goal_snapshot_refreshes_only_when_initialized_parity() {
    assert!(should_refresh_goal_snapshot(true));
    assert!(!should_refresh_goal_snapshot(false));
}

#[test]
fn goal_notice_sends_to_foreground_parity() {
    assert!(should_send_goal_notice(true, true));
}

#[test]
fn goal_notice_skips_without_destination_parity() {
    assert!(!should_send_goal_notice(false, true));
}

#[test]
fn goal_notice_skips_when_background_parity() {
    assert!(!should_send_goal_notice(true, false));
    assert!(!should_send_goal_notice(false, false));
}

// ---- presentQuestion guards (5) ----

#[test]
fn waiting_poll_drops_after_reset_parity() {
    assert!(should_drop_waiting_poll(Some(3), 4));
}

#[test]
fn waiting_poll_keeps_current_generation_parity() {
    assert!(!should_drop_waiting_poll(Some(4), 4));
}

#[test]
fn waiting_poll_live_path_keeps_parity() {
    assert!(!should_drop_waiting_poll(None, 4));
}

#[test]
fn poll_drops_when_question_ended_parity() {
    assert!(should_drop_poll_with_ending(true));
    assert!(!should_drop_poll_with_ending(false));
}

#[test]
fn question_waits_when_start_fails_parity() {
    assert!(should_wait_question_after_start(false));
    assert!(!should_wait_question_after_start(true));
}

// ---- replacing + error + settled (8) ----

#[test]
fn replacing_poll_same_session_parity() {
    assert!(is_replacing_poll(true, true));
}

#[test]
fn replacing_poll_needs_active_same_session_parity() {
    assert!(!is_replacing_poll(false, true));
    assert!(!is_replacing_poll(true, false));
    assert!(!is_replacing_poll(false, false));
}

#[test]
fn question_error_drops_waiting_when_inactive_parity() {
    assert_eq!(
        resolve_question_error_action(false, true, false),
        QuestionErrorAction::DropWaiting
    );
    assert_eq!(
        resolve_question_error_action(true, false, false),
        QuestionErrorAction::DropWaiting
    );
}

#[test]
fn question_error_ignores_settling_parity() {
    assert_eq!(
        resolve_question_error_action(true, true, true),
        QuestionErrorAction::IgnoreSettling
    );
}

#[test]
fn question_error_clears_active_poll_parity() {
    assert_eq!(
        resolve_question_error_action(true, true, false),
        QuestionErrorAction::ClearAndDelete
    );
}

#[test]
fn settled_waiting_and_sending_notes_parity() {
    assert!(should_drop_waiting_settled(false, true));
    assert!(should_drop_waiting_settled(true, false));
    assert!(!should_drop_waiting_settled(true, true));
    assert!(should_note_settled_while_sending(true));
    assert!(!should_note_settled_while_sending(false));
}

#[test]
fn cancelled_closes_as_cancelled_parity() {
    assert!(should_close_cancelled_as_cancelled(true, true));
}

#[test]
fn cancelled_falls_back_without_failed_cancel_parity() {
    assert!(!should_close_cancelled_as_cancelled(false, true));
    assert!(!should_close_cancelled_as_cancelled(true, false));
    assert!(!should_close_cancelled_as_cancelled(false, false));
}

// ---- permission + waiting-request + run-ended (4) ----

#[test]
fn stale_permission_drops_parity() {
    assert!(should_drop_stale_permission(true));
    assert!(!should_drop_stale_permission(false));
}

#[test]
fn permission_waits_behind_question_parity() {
    assert!(should_wait_permission_for_question(true));
    assert!(!should_wait_permission_for_question(false));
}

#[test]
fn waiting_request_kind_dispatch_parity() {
    assert!(is_waiting_question_request(true));
    assert!(!is_waiting_question_request(false));
}

#[test]
fn session_run_ended_applies_with_changes_parity() {
    assert!(should_apply_session_run_ended_changes(2));
    assert!(!should_apply_session_run_ended_changes(0));
}

// ---- inbox pickup (3) ----

#[test]
fn inbox_close_run_on_queue_pickup_parity() {
    assert!(should_close_run_on_inbox_pickup(true, true));
    assert!(!should_close_run_on_inbox_pickup(false, true));
    assert!(!should_close_run_on_inbox_pickup(true, false));
}

#[test]
fn inbox_starts_own_run_parity() {
    assert!(should_start_inbox_run(false, true));
    assert!(!should_start_inbox_run(true, true));
    assert!(!should_start_inbox_run(false, false));
}

#[test]
fn inbox_echoes_quote_parity() {
    assert!(should_echo_inbox_prompt(true));
    assert!(!should_echo_inbox_prompt(false));
}
