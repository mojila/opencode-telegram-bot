//! Event-delivery pure gates (slice 47, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/events/telegram-event-delivery.ts`
//!   (`getLowerCaseErrorMessage` lower-casing, `isMessageNotModifiedError`
//!   tolerated-edit classifier, `isMessageToDeleteMissingError`
//!   tolerated-delete classifier, `TelegramEventDelivery` draft-id
//!   numbering starting at 1 with `resetDraftIds`)
//! - `src/bot/events/background-notice-delivery.ts` (`formatShortSessionId`
//!   short id, `getBackgroundSessionLabel` title branch,
//!   `formatBackgroundSessionNotification` kind -> message-key mapping)
//! - `src/bot/events/failed-tool-event.ts` (`buildFailedToolEvent`
//!   synthetic lost-call event: `{callId}:lost` id, `message.part.updated`
//!   type, `error` status, empty error text, `start` from tool state time or
//!   `now`, `end` at `now`, `{}` default input, optional metadata)
//!
//! No grammy/Telegram I/O, no network/fetch, no timers live here — only
//! pure predicates, classifiers, id sequencing, and small value builders.
//!
//! Index contract (documented): the TS sources count `String.length` /
//! `slice` in UTF-16 code units. The Rust port counts `usize` indices in
//! `char`s (Unicode scalar values). Parity tests use ASCII-only inputs
//! where UTF-16 units, bytes, and chars coincide, so the port is exact
//! there. For non-BMP input the split positions may differ from TS by
//! design. `to_lowercase()` is Unicode-aware; ASCII case-folding matches
//! TS `toLowerCase()` exactly.
//!
//! DEVIATIONS from TS:
//! 1. `getLowerCaseErrorMessage` accepts `unknown` (Error objects,
//!    strings, other values stringified via `String(...)`). The Rust port
//!    takes the already-stringified message as `&str`; callers stringify
//!    first. Lower-casing itself is the ported logic.
//! 2. `getBackgroundSessionLabel` falls back to the translated
//!    `background.session_fallback` string (needs `t()` + i18n). The port
//!    splits this: `select_background_session_title` returns the trimmed
//!    non-blank title or `None`, and callers combine it with
//!    `format_short_session_id`. The `None` case means "use the session
//!    fallback text".
//! 3. `formatBackgroundSessionNotification` returns translated text via
//!    `t()`. The port maps the kind to its i18n message key
//!    (`background_notice_message_key`); translation stays out.
//! 4. `buildFailedToolEvent` input/metadata are `{[key: string]: unknown}`.
//!    The port models values as `String`; structure (default `{}`, optional
//!    metadata spread, time selection) is the ported logic.
//! 5. Network sends, draft rendering, keyboard building, settings reads,
//!    and destination routing stay out; only the pure decisions above
//!    live here.

use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Delivery error classifiers (`telegram-event-delivery.ts`)
// ---------------------------------------------------------------------------

/// Lower-cased view of an already-stringified error message (TS:
/// `getLowerCaseErrorMessage`: `error.message` for `Error`, `String(error)`
/// otherwise, then `.toLowerCase()`).
pub fn lower_case_error_message(message: &str) -> String {
    message.to_lowercase()
}

/// Whether an edit failure is the tolerated "changes nothing" case (TS:
/// `isMessageNotModifiedError`: lower-cased message includes
/// `"message is not modified"`).
pub fn is_message_not_modified_message(message: &str) -> bool {
    lower_case_error_message(message).contains("message is not modified")
}

/// Whether a delete failure is a tolerated "already gone" case (TS:
/// `isMessageToDeleteMissingError`: lower-cased message includes
/// `"message to delete not found"` or
/// `"message identifier is not specified"`).
pub fn is_message_to_delete_missing_message(message: &str) -> bool {
    let lower = lower_case_error_message(message);
    lower.contains("message to delete not found")
        || lower.contains("message identifier is not specified")
}

// ---------------------------------------------------------------------------
// Draft-id numbering (`TelegramEventDelivery`)
// ---------------------------------------------------------------------------

/// Sequential draft ids for the stream's in-flight message (TS:
/// `TelegramEventDelivery.nextDraftId`, starting at 1; `sendDraftPart`
/// hands out the current id then increments; `resetDraftIds` returns to 1).
#[allow(dead_code)]
pub struct DraftIdSequence {
    next: u64,
}

impl DraftIdSequence {
    /// Fresh sequence starting at id 1 (TS: `nextDraftId = 1`).
    pub fn new() -> Self {
        Self { next: 1 }
    }

