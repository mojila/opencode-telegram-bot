//! Scheduled-task scheduler core / store pure / run-once gates (slice 37b, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/scheduled-task-runtime-service.ts`
//!   (`normalizeTaskPrompt` 64-char preview, `calculateElapsedMs` NaN -> 0,
//!   `MAX_TIMER_DELAY_MS` clamp, schedule/overdue/activation gates,
//!   startup-recovery decisions, success/error `nextRunAt` resolution,
//!   once-removal, flush/start guards, `RESTART_INTERRUPTED_ERROR`)
//! - `src/app/stores/scheduled-task-store.ts` (`toggleScheduledTask`
//!   next-run nulling on disable, remove-changed gate)
//! - `src/app/services/scheduled-task-next-run-service.ts` (`computeNextRunAt`
//!   once null gate, disabled-cron not-due gate)
//! - `src/app/services/scheduled-task-executor-service.ts`
//!   (`collectResponseText` join, timeout regex/prefix gates,
//!   execution-timeout message/ms, docs-URL normalization,
//!   finish-reason/tool-calls gate, empty/idle/startup caps,
//!   session-active + interactive-session match, polling constants)
//! - `src/app/services/scheduled-task-schedule-parser-service.ts`
//!   (JSON candidate order, error-payload passthrough, summary/kind/cron/
//!   runAt gates, model/variant prompt inclusion)
//!
//! Pure sync logic only: no Telegram/SDK I/O, no timers, no filesystem.
//!
//! Known deviations from TS (documented inline where relevant):
//! - `normalize_task_prompt` counts Unicode scalar values (`char`s) where TS
//!   `String.length`/`slice` count UTF-16 code units; identical for BMP text.
//! - `calculate_elapsed_ms` takes pre-parsed `Option<i64>` millis because
//!   `Date.parse` NaN handling has no direct Rust equivalent; `None` models
//!   a missing/unparseable timestamp (TS `NaN` -> `0`).
//! - `is_timeout_error_message` lowercases the input where TS uses the `i`
//!   regex flag; equivalent for the matched ASCII patterns.

/// `setTimeout` clamp (TS: `MAX_TIMER_DELAY_MS`).
pub const MAX_TIMER_DELAY_MS: i64 = 2_147_483_647;

/// Preview length for delivery notification text (TS:
/// `TASK_DESCRIPTION_PREVIEW_LENGTH`).
pub const TASK_DESCRIPTION_PREVIEW_LENGTH: usize = 64;

/// Poll cadence while waiting for the assistant result.
pub const EXECUTION_POLL_INTERVAL_MS: u64 = 2000;

/// Idle polls with no result before failing.
pub const MAX_IDLE_POLLS_WITHOUT_RESULT: u32 = 3;

/// Startup polls before the server registers activity.
pub const MAX_STARTUP_POLLS_WITHOUT_ACTIVITY: u32 = 45;

/// Re-check interval for a completed-but-empty reply.
pub const COMPLETED_EMPTY_RESULT_RECHECK_INTERVAL_MS: u64 = 500;

/// Empty completed reads before failing.
pub const MAX_COMPLETED_EMPTY_RESULT_RECHECKS: u32 = 3;

/// Temporary session title for scheduled runs.
pub const SCHEDULED_TASK_SESSION_TITLE: &str = "Scheduled task run";

/// `lastError` written when a `running` task is recovered after restart.
pub const RESTART_INTERRUPTED_ERROR: &str =
    "Interrupted by bot restart during scheduled task execution.";

/// Rejection message for interactive permission requests.
pub const INTERACTIVE_PERMISSION_REJECT_MESSAGE: &str =
    "Scheduled task cannot continue because it requires interactive permission.";

/// Docs URL appended to timeout errors.
pub const MODELS_DOCS_URL: &str = "https://opencode.ai/docs/config/#models";

/// Prefix for the bot-side execution-timeout error.
pub const EXECUTION_TIMEOUT_ERROR_PREFIX: &str = "Scheduled task exceeded bot execution timeout";

// ---------------------------------------------------------------------------
// Runtime prompt / elapsed helpers
// ---------------------------------------------------------------------------

/// Whitespace-collapsed 64-char preview with `...` cut
/// (TS: `normalizeTaskPrompt`).
pub fn normalize_task_prompt(prompt: &str) -> String {
    // TS: `prompt.replace(/\s+/g, " ").trim()`; `split_whitespace` covers the
    // same ASCII/Unicode whitespace runs and also trims the ends.
    let normalized = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    // Deviation: TS `slice` counts UTF-16 code units; we count `char`s so a
    // cut never splits a scalar value (identical output for BMP-only text).
    if normalized.chars().count() <= TASK_DESCRIPTION_PREVIEW_LENGTH {
        return normalized;
    }
    let head: String = normalized.chars().take(TASK_DESCRIPTION_PREVIEW_LENGTH).collect();
    format!("{head}...")
}

