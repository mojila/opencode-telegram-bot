//! Parity port of the assistant/message-delivery pure gates (slice 59, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/bot/messages/telegram-text.ts` (parse-mode pick, plain-part gate,
//!   send/edit plain-fallback retry gates)
//! - `src/bot/messages/thinking-rendering.ts` (header format, CRLF/trimEnd
//!   normalize, per-section text limit, entity pick, part build, null gate)
//! - `src/bot/messages/assistant-rendering.ts` (stable boundary, stable/tail
//!   split, format-mode pick, empty-text null gates)
//! - `src/bot/messages/goal-delivery.ts` (markup on last chunk only)
//! - `src/bot/messages/scheduled-task-delivery.ts` (4096 budget, delivery
//!   format, silent gate, result presence, first-part split, per-status
//!   format, footer suppress/extra-send)
//! - `src/bot/messages/send-downloaded-file.ts` + `file-download-service.ts`
//!   (HTML escape, 50MB budget, `B`/`KB`/`MB` sizes, caption cut, announce
//!   gate, stat classification, falsy-size gate)
//! - `src/app/services/external-user-input-service.ts` + notification
//!   delivery (normalize, 2000-char cut, null gate, delivery gate,
//!   `markdown_v2` format)
//! - `src/bot/streaming/finalize-assistant-response.ts` (reply-options gate,
//!   notify-complete flag, first-part markup flip)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::assistant_delivery_gates::*;

// --- budgets / markers ---

#[test]
fn telegram_message_limit_is_4096() {
    assert_eq!(TELEGRAM_MESSAGE_LIMIT, 4096);
    assert!(can_retry_edit_as_plain(true, true, 4096));
}

#[test]
fn download_budgets() {
    assert_eq!(DOWNLOAD_MAX_FILE_SIZE_MB, 50);
    assert_eq!(DOWNLOAD_MAX_FILE_BYTES, 50 * 1024 * 1024);
    assert!(download_size_allowed(50 * 1024 * 1024));
}

#[test]
fn caption_cut_at_1024() {
    assert_eq!(DOWNLOAD_CAPTION_LIMIT, 1024);
    let long = "x".repeat(1100);
    assert_eq!(truncate_caption(&long).chars().count(), 1024);
}

#[test]
fn thinking_entity_collapsed_vs_open() {
    assert_eq!(THINKING_QUOTE_ENTITY_OPEN, "blockquote");
    assert_eq!(THINKING_QUOTE_ENTITY_COLLAPSED, "expandable_blockquote");
    assert_eq!(thinking_entity_type(true), THINKING_QUOTE_ENTITY_COLLAPSED);
    assert_eq!(thinking_entity_type(false), THINKING_QUOTE_ENTITY_OPEN);
}

// --- telegram-text.ts ---

#[test]
fn resolve_parse_mode_picks_format() {
    assert_eq!(resolve_parse_mode(TelegramTextFormat::MarkdownV2), Some("MarkdownV2"));
    assert_eq!(resolve_parse_mode(TelegramTextFormat::Raw), None);
}

#[test]
fn plain_part_gates() {
    assert!(is_plain_part("plain", 3));
    assert!(is_plain_part("rich", 0));
}

#[test]
fn rich_part_is_not_plain() {
    assert!(!is_plain_part("rich", 2));
}

#[test]
fn send_retry_needs_opt_in_and_bad_request() {
    assert!(can_retry_send_as_plain(true, true));
    assert!(!can_retry_send_as_plain(false, true));
    assert!(!can_retry_send_as_plain(true, false));
}

#[test]
fn edit_retry_rejects_over_limit_text() {
    assert!(!can_retry_edit_as_plain(true, true, 4097));
}

// --- thinking-rendering.ts ---

#[test]
fn thinking_header_variants() {
    assert_eq!(format_thinking_header("Thinking", None), "Thinking");
    assert_eq!(
        format_thinking_header("Thinking", Some("plan")),
        "Thinking — plan"
    );
    assert_eq!(THINKING_HEADER_SEPARATOR, " — ");
}

#[test]
fn thinking_header_trims_title() {
    assert_eq!(
        format_thinking_header("Thinking", Some("  plan  ")),
        "Thinking — plan"
    );
}

