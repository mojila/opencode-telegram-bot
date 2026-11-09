//! Slice 95 parity: context-control + feedback + inline-menu active-state gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/feedback.ts` (`CALLBACK_ANSWER_MAX_LENGTH`,
//! `resolveAnswerText` head cut),
//! `src/bot/callbacks/context-control-callback-handler.ts` (the exact
//! `compact:details` / `compact:confirm` dispatch, the details/confirm
//! stage gates, the missing-session gate),
//! `src/bot/callbacks/inline-menu-cancel-callback-handler.ts` remainder
//! (the ensure/clear flow behind the parse) and
//! `src/bot/menus/inline-menu.ts` remainder (the numeric-`message_id`
//! gate, the active-metadata validation, the ensure-active match, the
//! clear kind gate, the cancel-button row trim + settings-close text +
//! cancel data) plus `src/bot/menus/context-control-menu.ts` (window
//! percent, unknown-limit gate, `$X.XX` cost line).
//! `INLINE_MENU_KINDS` / `isInlineMenuKind` / `INLINE_MENU_CANCEL_PREFIX` /
//! `LEGACY_CONTEXT_CANCEL_CALLBACK` / `parse_inline_cancel_callback` and
//! `formatTokenCount` already live in `worktree_history_inline_gates` /
//! `keyboards` and are NOT re-tested here; kind recognition arrives as
//! explicit bools.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `context_inline_gates` is ported (only the constant-value tests pass).

use otb_core::context_inline_gates::*;

fn button(text: &str, data: &str) -> InlineButton {
    InlineButton {
        text: text.to_string(),
        callback_data: data.to_string(),
    }
}

// ---- constants (5) ----

#[test]
fn callback_answer_max_length_value_parity() {
    assert_eq!(CALLBACK_ANSWER_MAX_LENGTH, 200);
}

#[test]
fn compact_details_callback_value_parity() {
    assert_eq!(COMPACT_DETAILS_CALLBACK, "compact:details");
}

#[test]
fn compact_confirm_callback_value_parity() {
    assert_eq!(COMPACT_CONFIRM_CALLBACK, "compact:confirm");
}

#[test]
fn inline_inactive_key_value_parity() {
    assert_eq!(INLINE_MENU_INACTIVE_KEY, "inline.inactive_callback");
}

#[test]
fn context_no_session_key_value_parity() {
    assert_eq!(CONTEXT_NO_SESSION_KEY, "context.no_active_session");
}

// ---- answer truncation (6) ----

#[test]
fn short_answer_kept_whole_parity() {
    assert_eq!(truncate_callback_answer("done"), "done");
}

#[test]
fn exact_budget_answer_kept_whole_parity() {
    let text = "x".repeat(CALLBACK_ANSWER_MAX_LENGTH);
    assert_eq!(truncate_callback_answer(&text), text);
}

#[test]
fn overlong_answer_cut_to_budget_parity() {
    let text = "x".repeat(CALLBACK_ANSWER_MAX_LENGTH + 50);
    let out = truncate_callback_answer(&text);
    assert_eq!(out.chars().count(), CALLBACK_ANSWER_MAX_LENGTH);
}

#[test]
fn empty_answer_kept_whole_parity() {
    assert_eq!(truncate_callback_answer(""), "");
}

#[test]
fn truncation_needed_only_over_budget_parity() {
    assert!(!should_truncate_callback_answer(0));
    assert!(!should_truncate_callback_answer(CALLBACK_ANSWER_MAX_LENGTH));
    assert!(should_truncate_callback_answer(
        CALLBACK_ANSWER_MAX_LENGTH + 1
    ));
}

#[test]
fn truncation_keeps_head_parity() {
    let text = format!("{}tail", "h".repeat(CALLBACK_ANSWER_MAX_LENGTH));
    let out = truncate_callback_answer(&text);
    assert!(out.starts_with(&"h".repeat(10)));
    assert!(!out.contains("tail"));
}

// ---- callback message-id resolve (4) ----

#[test]
fn inline_message_id_some_parity() {
    assert_eq!(inline_callback_message_id(true, Some(7)), Some(7));
}

#[test]
fn inline_message_id_no_message_parity() {
    assert_eq!(inline_callback_message_id(false, Some(7)), None);
}

#[test]
fn inline_message_id_missing_id_parity() {
    assert_eq!(inline_callback_message_id(true, None), None);
}

#[test]
fn inline_message_id_non_numeric_parity() {
    // A non-numeric `message_id` arrives as `None` (TS `typeof` check).
    assert_eq!(inline_callback_message_id(true, None), None);
    assert_eq!(inline_callback_message_id(false, None), None);
}

// ---- active metadata validation (6) ----

#[test]
fn active_metadata_valid_parity() {
    assert_eq!(
        resolve_active_inline_metadata(true, Some("context"), true, Some(11)),
        Some(ActiveInlineMenuRef {
            menu_kind: "context".to_string(),
            message_id: 11,
        })
    );
}

#[test]
fn active_metadata_rejects_non_inline_parity() {
    assert_eq!(
        resolve_active_inline_metadata(false, Some("context"), true, Some(11)),
        None
    );
}

#[test]
fn active_metadata_rejects_unknown_kind_parity() {
    assert_eq!(
        resolve_active_inline_metadata(true, Some("nope"), false, Some(11)),
        None
    );
}

#[test]
fn active_metadata_rejects_missing_kind_parity() {
    assert_eq!(
        resolve_active_inline_metadata(true, None, false, Some(11)),
        None
    );
}

#[test]
fn active_metadata_rejects_missing_id_parity() {
    assert_eq!(
        resolve_active_inline_metadata(true, Some("context"), true, None),
        None
    );
}

