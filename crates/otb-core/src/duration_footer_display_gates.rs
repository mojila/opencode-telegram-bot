//! Duration / assistant-run-footer / task-display pure gates (slice 76, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/formatters/duration-formatter.ts` (`TOOL_ELAPSED_THRESHOLD_MS`,
//!   `DURATION_ICON`, `RUNNING_ICON`, `appendDuration`,
//!   `appendDurationToFirstLine`, `formatDuration`, `formatDurationOverHours`)
//! - `src/app/formatters/assistant-run-footer-formatter.ts`
//!   (`assistantRunTokensPerSecond`, `formatAssistantRunFooter`)
//! - `src/app/formatters/scheduled-task-display-formatter.ts` (cron-badge
//!   half: `parseCronParts`, `parseExactNumber`, `parseEveryStep`,
//!   `normalizeWeekdayValue`, `parseWeekdaySet`, `hasExactValues`,
//!   `padNumber`, `formatTime`, `WEEKDAY_LABELS`, `formatCronTaskBadge`)
//!
//! Out of scope (owned elsewhere or impure, callers keep them):
//! - `bucketElapsedMs` lives in `streaming.rs` (`bucket_elapsed_ms`, behind
//!   the running-tool-tracker rules); it is not restated here.
//! - `formatOnceTaskBadge` / `formatTaskListBadge` need `Intl.DateTimeFormat`
//!   timezone resolution and `Date.now()`; the once branch stays with the
//!   caller. This module owns the cron-badge half only.
//! - `getAgentDisplayName` (`src/app/types/agent.ts`) and the `t(...)`
//!   lookups (`assistant.footer.interrupted`, `subagent.*`) are i18n/store
//!   concerns: `format_assistant_run_footer` takes the resolved
//!   `agent_display` and `interrupted_label` as explicit arguments.
//! - `renderSubagentCard(s)` (`subagent-formatter.ts`) awaits the same
//!   `t(...)` dictionary and `formatCompactToolInfo`; only the shared
//!   duration primitives it reuses are covered here.
//!
//! Deviations (behavior-preserving for the covered cases):
//! 1. TS `formatDuration` rounds any float via `Math.round`; the port takes
//!    an `i64` millis value (parity fixtures use integer inputs, where the
//!    two coincide) and clamps negatives to `0` like `Math.max(0, ...)`.
//! 2. TS `speed.toFixed(1)` is rendered with Rust `{:.1}`. The two agree on
//!    the parity fixtures (`10.0`, `33.3`); banker's-vs-half-up rounding at
//!    an exact `.x5` boundary may differ by 0.1 in untested cases.
//! 3. TS `String.split("-")` on a weekday range with extra dashes keeps only
//!    the first two segments; the port mirrors that (takes segments 0/1).

use std::collections::BTreeSet;

/// Tool calls shorter than this render without elapsed time
/// (TS: `TOOL_ELAPSED_THRESHOLD_MS = 20 * SECOND_MS`).
pub const TOOL_ELAPSED_THRESHOLD_MS: i64 = 20_000;

/// Prefix every rendered duration shares with the assistant run footer
/// (TS: `DURATION_ICON`).
pub const DURATION_ICON: &str = "🕒";

/// Marks a step that is still running (TS: `RUNNING_ICON`).
pub const RUNNING_ICON: &str = "⏳";

/// Prefix for the tokens-per-second part of the run footer (TS: `SPEED_ICON`).
pub const SPEED_ICON: &str = "⚡";

/// Prefix for the interrupted part of the run footer (TS: `INTERRUPTED_ICON`).
pub const INTERRUPTED_ICON: &str = "⏹";

// ---------------------------------------------------------------------------
// Duration helpers (`duration-formatter.ts`)
// ---------------------------------------------------------------------------

/// Appends a duration the way the assistant run footer does
/// (TS: `appendDuration`: `` `${text} · ${DURATION_ICON} ${elapsed}` ``).
pub fn append_duration(text: &str, elapsed: &str) -> String {
    format!("{text} · {DURATION_ICON} {elapsed}")
}

/// Same as `append_duration`, on the first line of a multi-line message
/// (TS: `appendDurationToFirstLine`).
pub fn append_duration_to_first_line(text: &str, elapsed: &str) -> String {
    match text.find('\n') {
        None => append_duration(text, elapsed),
        Some(line_end) => {
            let (first, rest) = text.split_at(line_end);
            format!("{}{}", append_duration(first, elapsed), rest)
        }
    }
}

/// Shared duration format `1h 2m 3s` (TS: `formatDuration`).
pub fn format_duration(elapsed_ms: i64) -> String {
    let safe_ms = elapsed_ms.max(0);
    let total_seconds = safe_ms / 1000;

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let mut parts: Vec<String> = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}s"));
    }

    parts.join(" ")
}

/// Reads as "longer than N hours" in the same notation, e.g. `>24h`
/// (TS: `formatDurationOverHours`).
pub fn format_duration_over_hours(hours: i64) -> String {
    format!(">{hours}h")
}

// ---------------------------------------------------------------------------
// Assistant run footer (`assistant-run-footer-formatter.ts`)
// ---------------------------------------------------------------------------

