//! Task-list toggle outcomes + task-details builder + dashboard goal-line
//! builder + scheduled-task session-ignore guards (tasks/goal follow-up, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/scheduled-task-callback-handler.ts`
//!   (`handleTaskListCallback` toggle outcome branches: the toggle-null
//!   store-presence split (still stored answers `tasklist.inactive_callback`,
//!   gone clears the interaction and deletes the message), the
//!   `refreshTask(...) ?? task` display pick; `formatTaskDetails`
//!   model/cron/scheduling/project composition and the `formatDateTime`
//!   missing-date `"-"` fallback)
//! - `src/app/formatters/goal-text.ts` (`formatDashboardGoalLine` missing-goal
//!   null gate plus template render, `formatGoalNotice` complete/paused key
//!   pick, `formatObjectiveTooLong` limit/count params)
//! - `src/app/services/scheduled-task-session-ignore-service.ts`
//!   (`SCHEDULED_TASK_SESSION_IGNORE_TTL_MS`, `isFreshIgnore`, prune,
//!   ignored check, register/remove/cleanup next-state)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`), `toggleScheduledTask` / `getScheduledTask` /
//!   `removeScheduledTask` store calls, `scheduledTaskRuntime.refreshTask` /
//!   `removeTask` calls, `transition` / `clear`.
//! - `getScheduledTaskSessionIgnores` / `setScheduledTaskSessionIgnores`
//!   reads/writes and the mutation queue: these gates take a snapshot slice
//!   and return the next state; the caller persists it.
//! - `t(...)` lookups and `Intl` date rendering: reply keys and locale
//!   templates travel as literal key strings / pre-resolved lines here;
//!   datetimes arrive preformatted (or not at all).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `tasklist:open` / `tasklist:toggle` / `tasklist:delete` / `tasklist:cancel`
//!   parses and button-label composition (`recent_scheduled_gates` owner).
//! - Toggle/delete allow gates (`task_callback_gates::should_allow_tasklist_toggle`
//!   / `should_allow_tasklist_delete` owner) and the details prompt truncation
//!   (`task_callback_gates::truncate_prompt_for_details` owner).
//! - `GOAL_OBJECTIVE_LIMIT`, `truncate_goal_objective`, `flatten_goal_objective`
//!   (`session_gates` owner); the dashboard line reuses that truncation.
//! - Goal status-text composition (`goal_reason_gates::format_goal_status_text`
//!   owner) and `is_goal_internal_metadata` (`v2_mappers` owner).
//! - `/tasklist` sort order (`lifecycle_command_gates::compare_task_order`
//!   owner) and the delivery-format/footer gates
//!   (`assistant_delivery_gates` owner).
//!
//! Deviations: none intended. ISO datetimes arrive as pre-parsed
//! `Option<i64>` millis (`None` models a missing/unparseable value, i.e. TS
//! `Date.parse` yielding `NaN`), mirroring the `scheduled_tasks.rs`
//! precedent. The task-details and dashboard-goal templates are
//! locale-owned; the caller passes the resolved template (or resolved lines)
//! and this module only substitutes placeholders. `str::replace` is used for
//! substitution; every placeholder is distinct so ordering is irrelevant.

// ---------------------------------------------------------------------------
// Task-list toggle outcomes (`handleTaskListCallback` toggle branch)
// ---------------------------------------------------------------------------

/// What the toggle branch does when the store toggle resolves to no task
/// (TS: `if (!task)` after `toggleScheduledTask`: still stored answers
/// `tasklist.inactive_callback` and keeps the menu; gone clears the
/// interaction and deletes the menu message).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleMissingOutcome {
    /// The task is still in the store: answer inactive, keep the menu.
    AnswerInactive,
    /// The task is gone: clear the interaction and delete the menu message.
    ClearAndDelete,
}

/// Route a null toggle result by store presence
/// (TS: `if (getScheduledTask(taskId)) { inactive }` else
/// `clear + inactive + deleteMessage`).
pub fn decide_toggle_missing_task(still_in_store: bool) -> ToggleMissingOutcome {
    if still_in_store {
        ToggleMissingOutcome::AnswerInactive
    } else {
        ToggleMissingOutcome::ClearAndDelete
    }
}

/// Display pick after a successful toggle (TS: `task = (await
/// refreshTask(task.id)) ?? task`: the refreshed row wins when the runtime
/// still tracks it, otherwise the toggled row stays on screen).
pub fn prefer_refreshed_task<T>(refreshed: Option<T>, toggled: T) -> T {
    refreshed.unwrap_or(toggled)
}

