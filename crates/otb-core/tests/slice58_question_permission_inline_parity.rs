//! Parity port of the question + permission + inline-menu +
//! context-control pure gates (slice 58, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/bot/menus/question-menu.ts` (button budget, emoji markers,
//!   `question:*` builders, `parseInt` parses, progress text, button cut,
//!   custom-answer collapse, segment helpers, safe slice, tail truncation,
//!   fallback join)
//! - `src/bot/callbacks/question-callback-handler.ts` (prefix guard, action
//!   dispatch, last-question gate)
//! - `src/bot/menus/permission-menu.ts` + `src/bot/callbacks/permission-callback-handler.ts`
//!   (callbacks, emoji map, reply gate, not-found / not-modified matches,
//!   outcome lines, pattern budget text)
//! - `src/bot/menus/inline-menu.ts` + cancel handler + `callback-router.ts`
//!   (cancel prefix, legacy cancel, 11 kinds, prefix parse) + `feedback.ts`
//!   (200-char answer truncation)
//! - `src/bot/menus/context-control-menu.ts` + compact handler guards
//!   (`compact:details` / `compact:confirm`, percent, cost amount)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::question_permission_inline_gates::*;

// --- question constants ---

#[test]
fn question_budgets() {
    assert_eq!(QUESTION_BUTTON_MAX_LENGTH, 60);
    assert_eq!(QUESTION_MESSAGE_LIMIT, 4096);
}

#[test]
fn question_markers() {
    assert_eq!(QUESTION_EMOJI, "❓");
    assert_eq!(CUSTOM_ANSWER_EMOJI, "✏️");
    assert_eq!(QUESTION_TRUNCATION_SUFFIX, "…");
    assert_eq!(QUESTION_CALLBACK_PREFIX, "question:");
}

// --- question callback build ---

#[test]
fn build_question_select_callback_shape() {
    assert_eq!(build_question_select_callback(2, 5), "question:select:2:5");
}

#[test]
fn build_question_submit_callback_shape() {
    assert_eq!(build_question_submit_callback(0), "question:submit:0");
}

#[test]
fn build_question_custom_callback_shape() {
    assert_eq!(build_question_custom_callback(1), "question:custom:1");
}

#[test]
fn build_question_toggle_custom_callback_shape() {
    assert_eq!(
        build_question_toggle_custom_callback(3),
        "question:toggle_custom:3"
    );
}

#[test]
fn build_question_cancel_callback_shape() {
    assert_eq!(build_question_cancel_callback(4), "question:cancel:4");
}

// --- question callback parse ---

#[test]
fn parse_question_callback_select() {
    assert_eq!(
        parse_question_callback("question:select:2:5"),
        Some(QuestionCallback {
            action: QuestionCallbackAction::Select,
            question_index: 2,
            option_index: Some(5),
        })
    );
}

#[test]
fn parse_question_callback_submit_cancel_custom() {
    assert_eq!(
        parse_question_callback("question:submit:0"),
        Some(QuestionCallback {
            action: QuestionCallbackAction::Submit,
            question_index: 0,
            option_index: None,
        })
    );
    assert_eq!(
        parse_question_callback("question:cancel:4").map(|callback| callback.action),
        Some(QuestionCallbackAction::Cancel)
    );
    assert_eq!(
        parse_question_callback("question:custom:1").map(|callback| callback.action),
        Some(QuestionCallbackAction::Custom)
    );
    assert_eq!(
        parse_question_callback("question:toggle_custom:3").map(|callback| callback.action),
        Some(QuestionCallbackAction::ToggleCustom)
    );
}

#[test]
fn parse_question_callback_rejects_wrong_prefix() {
    assert_eq!(parse_question_callback("commands:select:1"), None);
    assert_eq!(parse_question_callback(""), None);
}

#[test]
fn parse_question_callback_rejects_unknown_action() {
    assert_eq!(parse_question_callback("question:bogus:1"), None);
}

#[test]
fn parse_question_callback_rejects_non_numeric_index() {
    assert_eq!(parse_question_callback("question:select:abc"), None);
    assert_eq!(parse_question_callback("question:cancel:"), None);
    assert_eq!(parse_question_callback("question:select:1:xyz"), None);
}

