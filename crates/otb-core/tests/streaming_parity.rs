//! Parity port of pure streaming/guard logic (slice 22b, RED).
//!
//! Mirrors `tests/bot/streaming/tool-call-streamer.test.ts` (33 its),
//! `tests/bot/streaming/response-streamer.test.ts` (27 its),
//! `tests/bot/streaming/running-tool-tracker.test.ts` (25 its),
//! `tests/bot/streaming/compact-progress-streamer.test.ts` (32 its),
//! `tests/bot/middleware/interaction-guard.test.ts` (30 its), and
//! `tests/bot/middleware/interaction-guard-decision.test.ts` (26 its).
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::streaming::*;

// --- tool-call-streamer: part building / splitting (8) ---

#[test]
fn slice22b_01_build_parts_joins_trimmed_lines() {
    let parts = build_stream_parts(&["first".into(), "  second  ".into(), "  ".into()]);
    assert_eq!(parts, vec!["first\n\nsecond".to_string()]);
}

#[test]
fn slice22b_02_build_parts_empty_when_blank() {
    assert!(build_stream_parts(&["   ".into()]).is_empty());
}

#[test]
fn slice22b_03_split_long_text_keeps_short_whole() {
    assert_eq!(split_long_text("hello", 4000), vec!["hello".to_string()]);
}

#[test]
fn slice22b_04_split_long_text_prefers_newline_and_fits() {
    let text = format!("{}\n{}", "a".repeat(3000), "b".repeat(3000));
    let chunks = split_long_text(&text, 4000);
    assert_eq!(chunks.len(), 2);
    for chunk in &chunks {
        assert!(chunk.len() <= 4000);
    }
    assert_eq!(chunks.join("\n").len(), text.len());
}

#[test]
fn slice22b_05_stream_keys_classify() {
    assert_eq!(classify_stream_key("default"), StreamKey::Default);
    assert_eq!(classify_stream_key("todo"), StreamKey::Todo);
    assert_eq!(
        classify_stream_key("subagent:card-1"),
        StreamKey::Subagent("card-1".into())
    );
}

#[test]
fn slice22b_06_retry_after_parses_429() {
    assert_eq!(parse_retry_after_ms("429: retry after 1"), Some(1000));
    assert_eq!(parse_retry_after_ms("ok"), None);
    assert_eq!(parse_retry_after_ms("429 without delay"), None);
}

#[test]
fn slice22b_07_progressive_throttle_tiers() {
    assert_eq!(resolve_progressive_throttle_ms(0), STREAM_THROTTLE_BASE_MS);
    assert_eq!(resolve_progressive_throttle_ms(60_000), 2000);
    assert_eq!(resolve_progressive_throttle_ms(5 * 60_000), 5000);
    assert_eq!(
        resolve_progressive_throttle_ms(15 * 60_000),
        STREAM_THROTTLE_MAX_MS
    );
}

#[test]
fn slice22b_08_resolve_throttle_floors_at_zero() {
    assert_eq!(resolve_stream_throttle_ms(200), 200);
    assert_eq!(resolve_stream_throttle_ms(-5), 0);
}

// --- response-streamer pure rules (10) ---

#[test]
fn slice22b_09_same_signatures_skip_sync() {
    assert!(!should_sync_payload(&["a".into()], &["a".into()]));
    assert!(should_sync_payload(&["a".into()], &["b".into()]));
}

#[test]
fn slice22b_10_grown_payload_needs_second_message() {
    assert_eq!(extra_message_delete_count(1, 2), 0);
}

#[test]
fn slice22b_11_shrunk_payload_deletes_extra() {
    assert_eq!(extra_message_delete_count(2, 1), 1);
}

#[test]
fn slice22b_12_flush_pending_sends_when_nothing_visible() {
    assert!(should_flush_pending_immediately(0, false));
    assert!(!should_flush_pending_immediately(1, false));
    assert!(!should_flush_pending_immediately(0, true));
}

#[test]
fn slice22b_13_plain_fallback_chunks_fit_limit() {
    let chunks = chunk_plain_fallback(&"x".repeat(9000), 4096);
    assert_eq!(chunks.len(), 3);
    for chunk in &chunks {
        assert!(chunk.chars().count() <= 4096);
    }
    assert_eq!(chunks.join("").len(), 9000);
}