    /// The current draft id, advancing the sequence (TS: `sendDraftPart`
    /// numbering).
    pub fn next_draft_id(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
    }

    /// Back to id 1 (TS: `resetDraftIds`).
    pub fn reset(&mut self) {
        self.next = 1;
    }
}

// ---------------------------------------------------------------------------
// Background notice labels (`background-notice-delivery.ts`)
// ---------------------------------------------------------------------------

/// At most 8 leading characters of a session id (TS:
/// `formatShortSessionId`: `length <= 8` keeps the whole id, else
/// `slice(0, 8)`; char-based, see index contract).
pub fn format_short_session_id(session_id: &str) -> String {
    if session_id.chars().count() <= 8 {
        session_id.to_string()
    } else {
        session_id.chars().take(8).collect()
    }
}

/// The trimmed non-blank session title, if any (TS:
/// `getBackgroundSessionLabel` title branch: `title.trim()` when truthy).
/// `None` means the caller falls back to the translated session-fallback
/// text built from [`format_short_session_id`].
pub fn select_background_session_title(session_title: Option<&str>) -> Option<String> {
    match session_title {
        Some(title) => {
            let trimmed = title.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => None,
    }
}

/// Background-session notice kinds (TS:
/// `BackgroundSessionNotificationKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundNoticeKind {
    AssistantResponse,
    QuestionAsked,
    PermissionAsked,
}

/// The i18n message key for a notice kind (TS:
/// `formatBackgroundSessionNotification` `t(...)` key:
/// `background.assistant_response` / `background.question_asked` /
/// `background.permission_asked`, each rendered with the session label).
pub fn background_notice_message_key(kind: BackgroundNoticeKind) -> &'static str {
    match kind {
        BackgroundNoticeKind::AssistantResponse => "background.assistant_response",
        BackgroundNoticeKind::QuestionAsked => "background.question_asked",
        BackgroundNoticeKind::PermissionAsked => "background.permission_asked",
    }
}

// ---------------------------------------------------------------------------
// Lost-call event (`failed-tool-event.ts`)
// ---------------------------------------------------------------------------

/// The `ToolInfo` subset `buildFailedToolEvent` reads (TS:
/// `src/app/managers/summary-aggregation-manager.ts` `ToolInfo`:
/// ids, tool name, state time, input, metadata).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FailedToolCall {
    pub session_id: String,
    pub message_id: String,
    pub call_id: String,
    pub tool: String,
    /// `state.time.start` when the tool state carries time (TS:
    /// `"time" in tool.state && tool.state.time ? ...start : now`).
    pub state_start: Option<u64>,
    /// `tool.input ?? {}` (values simplified to `String`, see DEVIATIONS).
    pub input: Option<BTreeMap<String, String>>,
    /// Spread onto the error state only when present (TS:
    /// `...(tool.metadata ? { metadata } : {})`).
    pub metadata: Option<BTreeMap<String, String>>,
}

/// The synthetic event for a call OpenCode stopped without ending (TS:
/// `buildFailedToolEvent` return value: `message.part.updated` event with
/// a `tool` part in `error` state).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FailedToolEvent {
    /// `{callId}:lost` (TS: `` `${tool.callId}:lost` ``).
    pub id: String,
    /// Always `"message.part.updated"`.
    pub event_type: String,
    pub session_id: String,
    pub message_id: String,
    pub call_id: String,
    pub tool: String,
    /// Always `"error"`.
    pub status: String,
    /// Always `""` (no failure detail is known).
    pub error: String,
    /// `tool.input ?? {}`.
    pub input: BTreeMap<String, String>,
    /// Whether `tool.metadata` was present and carried over.
    pub has_metadata: bool,
    /// `state.time.start`, or `now` when the state carries no time.
    pub start: u64,
    /// Always `now`.
    pub end: u64,
}

/// Builds the synthetic lost-call event (TS: `buildFailedToolEvent`).
pub fn build_failed_tool_event(call: &FailedToolCall, now: u64) -> FailedToolEvent {
    FailedToolEvent {
        id: format!("{}:lost", call.call_id),
        event_type: "message.part.updated".to_string(),
        session_id: call.session_id.clone(),
        message_id: call.message_id.clone(),
        call_id: call.call_id.clone(),
        tool: call.tool.clone(),
        status: "error".to_string(),
        error: String::new(),
        input: call.input.clone().unwrap_or_default(),
        has_metadata: call.metadata.is_some(),
        start: call.state_start.unwrap_or(now),
        end: now,
    }
}
