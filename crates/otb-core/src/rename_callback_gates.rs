//! Rename callback-handler pure gates (slice 85, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/rename-callback-handler.ts`
//!   (`handleRenameCancel` dispatch: the `!data || data !==
//!   RENAME_CANCEL_CALLBACK` guard; the not-waiting gate, the
//!   `interactionState?.kind !== "rename"` gate, the
//!   `!isActiveMessage(callbackMessageId)` stale-message gate;
//!   `handleRenameTextAnswer` dispatch: the `!isWaitingForName()`,
//!   `text === undefined`, and `text.startsWith("/")` guards; the
//!   wrong-interaction gate, the missing-`getSessionInfo()` gate, the
//!   `text.trim()` empty-title gate; `getCallbackMessageId` (missing
//!   message / missing `message_id` / non-`number` / number);
//!   `clearRenameInteraction` (only clears when `kind === "rename"`)).
//! - `src/bot/menus/rename-menu.ts` (`RENAME_CANCEL_CALLBACK`).
//! - `src/app/managers/rename-manager.ts` (`isActiveMessage`:
//!   `activeMessageId !== null && activeMessageId === messageId`;
//!   `isWaitingForName`: state non-null; `getSessionInfo`: null state or
//!   empty `sessionId` reads as missing).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `question_permission_inline_gates` (the shared `feedback.ts`
//!   cancel/alert/notify helpers used via `cancelPrompt`).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `reply`,
//!   `deleteMessage`, `editMessageText`).
//! - `opencodeClient.session.update` calls, `setCurrentSession`,
//!   `pinnedMessageManager.onSessionChange`, `renameManager` /
//!   `interactionManager` reads and clears.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `message_id` values are `i64`
//! (`typeof === "number"` in TS), the `Symbol`/manager tokens are not
//! modelled here (waiting-ness, interaction kind, message activeness,
//! and session-info presence arrive as bools/options), and
//! `String.prototype.trim` maps to `str::trim` (parity fixtures use
//! ASCII whitespace where the two coincide).
//!
//! Step 2 (GREEN): ported gates; all behavior parity tests pass.

/// Cancel callback data for the rename flow (TS:
/// `RENAME_CANCEL_CALLBACK` in `src/bot/menus/rename-menu.ts`).
pub const RENAME_CANCEL_CALLBACK: &str = "rename:cancel";

/// Interaction kind guarding the rename flow (TS: `kind === "rename"`
/// in `rename-callback-handler.ts`, `getPayload("rename")` in
/// `rename-manager.ts`).
pub const RENAME_INTERACTION_KIND: &str = "rename";

/// Raw `message_id` shape behind `getCallbackMessageId` (TS: no message
/// reads as `null`, a message without `message_id` reads as `null`, a
/// non-`number` `message_id` reads as `null`, a `number` passes
/// through).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageIdRaw {
    Missing,
    NonNumber,
    Number(i64),
}

/// Cancel-branch classification for `handleRenameCancel` (TS: the
/// `!isWaitingForName()`, wrong-interaction, and stale-message early
/// returns, else the cancel proceeds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameCancelOutcome {
    InactiveNotWaiting,
    InactiveWrongInteraction,
    InactiveStaleMessage,
    ProceedCancel,
}

/// Text-branch classification for `handleRenameTextAnswer` (TS: the
/// wrong-interaction, missing-session-info, and empty-title early
/// returns, else the rename proceeds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameTextOutcome {
    WrongInteraction,
    MissingSession,
    EmptyTitle,
    ProceedRename,
}

/// Rename-cancel dispatch guard (TS: `if (!data || data !==
/// RENAME_CANCEL_CALLBACK) return false` in `handleRenameCancel`).
pub fn handles_rename_cancel_callback(data: Option<&str>) -> bool {
    data == Some(RENAME_CANCEL_CALLBACK)
}

