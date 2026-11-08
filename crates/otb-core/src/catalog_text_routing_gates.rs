//! Catalog text-arguments routing gates (slice 112, GREEN).
//!
//! Rust port of the pure remainder in
//! `src/bot/handlers/text-message-handler.ts`
//! (`handleCommandTextArguments` / `handleSkillTextArguments` /
//! `handleCatalogTextArguments` routing):
//! - empty-arguments reply keys (`commands.arguments_empty` vs
//!   `skills.arguments_empty`)
//! - attempt gate: eligible non-command text plus `confirm`-stage metadata
//!   (eligibility and stage checks themselves owned by
//!   `catalog_callback_gates`, composed here)
//! - empty-arguments report vs execute branching (`text.trim()` empty
//!   answers the `arguments_empty` key and returns `true` without the
//!   `deleteMessage` / `execute` path)
//! - `ctx.chat` delete-message gate on the execute path
//! - command-first routing: skill args are tried only when command args
//!   did not handle the text; the catalog result is handled when either
//!   side handled it
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity
//! tests in `crates/otb-core/tests/slice112_catalog_text_routing_parity.rs`
//! assert the TS contract.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx` / `Api` I/O (`reply`, `deleteMessage`, `executeCommand` /
//!   `executeSkill`), `parseCommandsMetadata` / `parseSkillsMetadata`
//!   snapshot reads, `clearCommandsInteraction` / `clearSkillsInteraction`
//!   transitions, `t(...)` locale lookups (keys travel as literals here).
//!
//! Already ported elsewhere and NOT re-tested here:
//! `is_catalog_text_eligible` / `is_confirm_stage` / `is_arguments_empty` /
//! `trim_arguments` / `should_clear_catalog_interaction`
//! (`catalog_callback_gates` owner).
//!
//! Deviations: none intended. `None` text counts as ineligible (TS
//! `text === undefined`); `"/"`-prefixed text counts as ineligible (TS
//! `text.startsWith("/")`).

/// Reply key when command arguments are blank
/// (TS: `t("commands.arguments_empty")` in `handleCommandTextArguments`).
pub const COMMAND_ARGUMENTS_EMPTY_KEY: &str = "commands.arguments_empty";

/// Reply key when skill arguments are blank
/// (TS: `t("skills.arguments_empty")` in `handleSkillTextArguments`).
pub const SKILL_ARGUMENTS_EMPTY_KEY: &str = "skills.arguments_empty";

/// Empty-arguments key for the flow being handled
/// (TS: `commands.arguments_empty` on the command path,
/// `skills.arguments_empty` on the skill path).
pub fn select_empty_arguments_key(is_command_flow: bool) -> &'static str {
    if is_command_flow {
        COMMAND_ARGUMENTS_EMPTY_KEY
    } else {
        SKILL_ARGUMENTS_EMPTY_KEY
    }
}

/// `true` when the text-arguments path runs at all
/// (TS: eligible text — not `undefined`, not `"/..."` — plus
/// `metadata && metadata.stage === "confirm"`; otherwise `false`).
pub fn should_attempt_catalog_text_args(text_eligible: bool, is_confirm_stage: bool) -> bool {
    text_eligible && is_confirm_stage
}

/// `true` when the attempt answers the empty-arguments reply and stops
/// (TS: `if (!argumentsText)` after `text.trim()` — returns `true`
/// without deleting or executing).
pub fn should_report_empty_catalog_args(attempt: bool, arguments_empty: bool) -> bool {
    attempt && arguments_empty
}

/// `true` when the attempt clears the interaction, deletes the prompt
/// message (when there is a chat), and executes
/// (TS: non-empty `argumentsText` path returning `true`).
pub fn should_execute_catalog_command(attempt: bool, arguments_empty: bool) -> bool {
    attempt && !arguments_empty
}

/// `true` when the execute path deletes the catalog prompt message
/// (TS: `if (ctx.chat)` around `deleteMessage` on the execute path;
/// the empty-arguments path never deletes).
pub fn should_delete_catalog_prompt_message(will_execute: bool, has_chat: bool) -> bool {
    will_execute && has_chat
}

/// `true` when skill text arguments are tried
/// (TS: `handleCatalogTextArguments` tries skills only when
/// `handleCommandTextArguments` returned `false`).
pub fn should_try_skill_text_args(command_handled: bool) -> bool {
    !command_handled
}

/// Catalog result: handled when either side handled the text
/// (TS: `handleCatalogTextArguments` returns the command result when
/// `true`, otherwise the skill result).
pub fn catalog_text_handled(command_handled: bool, skill_handled: bool) -> bool {
    command_handled || skill_handled
}
