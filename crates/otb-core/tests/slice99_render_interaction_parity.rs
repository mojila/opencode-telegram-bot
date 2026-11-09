//! Slice 99 parity: block-parser + keyboard-debounce + interaction-guard +
//! message-router gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/render/block-parser.ts` (`MAX_NESTING_DEPTH`, heading clamp,
//! `prefixLines`, `createPlainBlock` trim/blank gate, `parseColumnAlign`
//! all-null gate with `null -> "left"`, ordered/unordered markers,
//! `checked` checkbox prefixes, list-item `trimEnd`, table-row `" | "`
//! join, `lang ?? undefined`, `depth >= 6` degrade gate, empty-table /
//! empty-list gates, thematic-break text),
//! `src/bot/keyboards/keyboard-manager.ts` (`UPDATE_DEBOUNCE_MS`,
//! `now - last < 2000` debounce, `state !== null` initialized gate,
//! `model.variant || "default"` blank gate, fresh-context zero gate,
//! `"build"` fallback, `keyboard.updated` key),
//! `src/bot/middleware/interaction-guard.ts`
//! (`getInteractionBlockedMessage` kind x reason matrix,
//! `getQueuedPhotoMediaBytes` empty/invalid/sum gates,
//! `isQueueableInput` busy/stateless/text/prompt gate) and
//! `src/bot/routers/message-router.ts` (withdraw `removed` / `gone` /
//! error reply-key choice, menu-blocked-while-active gate, `/` command
//! gate). `src/bot/render/pipeline.ts` is thin glue over owners in
//! `rich_blocks` / `block_split_gates` and contributes no gate here.
//!
//! Already covered elsewhere and NOT re-tested here:
//! `formatVariantForButton` rendering, `resolveInteractionGuardDecision`
//! (`interaction_guard_edge_gates`), `liftTextListContent` /
//! `splitOversizeTelegramBlocks` / `toRichBlock`.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `block_interaction_gates` is ported (only the constant-value tests pass).

use otb_core::block_interaction_gates::*;

// ---- constants (5) ----

#[test]
fn max_nesting_depth_value_parity() {
    assert_eq!(MAX_NESTING_DEPTH, 6);
}

#[test]
fn keyboard_debounce_ms_value_parity() {
    assert_eq!(KEYBOARD_UPDATE_DEBOUNCE_MS, 2000);
}

#[test]
fn thematic_break_text_value_parity() {
    assert_eq!(THEMATIC_BREAK_TEXT, "──────────");
}

#[test]
fn fallback_agent_value_parity() {
    assert_eq!(FALLBACK_AGENT, "build");
}

#[test]
fn keyboard_updated_key_value_parity() {
    assert_eq!(KEYBOARD_UPDATED_KEY, "keyboard.updated");
}

// ---- block-parser (16) ----

#[test]
fn heading_level_clamps_low_parity() {
    assert_eq!(clamp_heading_level(0), 1);
    assert_eq!(clamp_heading_level(-2), 1);
}

#[test]
fn heading_level_clamps_high_parity() {
    assert_eq!(clamp_heading_level(6), 6);
    assert_eq!(clamp_heading_level(9), 6);
}

#[test]
fn heading_level_passes_middle_parity() {
    assert_eq!(clamp_heading_level(1), 1);
    assert_eq!(clamp_heading_level(3), 3);
}

#[test]
fn prefix_lines_multiline_parity() {
    assert_eq!(prefix_lines("a\nb", "> "), "> a\n> b".to_string());
}

#[test]
fn prefix_lines_single_parity() {
    assert_eq!(prefix_lines("hi", "- "), "- hi".to_string());
}

#[test]
fn plain_block_trims_parity() {
    assert_eq!(create_plain_text("  hi  "), Some("hi".to_string()));
}

#[test]
fn plain_block_blank_is_none_parity() {
    assert_eq!(create_plain_text("   "), None);
    assert_eq!(create_plain_text(""), None);
}

#[test]
fn column_align_all_null_is_none_parity() {
    assert_eq!(parse_column_align(&[]), None);
    assert_eq!(parse_column_align(&[None, None]), None);
}

#[test]
fn column_align_maps_null_to_left_parity() {
    assert_eq!(
        parse_column_align(&[Some("center"), None]),
        Some(vec!["center".to_string(), "left".to_string()])
    );
}

#[test]
fn list_marker_ordered_parity() {
    assert_eq!(list_marker(true, 0), "1. ".to_string());
    assert_eq!(list_marker(true, 2), "3. ".to_string());
}

#[test]
fn list_marker_unordered_parity() {
    assert_eq!(list_marker(false, 0), "- ".to_string());
    assert_eq!(list_marker(false, 4), "- ".to_string());
}

#[test]
fn checkbox_prefix_parity() {
    assert_eq!(checkbox_prefix(Some(true)), "✅ ");
    assert_eq!(checkbox_prefix(Some(false)), "🔲 ");
    assert_eq!(checkbox_prefix(None), "");
}

#[test]
fn list_item_trims_trailing_parity() {
    assert_eq!(render_list_item("- ", "", "hi  "), "- hi".to_string());
    assert_eq!(
        render_list_item("1. ", "✅ ", "done\n"),
        "1. ✅ done".to_string()
    );
}

#[test]
fn table_row_join_parity() {
    assert_eq!(join_table_row(&["a", "b"]), "a | b".to_string());
    assert_eq!(join_table_row(&["only"]), "only".to_string());
}

#[test]
fn code_language_passthrough_parity() {
    assert_eq!(code_language(Some("ts")), Some("ts".to_string()));
    assert_eq!(code_language(None), None);
}