// ---------------------------------------------------------------------------
// Task-details message builder (`formatTaskDetails` + `formatDateTime`)
// ---------------------------------------------------------------------------

/// `providerID/modelID`, plus ` (variant)` when one is set
/// (TS: the `model` line inside `formatTaskDetails`).
pub fn format_task_model_label(provider_id: &str, model_id: &str, variant: Option<&str>) -> String {
    match variant {
        Some(variant) if !variant.is_empty() => {
            format!("{provider_id}/{model_id} ({variant})")
        }
        _ => format!("{provider_id}/{model_id}"),
    }
}

/// Cron line plus its trailing newline for cron tasks, else empty
/// (TS: `` task.kind === "cron" ? `${cronLine}\n` : "" ``, where `cronLine`
/// is the already-localized `tasklist.details.cron` line).
pub fn task_cron_block(is_cron: bool, cron_line: &str) -> String {
    if is_cron {
        format!("{cron_line}\n")
    } else {
        String::new()
    }
}

/// `Scheduling: On|Off` line plus its trailing newline for cron tasks, else
/// empty (TS: the `task.scheduling` + `task.enabled` / `task.disabled`
/// append; every label arrives pre-resolved so no locale lives here).
pub fn task_scheduling_block(is_cron: bool, scheduling_label: &str, enabled_label: &str) -> String {
    if is_cron {
        format!("{scheduling_label}: {enabled_label}\n")
    } else {
        String::new()
    }
}

/// Project block of the details view (TS: `` `${projectWorktree}\n${mode
/// line}\n${model line}` ``; mode/model lines arrive pre-resolved from
/// `status.line.mode` / `status.line.model`).
pub fn format_task_project_block(worktree: &str, mode_line: &str, model_line: &str) -> String {
    format!("{worktree}\n{mode_line}\n{model_line}")
}

/// A preformatted run date, or `"-"` when there is none
/// (TS: `formatDateTime`: `if (!dateIso) return "-"`; the `Intl` rendering
/// itself stays with the caller).
pub fn resolve_task_date_label<'a>(formatted: Option<&'a str>) -> &'a str {
    formatted.unwrap_or("-")
}

/// Fill the locale-owned `tasklist.details` template
/// (TS: `t("tasklist.details", { prompt, project, schedule, cronLine,
/// timezone, nextRunAt, lastRunAt, runCount })`). The prompt must already be
/// cut via `task_callback_gates::truncate_prompt_for_details`, the project
/// via [`format_task_project_block`], and the dates via
/// [`resolve_task_date_label`].
#[allow(clippy::too_many_arguments)]
pub fn render_task_details(
    template: &str,
    prompt: &str,
    project: &str,
    schedule: &str,
    cron_line: &str,
    timezone: &str,
    next_run_at: &str,
    last_run_at: &str,
    run_count: &str,
) -> String {
    template
        .replace("{prompt}", prompt)
        .replace("{project}", project)
        .replace("{schedule}", schedule)
        .replace("{cronLine}", cron_line)
        .replace("{timezone}", timezone)
        .replace("{nextRunAt}", next_run_at)
        .replace("{lastRunAt}", last_run_at)
        .replace("{runCount}", run_count)
}

// ---------------------------------------------------------------------------
// Dashboard goal-line + goal-notice message builders (`goal-text.ts`)
// ---------------------------------------------------------------------------

/// Fill the locale-owned `pinned.line.goal` template with the already
/// localized status and the already truncated objective
/// (TS: `formatDashboardGoalLine`: `t("pinned.line.goal", { status,
/// objective })`; the objective preview reuses
/// `session_gates::truncate_goal_objective`, the status label reuses the
/// localized `formatGoalStatus` line).
pub fn render_dashboard_goal_line(
    template: &str,
    status_label: &str,
    objective_preview: &str,
) -> String {
    template
        .replace("{status}", status_label)
        .replace("{objective}", objective_preview)
}

/// Reply key for a goal terminal notice (TS: `formatGoalNotice`: complete
/// picks `goal.notice.complete`, paused picks `goal.notice.paused`; the
/// caller renders the key with the localized status/reason lines).
pub fn select_goal_notice_key(is_complete: bool) -> &'static str {
    if is_complete {
        "goal.notice.complete"
    } else {
        "goal.notice.paused"
    }
}

/// Notice reason line (TS: `notice.reason ? localizeGoalReason(reason) :
/// t("goal.notice.no_reason")`; the reason arrives already localized via
/// `goal_reason_gates::classify_goal_reason`, the fallback as the resolved
/// `goal.notice.no_reason` line).
pub fn resolve_goal_notice_reason<'a>(
    localized_reason: Option<&'a str>,
    no_reason_line: &'a str,
) -> &'a str {
    localized_reason.unwrap_or(no_reason_line)
}

