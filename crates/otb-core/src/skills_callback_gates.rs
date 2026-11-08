//! Skills catalog callback-handler pure gates (slice 89, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/skills-callback-handler.ts`
//!   (the `skills:` prefix dispatch guard; the cancel vs execute vs
//!   page vs select dispatch branch; the `callbackMessageId === null`
//!   and `metadata.messageId !== callbackMessageId` stale-message gate
//!   (`skills.inactive_callback`); the page-branch `metadata.stage !==
//!   "list"` gate (`callback.processing_error`); the `page >=
//!   totalPages || page < 0` out-of-range gate
//!   (`skills.page_empty_callback`); the `skillIndex === null ||
//!   metadata.stage !== "list"` gate (`callback.processing_error`);
//!   the missing-skill gate (`skills.inactive_callback`); the execute
//!   branch `metadata.stage !== "confirm"` gate
//!   (`skills.inactive_callback`); the `/<skill> <args>` prompt-text
//!   build; the `skills.execute_callback` toast and trailing
//!   `callback.processing_error` catch-all).
//! - `src/bot/menus/skills-catalog-menu.ts` (the `skills:`,
//!   `skills:select:`, `skills:page:` callback constants plus
//!   `skills:cancel` / `skills:execute`; `parseSkillPageCallback` /
//!   `parseSkillSelectCallback`; page/select callback builders).
//!
//! Already ported elsewhere and NOT repeated here:
//! - keyboard layout builders (`buildSkillsListKeyboard`,
//!   `buildSkillsConfirmKeyboard`) and button-label truncation stay with
//!   the renderer (grammy `InlineKeyboard` I/O).
//! - `calculateSkillsPaginationRange` full range math lives with the
//!   menu renderer; only the `totalPages` / normalized-page /
//!   out-of-range predicates are gated here.
//! - `formatExecutingSkillMessage` entity offsets stay with the
//!   Telegram formatter.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, keyboard building).
//! - `interactionManager` snapshot/transition/clear,
//!   `processUserPrompt`, `getCurrentProject`.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: numeric parses accept non-empty ASCII digits only (TS
//! uses `Number(raw)` which also accepts whitespace, `+3`, `3.0`,
//! hex, and exponents, and maps `""` to `0`); the gates take the
//! already-loaded `skill_count`, parsed `index`/`page`, `stage`, and
//! message-id verdicts as explicit arguments. Skill-item array
//! validation (`parseSkillItems`) takes a pre-decoded
//! name/description slice instead of `unknown` JSON.

/// `skills:` callback prefix.
pub const SKILLS_CB_PREFIX: &str = "skills:";
/// `skills:select:<n>` callback prefix.
pub const SKILLS_CB_SELECT_PREFIX: &str = "skills:select:";
/// `skills:page:<n>` callback prefix.
pub const SKILLS_CB_PAGE_PREFIX: &str = "skills:page:";
/// Cancel callback data (TS: `SKILLS_CALLBACK_CANCEL`).
pub const SKILLS_CB_CANCEL: &str = "skills:cancel";
/// Execute callback data (TS: `SKILLS_CALLBACK_EXECUTE`).
pub const SKILLS_CB_EXECUTE: &str = "skills:execute";
/// Stale/inactive answer key (TS: `t("skills.inactive_callback")`).
pub const SKILLS_INACTIVE_KEY: &str = "skills.inactive_callback";
/// Execute toast key (TS: `t("skills.execute_callback")`).
pub const SKILLS_EXECUTE_KEY: &str = "skills.execute_callback";
/// Page out-of-range answer key (TS: `t("skills.page_empty_callback")`).
pub const SKILLS_PAGE_EMPTY_KEY: &str = "skills.page_empty_callback";
/// Null-index / wrong-stage answer key (TS: `t("callback.processing_error")`).
pub const SKILLS_PROCESSING_ERROR_KEY: &str = "callback.processing_error";
/// Missing-project reply key (TS: `t("bot.project_not_selected")`).
pub const SKILLS_PROJECT_MISSING_KEY: &str = "bot.project_not_selected";
/// List-stage marker (TS: `stage: "list"`).
pub const SKILLS_STAGE_LIST: &str = "list";

/// Handler dispatch branch for a `skills:*` payload (TS: cancel,
/// execute, page, select, and the trailing `processing_error`
/// fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsCallbackAction {
    Cancel,
    Execute,
    Page,
    Select,
    Unknown,
}

/// Answer-key failure behind a handled `skills:*` callback (TS: the
/// `answerCallbackQuery({ text })` keys; success answers empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsFailure {
    Inactive,
    ProcessingError,
    PageEmpty,
    SelectError,
    ProjectMissing,
}

/// Dispatch guard (TS: `if (!data ||
/// !data.startsWith(SKILLS_CALLBACK_PREFIX)) return false`).
pub fn handles_skills_callback(data: Option<&str>) -> bool {
    match data {
        None => false,
        Some(d) => !d.is_empty() && d.starts_with(SKILLS_CB_PREFIX),
    }
}

