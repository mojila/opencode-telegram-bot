//! Parity port of keyboard/message-pattern/render-remainder pure logic (slice 28b, RED).
//!
//! Mirrors `tests/bot/keyboards/queued-prompt-button.test.ts`,
//! `tests/bot/keyboards/main-reply-keyboard.test.ts`,
//! `tests/bot/message-patterns.test.ts`, `tests/bot/callbacks/goal-callback-handler.test.ts`
//! (label/action subset), `tests/bot/callbacks/permission-callback-handler.test.ts`
//! + `tests/bot/menus/permission-menu.test.ts` (pure text subset),
//! `tests/bot/callbacks/question-callback-handler.test.ts`
//! + `tests/bot/menus/variant-selection-menu.test.ts` (pure subsets),
//! `tests/bot/menus/*-selection-menu` callback/pagination subsets, and the
//! render remainder (`block-plain-text`, `text-splitter`, `block-splitter`
//! limits) not covered by the chunker/markdown-v2 slices.
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::keyboards::*;

// --- queued-prompt-button.ts (6 its) ---

#[test]
fn slice28b_01_queued_short_label_as_is() {
    assert_eq!(
        format_queued_prompt_button_label(1, "Fix the login bug"),
        "❌ 1. Fix the login bug"
    );
}

#[test]
fn slice28b_02_queued_collapses_whitespace() {
    assert_eq!(
        format_queued_prompt_button_label(2, "Fix\n  the   bug"),
        "❌ 2. Fix the bug"
    );
}

#[test]
fn slice28b_03_queued_truncates_long_prompt() {
    assert_eq!(
        format_queued_prompt_button_label(1, &"a".repeat(80)),
        format!("❌ 1. {}…", "a".repeat(27))
    );
}

#[test]
fn slice28b_04_queued_labels_numbered_by_position() {
    let labels = queued_prompt_button_labels(&["first".to_string(), "second".to_string()]);
    assert_eq!(labels, vec!["❌ 1. first", "❌ 2. second"]);
}

#[test]
fn slice28b_05_queued_find_behind_label() {
    let texts = vec!["first".to_string(), "second".to_string()];
    assert_eq!(
        find_queued_prompt_index_by_label(&texts, "❌ 2. second"),
        Some(1)
    );
}

#[test]
fn slice28b_06_queued_duplicates_and_stale() {
    let texts = vec!["same text".to_string(), "same text".to_string()];
    assert_eq!(
        find_queued_prompt_index_by_label(&texts, "❌ 1. same text"),
        Some(0)
    );
    assert_eq!(
        find_queued_prompt_index_by_label(&texts, "❌ 2. same text"),
        Some(1)
    );
    assert_eq!(find_queued_prompt_index_by_label(&texts, "❌ 2. gone"), None);
}

// --- main-reply-keyboard.ts (7 its) ---

#[test]
fn slice28b_07_main_keyboard_defaults() {
    let kb = create_main_keyboard("build", "openrouter", "openai/gpt-4o", None, None, &[], &[]);
    assert_eq!(kb.rows[0][0], "🛠️ Build Agent");
    assert_eq!(kb.rows[0][1], "📊 0");
    assert_eq!(kb.rows[1][0], "🧠 openrouter\nopenai/gpt-4o");
    assert_eq!(kb.rows[1][1], "💡 Default");
    assert!(kb.resized && kb.persistent);
}

#[test]
fn slice28b_08_main_keyboard_context_and_variant() {
    let info = ContextInfo { tokens_used: 150_000, tokens_limit: 1_500_000 };
    let kb = create_main_keyboard("plan", "provider", "model", Some(&info), Some("⚡ Fast"), &[], &[]);
    assert_eq!(kb.rows[0][0], "📋 Plan Agent");
    assert_eq!(kb.rows[0][1], "📊 150K / 1.5M (10%)");
    assert_eq!(kb.rows[1][1], "⚡ Fast");
}

#[test]
fn slice28b_09_main_keyboard_fixed_grid_without_queued() {
    let kb = create_main_keyboard("build", "openrouter", "openai/gpt-4o", None, None, &[], &[]);
    assert_eq!(kb.rows.iter().filter(|r| !r.is_empty()).count(), 2);
}