#[test]
fn active_metadata_rejects_none_state_parity() {
    assert_eq!(
        resolve_active_inline_metadata(false, None, false, None),
        None
    );
}

// ---- ensure-active match (5) ----

#[test]
fn inline_active_when_all_match_parity() {
    assert!(is_inline_menu_active(true, true, true, true));
}

#[test]
fn inline_inactive_on_kind_mismatch_parity() {
    assert!(!is_inline_menu_active(true, false, true, true));
}

#[test]
fn inline_inactive_on_message_mismatch_parity() {
    assert!(!is_inline_menu_active(true, true, false, true));
}

#[test]
fn inline_inactive_without_active_parity() {
    assert!(!is_inline_menu_active(false, true, true, true));
}

#[test]
fn inline_inactive_without_callback_id_parity() {
    assert!(!is_inline_menu_active(true, true, true, false));
}

// ---- clear gate (3) ----

#[test]
fn clears_when_inline_parity() {
    assert!(should_clear_active_inline_menu(Some("inline")));
}

#[test]
fn keeps_when_other_kind_parity() {
    assert!(!should_clear_active_inline_menu(Some("custom")));
    assert!(!should_clear_active_inline_menu(Some("question")));
}

#[test]
fn keeps_without_state_parity() {
    assert!(!should_clear_active_inline_menu(None));
}

// ---- cancel button (8) ----

#[test]
fn close_text_only_for_settings_parity() {
    assert!(is_close_text_kind("settings"));
    assert!(!is_close_text_kind("context"));
    assert!(!is_close_text_kind("model"));
}

#[test]
fn cancel_data_build_parity() {
    assert_eq!(
        inline_cancel_callback_for("context"),
        "inline:cancel:context"
    );
    assert_eq!(
        inline_cancel_callback_for("settings"),
        "inline:cancel:settings"
    );
}

#[test]
fn trailing_empty_count_none_parity() {
    assert_eq!(trailing_empty_row_count(&[false, false]), 0);
    assert_eq!(trailing_empty_row_count(&[]), 0);
}

#[test]
fn trailing_empty_count_trims_parity() {
    assert_eq!(trailing_empty_row_count(&[false, true]), 1);
    assert_eq!(trailing_empty_row_count(&[true, true]), 2);
    assert_eq!(trailing_empty_row_count(&[false, true, true]), 2);
}

#[test]
fn append_trims_trailing_empty_parity() {
    let rows = vec![vec![button("a", "a:1")], vec![]];
    let out = append_inline_cancel_button(rows, "Cancel", "inline:cancel:context");
    assert!(!out.iter().any(|row| row.is_empty()));
}

#[test]
fn append_adds_row_when_nonempty_parity() {
    let rows = vec![vec![button("a", "a:1")]];
    let before = rows.len();
    let out = append_inline_cancel_button(rows, "Cancel", "inline:cancel:context");
    assert_eq!(out.len(), before + 1);
}

#[test]
fn append_no_extra_row_when_empty_parity() {
    let out = append_inline_cancel_button(vec![], "Cancel", "inline:cancel:context");
    assert_eq!(out.len(), 1);
}

#[test]
fn append_pushes_cancel_button_parity() {
    let rows = vec![vec![button("a", "a:1")]];
    let out = append_inline_cancel_button(rows, "Cancel", "inline:cancel:context");
    let last = out.last().unwrap();
    assert_eq!(last.last().unwrap().text, "Cancel");
    assert_eq!(last.last().unwrap().callback_data, "inline:cancel:context");
}

// ---- context window + cost (5) ----

#[test]
fn window_percent_half_parity() {
    assert_eq!(context_window_percent(50, 100), 50);
}

#[test]
fn window_percent_zero_limit_parity() {
    assert_eq!(context_window_percent(10, 0), 0);
}

#[test]
fn window_percent_rounds_parity() {
    // TS `Math.round((1 / 3) * 100)` reads 33.
    assert_eq!(context_window_percent(1, 3), 33);
    assert_eq!(context_window_percent(0, 100), 0);
    assert_eq!(context_window_percent(100, 100), 100);
}

#[test]
fn unknown_limit_gate_parity() {
    assert!(is_unknown_window_limit(0));
    assert!(!is_unknown_window_limit(1));
}

#[test]
fn compact_cost_format_parity() {
    assert_eq!(format_compact_cost(1.5), "$1.50");
    assert_eq!(format_compact_cost(0.0), "$0.00");
}

// ---- compact dispatch + stages + session (6) ----

#[test]
fn compact_details_data_match_parity() {
    assert!(is_compact_details_data(Some("compact:details")));
    assert!(!is_compact_details_data(Some("compact:confirm")));
    assert!(!is_compact_details_data(None));
}

#[test]
fn compact_confirm_data_match_parity() {
    assert!(is_compact_confirm_data(Some("compact:confirm")));
    assert!(!is_compact_confirm_data(Some("compact:details")));
    assert!(!is_compact_confirm_data(None));
    assert!(!is_compact_confirm_data(Some("")));
}

#[test]
fn compact_details_stage_gate_parity() {
    assert!(is_details_stage(Some("details")));
    assert!(!is_details_stage(Some("confirm")));
    assert!(!is_details_stage(None));
}

#[test]
fn compact_confirm_stage_gate_parity() {
    assert!(is_confirm_stage(Some("confirm")));
    assert!(!is_confirm_stage(Some("details")));
    assert!(!is_confirm_stage(None));
}

#[test]
fn rejects_compact_without_session_parity() {
    assert!(should_reject_compact_without_session(false));
}

#[test]
fn keeps_compact_with_session_parity() {
    assert!(!should_reject_compact_without_session(true));
}
