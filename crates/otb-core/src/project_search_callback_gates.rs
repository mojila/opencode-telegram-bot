//! Project-search callback-handler pure gates (slice 81, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/menus/project-selection-menu.ts`
//!   (`PROJECT_SEARCH_CALLBACK`, `PROJECT_SEARCH_PAGE_PREFIX`,
//!   `PROJECT_SEARCH_RESULT_PREFIX`, `PROJECT_SEARCH_AGAIN`,
//!   `PROJECT_SEARCH_CANCEL`; the `projects:search:` dispatch shape)
//! - `src/bot/callbacks/project-callback-handler.ts`
//!   (`handleProjectSearchCallback` dispatch: the `!data` /
//!   `data !== PROJECT_SEARCH_CALLBACK && !startsWith("projects:search:")`
//!   guard, the exact `projects:search` / `...:again` / `...:cancel`
//!   matches, the `...:page:` / `...:result:` prefix matches, the page
//!   ` /^\d+$/` + `typeof query === "string"` gate, the result
//!   digits-plus-`ids`-array-plus-in-bounds gate, the `results`-stage +
//!   `messageId`-equality `inactive_callback` gate, the token/stage
//!   `isSameSearch` gate, the blank-query `trim()` gate, the text-input
//!   `state && input-stage && has-text` gate, the foreground-busy block;
//!   `handleProjectSelect` dispatch: page-callback vs `project:` prefix vs
//!   neither, the `project:<id>` id extraction)
//!
//! Page-number parsing for `projects:page:<n>` already lives in
//! `command_session_gates::parse_project_page_callback` and the per-project
//! search-needle match in `project_matches_search`; this slice takes the
//! `projectId` as an already-parsed `Option<u64>` so the dispatch gate
//! stays self-contained without re-implementing that parse. The
//! `PROJECT_PAGE_CALLBACK_PREFIX` value below repeats the same string so
//! the dispatch tests read against this slice (the same duplication
//! pattern `message_history_callback_gates` uses for the `messages:`
//! callbacks).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, transitions, `clear`).
//! - `getListedProjects` / `getProjects` reads, `filterProjectsForSearch`
//!   (needs the live current project + git worktree context),
//!   `buildProjectSearchView` / `buildProjectsMenuView` keyboard builds,
//!   `switchToProject`, `ensureActiveInlineMenu`, busy-blocked replies.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `token` (a TS `Symbol`) is modelled as a
//! `u64` id so equality is structural, `messageId` values are `i64`
//! (`typeof === "number"` in TS), page/result raws follow `/^\d+$/`
//! (empty, signed, fractional, and alpha raws all reject; leading zeros
//! read as decimal), and an empty `project:` rest reads as `None` (TS
//! passes `""` into a project lookup that fails with `select_error`).

/// Exact callback opening the project search (TS: `PROJECT_SEARCH_CALLBACK`).
pub const PROJECT_SEARCH_CALLBACK: &str = "projects:search";

/// Dispatch shape for every search sub-callback (TS: the inline
/// `"projects:search:"` literal in `handleProjectSearchCallback`).
pub const PROJECT_SEARCH_DISPATCH_PREFIX: &str = "projects:search:";

/// `projects:search:page:<n>` callback prefix (TS: `PROJECT_SEARCH_PAGE_PREFIX`).
pub const PROJECT_SEARCH_PAGE_PREFIX: &str = "projects:search:page:";

/// `projects:search:result:<n>` callback prefix
/// (TS: `PROJECT_SEARCH_RESULT_PREFIX`).
pub const PROJECT_SEARCH_RESULT_PREFIX: &str = "projects:search:result:";

/// Back-to-input search callback (TS: `PROJECT_SEARCH_AGAIN`).
pub const PROJECT_SEARCH_AGAIN: &str = "projects:search:again";

/// Search-cancel callback (TS: `PROJECT_SEARCH_CANCEL`).
pub const PROJECT_SEARCH_CANCEL: &str = "projects:search:cancel";

