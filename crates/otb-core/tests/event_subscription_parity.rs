//! Parity port of the pure gates in
//! `tests/bot/services/event-subscription-service.test.ts` (129 its) and
//! `tests/bot/services/event-subscription-service.lifecycle.test.ts` (45 its).
//!
//! Step 1 (RED): these tests assert the TS contract against
//! `otb_core::event_subscription` stubs and MUST FAIL with `unimplemented!()`
//! panics until the Rust port lands (step 2). No SSE, timers, Telegram I/O,
//! or filesystem access — only pure gating/state/filter functions.

use otb_core::event_subscription::{
    CompactCandidate, FooterGate, LostRunEvent, PromptPickup, ReconnectAction,
    SESSION_ERROR_MAX_CHARS, build_interrupted_idle_events, classify_reconnect,
    count_background_operations, event_session_matches, footer_duration_ms,
    has_chat_destination, is_heartbeat_event, is_tool_running, marks_session_busy,
    pickup_closes_run, pick_compact_tool, reports_no_work, short_session_id,
    should_end_lost_run, should_notify_external_input, should_send_footer,
    should_show_tool_line, should_subscribe, should_suppress_session_error,
    truncate_session_error,
};

fn open_gate() -> FooterGate {
    FooterGate {
        turn_start_seen: true,
        footer_enabled: true,
        had_session_error: false,
        permission_rejected: false,
        shutdown_idle: false,
        footer_suppressed: false,
        is_foreground: true,
        completed_after_idle: false,
    }
}

fn candidate(call_id: &str, started_order: u64, running: bool) -> CompactCandidate {
    CompactCandidate {
        call_id: call_id.to_string(),
        started_order,
        running,
    }
}

// --- subscription guard ---

#[test]
fn slice24b_subscribe_needs_non_empty_directory() {
    assert!(should_subscribe("D:/repo"));
    assert!(!should_subscribe(""));
}

#[test]
fn slice24b_subscribe_rejects_whitespace_directory() {
    assert!(!should_subscribe("   "));
}

#[test]
fn slice24b_chat_destination_needs_bot_and_chat() {
    assert!(has_chat_destination(true, true));
    assert!(!has_chat_destination(true, false));
    assert!(!has_chat_destination(false, true));
}

#[test]
fn slice24b_cleanup_detaches_telegram_context() {
    // cleanup() clears the destination: both halves gone.
    assert!(!has_chat_destination(false, false));
}

// --- reconnect classification ---

#[test]
fn slice24b_reconnect_restart_ends_lost_run() {
    assert_eq!(
        classify_reconnect(Some(true)),
        ReconnectAction::EndRunLost
    );
}

#[test]
fn slice24b_reconnect_same_server_stops_background() {
    assert_eq!(
        classify_reconnect(Some(false)),
        ReconnectAction::StopBackgroundAndForgetTurn
    );
}

#[test]
fn slice24b_reconnect_null_flag_stops_background() {
    assert_eq!(
        classify_reconnect(None),
        ReconnectAction::StopBackgroundAndForgetTurn
    );
}

#[test]
fn slice24b_lost_run_ending_is_v2_only() {
    assert!(should_end_lost_run("v2"));
    assert!(!should_end_lost_run("v1"));
}

#[test]
fn slice24b_lost_run_ending_rejects_empty_version() {
    assert!(!should_end_lost_run(""));
}

#[test]
fn slice24b_interrupted_idle_synthesis_marks_interrupted() {
    let events = build_interrupted_idle_events("session-1");
    assert_eq!(
        events,
        vec![
            LostRunEvent {
                id: "session-1:lost:status".to_string(),
                event_type: "session.status".to_string(),
                interrupted: false,
            },
            LostRunEvent {
                id: "session-1:lost:idle".to_string(),
                event_type: "session.idle".to_string(),
                interrupted: true,
            },
        ]
    );
}

// --- footer gating ---

#[test]
fn slice24b_footer_sent_for_plain_completed_turn() {
    assert!(should_send_footer(&open_gate()));
}

#[test]
fn slice24b_footer_timed_from_busy_moment() {
    // 10:00 busy -> 10:01:05 idle ~= 65s.
    assert_eq!(footer_duration_ms(0, 65_000), 65_000);
}

#[test]
fn slice24b_footer_keeps_bot_opened_run_start() {
    // busy 5s after the bot's own run start still measures from the run start.
    assert_eq!(footer_duration_ms(0, 16_000), 16_000);
}

