//! Session-select callback-handler pure gates (slice 84, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/session-callback-handler.ts`
//!   (`handleSessionSelect` dispatch: the `!data ||
//!   !startsWith("session:")` guard, the page-branch (`page !== null`)
//!   vs id-missing (`!sessionId`) gates, the foreground-busy block;
//!   `shouldBlockBackgroundSessionOpen` (`activeInteraction !== null &&
//!   kind !== "inline"`); `handleBackgroundSessionOpen` dispatch: the
//!   `!data` / `!parseBackgroundSessionCallback(data)` guards plus the
//!   `assistant_response`-vs-other `postSelectAction` map;
//!   `selectSessionById` guards: the missing-project gate, the stale
//!   `options.searchState && !sameSessionSearch(...)` gate, the
//!   `isSessionOnDashboard` (`pinned.sessionId === id &&
//!   Boolean(messageId)`) check, the `deliverPick =
//!   postSelectAction === "preview" && Boolean(chat)` gate;
//!   `extractTextParts` (text-part filter + join + `trim` option +
//!   blank-`trim()`-null gate); the `loadLatestAssistantResponse`
//!   reducer (assistant-role + non-summary + `trim: false` text +
//!   max-`created` with `>=` wins); the `sendSessionPickTranscript`
//!   gates (empty-preview `!reply && !quote`, pending-quote
//!   `reply && pendingQuote`); `sessionPickSendGapMs`
//!   (`VITEST ? 0 : 1000`)).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `command_session_gates::SESSION_CALLBACK_PREFIX`,
//!   `SESSION_PAGE_CALLBACK_PREFIX`, `BACKGROUND_SESSION_CALLBACK_PREFIX`,
//!   `parse_session_page_callback`, `parse_session_id_callback`,
//!   `parse_background_session_callback` (the parses behind the
//!   dispatch branches above).
//! - `session_pick_catalog_gates::find_eligible_reply`,
//!   `find_pending_user_prompt`, `find_message_by_id`,
//!   `build_session_pick_quote` (the pick/quote builders the transcript
//!   gates branch on; this slice takes their outcomes as bools).
//! - `session_search_callback_gates` (the `session:search*` dispatch and
//!   search-state gates in the same TS file).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, transitions, `clear`).
//! - `getCurrentProject` reads, `opencodeClient.session.*` calls,
//!   `searchSessions` / `loadSessionPage` / `buildSessionSelectionMenuView`
//!   builds, `switchToProject`, `attachToSession`, `applySessionSettings`,
//!   `ensureActiveInlineMenu`, busy-blocked replies,
//!   `renderAssistantFinalPartsSafe` / `sendRenderedBotPart`.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `messageId` values are `i64`
//! (`typeof === "number"` in TS), page presence arrives as a bool
//! (the `Option<u64>` parse lives in `command_session_gates`),
//! `Symbol` tokens are not modelled here (staleness arrives as a bool),
//! and `VITEST` arrives as an explicit `is_vitest: bool` so the gap
//! stays pure and testable on every host.
//!
//! Step 1 (GREEN): pure port of the TS contract above; all parity
//! tests pass.

/// Message limit for the latest-assistant-response read (TS:
/// `LATEST_ASSISTANT_RESPONSE_MESSAGES_LIMIT`).
pub const LATEST_ASSISTANT_RESPONSE_MESSAGES_LIMIT: usize = 20;

/// Page size for the session-pick transcript read (TS:
/// `SESSION_PICK_PAGE_SIZE`).
pub const SESSION_PICK_PAGE_SIZE: usize = 20;

/// Gap between session-pick sends (TS: `SESSION_PICK_SEND_GAP_MS`).
pub const SESSION_PICK_SEND_GAP_MS: u64 = 1000;

/// Post-select action for a background open (TS: the
/// `postSelectAction` passed to `selectSessionById` from
/// `handleBackgroundSessionOpen`: `"latest_assistant_response"` only for
/// an assistant-response notice, `"none"` otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundPostAction {
    LatestAssistantResponse,
    None,
}

/// One message part the local text extractor inspects (TS: the
/// `{ type, text }` parts read by `extractTextParts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTextPart {
    pub part_type: String,
    pub text: Option<String>,
}

/// One session message the latest-assistant reducer inspects (TS: the
/// `SessionMessageLike` rows read by `loadLatestAssistantResponse`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantCandidate {
    pub role: String,
    pub summary: bool,
    pub created: i64,
    pub parts: Vec<SessionTextPart>,
}

/// Newest assistant text (TS: the `{ text, created }` accumulator in
/// `loadLatestAssistantResponse`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestPick {
    pub text: String,
    pub created: i64,
}

/// Session-select dispatch guard: no data never handles; anything under
/// `session:` does (TS: `if (!callbackQuery?.data ||
/// !data.startsWith(SESSION_CALLBACK_PREFIX)) return false` in
/// `handleSessionSelect`).
pub fn handles_session_select_callback(_data: Option<&str>) -> bool {
    match _data {
        Some(data) => data.starts_with("session:"),
        None => false,
    }
}

/// Page branch of the session-select handler (TS: `if (page !== null)`
/// loads the page view).
pub fn is_session_select_page_branch(_has_page: bool) -> bool {
    _has_page
}