#[test]
fn parse_question_callback_tolerates_trailing_garbage() {
    assert_eq!(
        parse_question_callback("question:cancel:2x").map(|callback| callback.question_index),
        Some(2)
    );
}

// --- question guards + gone ---

#[test]
fn is_question_callback_guard() {
    assert!(is_question_callback("question:select:0:1"));
    assert!(!is_question_callback("permission:once"));
}

#[test]
fn is_last_question_branches() {
    assert!(is_last_question(2, 3));
    assert!(!is_last_question(1, 3));
}

#[test]
fn is_question_request_gone_tag_variants() {
    assert!(is_question_request_gone(
        Some("QuestionNotFoundError"),
        false
    ));
    assert!(is_question_request_gone(
        Some("FormAlreadySettledError"),
        false
    ));
}

#[test]
fn is_question_request_gone_flag_and_other() {
    assert!(is_question_request_gone(None, true));
    assert!(!is_question_request_gone(Some("OtherError"), false));
    assert!(!is_question_request_gone(None, false));
}

// --- question text ---

#[test]
fn format_question_button_text_unselected_short() {
    assert_eq!(format_question_button_text("Save", false), "Save");
}

#[test]
fn format_question_button_text_selected_adds_check() {
    assert_eq!(format_question_button_text("Save", true), "✅ Save");
}

#[test]
fn format_question_button_text_cuts_at_60() {
    let label = format_question_button_text(&"a".repeat(100), false);
    assert_eq!(label.chars().count(), 60);
    assert!(label.ends_with("..."));
}

#[test]
fn normalize_custom_answer_label_collapses_newlines() {
    assert_eq!(normalize_custom_answer_label("line1\nline2"), "line1 line2");
    assert_eq!(normalize_custom_answer_label("a  \n  b"), "a b");
}

#[test]
fn question_progress_text_branches() {
    assert_eq!(question_progress_text(0, 3), "1/3");
    assert_eq!(question_progress_text(2, 3), "3/3");
    assert_eq!(question_progress_text(0, 0), "");
}

// --- question segments ---

#[test]
fn question_segment_length_and_plain_text() {
    let segment = QuestionSegment {
        label: Some("Title".to_string()),
        rest: "body".to_string(),
    };
    assert_eq!(question_segment_length(&segment), 9);
    assert_eq!(question_segment_plain_text(&segment), "Titlebody");
    let bare = QuestionSegment {
        label: None,
        rest: "body".to_string(),
    };
    assert_eq!(question_segment_plain_text(&bare), "body");
}

#[test]
fn slice_on_safe_boundary_branches() {
    assert_eq!(slice_on_safe_boundary("abc", 10), "abc");
    assert_eq!(slice_on_safe_boundary("abcdef", 4), "abcd");
    assert_eq!(slice_on_safe_boundary("abcdef", 0), "");
}

#[test]
fn truncate_question_segments_fits() {
    let segments = vec![
        QuestionSegment {
            label: None,
            rest: "ab".to_string(),
        },
        QuestionSegment {
            label: None,
            rest: "cd".to_string(),
        },
    ];
    assert_eq!(truncate_question_segments(&segments, 100), segments);
}

#[test]
fn truncate_question_segments_cuts_tail() {
    let segments = vec![
        QuestionSegment {
            label: None,
            rest: "aaaa".to_string(),
        },
        QuestionSegment {
            label: None,
            rest: "bbbb".to_string(),
        },
    ];
    assert_eq!(
        truncate_question_segments(&segments, 8),
        vec![
            QuestionSegment {
                label: None,
                rest: "aaaa".to_string()
            },
            QuestionSegment {
                label: None,
                rest: "b…".to_string()
            },
        ]
    );
}

#[test]
fn question_fallback_text_joins_with_blank_line() {
    let segments = vec![
        QuestionSegment {
            label: Some("T".to_string()),
            rest: String::new(),
        },
        QuestionSegment {
            label: None,
            rest: "body".to_string(),
        },
    ];
    assert_eq!(question_fallback_text(&segments), "T\n\nbody");
}

// --- permission constants ---

