//! Task + model-search text-input routing gates (slice 113, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/commands/task-command.ts` (`handleTaskTextInput` early
//!   routing: `text === undefined || text.startsWith("/")` miss,
//!   `!taskCreationManager.isActive()` miss, `!isTaskInteraction(...)` /
//!   `!flowState` inactive (`task.inactive`) handling, `isParsingSchedule()`
//!   busy (`task.parse.in_progress`) handling, `isWaitingForSchedule()` blank
//!   (`task.schedule_empty`) vs parse handling, `!isWaitingForPrompt()`
//!   miss, blank prompt (`task.prompt_empty`) handling, missing
//!   `parsedSchedule || scheduleText` inactive (`task.inactive`) handling)
//! - `src/bot/callbacks/model-selection-callback-handler.ts`
//!   (`handleModelSearchTextInput` routing: missing/non-`input`-stage
//!   metadata miss, `text === undefined` miss, blank-trim swallow without
//!   search, non-blank search execution)
//!
//! And its cases in:
//! - `tests/bot/commands/task.test.ts` (`handleTaskTextInput` schedule parse,
//!   prompt save, limit, retry, cancel, frequency, empty branches)
//! - `tests/bot/handlers/model.test.ts` (`handleModelSearchTextInput`:
//!   no interaction / wrong flow / wrong stage / no text miss, blank swallow,
//!   non-blank search execution)
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity
//! tests in `crates/otb-core/tests/slice113_task_model_text_input_parity.rs`
//! assert the TS contract.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx` / `Api` I/O (`reply`, `deleteMessageIfPresent`,
//!   `searchModels`, `buildModelSearchResultsView`,
//!   `transitionToModelSearchResults`), `taskCreationManager` /
//!   `interactionManager` reads/writes (`isActive`, `isParsingSchedule`,
//!   `isWaitingForSchedule`, `isWaitingForPrompt`, `getState`,
//!   `markScheduleParsing`, `setParsedSchedule`, transitions, `clear`),
//!   `parseTaskSchedule` / `validateParsedSchedule` / `buildScheduledTask` /
//!   `addScheduledTask` / `registerTask` orchestration, `isTaskLimitReached`
//!   store reads, `t(...)` locale lookups (keys travel as literals here).
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `isTaskInteraction` / `isTaskCallbackActive` / task-callback dispatch
//!   (`task_callback_gates` owner)
//! - task-creation manager state machine (`task_creation_gates` owner)
//! - model-search callback constants / results dispatch
//!   (`agent_model_variant_gates` owner)
//! - `truncateTaskPrompt` / `formatTaskCreatedMessage` composition
//!   (localized message bodies, caller-owned).
//!
//! Deviations: none intended. `None` text counts as ineligible/miss (TS
//! `text === undefined`); `"/"`-prefixed text counts as ineligible on the
//! task path (TS `text.startsWith("/")`); whitespace-only text counts as
//! blank (TS `text.trim()` empty).

/// Reply key when the task flow has no usable state
/// (TS: `t("task.inactive")` in `handleTaskTextInput`).
pub const TASK_INACTIVE_KEY: &str = "task.inactive";

/// Reply key while a schedule parse is already running
/// (TS: `t("task.parse.in_progress")` in `handleTaskTextInput`).
pub const TASK_PARSE_IN_PROGRESS_KEY: &str = "task.parse.in_progress";

/// Reply key for blank schedule text while waiting for a schedule
/// (TS: `t("task.schedule_empty")` in `handleTaskTextInput`).
pub const TASK_SCHEDULE_EMPTY_KEY: &str = "task.schedule_empty";

/// Reply key for blank prompt text while waiting for a prompt
/// (TS: `t("task.prompt_empty")` in `handleTaskTextInput`).
pub const TASK_PROMPT_EMPTY_KEY: &str = "task.prompt_empty";

/// `true` when the task text path can be considered at all
/// (TS: `text !== undefined && !text.startsWith("/")`; otherwise `false`).
pub fn is_task_text_eligible(text: Option<&str>) -> bool {
    matches!(text, Some(t) if !t.starts_with('/'))
}

/// `true` when the task text path runs past the manager gate
/// (TS: eligible text plus `taskCreationManager.isActive()`; otherwise
/// `false`).
pub fn should_attempt_task_text(eligible: bool, manager_active: bool) -> bool {
    eligible && manager_active
}

/// `true` when the attempt answers `task.inactive` for missing flow state
/// (TS: `!isTaskInteraction(...) || !flowState` — clears and returns
/// `true` with the inactive reply).
pub fn should_report_task_inactive(is_task_interaction: bool, has_flow_state: bool) -> bool {
    !is_task_interaction || !has_flow_state
}

/// `true` when the attempt answers `task.parse.in_progress` and stops
/// (TS: `isParsingSchedule()` path returning `true`).
pub fn should_report_task_parse_in_progress(is_parsing_schedule: bool) -> bool {
    is_parsing_schedule
}

/// `true` when the schedule step answers `task.schedule_empty` and stops
/// (TS: `isWaitingForSchedule()` with blank `text.trim()`).
pub fn should_report_empty_schedule(is_waiting_for_schedule: bool, schedule_empty: bool) -> bool {
    is_waiting_for_schedule && schedule_empty
}

/// `true` when the schedule step starts parsing
/// (TS: `isWaitingForSchedule()` with non-blank schedule text).
pub fn should_start_schedule_parse(is_waiting_for_schedule: bool, schedule_empty: bool) -> bool {
    is_waiting_for_schedule && !schedule_empty
}

/// `true` when the prompt step answers `task.prompt_empty` and stops
/// (TS: `isWaitingForPrompt()` with blank `text.trim()`).
pub fn should_report_empty_prompt(is_waiting_for_prompt: bool, prompt_empty: bool) -> bool {
    is_waiting_for_prompt && prompt_empty
}

/// `true` when the prompt step answers `task.inactive` for a missing
/// schedule before saving (TS: `!flowState.parsedSchedule ||
/// !flowState.scheduleText` — clears and returns `true`).
pub fn should_report_task_inactive_before_save(
    has_schedule_text: bool,
    has_parsed_schedule: bool,
) -> bool {
    !has_schedule_text || !has_parsed_schedule
}

/// `true` when model-search text routing runs at all
/// (TS: metadata present plus `stage === "input"` plus
/// `text !== undefined`; otherwise `false`).
pub fn should_attempt_model_search_text(
    has_metadata: bool,
    is_input_stage: bool,
    text_present: bool,
) -> bool {
    has_metadata && is_input_stage && text_present
}

/// `true` when the attempt swallows blank text and stops without searching
/// (TS: `!text.trim()` path returning `true`).
pub fn should_swallow_blank_model_search(attempt: bool, text_blank: bool) -> bool {
    attempt && text_blank
}

/// `true` when the attempt runs the model search
/// (TS: attempted with non-blank text — `searchModels` path).
pub fn should_execute_model_search(attempt: bool, text_blank: bool) -> bool {
    attempt && !text_blank
}