/// One assistant message of a run, the unit OpenCode's speed formula sums
/// (TS: `AssistantRunStep`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantRunStep {
    pub message_id: String,
    pub created: i64,
    pub streamed: Option<i64>,
    pub output: i64,
    pub reasoning: i64,
}

/// Output and reasoning tokens per second across the run's steps, or `None`
/// until every step has streamed and the totals are positive. Each step is
/// weighted by its own stream span; a negative span counts as zero
/// (TS: `assistantRunTokensPerSecond`).
pub fn assistant_run_tokens_per_second(steps: &[AssistantRunStep]) -> Option<f64> {
    if steps.is_empty() || steps.iter().any(|step| step.streamed.is_none()) {
        return None;
    }

    let total_ms: i64 = steps
        .iter()
        .map(|step| (step.streamed.unwrap_or(0) - step.created).max(0))
        .sum();
    let tokens: i64 = steps
        .iter()
        .map(|step| step.output + step.reasoning)
        .sum();
    if tokens <= 0 || total_ms <= 0 {
        return None;
    }

    Some(tokens as f64 / (total_ms as f64 / 1_000.0))
}

/// Footer line: `agent · 🧠 provider/model · 🕒 elapsed` plus an optional
/// `⚡ x.x tok/s` part and an optional `⏹ label` part. `speed` is the
/// precomputed `assistant_run_tokens_per_second` value so this gate stays
/// pure (TS: `formatAssistantRunFooter`).
pub fn format_assistant_run_footer(
    agent_display: &str,
    provider_id: &str,
    model_id: &str,
    elapsed_ms: i64,
    speed: Option<f64>,
    interrupted: bool,
    interrupted_label: &str,
) -> String {
    let mut parts = vec![
        agent_display.to_string(),
        format!("🧠 {provider_id}/{model_id}"),
        format!("{DURATION_ICON} {}", format_duration(elapsed_ms)),
    ];
    if let Some(value) = speed {
        parts.push(format!("{SPEED_ICON} {value:.1} tok/s"));
    }
    if interrupted {
        parts.push(format!("{INTERRUPTED_ICON} {interrupted_label}"));
    }
    parts.join(" · ")
}

// ---------------------------------------------------------------------------
// Scheduled-task cron badge (`scheduled-task-display-formatter.ts`)
// ---------------------------------------------------------------------------

/// Five whitespace-separated cron fields (TS: `CronParts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronParts {
    pub minute: String,
    pub hour: String,
    pub day_of_month: String,
    pub month: String,
    pub day_of_week: String,
}

/// Splits a cron string into its five fields; `None` unless exactly five
/// non-empty parts are present (TS: `parseCronParts`).
pub fn parse_cron_parts(cron: &str) -> Option<CronParts> {
    let parts: Vec<&str> = cron.trim().split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }
    let [minute, hour, day_of_month, month, day_of_week] = parts.as_slice() else {
        return None;
    };
    if minute.is_empty()
        || hour.is_empty()
        || day_of_month.is_empty()
        || month.is_empty()
        || day_of_week.is_empty()
    {
        return None;
    }
    Some(CronParts {
        minute: minute.to_string(),
        hour: hour.to_string(),
        day_of_month: day_of_month.to_string(),
        month: month.to_string(),
        day_of_week: day_of_week.to_string(),
    })
}

/// Exact unsigned integer in `min..=max`; `None` on any other shape
/// (TS: `parseExactNumber` with `/^\d+$/`).
pub fn parse_exact_number(field: &str, min: i64, max: i64) -> Option<i64> {
    let trimmed = field.trim();
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: i64 = trimmed.parse().ok()?;
    if value < min || value > max {
        return None;
    }
    Some(value)
}

/// `*/step` with `min_step <= step <= max_step`; `None` otherwise
/// (TS: `parseEveryStep` with `/^\*\/(\d+)$/`).
pub fn parse_every_step(field: &str, min_step: i64, max_step: i64) -> Option<i64> {
    let trimmed = field.trim();
    let digits = trimmed.strip_prefix("*/")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let step: i64 = digits.parse().ok()?;
    if step < min_step || step > max_step {
        return None;
    }
    Some(step)
}

/// Weekday alias (`sun` -> `0`, ..., `sat` -> `6`, case-insensitive) or a
/// numeric `0..=7` value with `7` normalized to `0`; `None` otherwise
/// (TS: `normalizeWeekdayValue`).
pub fn normalize_weekday_value(token: &str) -> Option<i64> {
    let normalized = token.trim().to_lowercase();
    match normalized.as_str() {
        "sun" => return Some(0),
        "mon" => return Some(1),
        "tue" => return Some(2),
        "wed" => return Some(3),
        "thu" => return Some(4),
        "fri" => return Some(5),
        "sat" => return Some(6),
        _ => {}
    }
    let numeric = parse_exact_number(&normalized, 0, 7)?;
    Some(if numeric == 7 { 0 } else { numeric })
}