#[test]
fn permission_prefix_and_callback_values() {
    assert_eq!(PERMISSION_CALLBACK_PREFIX, "permission:");
    assert_eq!(PERMISSION_CALLBACK_ONCE, "permission:once");
    assert_eq!(PERMISSION_CALLBACK_ALWAYS, "permission:always");
    assert_eq!(PERMISSION_CALLBACK_REJECT, "permission:reject");
}

#[test]
fn permission_pattern_budget_constants() {
    assert_eq!(PERMISSION_MESSAGE_LIMIT, 4096);
    assert_eq!(PERMISSION_PATTERN_PREFIX, "• ");
    assert_eq!(PERMISSION_CUT_SUFFIX, "…\n");
    assert_eq!(PERMISSION_CUT_PATTERNS_LINE, "• …\n");
}

// --- permission gates ---

#[test]
fn permission_emoji_known_and_fallback() {
    assert_eq!(permission_emoji_for("bash"), "⚡");
    assert_eq!(permission_emoji_for("edit"), "✏️");
    assert_eq!(permission_emoji_for("external_directory"), "📁");
    assert_eq!(permission_emoji_for("nope"), "🔐");
}

#[test]
fn is_permission_reply_value_branches() {
    assert!(is_permission_reply_value("once"));
    assert!(is_permission_reply_value("always"));
    assert!(is_permission_reply_value("reject"));
    assert!(!is_permission_reply_value("later"));
    assert!(!is_permission_reply_value(""));
}

#[test]
fn classify_permission_callback_known() {
    assert_eq!(
        classify_permission_callback("permission:once"),
        Some(PermissionReply::Once)
    );
    assert_eq!(
        classify_permission_callback("permission:always"),
        Some(PermissionReply::Always)
    );
    assert_eq!(
        classify_permission_callback("permission:reject"),
        Some(PermissionReply::Reject)
    );
}

#[test]
fn classify_permission_callback_unknown() {
    assert_eq!(classify_permission_callback("permission:later"), None);
    assert_eq!(classify_permission_callback("question:cancel:0"), None);
}

#[test]
fn is_permission_callback_guard() {
    assert!(is_permission_callback("permission:once"));
    assert!(!is_permission_callback("question:cancel:0"));
}

#[test]
fn is_permission_request_not_found_matches() {
    assert!(is_permission_request_not_found(&PermissionErrorShape {
        tag: Some("PermissionNotFoundError".to_string()),
        ..Default::default()
    }));
    assert!(is_permission_request_not_found(&PermissionErrorShape {
        name: Some("NotFoundError".to_string()),
        ..Default::default()
    }));
    assert!(is_permission_request_not_found(&PermissionErrorShape {
        message: Some("Permission request not found".to_string()),
        ..Default::default()
    }));
    assert!(!is_permission_request_not_found(&PermissionErrorShape {
        message: Some("boom".to_string()),
        ..Default::default()
    }));
    assert!(!is_permission_request_not_found(
        &PermissionErrorShape::default()
    ));
}

#[test]
fn is_message_not_modified_branches() {
    assert!(is_message_not_modified_message(
        "Bad Request: message is not modified"
    ));
    assert!(is_message_not_modified_message("MESSAGE IS NOT MODIFIED"));
    assert!(!is_message_not_modified_message("chat not found"));
}

#[test]
fn classify_permission_outcome_line_branches() {
    assert_eq!(
        classify_permission_outcome_line(PermissionOutcomeKind::Replied {
            reply: PermissionReply::Once,
            outside: false
        }),
        PermissionOutcomeLine::ReplyOnce
    );
    assert_eq!(
        classify_permission_outcome_line(PermissionOutcomeKind::Replied {
            reply: PermissionReply::Reject,
            outside: true
        }),
        PermissionOutcomeLine::ReplyRejectOutside
    );
    assert_eq!(
        classify_permission_outcome_line(PermissionOutcomeKind::SettledOutside),
        PermissionOutcomeLine::SettledOutside
    );
    assert_eq!(
        classify_permission_outcome_line(PermissionOutcomeKind::NotAnswered),
        PermissionOutcomeLine::NotAnswered
    );
}

// --- permission text ---

