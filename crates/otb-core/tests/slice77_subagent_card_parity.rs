//! Slice 77 parity: subagent card pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/app/formatters/subagent-formatter.ts` (`formatModelDisplayName`,
//! `shouldPreferInputDetails`, the `task`-with-arguments check in
//! `formatToolStep`, `formatToolElapsed` threshold half, `formatEndedLine`,
//! `formatSubagentActivity`, `formatHeaderLines`, `renderSubagentCard`,
//! `renderSubagentCards`).
//!
//! `formatCompactToolInfo` / `formatRunningTaskLine` (`summary-formatter.ts`,
//! see `error_diff_display`), `bucketElapsedMs` (`streaming.rs`), the
//! duration primitives (`duration_footer_display_gates`) and every `t(...)`
//! lookup stay with the caller; labels arrive as explicit arguments using
//! the `en` values. Every test MUST FAIL with `unimplemented!()` panics
//! until `subagent_card_gates` is ported.

use otb_core::subagent_card_gates::*;

fn labels() -> SubagentLabels<'static> {
    SubagentLabels {
        completed: "Completed",
        interrupted: "Interrupted",
        failed: "Task failed",
        working: "Working...",
    }
}

fn templates() -> SubagentCardTemplates<'static> {
    SubagentCardTemplates {
        task_template: "Task: {task}",
        agent_template: "Agent: {agent}",
        model_template: "Model: {model}",
        starting_template: "Starting task: {task}",
        starting_untitled: "Starting task...",
        labels: labels(),
    }
}

fn card<'a>(description: &'a str, agent: &'a str, status: &'a str) -> SubagentCardInput<'a> {
    SubagentCardInput {
        description,
        agent,
        model_name: None,
        status,
        tool_step: None,
        tool_elapsed: None,
        launch_line: None,
        launch_elapsed: None,
        terminal_message: None,
        finished_at: None,
        created_at: 1000,
    }
}

// --- format_model_display_name ---

#[test]
fn slice77_model_display_name_basic_parity() {
    assert_eq!(
        format_model_display_name(Some("anthropic"), Some("claude"), None),
        Some("anthropic/claude".to_string())
    );
}

#[test]
fn slice77_model_display_name_variant_parity() {
    assert_eq!(
        format_model_display_name(Some("openai"), Some("gpt"), Some("codex")),
        Some("openai/gpt (codex)".to_string())
    );
}

#[test]
fn slice77_model_display_name_missing_provider_parity() {
    assert_eq!(format_model_display_name(None, Some("gpt"), None), None);
}

#[test]
fn slice77_model_display_name_missing_model_parity() {
    assert_eq!(format_model_display_name(Some("openai"), None, None), None);
}

#[test]
fn slice77_model_display_name_empty_provider_parity() {
    assert_eq!(
        format_model_display_name(Some(""), Some("gpt"), None),
        None
    );
}

#[test]
fn slice77_model_display_name_empty_variant_ignored_parity() {
    assert_eq!(
        format_model_display_name(Some("openai"), Some("gpt"), Some("")),
        Some("openai/gpt".to_string())
    );
}

// --- should_prefer_input_details ---

#[test]
fn slice77_input_details_read_path_parity() {
    assert!(should_prefer_input_details("read", &["path"]));
}

#[test]
fn slice77_input_details_edit_filepath_parity() {
    assert!(should_prefer_input_details("edit", &["filePath"]));
}

#[test]
fn slice77_input_details_write_without_file_parity() {
    assert!(!should_prefer_input_details("write", &["command"]));
}

#[test]
fn slice77_input_details_apply_patch_path_parity() {
    assert!(should_prefer_input_details("apply_patch", &["path"]));
}

#[test]
fn slice77_input_details_bash_command_parity() {
    assert!(should_prefer_input_details("bash", &["command", "description"]));
}

#[test]
fn slice77_input_details_bash_without_command_parity() {
    assert!(!should_prefer_input_details("bash", &["description"]));
}

#[test]
fn slice77_input_details_grep_pattern_parity() {
    assert!(should_prefer_input_details("grep", &["pattern"]));
}

#[test]
fn slice77_input_details_glob_without_pattern_parity() {
    assert!(!should_prefer_input_details("glob", &["description"]));
}

