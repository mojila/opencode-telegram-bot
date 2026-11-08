//! Subagent card pure gates (slice 77, GREEN).
//!
//! Rust port of the pure remainder of
//! `src/app/formatters/subagent-formatter.ts`:
//! `formatModelDisplayName`, `shouldPreferInputDetails`, the
//! `task`-with-arguments check inside `formatToolStep`, `formatToolElapsed`
//! (threshold half), `formatEndedLine`, `formatSubagentActivity`,
//! `formatHeaderLines`, `renderSubagentCard` (starting vs full-card state
//! machine) and `renderSubagentCards` (empty filter + blank-line join).
//!
//! Out of scope (owned elsewhere, callers precompute them):
//! - `formatToolStep`'s `formatCompactToolInfo` rendering and
//!   `formatRunningTaskLine` live in `summary-formatter.ts` (see
//!   `error_diff_display`); this module takes their output as `tool_step` /
//!   `launch_line`.
//! - `bucketElapsedMs` lives in `streaming.rs`; elapsed strings arrive
//!   preformatted, the same way `format_assistant_run_footer` takes a
//!   precomputed speed in `duration_footer_display_gates`.
//! - `formatDuration` / `appendDuration` live in
//!   `duration_footer_display_gates`; this module reuses them instead of
//!   restating the `1h 2m 3s` / `text · 🕒 elapsed` rules.
//! - `TOOL_ELAPSED_THRESHOLD_MS` is owned by `duration-formatter.ts` via
//!   `duration_footer_display_gates`; this module reuses that constant
//!   (`20_000`).
//! - `t(...)` lookups (`subagent.starting`, `subagent.starting_untitled`,
//!   `subagent.completed`, `subagent.interrupted`, `subagent.failed`,
//!   `subagent.working`, `subagent.line.task`, `subagent.line.agent`,
//!   `pinned.line.model`) are i18n/store concerns: every label arrives as an
//!   explicit argument.
//! - The `await` in `renderSubagentCard(s)` is incidental; the pure remainder
//!   is synchronous over already-resolved inputs.
//!
//! Deviations: none in behavior for the parity fixtures.
//! - Empty-string checks use `is_empty()` (TS falsy check): `""` counts as
//!   missing for provider/model/variant/description/agent/model-name, while
//!   whitespace-only strings stay present, exactly like TS truthiness.
//! - `tool_step` / `tool_elapsed` / `launch_elapsed` treat `Some("")` as
//!   absent (TS `if (toolStep)` / `elapsed ? ... : ...` falsy branch); the
//!   fixtures only use `None` vs non-empty values, where the two coincide.
//! - `launch_line` treats `Some("")` as absent and falls through to the
//!   working line; TS cannot produce an empty `formatRunningTaskLine` for a
//!   defined `launchStartedAt` in the covered cases.
//! - `SubagentStatus` is `"pending" | "running" | `"completed" | "error" |
//!   "interrupted"`
//!   (`src/app/managers/summary-aggregation-manager.ts`); statuses travel as
//!   `&str` so unknown values fall through to the working line exactly like
//!   the TS `else` chain.

use crate::duration_footer_display_gates::{
    TOOL_ELAPSED_THRESHOLD_MS, append_duration, format_duration,
};

/// Resolved `t("subagent.*")` labels for one activity line (TS: the `t(...)`
/// calls inside `formatSubagentActivity`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubagentLabels<'a> {
    /// `t("subagent.completed")` (en: `"Completed"`).
    pub completed: &'a str,
    /// `t("subagent.interrupted")` (en: `"Interrupted"`).
    pub interrupted: &'a str,
    /// `t("subagent.failed")` (en: `"Task failed"`).
    pub failed: &'a str,
    /// `t("subagent.working")` (en: `"Working..."`).
    pub working: &'a str,
}

/// Resolved line templates for one card (TS: the `t(...)` calls inside
/// `formatHeaderLines` and the starting branch of `renderSubagentCard`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubagentCardTemplates<'a> {
    /// `t("subagent.line.task")` (en: `"Task: {task}"`).
    pub task_template: &'a str,
    /// `t("subagent.line.agent")` (en: `"Agent: {agent}"`).
    pub agent_template: &'a str,
    /// `t("pinned.line.model")` (en: `"Model: {model}"`).
    pub model_template: &'a str,
    /// `t("subagent.starting")` (en: `"Starting task: {task}"`).
    pub starting_template: &'a str,
    /// `t("subagent.starting_untitled")` (en: `"Starting task..."`).
    pub starting_untitled: &'a str,
    /// Activity labels (see [`SubagentLabels`]).
    pub labels: SubagentLabels<'a>,
}

