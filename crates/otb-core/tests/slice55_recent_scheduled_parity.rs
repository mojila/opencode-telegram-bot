//! Parity port of the recent-selection + scheduled-task menu pure gates
//! (slice 55, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/bot/menus/recent-selection-menu.ts` (`recent_status_glyph`,
//!   `shorten_project`, `project_labels`, `project_names`,
//!   `build_recent_button_label`, `build_recent_callback`)
//! - `src/bot/callbacks/recent-callback-handler.ts`
//!   (`parse_recent_index_callback`: `/^\d+$/` digits-only index)
//! - `src/bot/menus/scheduled-task-menu.ts` (`truncate_task_text`,
//!   `normalize_task_prompt`, `task_state_prefix`,
//!   `format_task_button_label`, `build_tasklist_open_callback`,
//!   `build_tasklist_delete_callback`, `build_tasklist_toggle_callback`)
//! - `src/bot/callbacks/scheduled-task-callback-handler.ts`
//!   (`parse_tasklist_open_callback`, `parse_tasklist_delete_callback`,
//!   `parse_tasklist_toggle_callback`, `is_tasklist_cancel_callback`)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::recent_scheduled_gates::*;

// --- status glyphs ---

#[test]
fn running_status_glyph() {
    assert_eq!(recent_status_glyph("running"), Some("⏳"));
}

#[test]
fn idle_status_glyph() {
    assert_eq!(recent_status_glyph("idle"), Some("○"));
}

#[test]
fn question_status_glyph() {
    assert_eq!(recent_status_glyph("question"), Some("❓"));
}

#[test]
fn permission_status_glyph() {
    assert_eq!(recent_status_glyph("permission"), Some("🔐"));
}

#[test]
fn unknown_status_has_no_glyph() {
    assert_eq!(recent_status_glyph("busy"), None);
    assert_eq!(recent_status_glyph(""), None);
}

// --- shorten_project ---

#[test]
fn short_name_passes_through() {
    assert_eq!(shorten_project("my-project"), "my-project");
}

#[test]
fn single_long_segment_cuts_to_40_with_ellipsis() {
    let shortened = shorten_project(&"a".repeat(41));
    assert_eq!(shortened, format!("{}…{}", "a".repeat(20), "a".repeat(19)));
    assert_eq!(shortened.chars().count(), MAX_SHORT_PROJECT_LENGTH);
}

#[test]
fn two_segment_name_keeps_first_and_last() {
    let shortened = shorten_project(&format!("{}/{}", "A".repeat(30), "B".repeat(30)));
    assert_eq!(
        shortened,
        format!("{}…{}/{}…{}", "A".repeat(5), "A".repeat(18), "B".repeat(7), "B".repeat(7))
    );
}

#[test]
fn deep_name_keeps_first_and_last_with_gap() {
    let shortened = shorten_project(&format!("aaa/bbb/{}", "C".repeat(40)));
    assert_eq!(
        shortened,
        format!("aaa/…/{}…{}", "C".repeat(6), "C".repeat(6))
    );
}

// --- project_names ---

#[test]
fn distinct_leaves_use_folder_names() {
    assert_eq!(
        project_names(&["/x/foo".to_string(), "/y/bar".to_string()]),
        vec!["foo".to_string(), "bar".to_string()]
    );
}

#[test]
fn shared_leaf_uses_two_segments() {
    assert_eq!(
        project_names(&["/a/foo".to_string(), "/b/foo".to_string()]),
        vec!["a/foo".to_string(), "b/foo".to_string()]
    );
}

#[test]
fn deeper_tie_uses_three_segments() {
    assert_eq!(
        project_names(&["/x/a/foo".to_string(), "/y/a/foo".to_string()]),
        vec!["x/a/foo".to_string(), "y/a/foo".to_string()]
    );
}

// --- project_labels ---

#[test]
fn distinct_labels_pass_through() {
    let names = vec!["alpha".to_string(), "beta".to_string()];
    assert_eq!(project_labels(&names), names);
}

