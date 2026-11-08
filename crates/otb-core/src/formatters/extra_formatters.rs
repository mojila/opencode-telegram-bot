//! Extra pure formatters (RED stubs, cycle slice20b).
//!
//! Mirrors (TS source -> Rust stub):
//! - `src/app/formatters/duration-formatter.ts`
//!   -> format_duration, format_duration_over_hours, bucket_elapsed_ms,
//!      append_duration, append_duration_to_first_line
//! - `src/app/formatters/session-title-formatter.ts` -> format_session_title
//! - `src/app/formatters/compact-summary-formatter.ts`
//!   -> ToolCallCounts, empty_tool_call_counts, count_tool_call,
//!      format_compact_summary
//! - `src/app/formatters/assistant-run-footer-formatter.ts`
//!   -> AssistantRunStep, assistant_run_tokens_per_second,
//!      format_assistant_run_footer
//! - `src/app/formatters/scheduled-task-display-formatter.ts`
//!   -> ScheduledTaskBadge, format_task_list_badge (cron + once badges)
//! - `src/app/formatters/subagent-formatter.ts`
//!   -> SubagentCard, render_subagent_card (minimal pure stub)
//! - `src/bot/render/rich-blocks.ts`
//!   -> RichText, RichBlock, count_rich_blocks, count_rich_chars
//!
//! Every body is `unimplemented!()` so the parity tests below FAIL
//! until the real port lands. Signatures are kept minimal but typed so
//! the parity tests compile.

/// Counts from `compact-summary-formatter.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolCallCounts {
    pub commands: u32,
    pub edits: u32,
    pub reads: u32,
    pub tools: u32,
}

/// One assistant message of a run (`assistant-run-footer-formatter.ts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssistantRunStep {
    pub created_ms: i64,
    pub streamed_ms: Option<i64>,
    pub output: u32,
    pub reasoning: u32,
}

/// Params for `format_assistant_run_footer`.
#[derive(Debug, Clone, PartialEq)]
pub struct AssistantRunFooterParams {
    pub agent: String,
    pub provider_id: String,
    pub model_id: String,
    pub elapsed_ms: i64,
    pub steps: Vec<AssistantRunStep>,
    pub interrupted: bool,
    pub locale: String,
}

/// Minimal task kind for `format_task_list_badge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduledTaskBadge {
    Once { run_at: String, timezone: String },
    Cron { cron: String },
}

/// Minimal subagent card for `render_subagent_card`.
#[derive(Debug, Clone, PartialEq)]
pub struct SubagentCard {
    pub description: String,
    pub agent: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub variant: Option<String>,
    pub status: String,
    pub terminal_message: Option<String>,
    pub created_at_ms: i64,
    pub finished_at_ms: Option<i64>,
}

/// Minimal rich text (`rich-blocks.ts`).
#[derive(Debug, Clone, PartialEq)]
pub enum RichText {
    Plain(String),
    Bold(String),
    Italic(String),
}

/// Minimal rich block (`rich-blocks.ts`).
#[derive(Debug, Clone, PartialEq)]
pub enum RichBlock {
    Paragraph(String),
    Divider,
    List { items: Vec<String> },
    Table { rows: Vec<Vec<String>> },
    Blockquote { blocks: Vec<RichBlock> },
    Details { summary: String, blocks: Vec<RichBlock> },
    Pre { text: String },
}

// ---- duration-formatter.ts ----

const SECOND_MS: i64 = 1000;
const MINUTE_MS: i64 = 60 * SECOND_MS;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const TEN_MINUTES_MS: i64 = 10 * MINUTE_MS;
const FIVE_MINUTES_MS: i64 = 5 * MINUTE_MS;
const FIFTEEN_MINUTES_MS: i64 = 15 * MINUTE_MS;
const TEN_SECONDS_MS: i64 = 10 * SECOND_MS;

const DURATION_ICON: &str = "🕒";

fn floor_to(value: i64, step: i64) -> i64 {
    (value.div_euclid(step)) * step
}