#[test]
fn thinking_header_blank_title_falls_back() {
    assert_eq!(format_thinking_header("Thinking", Some("   ")), "Thinking");
}

#[test]
fn normalize_thinking_text_crlf_and_trim_end() {
    assert_eq!(normalize_thinking_text("a\r\nb  \n"), "a\nb");
}

#[test]
fn thinking_text_limit_respects_header() {
    assert_eq!(thinking_text_limit(10, 4000), 3989);
}

#[test]
fn thinking_text_limit_floors_at_one() {
    assert_eq!(thinking_text_limit(5000, 4000), 1);
}

#[test]
fn thinking_part_empty_text_is_header_only() {
    assert_eq!(
        build_thinking_part("Thinking", "", false),
        ThinkingPart {
            fallback_text: "Thinking".to_string(),
            entity_type: "blockquote",
            offset: 0,
            length: 0,
        }
    );
}

#[test]
fn thinking_part_geometry() {
    assert_eq!(
        build_thinking_part("Thinking", "abc", true),
        ThinkingPart {
            fallback_text: "Thinking\nabc".to_string(),
            entity_type: "expandable_blockquote",
            offset: 9,
            length: 3,
        }
    );
}

#[test]
fn thinking_payload_null_without_sections() {
    assert!(thinking_payload_is_null(0));
    assert!(!thinking_payload_is_null(1));
}

// --- assistant-rendering.ts ---

#[test]
fn stable_boundary_empty_is_zero() {
    assert_eq!(streaming_stable_boundary(""), 0);
}

#[test]
fn stable_boundary_trailing_blank_line_is_full_length() {
    assert_eq!(streaming_stable_boundary("abc\n\n"), 5);
}

#[test]
fn stable_boundary_uses_last_separator() {
    assert_eq!(streaming_stable_boundary("a\n\nb\n\ntail"), 5);
}

#[test]
fn stable_boundary_without_separator_is_zero() {
    assert_eq!(streaming_stable_boundary("just one block"), 0);
}

#[test]
fn split_streaming_text_partitions_stable_tail() {
    assert_eq!(
        split_streaming_text("a\n\ntail"),
        ("a\n\n".to_string(), "tail".to_string())
    );
}

#[test]
fn assistant_format_mode_pick() {
    assert_eq!(
        select_assistant_format_mode("markdown"),
        AssistantFormatMode::Blocks
    );
    assert_eq!(select_assistant_format_mode("raw"), AssistantFormatMode::Raw);
}

#[test]
fn assistant_empty_text_has_no_payload() {
    assert!(assistant_text_payload_is_null(""));
    assert!(!assistant_text_payload_is_null("hi"));
}

// --- goal-delivery.ts ---

#[test]
fn goal_markup_rides_last_chunk() {
    assert_eq!(goal_markup_chunk_index(3), Some(2));
    assert_eq!(goal_markup_chunk_index(1), Some(0));
}

// --- scheduled-task-delivery.ts ---

#[test]
fn scheduled_format_pick() {
    assert_eq!(
        scheduled_task_delivery_format("markdown"),
        TelegramTextFormat::MarkdownV2
    );
    assert_eq!(
        scheduled_task_delivery_format("raw"),
        TelegramTextFormat::Raw
    );
}

#[test]
fn silent_delivery_flag() {
    assert!(silent_delivery_enabled(true));
    assert!(!silent_delivery_enabled(false));
}

#[test]
fn scheduled_success_result_presence() {
    assert!(scheduled_success_has_result(Some("done")));
    assert!(!scheduled_success_has_result(Some("")));
    assert!(!scheduled_success_has_result(None));
}

#[test]
fn scheduled_first_part_split_at_limit() {
    assert!(!scheduled_first_part_needs_split(4096));
    assert!(scheduled_first_part_needs_split(4097));
}

#[test]
fn scheduled_status_format() {
    assert_eq!(
        scheduled_message_format_for_status(true, TelegramTextFormat::MarkdownV2),
        TelegramTextFormat::MarkdownV2
    );
    assert_eq!(
        scheduled_message_format_for_status(false, TelegramTextFormat::MarkdownV2),
        TelegramTextFormat::Raw
    );
}