#[test]
fn slice22b_14_plain_fallback_keeps_short_whole() {
    assert_eq!(
        chunk_plain_fallback("short", 4096),
        vec!["short".to_string()]
    );
}

#[test]
fn slice22b_15_degraded_plain_still_skips_unchanged() {
    assert!(!should_sync_payload(
        &["plain:hello".into()],
        &["plain:hello".into()]
    ));
}

#[test]
fn slice22b_16_entity_change_triggers_edit() {
    assert!(should_sync_payload(
        &["text+blockquote".into()],
        &["text+expandable".into()]
    ));
}

#[test]
fn slice22b_17_complete_without_partial_needs_no_sync() {
    assert!(!should_sync_payload(&[], &[]));
}

#[test]
fn slice22b_18_broken_stream_never_flushes() {
    assert!(!should_flush_pending_immediately(2, true));
}

// --- running-tool-tracker pure rules (10) ---

#[test]
fn slice22b_19_bucket_below_minute_steps_ten_seconds() {
    assert_eq!(bucket_elapsed_ms(20_000), 20_000);
    assert_eq!(bucket_elapsed_ms(25_000), 20_000);
}

#[test]
fn slice22b_20_bucket_minute_range_steps_minutes() {
    assert_eq!(bucket_elapsed_ms(12 * 60_000), 10 * 60_000);
}

#[test]
fn slice22b_21_silent_below_threshold() {
    assert!(!should_emit_tick(15_000, 20_000, None));
}

#[test]
fn slice22b_22_tick_at_threshold() {
    assert!(should_emit_tick(20_000, 20_000, None));
}

#[test]
fn slice22b_23_no_repeat_within_bucket() {
    assert!(!should_emit_tick(25_000, 20_000, Some(20_000)));
    assert!(should_emit_tick(30_000, 20_000, Some(20_000)));
}

#[test]
fn slice22b_24_final_tick_at_limit() {
    assert!(is_final_tick(60_000, 60_000));
    assert!(!is_final_tick(30_000, 60_000));
}

#[test]
fn slice22b_25_newest_call_picks_later_start() {
    let newest = newest_call_id(&[
        ("call-1", 1000, false, false),
        ("call-2", 2000, false, false),
    ]);
    assert_eq!(newest.as_deref(), Some("call-2"));
}

#[test]
fn slice22b_26_newest_skips_released_and_detached() {
    let newest = newest_call_id(&[
        ("call-1", 1000, false, false),
        ("call-2", 2000, true, false),
    ]);
    assert_eq!(newest.as_deref(), Some("call-1"));
    let detached = newest_call_id(&[("call-bg", 1000, false, true)]);
    assert_eq!(detached, None);
}

#[test]
fn slice22b_27_newest_none_when_empty() {
    let empty: [(&str, u64, bool, bool); 0] = [];
    assert_eq!(newest_call_id(&empty), None);
}

#[test]
fn slice22b_28_nonpositive_elapsed_buckets_zero() {
    assert_eq!(bucket_elapsed_ms(0), 0);
    assert_eq!(bucket_elapsed_ms(-100), 0);
}

// --- compact-progress pure rules (8) ---

#[test]
fn slice22b_29_empty_counts_summarize_none() {
    assert_eq!(format_compact_summary(empty_tool_call_counts(), 0), None);
}

#[test]
fn slice22b_30_reads_summarize() {
    let counts = ToolCallCounts {
        reads: 1,
        ..empty_tool_call_counts()
    };
    assert_eq!(
        format_compact_summary(counts, 0).as_deref(),
        Some("✅ reads: 1")
    );
}

#[test]
fn slice22b_31_counts_merge() {
    let a = ToolCallCounts {
        reads: 1,
        ..empty_tool_call_counts()
    };
    let b = ToolCallCounts {
        commands: 1,
        ..empty_tool_call_counts()
    };
    let merged = add_tool_call_counts(a, b);
    assert_eq!(merged.reads, 1);
    assert_eq!(merged.commands, 1);
}

#[test]
fn slice22b_32_changed_files_summarize() {
    let summary = format_compact_summary(empty_tool_call_counts(), 2).unwrap();
    assert!(summary.contains("changed files: 2"));
}