/// `project:<id>` selection prefix (TS: the `"project:"` literal in
/// `handleProjectSelect`).
pub const PROJECT_SELECT_PREFIX: &str = "project:";

/// `projects:page:` callback prefix, repeated for dispatch tests
/// (TS: `PROJECT_PAGE_CALLBACK_PREFIX`; canonical port lives in
/// `command_session_gates`).
pub const PROJECT_PAGE_CALLBACK_PREFIX: &str = "projects:page:";

/// Flattened interaction snapshot the search-state reader inspects
/// (TS: `InteractionState | null` with `kind` + a `project-search`
/// `metadata` bag).
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSearchSnapshot {
    pub kind: String,
    pub flow: Option<String>,
    pub stage: Option<String>,
    pub token: Option<u64>,
    pub query: Option<String>,
    pub message_id: Option<i64>,
    pub ids: Option<Vec<String>>,
}

/// Live `project-search` flow state (TS: the `searchState(deps)`
/// metadata with `flow === "project-search"`).
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSearchFlow {
    pub stage: String,
    pub token: u64,
}

/// Search-callback dispatch guard: no data never handles; the exact
/// `projects:search` opener and anything under `projects:search:` does
/// (TS: `!data || (data !== PROJECT_SEARCH_CALLBACK &&
/// !data.startsWith("projects:search:"))` returns `false`).
pub fn handles_project_search_callback(data: Option<&str>) -> bool {
    match data {
        None => false,
        Some(d) if d.is_empty() => false,
        Some(d) => d == PROJECT_SEARCH_CALLBACK || d.starts_with(PROJECT_SEARCH_DISPATCH_PREFIX),
    }
}

/// Exact `projects:search` opener match (TS: `data === PROJECT_SEARCH_CALLBACK`).
pub fn is_project_search_start(data: &str) -> bool {
    data == PROJECT_SEARCH_CALLBACK
}

/// Exact `projects:search:cancel` match (TS: `data === PROJECT_SEARCH_CANCEL`).
pub fn is_project_search_cancel(data: &str) -> bool {
    data == PROJECT_SEARCH_CANCEL
}

/// Exact `projects:search:again` match (TS: `data === PROJECT_SEARCH_AGAIN`).
pub fn is_project_search_again(data: &str) -> bool {
    data == PROJECT_SEARCH_AGAIN
}

/// `projects:search:page:` prefix match (TS:
/// `data.startsWith(PROJECT_SEARCH_PAGE_PREFIX)`).
pub fn is_project_search_page(data: &str) -> bool {
    data.starts_with(PROJECT_SEARCH_PAGE_PREFIX)
}

/// `projects:search:result:` prefix match (TS:
/// `data.startsWith(PROJECT_SEARCH_RESULT_PREFIX)`).
pub fn is_project_search_result(data: &str) -> bool {
    data.starts_with(PROJECT_SEARCH_RESULT_PREFIX)
}

/// Digits-only page raw behind `projects:search:page:` (TS:
/// `/^\d+$/.test(raw)` then `Number(raw)`; empty/signed/alpha reject).
pub fn parse_project_search_page_raw(raw: &str) -> Option<u64> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok()
}

/// Page requests need a digits-only raw and a string query; otherwise the
/// callback is consumed as a no-op (TS: `!/^\d+$/.test(raw) ||
/// typeof state.query !== "string"` returns `true` without acting).
pub fn is_project_search_page_request_valid(raw: &str, query_is_string: bool) -> bool {
    query_is_string && parse_project_search_page_raw(raw).is_some()
}

/// Result selection behind `projects:search:result:`: digits-only raw
/// indexing into the live `ids` array (TS: `/^\d+$/.test(raw) &&
/// Array.isArray(state.ids) ? state.ids[Number(raw)] : undefined`, then
/// the `typeof id !== "string"` failure gate).
pub fn resolve_project_search_result_id(ids: Option<&[String]>, raw: &str) -> Option<String> {
    let list = ids?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let index: usize = raw.parse().ok()?;
    list.get(index).cloned()
}

