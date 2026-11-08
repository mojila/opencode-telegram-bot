//! Session-pick + external-input display + catalog pure gates (slice 53, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/session-pick-reply.ts` (`findEligibleReply`: newest
//!   non-summary/non-error assistant text, incomplete runs skipped while
//!   busy, later-in-list ties win; `findLatestUserPrompt`: newest user
//!   message with text or file lines; `findMessageById`; the
//!   newer-than-reply `findPendingUserPrompt`; `buildSessionPickQuote`:
//!   CRLF-normalized `👤`-titled quote over text or `📎` file lines)
//! - `src/app/services/external-user-input-service.ts`
//!   (`EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH`, `...`-truncation,
//!   CRLF-normalize + trim + empty-to-`None` notification build)
//! - `src/app/services/command-catalog-service.ts`
//!   (`normalizeDirectoryForCommandApi` backslash fold; keep
//!   `source === "command"` with non-blank names except case-insensitive
//!   trimmed `"goal"`)
//! - `src/app/services/skills-catalog-service.ts` (same directory fold;
//!   keep `source === "skill"` with non-blank names)
//! - `src/app/services/mcp-catalog-service.ts`
//!   (`normalizeDirectoryForMcpApi` backslash fold, the five
//!   `MCP_STATUS_NAMES`, `failed`/`needs_client_registration` error
//!   normalization, `parseMcpCatalogServers` record/array forms with
//!   unknown-string-status skip and invalid-shape `null`)
//! - `src/app/services/recent-sessions-service.ts` (status precedence:
//!   question beats permission beats running (`"busy"`/`"retry"`) beats
//!   idle)
//!
//! No OpenCode clients, no filesystem reads, no `config`/env lookups live
//! here — only the pick predicates, display truncation, catalog filters,
//! status parsing, and status precedence the services above drive.
//!
//! DEVIATIONS from TS:
//! 1. `t(...)` lookups (`sessions.last_input.title`,
//!    `sessions.last_input.attachment`, `bot.external_user_input`) are
//!    implicit in TS. The port takes the translated labels as explicit
//!    arguments (`last_input_title`, `attachment_label`, `input_label`)
//!    and still composes the `👤 `/`📎 ` prefixes itself, so the gates
//!    stay pure and testable; callers resolve the locale.
//! 2. `message.info.error: unknown` truthiness is flattened to
//!    [`SessionPickInfo::has_error`]; `info.time.{created,completed}`
//!    becomes `Option<i64>` (`created ?? 0`, `completed === undefined`).
//! 3. `String.length` / `slice` count UTF-16 code units in TS. The port
//!    works on `char`s; parity fixtures use ASCII-only inputs where the
//!    two coincide.
//! 4. The catalog `filter`/`map` chains live inline in `async` loaders in
//!    TS. The port extracts them as pure functions over [`RawCatalogItem`]
//!    slices; the loaders keep doing the fetch.
//! 5. `unknown` JSON in `parseMcpCatalogServers` is modelled as
//!    [`CatalogJson`] (no serde dependency): objects as key/value vecs,
//!    arrays as vecs. `isRecord` admits arrays in TS, but the array
//!    branch runs first there too, so fixtures coincide.
//! 6. The two identical private `normalizeDirectoryFor*Api` helpers map to
//!    the single [`normalize_catalog_api_directory`]; both call sites keep
//!    their names in TS.
//! 7. `buildSessionPickQuote` / `buildExternalUserInputNotification` return
//!    `{ title, body }` here instead of the rendered `QuotedNotification`
//!    (`text` + `rawFallbackText` markdown); callers still route the pair
//!    through `buildQuotedNotification`.

// ---------------------------------------------------------------------------
// Session pick (`app/services/session-pick-reply.ts`)
// ---------------------------------------------------------------------------

/// One message part as the pick helpers inspect it (TS: `{ type,
/// text?, filename? }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPickPart {
    pub kind: String,
    pub text: Option<String>,
    pub filename: Option<String>,
}

/// Flattened `message.info` for the pick helpers (TS: `info.id`,
/// `info.role`, `info.summary`, `info.parentID`, `info.error`,
/// `info.time.{created,completed}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPickInfo {
    pub id: Option<String>,
    pub role: Option<String>,
    pub summary: bool,
    pub parent_id: Option<String>,
    pub has_error: bool,
    pub created: Option<i64>,
    pub completed: Option<i64>,
}

/// One transcript message as the pick helpers inspect it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPickMessage {
    pub info: SessionPickInfo,
    pub parts: Vec<SessionPickPart>,
}

