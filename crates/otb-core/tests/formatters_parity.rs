//! Parity port (RED, slice20b) for batch formatters.
//!
//! TS sources mapped:
//! - tests/app/formatters/duration-formatter.test.ts
//! - tests/app/formatters/session-title-formatter.test.ts
//! - tests/app/formatters/compact-summary-formatter.test.ts
//! - tests/app/formatters/assistant-run-footer-formatter.test.ts
//! - tests/app/formatters/scheduled-task-display-formatter.test.ts
//! - tests/app/formatters/subagent-formatter.test.ts (starting-card subset)
//! - tests/bot/render/rich-blocks.test.ts (countRichBlocks/countRichChars subset)
//!
//! Every test MUST FAIL with `unimplemented!()` until the Rust port lands.

use otb_core::formatters::extra_formatters::*;

// ---- duration-formatter.test.ts ----

#[test]
fn slice20b_duration_first_line_single() {
    assert_eq!(append_duration_to_first_line("one", "21s"), "one · 🕒 21s");
}

#[test]
fn slice20b_duration_first_line_multi() {
    assert_eq!(
        append_duration_to_first_line("one\ntwo", "21s"),
        "one · 🕒 21s\ntwo"
    );
}

#[test]
fn slice20b_format_duration_seconds() {
    assert_eq!(format_duration(20_000), "20s");
    assert_eq!(format_duration(59_000), "59s");
}

#[test]
fn slice20b_format_duration_whole_minutes() {
    assert_eq!(format_duration(60_000), "1m");
    assert_eq!(format_duration(5 * 60_000), "5m");
}

#[test]
fn slice20b_format_duration_minutes_seconds() {
    assert_eq!(format_duration(63_000), "1m 3s");
}

#[test]
fn slice20b_format_duration_hours() {
    assert_eq!(format_duration(3_600_000), "1h");
    assert_eq!(format_duration(24 * 3_600_000), "24h");
}

#[test]
fn slice20b_format_duration_hours_minutes_seconds() {
    assert_eq!(format_duration(3_600_000 + 15 * 60_000), "1h 15m");
    assert_eq!(
        format_duration(3_600_000 + 2 * 60_000 + 3_000),
        "1h 2m 3s"
    );
}

#[test]
fn slice20b_format_duration_zero_and_negative() {
    assert_eq!(format_duration(0), "0s");
    assert_eq!(format_duration(-5), "0s");
}

#[test]
fn slice20b_format_duration_over_hours() {
    assert_eq!(format_duration_over_hours(24), ">24h");
}

#[test]
fn slice20b_bucket_below_minute() {
    assert_eq!(bucket_elapsed_ms(20_000), 20_000);
    assert_eq!(bucket_elapsed_ms(29_000), 20_000);
    assert_eq!(bucket_elapsed_ms(59_000), 50_000);
}

#[test]
fn slice20b_bucket_below_ten_minutes() {
    assert_eq!(bucket_elapsed_ms(60_000), 60_000);
    assert_eq!(bucket_elapsed_ms(9 * 60_000 + 59_000), 9 * 60_000);
}

#[test]
fn slice20b_bucket_below_hour() {
    assert_eq!(bucket_elapsed_ms(10 * 60_000), 10 * 60_000);
    assert_eq!(bucket_elapsed_ms(14 * 60_000), 10 * 60_000);
    assert_eq!(bucket_elapsed_ms(59 * 60_000), 55 * 60_000);
}

#[test]
fn slice20b_bucket_from_hour() {
    assert_eq!(bucket_elapsed_ms(3_600_000), 3_600_000);
    assert_eq!(
        bucket_elapsed_ms(3_600_000 + 20 * 60_000),
        3_600_000 + 15 * 60_000
    );
}

#[test]
fn slice20b_bucket_non_positive() {
    assert_eq!(bucket_elapsed_ms(0), 0);
    assert_eq!(bucket_elapsed_ms(-1), 0);
}

// ---- session-title-formatter.test.ts ----

#[test]
fn slice20b_session_title_empty_en() {
    assert_eq!(format_session_title("", "en"), "new session");
}

#[test]
fn slice20b_session_title_empty_ru() {
    assert_eq!(format_session_title("", "ru"), "новая сессия");
}

#[test]
fn slice20b_session_title_kept() {
    assert_eq!(
        format_session_title("Greeting message", "en"),
        "Greeting message"
    );
    assert_eq!(format_session_title("   ", "en"), "   ");
}

// ---- compact-summary-formatter.test.ts ----

#[test]
fn slice20b_count_bash() {
    assert_eq!(
        count_tool_call("bash", &[]),
        ToolCallCounts {
            commands: 1,
            edits: 0,
            reads: 0,
            tools: 0
        }
    );
}

