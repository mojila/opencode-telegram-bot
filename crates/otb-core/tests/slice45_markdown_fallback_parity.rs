//! Parity port of the Markdown-fallback pure gates (slice 45, RED).
//!
//! Mirrors representative cases from:
//! - `src/bot/messages/send-with-markdown-fallback.ts`
//!   (`escapeTelegramMarkdownV2`, `unescapeTelegramMarkdownV2`,
//!   `getErrorText`, `isTelegramMarkdownParseError`,
//!   `isTelegramEntityUrlError`, `isTelegramBadRequestError`,
//!   `isTelegramMarkdownPresentationError`, fallback-text resolution,
//!   escape-retry gate, raw-fallback gate, format-flag stripping)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::markdown_fallback_gates::*;

// --- get_error_text (01-05) ---

#[test]
fn slice45_01_error_message_only() {
    let parts = ErrorTextParts {
        error_message: Some("Can't Parse Entities".to_string()),
        ..Default::default()
    };
    assert_eq!(get_error_text(&parts), "can't parse entities");
}

#[test]
fn slice45_02_description_and_message_joined() {
    let parts = ErrorTextParts {
        description: Some("Bad Request".to_string()),
        message_prop: Some("Entity URL".to_string()),
        ..Default::default()
    };
    assert_eq!(get_error_text(&parts), "bad request\nentity url");
}

#[test]
fn slice45_03_bare_string_form() {
    let parts = ErrorTextParts {
        string_form: Some("WRONG HTTP URL".to_string()),
        ..Default::default()
    };
    assert_eq!(get_error_text(&parts), "wrong http url");
}

#[test]
fn slice45_04_empty_parts_yield_empty() {
    assert_eq!(get_error_text(&ErrorTextParts::default()), "");
}

#[test]
fn slice45_05_all_parts_joined_in_order() {
    let parts = ErrorTextParts {
        error_message: Some("A".to_string()),
        description: Some("B".to_string()),
        message_prop: Some("C".to_string()),
        string_form: Some("D".to_string()),
        ..Default::default()
    };
    assert_eq!(get_error_text(&parts), "a\nb\nc\nd");
}

// --- is_telegram_markdown_parse_error_text (06-13) ---

#[test]
fn slice45_06_parse_entities_marker() {
    assert!(is_telegram_markdown_parse_error_text(
        "bad request: can't parse entities"
    ));
}

#[test]
fn slice45_07_parse_entity_singular_marker() {
    assert!(is_telegram_markdown_parse_error_text(
        "can't parse entity in message"
    ));
}

#[test]
fn slice45_08_end_of_entity_marker() {
    assert!(is_telegram_markdown_parse_error_text(
        "can't find end of the entity"
    ));
}

#[test]
fn slice45_09_entity_beginning_marker() {
    assert!(is_telegram_markdown_parse_error_text(
        "entity beginning at offset 3"
    ));
}

#[test]
fn slice45_10_bad_request_cant_parse_marker() {
    assert!(is_telegram_markdown_parse_error_text(
        "bad request: can't parse message"
    ));
}

#[test]
fn slice45_11_unrelated_error_is_not_parse_error() {
    assert!(!is_telegram_markdown_parse_error_text(
        "message is too long"
    ));
}

#[test]
fn slice45_12_empty_text_is_not_parse_error() {
    assert!(!is_telegram_markdown_parse_error_text(""));
}

#[test]
fn slice45_13_url_error_alone_is_not_parse_error() {
    assert!(!is_telegram_markdown_parse_error_text(
        "wrong http url specified"
    ));
}

// --- is_telegram_entity_url_error_text (14-18) ---

#[test]
fn slice45_14_entity_url_marker() {
    assert!(is_telegram_entity_url_error_text(
        "bad request: entity url invalid"
    ));
}

#[test]
fn slice45_15_wrong_http_url_marker() {
    assert!(is_telegram_entity_url_error_text(
        "wrong http url specified"
    ));
}

#[test]
fn slice45_16_url_host_empty_marker() {
    assert!(is_telegram_entity_url_error_text("url host is empty"));
}

#[test]
fn slice45_17_parse_error_alone_is_not_url_error() {
    assert!(!is_telegram_entity_url_error_text("can't parse entities"));
}

#[test]
fn slice45_18_empty_text_is_not_url_error() {
    assert!(!is_telegram_entity_url_error_text(""));
}

// --- is_telegram_bad_request_error (19-23) ---

#[test]
fn slice45_19_error_code_400_is_bad_request() {
    assert!(is_telegram_bad_request_error(Some(400), None));
}

#[test]
fn slice45_20_status_400_is_bad_request() {
    assert!(is_telegram_bad_request_error(None, Some(400)));
}

#[test]
fn slice45_21_other_code_is_not_bad_request() {
    assert!(!is_telegram_bad_request_error(Some(429), None));
}

#[test]
fn slice45_22_other_status_is_not_bad_request() {
    assert!(!is_telegram_bad_request_error(None, Some(500)));
}

#[test]
fn slice45_23_no_codes_is_not_bad_request() {
    assert!(!is_telegram_bad_request_error(None, None));
}