/// Live `project-search` flow state: `custom` kind plus
/// `flow === "project-search"` (TS: `searchState(deps)`).
pub fn project_search_flow_state(
    state: Option<&ProjectSearchSnapshot>,
) -> Option<ProjectSearchFlow> {
    let snapshot = state?;
    if snapshot.kind != "custom" {
        return None;
    }
    if snapshot.flow.as_deref() != Some("project-search") {
        return None;
    }
    Some(ProjectSearchFlow {
        stage: snapshot.stage.clone()?,
        token: snapshot.token?,
    })
}

/// Search page/result callbacks require the `results` stage (TS:
/// `state.stage !== "results"` answers `inactive_callback`).
pub fn is_project_search_results_stage(stage: Option<&str>) -> bool {
    stage == Some("results")
}

/// Search text input requires the `input` stage (TS:
/// `state.stage !== "input"` returns `false`).
pub fn is_project_search_input_stage(stage: Option<&str>) -> bool {
    stage == Some("input")
}

/// Stale-search guard: the live interaction must carry the same token
/// and stage as the stored search state (TS: `isSameSearch`: `!!active
/// && active.stage === state.stage && active.token === state.token`).
pub fn is_same_project_search(
    active_token: Option<u64>,
    active_stage: Option<&str>,
    state_token: Option<u64>,
    state_stage: Option<&str>,
) -> bool {
    match (active_token, active_stage, state_token, state_stage) {
        (Some(active), Some(active_s), Some(state), Some(state_s)) => {
            active == state && active_s == state_s
        }
        _ => false,
    }
}

/// Stale-callback guard: no stored message id, no callback message id,
/// or id mismatch answers `inactive_callback` (TS:
/// `state.messageId !== ctx.callbackQuery?.message?.message_id`).
pub fn is_project_search_message_mismatch(
    state_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (state_message_id, callback_message_id) {
        (Some(state), Some(callback)) => state != callback,
        _ => true,
    }
}

/// Combined `inactive_callback` gate for search page/result callbacks
/// (TS: `!state || state.stage !== "results" || state.messageId !==
/// callbackMessageId`).
pub fn should_answer_project_search_inactive(
    has_state: bool,
    is_results_stage: bool,
    messages_match: bool,
) -> bool {
    !has_state || !is_results_stage || !messages_match
}

/// Blank typed queries are consumed without searching (TS:
/// `if (!query.trim()) return true` in `handleProjectSearchTextInput`).
pub fn is_blank_project_search_query(query: &str) -> bool {
    query.trim().is_empty()
}

/// Search text input is handled only with live state, the `input`
/// stage, and message text (TS: `!state || state.stage !== "input" ||
/// ctx.message?.text === undefined` returns `false`).
pub fn handles_project_search_text_input(
    has_state: bool,
    is_input_stage: bool,
    has_text: bool,
) -> bool {
    has_state && is_input_stage && has_text
}

/// Project-select dispatch: a parsed `projects:page:<n>` page or a
/// `project:`-prefixed callback handles; anything else falls through
/// (TS: `page === null && !isProjectSelection` returns `false`; the page
/// parse itself lives in `command_session_gates`).
pub fn handles_project_select(page: Option<u64>, data: Option<&str>) -> bool {
    if page.is_some() {
        return true;
    }
    matches!(data, Some(d) if d.starts_with(PROJECT_SELECT_PREFIX))
}

/// `project:<id>` id extraction (TS:
/// `callbackQuery.data.replace("project:", "")`; the guard guarantees
/// the prefix, so this strips it; an empty rest reads as `None` since no
/// project can carry it).
pub fn parse_project_select_id(data: &str) -> Option<String> {
    let rest = data.strip_prefix(PROJECT_SELECT_PREFIX)?;
    if rest.is_empty() {
        return None;
    }
    Some(rest.to_string())
}

/// A busy foreground session blocks the callback with the busy reply
/// (TS: `isForegroundBusy(deps)` before state parsing).
pub fn should_block_project_callback_when_busy(is_foreground_busy: bool) -> bool {
    is_foreground_busy
}