#[test]
fn slice22b_33_multi_category_summary() {
    let counts = ToolCallCounts {
        commands: 1,
        reads: 1,
        ..empty_tool_call_counts()
    };
    let summary = format_compact_summary(counts, 2).unwrap();
    assert!(summary.contains("commands: 1"));
    assert!(summary.contains("reads: 1"));
    assert!(summary.contains("changed files: 2"));
}

#[test]
fn slice22b_34_progress_text_marks_working() {
    assert_eq!(format_progress_text("reading"), "⏳ Working\nreading");
}

#[test]
fn slice22b_35_commands_only_summary() {
    let counts = ToolCallCounts {
        commands: 1,
        ..empty_tool_call_counts()
    };
    assert_eq!(
        format_compact_summary(counts, 0).as_deref(),
        Some("✅ commands: 1")
    );
}

#[test]
fn slice22b_36_empty_counts_are_zero() {
    assert_eq!(empty_tool_call_counts(), ToolCallCounts::default());
}

// --- interaction-guard decision pure rules (8) ---

#[test]
fn slice22b_37_normalize_command_strips_mention_and_args() {
    assert_eq!(
        normalize_incoming_command("/status"),
        Some("/status".into())
    );
    assert_eq!(
        normalize_incoming_command("/status@bot extra"),
        Some("/status".into())
    );
    assert_eq!(
        normalize_incoming_command("/STATUS"),
        Some("/status".into())
    );
    assert_eq!(normalize_incoming_command("hello"), None);
    assert_eq!(normalize_incoming_command("/"), None);
}

#[test]
fn slice22b_38_classify_callback_text_media() {
    assert_eq!(
        classify_incoming_input(Some("model:foo"), None, false),
        IncomingInputType::Callback
    );
    assert_eq!(
        classify_incoming_input(None, Some("hello"), false),
        IncomingInputType::Text
    );
    assert_eq!(
        classify_incoming_input(None, Some("/status"), false),
        IncomingInputType::Command
    );
    assert_eq!(
        classify_incoming_input(None, None, true),
        IncomingInputType::Other
    );
}

#[test]
fn slice22b_39_expected_input_block_reasons() {
    assert_eq!(
        expected_input_block_reason(ExpectedInput::Callback),
        BlockReason::ExpectedCallback
    );
    assert_eq!(
        expected_input_block_reason(ExpectedInput::Text),
        BlockReason::ExpectedText
    );
    assert_eq!(
        expected_input_block_reason(ExpectedInput::Mixed),
        BlockReason::ExpectedText
    );
}

#[test]
fn slice22b_40_busy_allowlists() {
    assert!(is_busy_allowed_command(Some("/abort")));
    assert!(is_busy_allowed_command(Some("/opencode_stop")));
    assert!(!is_busy_allowed_command(Some("/new")));
    assert!(!is_busy_allowed_command(None));
    assert!(allows_busy_interaction(Some(InteractionKind::Question)));
    assert!(allows_busy_interaction(Some(InteractionKind::Permission)));
    assert!(!allows_busy_interaction(Some(InteractionKind::Rename)));
    assert!(!allows_busy_interaction(None));
}

#[test]
fn slice22b_41_allow_when_no_interaction_idle() {
    let d = decide_guard(None, false, false, IncomingInputType::Text, None);
    assert!(d.allow);
    assert_eq!(d.reason, None);
}

#[test]
fn slice22b_42_block_text_when_callback_expected() {
    let d = decide_guard(
        Some((InteractionKind::Inline, ExpectedInput::Callback, &[])),
        false,
        false,
        IncomingInputType::Text,
        None,
    );
    assert!(!d.allow);
    assert_eq!(d.reason, Some(BlockReason::ExpectedCallback));
}

#[test]
fn slice22b_43_expired_reports_expired() {
    let d = decide_guard(
        Some((InteractionKind::Permission, ExpectedInput::Callback, &[])),
        true,
        false,
        IncomingInputType::Text,
        None,
    );
    assert!(!d.allow);
    assert_eq!(d.reason, Some(BlockReason::Expired));
}

#[test]
fn slice22b_44_busy_blocks_plain_text() {
    let d = decide_guard(None, false, true, IncomingInputType::Text, None);
    assert!(!d.allow);
    assert!(d.busy);
}
