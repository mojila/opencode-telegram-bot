//! Slice 76 parity: duration / assistant-run-footer / task-display gates (RED).
//!
//! Mirrors the pure TS contract in:
//! - `src/app/formatters/duration-formatter.ts` (`TOOL_ELAPSED_THRESHOLD_MS`,
//!   `DURATION_ICON`, `RUNNING_ICON`, `appendDuration`,
//!   `appendDurationToFirstLine`, `formatDuration`, `formatDurationOverHours`)
//! - `src/app/formatters/assistant-run-footer-formatter.ts`
//!   (`assistantRunTokensPerSecond`, `formatAssistantRunFooter`)
//! - `src/app/formatters/scheduled-task-display-formatter.ts`, cron-badge
//!   half (`parseCronParts`, `parseExactNumber`, `parseEveryStep`,
//!   `normalizeWeekdayValue`, `parseWeekdaySet`, `hasExactValues`,
//!   `padNumber`, `formatTime`, `WEEKDAY_LABELS`, `formatCronTaskBadge`)
//!
//! `bucketElapsedMs` is owned by `streaming.rs` and the timezone-dependent
//! once-badge stays with the caller; neither is restated here.
//! Every test MUST FAIL until `duration_footer_display_gates` is ported.

use otb_core::duration_footer_display_gates::*;
use std::collections::BTreeSet;

// --- constants ---

#[test]
fn slice76_tool_elapsed_threshold_is_20s_parity() {
    assert_eq!(TOOL_ELAPSED_THRESHOLD_MS, 20_000);
}

#[test]
fn slice76_duration_icons_parity() {
    assert_eq!(DURATION_ICON, "🕒");
    assert_eq!(RUNNING_ICON, "⏳");
    assert_eq!(SPEED_ICON, "⚡");
    assert_eq!(INTERRUPTED_ICON, "⏹");
}

// --- append_duration ---

#[test]
fn slice76_append_duration_parity() {
    assert_eq!(append_duration("hello", "5s"), "hello · 🕒 5s");
}

#[test]
fn slice76_append_first_line_single_parity() {
    assert_eq!(append_duration_to_first_line("hello", "5s"), "hello · 🕒 5s");
}

#[test]
fn slice76_append_first_line_multi_parity() {
    assert_eq!(
        append_duration_to_first_line("first\nsecond", "5s"),
        "first · 🕒 5s\nsecond"
    );
}

// --- format_duration ---

#[test]
fn slice76_format_duration_zero_parity() {
    assert_eq!(format_duration(0), "0s");
}

#[test]
fn slice76_format_duration_negative_clamped_parity() {
    assert_eq!(format_duration(-5000), "0s");
}

#[test]
fn slice76_format_duration_seconds_parity() {
    assert_eq!(format_duration(5000), "5s");
}

#[test]
fn slice76_format_duration_minute_exact_parity() {
    assert_eq!(format_duration(60_000), "1m");
}

#[test]
fn slice76_format_duration_minute_seconds_parity() {
    assert_eq!(format_duration(61_000), "1m 1s");
}

#[test]
fn slice76_format_duration_hour_exact_parity() {
    assert_eq!(format_duration(3_600_000), "1h");
}

#[test]
fn slice76_format_duration_full_parity() {
    assert_eq!(format_duration(3_661_000), "1h 1m 1s");
}

#[test]
fn slice76_format_duration_over_hours_parity() {
    assert_eq!(format_duration_over_hours(24), ">24h");
}

// --- assistant_run_tokens_per_second ---

fn slice76_step(created: i64, streamed: Option<i64>, output: i64, reasoning: i64) -> AssistantRunStep {
    AssistantRunStep {
        message_id: "m1".to_string(),
        created,
        streamed,
        output,
        reasoning,
    }
}

#[test]
fn slice76_tokens_empty_none_parity() {
    assert_eq!(assistant_run_tokens_per_second(&[]), None);
}

#[test]
fn slice76_tokens_missing_streamed_none_parity() {
    assert_eq!(
        assistant_run_tokens_per_second(&[slice76_step(0, None, 10, 5)]),
        None
    );
}