/// `finished - started`, or `0` when either side is missing/NaN
/// (TS: `calculateElapsedMs`).
pub fn calculate_elapsed_ms(started_at_ms: Option<i64>, finished_at_ms: Option<i64>) -> i64 {
    match (started_at_ms, finished_at_ms) {
        (Some(started), Some(finished)) => finished - started,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Scheduler gates (`scheduleTask`, `startExecution`, timers)
// ---------------------------------------------------------------------------

/// `min(delay, MAX_TIMER_DELAY_MS)` (TS: `timeoutMs` clamp).
pub fn clamp_timer_delay(delay_ms: i64) -> i64 {
    delay_ms.min(MAX_TIMER_DELAY_MS)
}

/// False for a disabled cron task or when `nextRunAt` is missing
/// (TS: `scheduleTask` early return).
pub fn should_schedule_task(kind: &str, enabled: bool, has_next_run: bool) -> bool {
    // TS: `if ((task.kind === "cron" && task.enabled === false) || !task.nextRunAt) return;`
    // Note: a disabled `once` task still schedules while it has a next run,
    // faithfully mirroring the TS gate which only exempts disabled cron.
    !(kind == "cron" && !enabled) && has_next_run
}

/// True when `delayMs <= 0` (TS: overdue branch).
pub fn is_overdue(delay_ms: i64) -> bool {
    delay_ms <= 0
}

/// True for an overdue cron task with `activationPending` (TS: defer to
/// `refreshTask` instead of `startExecution`).
pub fn overdue_cron_activation_defers(
    is_cron: bool,
    activation_pending: bool,
    overdue: bool,
) -> bool {
    is_cron && activation_pending && overdue
}

/// True when `lastStatus === "running"` (TS: restart recovery marks `error`).
pub fn should_mark_running_interrupted(last_status: &str) -> bool {
    last_status == "running"
}

/// True for a disabled cron task (TS: `nextRunAt` forced to `null`).
pub fn should_null_disabled_cron_next_run(kind: &str, enabled: bool) -> bool {
    kind == "cron" && !enabled
}

/// True when the cron `nextRunAt` must be recomputed on recovery
/// (TS: missing, NaN, or overdue activation).
pub fn should_recompute_cron_next_run(
    has_next_run: bool,
    next_run_valid: bool,
    activation_overdue: bool,
) -> bool {
    // TS: `!nextRunAt || Number.isNaN(Date.parse(nextRunAt)) ||
    //      (activationPending && Date.parse(nextRunAt) <= now)`
    !has_next_run || !next_run_valid || activation_overdue
}

/// True when a once task with `nextRunAt === null` and `lastStatus === "idle"`
/// gets its `nextRunAt` backfilled from `runAt` on recovery.
pub fn should_backfill_once_next_run(next_run_present: bool, last_status: &str) -> bool {
    !next_run_present && last_status == "idle"
}

/// True when a once task has no parseable `runAt` (TS: recovery error path).
pub fn is_invalid_once_run_at(run_at_present: bool, run_at_valid: bool) -> bool {
    // `Date.parse` of a missing value is NaN, so a missing `runAt` is invalid.
    !run_at_present || !run_at_valid
}

/// Next-run resolution after success: `"null"` (disabled cron), `"keep"`
/// (activation pending), or `"computed"`
/// (TS: `handleSuccessfulExecution` / `handleFailedExecution` mapping).
pub fn next_run_after_success(
    kind: &str,
    enabled: bool,
    activation_pending: bool,
) -> &'static str {
    // TS success: `kind === "cron" && enabled === false ? null
    //   : kind === "cron" && activationPending ? current.nextRunAt : nextRunAt`.
    // A `once` success removes the task before this mapping applies; it falls
    // through to `"computed"` here (uncalled in practice).
    if kind == "cron" && !enabled {
        "null"
    } else if kind == "cron" && activation_pending {
        "keep"
    } else {
        "computed"
    }
}

/// Next-run resolution after error: `"null"` (once/disabled), `"keep"`
/// (activation pending), or `"computed"` (cron).
pub fn next_run_after_error(kind: &str, enabled: bool, activation_pending: bool) -> &'static str {
    // TS error: only cron tasks compute a next run (`let nextRunAt = null;
    // if (kind === "cron") nextRunAt = compute...`), then the same
    // disabled/activationPending override applies.
    if kind != "cron" || !enabled {
        "null"
    } else if activation_pending {
        "keep"
    } else {
        "computed"
    }
}

/// True for `kind === "once"` (TS: remove + enqueue delivery, no reschedule).
pub fn should_remove_task_after_success(kind: &str) -> bool {
    kind == "once"
}