#[test]
fn slice28b_10_main_keyboard_queued_rows_on_top() {
    let kb = create_main_keyboard(
        "build", "openrouter", "openai/gpt-4o", None, None,
        &["❌ 1. first".to_string(), "❌ 2. second".to_string()], &[],
    );
    assert_eq!(kb.rows[0][0], "❌ 1. first");
    assert_eq!(kb.rows[1][0], "❌ 2. second");
    assert_eq!(kb.rows[2][0], "🛠️ Build Agent");
    assert_eq!(kb.rows[3][0], "🧠 openrouter\nopenai/gpt-4o");
}

#[test]
fn slice28b_11_main_keyboard_goal_row_above_grid() {
    let kb = create_main_keyboard(
        "build", "p", "m", None, None,
        &["queued 1".to_string(), "queued 2".to_string()],
        &["goal objective".to_string(), "pause".to_string()],
    );
    assert_eq!(kb.rows[2], vec!["goal objective".to_string(), "pause".to_string()]);
    assert_eq!(kb.rows[0][0], "queued 1");
    assert_eq!(kb.rows[3][0], "🛠️ Build Agent");
}

#[test]
fn slice28b_12_agent_keyboard_and_remove() {
    let kb = create_agent_keyboard("custom");
    let non_empty: Vec<_> = kb.rows.iter().filter(|r| !r.is_empty()).collect();
    assert_eq!(non_empty, vec![&vec!["🤖 Custom Agent".to_string()]]);
    assert!(kb.resized && kb.persistent);
    assert!(is_remove_keyboard_payload(true));
}

#[test]
fn slice28b_13_token_and_model_format() {
    assert_eq!(format_token_count(999), "999");
    assert_eq!(format_token_count(150_000), "150K");
    assert_eq!(format_token_count(1_500_000), "1.5M");
    assert_eq!(
        format_model_for_button("openrouter", "openai/gpt-4o"),
        "🧠 openrouter\nopenai/gpt-4o"
    );
    assert_eq!(agent_button_label("build"), "🛠️ Build Agent");
}

// --- goal-button.ts (5 its) ---

#[test]
fn slice28b_14_goal_active_labels_pause() {
    let labels = get_goal_button_labels(Some(GoalStatus::Active), "ship it");
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[1], "⏸ Pause");
    assert!(labels[0].starts_with("◎ active · "));
}

#[test]
fn slice28b_15_goal_paused_labels_resume() {
    let labels = get_goal_button_labels(Some(GoalStatus::Paused), "ship it");
    assert_eq!(labels[1], "▶ Continue");
}

#[test]
fn slice28b_16_goal_absent_labels_empty() {
    assert!(get_goal_button_labels(None, "ship it").is_empty());
}

#[test]
fn slice28b_17_goal_resolve_actions() {
    let labels = get_goal_button_labels(Some(GoalStatus::Active), "ship it");
    assert_eq!(
        resolve_goal_button(Some(GoalStatus::Active), &labels, &labels[0]),
        Some(GoalButtonAction::Details)
    );
    assert_eq!(
        resolve_goal_button(Some(GoalStatus::Active), &labels, &labels[1]),
        Some(GoalButtonAction::Pause)
    );
    assert_eq!(
        resolve_goal_button(Some(GoalStatus::Active), &labels, "nope"),
        None
    );
}

#[test]
fn slice28b_18_goal_button_pattern() {
    assert!(matches_goal_button_text("◎ active · ship it"));
    assert!(matches_goal_button_text("⏸ Pause"));
    assert!(!matches_goal_button_text("Create a migration plan"));
}

// --- message-patterns.ts (6 its) ---

#[test]
fn slice28b_19_pattern_model_button() {
    assert!(is_model_button_text("🧠 openrouter\nopenai/gpt-4o"));
    assert!(is_model_button_text("🧠 cliproxyapi2/gpt-5.3-codex"));
}

#[test]
fn slice28b_20_pattern_custom_agent_not_model() {
    assert!(!is_model_button_text("🤖 Reviewer Agent"));
    assert!(is_agent_mode_button_text("🤖 Sisyphus (Ultraworker) Agent"));
    assert!(is_agent_mode_button_text("🤖 Sisyphus (Ultraworker) Mode"));
    assert!(!is_model_button_text("🤖 Sisyphus (Ultraworker) Agent"));
}