/// Callback message-id extractor (TS: `getCallbackMessageId`: `null`
/// without a message, without `message_id`, or for a non-`number`
/// `message_id`; the number otherwise).
pub fn callback_message_id(has_message: bool, raw: MessageIdRaw) -> Option<i64> {
    if !has_message {
        return None;
    }
    match raw {
        MessageIdRaw::Number(id) => Some(id),
        MessageIdRaw::Missing | MessageIdRaw::NonNumber => None,
    }
}

/// Rename-interaction clear guard (TS: `clearRenameInteraction` only
/// clears when `state?.kind === "rename"`).
pub fn should_clear_rename_interaction(kind: Option<&str>) -> bool {
    kind == Some(RENAME_INTERACTION_KIND)
}

/// Rename-cancel branch classifier (TS: `!isWaitingForName()` answers
/// `rename.inactive_callback`; a non-`"rename"` interaction clears the
/// manager and answers inactive; a non-active callback message answers
/// inactive; otherwise the cancel proceeds).
pub fn classify_rename_cancel(
    waiting_for_name: bool,
    interaction_kind: Option<&str>,
    is_active_message: bool,
) -> RenameCancelOutcome {
    if !waiting_for_name {
        return RenameCancelOutcome::InactiveNotWaiting;
    }
    if interaction_kind != Some(RENAME_INTERACTION_KIND) {
        return RenameCancelOutcome::InactiveWrongInteraction;
    }
    if !is_active_message {
        return RenameCancelOutcome::InactiveStaleMessage;
    }
    RenameCancelOutcome::ProceedCancel
}

/// Rename text-answer routing guard (TS: `handleRenameTextAnswer`
/// returns `false` when `!isWaitingForName()`, when `text ===
/// undefined`, or when `text.startsWith("/")`; otherwise it handles
/// the message).
pub fn should_route_rename_text(waiting_for_name: bool, text: Option<&str>) -> bool {
    if !waiting_for_name {
        return false;
    }
    match text {
        Some(t) => !is_rename_command_text(t),
        None => false,
    }
}

/// Rename text-answer branch classifier (TS: a non-`"rename"`
/// interaction clears and replies `rename.inactive`; missing session
/// info clears and replies inactive; an empty `trim()` replies
/// `rename.empty_title`; otherwise the rename proceeds).
pub fn classify_rename_text(
    interaction_kind: Option<&str>,
    has_session_info: bool,
    trimmed_title: &str,
) -> RenameTextOutcome {
    if interaction_kind != Some(RENAME_INTERACTION_KIND) {
        return RenameTextOutcome::WrongInteraction;
    }
    if !has_session_info {
        return RenameTextOutcome::MissingSession;
    }
    if trimmed_title.is_empty() {
        return RenameTextOutcome::EmptyTitle;
    }
    RenameTextOutcome::ProceedRename
}

/// Rename-title normalizer (TS: `const newTitle = text.trim()` in
/// `handleRenameTextAnswer`).
pub fn normalize_rename_title(text: &str) -> String {
    text.trim().to_string()
}

/// Slash-command guard for rename text answers (TS:
/// `if (text.startsWith("/")) return false`: commands never become
/// titles).
pub fn is_rename_command_text(text: &str) -> bool {
    text.starts_with('/')
}

/// Active rename-message check (TS: `RenameManager::isActiveMessage`:
/// `activeMessageId !== null && activeMessageId === messageId`).
pub fn is_active_rename_message(
    stored_message_id: Option<i64>,
    candidate_message_id: Option<i64>,
) -> bool {
    match (stored_message_id, candidate_message_id) {
        (Some(stored), Some(candidate)) => stored == candidate,
        _ => false,
    }
}

/// Rename session-info presence check (TS:
/// `RenameManager::getSessionInfo`: a null state or an empty
/// `sessionId` reads as missing).
pub fn has_rename_session(session_id: Option<&str>) -> bool {
    match session_id {
        Some(id) => !id.is_empty(),
        None => false,
    }
}
