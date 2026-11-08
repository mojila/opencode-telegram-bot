//! Message-history callback-handler pure gates (slice 80, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/message-history-callback-handler.ts`
//!   (`parseMessages` id/text/created validation,
//!   `parseMessagesMetadata` kind/flow/stage/messageId/projectDirectory/
//!   sessionId/messages gates with the list-page `Math.max(0, page)`
//!   clamp and the detail `selectedIndex` integer/`>= 0` gate,
//!   `getCallbackMessageId` shape, the `messages:` prefix guard, exact
//!   `messages:revert` / `messages:fork` / `messages:back` /
//!   `messages:cancel` matches, the metadata/message-id mismatch guard,
//!   revert+fork+back `detail`-stage gates, the selected-message-exists
//!   gate, the page `page >= totalPages || page < 0` empty gate, the
//!   select `list`-stage + index-bounds gate, the foreground-busy block,
//!   the `if (metadata) { clear(reason) }` clear gate)
//!
//! Menu-level `MESSAGES_CALLBACK_*` constants plus
//! `parseMessagePageCallback` / `parseMessageSelectCallback` already live
//! in `worktree_history_inline_gates`; the constants below repeat the
//! same values so this handler slice stays self-contained (the same
//! duplication pattern `catalog_callback_gates` uses for the
//! command/skills prefixes).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   transitions, `cancelMenu`, busy-blocked replies).
//! - OpenCode session revert/fork/get calls, `setCurrentSession`,
//!   `applySessionSettings`, attach flows, cache ingest, config
//!   `bot.messagesListLimit`, `calculateMessagesPaginationRange`,
//!   keyboard building, `loadLatestAssistantResponse` resend.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; message ids are `i64` (`typeof ===
//! "number"` in TS), `created` is `f64` (`typeof === "number"` in TS),
//! `page` follows `typeof === "number" && Number.isInteger` then
//! `Math.max(0, page)` else `0`, `selectedIndex` needs
//! `Number.isInteger && >= 0`, and `(None, None)` message-id pairs
//! count as mismatch (TS answers `inactive_callback` when metadata or
//! the callback message id is missing).

/// Callback prefix for message-history callbacks (TS: `MESSAGES_CALLBACK_PREFIX`).
pub const MESSAGES_CALLBACK_PREFIX: &str = "messages:";
/// Exact revert callback (TS: `MESSAGES_CALLBACK_REVERT`).
pub const MESSAGES_CALLBACK_REVERT: &str = "messages:revert";
/// Exact fork callback (TS: `MESSAGES_CALLBACK_FORK`).
pub const MESSAGES_CALLBACK_FORK: &str = "messages:fork";
/// Exact back callback (TS: `MESSAGES_CALLBACK_BACK`).
pub const MESSAGES_CALLBACK_BACK: &str = "messages:back";
/// Exact cancel callback (TS: `MESSAGES_CALLBACK_CANCEL`).
pub const MESSAGES_CALLBACK_CANCEL: &str = "messages:cancel";

/// One raw message row as the metadata parser validates it
/// (TS: `unknown` item with `id?: unknown, text?: unknown, created?: unknown`).
#[derive(Debug, Clone, PartialEq)]
pub struct RawMessageItem {
    pub id: Option<String>,
    pub text: Option<String>,
    pub created: Option<f64>,
}

/// A validated user message row (TS: `UserMessageItem`).
#[derive(Debug, Clone, PartialEq)]
pub struct UserMessageItem {
    pub id: String,
    pub text: String,
    pub created: f64,
}

/// Flattened interaction snapshot the metadata parser reads
/// (TS: `InteractionState | null` with `kind` + `metadata` bag).
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionSnapshot {
    pub kind: String,
    pub flow: Option<String>,
    pub stage: Option<String>,
    pub message_id: Option<i64>,
    pub project_directory: Option<String>,
    pub session_id: Option<String>,
    pub messages: Option<Vec<RawMessageItem>>,
    pub page: Option<f64>,
    pub selected_index: Option<f64>,
}

/// Parsed `messages` list metadata (TS: `MessagesListMetadata`).
#[derive(Debug, Clone, PartialEq)]
pub struct MessagesListMetadata {
    pub message_id: i64,
    pub project_directory: String,
    pub session_id: String,
    pub messages: Vec<UserMessageItem>,
    pub page: i64,
}

/// Parsed `messages` detail metadata (TS: `MessagesDetailMetadata`).
#[derive(Debug, Clone, PartialEq)]
pub struct MessagesDetailMetadata {
    pub message_id: i64,
    pub project_directory: String,
    pub session_id: String,
    pub messages: Vec<UserMessageItem>,
    pub page: i64,
    pub selected_index: i64,
}

/// Parsed `messages` metadata (TS: `MessagesMetadata`).
#[derive(Debug, Clone, PartialEq)]
pub enum MessagesMetadata {
    List(MessagesListMetadata),
    Detail(MessagesDetailMetadata),
}

/// Validate raw message rows: every row must carry a string `id`, a
/// string `text`, and a numeric `created` (TS: `parseMessages`).
pub fn parse_messages(items: &[RawMessageItem]) -> Option<Vec<UserMessageItem>> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let id = item.id.clone()?;
        let text = item.text.clone()?;
        let created = item.created?;
        out.push(UserMessageItem { id, text, created });
    }
    Some(out)
}

