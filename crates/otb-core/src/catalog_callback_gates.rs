//! Catalog callback + text-args pure gates (slice 79, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/command-catalog-callback-handler.ts`
//!   (`parseCommandItems` name/description validation,
//!   `parseCommandsMetadata` kind/flow/stage/messageId/project gates with
//!   the list-page `Math.max(0, page)` clamp, `getCallbackMessageId`
//!   shape, the `commands:` prefix guard, exact `commands:cancel` /
//!   `commands:execute` matches, the metadata/message-id mismatch guard,
//!   execute-stage `confirm` gate, page `page >= totalPages || page < 0`
//!   empty gate, select-index bounds, `argumentsText.trim()` + the
//!   `/name args` invocation text, `sessionStatus.type === "busy"`)
//! - `src/bot/callbacks/skills-catalog-callback-handler.ts` (same shape
//!   over `skills:`: `parseSkillItems`, `parseSkillsMetadata`, prefix /
//!   cancel / execute / mismatch / confirm / page-empty / index-bounds /
//!   invocation gates)
//! - `src/bot/handlers/text-message-handler.ts` (`text === undefined ||
//!   text.startsWith("/")` early-`false` guard, `confirm`-stage gate, the
//!   `text.trim()` empty-arguments gate)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, keyboard building, transitions).
//! - OpenCode session create/status/command calls, attach/busy/idle
//!   managers, `config.bot.commandsListLimit`, `getCurrentSession`,
//!   `getCurrentProject`, `getStoredAgent/Model`.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `""`/whitespace-only counts as missing for
//! names (`!name.trim()`), `page` follows `typeof === "number" &&
//! Number.isInteger` then `Math.max(0, page)` else `0`, and message ids
//! are `i64` (`typeof === "number"` in TS). `(None, None)` message-id
//! pairs count as mismatch (TS answers `inactive_callback` when metadata
//! or the callback message id is missing). `is_arguments_empty` trims
//! before the emptiness check to mirror `if (!argumentsText)` after
//! `text.trim()`.

/// Callback prefix for command-catalog callbacks (TS: `COMMANDS_CALLBACK_PREFIX`).
pub const COMMANDS_CALLBACK_PREFIX: &str = "commands:";
/// Exact cancel callback (TS: `COMMANDS_CALLBACK_CANCEL`).
pub const COMMANDS_CALLBACK_CANCEL: &str = "commands:cancel";
/// Exact execute callback (TS: `COMMANDS_CALLBACK_EXECUTE`).
pub const COMMANDS_CALLBACK_EXECUTE: &str = "commands:execute";
/// Callback prefix for skill-catalog callbacks (TS: `SKILLS_CALLBACK_PREFIX`).
pub const SKILLS_CALLBACK_PREFIX: &str = "skills:";
/// Exact cancel callback (TS: `SKILLS_CALLBACK_CANCEL`).
pub const SKILLS_CALLBACK_CANCEL: &str = "skills:cancel";
/// Exact execute callback (TS: `SKILLS_CALLBACK_EXECUTE`).
pub const SKILLS_CALLBACK_EXECUTE: &str = "skills:execute";

/// One raw catalog row as the metadata parsers validate it
/// (TS: `unknown` item with `name?: unknown, description?: unknown`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCatalogItem {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A validated catalog row (TS: `CommandCatalogItem | SkillCatalogItem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItem {
    pub name: String,
    pub description: Option<String>,
}

/// Flattened interaction snapshot the metadata parsers read
/// (TS: `InteractionState | null` with `kind` + `metadata` bag).
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionSnapshot {
    pub kind: String,
    pub flow: Option<String>,
    pub stage: Option<String>,
    pub message_id: Option<i64>,
    pub project_directory: Option<String>,
    pub commands: Option<Vec<RawCatalogItem>>,
    pub skills: Option<Vec<RawCatalogItem>>,
    pub command_name: Option<String>,
    pub skill_name: Option<String>,
    pub page: Option<f64>,
}

/// Parsed `commands` list metadata (TS: `CommandsListMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandsListMetadata {
    pub message_id: i64,
    pub project_directory: String,
    pub commands: Vec<CatalogItem>,
    pub page: i64,
}

/// Parsed `commands` confirm metadata (TS: `CommandsConfirmMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandsConfirmMetadata {
    pub message_id: i64,
    pub project_directory: String,
    pub command_name: String,
}

