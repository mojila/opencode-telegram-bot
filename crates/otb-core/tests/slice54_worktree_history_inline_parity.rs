//! Parity port of the worktree / message-history / inline-menu pure gates
//! (slice 54, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/bot/menus/worktree-selection-menu.ts` (`formatWorktreeButtonLabel`,
//!   `buildWorktreeButtonLabel`, `parseWorktreePageCallback`,
//!   `parseWorktreeIndexCallback`)
//! - `src/bot/menus/message-history-menu.ts` (`truncateMessageHistoryText`,
//!   `normalizeButtonText`, the `[HH:MM]` button/detail labels,
//!   `message_select_text_kind`, `buildMessagePageCallback`,
//!   `parseMessagePageCallback`, `parseMessageSelectCallback`,
//!   `calculateMessagesPaginationRange`)
//! - `src/bot/menus/inline-menu.ts` + `inline-menu-cancel-callback-handler.ts`
//!   (`isInlineMenuKind`, `inline_cancel_callback_data`,
//!   `parse_inline_cancel_callback`)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::worktree_history_inline_gates::*;

// --- worktree button labels ---

#[test]
fn short_inactive_label_passes_through() {
    assert_eq!(
        format_worktree_button_label("1. repo [/repo]", false),
        "1. repo [/repo]"
    );
}

#[test]
fn active_label_gains_check_prefix() {
    assert_eq!(
        format_worktree_button_label("1. repo [/repo]", true),
        "✅ 1. repo [/repo]"
    );
}

#[test]
fn long_inactive_label_cuts_to_64_with_ellipsis() {
    let label = "1. ".to_string() + &"b".repeat(70);
    let rendered = format_worktree_button_label(&label, false);
    assert_eq!(rendered.chars().count(), MAX_INLINE_BUTTON_LABEL_LENGTH);
    assert!(rendered.ends_with("..."));
    assert!(!rendered.contains(&"b".repeat(70)));
}

#[test]
fn long_active_label_cuts_to_64_with_prefix() {
    let label = "1. ".to_string() + &"b".repeat(70);
    let rendered = format_worktree_button_label(&label, true);
    assert_eq!(rendered.chars().count(), MAX_INLINE_BUTTON_LABEL_LENGTH);
    assert!(rendered.starts_with("✅ "));
    assert!(rendered.ends_with("..."));
}

#[test]
fn build_numbered_label_uses_folder_name() {
    assert_eq!(build_worktree_button_label(0, "/repo"), "1. repo [/repo]");
}

#[test]
fn build_tolerates_trailing_separator() {
    assert_eq!(
        build_worktree_button_label(1, "/repo-feature/"),
        "2. repo-feature [/repo-feature/]"
    );
}

#[test]
fn build_tolerates_backslash_path() {
    assert_eq!(
        build_worktree_button_label(0, "C:\\projects\\repo"),
        "1. repo [C:\\projects\\repo]"
    );
}

// --- worktree callback parsing ---

#[test]
fn parse_worktree_page_valid() {
    assert_eq!(parse_worktree_page_callback("worktree:page:2"), Some(2));
}

#[test]
fn parse_worktree_page_rejects_non_digits() {
    assert_eq!(parse_worktree_page_callback("worktree:page:abc"), None);
}

#[test]
fn parse_worktree_page_rejects_plain_index_callback() {
    assert_eq!(parse_worktree_page_callback("worktree:3"), None);
}

#[test]
fn parse_worktree_index_valid() {
    assert_eq!(parse_worktree_index_callback("worktree:3"), Some(3));
}

#[test]
fn parse_worktree_index_rejects_page_callback() {
    assert_eq!(parse_worktree_index_callback("worktree:page:1"), None);
}

#[test]
fn parse_worktree_index_rejects_non_digit() {
    assert_eq!(parse_worktree_index_callback("worktree:abc"), None);
}

// --- message-history truncation / normalization ---

#[test]
fn truncate_short_text_unchanged() {
    assert_eq!(truncate_message_history_text("hello", 10), "hello");
}

#[test]
fn truncate_long_text_cuts_with_ellipsis() {
    // slice(0, 11 - 3) = "hello wo", then + "...".
    assert_eq!(
        truncate_message_history_text("hello world!", 11),
        "hello wo..."
    );
}

#[test]
fn truncate_trims_trailing_space_before_ellipsis() {
    assert_eq!(
        truncate_message_history_text("hello   world", 8),
        "hello..."
    );
}

#[test]
fn truncate_tiny_budget_yields_ellipsis() {
    assert_eq!(truncate_message_history_text("hello", 2), "...");
}

#[test]
fn normalize_collapses_whitespace_runs() {
    assert_eq!(
        normalize_button_text("hello   world\n\tnext"),
        "hello world next"
    );
}

#[test]
fn normalize_trims_edges() {
    assert_eq!(normalize_button_text("  hello  "), "hello");
}

// --- message-history labels ---

#[test]
fn button_label_prefixes_time() {
    assert_eq!(
        format_message_button_label("12:30", "hello"),
        "[12:30] hello"
    );
}

#[test]
fn button_label_truncates_long_text_to_64() {
    let rendered = format_message_button_label("12:30", &"a".repeat(60));
    assert_eq!(rendered.chars().count(), MAX_INLINE_BUTTON_LABEL_LENGTH);
    assert!(rendered.starts_with("[12:30] "));
    assert!(rendered.ends_with("..."));
}