/// The newest usable assistant reply (TS: `EligibleReply`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleReply {
    pub text: String,
    pub parent_id: Option<String>,
    pub created: i64,
}

/// Title + display body of a session-pick quote (TS: the arguments passed
/// to `buildQuotedNotification`, which callers then render).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPickQuote {
    pub title: String,
    pub body: String,
}

/// Joined `text`-part payload, or `None` when the trimmed view is empty
/// (TS: `extractText`).
fn extract_text(parts: &[SessionPickPart]) -> Option<String> {
    let mut joined = String::new();
    for part in parts {
        if part.kind == "text" {
            joined.push_str(part.text.as_deref().unwrap_or(""));
        }
    }
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

/// One `📎 <name>` line per `file` part (TS: `fileLines`).
fn file_lines(parts: &[SessionPickPart], attachment_label: &str) -> Vec<String> {
    parts
        .iter()
        .filter(|part| part.kind == "file")
        .map(|part| {
            let name = part
                .filename
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty());
            format!("📎 {}", name.unwrap_or(attachment_label))
        })
        .collect()
}

/// Newest non-summary, non-error assistant text, skipping runs that have
/// not completed while `busy`; later-in-list ties win
/// (TS: `findEligibleReply`).
pub fn find_eligible_reply(messages: &[SessionPickMessage], busy: bool) -> Option<EligibleReply> {
    let mut latest: Option<EligibleReply> = None;
    for message in messages {
        if message.info.role.as_deref() != Some("assistant") || message.info.summary {
            continue;
        }
        if message.info.has_error {
            continue;
        }
        if busy && message.info.completed.is_none() {
            continue;
        }
        let text = match extract_text(&message.parts) {
            Some(text) => text,
            None => continue,
        };
        let created = message.info.created.unwrap_or(0);
        let replace = match &latest {
            None => true,
            Some(current) => created >= current.created,
        };
        if replace {
            latest = Some(EligibleReply {
                text,
                parent_id: message.info.parent_id.clone(),
                created,
            });
        }
    }
    latest
}

/// Newest user message carrying text or file lines (TS:
/// `findLatestUserPrompt`). `attachment_label` stands in for
/// `t("sessions.last_input.attachment")` when a file part has no name.
pub fn find_latest_user_prompt<'m>(
    messages: &'m [SessionPickMessage],
    attachment_label: &str,
) -> Option<&'m SessionPickMessage> {
    let mut latest: Option<&'m SessionPickMessage> = None;
    let mut latest_created: i64 = -1;
    for message in messages {
        if message.info.role.as_deref() != Some("user") {
            continue;
        }
        if extract_text(&message.parts).is_none()
            && file_lines(&message.parts, attachment_label).is_empty()
        {
            continue;
        }
        let created = message.info.created.unwrap_or(0);
        if latest.is_none() || created >= latest_created {
            latest = Some(message);
            latest_created = created;
        }
    }
    latest
}

/// Message whose `info.id` matches (TS: `findMessageById`).
pub fn find_message_by_id<'m>(
    messages: &'m [SessionPickMessage],
    message_id: &str,
) -> Option<&'m SessionPickMessage> {
    messages
        .iter()
        .find(|message| message.info.id.as_deref() == Some(message_id))
}

/// Newest user prompt created after the shown reply, if any (TS:
/// `findPendingUserPrompt`).
pub fn find_pending_user_prompt<'m>(
    messages: &'m [SessionPickMessage],
    shown_reply_created: i64,
    attachment_label: &str,
) -> Option<&'m SessionPickMessage> {
    let latest = find_latest_user_prompt(messages, attachment_label)?;
    if latest.info.created.unwrap_or(0) > shown_reply_created {
        Some(latest)
    } else {
        None
    }
}

/// `👤`-titled quote over the message text, or over its `📎` file lines
/// when it has no text; `None` when both are empty (TS:
/// `buildSessionPickQuote`). The body is CRLF-normalized, trimmed, and
/// truncated like external user input.
pub fn build_session_pick_quote(
    message: &SessionPickMessage,
    last_input_title: &str,
    attachment_label: &str,
) -> Option<SessionPickQuote> {
    let raw_body = match extract_text(&message.parts) {
        Some(text) => text,
        None => file_lines(&message.parts, attachment_label).join("\n"),
    };
    let normalized = raw_body.replace("\r\n", "\n").trim().to_string();
    if normalized.is_empty() {
        return None;
    }
    Some(SessionPickQuote {
        title: format!("👤 {last_input_title}"),
        body: truncate_external_user_input_text(&normalized),
    })
}

