//! Slice 114 parity: goal text-input routing gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/commands/goal-command.ts` (`handleGoalTextInput` early routing)
//! and its cases in `tests/bot/commands/goal.test.ts`:
//! - non-`goal` flow or non-`objective` stage never attempts
//! - attached photo / voice / audio / document / prompt photos / rich
//!   skipped media blocks with `interaction.blocked.expected_text`
//! - `undefined` / `"/..."` text never attempts
//! - `empty` reports `goal.input.empty`, `reserved` reports
//!   `goal.input.reserved`, `too_long` reports `goal.too_long`,
//!   `objective` proceeds to confirmation
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `goal_text_input_gates` is ported (only constant-value tests pass).

use otb_core::goal_text_input_gates::*;

// --- reply keys (4) ---

#[test]
fn slice114_01_goal_expected_text_key_is_interaction_blocked_expected_text() {
    assert_eq!(GOAL_EXPECTED_TEXT_KEY, "interaction.blocked.expected_text");
}

#[test]
fn slice114_02_goal_input_empty_key_is_goal_input_empty() {
    assert_eq!(GOAL_INPUT_EMPTY_KEY, "goal.input.empty");
}

#[test]
fn slice114_03_goal_input_reserved_key_is_goal_input_reserved() {
    assert_eq!(GOAL_INPUT_RESERVED_KEY, "goal.input.reserved");
}

#[test]
fn slice114_04_goal_too_long_key_is_goal_too_long() {
    assert_eq!(GOAL_TOO_LONG_KEY, "goal.too_long");
}

// --- is_goal_objective_flow (2) ---

#[test]
fn slice114_05_goal_objective_flow_attempts() {
    assert!(is_goal_objective_flow(Some("goal"), Some("objective")));
}

#[test]
fn slice114_06_non_goal_flow_or_stage_misses() {
    assert!(!is_goal_objective_flow(Some("task"), Some("objective")));
    assert!(!is_goal_objective_flow(Some("goal"), Some("confirm")));
    assert!(!is_goal_objective_flow(None, Some("objective")));
    assert!(!is_goal_objective_flow(Some("goal"), None));
    assert!(!is_goal_objective_flow(None, None));
}

// --- should_block_goal_media (2) ---

#[test]
fn slice114_07_any_attached_media_blocks() {
    assert!(should_block_goal_media(true, false, false, false, false, false));
    assert!(should_block_goal_media(false, true, false, false, false, false));
    assert!(should_block_goal_media(false, false, true, false, false, false));
    assert!(should_block_goal_media(false, false, false, true, false, false));
    assert!(should_block_goal_media(false, false, false, false, true, false));
    assert!(should_block_goal_media(false, false, false, false, false, true));
}

#[test]
fn slice114_08_text_only_does_not_block() {
    assert!(!should_block_goal_media(false, false, false, false, false, false));
}

// --- is_goal_text_eligible (2) ---

#[test]
fn slice114_09_goal_text_undefined_or_slash_is_ineligible() {
    assert!(!is_goal_text_eligible(None));
    assert!(!is_goal_text_eligible(Some("/goal")));
}

#[test]
fn slice114_10_plain_goal_text_is_eligible() {
    assert!(is_goal_text_eligible(Some("Fix all tests")));
}

// --- should_attempt_goal_text (2) ---

#[test]
fn slice114_11_flow_and_eligible_text_attempts() {
    assert!(should_attempt_goal_text(true, true));
}

#[test]
fn slice114_12_flow_miss_or_ineligible_text_never_attempts() {
    assert!(!should_attempt_goal_text(false, true));
    assert!(!should_attempt_goal_text(true, false));
    assert!(!should_attempt_goal_text(false, false));
}

// --- should_report_goal_expected_text (2) ---

#[test]
fn slice114_13_flow_with_media_reports_expected_text() {
    assert!(should_report_goal_expected_text(true, true));
}

#[test]
fn slice114_14_no_report_without_flow_or_media() {
    assert!(!should_report_goal_expected_text(false, true));
    assert!(!should_report_goal_expected_text(true, false));
    assert!(!should_report_goal_expected_text(false, false));
}

// --- objective-kind dispatch (4) ---

#[test]
fn slice114_15_empty_reports_empty_only() {
    assert!(should_report_goal_empty(GoalObjectiveKind::Empty));
    assert!(!should_report_goal_empty(GoalObjectiveKind::Reserved));
    assert!(!should_report_goal_empty(GoalObjectiveKind::TooLong));
    assert!(!should_report_goal_empty(GoalObjectiveKind::Objective));
}

#[test]
fn slice114_16_reserved_reports_reserved_only() {
    assert!(should_report_goal_reserved(GoalObjectiveKind::Reserved));
    assert!(!should_report_goal_reserved(GoalObjectiveKind::Empty));
    assert!(!should_report_goal_reserved(GoalObjectiveKind::TooLong));
    assert!(!should_report_goal_reserved(GoalObjectiveKind::Objective));
}

#[test]
fn slice114_17_too_long_reports_too_long_only() {
    assert!(should_report_goal_too_long(GoalObjectiveKind::TooLong));
    assert!(!should_report_goal_too_long(GoalObjectiveKind::Empty));
    assert!(!should_report_goal_too_long(GoalObjectiveKind::Reserved));
    assert!(!should_report_goal_too_long(GoalObjectiveKind::Objective));
}

#[test]
fn slice114_18_objective_accepts_only() {
    assert!(should_accept_goal_objective(GoalObjectiveKind::Objective));
    assert!(!should_accept_goal_objective(GoalObjectiveKind::Empty));
    assert!(!should_accept_goal_objective(GoalObjectiveKind::Reserved));
    assert!(!should_accept_goal_objective(GoalObjectiveKind::TooLong));
}