/// Parsed `commands` metadata (TS: `CommandsMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandsMetadata {
    List(CommandsListMetadata),
    Confirm(CommandsConfirmMetadata),
}

/// Parsed `skills` list metadata (TS: `SkillsListMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsListMetadata {
    pub message_id: i64,
    pub project_directory: String,
    pub skills: Vec<CatalogItem>,
    pub page: i64,
}

/// Parsed `skills` confirm metadata (TS: `SkillsConfirmMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsConfirmMetadata {
    pub message_id: i64,
    pub project_directory: String,
    pub skill_name: String,
}

/// Parsed `skills` metadata (TS: `SkillsMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillsMetadata {
    List(SkillsListMetadata),
    Confirm(SkillsConfirmMetadata),
}

fn validate_catalog_items(items: &[RawCatalogItem]) -> Option<Vec<CatalogItem>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let name = item.name.as_ref()?;
        if name.trim().is_empty() {
            return None;
        }
        out.push(CatalogItem {
            name: name.clone(),
            description: item.description.clone(),
        });
    }
    Some(out)
}

/// Validate raw command rows: every row must be an object with a
/// non-blank `name`; `description` survives only when a string
/// (TS: `parseCommandItems`).
pub fn parse_command_items(items: &[RawCatalogItem]) -> Option<Vec<CatalogItem>> {
    validate_catalog_items(items)
}

/// Validate raw skill rows (TS: `parseSkillItems`, same shape as commands).
pub fn parse_skill_items(items: &[RawCatalogItem]) -> Option<Vec<CatalogItem>> {
    validate_catalog_items(items)
}

/// Parse `commands` interaction metadata with the kind/flow/stage/
/// messageId/project gates and the list-page clamp
/// (TS: `parseCommandsMetadata`).
pub fn parse_commands_metadata(state: Option<&InteractionSnapshot>) -> Option<CommandsMetadata> {
    let state = state?;
    if state.kind != "custom" {
        return None;
    }
    if state.flow.as_deref() != Some("commands") {
        return None;
    }
    let message_id = state.message_id?;
    let project_directory = state.project_directory.clone()?;
    match state.stage.as_deref() {
        Some("list") => {
            let raw = state.commands.as_ref()?;
            let commands = parse_command_items(raw)?;
            Some(CommandsMetadata::List(CommandsListMetadata {
                message_id,
                project_directory,
                commands,
                page: resolve_catalog_page(state.page),
            }))
        }
        Some("confirm") => {
            let command_name = state.command_name.clone()?;
            if command_name.trim().is_empty() {
                return None;
            }
            Some(CommandsMetadata::Confirm(CommandsConfirmMetadata {
                message_id,
                project_directory,
                command_name,
            }))
        }
        _ => None,
    }
}

/// Parse `skills` interaction metadata (TS: `parseSkillsMetadata`).
pub fn parse_skills_metadata(state: Option<&InteractionSnapshot>) -> Option<SkillsMetadata> {
    let state = state?;
    if state.kind != "custom" {
        return None;
    }
    if state.flow.as_deref() != Some("skills") {
        return None;
    }
    let message_id = state.message_id?;
    let project_directory = state.project_directory.clone()?;
    match state.stage.as_deref() {
        Some("list") => {
            let raw = state.skills.as_ref()?;
            let skills = parse_skill_items(raw)?;
            Some(SkillsMetadata::List(SkillsListMetadata {
                message_id,
                project_directory,
                skills,
                page: resolve_catalog_page(state.page),
            }))
        }
        Some("confirm") => {
            let skill_name = state.skill_name.clone()?;
            if skill_name.trim().is_empty() {
                return None;
            }
            Some(SkillsMetadata::Confirm(SkillsConfirmMetadata {
                message_id,
                project_directory,
                skill_name,
            }))
        }
        _ => None,
    }
}

/// Callback belongs to the command catalog (TS:
/// `data.startsWith(COMMANDS_CALLBACK_PREFIX)`; `None`/empty is `false`).
pub fn handles_commands_callback(data: Option<&str>) -> bool {
    data.is_some_and(|d| d.starts_with(COMMANDS_CALLBACK_PREFIX))
}

/// Callback belongs to the skill catalog (TS:
/// `data.startsWith(SKILLS_CALLBACK_PREFIX)`).
pub fn handles_skills_callback(data: Option<&str>) -> bool {
    data.is_some_and(|d| d.starts_with(SKILLS_CALLBACK_PREFIX))
}

