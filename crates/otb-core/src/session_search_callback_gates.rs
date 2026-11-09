//! Session-search callback-handler pure gates (slice 82, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/session-callback-handler.ts`
//!   (`sessionSearchState` kind/flow gate, `sameSessionSearch`
//!   token/stage equality, `handleSessionSearchCallback` dispatch: the
//!   `!data` / `data !== "session:search" &&
//!   !startsWith("session:search:")` guard, the exact `session:search` /
//!   `...:again` / `...:cancel` matches, the `...:page:` / `...:result:`
//!   prefix matches, the page `/^\d+$/` + `typeof query === "string"`
//!   gate, the result digits-plus-`ids`-array-plus-in-bounds gate, the
//!   `results`-stage + `messageId`-equality `inactive_callback` gate, the
//!   blank-query `trim()` gate, the text-input `state && input-stage +
//!   has-text` gate, the foreground-busy block; `handleSessionSearchTextInput`
//!   handled gate)
//!
//! Already ported elsewhere and NOT repeated here:
//! - `command_session_gates::cap_session_search_query` (the typed-query
//!   `slice(0, SESSION_SEARCH_QUERY_MAX_LENGTH)` cap),
//!   `command_session_gates::session_matches_search` (the needle match
//!   behind `searchSessions`), `parseSessionPageCallback`,
//!   `parseSessionIdCallback`, `parseBackgroundSessionCallback`,
//!   `SESSION_CALLBACK_PREFIX` (the `handleSessionSelect` dispatch).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, transitions, `clear`).
//! - `getCurrentProject` reads, `searchSessions` / `selectSessionById`
//!   calls, `buildSessionSearchView` keyboard builds,
//!   `ensureActiveInlineMenu`, busy-blocked replies.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `token` (a TS `Symbol`) is modelled as a
//! `u64` id so equality is structural, `messageId` values are `i64`
//! (`typeof === "number"` in TS), page/result raws follow `/^\d+$/`
//! (empty, signed, fractional, and alpha raws all reject; leading zeros
//! read as decimal). `is_session_search_message_mismatch` treats a
//! missing stored `messageId` as a mismatch even when the callback id is
//! also missing (TS `undefined !== undefined` is `false`); the parity
//! test requires `(None, None) -> true` so a storeless state always
//! answers `inactive_callback`.

/// Exact callback opening the session search (TS: the `"session:search"` literal).
pub const SESSION_SEARCH_CALLBACK: &str = "session:search";

/// Dispatch shape for every search sub-callback (TS: the inline
/// `"session:search:"` literal in `handleSessionSearchCallback`).
pub const SESSION_SEARCH_DISPATCH_PREFIX: &str = "session:search:";

/// `session:search:page:<n>` callback prefix (TS: the
/// `"session:search:page:"` literal).
pub const SESSION_SEARCH_PAGE_PREFIX: &str = "session:search:page:";

/// `session:search:result:<n>` callback prefix (TS: the
/// `"session:search:result:"` literal).
pub const SESSION_SEARCH_RESULT_PREFIX: &str = "session:search:result:";

/// Back-to-input search callback (TS: the `"session:search:again"` literal).
pub const SESSION_SEARCH_AGAIN: &str = "session:search:again";

/// Search-cancel callback (TS: the `"session:search:cancel"` literal).
pub const SESSION_SEARCH_CANCEL: &str = "session:search:cancel";

/// Flattened interaction snapshot the search-state reader inspects
/// (TS: `InteractionState | null` with `kind` + a `session-search`
/// `metadata` bag).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSearchSnapshot {
    pub kind: String,
    pub flow: Option<String>,
    pub stage: Option<String>,
    pub token: Option<u64>,
    pub query: Option<String>,
    pub message_id: Option<i64>,
    pub ids: Option<Vec<String>>,
}

/// Live `session-search` flow state (TS: the `sessionSearchState(deps)`
/// metadata with `flow === "session-search"`).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSearchFlow {
    pub stage: String,
    pub token: u64,
}

/// Search-callback dispatch guard: no data never handles; the exact
/// `session:search` opener and anything under `session:search:` does
/// (TS: `!data || (data !== "session:search" &&
/// !data.startsWith("session:search:"))` returns `false`).
pub fn handles_session_search_callback(data: Option<&str>) -> bool {
    match data {
        Some(d) if !d.is_empty() => {
            d == SESSION_SEARCH_CALLBACK || d.starts_with(SESSION_SEARCH_DISPATCH_PREFIX)
        }
        _ => false,
    }
}

/// Exact `session:search` opener match (TS: `data === "session:search"`).
pub fn is_session_search_start(data: &str) -> bool {
    data == SESSION_SEARCH_CALLBACK
}

/// Exact `session:search:cancel` match (TS: `data === "session:search:cancel"`).
pub fn is_session_search_cancel(data: &str) -> bool {
    data == SESSION_SEARCH_CANCEL
}

/// Exact `session:search:again` match (TS: `data === "session:search:again"`).
pub fn is_session_search_again(data: &str) -> bool {
    data == SESSION_SEARCH_AGAIN
}

