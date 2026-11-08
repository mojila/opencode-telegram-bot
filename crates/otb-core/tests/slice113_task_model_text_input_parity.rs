//! Slice 113 parity: task + model-search text-input routing gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/commands/task-command.ts` (`handleTaskTextInput` early routing)
//! and `src/bot/callbacks/model-selection-callback-handler.ts`
//! (`handleModelSearchTextInput` routing) and its cases in
//! `tests/bot/commands/task.test.ts` / `tests/bot/handlers/model.test.ts`:
//! - task text `undefined` / `"/..."` never attempts; inactive manager never
//!   attempts
//! - attempted with non-task interaction or missing flow state reports
//!   `task.inactive`
//! - parsing schedule reports `task.parse.in_progress`
//! - waiting-for-schedule with blank text reports `task.schedule_empty`,
//!   with text starts the parse
//! - waiting-for-prompt with blank text reports `task.prompt_empty`
//! - prompt save without schedule text/parsed schedule reports
//!   `task.inactive`
//! - model-search text runs only with metadata + `input` stage + text;
//!   blank text is swallowed without search, non-blank text executes
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `task_model_text_input_gates` is ported (only constant-value tests pass).

use otb_core::task_model_text_input_gates::*;

// --- reply keys (4) ---

#[test]
fn slice113_01_task_inactive_key_is_task_inactive() {
    assert_eq!(TASK_INACTIVE_KEY, "task.inactive");
}

#[test]
fn slice113_02_task_parse_key_is_task_parse_in_progress() {
    assert_eq!(TASK_PARSE_IN_PROGRESS_KEY, "task.parse.in_progress");
}

#[test]
fn slice113_03_task_schedule_empty_key_is_task_schedule_empty() {
    assert_eq!(TASK_SCHEDULE_EMPTY_KEY, "task.schedule_empty");
}

#[test]
fn slice113_04_task_prompt_empty_key_is_task_prompt_empty() {
    assert_eq!(TASK_PROMPT_EMPTY_KEY, "task.prompt_empty");
}

// --- is_task_text_eligible (2) ---

#[test]
fn slice113_05_task_text_undefined_or_slash_is_ineligible() {
    assert!(!is_task_text_eligible(None));
    assert!(!is_task_text_eligible(Some("/task")));
}

#[test]
fn slice113_06_plain_task_text_is_eligible() {
    assert!(is_task_text_eligible(Some("every day at 17:00")));
}

// --- should_attempt_task_text (2) ---

#[test]
fn slice113_07_eligible_and_active_attempts() {
    assert!(should_attempt_task_text(true, true));
}

#[test]
fn slice113_08_ineligible_or_inactive_never_attempts() {
    assert!(!should_attempt_task_text(false, true));
    assert!(!should_attempt_task_text(true, false));
    assert!(!should_attempt_task_text(false, false));
}

// --- should_report_task_inactive (2) ---

#[test]
fn slice113_09_non_task_interaction_or_missing_state_reports_inactive() {
    assert!(should_report_task_inactive(false, true));
    assert!(should_report_task_inactive(true, false));
    assert!(should_report_task_inactive(false, false));
}

#[test]
fn slice113_10_live_task_flow_does_not_report_inactive() {
    assert!(!should_report_task_inactive(true, true));
}

// --- should_report_task_parse_in_progress (2) ---

#[test]
fn slice113_11_parsing_schedule_reports_in_progress() {
    assert!(should_report_task_parse_in_progress(true));
}

#[test]
fn slice113_12_idle_schedule_does_not_report_in_progress() {
    assert!(!should_report_task_parse_in_progress(false));
}

// --- should_report_empty_schedule / should_start_schedule_parse (2) ---

#[test]
fn slice113_13_waiting_with_blank_schedule_reports_empty() {
    assert!(should_report_empty_schedule(true, true));
    assert!(!should_start_schedule_parse(true, true));
}

#[test]
fn slice113_14_waiting_with_text_starts_parse() {
    assert!(should_start_schedule_parse(true, false));
    assert!(!should_report_empty_schedule(true, false));
    assert!(!should_report_empty_schedule(false, true));
    assert!(!should_start_schedule_parse(false, false));
}

// --- should_report_empty_prompt (2) ---

#[test]
fn slice113_15_waiting_with_blank_prompt_reports_empty() {
    assert!(should_report_empty_prompt(true, true));
}

#[test]
fn slice113_16_no_report_without_wait_or_with_text() {
    assert!(!should_report_empty_prompt(false, true));
    assert!(!should_report_empty_prompt(true, false));
    assert!(!should_report_empty_prompt(false, false));
}

// --- should_report_task_inactive_before_save (2) ---

#[test]
fn slice113_17_prompt_save_without_schedule_reports_inactive() {
    assert!(should_report_task_inactive_before_save(false, true));
    assert!(should_report_task_inactive_before_save(true, false));
    assert!(should_report_task_inactive_before_save(false, false));
}

#[test]
fn slice113_18_prompt_save_with_schedule_does_not_report_inactive() {
    assert!(!should_report_task_inactive_before_save(true, true));
}

// --- should_attempt_model_search_text (3) ---

#[test]
fn slice113_19_model_search_runs_with_meta_input_and_text() {
    assert!(should_attempt_model_search_text(true, true, true));
}

#[test]
fn slice113_20_model_search_misses_without_meta_or_stage_or_text() {
    assert!(!should_attempt_model_search_text(false, true, true));
    assert!(!should_attempt_model_search_text(true, false, true));
    assert!(!should_attempt_model_search_text(true, true, false));
    assert!(!should_attempt_model_search_text(false, false, false));
}

// --- should_swallow_blank_model_search / should_execute_model_search (2) ---

#[test]
fn slice113_21_blank_model_search_swallows_without_execute() {
    assert!(should_swallow_blank_model_search(true, true));
    assert!(!should_execute_model_search(true, true));
}

#[test]
fn slice113_22_non_blank_model_search_executes_without_swallow() {
    assert!(should_execute_model_search(true, false));
    assert!(!should_swallow_blank_model_search(true, false));
    assert!(!should_swallow_blank_model_search(false, true));
    assert!(!should_execute_model_search(false, false));
}