#[test]
fn format_permission_text_no_patterns() {
    assert_eq!(format_permission_text("H", &[], "", None), "H");
}

#[test]
fn format_permission_text_with_patterns_grouped_and_status() {
    assert_eq!(
        format_permission_text("H", &["ls".to_string()], " (2)", Some("done")),
        "H• ls\n (2)\ndone"
    );
}

#[test]
fn format_permission_text_truncates_long_pattern() {
    let text = format_permission_text("H", &["a".repeat(5000)], "", None);
    assert_eq!(text.chars().count(), 4096);
    assert!(text.starts_with("H• "));
    assert!(text.ends_with("…\n"));
}

// --- inline menu ---

#[test]
fn inline_cancel_prefix_and_legacy() {
    assert_eq!(INLINE_MENU_CANCEL_PREFIX, "inline:cancel:");
    assert_eq!(LEGACY_CONTEXT_CANCEL_CALLBACK, "compact:cancel");
}

#[test]
fn inline_menu_kinds_eleven() {
    assert_eq!(INLINE_MENU_KINDS.len(), 11);
    assert!(INLINE_MENU_KINDS.contains(&"context"));
    assert!(INLINE_MENU_KINDS.contains(&"settings"));
    assert!(inline_menu_kind_name(InlineMenuKind::Context) == "context");
    assert!(inline_menu_kind_name(InlineMenuKind::Settings) == "settings");
}

#[test]
fn inline_menu_kind_parse_branches() {
    assert!(is_inline_menu_kind("context"));
    assert!(is_inline_menu_kind("worktree"));
    assert!(!is_inline_menu_kind("nope"));
    assert!(!is_inline_menu_kind(""));
    assert_eq!(
        parse_inline_menu_kind("context"),
        Some(InlineMenuKind::Context)
    );
    assert_eq!(parse_inline_menu_kind("nope"), None);
}

#[test]
fn inline_cancel_callback_data_shape() {
    assert_eq!(
        inline_cancel_callback_data(InlineMenuKind::Context),
        "inline:cancel:context"
    );
}

#[test]
fn parse_inline_cancel_callback_branches() {
    assert_eq!(
        parse_inline_cancel_callback("compact:cancel"),
        Some(InlineMenuKind::Context)
    );
    assert_eq!(
        parse_inline_cancel_callback("inline:cancel:model"),
        Some(InlineMenuKind::Model)
    );
    assert_eq!(parse_inline_cancel_callback("inline:cancel:nope"), None);
    assert_eq!(parse_inline_cancel_callback("question:cancel:0"), None);
}

// --- callback router + feedback ---

#[test]
fn parse_callback_prefix_branches() {
    assert_eq!(parse_callback_prefix("permission:once"), Some("permission"));
    assert_eq!(parse_callback_prefix("nocolon"), None);
    assert_eq!(parse_callback_prefix(":lead"), None);
}

#[test]
fn truncate_callback_answer_text_branches() {
    assert_eq!(truncate_callback_answer_text("ok"), "ok");
    let text = truncate_callback_answer_text(&"a".repeat(300));
    assert_eq!(text.chars().count(), 200);
}

// --- context control ---

#[test]
fn classify_compact_callback_branches() {
    assert_eq!(COMPACT_DETAILS_CALLBACK, "compact:details");
    assert_eq!(COMPACT_CONFIRM_CALLBACK, "compact:confirm");
    assert_eq!(
        classify_compact_callback("compact:details"),
        Some(CompactCallback::Details)
    );
    assert_eq!(
        classify_compact_callback("compact:confirm"),
        Some(CompactCallback::Confirm)
    );
    assert_eq!(classify_compact_callback("compact:nope"), None);
    assert!(is_compact_callback("compact:confirm"));
    assert!(!is_compact_callback("inline:cancel:context"));
}

#[test]
fn format_context_percent_branches() {
    assert_eq!(format_context_percent(50, 200), 25);
    assert_eq!(format_context_percent(1, 3), 33);
    assert_eq!(format_context_percent(10, 0), 0);
}

#[test]
fn format_cost_amount_shape() {
    assert_eq!(format_cost_amount(3.5), "$3.50");
    assert_eq!(format_cost_amount(0.0), "$0.00");
}