#[test]
fn slice28b_21_pattern_variant_current_and_legacy() {
    assert!(is_variant_button_text("💡 Default"));
    assert!(is_variant_button_text("💭 Default"));
    assert!(!is_variant_button_text("Create a migration plan"));
}

#[test]
fn slice28b_22_pattern_queued() {
    assert!(is_queued_prompt_button_text(
        &format_queued_prompt_button_label(1, "Fix the bug")
    ));
    assert!(is_queued_prompt_button_text(
        &format_queued_prompt_button_label(12, "Fix the bug")
    ));
    assert!(!is_queued_prompt_button_text("❌ do not do that"));
    assert!(!is_queued_prompt_button_text("💡 Default"));
}

#[test]
fn slice28b_23_pattern_plain_prompt_matches_nothing() {
    assert!(!is_model_button_text("Create a migration plan"));
    assert!(!is_agent_mode_button_text("Create a migration plan"));
    assert!(!is_variant_button_text("Create a migration plan"));
    assert!(!is_queued_prompt_button_text("Create a migration plan"));
    assert!(!is_context_button_text("Create a migration plan"));
}

#[test]
fn slice28b_24_pattern_reply_keyboard_recognises_all() {
    assert!(is_reply_keyboard_button_text("🛠️ Build Agent"));
    assert!(is_reply_keyboard_button_text("📊 0"));
    assert!(is_reply_keyboard_button_text("🧠 openrouter\nopenai/gpt-4o"));
    assert!(is_reply_keyboard_button_text("💡 Default"));
    assert!(is_reply_keyboard_button_text(
        &format_queued_prompt_button_label(1, "queued")
    ));
    assert!(!is_reply_keyboard_button_text("Create a migration plan"));
}

// --- model-selection-menu.ts (7 its) ---

#[test]
fn slice28b_25_model_list_callback() {
    assert_eq!(
        build_model_list_callback(ModelListKind::Favorites, 0),
        "model:list:favorites:0"
    );
    assert_eq!(
        build_model_list_callback(ModelListKind::Recent, 2),
        "model:list:recent:2"
    );
}

#[test]
fn slice28b_26_parse_providers_page() {
    assert_eq!(parse_providers_page_callback("model:providers:3"), Some(3));
    assert_eq!(parse_providers_page_callback("model:providers:x"), None);
    assert_eq!(parse_providers_page_callback("model:root"), None);
}

#[test]
fn slice28b_27_parse_provider_callback() {
    assert_eq!(
        parse_provider_callback("model:provider:2:1"),
        Some(ProviderCallback { provider_index: 2, page: 1 })
    );
    assert_eq!(parse_provider_callback("model:provider:2"), None);
    assert_eq!(parse_provider_callback("model:provider:a:b"), None);
}

#[test]
fn slice28b_28_parse_provider_model_callback() {
    assert_eq!(parse_provider_model_callback("model:pick:4"), Some(4));
    assert_eq!(parse_provider_model_callback("model:pick:x"), None);
    assert_eq!(parse_provider_model_callback("model:root"), None);
}

#[test]
fn slice28b_29_pagination_range_clamps() {
    let r = calculate_models_pagination_range(100, 0, 10);
    assert_eq!(
        r,
        PaginationRange { page: 0, total_pages: 10, start_index: 0, end_index: 10 }
    );
    let clamped = calculate_models_pagination_range(100, 99, 10);
    assert_eq!(clamped.page, 9);
    assert_eq!((clamped.start_index, clamped.end_index), (90, 100));
}

#[test]
fn slice28b_30_model_option_labels() {
    assert_eq!(
        model_option_label("openrouter", "openai/gpt-4o", "⭐", false),
        "⭐ openrouter/openai/gpt-4o"
    );
    assert_eq!(
        model_option_label("openrouter", "openai/gpt-4o", "🕘", true),
        "✅ 🕘 openrouter/openai/gpt-4o"
    );
    assert_eq!(provider_option_label("OpenAI", 12), "OpenAI (12)");
}

#[test]
fn slice28b_31_model_menu_empty_gate() {
    assert!(!model_menu_has_choices(0, 0));
    assert!(model_menu_has_choices(1, 0));
    assert!(model_menu_has_choices(0, 2));
}

// --- agent-selection-menu.ts (2 its) ---

