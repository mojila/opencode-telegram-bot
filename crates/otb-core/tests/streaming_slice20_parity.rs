//! Parity port of the streaming remainder:
//! `response-streamer.ts` (draft/edit routing, broken flush),
//! `tool-call-streamer.ts` (line flush), `compact-progress-streamer.ts`
//! (hold/release), `stream-throttle.ts` (pick gap).

use otb_core::streaming::{
    session_pick_send_gap_ms, should_flush_broken_stream_pending, should_flush_tool_call_line,
    should_hold_compact_progress, should_use_draft_response_mode,
};

#[test]
fn streaming_routes_draft_mode_for_raw_or_requested_draft() {
    assert!(should_use_draft_response_mode(true, false));
    assert!(should_use_draft_response_mode(false, true));
    assert!(!should_use_draft_response_mode(false, false));
}

#[test]
fn streaming_flushes_broken_pending_immediately() {
    assert!(should_flush_broken_stream_pending(true, 2));
    assert!(!should_flush_broken_stream_pending(true, 0));
    assert!(!should_flush_broken_stream_pending(false, 2));
}

#[test]
fn streaming_flushes_tool_lines_on_new_calls_or_transitions() {
    assert!(should_flush_tool_call_line(true, false));
    assert!(should_flush_tool_call_line(false, true));
    assert!(!should_flush_tool_call_line(false, false));
}

#[test]
fn streaming_holds_compact_progress_until_terminal_counts() {
    assert!(should_hold_compact_progress(true, false));
    assert!(!should_hold_compact_progress(true, true));
    assert!(!should_hold_compact_progress(false, false));
}

#[test]
fn streaming_uses_zero_pick_gap_in_tests() {
    assert_eq!(session_pick_send_gap_ms(true, 250), 0);
    assert_eq!(session_pick_send_gap_ms(false, 250), 250);
}