/// Dispatch classifier (TS: exact `skills:cancel` / `skills:execute`
/// match first, then `parseSkillPageCallback(data) !== null` takes the
/// page branch, then `parseSkillSelectCallback(data)` takes the select
/// branch, anything else falls through to `callback.processing_error`).
pub fn classify_skills_callback_action(data: &str) -> SkillsCallbackAction {
    if data == SKILLS_CB_CANCEL {
        return SkillsCallbackAction::Cancel;
    }
    if data == SKILLS_CB_EXECUTE {
        return SkillsCallbackAction::Execute;
    }
    if parse_skills_page_value(data).is_some() {
        return SkillsCallbackAction::Page;
    }
    if parse_skills_select_index(data).is_some() {
        return SkillsCallbackAction::Select;
    }
    SkillsCallbackAction::Unknown
}

/// Page of a `skills:page:<n>` callback; `None` unless the payload is
/// non-empty ASCII digits (TS: `parseSkillPageCallback`).
pub fn parse_skills_page_value(data: &str) -> Option<u32> {
    let raw = data.strip_prefix(SKILLS_CB_PAGE_PREFIX)?;
    parse_ascii_digits(raw)
}

/// Skill index of a `skills:select:<n>` callback; `None` for
/// non-select payloads and non-digit indexes (TS:
/// `parseSkillSelectCallback`).
pub fn parse_skills_select_index(data: &str) -> Option<u32> {
    let raw = data.strip_prefix(SKILLS_CB_SELECT_PREFIX)?;
    parse_ascii_digits(raw)
}

/// Page callback builder (TS: `` `${SKILLS_CALLBACK_PAGE_PREFIX}${page}` ``).
pub fn build_skills_page_callback(page: u32) -> String {
    format!("{SKILLS_CB_PAGE_PREFIX}{page}")
}

/// Select callback builder (TS: `` `${SKILLS_CALLBACK_SELECT_PREFIX}${index}` ``).
pub fn build_skills_select_callback(index: u32) -> String {
    format!("{SKILLS_CB_SELECT_PREFIX}{index}")
}

/// Prompt text for executing a skill (TS: `args ? `/${skillName}
/// ${args}` : `/${skillName}``, with `args` trimmed by the caller).
pub fn build_skills_prompt_text(skill_name: &str, args: &str) -> String {
    if args.is_empty() {
        format!("/{skill_name}")
    } else {
        format!("/{skill_name} {args}")
    }
}

/// Stale-message gate (TS: `!metadata || callbackMessageId === null
/// || metadata.messageId !== callbackMessageId` answers
/// `skills.inactive_callback`).
pub fn is_skills_message_stale(
    stored_message_id: Option<u64>,
    callback_message_id: Option<u64>,
) -> bool {
    match (stored_message_id, callback_message_id) {
        (Some(stored), Some(current)) => stored != current,
        _ => true,
    }
}

/// Page-branch stage gate (TS: `metadata.stage !== "list"` answers
/// `callback.processing_error`).
pub fn should_reject_page_for_stage(stage: &str) -> bool {
    stage != SKILLS_STAGE_LIST
}

/// Execute-branch stage gate (TS: `metadata.stage !== "confirm"`
/// answers `skills.inactive_callback`).
pub fn should_reject_execute_for_stage(stage: &str) -> bool {
    stage != "confirm"
}

/// Select-branch stage gate (TS: `metadata.stage !== "list"` answers
/// `callback.processing_error`).
pub fn should_reject_select_for_stage(stage: &str) -> bool {
    stage != SKILLS_STAGE_LIST
}

/// List gate (TS: empty skill lists still render; kept for symmetry
/// with the page-empty branch).
pub fn is_skills_list_empty(skill_count: usize) -> bool {
    skill_count == 0
}

/// Page out-of-range gate (TS: `page >= totalPages || page < 0`
/// answers `skills.page_empty_callback`).
pub fn is_skills_page_out_of_range(page: u32, total_pages: u32) -> bool {
    page >= total_pages
}

/// Index bounds against the listed skills (TS:
/// `metadata.skills[skillIndex]`; missing reads as
/// `skills.inactive_callback`).
pub fn resolve_skills_selection(count: usize, index: Option<u32>) -> Option<usize> {
    let i = index? as usize;
    if i < count {
        Some(i)
    } else {
        None
    }
}

/// Skill-name validity (TS: `typeof skillName === "string" &&
/// skillName.trim()` for confirm-stage metadata).
pub fn is_valid_skill_name(name: Option<&str>) -> bool {
    match name {
        Some(n) => !n.trim().is_empty(),
        None => false,
    }
}

/// List-stage page normalization (TS: integer `page` clamped with
/// `Math.max(0, page)`, defaulting to `0`).
pub fn normalize_skills_page(page: Option<i64>) -> u32 {
    match page {
        Some(p) if p > 0 => p as u32,
        Some(_) => 0,
        None => 0,
    }
}

/// Answer key for a handled outcome (`None` = empty answer, TS: bare
/// `answerCallbackQuery()` on page render / select render).
pub fn skills_answer_key(failure: Option<SkillsFailure>) -> Option<&'static str> {
    match failure {
        None => None,
        Some(SkillsFailure::Inactive) => Some(SKILLS_INACTIVE_KEY),
        Some(SkillsFailure::ProcessingError) => Some(SKILLS_PROCESSING_ERROR_KEY),
        Some(SkillsFailure::SelectError) => Some(SKILLS_PROCESSING_ERROR_KEY),
        Some(SkillsFailure::PageEmpty) => Some(SKILLS_PAGE_EMPTY_KEY),
        Some(SkillsFailure::ProjectMissing) => Some(SKILLS_PROJECT_MISSING_KEY),
    }
}

fn parse_ascii_digits(raw: &str) -> Option<u32> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u32>().ok()
}