pub fn format_duration(elapsed_ms: i64) -> String {
    let safe = elapsed_ms.max(0) as u64;
    let total_seconds = safe / 1000;
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

pub fn format_duration_over_hours(hours: i64) -> String {
    format!(">{hours}h")
}

pub fn bucket_elapsed_ms(elapsed_ms: i64) -> i64 {
    if elapsed_ms <= 0 {
        return 0;
    }
    if elapsed_ms < MINUTE_MS {
        return floor_to(elapsed_ms, TEN_SECONDS_MS);
    }
    if elapsed_ms < TEN_MINUTES_MS {
        return floor_to(elapsed_ms, MINUTE_MS);
    }
    if elapsed_ms < HOUR_MS {
        return floor_to(elapsed_ms, FIVE_MINUTES_MS);
    }
    floor_to(elapsed_ms, FIFTEEN_MINUTES_MS)
}

pub fn append_duration(text: &str, elapsed: &str) -> String {
    format!("{text} · {DURATION_ICON} {elapsed}")
}

pub fn append_duration_to_first_line(text: &str, elapsed: &str) -> String {
    match text.find('\n') {
        None => append_duration(text, elapsed),
        Some(idx) => {
            let (first, rest) = text.split_at(idx);
            format!("{}{rest}", append_duration(first, elapsed))
        }
    }
}

// ---- session-title-formatter.ts ----

pub fn format_session_title(title: &str, locale: &str) -> String {
    if title.is_empty() {
        if locale.starts_with("ru") {
            return "новая сессия".to_string();
        }
        return "new session".to_string();
    }
    title.to_string()
}

// ---- compact-summary-formatter.ts ----

pub fn empty_tool_call_counts() -> ToolCallCounts {
    ToolCallCounts {
        commands: 0,
        edits: 0,
        reads: 0,
        tools: 0,
    }
}

fn is_edit_tool(tool: &str) -> bool {
    matches!(tool, "edit" | "write" | "apply_patch")
}

pub fn count_tool_call(tool: &str, nested: &[(&str, &str)]) -> ToolCallCounts {
    let mut counts = empty_tool_call_counts();
    if tool == "question" {
        return counts;
    }
    if tool == "execute" {
        let finished: Vec<&&str> = nested
            .iter()
            .filter(|(_, status)| *status == "completed" || *status == "error")
            .map(|(t, _)| t)
            .collect();
        if finished.is_empty() {
            counts.tools = 1;
        }
        for nested_tool in finished {
            if *nested_tool == "read" {
                counts.reads += 1;
            } else {
                counts.tools += 1;
            }
        }
        return counts;
    }
    if tool == "bash" {
        counts.commands = 1;
    } else if is_edit_tool(tool) {
        counts.edits = 1;
    } else if tool == "read" {
        counts.reads = 1;
    } else {
        counts.tools = 1;
    }
    counts
}

fn compact_labels(locale: &str) -> [&str; 5] {
    if locale.starts_with("ru") {
        [
            "команды",
            "правки",
            "чтения",
            "инструменты",
            "изменённые файлы",
        ]
    } else {
        [
            "commands",
            "edits",
            "reads",
            "tools",
            "changed files",
        ]
    }
}

pub fn format_compact_summary(
    counts: &ToolCallCounts,
    changed_files: u32,
    locale: &str,
) -> String {
    let labels = compact_labels(locale);
    let values = [
        counts.commands,
        counts.edits,
        counts.reads,
        counts.tools,
        changed_files,
    ];
    let parts: Vec<String> = labels
        .iter()
        .zip(values.iter())
        .filter(|(_, v)| **v > 0)
        .map(|(label, v)| format!("{label}: {v}"))
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!("✅ {}", parts.join(", "))
    }
}

// ---- assistant-run-footer-formatter.ts ----

fn agent_emoji(agent: &str) -> &str {
    match agent {
        "plan" => "📋",
        "build" => "🛠️",
        "general" => "💬",
        "explore" => "🔍",
        "title" => "📝",
        "summary" => "📄",
        "compaction" => "📦",
        _ => "🤖",
    }
}

fn agent_display_name(agent: &str) -> String {
    let mut chars = agent.chars();
    let capitalized = match chars.next() {
        None => String::new(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
    };
    format!("{} {capitalized}", agent_emoji(agent))
}

pub fn assistant_run_tokens_per_second(steps: &[AssistantRunStep]) -> Option<f64> {
    if steps.is_empty() || steps.iter().any(|s| s.streamed_ms.is_none()) {
        return None;
    }
    let total_ms: i64 = steps
        .iter()
        .map(|s| (s.streamed_ms.unwrap_or(0) - s.created_ms).max(0))
        .sum();
    let tokens: u64 = steps
        .iter()
        .map(|s| s.output as u64 + s.reasoning as u64)
        .sum();
    if tokens == 0 || total_ms <= 0 {
        return None;
    }
    Some(tokens as f64 / (total_ms as f64 / 1000.0))
}

pub fn format_assistant_run_footer(params: &AssistantRunFooterParams) -> String {
    let mut parts = vec![
        agent_display_name(&params.agent),
        format!("🧠 {}/{}", params.provider_id, params.model_id),
        format!("{DURATION_ICON} {}", format_duration(params.elapsed_ms)),
    ];
    if let Some(speed) = assistant_run_tokens_per_second(&params.steps) {
        parts.push(format!("⚡ {speed:.1} tok/s"));
    }
    if params.interrupted {
        let word = if params.locale.starts_with("ru") {
            "прерван"
        } else {
            "interrupted"
        };
        parts.push(format!("⏹ {word}"));
    }
    parts.join(" · ")
}

// ---- scheduled-task-display-formatter.ts ----

const WEEKDAY_LABELS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

fn pad_number(value: u32) -> String {
    format!("{value:02}")
}

fn format_time(hour: u32, minute: u32) -> String {
    format!("{}:{}", pad_number(hour), pad_number(minute))
}

fn parse_cron_parts(cron: &str) -> Option<[String; 5]> {
    let parts: Vec<&str> = cron.trim().split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }
    Some([
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
        parts[3].to_string(),
        parts[4].to_string(),
    ])
}