// ---------------------------------------------------------------------------
// External user input display (`app/services/external-user-input-service.ts`)
// ---------------------------------------------------------------------------

/// Display budget before `...` truncation
/// (`EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH`).
pub const EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH: usize = 2000;

/// Truncation marker appended by `truncateExternalUserInputText`.
pub const EXTERNAL_USER_INPUT_TRUNCATION_SUFFIX: &str = "...";

/// Truncate over-budget text with a `...` suffix (TS:
/// `truncateExternalUserInputText`).
pub fn truncate_external_user_input_text(text: &str) -> String {
    if text.chars().count() <= EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH {
        return text.to_string();
    }
    let prefix: String = text
        .chars()
        .take(EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH - EXTERNAL_USER_INPUT_TRUNCATION_SUFFIX.len())
        .collect();
    format!("{prefix}{}", EXTERNAL_USER_INPUT_TRUNCATION_SUFFIX)
}

/// Title + display body of an external-input notice (TS: the arguments
/// passed to `buildQuotedNotification`, i.e. `ExternalUserInputNotification`
/// without the rendering step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalUserInputNotification {
    pub title: String,
    pub body: String,
}

/// CRLF-normalize + trim, `None` when empty, otherwise the truncated
/// display text under a `👤` title (TS:
/// `buildExternalUserInputNotification`). `input_label` stands in for
/// `t("bot.external_user_input")`.
pub fn build_external_user_input_notification(
    text: &str,
    input_label: &str,
) -> Option<ExternalUserInputNotification> {
    let normalized = text.replace("\r\n", "\n").trim().to_string();
    if normalized.is_empty() {
        return None;
    }
    Some(ExternalUserInputNotification {
        title: format!("👤 {input_label}"),
        body: truncate_external_user_input_text(&normalized),
    })
}

// ---------------------------------------------------------------------------
// Command / skills catalogs (`command-catalog-service.ts`,
// `skills-catalog-service.ts`)
// ---------------------------------------------------------------------------

/// Fold Windows separators for the command/skills/MCP APIs (TS:
/// `normalizeDirectoryForCommandApi`, `normalizeDirectoryForMcpApi`).
pub fn normalize_catalog_api_directory(directory: &str) -> String {
    directory.replace('\\', "/")
}

/// One raw catalog row as the loaders receive it (TS: `{ name,
/// description?, source }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCatalogItem {
    pub name: String,
    pub description: Option<String>,
    pub source: String,
}

/// One catalog entry as the loaders return it (TS: `{ name, description }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub name: String,
    pub description: Option<String>,
}

/// Keep `source === "command"` rows with non-blank names, except
/// case-insensitive trimmed `"goal"`; the original (untrimmed) name is
/// preserved (TS: `loadCommandCatalog` filter/map).
pub fn filter_command_catalog_items(items: &[RawCatalogItem]) -> Vec<CatalogEntry> {
    items
        .iter()
        .filter(|item| {
            item.source == "command"
                && !item.name.trim().is_empty()
                && item.name.trim().to_lowercase() != "goal"
        })
        .map(|item| CatalogEntry {
            name: item.name.clone(),
            description: item.description.clone(),
        })
        .collect()
}

/// Keep `source === "skill"` rows with non-blank names; the original
/// (untrimmed) name is preserved (TS: `loadSkillsCatalog` filter/map).
pub fn filter_skills_catalog_items(items: &[RawCatalogItem]) -> Vec<CatalogEntry> {
    items
        .iter()
        .filter(|item| item.source == "skill" && !item.name.trim().is_empty())
        .map(|item| CatalogEntry {
            name: item.name.clone(),
            description: item.description.clone(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// MCP catalog (`app/services/mcp-catalog-service.ts`)
// ---------------------------------------------------------------------------

/// Known MCP server statuses in wire order (TS: `MCP_STATUS_NAMES`).
pub const MCP_STATUS_NAMES: [&str; 5] = [
    "connected",
    "disabled",
    "failed",
    "needs_auth",
    "needs_client_registration",
];

/// A recognized MCP server status (TS: entries of `MCP_STATUS_NAMES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStatusName {
    Connected,
    Disabled,
    Failed,
    NeedsAuth,
    NeedsClientRegistration,
}

/// Recognize one status name (TS: `isMcpStatusName`).
pub fn mcp_status_name_of(value: &str) -> Option<McpStatusName> {
    match value {
        "connected" => Some(McpStatusName::Connected),
        "disabled" => Some(McpStatusName::Disabled),
        "failed" => Some(McpStatusName::Failed),
        "needs_auth" => Some(McpStatusName::NeedsAuth),
        "needs_client_registration" => Some(McpStatusName::NeedsClientRegistration),
        _ => None,
    }
}

/// A parsed MCP server status. `failed` and `needs_client_registration`
/// always carry an error string (empty when the payload has none); the
/// other states carry none (TS: `buildMcpStatus`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerStatus {
    pub name: McpStatusName,
    pub error: Option<String>,
}

/// One parsed catalog server (TS: `McpCatalogServerItem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCatalogServerItem {
    pub name: String,
    pub status: McpServerStatus,
}

/// Minimal JSON shape for `parseMcpCatalogServers` inputs (TS:
/// `unknown`; objects preserve insertion order like `Object.entries`).
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogJson {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<CatalogJson>),
    Obj(Vec<(String, CatalogJson)>),
}

