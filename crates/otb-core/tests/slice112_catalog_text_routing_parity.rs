//! Slice 112 parity: catalog text-arguments routing gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/handlers/text-message-handler.ts`
//! (`handleCommandTextArguments` / `handleSkillTextArguments` /
//! `handleCatalogTextArguments`) and its cases in
//! `tests/bot/commands/commands.test.ts` /
//! `tests/bot/commands/skills.test.ts`:
//! - non-command text with `confirm`-stage metadata attempts the path;
//!   `undefined` / `"/..."` text or a missing/non-`confirm` metadata
//!   returns `false` without a reply
//! - blank trimmed arguments answer `commands.arguments_empty` (command
//!   flow) or `skills.arguments_empty` (skill flow) and return `true`
//!   without deleting or executing
//! - non-empty arguments clear, delete the prompt message when there is
//!   a chat, execute, and return `true`
//! - `handleCatalogTextArguments` tries skill args only when command
//!   args did not handle the text
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `catalog_text_routing_gates` is ported (only constant-value tests pass).

use otb_core::catalog_text_routing_gates::*;

// --- reply keys (2) ---

#[test]
fn slice112_01_command_empty_key_is_commands_arguments_empty() {
    assert_eq!(COMMAND_ARGUMENTS_EMPTY_KEY, "commands.arguments_empty");
}

#[test]
fn slice112_02_skill_empty_key_is_skills_arguments_empty() {
    assert_eq!(SKILL_ARGUMENTS_EMPTY_KEY, "skills.arguments_empty");
}

// --- select_empty_arguments_key (2) ---

#[test]
fn slice112_03_command_flow_selects_commands_key() {
    assert_eq!(select_empty_arguments_key(true), "commands.arguments_empty");
}

#[test]
fn slice112_04_skill_flow_selects_skills_key() {
    assert_eq!(select_empty_arguments_key(false), "skills.arguments_empty");
}

// --- should_attempt_catalog_text_args (3) ---

#[test]
fn slice112_05_eligible_confirm_attempts() {
    assert!(should_attempt_catalog_text_args(true, true));
}

#[test]
fn slice112_06_ineligible_text_never_attempts() {
    assert!(!should_attempt_catalog_text_args(false, true));
    assert!(!should_attempt_catalog_text_args(false, false));
}

#[test]
fn slice112_07_eligible_non_confirm_skips() {
    assert!(!should_attempt_catalog_text_args(true, false));
}

// --- should_report_empty_catalog_args (2) ---

#[test]
fn slice112_08_attempt_with_empty_args_reports() {
    assert!(should_report_empty_catalog_args(true, true));
}

#[test]
fn slice112_09_no_report_without_attempt_or_with_args() {
    assert!(!should_report_empty_catalog_args(false, true));
    assert!(!should_report_empty_catalog_args(true, false));
    assert!(!should_report_empty_catalog_args(false, false));
}

// --- should_execute_catalog_command (2) ---

#[test]
fn slice112_10_attempt_with_args_executes() {
    assert!(should_execute_catalog_command(true, false));
}

#[test]
fn slice112_11_no_execute_without_attempt_or_with_empty_args() {
    assert!(!should_execute_catalog_command(false, false));
    assert!(!should_execute_catalog_command(true, true));
    assert!(!should_execute_catalog_command(false, true));
}

// --- should_delete_catalog_prompt_message (2) ---

#[test]
fn slice112_12_execute_with_chat_deletes() {
    assert!(should_delete_catalog_prompt_message(true, true));
}

#[test]
fn slice112_13_no_delete_without_execute_or_chat() {
    assert!(!should_delete_catalog_prompt_message(true, false));
    assert!(!should_delete_catalog_prompt_message(false, true));
    assert!(!should_delete_catalog_prompt_message(false, false));
}

// --- should_try_skill_text_args (2) ---

#[test]
fn slice112_14_skill_tried_only_when_command_missed() {
    assert!(should_try_skill_text_args(false));
}

#[test]
fn slice112_15_skill_skipped_when_command_handled() {
    assert!(!should_try_skill_text_args(true));
}

// --- catalog_text_handled (3) ---

#[test]
fn slice112_16_command_handled_wins() {
    assert!(catalog_text_handled(true, false));
    assert!(catalog_text_handled(true, true));
}

#[test]
fn slice112_17_skill_handled_counts_when_command_missed() {
    assert!(catalog_text_handled(false, true));
}

#[test]
fn slice112_18_neither_handled_is_unhandled() {
    assert!(!catalog_text_handled(false, false));
}

// --- end-to-end routing table mirroring the TS handler tests (2) ---

#[test]
fn slice112_19_blank_command_args_report_without_execute_or_delete() {
    // TS `commands.test.ts`: `"about spring"`-style confirm flow with
    // whitespace-only text answers `commands.arguments_empty`, returns
    // `true`, never deletes or executes.
    let attempt = should_attempt_catalog_text_args(true, true);
    assert!(attempt);
    assert!(should_report_empty_catalog_args(attempt, true));
    assert!(!should_execute_catalog_command(attempt, true));
    assert!(!should_delete_catalog_prompt_message(
        should_execute_catalog_command(attempt, true),
        true
    ));
    assert_eq!(
        select_empty_arguments_key(true),
        COMMAND_ARGUMENTS_EMPTY_KEY
    );
}

#[test]
fn slice112_20_catalog_falls_through_to_skill_result() {
    // TS `text-message-handler.ts`: `handleCatalogTextArguments` returns
    // the command result when handled, otherwise the skill result.
    assert!(catalog_text_handled(true, should_try_skill_text_args(true) && false));
    assert!(catalog_text_handled(false, true));
    assert!(!catalog_text_handled(false, false));
    assert!(should_try_skill_text_args(false));
}