/// Comma-separated weekday tokens with `-` ranges; `None` on empty tokens,
/// `/` steps, bad ranges, or `start > end` (TS: `parseWeekdaySet`).
pub fn parse_weekday_set(field: &str) -> Option<BTreeSet<i64>> {
    let mut values = BTreeSet::new();
    for token in field.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() || trimmed.contains('/') {
            return None;
        }
        if trimmed.contains('-') {
            let dash_parts: Vec<&str> = trimmed.split('-').collect();
            if dash_parts.len() < 2 {
                return None;
            }
            let start = normalize_weekday_value(dash_parts[0])?;
            let end = normalize_weekday_value(dash_parts[1])?;
            if start > end {
                return None;
            }
            for value in start..=end {
                values.insert(value);
            }
            continue;
        }
        values.insert(normalize_weekday_value(trimmed)?);
    }
    Some(values)
}

/// True when the set holds exactly the expected values
/// (TS: `hasExactValues`).
pub fn has_exact_values(values: &BTreeSet<i64>, expected: &[i64]) -> bool {
    values.len() == expected.len() && expected.iter().all(|value| values.contains(value))
}

/// Sunday-first weekday label (`0` -> `"Sun"`, ..., `6` -> `"Sat"`);
/// `None` outside `0..=6` (TS: `WEEKDAY_LABELS[weekday]`).
pub fn weekday_label(value: i64) -> Option<&'static str> {
    match value {
        0 => Some("Sun"),
        1 => Some("Mon"),
        2 => Some("Tue"),
        3 => Some("Wed"),
        4 => Some("Thu"),
        5 => Some("Fri"),
        6 => Some("Sat"),
        _ => None,
    }
}

/// Two-digit padding (TS: `padNumber`).
pub fn pad_number(value: i64) -> String {
    format!("{value:02}")
}

/// `HH:MM` with both parts padded (TS: `formatTime`).
pub fn format_time(hour: i64, minute: i64) -> String {
    format!("{}:{}", pad_number(hour), pad_number(minute))
}

/// Short badge for a cron schedule (`15m`, `2h`, `hourly`, `daily 09:00`,
/// `weekdays 09:00`, `weekends 09:00`, `Mon 09:00`, `monthly 15 09:00`);
/// `"cron"` when no short form matches (TS: `formatCronTaskBadge`).
pub fn format_cron_task_badge(cron: &str) -> String {
    let parts = match parse_cron_parts(cron) {
        Some(parts) => parts,
        None => return "cron".to_string(),
    };

    let minute = parse_exact_number(&parts.minute, 0, 59);
    let hour = parse_exact_number(&parts.hour, 0, 23);
    let every_minute_step = parse_every_step(&parts.minute, 1, 59);
    let every_hour_step = parse_every_step(&parts.hour, 1, 23);
    let month_is_wildcard = parts.month == "*";
    let day_of_month_is_wildcard = parts.day_of_month == "*";
    let day_of_week_is_wildcard = parts.day_of_week == "*";

    if every_minute_step.is_some()
        && parts.hour == "*"
        && month_is_wildcard
        && day_of_month_is_wildcard
        && day_of_week_is_wildcard
    {
        return format!("{}m", every_minute_step.unwrap_or(0));
    }

    if let (Some(minute_value), Some(every_hour_value)) = (minute, every_hour_step) {
        if month_is_wildcard && day_of_month_is_wildcard && day_of_week_is_wildcard {
            if minute_value == 0 {
                return format!("{every_hour_value}h");
            }
            return format!("{every_hour_value}h :{}", pad_number(minute_value));
        }
    }

    if let Some(minute_value) = minute {
        if parts.hour == "*"
            && month_is_wildcard
            && day_of_month_is_wildcard
            && day_of_week_is_wildcard
        {
            if minute_value == 0 {
                return "hourly".to_string();
            }
            return format!("hourly :{}", pad_number(minute_value));
        }
    }

    if let (Some(minute_value), Some(hour_value)) = (minute, hour) {
        if month_is_wildcard && day_of_month_is_wildcard && day_of_week_is_wildcard {
            return format!("daily {}", format_time(hour_value, minute_value));
        }

        if month_is_wildcard && day_of_month_is_wildcard {
            let weekday_values = match parse_weekday_set(&parts.day_of_week) {
                Some(values) => values,
                None => return "cron".to_string(),
            };
            if has_exact_values(&weekday_values, &[1, 2, 3, 4, 5]) {
                return format!("weekdays {}", format_time(hour_value, minute_value));
            }
            if has_exact_values(&weekday_values, &[0, 6]) {
                return format!("weekends {}", format_time(hour_value, minute_value));
            }
            if weekday_values.len() == 1 {
                let weekday = weekday_values.iter().next().copied().unwrap_or(-1);
                match weekday_label(weekday) {
                    Some(label) => {
                        return format!("{label} {}", format_time(hour_value, minute_value));
                    }
                    None => return "cron".to_string(),
                }
            }
        }

        if let Some(day) = parse_exact_number(&parts.day_of_month, 1, 31) {
            if month_is_wildcard && day_of_week_is_wildcard {
                return format!("monthly {day} {}", format_time(hour_value, minute_value));
            }
        }
    }

    "cron".to_string()
}