#[test]
fn button_label_collapses_whitespace() {
    assert_eq!(
        format_message_button_label("09:05", "  a   b "),
        "[09:05] a b"
    );
}

#[test]
fn detail_text_prefixes_time_with_blank_line() {
    assert_eq!(
        format_message_detail_text("12:30", "hello"),
        "[12:30]\n\nhello"
    );
}

#[test]
fn detail_text_truncates_to_telegram_limit() {
    let rendered = format_message_detail_text("12:30", &"x".repeat(5000));
    assert_eq!(rendered.chars().count(), TELEGRAM_MESSAGE_LIMIT);
    assert!(rendered.starts_with("[12:30]\n\nxxx"));
    assert!(rendered.ends_with("..."));
}

#[test]
fn select_kind_first_page() {
    assert_eq!(
        message_select_text_kind(0),
        MessageSelectTextKind::FirstPage
    );
}

#[test]
fn select_kind_later_page() {
    assert_eq!(message_select_text_kind(2), MessageSelectTextKind::Page(2));
}

#[test]
fn build_page_callback_shape() {
    assert_eq!(build_message_page_callback(3), "messages:page:3");
}

// --- message-history callback parsing ---

#[test]
fn parse_message_page_valid() {
    assert_eq!(parse_message_page_callback("messages:page:2"), Some(2));
}

#[test]
fn parse_message_page_rejects_wrong_prefix() {
    assert_eq!(parse_message_page_callback("messages:select:1"), None);
}

#[test]
fn parse_message_page_rejects_negative() {
    assert_eq!(parse_message_page_callback("messages:page:-1"), None);
}

#[test]
fn parse_message_page_rejects_fraction() {
    assert_eq!(parse_message_page_callback("messages:page:1.5"), None);
}

#[test]
fn parse_message_page_empty_reads_as_zero() {
    // JS `Number("")` is `0`: the empty payload reads as page 0.
    assert_eq!(parse_message_page_callback("messages:page:"), Some(0));
}

#[test]
fn parse_message_select_valid() {
    assert_eq!(parse_message_select_callback("messages:select:4"), Some(4));
}

#[test]
fn parse_message_select_rejects_wrong_prefix() {
    assert_eq!(parse_message_select_callback("messages:page:1"), None);
}

#[test]
fn parse_message_select_rejects_negative() {
    assert_eq!(parse_message_select_callback("messages:select:-2"), None);
}

// --- message-history pagination ---

#[test]
fn pagination_first_page_window() {
    assert_eq!(
        calculate_messages_pagination_range(10, 0, 3),
        MessagesPaginationRange {
            page: 0,
            total_pages: 4,
            start_index: 0,
            end_index: 3,
        }
    );
}

#[test]
fn pagination_clamps_to_last_page() {
    assert_eq!(
        calculate_messages_pagination_range(10, 99, 3),
        MessagesPaginationRange {
            page: 3,
            total_pages: 4,
            start_index: 9,
            end_index: 10,
        }
    );
}

#[test]
fn pagination_empty_list_has_single_page() {
    assert_eq!(
        calculate_messages_pagination_range(0, 0, 5),
        MessagesPaginationRange {
            page: 0,
            total_pages: 1,
            start_index: 0,
            end_index: 0,
        }
    );
}

#[test]
fn pagination_page_size_floors_at_one() {
    assert_eq!(
        calculate_messages_pagination_range(10, 1, 0),
        MessagesPaginationRange {
            page: 1,
            total_pages: 10,
            start_index: 1,
            end_index: 2,
        }
    );
}

// --- inline-menu kinds ---

#[test]
fn kind_recognizes_project() {
    assert!(is_inline_menu_kind("project"));
    assert_eq!(
        inline_menu_kind_of("project"),
        Some(InlineMenuKind::Project)
    );
}

#[test]
fn kind_recognizes_worktree() {
    assert!(is_inline_menu_kind("worktree"));
    assert_eq!(
        inline_menu_kind_of("worktree"),
        Some(InlineMenuKind::Worktree)
    );
}

#[test]
fn kind_rejects_unknown() {
    assert!(!is_inline_menu_kind("unknown"));
    assert_eq!(inline_menu_kind_of("unknown"), None);
}

#[test]
fn kind_rejects_empty() {
    assert!(!is_inline_menu_kind(""));
}

#[test]
fn kind_match_is_case_sensitive() {
    assert!(!is_inline_menu_kind("Project"));
}

// --- inline-menu cancel callbacks ---

#[test]
fn cancel_data_unified_shape() {
    assert_eq!(
        inline_cancel_callback_data(InlineMenuKind::Model),
        "inline:cancel:model"
    );
}

#[test]
fn cancel_data_settings_shape() {
    assert_eq!(
        inline_cancel_callback_data(InlineMenuKind::Settings),
        "inline:cancel:settings"
    );
}

#[test]
fn parse_unified_cancel_callback() {
    assert_eq!(
        parse_inline_cancel_callback("inline:cancel:worktree"),
        Some(InlineCancelTarget::Unified(InlineMenuKind::Worktree))
    );
}

#[test]
fn parse_legacy_context_cancel_callback() {
    assert_eq!(
        parse_inline_cancel_callback("compact:cancel"),
        Some(InlineCancelTarget::LegacyContext)
    );
}

#[test]
fn parse_rejects_unknown_kind() {
    assert_eq!(parse_inline_cancel_callback("inline:cancel:unknown"), None);
}

#[test]
fn parse_rejects_unrelated_data() {
    assert_eq!(parse_inline_cancel_callback("model:pick:1"), None);
}