fn catalog_field<'a>(fields: &'a [(String, CatalogJson)], key: &str) -> Option<&'a CatalogJson> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn build_mcp_status(name: McpStatusName, error_value: Option<&CatalogJson>) -> McpServerStatus {
    match name {
        McpStatusName::Failed | McpStatusName::NeedsClientRegistration => {
            let error = match error_value {
                Some(CatalogJson::Str(text)) => text.clone(),
                _ => String::new(),
            };
            McpServerStatus {
                name,
                error: Some(error),
            }
        }
        _ => McpServerStatus { name, error: None },
    }
}

enum ParsedMcpServerStatus {
    Ok(McpServerStatus),
    Skip,
    Invalid,
}

fn parse_mcp_server_status(status: &CatalogJson) -> ParsedMcpServerStatus {
    let CatalogJson::Obj(fields) = status else {
        return ParsedMcpServerStatus::Invalid;
    };
    let Some(status_value) = catalog_field(fields, "status") else {
        return ParsedMcpServerStatus::Invalid;
    };
    let CatalogJson::Str(name) = status_value else {
        // Unknown non-string status shape: the whole catalog is invalid.
        // Known-string but unrecognized names skip just that server below.
        return ParsedMcpServerStatus::Invalid;
    };
    match mcp_status_name_of(name) {
        Some(recognized) => {
            let error_value = catalog_field(fields, "error");
            ParsedMcpServerStatus::Ok(build_mcp_status(recognized, error_value))
        }
        None => ParsedMcpServerStatus::Skip,
    }
}

/// Parse the record or array form into servers; unknown string statuses
/// skip that server, any invalid shape yields `None` (TS:
/// `parseMcpCatalogServers`).
pub fn parse_mcp_catalog_servers(value: &CatalogJson) -> Option<Vec<McpCatalogServerItem>> {
    match value {
        CatalogJson::Arr(items) => {
            let mut servers = Vec::with_capacity(items.len());
            for item in items {
                let CatalogJson::Obj(fields) = item else {
                    return None;
                };
                let Some(CatalogJson::Str(name)) = catalog_field(fields, "name") else {
                    return None;
                };
                let Some(status_value) = catalog_field(fields, "status") else {
                    return None;
                };
                match parse_mcp_server_status(status_value) {
                    ParsedMcpServerStatus::Invalid => return None,
                    ParsedMcpServerStatus::Skip => continue,
                    ParsedMcpServerStatus::Ok(status) => servers.push(McpCatalogServerItem {
                        name: name.clone(),
                        status,
                    }),
                }
            }
            Some(servers)
        }
        CatalogJson::Obj(entries) => {
            let mut servers = Vec::with_capacity(entries.len());
            for (name, status_value) in entries {
                match parse_mcp_server_status(status_value) {
                    ParsedMcpServerStatus::Invalid => return None,
                    ParsedMcpServerStatus::Skip => continue,
                    ParsedMcpServerStatus::Ok(status) => servers.push(McpCatalogServerItem {
                        name: name.clone(),
                        status,
                    }),
                }
            }
            Some(servers)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Recent sessions (`app/services/recent-sessions-service.ts`)
// ---------------------------------------------------------------------------

/// Session badge shown in the recent list (TS: `RecentStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecentStatus {
    Question,
    Permission,
    Running,
    Idle,
}

/// Question beats permission beats running (`"busy"`/`"retry"`) beats idle
/// (TS: the `statuses.set` precedence in `loadRecentSessions`).
pub fn decide_recent_status(
    has_question: bool,
    has_permission: bool,
    run_type: Option<&str>,
) -> RecentStatus {
    if has_question {
        return RecentStatus::Question;
    }
    if has_permission {
        return RecentStatus::Permission;
    }
    if matches!(run_type, Some("busy") | Some("retry")) {
        return RecentStatus::Running;
    }
    RecentStatus::Idle
}