/// True when `flushDeferredDeliveries` must return early
/// (TS: in-progress, no api/chat, foreground busy, or empty queue).
pub fn should_skip_flush(
    flush_in_progress: bool,
    has_api: bool,
    has_chat: bool,
    is_busy: bool,
    queue_empty: bool,
) -> bool {
    // TS: `flushInProgress || !botApi || chatId === null || isBusy() ||
    //      deliveryQueue.length === 0`; `has_chat` models `chatId !== null`.
    flush_in_progress || !has_api || !has_chat || is_busy || queue_empty
}

/// `None` means start, else the block reason: `"already_running"`,
/// `"missing"`, or `"not_due"` (TS: `startExecution` guards).
pub fn start_execution_blocked(
    already_running: bool,
    has_task: bool,
    due: bool,
) -> Option<&'static str> {
    if already_running {
        return Some("already_running");
    }
    if !has_task {
        return Some("missing");
    }
    if !due {
        return Some("not_due");
    }
    None
}

// ---------------------------------------------------------------------------
// Store pure remainder (`scheduled-task-store.ts`)
// ---------------------------------------------------------------------------

/// `nextRunAt` after `toggleScheduledTask`: `None` on disable, else recompute
/// (TS: `nextRunAt: enabled ? computeNextRunAt(task) : null`).
pub fn cron_toggle_next_run_is_null(enabled: bool) -> bool {
    !enabled
}

/// True when `removeScheduledTask` actually dropped a row
/// (TS: `nextTasks.length !== tasks.length`).
pub fn store_remove_changed(before_len: usize, after_len: usize) -> bool {
    before_len != after_len
}

// ---------------------------------------------------------------------------
// Run-once gates (`scheduled-task-next-run-service.ts`)
// ---------------------------------------------------------------------------

/// True for a once task whose `runAt` is missing or at/before `from`
/// (TS: `computeNextRunAt` once branch returns `null`).
pub fn once_next_run_is_null(run_at_ms: Option<i64>, from_ms: i64) -> bool {
    // TS: `if (Number.isNaN(runAtMs) || runAtMs <= fromDate.getTime()) return null;`
    // `None` models NaN/missing.
    match run_at_ms {
        None => true,
        Some(run_at) => run_at <= from_ms,
    }
}

/// Disabled cron never due; else `nextRunAt <= now`
/// (TS: `isTaskDue` with the `enabled === false` gate).
pub fn is_cron_task_due(enabled: bool, next_run_ms: Option<i64>, now_ms: i64) -> bool {
    if !enabled {
        return false;
    }
    match next_run_ms {
        None => false,
        Some(next_run) => next_run <= now_ms,
    }
}

// ---------------------------------------------------------------------------
// Executor pure remainder (`scheduled-task-executor-service.ts`)
// ---------------------------------------------------------------------------

/// Join non-ignored `text` parts and trim (TS: `collectResponseText`).
///
/// Each tuple is `(part_type, text, ignored)`; only `type === "text"` parts
/// that are not ignored contribute, joined with `""` then trimmed.
pub fn collect_response_text(parts: &[(&str, &str, bool)]) -> String {
    parts
        .iter()
        .filter(|(kind, _, ignored)| *kind == "text" && !ignored)
        .map(|(_, text, _)| *text)
        .collect::<String>()
        .trim()
        .to_string()
}

/// `/timed out|timeout|time out|deadline exceeded|request aborted/i`
/// (TS: `isTimeoutErrorMessage`).
pub fn is_timeout_error_message(msg: &str) -> bool {
    // Deviation: TS uses the regex `i` flag; lowercasing first is equivalent
    // for these ASCII-only alternatives.
    let lower = msg.to_lowercase();
    lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("time out")
        || lower.contains("deadline exceeded")
        || lower.contains("request aborted")
}

/// `startsWith(EXECUTION_TIMEOUT_ERROR_PREFIX)`
/// (TS: `isBotExecutionTimeoutMessage`).
pub fn is_bot_execution_timeout_message(msg: &str) -> bool {
    msg.starts_with(EXECUTION_TIMEOUT_ERROR_PREFIX)
}

/// ``format: `{prefix} after {minutes} minutes.`` (TS:
/// `createExecutionTimeoutMessage`).
pub fn create_execution_timeout_message(minutes: u64) -> String {
    // TS reads the minutes from config; the parameter carries that value here.
    format!("{EXECUTION_TIMEOUT_ERROR_PREFIX} after {minutes} minutes.")
}

/// `minutes * 60 * 1000` (TS: `getExecutionTimeoutMs`).
pub fn execution_timeout_ms(minutes: u64) -> u64 {
    minutes * 60 * 1000
}