#[test]
fn slice28b_32_agent_option_label() {
    assert_eq!(agent_option_label("🛠️ Build", false), "🛠️ Build");
    assert_eq!(agent_option_label("🛠️ Build", true), "✅ 🛠️ Build");
}

#[test]
fn slice28b_33_agent_option_callback() {
    assert_eq!(agent_option_callback("build"), "agent:build");
}

// --- variant-selection-menu.ts (4 its) ---

#[test]
fn slice28b_34_variant_display_and_callback() {
    assert_eq!(format_variant_for_display("default"), "Default");
    assert_eq!(format_variant_for_display("high"), "High");
    assert_eq!(variant_option_callback("high"), "variant:high");
}

#[test]
fn slice28b_35_variant_option_label() {
    assert_eq!(variant_option_label("default", true), "✅ Default");
    assert_eq!(variant_option_label("high", false), "High");
}

#[test]
fn slice28b_36_variant_visible_ids_skip_disabled() {
    let variants = vec![
        VariantEntry { id: "default".to_string(), disabled: false },
        VariantEntry { id: "high".to_string(), disabled: false },
        VariantEntry { id: "low".to_string(), disabled: true },
    ];
    assert_eq!(visible_variant_ids(&variants), vec!["default", "high"]);
}

#[test]
fn slice28b_37_variant_after_change_gate() {
    let only_default = vec![VariantEntry { id: "default".to_string(), disabled: false }];
    assert!(!should_show_variant_menu_after_change(&only_default));
    let all_disabled = vec![
        VariantEntry { id: "default".to_string(), disabled: false },
        VariantEntry { id: "high".to_string(), disabled: true },
    ];
    assert!(!should_show_variant_menu_after_change(&all_disabled));
    let choice = vec![
        VariantEntry { id: "default".to_string(), disabled: false },
        VariantEntry { id: "high".to_string(), disabled: false },
    ];
    assert!(should_show_variant_menu_after_change(&choice));
}

// --- permission-menu.ts (5 its) ---

#[test]
fn slice28b_38_permission_emoji_and_name() {
    assert_eq!(permission_emoji("bash"), "⚡");
    assert_eq!(permission_emoji("unknown-kind"), "🔐");
    assert_eq!(permission_display_name("bash"), "Bash");
    assert_eq!(permission_display_name("mystery"), "mystery");
}

#[test]
fn slice28b_39_permission_outcome_lines() {
    assert_eq!(
        format_permission_outcome_line(PermissionReply::Once, false),
        "✅ Allowed once"
    );
    assert_eq!(
        format_permission_outcome_line(PermissionReply::Once, true),
        "✅ Allowed once · answered outside Telegram"
    );
    assert_eq!(
        format_permission_outcome_line(PermissionReply::Reject, false),
        "❌ Rejected"
    );
}

#[test]
fn slice28b_40_permission_text_with_patterns() {
    let text = format_permission_text(
        "⚡",
        "Bash",
        &["npm test".to_string()],
        1,
        Some("⏹ Not answered"),
    );
    assert!(text.contains("npm test"));
    assert!(text.ends_with("\n⏹ Not answered"));
    assert!(text.len() <= PERMISSION_MESSAGE_LIMIT);
}

#[test]
fn slice28b_41_permission_text_budget_cut() {
    let long = format!("echo {}", "x".repeat(5000));
    let text = format_permission_text("⚡", "Bash", &[long], 1, Some("⏹ Not answered"));
    assert!(text.len() <= PERMISSION_MESSAGE_LIMIT);
    assert!(text.contains("…\n"));
    assert!(text.ends_with("\n⏹ Not answered"));
}

#[test]
fn slice28b_42_permission_keyboard_layout() {
    let kb = build_permission_keyboard();
    assert_eq!(kb.len(), 3);
    assert_eq!(kb[0][0].callback_data, "permission:once");
    assert_eq!(kb[1][0].callback_data, "permission:always");
    assert_eq!(kb[2][0].callback_data, "permission:reject");
    assert_eq!(slice_on_safe_boundary("abcdef", 4), "abcd");
}

// --- question-menu.ts (3 its) ---