#[test]
fn footer_suppresses_result_notification() {
    assert!(suppress_result_notification(true, true, false));
    assert!(!suppress_result_notification(true, true, true));
    assert!(!suppress_result_notification(true, false, false));
    assert!(!suppress_result_notification(false, true, false));
}

#[test]
fn footer_extra_send_on_success_only() {
    assert!(footer_extra_send(true, true));
    assert!(!footer_extra_send(true, false));
    assert!(!footer_extra_send(false, true));
}

// --- send-downloaded-file.ts + file-download-service.ts ---

#[test]
fn escape_html_covers_five_chars() {
    assert_eq!(
        escape_html("&<>\"'"),
        "&amp;&lt;&gt;&quot;&#39;".to_string()
    );
}

#[test]
fn escape_html_leaves_plain_text() {
    assert_eq!(escape_html("file.txt"), "file.txt".to_string());
}

#[test]
fn download_budget_boundary() {
    assert!(download_size_allowed(DOWNLOAD_MAX_FILE_BYTES));
    assert!(!download_size_allowed(DOWNLOAD_MAX_FILE_BYTES + 1));
}

#[test]
fn falsy_size_reads_as_allowed() {
    assert!(file_size_allowed_generic(None, 100));
    assert!(file_size_allowed_generic(Some(102_400), 100));
    assert!(!file_size_allowed_generic(Some(102_401), 100));
}

#[test]
fn format_file_size_tiers() {
    assert_eq!(format_file_size(999), "999B".to_string());
    assert_eq!(format_file_size(1536), "1.5KB".to_string());
    assert_eq!(format_file_size(2 * 1024 * 1024), "2.0MB".to_string());
}

#[test]
fn caption_cut_truncates_long_captions() {
    let long = "x".repeat(1100);
    assert_eq!(truncate_caption(&long).chars().count(), 1024);
}

#[test]
fn announce_defaults_to_on() {
    assert!(should_announce_download(None));
    assert!(should_announce_download(Some(true)));
    assert!(!should_announce_download(Some(false)));
}

#[test]
fn classify_download_targets() {
    assert_eq!(classify_download_target(false, false, 0), DownloadGate::NotFound);
    assert_eq!(classify_download_target(true, false, 0), DownloadGate::NotFile);
    assert_eq!(
        classify_download_target(true, true, DOWNLOAD_MAX_FILE_BYTES + 1),
        DownloadGate::TooLarge
    );
    assert_eq!(classify_download_target(true, true, 10), DownloadGate::Ready);
}

// --- external user input ---

#[test]
fn normalize_external_user_input_text() {
    assert_eq!(normalize_external_user_input("  a\r\nb  "), "a\nb".to_string());
}

#[test]
fn truncate_external_input_at_budget() {
    assert_eq!(EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH, 2000);
    assert_eq!(EXTERNAL_INPUT_TRUNCATION_SUFFIX, "...");
    let long = "y".repeat(2005);
    let cut = truncate_external_user_input(&long);
    assert_eq!(cut.chars().count(), 2000);
    assert!(cut.ends_with("..."));
}

#[test]
fn truncate_external_input_short_passthrough() {
    assert_eq!(truncate_external_user_input("hi"), "hi".to_string());
}

#[test]
fn external_notification_null_gate() {
    assert!(external_input_notification_is_null(true));
    assert!(!external_input_notification_is_null(false));
}

#[test]
fn external_delivery_gate() {
    assert!(should_deliver_external_input(true, true, false));
    assert!(!should_deliver_external_input(false, true, false));
    assert!(!should_deliver_external_input(true, false, false));
    assert!(!should_deliver_external_input(true, true, true));
}

#[test]
fn external_notification_format_is_markdown_v2() {
    assert_eq!(external_notification_format(), TelegramTextFormat::MarkdownV2);
}

// --- finalize-assistant-response.ts ---

#[test]
fn finalize_option_flags() {
    assert!(finalize_has_reply_options(true));
    assert!(!finalize_has_reply_options(false));
    assert!(finalize_uses_notify_complete(true));
    assert!(!finalize_uses_notify_complete(false));
}

#[test]
fn finalize_first_part_keeps_markup_after_notify() {
    assert!(finalize_part_gets_reply_markup(true, 0));
    assert!(!finalize_part_gets_reply_markup(true, 1));
    assert!(!finalize_part_gets_reply_markup(false, 0));
}