/// Pass through bot-timeout/known-timeout+docs messages, else append the
/// models docs URL (TS: `normalizeScheduledTaskErrorMessage`).
pub fn normalize_scheduled_task_error(msg: &str) -> String {
    if is_bot_execution_timeout_message(msg)
        || !is_timeout_error_message(msg)
        || msg.contains(MODELS_DOCS_URL)
    {
        return msg.to_string();
    }
    format!("{msg} Check OpenCode model timeout settings: {MODELS_DOCS_URL}")
}

/// `step-finish` reason wins, else `info.finish`; blank means `None`
/// (TS: `getAssistantFinishReason`).
pub fn finish_reason_from_parts(
    step_reason: Option<&str>,
    info_finish: Option<&str>,
) -> Option<String> {
    if let Some(reason) = step_reason {
        let trimmed = reason.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(finish) = info_finish {
        let trimmed = finish.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// A completed turn with `finish === "tool-calls"` still awaits the final
/// reply (TS: `awaitingToolCalls` gate in `extractAssistantResult`).
pub fn assistant_completed_is_awaiting_tool(completed: bool, finish_reason: Option<&str>) -> bool {
    // TS compares an already-trimmed reason; trim here too for parity with
    // `getAssistantFinishReason` output.
    completed && finish_reason.map(str::trim) == Some("tool-calls")
}

/// True when empty-completed reads exceed the recheck budget
/// (TS: `completedEmptyResultReadCount > MAX_COMPLETED_EMPTY_RESULT_RECHECKS`).
pub fn should_throw_empty_after_rechecks(read_count: u32) -> bool {
    read_count > MAX_COMPLETED_EMPTY_RESULT_RECHECKS
}

/// True when idle polls reach the budget (TS: `>= MAX_IDLE_POLLS...`).
pub fn should_throw_idle_timeout(idle_polls: u32) -> bool {
    idle_polls >= MAX_IDLE_POLLS_WITHOUT_RESULT
}

/// True when startup polls reach the budget (TS: `>= MAX_STARTUP_POLLS...`).
pub fn should_throw_startup_timeout(startup_polls: u32) -> bool {
    startup_polls >= MAX_STARTUP_POLLS_WITHOUT_ACTIVITY
}

/// True when the status map has the session and it is not `idle`
/// (TS: `sessionStatus !== undefined && type !== "idle"`).
pub fn session_is_active(present: bool, status_type: &str) -> bool {
    present && status_type != "idle"
}

/// True only when the pending request belongs to this session
/// (TS: `request.sessionID === sessionId`).
pub fn interactive_request_matches(request_session: &str, current_session: &str) -> bool {
    request_session == current_session
}

// ---------------------------------------------------------------------------
// Schedule-parser pure remainder
// (`scheduled-task-schedule-parser-service.ts`)
// ---------------------------------------------------------------------------

/// Fenced payload is tried first when a ``` fence exists
/// (TS: `extractJsonPayload` `directCandidates.unshift(fenced)`).
pub fn fenced_candidate_is_first(has_fence: bool) -> bool {
    has_fence
}

/// Brace slice candidate exists when `first >= 0 && last > first`
/// (TS: `firstBrace` / `lastBrace` gate).
pub fn brace_slice_present(first: i32, last: i32) -> bool {
    first >= 0 && last > first
}

/// `payload.error` short string fails the parse with its message
/// (TS: `parseSchedulePayload` error passthrough).
pub fn is_schedule_error_payload(has_error_string: bool, trimmed_nonempty: bool) -> bool {
    // TS: `typeof payload.error === "string" && payload.error.trim()`.
    has_error_string && trimmed_nonempty
}

/// Non-blank summary required (TS: `Schedule summary is missing`).
pub fn is_valid_schedule_summary(summary: Option<&str>) -> bool {
    match summary {
        None => false,
        Some(text) => !text.trim().is_empty(),
    }
}

/// Only `"cron"` / `"once"` accepted (TS: `Schedule kind is invalid`).
pub fn is_valid_schedule_kind(kind: &str) -> bool {
    kind == "cron" || kind == "once"
}

/// Cron schedules need a non-blank `cron` expression
/// (TS: `Schedule cron expression is missing`).
pub fn schedule_cron_missing(cron: Option<&str>) -> bool {
    match cron {
        None => true,
        Some(expr) => expr.trim().is_empty(),
    }
}

/// Once schedules need a valid `runAt` datetime
/// (TS: `Schedule runAt is invalid`).
pub fn schedule_run_at_invalid(run_at_valid: bool) -> bool {
    !run_at_valid
}

/// Model is sent only when both ids are set
/// (TS: `if (model?.providerID && model.modelID)`).
pub fn should_include_model_in_prompt(has_provider: bool, has_model: bool) -> bool {
    has_provider && has_model
}

/// Variant is sent only when set (TS: `if (model?.variant)`).
pub fn should_include_variant_in_prompt(has_variant: bool) -> bool {
    has_variant
}