#[test]
fn identical_full_names_pass_through() {
    let names = vec!["same".to_string(), "same".to_string()];
    assert_eq!(project_labels(&names), names);
}

#[test]
fn shortened_collision_falls_back_to_indexed_excerpt() {
    let first = format!("{}X{}", "A".repeat(20), "B".repeat(20));
    let second = format!("{}Y{}", "A".repeat(20), "B".repeat(20));
    let labels = project_labels(&[first, second]);
    assert_eq!(
        labels,
        vec![
            format!("…{}X{}…/{}…{}", "A".repeat(8), "B".repeat(13), "A".repeat(6), "B".repeat(5)),
            format!("…{}Y{}…/{}…{}", "A".repeat(8), "B".repeat(13), "A".repeat(6), "B".repeat(5)),
        ]
    );
}

// --- recent callbacks ---

#[test]
fn build_recent_callback_uses_prefix() {
    assert_eq!(build_recent_callback(3), format!("{RECENT_CALLBACK_PREFIX}3"));
}

#[test]
fn parse_recent_index_reads_digits() {
    assert_eq!(parse_recent_index_callback("recent:12"), Some(12));
}

#[test]
fn parse_recent_index_reads_zero() {
    assert_eq!(parse_recent_index_callback("recent:0"), Some(0));
}

#[test]
fn parse_recent_index_rejects_alpha_payload() {
    assert_eq!(parse_recent_index_callback("recent:1a"), None);
}

#[test]
fn parse_recent_index_rejects_empty_payload() {
    assert_eq!(parse_recent_index_callback("recent:"), None);
}

#[test]
fn parse_recent_index_rejects_wrong_prefix() {
    assert_eq!(parse_recent_index_callback("worktree:1"), None);
}

// --- recent button labels ---

#[test]
fn short_recent_label_combines_glyph_project_title() {
    assert_eq!(
        build_recent_button_label("⏳", "proj", "Fix bug"),
        "⏳ [proj] Fix bug"
    );
}

#[test]
fn long_recent_title_cuts_to_64_with_ellipsis() {
    let label = build_recent_button_label("⏳", "p", &"t".repeat(70));
    assert_eq!(label.chars().count(), MAX_RECENT_BUTTON_LABEL_LENGTH);
    assert!(label.starts_with("⏳ [p] "));
    assert!(label.ends_with('…'));
    assert_eq!(label, format!("⏳ [p] {}…", "t".repeat(57)));
}

// --- truncate_task_text ---

#[test]
fn short_task_text_passes_through() {
    assert_eq!(truncate_task_text("hello", MAX_TASK_BUTTON_LABEL_LENGTH), "hello");
}

#[test]
fn long_task_text_cuts_with_trim_end_and_dots() {
    let text = format!("{}   b", "a".repeat(61));
    assert_eq!(truncate_task_text(&text, 64), format!("{}...", "a".repeat(61)));
}

#[test]
fn task_cut_trims_trailing_spaces_before_dots() {
    let text = format!("{}          b", "a".repeat(59));
    assert_eq!(truncate_task_text(&text, 64), format!("{}...", "a".repeat(59)));
}

// --- normalize_task_prompt ---

#[test]
fn prompt_whitespace_collapses_to_single_spaces() {
    assert_eq!(
        normalize_task_prompt("  fix   the\n\tbug  "),
        "fix the bug"
    );
}

// --- task_state_prefix ---

#[test]
fn disabled_cron_reads_sleeping() {
    assert_eq!(task_state_prefix(true, false), "💤 ");
}

#[test]
fn enabled_cron_reads_alarm() {
    assert_eq!(task_state_prefix(true, true), "⏰ ");
}

#[test]
fn once_task_reads_alarm() {
    assert_eq!(task_state_prefix(false, false), "⏰ ");
}

// --- format_task_button_label ---

#[test]
fn short_task_label_combines_state_badge_prompt() {
    assert_eq!(
        format_task_button_label(true, true, "daily 09:00", "hello"),
        "⏰ [daily 09:00] hello"
    );
}

