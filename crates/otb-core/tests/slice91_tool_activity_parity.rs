//! Slice 91 parity: tool-activity pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/events/handlers/tool-activity-handler.ts` (the live-line
//! `RUNNING_ICON` + callId prefix, the `todowrite` -> `todo` stream key,
//! the `subagent:<cardId>` stream key, the 1024-char caption trim + `...`
//! cut, the input-or-metadata `background` gate, the `pending`-status
//! composing gate, the compact-or-non-`task` elapsed gate, the
//! `completed`/`error` terminal gate, the write/edit/apply_patch diff-file
//! gate, the file-or-`task` skip gate, the `apply_patch` first-line
//! duration gate, the pending/running heartbeat gate, the
//! outlives-parent + pending/running pin gate, the failed-while-composing
//! vs seen-failing gates, and the no-fileData text-line extraction)
//! plus `SUBAGENT_STREAM_PREFIX` from `handler-context.ts`,
//! `TOOL_ELAPSED_MAX_TRACKING_HOURS` from `session-runtime-state.ts`, and
//! `RUNNING_ICON` from `duration-formatter.ts`.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `tool_activity_gates` is ported (only the constant-value tests pass).

use otb_core::tool_activity_gates::{
    SUBAGENT_STREAM_PREFIX, TELEGRAM_DOCUMENT_CAPTION_MAX_LENGTH,
    TOOL_ELAPSED_MAX_TRACKING_HOURS, TOOL_STREAM_KEY_DEFAULT, TOOL_STREAM_KEY_TODO, RUNNING_ICON,
    count_text_lines, failed_while_composing, first_file_is_text, heartbeat_active,
    is_background_tool, is_composing, is_terminal_status, live_tool_prefix,
    prepare_document_caption, seen_failing, should_send_tool_file_attachment,
    should_skip_tool_delivery, subagent_pin_active, subagent_stream_key, tool_stream_key,
    tracks_elapsed, use_first_line_duration,
};

// --- constants ---

#[test]
fn caption_max_is_1024_parity() {
    assert_eq!(TELEGRAM_DOCUMENT_CAPTION_MAX_LENGTH, 1024);
}

#[test]
fn running_icon_is_hourglass_parity() {
    assert_eq!(RUNNING_ICON, "⏳");
}

#[test]
fn max_tracking_hours_is_24_parity() {
    assert_eq!(TOOL_ELAPSED_MAX_TRACKING_HOURS, 24);
}

#[test]
fn subagent_stream_prefix_is_puzzle_parity() {
    assert_eq!(SUBAGENT_STREAM_PREFIX, "🧩");
}

#[test]
fn todo_stream_key_parity() {
    assert_eq!(TOOL_STREAM_KEY_TODO, "todo");
}

#[test]
fn default_stream_key_parity() {
    assert_eq!(TOOL_STREAM_KEY_DEFAULT, "default");
}

// --- live_tool_prefix ---

#[test]
fn live_prefix_concatenates_icon_and_call_id_parity() {
    assert_eq!(live_tool_prefix("abc"), format!("{RUNNING_ICON}abc"));
}

#[test]
fn live_prefix_empty_call_id_is_bare_icon_parity() {
    assert_eq!(live_tool_prefix(""), RUNNING_ICON.to_string());
}

#[test]
fn live_prefix_differs_per_call_id_parity() {
    assert_ne!(live_tool_prefix("a"), live_tool_prefix("b"));
}

// --- tool_stream_key ---

#[test]
fn todowrite_routes_to_todo_stream_parity() {
    assert_eq!(tool_stream_key("todowrite"), "todo");
}

#[test]
fn write_routes_to_default_stream_parity() {
    assert_eq!(tool_stream_key("write"), "default");
}

#[test]
fn task_routes_to_default_stream_parity() {
    assert_eq!(tool_stream_key("task"), "default");
}

#[test]
fn empty_tool_routes_to_default_stream_parity() {
    assert_eq!(tool_stream_key(""), "default");
}

// --- subagent_stream_key ---

#[test]
fn subagent_key_prefixes_card_id_parity() {
    assert_eq!(subagent_stream_key("c1"), "subagent:c1");
}

#[test]
fn subagent_key_differs_per_card_parity() {
    assert_ne!(subagent_stream_key("a"), subagent_stream_key("b"));
}

// --- prepare_document_caption ---

#[test]
fn caption_trims_surrounding_whitespace_parity() {
    assert_eq!(prepare_document_caption("  hi  "), "hi");
}

#[test]
fn caption_blank_becomes_empty_parity() {
    assert_eq!(prepare_document_caption("   "), "");
}

#[test]
fn caption_short_passthrough_parity() {
    assert_eq!(prepare_document_caption("hello"), "hello");
}

#[test]
fn caption_at_cap_not_truncated_parity() {
    let s = "x".repeat(1024);
    assert_eq!(prepare_document_caption(&s), s);
}

#[test]
fn caption_over_cap_truncates_with_ellipsis_parity() {
    let s = "y".repeat(2000);
    let out = prepare_document_caption(&s);
    assert_eq!(out.len(), 1024);
    assert!(out.ends_with("..."));
}

// --- is_background_tool ---

#[test]
fn background_via_input_parity() {
    assert!(is_background_tool(true, false));
}

#[test]
fn background_via_metadata_parity() {
    assert!(is_background_tool(false, true));
}

#[test]
fn foreground_when_neither_flag_parity() {
    assert!(!is_background_tool(false, false));
}

// --- is_composing ---

