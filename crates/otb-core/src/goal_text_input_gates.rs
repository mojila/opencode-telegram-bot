//! Goal text-input routing gates (slice 114, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/commands/goal-command.ts` (`handleGoalTextInput` early
//!   routing: non-`goal` flow or non-`objective` stage miss, photo / voice
//!   / audio / document / `getIncomingPrompt` photos / rich skipped-media
//!   block (`interaction.blocked.expected_text`), `text === undefined` or
//!   `text.startsWith("/")` miss, `parseGoalObjective` kind dispatch
//!   (`empty` -> `goal.input.empty`, `reserved` ->
//!   `goal.input.reserved`, `too_long` -> `formatObjectiveTooLong`
//!   (`goal.too_long`), `objective` -> confirmation path))
//!
//! And its cases in:
//! - `tests/bot/commands/goal.test.ts` (`handleGoalTextInput` invalid input
//!   stays on `objective` without a server call, voice/media refusal,
//!   `"/..."` and non-goal flow miss)
//! - `src/app/types/goal.ts` (`parseGoalObjective` empty / reserved /
//!   `too_long` / `objective` kinds; the trim/quote/reserved/limit rules
//!   themselves live in `session_gates` and are NOT re-tested here)
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity
//! tests in `crates/otb-core/tests/slice114_goal_text_input_parity.rs`
//! assert the TS contract.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx` / `Api` I/O (`reply`, `replyGoalText`,
//!   `interactionManager.start` / `transition` / `clear`,
//!   `convertRichMessage`, `getIncomingPrompt`, `goalService.read`,
//!   `openGoalConfirmation`), `parseGoalObjective` execution (its kinds
//!   arrive here as [`GoalObjectiveKind`]), `formatObjectiveTooLong`
//!   composition, `t(...)` locale lookups (keys travel as literals here).
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `parseGoalObjective` trim/quote/reserved/limit rules
//!   (`session_gates` owner)
//! - `goal:confirm` / `goal:cancel` / `goal:delete:` values and the
//!   confirmation/callback dispatch (`goal_callback_gates` owner)
//! - goal manager / service state machines
//!   (`goal_manager_gates` / `goal_service_gates` owners)
//! - `GOAL_OBJECTIVE_LIMIT` (`session_gates` owner)
//! - `formatObjectiveTooLong` / `formatGoalConfirmation` composition
//!   (`goal_reason_gates` owner).
//!
//! Deviations: none intended. `None` text counts as ineligible/miss (TS
//! `text === undefined`); `"/"`-prefixed text counts as ineligible on the
//! goal path (TS `text.startsWith("/")`); `flow` / `stage` arrive as the
//! already-read `metadata.flow` / `metadata.stage` strings (`None` models a
//! missing/non-string value, which misses like any non-`goal` /
//! non-`objective` value).

/// Reply key when goal objective input receives media instead of text
/// (TS: `t("interaction.blocked.expected_text")` in `handleGoalTextInput`).
pub const GOAL_EXPECTED_TEXT_KEY: &str = "interaction.blocked.expected_text";

/// Reply key for blank/quoted-blank goal input
/// (TS: `t("goal.input.empty")` in `handleGoalTextInput`).
pub const GOAL_INPUT_EMPTY_KEY: &str = "goal.input.empty";

/// Reply key for reserved-word goal input (`pause` / `resume` / `clear`)
/// (TS: `t("goal.input.reserved")` in `handleGoalTextInput`).
pub const GOAL_INPUT_RESERVED_KEY: &str = "goal.input.reserved";

/// Reply key for over-limit goal input
/// (TS: `t("goal.too_long")` via `formatObjectiveTooLong`).
pub const GOAL_TOO_LONG_KEY: &str = "goal.too_long";

/// `parseGoalObjective` outcome (TS: `GoalObjective["kind"]`
/// in `src/app/types/goal.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalObjectiveKind {
    Empty,
    Reserved,
    Objective,
    TooLong,
}

/// `true` when the goal text path can be considered at all
/// (TS: `metadata.flow === "goal" && metadata.stage === "objective"`;
/// otherwise `false`).
pub fn is_goal_objective_flow(flow: Option<&str>, stage: Option<&str>) -> bool {
    flow == Some("goal") && stage == Some("objective")
}

/// `true` when media was attached to the objective message
/// (TS: `message.photo.length || message.voice || message.audio ||
/// message.document || getIncomingPrompt(ctx).photos.length ||
/// rich skippedMediaCount`; any present blocks with `expected_text`).
pub fn should_block_goal_media(
    has_photo: bool,
    has_voice: bool,
    has_audio: bool,
    has_document: bool,
    has_prompt_photos: bool,
    has_skipped_media: bool,
) -> bool {
    has_photo || has_voice || has_audio || has_document || has_prompt_photos || has_skipped_media
}

/// `true` when the goal text itself is routable
/// (TS: `text !== undefined && !text.startsWith("/")`; otherwise `false`).
pub fn is_goal_text_eligible(text: Option<&str>) -> bool {
    match text {
        Some(t) => !t.starts_with('/'),
        None => false,
    }
}

/// `true` when the attempt runs past the flow + text gates
/// (TS: objective flow plus eligible text; otherwise `false`).
pub fn should_attempt_goal_text(is_flow: bool, eligible: bool) -> bool {
    is_flow && eligible
}

/// `true` when the attempt answers `interaction.blocked.expected_text`
/// and stops (TS: objective flow with attached media).
pub fn should_report_goal_expected_text(is_flow: bool, has_media: bool) -> bool {
    is_flow && has_media
}

/// `true` when the attempt answers `goal.input.empty` and stops
/// (TS: `parseGoalObjective` returns `{ kind: "empty" }`).
pub fn should_report_goal_empty(kind: GoalObjectiveKind) -> bool {
    kind == GoalObjectiveKind::Empty
}

/// `true` when the attempt answers `goal.input.reserved` and stops
/// (TS: `parseGoalObjective` returns `{ kind: "reserved" }`).
pub fn should_report_goal_reserved(kind: GoalObjectiveKind) -> bool {
    kind == GoalObjectiveKind::Reserved
}

/// `true` when the attempt answers `goal.too_long` and stops
/// (TS: `parseGoalObjective` returns `{ kind: "too_long" }`).
pub fn should_report_goal_too_long(kind: GoalObjectiveKind) -> bool {
    kind == GoalObjectiveKind::TooLong
}

/// `true` when the attempt leaves `objective` stage toward confirmation
/// (TS: `parseGoalObjective` returns `{ kind: "objective" }`).
pub fn should_accept_goal_objective(kind: GoalObjectiveKind) -> bool {
    kind == GoalObjectiveKind::Objective
}
