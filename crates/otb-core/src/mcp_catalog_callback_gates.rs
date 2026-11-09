//! MCP catalog callback-handler pure gates (slice 87, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/mcp-catalog-callback-handler.ts`
//!   (`MCPS_CALLBACK_PREFIX` dispatch guard; the exact `mcps:cancel` /
//!   `mcps:back` / `mcps:toggle` matches; the `mcps:select:` prefix branch
//!   with `parseMcpSelectCallback` index bounds; `parseMcpsMetadata`:
//!   `custom` kind + `mcps` flow + numeric `messageId` + string
//!   `projectDirectory` + `parseMcpCatalogServers` validity gates, the
//!   `list` stage vs the `detail` stage with its non-blank `serverName`;
//!   the metadata/callback message-id mismatch guard (`inline.
//!   inactive_callback`); the `back`/`toggle`-require-`detail` and
//!   `select`-requires-`list` stage guards (`callback.processing_error`);
//!   the toggle server-by-name lookup (`inline.inactive_callback` when
//!   missing); the `enable = status !== "connected"` gate with the
//!   `mcps.enabling` / `mcps.disabling` toasts; the updated-server-missing
//!   list fallback; the trailing `callback.unknown_command` fallback; the
//!   `mcps.toggle_error` catch-all).
//! - `src/bot/menus/mcp-catalog-menu.ts` (the `MCPS_*` callback constants;
//!   already classified in `catalog_settings_menu_gates`, repeated here
//!   only as the dispatch table the handler matches).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `catalog_settings_menu_gates` (the `MCPS_*` constants,
//!   `parseMcpSelectCallback`, the status emoji/label maps, the
//!   list/detail keyboard layouts, `buildMcpsDetailText`).
//! - `session_pick_catalog_gates` (`parseMcpCatalogServers` record/array
//!   forms, `MCP_STATUS_NAMES`).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, keyboard building, transitions).
//! - `loadMcpCatalog` / `toggleMcpCatalogServer` SDK calls,
//!   `interactionManager` snapshots/transitions/clears.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: the `InteractionState | null` snapshot is flattened into
//! `RawMcpsState` so the gates stay pure, and message ids are `i64`
//! (`typeof === "number"` in TS). `(None, None)` message-id pairs count
//! as mismatch (TS answers `inactive_callback` when metadata or the
//! callback message id is missing). `McpsCallbackAction::Unknown` is
//! stage-free-rejected by `mcps_stage_allows` (it never reaches a stage
//! check in TS; it falls through to `callback.unknown_command`).
//!
//! Status: GREEN — ported; parity tests pass.

/// `mcps:` callback prefix.
pub const MCPS_CALLBACK_PREFIX: &str = "mcps:";
/// `mcps:select:` callback prefix.
pub const MCPS_CALLBACK_SELECT_PREFIX: &str = "mcps:select:";
/// `mcps:toggle` callback data.
pub const MCPS_CALLBACK_TOGGLE: &str = "mcps:toggle";
/// `mcps:back` callback data.
pub const MCPS_CALLBACK_BACK: &str = "mcps:back";
/// `mcps:cancel` callback data.
pub const MCPS_CALLBACK_CANCEL: &str = "mcps:cancel";

/// Stale-menu answer key (TS: `t("inline.inactive_callback")`).
pub const MCPS_INACTIVE_KEY: &str = "inline.inactive_callback";
/// Wrong-stage answer key (TS: `t("callback.processing_error")`).
pub const MCPS_PROCESSING_ERROR_KEY: &str = "callback.processing_error";
/// Catch-all error answer key (TS: `t("mcps.toggle_error")`).
pub const MCPS_TOGGLE_ERROR_KEY: &str = "mcps.toggle_error";

/// Handler dispatch branch for an `mcps:*` payload (TS: the exact-match
/// `cancel` / `back` / `toggle` branches, the `select:` prefix branch,
/// and the trailing `unknown_command` fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpsCallbackAction {
    Cancel,
    Back,
    Toggle,
    Select,
    Unknown,
}

/// Catalog interaction stage (TS: `metadata.stage`: `list` | `detail`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpsStage {
    List,
    Detail,
}

/// Flattened `InteractionState | null` snapshot behind
/// `parseMcpsMetadata` (TS reads `state.metadata.flow/stage/messageId/
//// projectDirectory/servers/serverName` with `typeof` gates).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMcpsState {
    pub kind_is_custom: bool,
    pub flow: String,
    pub stage: String,
    pub message_id: Option<i64>,
    pub project_directory: Option<String>,
    pub servers_valid: bool,
    pub server_count: usize,
    pub server_name: Option<String>,
}

/// Validated MCP catalog interaction (TS: `McpsMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpsMetadata {
    pub message_id: i64,
    pub project_directory: String,
    pub stage: McpsStage,
    pub server_name: Option<String>,
    pub server_count: usize,
}