fn parse_exact_number(field: &str, min: i64, max: i64) -> Option<i64> {
    let t = field.trim();
    if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: i64 = t.parse().ok()?;
    if value < min || value > max {
        return None;
    }
    Some(value)
}

fn parse_every_step(field: &str, min_step: i64, max_step: i64) -> Option<i64> {
    let t = field.trim();
    let rest = t.strip_prefix("*/")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let step: i64 = rest.parse().ok()?;
    if step < min_step || step > max_step {
        return None;
    }
    Some(step)
}

fn normalize_weekday_value(token: &str) -> Option<i64> {
    let n = token.trim().to_lowercase();
    match n.as_str() {
        "sun" => return Some(0),
        "mon" => return Some(1),
        "tue" => return Some(2),
        "wed" => return Some(3),
        "thu" => return Some(4),
        "fri" => return Some(5),
        "sat" => return Some(6),
        _ => {}
    }
    let v = parse_exact_number(&n, 0, 7)?;
    Some(if v == 7 { 0 } else { v })
}

fn parse_weekday_set(field: &str) -> Option<Vec<i64>> {
    let mut values: Vec<i64> = Vec::new();
    for token in field.split(',') {
        let t = token.trim();
        if t.is_empty() || t.contains('/') {
            return None;
        }
        if let Some(dash) = t.find('-') {
            let (start_raw, end_raw) = t.split_at(dash);
            let end_raw = &end_raw[1..];
            let start = normalize_weekday_value(start_raw)?;
            let end = normalize_weekday_value(end_raw)?;
            if start > end {
                return None;
            }
            for v in start..=end {
                if !values.contains(&v) {
                    values.push(v);
                }
            }
            continue;
        }
        let v = normalize_weekday_value(t)?;
        if !values.contains(&v) {
            values.push(v);
        }
    }
    Some(values)
}

fn has_exact_values(values: &[i64], expected: &[i64]) -> bool {
    values.len() == expected.len() && expected.iter().all(|v| values.contains(v))
}

fn format_cron_task_badge(cron: &str) -> String {
    let parts = match parse_cron_parts(cron) {
        Some(p) => p,
        None => return "cron".to_string(),
    };
    let (minute_s, hour_s, dom_s, month_s, dow_s) =
        (parts[0].as_str(), parts[1].as_str(), parts[2].as_str(), parts[3].as_str(), parts[4].as_str());

    let minute = parse_exact_number(minute_s, 0, 59);
    let hour = parse_exact_number(hour_s, 0, 23);
    let every_minute_step = parse_every_step(minute_s, 1, 59);
    let every_hour_step = parse_every_step(hour_s, 1, 23);
    let month_wild = month_s == "*";
    let dom_wild = dom_s == "*";
    let dow_wild = dow_s == "*";

    if every_minute_step.is_some()
        && hour_s == "*"
        && month_wild
        && dom_wild
        && dow_wild
    {
        return format!("{}m", every_minute_step.unwrap_or(0));
    }

    if minute.is_some() && every_hour_step.is_some() && month_wild && dom_wild && dow_wild {
        let m = minute.unwrap_or(0) as u32;
        let step = every_hour_step.unwrap_or(0);
        if m == 0 {
            return format!("{step}h");
        }
        return format!("{step}h :{}", pad_number(m));
    }

    if minute.is_some() && hour_s == "*" && month_wild && dom_wild && dow_wild {
        let m = minute.unwrap_or(0) as u32;
        if m == 0 {
            return "hourly".to_string();
        }
        return format!("hourly :{}", pad_number(m));
    }

    if minute.is_some() && hour.is_some() && month_wild && dom_wild && dow_wild {
        return format!(
            "daily {}",
            format_time(hour.unwrap_or(0) as u32, minute.unwrap_or(0) as u32)
        );
    }

    if minute.is_some() && hour.is_some() && month_wild && dom_wild {
        let weekday_values = match parse_weekday_set(dow_s) {
            Some(v) => v,
            None => return "cron".to_string(),
        };
        let h = hour.unwrap_or(0) as u32;
        let m = minute.unwrap_or(0) as u32;
        if has_exact_values(&weekday_values, &[1, 2, 3, 4, 5]) {
            return format!("weekdays {}", format_time(h, m));
        }
        if has_exact_values(&weekday_values, &[0, 6]) {
            return format!("weekends {}", format_time(h, m));
        }
        if weekday_values.len() == 1 {
            let label = WEEKDAY_LABELS
                .get(weekday_values[0] as usize)
                .copied()
                .unwrap_or("");
            if label.is_empty() {
                return "cron".to_string();
            }
            return format!("{label} {}", format_time(h, m));
        }
    }

    let dom = parse_exact_number(dom_s, 1, 31);
    if minute.is_some() && hour.is_some() && dom.is_some() && month_wild && dow_wild {
        return format!(
            "monthly {} {}",
            dom.unwrap_or(0),
            format_time(hour.unwrap_or(0) as u32, minute.unwrap_or(0) as u32)
        );
    }

    "cron".to_string()
}