#[test]
fn composing_when_pending_parity() {
    assert!(is_composing(Some("pending")));
}

#[test]
fn not_composing_when_running_parity() {
    assert!(!is_composing(Some("running")));
}

#[test]
fn not_composing_when_completed_parity() {
    assert!(!is_composing(Some("completed")));
}

#[test]
fn not_composing_without_status_parity() {
    assert!(!is_composing(None));
}

// --- tracks_elapsed ---

#[test]
fn compact_mode_tracks_task_tool_parity() {
    assert!(tracks_elapsed(true, "task"));
}

#[test]
fn full_mode_skips_task_tool_parity() {
    assert!(!tracks_elapsed(false, "task"));
}

#[test]
fn full_mode_tracks_write_tool_parity() {
    assert!(tracks_elapsed(false, "write"));
}

// --- is_terminal_status ---

#[test]
fn completed_is_terminal_parity() {
    assert!(is_terminal_status(Some("completed")));
}

#[test]
fn error_is_terminal_parity() {
    assert!(is_terminal_status(Some("error")));
}

#[test]
fn running_is_not_terminal_parity() {
    assert!(!is_terminal_status(Some("running")));
}

#[test]
fn pending_is_not_terminal_parity() {
    assert!(!is_terminal_status(Some("pending")));
}

#[test]
fn missing_status_is_not_terminal_parity() {
    assert!(!is_terminal_status(None));
}

// --- should_send_tool_file_attachment ---

#[test]
fn file_attach_write_when_enabled_parity() {
    assert!(should_send_tool_file_attachment(true, true, "write"));
}

#[test]
fn file_attach_edit_when_enabled_parity() {
    assert!(should_send_tool_file_attachment(true, true, "edit"));
}

#[test]
fn file_attach_apply_patch_when_enabled_parity() {
    assert!(should_send_tool_file_attachment(true, true, "apply_patch"));
}

#[test]
fn file_attach_denied_without_attachment_parity() {
    assert!(!should_send_tool_file_attachment(false, true, "write"));
}

#[test]
fn file_attach_denied_when_setting_off_parity() {
    assert!(!should_send_tool_file_attachment(true, false, "write"));
}

#[test]
fn file_attach_denied_for_read_tool_parity() {
    assert!(!should_send_tool_file_attachment(true, true, "read"));
}

// --- should_skip_tool_delivery ---

#[test]
fn task_tool_always_skipped_parity() {
    assert!(should_skip_tool_delivery(false, "task"));
}

#[test]
fn file_attachment_skips_line_delivery_parity() {
    assert!(should_skip_tool_delivery(true, "write"));
}

#[test]
fn plain_write_is_delivered_parity() {
    assert!(!should_skip_tool_delivery(false, "write"));
}

// --- use_first_line_duration ---

#[test]
fn apply_patch_uses_first_line_duration_parity() {
    assert!(use_first_line_duration("apply_patch"));
}

#[test]
fn write_does_not_use_first_line_duration_parity() {
    assert!(!use_first_line_duration("write"));
}

// --- heartbeat_active ---

#[test]
fn heartbeat_active_with_running_subagent_parity() {
    assert!(heartbeat_active(&["completed", "running"]));
}

#[test]
fn heartbeat_active_with_pending_subagent_parity() {
    assert!(heartbeat_active(&["pending"]));
}

#[test]
fn heartbeat_idle_when_all_done_parity() {
    assert!(!heartbeat_active(&["completed", "error"]));
}

#[test]
fn heartbeat_idle_with_no_subagents_parity() {
    assert!(!heartbeat_active(&[] as &[&str]));
}

// --- subagent_pin_active ---

#[test]
fn background_running_subagent_stays_pinned_parity() {
    assert!(subagent_pin_active(true, Some("running")));
}

#[test]
fn background_pending_subagent_stays_pinned_parity() {
    assert!(subagent_pin_active(true, Some("pending")));
}

#[test]
fn completed_subagent_unpins_even_if_background_parity() {
    assert!(!subagent_pin_active(true, Some("completed")));
}

#[test]
fn foreground_running_subagent_does_not_pin_parity() {
    assert!(!subagent_pin_active(false, Some("running")));
}

// --- failed_while_composing / seen_failing ---

#[test]
fn error_with_composing_store_is_failed_while_composing_parity() {
    assert!(failed_while_composing(Some("error"), true, true));
}

#[test]
fn completed_is_never_failed_while_composing_parity() {
    assert!(!failed_while_composing(Some("completed"), true, true));
}

#[test]
fn seen_failing_when_stored_and_shown_parity() {
    assert!(seen_failing(Some("error"), true, false));
}

#[test]
fn not_seen_failing_when_failed_while_composing_parity() {
    assert!(!seen_failing(Some("error"), true, true));
}

#[test]
fn not_seen_failing_when_never_stored_parity() {
    assert!(!seen_failing(Some("error"), false, false));
}

// --- file text-line extraction ---

#[test]
fn count_text_lines_counts_data_less_entries_with_lines_parity() {
    assert_eq!(count_text_lines(&[false, true, false], &[true, true, false]), 1);
}

#[test]
fn count_text_lines_empty_when_all_binary_parity() {
    assert_eq!(count_text_lines(&[true, true], &[true, true]), 0);
}

#[test]
fn first_text_file_renders_inline_parity() {
    assert!(first_file_is_text(false));
}

#[test]
fn first_binary_file_does_not_render_inline_parity() {
    assert!(!first_file_is_text(true));
}