/// `session:search:page:` prefix match (TS:
/// `data.startsWith("session:search:page:")`).
pub fn is_session_search_page(data: &str) -> bool {
    data.starts_with(SESSION_SEARCH_PAGE_PREFIX)
}

/// `session:search:result:` prefix match (TS:
/// `data.startsWith("session:search:result:")`).
pub fn is_session_search_result(data: &str) -> bool {
    data.starts_with(SESSION_SEARCH_RESULT_PREFIX)
}

fn is_digits_raw(raw: &str) -> bool {
    !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit())
}

/// Digits-only page raw behind `session:search:page:` (TS:
/// `/^\d+$/.test(raw)` then `Number(raw)`; empty/signed/alpha reject).
pub fn parse_session_search_page_raw(raw: &str) -> Option<u64> {
    if !is_digits_raw(raw) {
        return None;
    }
    raw.parse::<u64>().ok()
}

/// Page requests need a digits-only raw and a string query; otherwise the
/// callback is consumed as a no-op (TS: `!/^\d+$/.test(raw) ||
/// typeof state.query !== "string"` returns `true` without acting).
pub fn is_session_search_page_request_valid(raw: &str, query_is_string: bool) -> bool {
    query_is_string && parse_session_search_page_raw(raw).is_some()
}

/// Result selection behind `session:search:result:`: digits-only raw
/// indexing into the live `ids` array (TS: `/^\d+$/.test(raw) &&
/// Array.isArray(state.ids) ? state.ids[Number(raw)] : undefined`, then
/// the `typeof id !== "string"` failure gate).
pub fn resolve_session_search_result_id(ids: Option<&[String]>, raw: &str) -> Option<String> {
    let list = ids?;
    if list.is_empty() || !is_digits_raw(raw) {
        return None;
    }
    let index: usize = raw.parse().ok()?;
    list.get(index).cloned()
}

/// Live `session-search` flow state: `custom` kind plus
/// `flow === "session-search"` (TS: `sessionSearchState(deps)`).
pub fn session_search_flow_state(
    state: Option<&SessionSearchSnapshot>,
) -> Option<SessionSearchFlow> {
    let s = state?;
    if s.kind != "custom" {
        return None;
    }
    if s.flow.as_deref() != Some("session-search") {
        return None;
    }
    Some(SessionSearchFlow {
        stage: s.stage.clone()?,
        token: s.token?,
    })
}

/// Search page/result callbacks require the `results` stage (TS:
/// `state.stage !== "results"` answers `inactive_callback`).
pub fn is_session_search_results_stage(stage: Option<&str>) -> bool {
    stage == Some("results")
}

/// Search text input requires the `input` stage (TS:
/// `state.stage !== "input"` returns `false`).
pub fn is_session_search_input_stage(stage: Option<&str>) -> bool {
    stage == Some("input")
}

/// Stale-search guard: the live interaction must carry the same token
/// and stage as the stored search state (TS: `sameSessionSearch`:
/// `!!current && current.token === state.token && current.stage ===
/// state.stage`).
pub fn is_same_session_search(
    active_token: Option<u64>,
    active_stage: Option<&str>,
    state_token: Option<u64>,
    state_stage: Option<&str>,
) -> bool {
    match (active_token, active_stage, state_token, state_stage) {
        (Some(a), Some(b), Some(c), Some(d)) => a == c && b == d,
        _ => false,
    }
}

/// Stale-callback guard: no stored message id, no callback message id,
/// or id mismatch answers `inactive_callback` (TS:
/// `state.messageId !== ctx.callbackQuery?.message?.message_id`).
pub fn is_session_search_message_mismatch(
    state_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (state_message_id, callback_message_id) {
        (Some(a), Some(b)) => a != b,
        _ => true,
    }
}

/// Combined `inactive_callback` gate for search page/result callbacks
/// (TS: `!state || state.stage !== "results" || state.messageId !==
/// callbackMessageId`).
pub fn should_answer_session_search_inactive(
    has_state: bool,
    is_results_stage: bool,
    messages_match: bool,
) -> bool {
    !has_state || !is_results_stage || !messages_match
}

/// Blank typed queries are consumed without searching (TS:
/// `if (!query.trim()) return true` in `handleSessionSearchTextInput`).
pub fn is_blank_session_search_query(query: &str) -> bool {
    query.trim().is_empty()
}

/// Search text input is handled only with live state, the `input`
/// stage, and message text (TS: `!state || state.stage !== "input" ||
/// ctx.message?.text === undefined` returns `false`).
pub fn handles_session_search_text_input(
    has_state: bool,
    is_input_stage: bool,
    has_text: bool,
) -> bool {
    has_state && is_input_stage && has_text
}

/// A busy foreground session blocks the callback with the busy reply
/// (TS: `isForegroundBusy(deps)` before state parsing).
pub fn should_block_session_callback_when_busy(is_foreground_busy: bool) -> bool {
    is_foreground_busy
}