pub fn format_task_list_badge(task: &ScheduledTaskBadge) -> String {
    match task {
        ScheduledTaskBadge::Once { run_at, .. } => run_at.clone(),
        ScheduledTaskBadge::Cron { cron } => format_cron_task_badge(cron),
    }
}

// ---- subagent-formatter.ts (minimal pure stub) ----

fn subagent_starting(description: &str, locale: &str) -> String {
    let ru = locale.starts_with("ru");
    if description.is_empty() {
        if ru {
            return "🧩 Запуск задачи...".to_string();
        }
        return "🧩 Starting task...".to_string();
    }
    if ru {
        return format!("🧩 Запуск задачи: {description}");
    }
    format!("🧩 Starting task: {description}")
}

pub fn render_subagent_card(card: &SubagentCard, locale: &str, _now_ms: i64) -> String {
    let has_model = card
        .provider_id
        .as_ref()
        .is_some_and(|s| !s.is_empty())
        && card.model_id.as_ref().is_some_and(|s| !s.is_empty());
    let mut header_count = 0;
    if !card.description.is_empty() {
        header_count += 1;
    }
    if !card.agent.is_empty() {
        header_count += 1;
    }
    if has_model {
        header_count += 1;
    }
    let is_complete = header_count == 3;
    let has_ended = matches!(card.status.as_str(), "completed" | "error" | "interrupted");

    if !is_complete && !has_ended {
        return subagent_starting(&card.description, locale);
    }

    // Ended or complete fallback (not covered by starting-card parity tests).
    match card.status.as_str() {
        "completed" => {
            let mut lines: Vec<String> = Vec::new();
            if !card.description.is_empty() {
                lines.push(format!("🧩 Task: {}", card.description));
            }
            if !card.agent.is_empty() {
                lines.push(format!("Agent: {}", card.agent));
            }
            let activity = match card.finished_at_ms {
                Some(finished) => append_duration(
                    "✅ Completed",
                    &format_duration(finished - card.created_at_ms),
                ),
                None => "✅ Completed".to_string(),
            };
            if lines.is_empty() {
                return activity;
            }
            lines.push(String::new());
            lines.push(activity);
            lines.join("\n")
        }
        "interrupted" => "🧩 ⏹ Interrupted".to_string(),
        "error" => {
            let msg = card
                .terminal_message
                .as_ref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .unwrap_or("Task failed");
            format!("❌ {msg}")
        }
        _ => "🧩 ⚙️ Working...".to_string(),
    }
}

// ---- rich-blocks.ts (counts only) ----

pub fn count_rich_blocks(block: &RichBlock) -> u32 {
    match block {
        RichBlock::Paragraph(_) | RichBlock::Divider | RichBlock::Pre { .. } => 1,
        RichBlock::List { items } => 1 + items.len() as u32,
        RichBlock::Table { rows } => 1 + rows.len() as u32,
        RichBlock::Blockquote { blocks } => {
            1 + blocks.iter().map(count_rich_blocks).sum::<u32>()
        }
        RichBlock::Details { blocks, .. } => {
            1 + blocks.iter().map(count_rich_blocks).sum::<u32>()
        }
    }
}

pub fn count_rich_chars(block: &RichBlock) -> usize {
    match block {
        RichBlock::Paragraph(s) => s.chars().count(),
        RichBlock::Divider => 0,
        RichBlock::List { items } => items.iter().map(|s| s.chars().count()).sum(),
        RichBlock::Table { rows } => rows
            .iter()
            .flat_map(|row| row.iter())
            .map(|s| s.chars().count())
            .sum(),
        RichBlock::Blockquote { blocks } => blocks.iter().map(count_rich_chars).sum(),
        RichBlock::Details { summary, blocks } => {
            summary.chars().count() + blocks.iter().map(count_rich_chars).sum::<usize>()
        }
        RichBlock::Pre { text } => text.chars().count(),
    }
}