/// Missing-id branch of the session-select handler (TS: `if (!sessionId)`
/// answers `callback.processing_error`).
pub fn is_session_select_id_missing(_has_session_id: bool) -> bool {
    !_has_session_id
}

/// Foreground-busy block for the session-select handler (TS:
/// `if (isForegroundBusy(deps)) { await replyBusyBlocked(ctx); return true; }`).
pub fn should_block_session_select_on_busy(_is_busy: bool) -> bool {
    _is_busy
}

/// Background-open dispatch guard (TS: `if (!data) return false` plus
/// `const payload = parseBackgroundSessionCallback(data); if (!payload)
/// return false` in `handleBackgroundSessionOpen`; the parse itself lives
/// in `command_session_gates` and arrives here as `has_payload`).
pub fn handles_background_session_open(_has_data: bool, _has_payload: bool) -> bool {
    _has_data && _has_payload
}

/// Background-open interaction block (TS: `shouldBlockBackgroundSessionOpen`:
/// `activeInteraction !== null && activeInteraction.kind !== "inline"`).
pub fn should_block_background_session_open(
    _has_active_interaction: bool,
    _interaction_is_inline: bool,
) -> bool {
    _has_active_interaction && !_interaction_is_inline
}

/// Background-open post-select action (TS: `postSelectAction:
/// payload.kind === "assistant_response" ? "latest_assistant_response" :
/// "none"`).
pub fn background_post_select_action(
    _is_assistant_response: bool,
) -> BackgroundPostAction {
    if _is_assistant_response {
        BackgroundPostAction::LatestAssistantResponse
    } else {
        BackgroundPostAction::None
    }
}

/// Stale search-state gate for `selectSessionById` (TS:
/// `if (options.searchState && !sameSessionSearch(deps,
/// options.searchState))` returns early; sameness arrives as a bool).
pub fn is_stale_search_pick(_has_search_state: bool, _same_search: bool) -> bool {
    _has_search_state && !_same_search
}

/// Dashboard-skip check for `selectSessionById` (TS:
/// `pinnedState.sessionId === session.id && Boolean(pinnedState.messageId)`).
pub fn is_session_on_dashboard(
    _pinned_session_id: Option<&str>,
    _pinned_message_id: Option<i64>,
    _session_id: &str,
) -> bool {
    _pinned_session_id == Some(_session_id) && _pinned_message_id.is_some()
}

/// Pick-transcript load gate (TS: `const deliverPick =
/// options.postSelectAction === "preview" && Boolean(ctx.chat)`).
pub fn should_load_pick_transcript(_is_preview_action: bool, _has_chat: bool) -> bool {
    _is_preview_action && _has_chat
}

/// Local text-part extractor (TS: `extractTextParts`: keeps
/// `type === "text"` parts with string text, joins with `""`, trims
/// unless `trim === false`, and returns `null` when the trimmed view is
/// empty).
pub fn extract_session_text_parts(
    _parts: &[SessionTextPart],
    _trim_text: bool,
) -> Option<String> {
    let mut text = String::new();
    let mut has_text = false;
    for part in _parts {
        if part.part_type == "text" {
            if let Some(t) = part.text.as_ref() {
                text.push_str(t);
                has_text = true;
            }
        }
    }
    if !has_text {
        return None;
    }
    let normalized = if _trim_text {
        text.trim().to_string()
    } else {
        text
    };
    if normalized.trim().is_empty() {
        return None;
    }
    Some(normalized)
}

/// Latest-assistant reducer (TS: the `reduce` in
/// `loadLatestAssistantResponse`: skips non-assistant roles and summary
/// rows, reads `extractTextParts(parts, { trim: false })`, skips
/// blank/None texts, and keeps the max `created` with `>=` so a later
/// equal row wins).
pub fn pick_latest_assistant_text(_messages: &[AssistantCandidate]) -> Option<LatestPick> {
    let mut latest: Option<LatestPick> = None;
    for message in _messages {
        if message.role != "assistant" || message.summary {
            continue;
        }
        let text = match extract_session_text_parts(&message.parts, false) {
            Some(t) => t,
            None => continue,
        };
        let created = message.created;
        match latest.as_ref() {
            Some(current) if created < current.created => continue,
            _ => {
                latest = Some(LatestPick { text, created });
            }
        }
    }
    latest
}

/// Empty-preview gate for `sendSessionPickTranscript` (TS: `if (quote)
/// send it; else if (!reply) send "sessions.preview.empty"`).
pub fn should_send_empty_session_preview(_has_reply: bool, _has_quote: bool) -> bool {
    !_has_reply && !_has_quote
}

/// Pending-prompt gate for `sendSessionPickTranscript` (TS: after the
/// reply parts, `findPendingUserPrompt(messages, reply.created)` plus its
/// quote must both exist to send the extra quote; without a reply the
/// function already returned).
pub fn should_show_pending_prompt(_has_reply: bool, _has_pending_quote: bool) -> bool {
    _has_reply && _has_pending_quote
}

/// Send gap for session-pick messages (TS: `process.env.VITEST ? 0 :
/// SESSION_PICK_SEND_GAP_MS`).
pub fn session_pick_send_gap_ms(_is_vitest: bool) -> u64 {
    if _is_vitest {
        0
    } else {
        SESSION_PICK_SEND_GAP_MS
    }
}