#[test]
fn slice28b_43_question_button_text_and_rows() {
    assert_eq!(format_question_button_text("Yes", true), "✅ Yes");
    let long = "l".repeat(100);
    let cut = format_question_button_text(&long, false);
    assert_eq!(cut.len(), MAX_QUESTION_BUTTON_LENGTH);
    assert!(cut.ends_with("..."));
    let spec = QuestionKeyboardSpec {
        question_index: 0,
        option_count: 2,
        multiple: false,
        custom: None,
        selected: vec![0],
        has_custom_answer: false,
        custom_answer_selected: false,
        custom_answer_text: String::new(),
    };
    let rows = build_question_keyboard_rows(&spec);
    assert_eq!(rows[0][0].callback_data, "question:select:0:0");
    assert!(rows[0][0].text.starts_with("✅ "));
}

#[test]
fn slice28b_44_question_submit_custom_gating() {
    let multi = QuestionKeyboardSpec {
        question_index: 0,
        option_count: 2,
        multiple: true,
        custom: None,
        selected: vec![],
        has_custom_answer: false,
        custom_answer_selected: false,
        custom_answer_text: String::new(),
    };
    let rows = build_question_keyboard_rows(&multi);
    assert!(rows.iter().any(|r| r.iter().any(|b| b.callback_data == "question:submit:0")));
    let single = QuestionKeyboardSpec { multiple: false, ..multi.clone() };
    let single_rows = build_question_keyboard_rows(&single);
    assert!(!single_rows.iter().any(|r| r.iter().any(|b| b.callback_data == "question:submit:0")));
    assert!(single_rows.iter().any(|r| r.iter().any(|b| b.callback_data == "question:custom:0")));
    assert!(single_rows.iter().any(|r| r.iter().any(|b| b.callback_data == "question:cancel:0")));
}

#[test]
fn slice28b_45_question_summary_and_parse() {
    let summary = format_answers_summary(&[("Q1".to_string(), "Yes".to_string())]);
    assert!(summary.contains("Q1"));
    assert!(summary.contains("Yes"));
    assert_eq!(parse_question_select_callback("question:select:1:2"), Some((1, 2)));
    assert_eq!(parse_question_select_callback("question:cancel:0"), None);
    assert!(is_question_request_gone_tag("FormAlreadySettledError"));
    assert!(!is_question_request_gone_tag("OtherError"));
}

// --- render remainder (5 its) ---

#[test]
fn slice28b_46_render_inline_plain_text() {
    let nodes = vec![
        PlainInlineNode::Text("hi ".to_string()),
        PlainInlineNode::Styled(vec![PlainInlineNode::Text("there".to_string())]),
        PlainInlineNode::Code("x".to_string()),
    ];
    assert_eq!(extract_inline_plain_text(&nodes), "hi therex");
}

#[test]
fn slice28b_47_render_aligned_table() {
    let text = build_aligned_table_text(&[
        vec!["a".to_string(), "bb".to_string()],
        vec!["ccc".to_string(), "d".to_string()],
    ]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[1].contains('-'));
    assert_eq!(lines[0].len(), lines[2].len());
}

#[test]
fn slice28b_48_render_split_chunks() {
    assert_eq!(split_text_into_chunks("", 10), Vec::<String>::new());
    assert_eq!(split_text_into_chunks("short", 10), vec!["short"]);
    let chunks = split_text_into_chunks("aaa bbb ccc", 5);
    assert!(chunks.iter().all(|c| c.len() <= 5));
    assert_eq!(chunks.concat().replace(' ', " ").len() > 0, true);
}

#[test]
fn slice28b_49_render_split_limits_normalize() {
    assert_eq!(
        normalize_block_split_limits(None, None),
        BlockSplitLimits { max_chars: DEFAULT_MAX_PART_CHARS, max_blocks: DEFAULT_MAX_PART_BLOCKS }
    );
    assert_eq!(
        normalize_block_split_limits(Some(0), Some(-5)),
        BlockSplitLimits { max_chars: 1, max_blocks: 1 }
    );
}

#[test]
fn slice28b_50_render_fallback_and_limits() {
    let text = format_question_fallback_text(
        "Q1",
        "Pick one",
        &[("Yes".to_string(), "accept".to_string())],
        false,
        Some((0, 1)),
        None,
    );
    assert!(text.contains("Q1"));
    assert!(text.contains("Pick one"));
    assert_eq!(TELEGRAM_TEXT_MESSAGE_LIMIT, 4096);
    assert_eq!(PLAIN_MAX_PART_CHARS, 3_800);
}