#[test]
fn slice76_tokens_zero_tokens_none_parity() {
    assert_eq!(
        assistant_run_tokens_per_second(&[slice76_step(0, Some(1000), 0, 0)]),
        None
    );
}

#[test]
fn slice76_tokens_zero_span_none_parity() {
    assert_eq!(
        assistant_run_tokens_per_second(&[slice76_step(1000, Some(1000), 10, 5)]),
        None
    );
}

#[test]
fn slice76_tokens_single_step_parity() {
    assert_eq!(
        assistant_run_tokens_per_second(&[slice76_step(0, Some(1000), 8, 2)]),
        Some(10.0)
    );
}

#[test]
fn slice76_tokens_negative_span_counts_zero_parity() {
    // created=1000, streamed=500 contributes max(0, -500) = 0; the live
    // step still carries the run: 1000ms span, 10 tokens -> 10 tok/s.
    let steps = vec![
        slice76_step(0, Some(1000), 10, 0),
        slice76_step(1000, Some(500), 0, 0),
    ];
    assert_eq!(assistant_run_tokens_per_second(&steps), Some(10.0));
}

// --- format_assistant_run_footer ---

#[test]
fn slice76_footer_basic_parity() {
    assert_eq!(
        format_assistant_run_footer("🛠️ Build", "anthropic", "claude", 61_000, None, false, "interrupted"),
        "🛠️ Build · 🧠 anthropic/claude · 🕒 1m 1s"
    );
}

#[test]
fn slice76_footer_with_speed_parity() {
    assert_eq!(
        format_assistant_run_footer("🛠️ Build", "anthropic", "claude", 61_000, Some(10.0), false, "interrupted"),
        "🛠️ Build · 🧠 anthropic/claude · 🕒 1m 1s · ⚡ 10.0 tok/s"
    );
}

#[test]
fn slice76_footer_speed_one_decimal_parity() {
    // TS `speed.toFixed(1)`.
    assert!(
        format_assistant_run_footer("🛠️ Build", "p", "m", 1000, Some(33.333), false, "x")
            .ends_with("⚡ 33.3 tok/s")
    );
}

#[test]
fn slice76_footer_interrupted_parity() {
    assert_eq!(
        format_assistant_run_footer("🛠️ Build", "p", "m", 1000, None, true, "interrupted"),
        "🛠️ Build · 🧠 p/m · 🕒 1s · ⏹ interrupted"
    );
}

// --- parse_exact_number ---

#[test]
fn slice76_exact_number_plain_parity() {
    assert_eq!(parse_exact_number("5", 0, 59), Some(5));
}

#[test]
fn slice76_exact_number_trims_and_pads_parity() {
    assert_eq!(parse_exact_number("  07 ", 0, 59), Some(7));
}

#[test]
fn slice76_exact_number_garbage_none_parity() {
    assert_eq!(parse_exact_number("5x", 0, 59), None);
    assert_eq!(parse_exact_number("", 0, 59), None);
}

#[test]
fn slice76_exact_number_range_none_parity() {
    assert_eq!(parse_exact_number("60", 0, 59), None);
    assert_eq!(parse_exact_number("-1", 0, 59), None);
}

// --- parse_every_step ---

#[test]
fn slice76_every_step_plain_parity() {
    assert_eq!(parse_every_step("*/15", 1, 59), Some(15));
}

#[test]
fn slice76_every_step_bare_none_parity() {
    assert_eq!(parse_every_step("5", 1, 59), None);
}

#[test]
fn slice76_every_step_zero_none_parity() {
    assert_eq!(parse_every_step("*/0", 1, 59), None);
    assert_eq!(parse_every_step("*/60", 1, 59), None);
}

// --- normalize_weekday_value ---

#[test]
fn slice76_weekday_alias_parity() {
    assert_eq!(normalize_weekday_value("mon"), Some(1));
    assert_eq!(normalize_weekday_value("sat"), Some(6));
}

#[test]
fn slice76_weekday_case_insensitive_parity() {
    assert_eq!(normalize_weekday_value("TUE"), Some(2));
}

#[test]
fn slice76_weekday_seven_normalizes_parity() {
    assert_eq!(normalize_weekday_value("7"), Some(0));
    assert_eq!(normalize_weekday_value("0"), Some(0));
}