/// One subagent card with every impure lookup already resolved (TS:
/// `SubagentInfo` in `summary-aggregation-manager.ts` plus the
/// `formatCompactToolInfo` / `formatRunningTaskLine` outputs the card
/// embeds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentCardInput<'a> {
    /// `subagent.description` (may be empty while the card is still starting).
    pub description: &'a str,
    /// `subagent.agent` (may be empty while the card is still starting).
    pub agent: &'a str,
    /// Precomputed `format_model_display_name` output (`None` while unknown).
    pub model_name: Option<&'a str>,
    /// `subagent.status` (`"pending" | "running" | "completed" | "error" |
    /// "interrupted"`).
    pub status: &'a str,
    /// Precomputed `formatToolStep` display (`None`/empty while there is no
    /// current tool step to show).
    pub tool_step: Option<&'a str>,
    /// Preformatted elapsed of the current tool step (`formatDuration` of the
    /// bucketed `currentToolStartedAt` span); `None` below the threshold.
    pub tool_elapsed: Option<&'a str>,
    /// Precomputed `formatRunningTaskLine({ description: launchDescription })`
    /// (`Some` exactly when `launchStartedAt` is defined).
    pub launch_line: Option<&'a str>,
    /// Preformatted elapsed of the nested launch; `None` below the threshold.
    pub launch_elapsed: Option<&'a str>,
    /// `subagent.terminalMessage` for the `"error"` status.
    pub terminal_message: Option<&'a str>,
    /// `subagent.finishedAt` for the ended line.
    pub finished_at: Option<i64>,
    /// `subagent.createdAt` for the ended line.
    pub created_at: i64,
}

/// `providerID/modelID` with an optional ` (variant)` suffix, or `None`
/// unless both ids are present and non-empty (TS: `formatModelDisplayName`;
/// empty strings are falsy in TS, so they count as missing).
pub fn format_model_display_name(
    provider_id: Option<&str>,
    model_id: Option<&str>,
    variant: Option<&str>,
) -> Option<String> {
    let provider = match provider_id {
        Some(id) if !id.is_empty() => id,
        _ => return None,
    };
    let model = match model_id {
        Some(id) if !id.is_empty() => id,
        _ => return None,
    };
    let name = format!("{provider}/{model}");
    match variant {
        Some(suffix) if !suffix.is_empty() => Some(format!("{name} ({suffix})")),
        _ => Some(name),
    }
}

/// True when the tool's input names the step better than its title
/// (TS: `shouldPreferInputDetails`: `read`/`edit`/`write`/`apply_patch`
/// look for `path`/`filePath`, `bash` for `command`, `grep`/`glob` for
/// `pattern`, every other tool for `query`/`url`/`name`/`prompt`/`text`;
/// `string_keys` holds the input keys whose value is a string).
pub fn should_prefer_input_details(tool: &str, string_keys: &[&str]) -> bool {
    match tool {
        "read" | "edit" | "write" | "apply_patch" => {
            string_keys.contains(&"path") || string_keys.contains(&"filePath")
        }
        "bash" => string_keys.contains(&"command"),
        "grep" | "glob" => string_keys.contains(&"pattern"),
        _ => ["query", "url", "name", "prompt", "text"]
            .iter()
            .any(|field| string_keys.contains(field)),
    }
}

/// True for a nested-launch step that already has arguments (TS: the
/// `currentTool === "task"` + `Object.keys(input ?? {}).length > 0` check in
/// `formatToolStep`).
pub fn is_task_with_arguments(tool: Option<&str>, input_key_count: usize) -> bool {
    matches!(tool, Some("task")) && input_key_count > 0
}

/// Elapsed millis of a step/launch started at `started_at`, or `None` when
/// `started_at` is undefined or the span is still below
/// `TOOL_ELAPSED_THRESHOLD_MS` (TS: `formatToolElapsed` without the
/// `formatDuration(bucketElapsedMs(...))` wrap, which stays with the caller).
pub fn tool_elapsed_visible(started_at: Option<i64>, now_ms: i64) -> Option<i64> {
    let started = started_at?;
    let elapsed = now_ms - started;
    if elapsed < TOOL_ELAPSED_THRESHOLD_MS {
        return None;
    }
    Some(elapsed)
}