#[test]
fn slice77_input_details_custom_query_parity() {
    assert!(should_prefer_input_details("webfetch", &["query"]));
}

#[test]
fn slice77_input_details_custom_description_only_parity() {
    assert!(!should_prefer_input_details("question", &["description"]));
}

// --- is_task_with_arguments ---

#[test]
fn slice77_task_with_arguments_parity() {
    assert!(is_task_with_arguments(Some("task"), 2));
}

#[test]
fn slice77_task_without_arguments_parity() {
    assert!(!is_task_with_arguments(Some("task"), 0));
}

#[test]
fn slice77_non_task_with_keys_parity() {
    assert!(!is_task_with_arguments(Some("read"), 2));
}

#[test]
fn slice77_no_tool_with_keys_parity() {
    assert!(!is_task_with_arguments(None, 2));
}

// --- tool_elapsed_visible ---

#[test]
fn slice77_tool_elapsed_undefined_parity() {
    assert_eq!(tool_elapsed_visible(None, 60_000), None);
}

#[test]
fn slice77_tool_elapsed_below_threshold_parity() {
    assert_eq!(tool_elapsed_visible(Some(40_001), 60_000), None);
    assert_eq!(tool_elapsed_visible(Some(70_000), 60_000), None);
}

#[test]
fn slice77_tool_elapsed_exact_threshold_parity() {
    assert_eq!(tool_elapsed_visible(Some(40_000), 60_000), Some(20_000));
}

#[test]
fn slice77_tool_elapsed_above_threshold_parity() {
    assert_eq!(tool_elapsed_visible(Some(15_000), 60_000), Some(45_000));
}

// --- format_ended_line ---

#[test]
fn slice77_ended_line_open_parity() {
    assert_eq!(
        format_ended_line("✅ Completed", None, 1000),
        "✅ Completed"
    );
}

#[test]
fn slice77_ended_line_finished_parity() {
    assert_eq!(
        format_ended_line("✅ Completed", Some(6000), 1000),
        "✅ Completed · 🕒 5s"
    );
}

// --- format_subagent_activity ---

#[test]
fn slice77_activity_completed_finished_parity() {
    assert_eq!(
        format_subagent_activity(
            "completed",
            None,
            None,
            None,
            None,
            None,
            Some(6000),
            1000,
            &labels()
        ),
        "✅ Completed · 🕒 5s"
    );
}

#[test]
fn slice77_activity_completed_open_parity() {
    assert_eq!(
        format_subagent_activity(
            "completed",
            None,
            None,
            None,
            None,
            None,
            None,
            1000,
            &labels()
        ),
        "✅ Completed"
    );
}

#[test]
fn slice77_activity_interrupted_finished_parity() {
    assert_eq!(
        format_subagent_activity(
            "interrupted",
            None,
            None,
            None,
            None,
            None,
            Some(61_000),
            1000,
            &labels()
        ),
        "⏹ Interrupted · 🕒 1m"
    );
}

#[test]
fn slice77_activity_error_message_parity() {
    assert_eq!(
        format_subagent_activity(
            "error",
            Some("  boom  "),
            None,
            None,
            None,
            None,
            None,
            1000,
            &labels()
        ),
        "❌ boom"
    );
}

#[test]
fn slice77_activity_error_blank_message_parity() {
    assert_eq!(
        format_subagent_activity(
            "error",
            Some("   "),
            None,
            None,
            None,
            None,
            None,
            1000,
            &labels()
        ),
        "❌ Task failed"
    );
}

#[test]
fn slice77_activity_error_no_message_parity() {
    assert_eq!(
        format_subagent_activity(
            "error",
            None,
            None,
            None,
            None,
            None,
            None,
            1000,
            &labels()
        ),
        "❌ Task failed"
    );
}

#[test]
fn slice77_activity_tool_step_parity() {
    assert_eq!(
        format_subagent_activity(
            "running",
            None,
            Some("💻 deploy"),
            None,
            None,
            None,
            None,
            1000,
            &labels()
        ),
        "💻 deploy"
    );
}