#[test]
fn slice76_weekday_unknown_none_parity() {
    assert_eq!(normalize_weekday_value("funday"), None);
    assert_eq!(normalize_weekday_value("8"), None);
}

// --- parse_weekday_set ---

#[test]
fn slice76_weekday_set_list_parity() {
    assert_eq!(
        parse_weekday_set("1,2,3"),
        Some(BTreeSet::from([1, 2, 3]))
    );
}

#[test]
fn slice76_weekday_set_range_parity() {
    assert_eq!(
        parse_weekday_set("mon-fri"),
        Some(BTreeSet::from([1, 2, 3, 4, 5]))
    );
}

#[test]
fn slice76_weekday_set_step_none_parity() {
    assert_eq!(parse_weekday_set("1/2"), None);
}

#[test]
fn slice76_weekday_set_reversed_none_parity() {
    assert_eq!(parse_weekday_set("5-2"), None);
}

// --- has_exact_values ---

#[test]
fn slice76_has_exact_true_parity() {
    assert!(has_exact_values(&BTreeSet::from([1, 2, 3, 4, 5]), &[1, 2, 3, 4, 5]));
}

#[test]
fn slice76_has_exact_mismatch_parity() {
    assert!(!has_exact_values(&BTreeSet::from([1, 2]), &[1, 2, 3]));
    assert!(!has_exact_values(&BTreeSet::from([1, 2]), &[1, 3]));
}

// --- pad_number / format_time / weekday_label ---

#[test]
fn slice76_pad_and_time_parity() {
    assert_eq!(pad_number(5), "05");
    assert_eq!(pad_number(12), "12");
    assert_eq!(format_time(9, 5), "09:05");
    assert_eq!(format_time(0, 0), "00:00");
}

#[test]
fn slice76_weekday_label_parity() {
    assert_eq!(weekday_label(0), Some("Sun"));
    assert_eq!(weekday_label(6), Some("Sat"));
    assert_eq!(weekday_label(7), None);
}

// --- parse_cron_parts ---

#[test]
fn slice76_cron_parts_five_parity() {
    let parts = parse_cron_parts("0 9 * * mon").expect("five parts parse");
    assert_eq!(parts.minute, "0");
    assert_eq!(parts.hour, "9");
    assert_eq!(parts.day_of_month, "*");
    assert_eq!(parts.month, "*");
    assert_eq!(parts.day_of_week, "mon");
}

#[test]
fn slice76_cron_parts_bad_none_parity() {
    assert_eq!(parse_cron_parts("* * * *"), None);
    assert_eq!(parse_cron_parts("* * * * * *"), None);
}

// --- format_cron_task_badge ---

#[test]
fn slice76_badge_every_minutes_parity() {
    assert_eq!(format_cron_task_badge("*/15 * * * *"), "15m");
}

#[test]
fn slice76_badge_hourly_daily_parity() {
    assert_eq!(format_cron_task_badge("0 * * * *"), "hourly");
    assert_eq!(format_cron_task_badge("30 * * * *"), "hourly :30");
    assert_eq!(format_cron_task_badge("0 */2 * * *"), "2h");
    assert_eq!(format_cron_task_badge("30 */2 * * *"), "2h :30");
    assert_eq!(format_cron_task_badge("0 9 * * *"), "daily 09:00");
}

#[test]
fn slice76_badge_weekly_parity() {
    assert_eq!(format_cron_task_badge("0 9 * * 1-5"), "weekdays 09:00");
    assert_eq!(format_cron_task_badge("0 9 * * 0,6"), "weekends 09:00");
    assert_eq!(format_cron_task_badge("0 9 * * mon"), "Mon 09:00");
    assert_eq!(format_cron_task_badge("0 9 * * 7"), "Sun 09:00");
}

#[test]
fn slice76_badge_monthly_fallback_parity() {
    assert_eq!(format_cron_task_badge("0 9 15 * *"), "monthly 15 09:00");
    assert_eq!(format_cron_task_badge("* * * * *"), "cron");
    assert_eq!(format_cron_task_badge("not a cron"), "cron");
}