// --- is_telegram_markdown_presentation_error (24-28) ---

#[test]
fn slice45_24_bad_request_plus_parse_is_presentation_error() {
    assert!(is_telegram_markdown_presentation_error(true, true, false));
}

#[test]
fn slice45_25_bad_request_plus_url_is_presentation_error() {
    assert!(is_telegram_markdown_presentation_error(true, false, true));
}

#[test]
fn slice45_26_parse_without_bad_request_is_not() {
    assert!(!is_telegram_markdown_presentation_error(false, true, false));
}

#[test]
fn slice45_27_bad_request_alone_is_not() {
    assert!(!is_telegram_markdown_presentation_error(true, false, false));
}

#[test]
fn slice45_28_nothing_is_not_presentation_error() {
    assert!(!is_telegram_markdown_presentation_error(
        false, false, false
    ));
}

// --- escape_telegram_markdown_v2 (29-36) ---

#[test]
fn slice45_29_plain_text_unchanged() {
    assert_eq!(escape_telegram_markdown_v2("hello world"), "hello world");
}

#[test]
fn slice45_30_reserved_chars_escaped() {
    assert_eq!(escape_telegram_markdown_v2("a_b*c[d]"), "a\\_b\\*c\\[d\\]");
}

#[test]
fn slice45_31_already_escaped_char_kept() {
    assert_eq!(escape_telegram_markdown_v2("a\\_b"), "a\\_b");
}

#[test]
fn slice45_32_even_backslashes_do_not_escape() {
    assert_eq!(escape_telegram_markdown_v2("a\\\\_b"), "a\\\\\\_b");
}

#[test]
fn slice45_33_trailing_backslash_kept() {
    assert_eq!(escape_telegram_markdown_v2("abc\\"), "abc\\");
}

#[test]
fn slice45_34_dot_and_bang_escaped() {
    assert_eq!(escape_telegram_markdown_v2("v1.0!"), "v1\\.0\\!");
}

#[test]
fn slice45_35_empty_stays_empty() {
    assert_eq!(escape_telegram_markdown_v2(""), "");
}

#[test]
fn slice45_36_parens_and_tilde_escaped() {
    assert_eq!(escape_telegram_markdown_v2("(a~b)"), "\\(a\\~b\\)");
}

// --- unescape_telegram_markdown_v2 (37-40) ---

#[test]
fn slice45_37_unescape_restores_reserved() {
    assert_eq!(unescape_telegram_markdown_v2("a\\_b\\*c"), "a_b*c");
}

#[test]
fn slice45_38_unescape_leaves_plain_backslash_runs() {
    assert_eq!(unescape_telegram_markdown_v2("a\\\\b"), "a\\\\b");
}

#[test]
fn slice45_39_unescape_empty_stays_empty() {
    assert_eq!(unescape_telegram_markdown_v2(""), "");
}

#[test]
fn slice45_40_unescape_plain_text_unchanged() {
    assert_eq!(unescape_telegram_markdown_v2("hello"), "hello");
}

// --- resolve_fallback_text (41-44) ---

#[test]
fn slice45_41_explicit_raw_fallback_wins() {
    assert_eq!(
        resolve_fallback_text("a\\_b", Some("raw"), Some(FallbackParseMode::MarkdownV2)),
        "raw"
    );
}

#[test]
fn slice45_42_markdown_v2_unescapes_by_default() {
    assert_eq!(
        resolve_fallback_text("a\\_b", None, Some(FallbackParseMode::MarkdownV2)),
        "a_b"
    );
}

#[test]
fn slice45_43_markdown_keeps_text_as_is() {
    assert_eq!(
        resolve_fallback_text("a\\_b", None, Some(FallbackParseMode::Markdown)),
        "a\\_b"
    );
}

#[test]
fn slice45_44_no_parse_mode_keeps_text_as_is() {
    assert_eq!(resolve_fallback_text("a\\_b", None, None), "a\\_b");
}

// --- should_try_escape_retry (45-46) ---

#[test]
fn slice45_45_escape_retry_when_all_conditions_hold() {
    assert!(should_try_escape_retry(
        Some(FallbackParseMode::MarkdownV2),
        true,
        true,
        true
    ));
}

#[test]
fn slice45_46_no_retry_when_escape_changes_nothing() {
    assert!(!should_try_escape_retry(
        Some(FallbackParseMode::MarkdownV2),
        true,
        true,
        false
    ));
}

// --- should_fall_back_to_raw (47) + strip flags (48-49) ---

#[test]
fn slice45_47_presentation_error_falls_back() {
    assert!(should_fall_back_to_raw(true));
    assert!(!should_fall_back_to_raw(false));
}

#[test]
fn slice45_48_strip_drops_both_flags_when_present() {
    assert_eq!(
        strip_markdown_formatting_options(true, true),
        StrippedFormatFlags {
            parse_mode_removed: true,
            entities_removed: true,
        }
    );
}

#[test]
fn slice45_49_strip_keeps_absent_flags() {
    assert_eq!(
        strip_markdown_formatting_options(false, false),
        StrippedFormatFlags {
            parse_mode_removed: false,
            entities_removed: false,
        }
    );
}