#[test]
fn slice20b_count_edit_family() {
    for tool in ["edit", "write", "apply_patch"] {
        assert_eq!(
            count_tool_call(tool, &[]),
            ToolCallCounts {
                commands: 0,
                edits: 1,
                reads: 0,
                tools: 0
            },
            "tool={tool}"
        );
    }
}

#[test]
fn slice20b_count_read_and_other() {
    assert_eq!(
        count_tool_call("read", &[]),
        ToolCallCounts {
            commands: 0,
            edits: 0,
            reads: 1,
            tools: 0
        }
    );
    assert_eq!(
        count_tool_call("grep", &[]),
        ToolCallCounts {
            commands: 0,
            edits: 0,
            reads: 0,
            tools: 1
        }
    );
}

#[test]
fn slice20b_count_question_ignored() {
    assert_eq!(
        count_tool_call("question", &[]),
        ToolCallCounts {
            commands: 0,
            edits: 0,
            reads: 0,
            tools: 0
        }
    );
}

#[test]
fn slice20b_count_execute_nested() {
    // read(completed) + bash(error) count; grep(running) + garbage dropped.
    assert_eq!(
        count_tool_call(
            "execute",
            &[("read", "completed"), ("bash", "error"), ("grep", "running")]
        ),
        ToolCallCounts {
            commands: 0,
            edits: 0,
            reads: 1,
            tools: 1
        }
    );
}

#[test]
fn slice20b_count_execute_empty_is_tool() {
    assert_eq!(
        count_tool_call("execute", &[]),
        ToolCallCounts {
            commands: 0,
            edits: 0,
            reads: 0,
            tools: 1
        }
    );
}

#[test]
fn slice20b_compact_summary_fixed_order() {
    let counts = ToolCallCounts {
        commands: 3,
        edits: 1,
        reads: 4,
        tools: 2,
    };
    assert_eq!(
        format_compact_summary(&counts, 2, "en"),
        "✅ commands: 3, edits: 1, reads: 4, tools: 2, changed files: 2"
    );
}

#[test]
fn slice20b_compact_summary_skips_zero() {
    let counts = ToolCallCounts {
        commands: 0,
        edits: 0,
        reads: 6,
        tools: 0,
    };
    assert_eq!(
        format_compact_summary(&counts, 1, "en"),
        "✅ reads: 6, changed files: 1"
    );
}

#[test]
fn slice20b_compact_summary_empty() {
    assert_eq!(
        format_compact_summary(
            &ToolCallCounts {
                commands: 0,
                edits: 0,
                reads: 0,
                tools: 0
            },
            0,
            "en"
        ),
        ""
    );
}

#[test]
fn slice20b_compact_summary_ru() {
    let counts = ToolCallCounts {
        commands: 3,
        edits: 0,
        reads: 1,
        tools: 0,
    };
    assert_eq!(
        format_compact_summary(&counts, 0, "ru"),
        "✅ команды: 3, чтения: 1"
    );
}

// ---- assistant-run-footer-formatter.test.ts ----

fn footer_params(agent: &str, elapsed_ms: i64) -> AssistantRunFooterParams {
    AssistantRunFooterParams {
        agent: agent.to_string(),
        provider_id: "openai".to_string(),
        model_id: "gpt-5.4".to_string(),
        elapsed_ms,
        steps: vec![],
        interrupted: false,
        locale: "en".to_string(),
    }
}

#[test]
fn slice20b_footer_agent_model_elapsed() {
    let mut p = footer_params("plan", 57_932);
    p.model_id = "gpt-5.4".to_string();
    assert_eq!(
        format_assistant_run_footer(&p),
        "📋 Plan · 🧠 openai/gpt-5.4 · 🕒 57s"
    );
}

#[test]
fn slice20b_footer_sub_second_is_zero() {
    assert_eq!(
        format_assistant_run_footer(&AssistantRunFooterParams {
            agent: "build".to_string(),
            provider_id: "anthropic".to_string(),
            model_id: "claude-3".to_string(),
            elapsed_ms: 500,
            steps: vec![],
            interrupted: false,
            locale: "en".to_string(),
        }),
        "🛠️ Build · 🧠 anthropic/claude-3 · 🕒 0s"
    );
}

#[test]
fn slice20b_footer_minutes_seconds() {
    let mut p = footer_params("build", 1_425_000);
    p.provider_id = "anthropic".to_string();
    p.model_id = "claude-3".to_string();
    assert_eq!(
        format_assistant_run_footer(&p),
        "🛠️ Build · 🧠 anthropic/claude-3 · 🕒 23m 45s"
    );
}