/// Limit/count params for the over-limit objective line
/// (TS: `formatObjectiveTooLong`: `t("goal.too_long", { limit:
/// GOAL_OBJECTIVE_LIMIT, count })`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectiveTooLongParams {
    pub limit: usize,
    pub count: usize,
}

/// Pack the `goal.too_long` params for an over-limit objective length
/// (TS: `formatObjectiveTooLong(length)`; the limit is
/// `session_gates::GOAL_OBJECTIVE_LIMIT`).
pub fn objective_too_long_params(length: usize) -> ObjectiveTooLongParams {
    ObjectiveTooLongParams {
        limit: crate::session_gates::GOAL_OBJECTIVE_LIMIT,
        count: length,
    }
}

// ---------------------------------------------------------------------------
// Scheduled-task session-ignore guards
// (`scheduled-task-session-ignore-service.ts`)
// ---------------------------------------------------------------------------

/// How long a scheduled-task session ignore stays fresh
/// (TS: `SCHEDULED_TASK_SESSION_IGNORE_TTL_MS`).
pub const SCHEDULED_TASK_SESSION_IGNORE_TTL_MS: i64 = 24 * 60 * 60 * 1000;

/// One session-ignore row (TS: `ScheduledTaskSessionIgnoreInfo`);
/// `created_at_ms: None` models a missing/unparseable `createdAt`
/// (TS `Date.parse` yielding `NaN`, which is never fresh).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIgnore {
    pub session_id: String,
    pub created_at_ms: Option<i64>,
}

/// Whether one ignore row is still fresh
/// (TS: `isFreshIgnore`: `!Number.isNaN(createdAtMs) && nowMs - createdAtMs
/// < TTL`).
pub fn is_fresh_ignore(created_at_ms: Option<i64>, now_ms: i64) -> bool {
    match created_at_ms {
        None => false,
        Some(created) => now_ms - created < SCHEDULED_TASK_SESSION_IGNORE_TTL_MS,
    }
}

/// Drop expired rows, keeping input order
/// (TS: `pruneExpiredIgnores`).
pub fn prune_expired_ignores(ignores: &[SessionIgnore], now_ms: i64) -> Vec<SessionIgnore> {
    ignores
        .iter()
        .filter(|ignore| is_fresh_ignore(ignore.created_at_ms, now_ms))
        .cloned()
        .collect()
}

/// Whether the session currently carries a fresh ignore
/// (TS: `isScheduledTaskSessionIgnored`: any row with a matching
/// `sessionId` that is still fresh).
pub fn is_session_ignored(ignores: &[SessionIgnore], session_id: &str, now_ms: i64) -> bool {
    ignores.iter().any(|ignore| {
        ignore.session_id == session_id && is_fresh_ignore(ignore.created_at_ms, now_ms)
    })
}

/// Next state after registering an ignore: prune at `now_ms`, drop any row
/// for the session, then append the new row
/// (TS: `registerScheduledTaskSessionIgnore`; the TS clock reads
/// `createdAt.getTime()`, so callers normally pass the new row's time as
/// both `created_at_ms` and `now_ms`).
pub fn register_ignore_next(
    ignores: &[SessionIgnore],
    session_id: &str,
    created_at_ms: Option<i64>,
    now_ms: i64,
) -> Vec<SessionIgnore> {
    let mut next: Vec<SessionIgnore> = prune_expired_ignores(ignores, now_ms)
        .into_iter()
        .filter(|ignore| ignore.session_id != session_id)
        .collect();
    next.push(SessionIgnore {
        session_id: session_id.to_string(),
        created_at_ms,
    });
    next
}

/// Next state after removing an ignore (TS: `removeScheduledTaskSessionIgnore`).
pub fn remove_ignore_next(ignores: &[SessionIgnore], session_id: &str) -> Vec<SessionIgnore> {
    ignores
        .iter()
        .filter(|ignore| ignore.session_id != session_id)
        .cloned()
        .collect()
}

/// Next state plus dropped-row count after expiring old ignores
/// (TS: `cleanupScheduledTaskSessionIgnores` returns
/// `ignores.length - nextIgnores.length`).
pub fn cleanup_ignores_next(ignores: &[SessionIgnore], now_ms: i64) -> (Vec<SessionIgnore>, usize) {
    let next = prune_expired_ignores(ignores, now_ms);
    let removed = ignores.len() - next.len();
    (next, removed)
}