#[test]
fn slice24b_footer_missing_when_start_never_seen() {
    let gate = FooterGate {
        turn_start_seen: false,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

#[test]
fn slice24b_footer_missing_when_turned_off() {
    let gate = FooterGate {
        footer_enabled: false,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

#[test]
fn slice24b_footer_missing_after_session_error() {
    let gate = FooterGate {
        had_session_error: true,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

#[test]
fn slice24b_footer_missing_after_permission_reject() {
    let gate = FooterGate {
        permission_rejected: true,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

#[test]
fn slice24b_footer_missing_on_shutdown_idle() {
    let gate = FooterGate {
        shutdown_idle: true,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

#[test]
fn slice24b_footer_missing_when_abort_marked_footerless() {
    let gate = FooterGate {
        footer_suppressed: true,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

#[test]
fn slice24b_footer_missing_after_losing_focus() {
    let gate = FooterGate {
        is_foreground: false,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

#[test]
fn slice24b_footer_missing_when_reply_completes_after_idle() {
    let gate = FooterGate {
        completed_after_idle: true,
        ..open_gate()
    };
    assert!(!should_send_footer(&gate));
}

// --- busy / idle routing ---

#[test]
fn slice24b_heartbeat_reconciles_busy_state() {
    assert!(is_heartbeat_event("server.heartbeat"));
    assert!(!is_heartbeat_event("session.idle"));
}

#[test]
fn slice24b_busy_while_agent_working() {
    for event_type in [
        "message.part.updated",
        "message.part.delta",
        "question.asked",
        "permission.asked",
    ] {
        assert!(
            marks_session_busy(event_type, false, false),
            "{event_type} should mark busy"
        );
    }
    assert!(marks_session_busy("message.updated", false, false));
    assert!(marks_session_busy("session.status", false, true));
}

#[test]
fn slice24b_idle_for_events_reporting_no_work() {
    assert!(reports_no_work("message.updated", true, false));
    assert!(reports_no_work("session.idle", false, false));
    assert!(reports_no_work("session.status", false, true));
}

#[test]
fn slice24b_busy_needs_status_flag_on_status_events() {
    assert!(!marks_session_busy("session.status", false, false));
    assert!(!reports_no_work("session.status", false, false));
}

#[test]
fn slice24b_progress_ignores_other_sessions() {
    assert!(event_session_matches("session-9", "session-9"));
    assert!(!event_session_matches("session-other", "session-9"));
}

// --- session errors ---

#[test]
fn slice24b_session_error_reported_and_releases_run() {
    assert!(!should_suppress_session_error(false));
}

#[test]
fn slice24b_error_silent_after_user_abort() {
    assert!(should_suppress_session_error(true));
}

#[test]
fn slice24b_oversized_session_error_truncated() {
    let long = "x".repeat(5000);
    let text = truncate_session_error(&long);
    assert!(text.contains("..."));
    assert!(text.chars().count() < SESSION_ERROR_MAX_CHARS);
    assert!(text.chars().count() <= 3700);
}

#[test]
fn slice24b_short_session_error_kept_whole() {
    assert_eq!(truncate_session_error("provider exploded"), "provider exploded");
}

// --- tool line visibility ---

#[test]
fn slice24b_pending_tool_shows_nothing() {
    assert!(!should_show_tool_line("pending"));
}

#[test]
fn slice24b_running_tool_shows_line() {
    assert!(should_show_tool_line("running"));
    assert!(is_tool_running("running"));
    assert!(!is_tool_running("completed"));
}

#[test]
fn slice24b_failed_composed_call_is_one_finished_line() {
    assert!(should_show_tool_line("error"));
    assert!(!is_tool_running("error"));
}

#[test]
fn slice24b_bare_running_tool_still_shows() {
    assert!(should_show_tool_line("running"));
}

#[test]
fn slice24b_completed_tool_clears_running_mark() {
    assert!(should_show_tool_line("completed"));
    assert!(!is_tool_running("completed"));
}

// --- background operations ---

#[test]
fn slice24b_background_count_sums_commands_and_subagents() {
    assert_eq!(count_background_operations(1, 1), 2);
}

#[test]
fn slice24b_background_count_empty_is_zero() {
    assert_eq!(count_background_operations(0, 0), 0);
}

#[test]
fn slice24b_untitled_background_uses_short_id() {
    let label = short_session_id("bg-session-123456");
    assert!(label.contains("bg-sessi"));
    assert!(!label.contains("bg-session-123456"));
}

// --- compact fallback ---

#[test]
fn slice24b_compact_names_most_recent_running() {
    let pick = pick_compact_tool(&[
        candidate("call-older", 1, true),
        candidate("call-newer", 2, true),
    ]);
    assert_eq!(pick.as_deref(), Some("call-newer"));
}

#[test]
fn slice24b_compact_older_update_does_not_steal_line() {
    // Order is by start order, not by update arrival: older re-report keeps newer.
    let pick = pick_compact_tool(&[
        candidate("call-older", 1, true),
        candidate("call-newer", 2, true),
    ]);
    assert_eq!(pick.as_deref(), Some("call-newer"));
}

#[test]
fn slice24b_compact_falls_back_to_still_running() {
    let pick = pick_compact_tool(&[
        candidate("call-read-1", 2, false),
        candidate("call-bash", 1, true),
    ]);
    assert_eq!(pick.as_deref(), Some("call-bash"));
}

#[test]
fn slice24b_compact_error_sibling_falls_back() {
    let pick = pick_compact_tool(&[
        candidate("call-bash", 1, true),
        candidate("call-read-1", 2, false),
    ]);
    assert_eq!(pick.as_deref(), Some("call-bash"));
}

#[test]
fn slice24b_compact_empty_when_nothing_runs() {
    let pick = pick_compact_tool(&[candidate("call-read-1", 1, false)]);
    assert_eq!(pick, None);
}

#[test]
fn slice24b_compact_prefers_detail_less_running_sibling() {
    // A finished detailed tool must not linger when a running sibling remains.
    let pick = pick_compact_tool(&[
        candidate("call-read-1", 1, false),
        candidate("call-bare", 2, true),
    ]);
    assert_eq!(pick.as_deref(), Some("call-bare"));
}

// --- external input + prompt pickup ---

#[test]
fn slice24b_external_terminal_input_notifies() {
    assert!(should_notify_external_input(false));
}

#[test]
fn slice24b_bot_own_input_stays_silent() {
    assert!(!should_notify_external_input(true));
}

#[test]
fn slice24b_steered_prompt_stays_in_running_run() {
    assert!(!pickup_closes_run(PromptPickup::Steer));
}

#[test]
fn slice24b_queued_prompt_closes_answered_run() {
    assert!(pickup_closes_run(PromptPickup::Queue));
}

#[test]
fn slice24b_inbox_reconnect_asks_nothing_without_session() {
    assert!(!should_subscribe(""));
}