#[test]
fn slice20b_footer_hours_minutes_seconds() {
    let mut p = footer_params("build", 5_025_000);
    p.provider_id = "anthropic".to_string();
    p.model_id = "claude-3".to_string();
    assert_eq!(
        format_assistant_run_footer(&p),
        "🛠️ Build · 🧠 anthropic/claude-3 · 🕒 1h 23m 45s"
    );
}

#[test]
fn slice20b_footer_negative_is_zero() {
    let mut p = footer_params("build", -1_000);
    p.provider_id = "anthropic".to_string();
    p.model_id = "claude-3".to_string();
    assert_eq!(
        format_assistant_run_footer(&p),
        "🛠️ Build · 🧠 anthropic/claude-3 · 🕒 0s"
    );
}

#[test]
fn slice20b_footer_speed_last() {
    let p = AssistantRunFooterParams {
        agent: "build".to_string(),
        provider_id: "openai".to_string(),
        model_id: "gpt-5".to_string(),
        elapsed_ms: 68_000,
        steps: vec![
            AssistantRunStep {
                created_ms: 0,
                streamed_ms: Some(500),
                output: 10,
                reasoning: 5,
            },
            AssistantRunStep {
                created_ms: 1_000,
                streamed_ms: Some(1_500),
                output: 20,
                reasoning: 0,
            },
        ],
        interrupted: false,
        locale: "ru".to_string(),
    };
    assert_eq!(
        format_assistant_run_footer(&p),
        "🛠️ Build · 🧠 openai/gpt-5 · 🕒 1m 8s · ⚡ 35.0 tok/s"
    );
}

#[test]
fn slice20b_tokens_per_second_hides_zero() {
    assert_eq!(
        assistant_run_tokens_per_second(&[AssistantRunStep {
            created_ms: 0,
            streamed_ms: Some(1_000),
            output: 0,
            reasoning: 0,
        }]),
        None
    );
    assert_eq!(
        assistant_run_tokens_per_second(&[AssistantRunStep {
            created_ms: 2_000,
            streamed_ms: Some(1_000),
            output: 10,
            reasoning: 0,
        }]),
        None
    );
}

// ---- scheduled-task-display-formatter.test.ts ----

#[test]
fn slice20b_badge_minute_interval() {
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "*/5 * * * *".to_string()
        }),
        "5m"
    );
}

#[test]
fn slice20b_badge_hour_offset() {
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "15 */2 * * *".to_string()
        }),
        "2h :15"
    );
}

#[test]
fn slice20b_badge_hourly() {
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "30 * * * *".to_string()
        }),
        "hourly :30"
    );
}

#[test]
fn slice20b_badge_daily() {
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "0 9 * * *".to_string()
        }),
        "daily 09:00"
    );
}

#[test]
fn slice20b_badge_weekdays_weekends() {
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "0 9 * * 1-5".to_string()
        }),
        "weekdays 09:00"
    );
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "0 11 * * 0,6".to_string()
        }),
        "weekends 11:00"
    );
}

#[test]
fn slice20b_badge_monthly_and_cron_fallback() {
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "30 12 15 * *".to_string()
        }),
        "monthly 15 12:30"
    );
    assert_eq!(
        format_task_list_badge(&ScheduledTaskBadge::Cron {
            cron: "0 9 * * 1,3,5".to_string()
        }),
        "cron"
    );
}

// ---- subagent-formatter.test.ts (starting-card subset) ----

fn starting_card(description: &str, status: &str) -> SubagentCard {
    SubagentCard {
        description: description.to_string(),
        agent: "explore".to_string(),
        provider_id: None,
        model_id: None,
        variant: None,
        status: status.to_string(),
        terminal_message: None,
        created_at_ms: 1_000_000,
        finished_at_ms: None,
    }
}

#[test]
fn slice20b_subagent_starting_with_task() {
    assert_eq!(
        render_subagent_card(&starting_card("task description", "running"), "en", 1_000_000),
        "🧩 Starting task: task description"
    );
}

#[test]
fn slice20b_subagent_starting_without_task() {
    assert_eq!(
        render_subagent_card(&starting_card("", "running"), "en", 1_000_000),
        "🧩 Starting task..."
    );
}

// ---- rich-blocks.test.ts (counts subset) ----

#[test]
fn slice20b_rich_blocks_leaf_is_one() {
    assert_eq!(count_rich_blocks(&RichBlock::Paragraph("x".to_string())), 1);
    assert_eq!(count_rich_blocks(&RichBlock::Divider), 1);
}

#[test]
fn slice20b_rich_chars_paragraph_bold() {
    // TS: paragraph ["ab", bold "cd"] -> 4 chars.
    assert_eq!(
        count_rich_chars(&RichBlock::Paragraph("abcd".to_string())),
        4
    );
}