#[test]
fn slice77_activity_tool_step_elapsed_parity() {
    assert_eq!(
        format_subagent_activity(
            "running",
            None,
            Some("💻 deploy"),
            Some("30s"),
            None,
            None,
            None,
            1000,
            &labels()
        ),
        "💻 deploy · 🕒 30s"
    );
}

#[test]
fn slice77_activity_launch_line_parity() {
    assert_eq!(
        format_subagent_activity(
            "running",
            None,
            None,
            None,
            Some("🤖 Running Task: backfill"),
            Some("25s"),
            None,
            1000,
            &labels()
        ),
        "🤖 Running Task: backfill · 🕒 25s"
    );
}

#[test]
fn slice77_activity_working_fallback_parity() {
    assert_eq!(
        format_subagent_activity(
            "running",
            None,
            None,
            None,
            None,
            None,
            None,
            1000,
            &labels()
        ),
        "⚙️ Working..."
    );
}

// --- format_header_lines ---

#[test]
fn slice77_header_lines_full_parity() {
    assert_eq!(
        format_header_lines("ship", "core", Some("openai/gpt"), &templates()),
        vec![
            "Task: ship".to_string(),
            "Agent: core".to_string(),
            "Model: openai/gpt".to_string(),
        ]
    );
}

#[test]
fn slice77_header_lines_no_description_parity() {
    assert_eq!(
        format_header_lines("", "core", Some("openai/gpt"), &templates()),
        vec!["Agent: core".to_string(), "Model: openai/gpt".to_string(),]
    );
}

#[test]
fn slice77_header_lines_no_agent_parity() {
    assert_eq!(
        format_header_lines("ship", "", Some("openai/gpt"), &templates()),
        vec!["Task: ship".to_string(), "Model: openai/gpt".to_string(),]
    );
}

#[test]
fn slice77_header_lines_no_model_parity() {
    assert_eq!(
        format_header_lines("ship", "core", None, &templates()),
        vec!["Task: ship".to_string(), "Agent: core".to_string(),]
    );
}

#[test]
fn slice77_header_lines_empty_parity() {
    assert!(format_header_lines("", "", None, &templates()).is_empty());
}

// --- has_ended_status ---

#[test]
fn slice77_ended_statuses_parity() {
    assert!(has_ended_status("completed"));
    assert!(has_ended_status("error"));
    assert!(has_ended_status("interrupted"));
}

#[test]
fn slice77_live_statuses_not_ended_parity() {
    assert!(!has_ended_status("running"));
    assert!(!has_ended_status("pending"));
}

// --- render_subagent_card ---

#[test]
fn slice77_render_starting_parity() {
    assert_eq!(
        render_subagent_card(&card("ship", "", "running"), &templates()),
        "🧩 Starting task: ship"
    );
}

#[test]
fn slice77_render_starting_untitled_parity() {
    assert_eq!(
        render_subagent_card(&card("", "", "running"), &templates()),
        "🧩 Starting task..."
    );
}

#[test]
fn slice77_render_ended_partial_headers_parity() {
    let mut incomplete = card("ship", "", "completed");
    incomplete.finished_at = Some(6000);
    assert_eq!(
        render_subagent_card(&incomplete, &templates()),
        "🧩 Task: ship\n\n✅ Completed · 🕒 5s"
    );
}

#[test]
fn slice77_render_complete_running_parity() {
    let mut full = card("ship", "core", "running");
    full.model_name = Some("openai/gpt");
    full.tool_step = Some("💻 deploy");
    assert_eq!(
        render_subagent_card(&full, &templates()),
        "🧩 Task: ship\nAgent: core\nModel: openai/gpt\n\n💻 deploy"
    );
}

#[test]
fn slice77_render_ended_no_headers_parity() {
    let mut failed = card("", "", "error");
    failed.terminal_message = Some("boom");
    assert_eq!(
        render_subagent_card(&failed, &templates()),
        "🧩 ❌ boom"
    );
}

// --- join_rendered_cards ---

#[test]
fn slice77_join_empty_parity() {
    assert_eq!(join_rendered_cards(&[]), "");
}

#[test]
fn slice77_join_filters_and_joins_parity() {
    assert_eq!(
        join_rendered_cards(&["🧩 a", "", "🧩 b"]),
        "🧩 a\n\n🧩 b"
    );
}
