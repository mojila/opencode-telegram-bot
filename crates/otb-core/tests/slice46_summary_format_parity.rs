//! Parity port of the summary-message pure gates (slice 46, RED).
//!
//! Mirrors representative cases from:
//! - `src/bot/messages/summary-message-formatter.ts`
//!   (`endsWithOddTrailingBackslashes`, `resolveSplitEndIndex`,
//!   `splitText`, `escapeMarkdownV2PipesOutsideCode`,
//!   `formatSummaryWithMode` in raw + markdown modes)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::summary_format_gates::*;

// --- ends_with_odd_trailing_backslashes (01-08) ---

#[test]
fn slice46_01_no_backslash_is_even() {
    assert!(!ends_with_odd_trailing_backslashes("abc", 0, 3));
}

#[test]
fn slice46_02_single_trailing_backslash_is_odd() {
    assert!(ends_with_odd_trailing_backslashes("ab\\", 0, 3));
}

#[test]
fn slice46_03_double_trailing_backslash_is_even() {
    assert!(!ends_with_odd_trailing_backslashes("ab\\\\", 0, 4));
}

#[test]
fn slice46_04_triple_trailing_backslash_is_odd() {
    assert!(ends_with_odd_trailing_backslashes("ab\\\\\\", 0, 5));
}

#[test]
fn slice46_05_empty_window_is_even() {
    assert!(!ends_with_odd_trailing_backslashes("abc", 1, 1));
}

#[test]
fn slice46_06_backslash_mid_window_is_even() {
    assert!(!ends_with_odd_trailing_backslashes("ab\\cd", 0, 5));
}

#[test]
fn slice46_07_window_over_pair_is_even() {
    assert!(!ends_with_odd_trailing_backslashes("a\\\\b", 1, 3));
}

#[test]
fn slice46_08_all_backslash_odd_window_is_odd() {
    assert!(ends_with_odd_trailing_backslashes("\\\\\\", 0, 3));
}

// --- resolve_split_end_index (09-18) ---

#[test]
fn slice46_09_short_text_returns_length() {
    assert_eq!(resolve_split_end_index("hello", 0, 10, false), 5);
}

#[test]
fn slice46_10_exact_fit_returns_length() {
    assert_eq!(resolve_split_end_index("abcd", 0, 4, false), 4);
}

#[test]
fn slice46_11_no_newline_returns_hard_limit() {
    assert_eq!(resolve_split_end_index("abcdef", 0, 4, false), 4);
}

#[test]
fn slice46_12_newline_before_limit_splits_after_it() {
    assert_eq!(resolve_split_end_index("ab\ncdef", 0, 4, false), 3);
}

#[test]
fn slice46_13_newline_at_cursor_does_not_split() {
    assert_eq!(resolve_split_end_index("\nabcdef", 0, 4, false), 4);
}

#[test]
fn slice46_14_escape_guard_off_keeps_hard_limit() {
    assert_eq!(resolve_split_end_index("ab\\cde", 0, 4, false), 4);
}

#[test]
fn slice46_15_escape_guard_trims_odd_backslash() {
    assert_eq!(resolve_split_end_index("abc\\def", 0, 4, true), 3);
}

#[test]
fn slice46_16_escape_guard_keeps_even_backslashes() {
    assert_eq!(resolve_split_end_index("ab\\\\cd", 0, 4, true), 4);
}

#[test]
fn slice46_17_trim_to_cursor_falls_back_to_hard_limit() {
    assert_eq!(resolve_split_end_index("\\ab", 0, 1, true), 1);
}

#[test]
fn slice46_18_full_fit_ignores_escape_guard() {
    assert_eq!(resolve_split_end_index("ab\\", 0, 10, true), 3);
}

// --- split_text (19-26) ---

#[test]
fn slice46_19_empty_text_yields_no_parts() {
    assert_eq!(split_text("", 10, false), Vec::<String>::new());
}

#[test]
fn slice46_20_short_text_yields_single_part() {
    assert_eq!(split_text("hello", 10, false), vec!["hello".to_string()]);
}

#[test]
fn slice46_21_exact_fit_yields_single_part() {
    assert_eq!(split_text("abcd", 4, false), vec!["abcd".to_string()]);
}

#[test]
fn slice46_22_long_text_without_newline_splits_at_limit() {
    assert_eq!(
        split_text("abcdef", 4, false),
        vec!["abcd".to_string(), "ef".to_string()]
    );
}

#[test]
fn slice46_23_newline_boundary_is_preferred() {
    assert_eq!(
        split_text("ab\ncdef", 4, false),
        vec!["ab\n".to_string(), "cdef".to_string()]
    );
}

#[test]
fn slice46_24_parts_concatenate_to_original() {
    let text = "ab\ncdefgh\nij";
    let parts = split_text(text, 4, false);
    assert!(parts.len() > 1);
    assert_eq!(parts.concat(), text);
}

#[test]
fn slice46_25_escape_guard_without_backslash_matches_plain_split() {
    assert_eq!(
        split_text("ab\ncdef", 4, true),
        split_text("ab\ncdef", 4, false)
    );
}

#[test]
fn slice46_26_unit_limit_splits_per_char() {
    assert_eq!(
        split_text("ab", 1, false),
        vec!["a".to_string(), "b".to_string()]
    );
}

// --- escape_markdown_v2_pipes_outside_code (27-36) ---