#[test]
fn disabled_task_label_uses_sleeping_prefix() {
    assert_eq!(
        format_task_button_label(true, false, "daily 09:00", "hello"),
        "💤 [daily 09:00] hello"
    );
}

#[test]
fn long_task_label_cuts_to_64_with_dots() {
    let label = format_task_button_label(true, true, "daily 09:00", &"x".repeat(70));
    assert_eq!(label.chars().count(), MAX_TASK_BUTTON_LABEL_LENGTH);
    assert!(label.ends_with("..."));
    assert_eq!(label, format!("⏰ [daily 09:00] {}...", "x".repeat(45)));
}

#[test]
fn task_label_normalizes_prompt_whitespace() {
    assert_eq!(
        format_task_button_label(false, true, "once", "  a   b  "),
        "⏰ [once] a b"
    );
}

// --- tasklist callback builds ---

#[test]
fn build_open_callback_appends_id() {
    assert_eq!(
        build_tasklist_open_callback("abc"),
        format!("{TASKLIST_OPEN_PREFIX}abc")
    );
}

#[test]
fn build_delete_callback_appends_id() {
    assert_eq!(
        build_tasklist_delete_callback("abc"),
        format!("{TASKLIST_DELETE_PREFIX}abc")
    );
}

#[test]
fn build_toggle_on_callback() {
    assert_eq!(
        build_tasklist_toggle_callback(true, "abc"),
        format!("{TASKLIST_TOGGLE_PREFIX}on:abc")
    );
}

#[test]
fn build_toggle_off_callback() {
    assert_eq!(
        build_tasklist_toggle_callback(false, "abc"),
        format!("{TASKLIST_TOGGLE_PREFIX}off:abc")
    );
}

// --- tasklist callback parses ---

#[test]
fn parse_open_callback_reads_id() {
    assert_eq!(
        parse_tasklist_open_callback("tasklist:open:abc"),
        Some("abc".to_string())
    );
}

#[test]
fn parse_open_callback_keeps_empty_id_for_lookup() {
    assert_eq!(
        parse_tasklist_open_callback("tasklist:open:"),
        Some(String::new())
    );
}

#[test]
fn parse_open_callback_rejects_wrong_prefix() {
    assert_eq!(parse_tasklist_open_callback("tasklist:delete:abc"), None);
}

#[test]
fn parse_delete_callback_reads_id() {
    assert_eq!(
        parse_tasklist_delete_callback("tasklist:delete:abc"),
        Some("abc".to_string())
    );
}

#[test]
fn parse_delete_callback_rejects_wrong_prefix() {
    assert_eq!(parse_tasklist_delete_callback("tasklist:open:abc"), None);
}

#[test]
fn parse_toggle_on_callback() {
    assert_eq!(
        parse_tasklist_toggle_callback("tasklist:toggle:on:abc"),
        Some(TasklistToggle { action: ToggleAction::On, id: "abc".to_string() })
    );
}

#[test]
fn parse_toggle_off_callback() {
    assert_eq!(
        parse_tasklist_toggle_callback("tasklist:toggle:off:abc"),
        Some(TasklistToggle { action: ToggleAction::Off, id: "abc".to_string() })
    );
}

#[test]
fn parse_toggle_rejects_bad_action() {
    assert_eq!(parse_tasklist_toggle_callback("tasklist:toggle:maybe:abc"), None);
}

#[test]
fn parse_toggle_rejects_empty_id() {
    assert_eq!(parse_tasklist_toggle_callback("tasklist:toggle:on:"), None);
}

#[test]
fn parse_toggle_ignores_extra_segments() {
    assert_eq!(
        parse_tasklist_toggle_callback("tasklist:toggle:on:abc:def"),
        Some(TasklistToggle { action: ToggleAction::On, id: "abc".to_string() })
    );
}

#[test]
fn cancel_callback_matches_exactly() {
    assert!(is_tasklist_cancel_callback("tasklist:cancel"));
}

#[test]
fn cancel_callback_rejects_extended_data() {
    assert!(!is_tasklist_cancel_callback("tasklist:cancel:x"));
    assert!(!is_tasklist_cancel_callback("task:cancel"));
}