/// Parse `messages` interaction metadata with the kind/flow/stage/
/// messageId/project/session/messages gates, the list-page clamp, and
/// the detail `selectedIndex` gate (TS: `parseMessagesMetadata`).
pub fn parse_messages_metadata(state: Option<&InteractionSnapshot>) -> Option<MessagesMetadata> {
    let s = state?;
    if s.kind != "custom" {
        return None;
    }
    if s.flow.as_deref() != Some("messages") {
        return None;
    }
    let message_id = s.message_id?;
    let project_directory = s.project_directory.clone()?;
    let session_id = s.session_id.clone()?;
    let messages = parse_messages(s.messages.as_ref()?)?;
    let page = resolve_messages_page(s.page);

    match s.stage.as_deref() {
        Some("list") => Some(MessagesMetadata::List(MessagesListMetadata {
            message_id,
            project_directory,
            session_id,
            messages,
            page,
        })),
        Some("detail") => {
            let raw = s.selected_index?;
            if !raw.is_finite() || raw.fract() != 0.0 || raw < 0.0 {
                return None;
            }
            Some(MessagesMetadata::Detail(MessagesDetailMetadata {
                message_id,
                project_directory,
                session_id,
                messages,
                page,
                selected_index: raw as i64,
            }))
        }
        _ => None,
    }
}

/// Clamp for list/detail metadata pages: an integer page reads as
/// `Math.max(0, page)`, anything else as `0`
/// (TS: `typeof page === "number" && Number.isInteger(page) ? ... : 0`).
pub fn resolve_messages_page(page: Option<f64>) -> i64 {
    match page {
        Some(v) if v.is_finite() && v.fract() == 0.0 => (v as i64).max(0),
        _ => 0,
    }
}

/// Callback belongs to the message-history flow (TS:
/// `data.startsWith(MESSAGES_CALLBACK_PREFIX)`; `None`/empty is `false`).
pub fn handles_messages_callback(data: Option<&str>) -> bool {
    data.is_some_and(|d| d.starts_with(MESSAGES_CALLBACK_PREFIX))
}

/// Exact `messages:revert` match (TS: `data === MESSAGES_CALLBACK_REVERT`).
pub fn is_messages_revert(data: &str) -> bool {
    data == MESSAGES_CALLBACK_REVERT
}

/// Exact `messages:fork` match (TS: `data === MESSAGES_CALLBACK_FORK`).
pub fn is_messages_fork(data: &str) -> bool {
    data == MESSAGES_CALLBACK_FORK
}

/// Exact `messages:back` match (TS: `data === MESSAGES_CALLBACK_BACK`).
pub fn is_messages_back(data: &str) -> bool {
    data == MESSAGES_CALLBACK_BACK
}

/// Exact `messages:cancel` match (TS: `data === MESSAGES_CALLBACK_CANCEL`).
pub fn is_messages_cancel(data: &str) -> bool {
    data == MESSAGES_CALLBACK_CANCEL
}

/// Stale-callback guard: no metadata, no callback message id, or id
/// mismatch answers `inactive_callback` (TS: `!metadata ||
/// callbackMessageId === null || metadata.messageId !== callbackMessageId`).
pub fn is_callback_message_mismatch(
    metadata_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (metadata_message_id, callback_message_id) {
        (Some(a), Some(b)) if a == b => false,
        _ => true,
    }
}

/// Revert/fork/back require the `detail` stage (TS:
/// `metadata.stage !== "detail"` answers `inactive_callback`).
pub fn is_detail_stage(stage: Option<&str>) -> bool {
    stage == Some("detail")
}

/// Page/select callbacks require the `list` stage (TS:
/// `metadata.stage !== "list"` answers `processing_error`).
pub fn is_list_stage(stage: Option<&str>) -> bool {
    stage == Some("list")
}

/// Page callbacks outside `[0, totalPages)` answer `page_empty_callback`
/// (TS: `page >= totalPages || page < 0`).
pub fn is_messages_page_empty(page: i64, total_pages: i64) -> bool {
    page < 0 || page >= total_pages
}

/// Select callbacks need the `list` stage and an in-bounds index
/// (TS: `messageIndex === null || metadata.stage !== "list"` answers
/// `processing_error`; missing message answers `inactive_callback`).
pub fn is_message_select_valid(index: i64, message_count: usize, is_list_stage: bool) -> bool {
    is_list_stage && index >= 0 && (index as usize) < message_count
}

/// Revert/fork need the selected message to still exist
/// (TS: `metadata.messages[metadata.selectedIndex]` missing answers
/// `fetch_error`).
pub fn has_selected_message(message_count: usize, selected_index: i64) -> bool {
    selected_index >= 0 && (selected_index as usize) < message_count
}

/// A busy foreground session blocks the callback with the busy reply
/// (TS: `isForegroundBusy(deps)` before metadata parsing).
pub fn should_block_when_busy(is_foreground_busy: bool) -> bool {
    is_foreground_busy
}

/// Interaction clear only runs with live messages metadata
/// (TS: `if (metadata) { clear(reason) }`).
pub fn should_clear_messages_interaction(has_metadata: bool) -> bool {
    has_metadata
}