#[test]
fn slice46_27_plain_text_unchanged() {
    assert_eq!(escape_markdown_v2_pipes_outside_code("hello"), "hello");
}

#[test]
fn slice46_28_bare_pipe_escaped() {
    assert_eq!(escape_markdown_v2_pipes_outside_code("a|b"), "a\\|b");
}

#[test]
fn slice46_29_already_escaped_pipe_untouched() {
    assert_eq!(escape_markdown_v2_pipes_outside_code("a\\|b"), "a\\|b");
}

#[test]
fn slice46_30_pipe_inside_inline_code_untouched() {
    assert_eq!(escape_markdown_v2_pipes_outside_code("`a|b`"), "`a|b`");
}

#[test]
fn slice46_31_pipe_inside_fence_untouched() {
    assert_eq!(
        escape_markdown_v2_pipes_outside_code("```\na|b\n```"),
        "```\na|b\n```"
    );
}

#[test]
fn slice46_32_pipe_after_closing_fence_escaped() {
    assert_eq!(
        escape_markdown_v2_pipes_outside_code("```\na|b\n```\nc|d"),
        "```\na|b\n```\nc\\|d"
    );
}

#[test]
fn slice46_33_multiple_pipes_each_escaped() {
    assert_eq!(
        escape_markdown_v2_pipes_outside_code("a|b|c"),
        "a\\|b\\|c"
    );
}

#[test]
fn slice46_34_empty_stays_empty() {
    assert_eq!(escape_markdown_v2_pipes_outside_code(""), "");
}

#[test]
fn slice46_35_code_span_then_bare_pipe() {
    assert_eq!(
        escape_markdown_v2_pipes_outside_code("`a|b` | c"),
        "`a|b` \\| c"
    );
}

#[test]
fn slice46_36_unclosed_fence_treats_rest_as_code() {
    assert_eq!(
        escape_markdown_v2_pipes_outside_code("```\na|b"),
        "```\na|b"
    );
}

// --- format_summary_with_mode, raw (37-44) ---

#[test]
fn slice46_37_empty_raw_yields_no_parts() {
    assert_eq!(
        format_summary_with_mode("", MessageFormatMode::Raw, TELEGRAM_MESSAGE_LIMIT),
        Vec::<String>::new()
    );
}

#[test]
fn slice46_38_blank_raw_yields_no_parts() {
    assert_eq!(
        format_summary_with_mode("   \n  ", MessageFormatMode::Raw, TELEGRAM_MESSAGE_LIMIT),
        Vec::<String>::new()
    );
}

#[test]
fn slice46_39_single_short_raw_part_unwrapped() {
    assert_eq!(
        format_summary_with_mode("hello", MessageFormatMode::Raw, TELEGRAM_MESSAGE_LIMIT),
        vec!["hello".to_string()]
    );
}

#[test]
fn slice46_40_multi_part_raw_parts_wrapped_in_fences() {
    assert_eq!(
        format_summary_with_mode("aaa\nbbb", MessageFormatMode::Raw, 12),
        vec![
            "```\naaa\n```".to_string(),
            "```\nbbb\n```".to_string()
        ]
    );
}

#[test]
fn slice46_41_padded_single_raw_part_trimmed_unwrapped() {
    assert_eq!(
        format_summary_with_mode("  hi  ", MessageFormatMode::Raw, TELEGRAM_MESSAGE_LIMIT),
        vec!["hi".to_string()]
    );
}

#[test]
fn slice46_42_blank_middle_raw_part_skipped() {
    assert_eq!(
        format_summary_with_mode("ab\n   \ncd", MessageFormatMode::Raw, 12),
        vec!["```\nab\n```".to_string(), "```\ncd\n```".to_string()]
    );
}

#[test]
fn slice46_43_raw_limit_reserves_fence_overhead() {
    assert_eq!(
        format_summary_with_mode("abcd", MessageFormatMode::Raw, 10),
        vec!["```\nab\n```".to_string(), "```\ncd\n```".to_string()]
    );
}

#[test]
fn slice46_44_zero_limit_normalized_without_panic() {
    let parts = format_summary_with_mode("ab", MessageFormatMode::Raw, 0);
    assert_eq!(parts.len(), 2);
}

// --- format_summary_with_mode, markdown (45-48) ---

#[test]
fn slice46_45_empty_markdown_yields_no_parts() {
    assert_eq!(
        format_summary_with_mode("", MessageFormatMode::Markdown, TELEGRAM_MESSAGE_LIMIT),
        Vec::<String>::new()
    );
}

#[test]
fn slice46_46_blank_markdown_yields_no_parts() {
    assert_eq!(
        format_summary_with_mode("  \n ", MessageFormatMode::Markdown, TELEGRAM_MESSAGE_LIMIT),
        Vec::<String>::new()
    );
}

#[test]
fn slice46_47_short_markdown_yields_single_nonempty_part() {
    let parts = format_summary_with_mode("hello", MessageFormatMode::Markdown, TELEGRAM_MESSAGE_LIMIT);
    assert_eq!(parts.len(), 1);
    assert!(!parts[0].is_empty());
}

#[test]
fn slice46_48_long_markdown_splits_within_limit() {
    let text: String = "a ".repeat(3000);
    let parts = format_summary_with_mode(&text, MessageFormatMode::Markdown, 100);
    assert!(parts.len() >= 2);
    assert!(parts.iter().all(|part| !part.is_empty() && part.len() <= 100));
}