/// MCP-callback dispatch guard (TS: `if (!data ||
/// !data.startsWith(MCPS_CALLBACK_PREFIX)) return false`).
pub fn handles_mcps_callback(data: Option<&str>) -> bool {
    match data {
        Some(s) => s.starts_with(MCPS_CALLBACK_PREFIX),
        None => false,
    }
}

/// Handler dispatch classifier (TS: exact `cancel` / `back` / `toggle`
/// matches, the `select:` prefix branch, anything else falls through to
/// `callback.unknown_command`).
pub fn classify_mcps_callback_action(data: &str) -> McpsCallbackAction {
    if data == MCPS_CALLBACK_CANCEL {
        McpsCallbackAction::Cancel
    } else if data == MCPS_CALLBACK_BACK {
        McpsCallbackAction::Back
    } else if data == MCPS_CALLBACK_TOGGLE {
        McpsCallbackAction::Toggle
    } else if data.starts_with(MCPS_CALLBACK_SELECT_PREFIX) {
        McpsCallbackAction::Select
    } else {
        McpsCallbackAction::Unknown
    }
}

/// Interaction metadata parser (TS: `parseMcpsMetadata`: non-`custom`
/// kind, non-`mcps` flow, non-numeric `messageId`, non-string
/// `projectDirectory`, or invalid servers read as `None`; `list` keeps
/// the list shape; `detail` additionally needs a non-blank `serverName`).
pub fn parse_mcps_metadata(state: &RawMcpsState) -> Option<McpsMetadata> {
    if !state.kind_is_custom {
        return None;
    }
    if state.flow != "mcps" {
        return None;
    }
    let message_id = state.message_id?;
    let project_directory = state.project_directory.clone()?;
    if !state.servers_valid {
        return None;
    }
    match state.stage.as_str() {
        "list" => Some(McpsMetadata {
            message_id,
            project_directory,
            stage: McpsStage::List,
            server_name: None,
            server_count: state.server_count,
        }),
        "detail" => {
            let name = state.server_name.clone()?;
            if name.trim().is_empty() {
                return None;
            }
            Some(McpsMetadata {
                message_id,
                project_directory,
                stage: McpsStage::Detail,
                server_name: Some(name),
                server_count: state.server_count,
            })
        }
        _ => None,
    }
}

/// Stale-menu guard (TS: `!metadata || callbackMessageId === null ||
/// metadata.messageId !== callbackMessageId` answers
/// `inline.inactive_callback`). `(None, None)` counts as mismatch.
pub fn is_mcps_message_mismatch(
    metadata_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (metadata_message_id, callback_message_id) {
        (Some(a), Some(b)) => a != b,
        _ => true,
    }
}

/// Stage guard (TS: `back` / `toggle` answer `callback.processing_error`
/// outside `detail`; `select` answers it outside `list`; `cancel` is
/// stage-free; `unknown` never reaches a stage).
pub fn mcps_stage_allows(action: McpsCallbackAction, stage: McpsStage) -> bool {
    match action {
        McpsCallbackAction::Cancel => true,
        McpsCallbackAction::Back | McpsCallbackAction::Toggle => stage == McpsStage::Detail,
        McpsCallbackAction::Select => stage == McpsStage::List,
        McpsCallbackAction::Unknown => false,
    }
}

/// Toggle server lookup (TS: `metadata.servers.find((s) => s.name ===
/// serverName)`; missing reads as `inline.inactive_callback`).
pub fn resolve_mcps_server_by_name(names: &[&str], target: &str) -> Option<usize> {
    names.iter().position(|name| *name == target)
}

/// Select-index bounds (TS: `parseMcpSelectCallback` plus
/// `metadata.servers[serverIndex]`; `None`/out-of-range reads as
/// `inline.inactive_callback`).
pub fn resolve_mcps_select_server(server_count: usize, index: Option<u64>) -> Option<usize> {
    let i = index?;
    if i < server_count as u64 {
        Some(i as usize)
    } else {
        None
    }
}

/// Toggle direction (TS: `const enable = server.status.status !==
/// "connected"`).
pub fn should_enable_mcp_server(status: &str) -> bool {
    status != "connected"
}

/// Updated-server-missing fallback (TS: when the toggled server is gone
/// from the reloaded catalog the handler re-renders the list menu).
pub fn should_fall_back_to_mcp_list(updated_found: bool) -> bool {
    !updated_found
}

/// Success-toast key for a handled action (`None` = empty answer, TS:
/// bare `answerCallbackQuery()`): `cancel` -> `common.cancelled`,
/// `toggle` -> `mcps.enabling` / `mcps.disabling`, `back` / `select` ->
/// empty, `unknown` -> `callback.unknown_command`.
pub fn mcps_answer_key(action: McpsCallbackAction, enable: bool) -> Option<&'static str> {
    match action {
        McpsCallbackAction::Cancel => Some("common.cancelled"),
        McpsCallbackAction::Toggle => {
            if enable {
                Some("mcps.enabling")
            } else {
                Some("mcps.disabling")
            }
        }
        McpsCallbackAction::Back | McpsCallbackAction::Select => None,
        McpsCallbackAction::Unknown => Some("callback.unknown_command"),
    }
}