/// Exact `commands:cancel` match (TS: `data === COMMANDS_CALLBACK_CANCEL`).
pub fn is_commands_cancel(data: &str) -> bool {
    data == COMMANDS_CALLBACK_CANCEL
}

/// Exact `commands:execute` match (TS: `data === COMMANDS_CALLBACK_EXECUTE`).
pub fn is_commands_execute(data: &str) -> bool {
    data == COMMANDS_CALLBACK_EXECUTE
}

/// Exact `skills:cancel` match (TS: `data === SKILLS_CALLBACK_CANCEL`).
pub fn is_skills_cancel(data: &str) -> bool {
    data == SKILLS_CALLBACK_CANCEL
}

/// Exact `skills:execute` match (TS: `data === SKILLS_CALLBACK_EXECUTE`).
pub fn is_skills_execute(data: &str) -> bool {
    data == SKILLS_CALLBACK_EXECUTE
}

/// Stale-callback guard: no metadata, no callback message id, or id
/// mismatch answers `inactive_callback` (TS: `!metadata ||
/// callbackMessageId === null || metadata.messageId !== callbackMessageId`).
pub fn is_callback_message_mismatch(
    metadata_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (metadata_message_id, callback_message_id) {
        (Some(a), Some(b)) => a != b,
        _ => true,
    }
}

/// Execute requires the `confirm` stage (TS: `metadata.stage !== "confirm"`
/// answers `inactive_callback` on execute).
pub fn is_confirm_stage(stage: Option<&str>) -> bool {
    stage == Some("confirm")
}

/// Page callbacks outside `[0, totalPages)` answer `page_empty_callback`
/// (TS: `page >= totalPages || page < 0`).
pub fn is_catalog_page_empty(page: i64, total_pages: i64) -> bool {
    page >= total_pages || page < 0
}

/// Select callbacks need the `list` stage and an in-bounds index
/// (TS: `parseXSelectCallback(...) === null || metadata.stage !== "list"`
/// answers `processing_error`; missing item answers `inactive_callback`).
pub fn is_catalog_select_valid(index: i64, item_count: usize, is_list_stage: bool) -> bool {
    if !is_list_stage || index < 0 {
        return false;
    }
    (index as usize) < item_count
}

/// Clamp for list metadata pages: an integer page reads as
/// `Math.max(0, page)`, anything else as `0`
/// (TS: `typeof page === "number" && Number.isInteger(page) ? ... : 0`).
pub fn resolve_catalog_page(page: Option<f64>) -> i64 {
    match page {
        Some(v) if v.is_finite() && v.fract() == 0.0 => (v as i64).max(0),
        _ => 0,
    }
}

/// Catalog text arguments are only eligible for non-command text
/// (TS: `text === undefined || text.startsWith("/")` returns `false`).
pub fn is_catalog_text_eligible(text: Option<&str>) -> bool {
    match text {
        Some(t) => !t.starts_with('/'),
        None => false,
    }
}

/// Empty trimmed arguments get the `arguments_empty` reply instead of
/// executing (TS: `if (!argumentsText)` after `text.trim()`).
pub fn is_arguments_empty(arguments_text: &str) -> bool {
    arguments_text.trim().is_empty()
}

/// `text.trim()` before the empty check and execution
/// (TS: `const argumentsText = text.trim()`).
pub fn trim_arguments(text: &str) -> String {
    text.trim().to_string()
}

/// `/name args` when args survive, otherwise `/name`
/// (TS: `` args ? `/${commandName} ${args}` : `/${commandName}` ``).
pub fn build_command_invocation(command_name: &str, args: &str) -> String {
    if args.is_empty() {
        format!("/{command_name}")
    } else {
        format!("/{command_name} {args}")
    }
}

/// Same invocation shape for skills (TS: `` args ? `/${skillName} ${args}`
/// : `/${skillName}` `` in `executeSkill`).
pub fn build_skill_invocation(skill_name: &str, args: &str) -> String {
    if args.is_empty() {
        format!("/{skill_name}")
    } else {
        format!("/{skill_name} {args}")
    }
}

/// Session busy predicate before running a command (TS:
/// `sessionStatus.type === "busy"`, `false` when unknown).
pub fn is_session_busy(status_type: Option<&str>) -> bool {
    status_type == Some("busy")
}

/// Interaction clear only runs with live catalog metadata
/// (TS: `if (metadata) { clear(reason) }`).
pub fn should_clear_catalog_interaction(has_metadata: bool) -> bool {
    has_metadata
}