#[test]
fn degrade_at_max_depth_parity() {
    assert!(should_degrade_to_plain(6));
    assert!(should_degrade_to_plain(9));
    assert!(!should_degrade_to_plain(5));
}

// ---- keyboard (5) ----

#[test]
fn debounce_inside_window_parity() {
    assert!(should_debounce_keyboard_update(1500, 0));
    assert!(should_debounce_keyboard_update(1999, 0));
}

#[test]
fn debounce_at_edge_parity() {
    assert!(!should_debounce_keyboard_update(2000, 0));
}

#[test]
fn debounce_after_window_parity() {
    assert!(!should_debounce_keyboard_update(5000, 0));
}

#[test]
fn keyboard_initialized_gate_parity() {
    assert!(is_keyboard_initialized(true));
    assert!(!is_keyboard_initialized(false));
}

#[test]
fn variant_blank_falls_back_parity() {
    assert_eq!(or_default_variant(Some("pro")), "pro");
    assert_eq!(or_default_variant(None), "default");
    assert_eq!(or_default_variant(Some("")), "default");
}

#[test]
fn fresh_context_zero_gate_parity() {
    assert!(is_fresh_context(0));
    assert!(!is_fresh_context(5));
}

// ---- interaction-guard (14) ----

#[test]
fn blocked_permission_command_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("permission"), Some("command_not_allowed")),
        "permission.blocked.command_not_allowed"
    );
}

#[test]
fn blocked_permission_default_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("permission"), Some("expected_callback")),
        "permission.blocked.expected_reply"
    );
    assert_eq!(
        interaction_blocked_message_key(Some("permission"), None),
        "permission.blocked.expected_reply"
    );
}

#[test]
fn blocked_inline_command_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("inline"), Some("command_not_allowed")),
        "inline.blocked.command_not_allowed"
    );
}

#[test]
fn blocked_inline_default_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("inline"), Some("expected_callback")),
        "inline.blocked.expected_choice"
    );
    assert_eq!(
        interaction_blocked_message_key(Some("inline"), None),
        "inline.blocked.expected_choice"
    );
}

#[test]
fn blocked_question_command_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("question"), Some("command_not_allowed")),
        "question.blocked.command_not_allowed"
    );
}

#[test]
fn blocked_question_default_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("question"), Some("expected_text")),
        "question.blocked.expected_answer"
    );
    assert_eq!(
        interaction_blocked_message_key(Some("question"), None),
        "question.blocked.expected_answer"
    );
}

#[test]
fn blocked_rename_command_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("rename"), Some("command_not_allowed")),
        "rename.blocked.command_not_allowed"
    );
}

#[test]
fn blocked_rename_default_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("rename"), Some("expected_callback")),
        "rename.blocked.expected_name"
    );
    assert_eq!(
        interaction_blocked_message_key(Some("rename"), None),
        "rename.blocked.expected_name"
    );
}

#[test]
fn blocked_task_command_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("task"), Some("command_not_allowed")),
        "task.blocked.command_not_allowed"
    );
}

#[test]
fn blocked_task_default_parity() {
    assert_eq!(
        interaction_blocked_message_key(Some("task"), Some("expected_text")),
        "task.blocked.expected_input"
    );
    assert_eq!(
        interaction_blocked_message_key(Some("task"), None),
        "task.blocked.expected_input"
    );
}

#[test]
fn blocked_generic_expired_parity() {
    assert_eq!(
        interaction_blocked_message_key(None, Some("expired")),
        "interaction.blocked.expired"
    );
}

#[test]
fn blocked_generic_default_parity() {
    assert_eq!(
        interaction_blocked_message_key(None, Some("expected_callback")),
        "interaction.blocked.expected_callback"
    );
    assert_eq!(
        interaction_blocked_message_key(None, Some("command_not_allowed")),
        "interaction.blocked.command_not_allowed"
    );
    assert_eq!(
        interaction_blocked_message_key(None, None),
        "interaction.blocked.expected_text"
    );
}

#[test]
fn photo_bytes_sum_parity() {
    assert_eq!(sum_photo_media_bytes(&[]), Some(0));
    assert_eq!(sum_photo_media_bytes(&[Some(10), Some(5)]), Some(15));
}

#[test]
fn photo_bytes_invalid_parity() {
    assert_eq!(sum_photo_media_bytes(&[None]), None);
    assert_eq!(sum_photo_media_bytes(&[Some(-1)]), None);
    assert_eq!(sum_photo_media_bytes(&[Some(i64::MAX), Some(1)]), None);
}

// ---- message-router (7) ----

#[test]
fn queueable_gate_true_parity() {
    assert!(should_queue_text_prompt(true, false, true, true));
}

#[test]
fn queueable_gate_false_parity() {
    assert!(!should_queue_text_prompt(false, false, true, true));
    assert!(!should_queue_text_prompt(true, true, true, true));
    assert!(!should_queue_text_prompt(true, false, false, true));
    assert!(!should_queue_text_prompt(true, false, true, false));
}

#[test]
fn withdraw_removed_key_parity() {
    assert_eq!(withdraw_reply_key("removed"), "queue.removed");
}

#[test]
fn withdraw_gone_key_parity() {
    assert_eq!(withdraw_reply_key("gone"), "queue.not_found");
}

#[test]
fn withdraw_error_fallback_parity() {
    assert_eq!(withdraw_reply_key("error"), "bot.prompt_send_error");
    assert_eq!(withdraw_reply_key(""), "bot.prompt_send_error");
}

#[test]
fn menu_block_active_gate_parity() {
    assert!(should_block_menu_while_active(true));
    assert!(!should_block_menu_while_active(false));
}

#[test]
fn command_text_prefix_parity() {
    assert!(is_command_text("/new"));
    assert!(!is_command_text("hi"));
    assert!(!is_command_text(""));
}