/// `line` plus ` · 🕒 elapsed` once `finished_at` is known, else `line`
/// unchanged (TS: `formatEndedLine`; reuses `append_duration` +
/// `format_duration` from `duration_footer_display_gates`).
pub fn format_ended_line(line: &str, finished_at: Option<i64>, created_at: i64) -> String {
    match finished_at {
        None => line.to_string(),
        Some(finished) => append_duration(line, &format_duration(finished - created_at)),
    }
}

/// The card's last line for `status`: ended lines for `"completed"` /
/// `"interrupted"`, `❌ message` for `"error"`, the tool step (plus elapsed)
/// while one shows, else the nested-launch line (plus elapsed), else the
/// working line (TS: `formatSubagentActivity`).
pub fn format_subagent_activity(
    status: &str,
    terminal_message: Option<&str>,
    tool_step: Option<&str>,
    tool_elapsed: Option<&str>,
    launch_line: Option<&str>,
    launch_elapsed: Option<&str>,
    finished_at: Option<i64>,
    created_at: i64,
    labels: &SubagentLabels,
) -> String {
    if status == "completed" {
        return format_ended_line(
            &format!("✅ {}", labels.completed),
            finished_at,
            created_at,
        );
    }
    if status == "interrupted" {
        return format_ended_line(
            &format!("⏹ {}", labels.interrupted),
            finished_at,
            created_at,
        );
    }
    if status == "error" {
        let message = terminal_message
            .map(str::trim)
            .filter(|trimmed| !trimmed.is_empty())
            .unwrap_or(labels.failed);
        return format!("❌ {message}");
    }
    if let Some(step) = tool_step.filter(|step| !step.is_empty()) {
        return match tool_elapsed.filter(|elapsed| !elapsed.is_empty()) {
            Some(elapsed) => append_duration(step, elapsed),
            None => step.to_string(),
        };
    }
    if let Some(line) = launch_line.filter(|line| !line.is_empty()) {
        return match launch_elapsed.filter(|elapsed| !elapsed.is_empty()) {
            Some(elapsed) => append_duration(line, elapsed),
            None => line.to_string(),
        };
    }
    format!("⚙️ {}", labels.working)
}

/// The Task, Agent and Model lines known so far, in order, skipping unknowns
/// (TS: `formatHeaderLines`; empty `description`/`agent` and a `None` model
/// contribute no line).
pub fn format_header_lines(
    description: &str,
    agent: &str,
    model_name: Option<&str>,
    templates: &SubagentCardTemplates,
) -> Vec<String> {
    let mut lines = Vec::new();
    if !description.is_empty() {
        lines.push(templates.task_template.replace("{task}", description));
    }
    if !agent.is_empty() {
        lines.push(templates.agent_template.replace("{agent}", agent));
    }
    if let Some(model) = model_name.filter(|model| !model.is_empty()) {
        lines.push(templates.model_template.replace("{model}", model));
    }
    lines
}

/// True for the ended statuses (TS: the `hasEnded` check in
/// `renderSubagentCard`: `"completed" | "error" | "interrupted"`).
pub fn has_ended_status(status: &str) -> bool {
    matches!(status, "completed" | "error" | "interrupted")
}

/// One line while Task, Agent and Model are not all known yet and the
/// subagent has not ended, else the full header + activity card (TS:
/// `renderSubagentCard`).
pub fn render_subagent_card(
    card: &SubagentCardInput,
    templates: &SubagentCardTemplates,
) -> String {
    let header_lines =
        format_header_lines(card.description, card.agent, card.model_name, templates);
    let is_complete = header_lines.len() == 3;
    let has_ended = has_ended_status(card.status);
    if !is_complete && !has_ended {
        if !card.description.is_empty() {
            return format!(
                "🧩 {}",
                templates
                    .starting_template
                    .replace("{task}", card.description)
            );
        }
        return format!("🧩 {}", templates.starting_untitled);
    }
    let activity = format_subagent_activity(
        card.status,
        card.terminal_message,
        card.tool_step,
        card.tool_elapsed,
        card.launch_line,
        card.launch_elapsed,
        card.finished_at,
        card.created_at,
        &templates.labels,
    );
    if header_lines.is_empty() {
        return format!("🧩 {activity}");
    }
    let mut out = format!("🧩 {}", header_lines[0]);
    for line in &header_lines[1..] {
        out.push('\n');
        out.push_str(line);
    }
    out.push_str("\n\n");
    out.push_str(&activity);
    out
}

/// Blank-line join of already-rendered cards, dropping empties (TS: the
/// `parts.filter(Boolean).join("\n\n")` tail of `renderSubagentCards`; `[]`
/// renders as `""`).
pub fn join_rendered_cards(parts: &[&str]) -> String {
    parts
        .iter()
        .copied()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
