//! Thin Telegram wiring over `teloxide-core` (T6 scaffold).
//!
//! Node parity (`src/bot/index.ts`, `src/bot/middleware/auth.ts`,
//! `src/config.ts`):
//! - The bot is constructed from a token with `teloxide_core::Bot::new`
//!   (grammy `new Bot(token)` parity). Construction performs zero requests.
//! - Auth mirrors `authMiddleware`: only `TELEGRAM_ALLOWED_USER_ID` passes;
//!   the bot's own updates are ignored; strangers are denied (the
//!   hide-commands side effect stays Node-side until a dispatcher exists).
//!   The verdict reuses
//!   `otb_core::router_middleware_gates::decide_auth`.
//! - Outbound text honours the Telegram 4096-char limit
//!   (`TELEGRAM_TEXT_MESSAGE_LIMIT` in `src/bot/render/limits.ts`).
//!
//! SAFETY (explicit):
//! 1. The token is resolved `TELEGRAM_BOT_TOKEN` -> `OTB_TEST_TOKEN` ->
//!    dummy. The value itself is never logged (only `token_len`); the dummy
//!    is not a real credential and must never reach the network (polling is
//!    off by default).
//! 2. Constructing the bot or building payloads performs no network I/O.
//!    `send_message` is a plain async helper; only the opt-in poll loop
//!    (`run_telegram_from_env`, never called by tests) invokes it.

use teloxide_core::payloads::{
    AnswerCallbackQuerySetters, EditMessageTextSetters, GetUpdatesSetters, SendMessageSetters,
};
use teloxide_core::requests::Requester;
use teloxide_core::types::{
    CallbackQuery, ChatId, InlineKeyboardButton, InlineKeyboardMarkup, LinkPreviewOptions,
    MaybeInaccessibleMessage, MediaKind, Message, MessageId, MessageKind, Update, UpdateKind,
};

/// Env flag that opts into (future) polling. Anything but `"1"` means off.
pub const POLLING_ENV: &str = "OTB_ENABLE_POLLING";

/// Live bot token (Node `TELEGRAM_BOT_TOKEN`). Preferred over the test key.
pub const LIVE_TOKEN_ENV: &str = "TELEGRAM_BOT_TOKEN";

/// Test-only token override. Used when the live token is absent.
pub const TEST_TOKEN_ENV: &str = "OTB_TEST_TOKEN";

/// Allowed-user override for tests; falls back to `TELEGRAM_ALLOWED_USER_ID`.
pub const TEST_ALLOWED_USER_ENV: &str = "OTB_TEST_ALLOWED_USER_ID";

/// Dummy token used whenever no test token is provided. It is not a real
/// credential and must never reach the network (polling is off by default).
pub const DUMMY_TOKEN: &str = "0000000000:TEST-DUMMY-TOKEN-DO-NOT-USE";

/// Telegram text-message limit (Node: `TELEGRAM_TEXT_MESSAGE_LIMIT = 4096`).
pub const TELEGRAM_TEXT_LIMIT: usize = 4096;

/// Pure gate: polling is enabled only for the literal value `"1"`.
pub fn polling_enabled_from(raw: Option<&str>) -> bool {
    raw == Some("1")
}

/// Read the polling flag from the environment. Default: disabled.
pub fn polling_enabled() -> bool {
    polling_enabled_from(std::env::var(POLLING_ENV).ok().as_deref())
}

/// Pure token resolution: non-blank explicit value wins, else the dummy.
pub fn resolve_token(explicit: Option<&str>) -> String {
    match explicit {
        Some(raw) if !raw.trim().is_empty() => raw.trim().to_string(),
        _ => DUMMY_TOKEN.to_string(),
    }
}

/// Pure ordered resolution: live token wins, then test token, else dummy.
pub fn resolve_token_ordered(live: Option<&str>, test: Option<&str>) -> String {
    match live {
        Some(raw) if !raw.trim().is_empty() => return raw.trim().to_string(),
        _ => {}
    }
    resolve_token(test)
}

/// Token from the environment (`TELEGRAM_BOT_TOKEN`, else `OTB_TEST_TOKEN`,
/// else dummy). Reads env only; the value itself is never logged.
pub fn token_from_env() -> String {
    resolve_token_ordered(
        std::env::var(LIVE_TOKEN_ENV).ok().as_deref(),
        std::env::var(TEST_TOKEN_ENV).ok().as_deref(),
    )
}

/// Pure allowed-user parse: digits only, else `0` (deny-all sentinel).
pub fn parse_allowed_user_id(raw: Option<&str>) -> u64 {
    raw.and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// Allowed user from the environment: `OTB_TEST_ALLOWED_USER_ID`, else the
/// Node `TELEGRAM_ALLOWED_USER_ID` key (the ID is not a secret). Reads env
/// only; unparseable/missing becomes `0`.
pub fn allowed_user_id_from_env() -> u64 {
    let test = std::env::var(TEST_ALLOWED_USER_ENV).ok();
    if let Some(raw) = test.filter(|value| !value.trim().is_empty()) {
        return parse_allowed_user_id(Some(&raw));
    }
    parse_allowed_user_id(std::env::var("TELEGRAM_ALLOWED_USER_ID").ok().as_deref())
}

/// Pure auth check (Node `authMiddleware` head): the update passes only when
/// it carries the allowed user id.
pub fn is_authorized(user_id: Option<u64>, allowed_user_id: u64) -> bool {
    otb_core::router_middleware_gates::decide_auth(user_id, allowed_user_id, 0, None)
        == otb_core::router_middleware_gates::AuthVerdict::Allow
}

/// Truncate outbound text to the Telegram limit on a char boundary
/// (pure; testable). Short text is returned unchanged.
pub fn truncate_text(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Build the `(chat_id, text)` payload for an outbound message (pure).
/// Text is truncated to [`TELEGRAM_TEXT_LIMIT`].
pub fn build_send_payload(chat_id: i64, text: &str) -> (ChatId, String) {
    (
        ChatId(chat_id),
        truncate_text(text, TELEGRAM_TEXT_LIMIT).to_string(),
    )
}

/// Construct the thin API client. Performs zero requests; safe with the
/// dummy token as long as no request is sent (polling stays off by default).
pub fn create_bot(token: &str) -> teloxide_core::Bot {
    teloxide_core::Bot::new(token)
}

/// Send a text message (thin `sendMessage` parity). Plain async helper with
/// no auto-call: the scaffold never invokes it.
pub async fn send_message(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    text: &str,
) -> Result<(), teloxide_core::RequestError> {
    let (chat, body) = build_send_payload(chat_id, text);
    bot.send_message(chat, body).await?;
    Ok(())
}

/// Env key for the configured model provider (Node `OPENCODE_MODEL_PROVIDER`).
pub const MODEL_PROVIDER_ENV: &str = "OPENCODE_MODEL_PROVIDER";

/// Env key for the configured model id (Node `OPENCODE_MODEL_ID`).
pub const MODEL_ID_ENV: &str = "OPENCODE_MODEL_ID";

/// Placeholder model display until the live model fetch lands (T15
/// follow-up: `fetchCurrentModel` parity with variant + availability).
/// Reads the Node-required `OPENCODE_MODEL_PROVIDER`/`OPENCODE_MODEL_ID`
/// env keys; falls back to an explicit `not set` marker. Pure; no I/O.
/// Used only when `settings.json` holds no `currentModel` (see
/// [`build_status_snapshot_from`]).
pub fn default_model_display() -> String {
    let provider = std::env::var(MODEL_PROVIDER_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let model = std::env::var(MODEL_ID_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    match (provider, model) {
        (Some(provider), Some(model)) => {
            format_status_model_display(provider.trim(), model.trim(), None)
        }
        _ => "🧠 not set".to_string(),
    }
}

/// Pure `🧠 provider/id` display with an optional ` (variant)` suffix
/// (Node `status-command.ts` parity:
/// `` `🧠 ${providerID}/${modelID}` + ` (${variant})` `` when set).
pub fn format_status_model_display(provider: &str, model: &str, variant: Option<&str>) -> String {
    let base = format!("🧠 {}/{}", provider.trim(), model.trim());
    match variant.map(str::trim).filter(|value| !value.is_empty()) {
        Some(variant) => format!("{base} ({variant})"),
        None => base,
    }
}

/// Pure snapshot from an explicit settings view (no env/file I/O beyond the
/// [`default_model_display`] env fallback; testable).
///
/// Precedence: `settings.json` `current*` → `OPENCODE_MODEL_PROVIDER`/`ID`
/// env → placeholders.
///
/// Field mapping (Node `status-command.ts` parity):
/// - `agent_display`: settings `currentAgent` through
///   `get_agent_display_name` (`getAgentDisplayName` parity); `None` until
///   the live `fetchCurrentAgent` session-message fetch lands.
/// - `model_display`: settings `currentModel` (`provider/id + variant`)
///   first, else the env fallback above.
/// - `project_display`: settings `currentProject.worktree` verbatim; the
///   `mainProjectPath: branch` enrichment (`getGitWorktreeContext`) is a
///   follow-up, so `worktree_path` (linked-worktree only) stays `None`.
/// - `session_title`: the remembered settings title; the live
///   `fetchSessionTitle` refresh is a follow-up.
pub fn build_status_snapshot_from(
    health: &crate::opencode::HealthProbe,
    current: &crate::settings_current::SettingsCurrent,
) -> crate::commands::StatusSnapshot {
    let model_display = match (
        current.model_provider.as_deref(),
        current.model_id.as_deref(),
    ) {
        (Some(provider), Some(model)) => {
            format_status_model_display(provider, model, current.model_variant.as_deref())
        }
        _ => default_model_display(),
    };
    crate::commands::StatusSnapshot {
        bot_version: env!("CARGO_PKG_VERSION").to_string(),
        healthy: health.ok,
        opencode_version: health.version.clone(),
        agent_display: current
            .agent
            .as_deref()
            .map(otb_core::types_config::get_agent_display_name),
        model_display,
        project_display: current.project_worktree.clone(),
        worktree_path: None,
        session_title: current.session_title.clone(),
    }
}

/// Build the live `/status` snapshot from a health probe (read-only).
///
/// Resolves the App Home (`OPENCODE_TELEGRAM_HOME` override else the
/// installed/current-dir default) and reads `<app_home>/settings.json`
/// tolerantly: a missing or invalid file keeps the env/placeholder
/// fallbacks. [`crate::commands::render_status`] stays pure and unchanged.
pub fn build_status_snapshot(
    health: &crate::opencode::HealthProbe,
) -> crate::commands::StatusSnapshot {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    build_status_snapshot_from(health, &current)
}

/// Pure session identity from a create/list entry (testable).
/// Accepts a bare session (`{"id":...,"title":...}`) or the live V2
/// envelope (`{"data":{"id":...,"title":...}}`); anything else yields
/// empty ids/titles. Only shapes are handled here — no I/O, no logging.
pub fn session_identity_from_value(value: &serde_json::Value) -> (String, String) {
    let inner = value.get("data").unwrap_or(value);
    let id = inner
        .get("id")
        .and_then(|id| id.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .unwrap_or("");
    let title = inner
        .get("title")
        .and_then(|title| title.as_str())
        .unwrap_or("");
    (id.to_string(), title.to_string())
}

/// Node `SESSION_CALLBACK_PREFIX` parity (`session-selection-menu.ts`):
/// each session row carries `callback_data = "session:<id>"`.
pub const SESSION_CALLBACK_PREFIX: &str = "session:";

/// Node `SESSION_PAGE_CALLBACK_PREFIX` parity: `"session:page:<n>"`.
/// The scaffold sends no page buttons yet; page taps are only acked.
pub const SESSION_PAGE_CALLBACK_PREFIX: &str = "session:page:";

/// Parse a `session:<id>` callback into the session id (pure; testable).
/// Node `parseSessionIdCallback` parity: page callbacks are excluded and an
/// empty id yields `None`.
pub fn parse_session_id_callback(data: &str) -> Option<&str> {
    let id = data.strip_prefix(SESSION_CALLBACK_PREFIX)?;
    if data.starts_with(SESSION_PAGE_CALLBACK_PREFIX) || id.is_empty() {
        return None;
    }
    Some(id)
}

/// Parse a `session:page:<n>` callback into the page number (pure; testable).
/// Node `parseSessionPageCallback` parity: anything but a non-negative
/// integer yields `None`.
pub fn parse_session_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(SESSION_PAGE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Node exact search taps (`session-selection-menu.ts`):
/// `"session:search"` opens the prompt, `"session:search:again"` re-opens
/// it, `"session:search:cancel"` dismisses the results message.
pub const SESSION_SEARCH_CALLBACK: &str = "session:search";
pub const SESSION_SEARCH_AGAIN_CALLBACK: &str = "session:search:again";
pub const SESSION_SEARCH_CANCEL_CALLBACK: &str = "session:search:cancel";

/// Node `session:search:page:<n>` pager-button prefix (search-results
/// paging; the query itself travels in interaction memory Node-side).
pub const SESSION_SEARCH_PAGE_PREFIX: &str = "session:search:page:";

/// Node `session:search:result:<index>` result-button prefix (the index is
/// global within the filtered results and resolves via remembered ids).
pub const SESSION_SEARCH_RESULT_PREFIX: &str = "session:search:result:";

/// Parse a `session:search:page:<n>` callback into the page number (pure;
/// digits-only, empty yields `None`).
pub fn parse_session_search_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(SESSION_SEARCH_PAGE_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `session:search:result:<n>` callback into the global result
/// index (pure; digits-only, empty yields `None`).
pub fn parse_session_search_result_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(SESSION_SEARCH_RESULT_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Node `project:<id>` button parity (`project-selection-menu.ts`): each
/// project row carries `callback_data = "project:<id>"`.
pub const PROJECT_CALLBACK_PREFIX: &str = "project:";

/// Node `projects:page:<n>` paging-button parity.
pub const PROJECT_PAGE_CALLBACK_PREFIX: &str = "projects:page:";

/// Node `worktree:<gitIndex>` button parity
/// (`worktree-selection-menu.ts`). The scaffold carries the position in the
/// *presented* (missing-folder-filtered) list instead of the raw git index,
/// because the presented list is what the buttons show.
pub const WORKTREE_CALLBACK_PREFIX: &str = "worktree:";

/// Node `worktree:page:<n>` paging-button parity.
pub const WORKTREE_PAGE_CALLBACK_PREFIX: &str = "worktree:page:";

/// Node `recent:<index>` button parity (`recent-selection-menu.ts`): the
/// position in the presented recent list.
pub const RECENT_CALLBACK_PREFIX: &str = "recent:";

/// `recent:page:<n>` paging-button parity (scaffold extension of the Node
/// `recent-selection-menu.ts` shape, which carries no paging buttons; the
/// prefix follows the `session:page:` / `projects:page:` pattern so page
/// taps round-trip through [`parse_recent_page_callback`]).
pub const RECENT_PAGE_CALLBACK_PREFIX: &str = "recent:page:";

/// Node `messages:select:<index>` button parity
/// (`message-history-menu.ts`): the position in the presented user-message
/// list (newest-first).
pub const MESSAGES_SELECT_CALLBACK_PREFIX: &str = "messages:select:";

/// Node `messages:page:<n>` paging-button parity.
pub const MESSAGES_PAGE_CALLBACK_PREFIX: &str = "messages:page:";

/// Node `MESSAGES_CALLBACK_PREFIX` parity (`message-history-menu.ts`):
/// every detail/list callback starts with `"messages:"`.
pub const MESSAGES_CALLBACK_PREFIX: &str = "messages:";

/// Node `MESSAGES_CALLBACK_REVERT` parity: exact revert tap (no payload;
/// the target resolves from the detail message text, see
/// [`resolve_message_detail_index`]).
pub const MESSAGES_REVERT_CALLBACK: &str = "messages:revert";

/// Node `MESSAGES_CALLBACK_FORK` parity: exact fork tap (no payload).
pub const MESSAGES_FORK_CALLBACK: &str = "messages:fork";

/// Node `MESSAGES_CALLBACK_BACK` parity: exact back-to-list tap.
pub const MESSAGES_BACK_CALLBACK: &str = "messages:back";

/// Node `MESSAGES_CALLBACK_CANCEL` parity: exact cancel tap.
pub const MESSAGES_CANCEL_CALLBACK: &str = "messages:cancel";

/// Exact `messages:revert` match (Node `data === MESSAGES_CALLBACK_REVERT`).
pub fn is_messages_revert(data: &str) -> bool {
    data == MESSAGES_REVERT_CALLBACK
}

/// Exact `messages:fork` match (Node `data === MESSAGES_CALLBACK_FORK`).
pub fn is_messages_fork(data: &str) -> bool {
    data == MESSAGES_FORK_CALLBACK
}

/// Exact `messages:back` match (Node `data === MESSAGES_CALLBACK_BACK`).
pub fn is_messages_back(data: &str) -> bool {
    data == MESSAGES_BACK_CALLBACK
}

/// Exact `messages:cancel` match (Node `data === MESSAGES_CALLBACK_CANCEL`).
pub fn is_messages_cancel(data: &str) -> bool {
    data == MESSAGES_CANCEL_CALLBACK
}

/// Parse a `project:<id>` callback into the project id (pure; testable).
/// Page callbacks are excluded and an empty id yields `None`.
pub fn parse_project_id_callback(data: &str) -> Option<&str> {
    let id = data.strip_prefix(PROJECT_CALLBACK_PREFIX)?;
    if data.starts_with(PROJECT_PAGE_CALLBACK_PREFIX) || id.is_empty() {
        return None;
    }
    Some(id)
}

/// Parse a `projects:page:<n>` callback into the page number (pure).
pub fn parse_project_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(PROJECT_PAGE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Node exact search taps (`project-selection-menu.ts`):
/// `"projects:search"` opens the prompt, `"projects:search:again"`
/// re-opens it, `"projects:search:cancel"` dismisses the results message.
pub const PROJECT_SEARCH_CALLBACK: &str = "projects:search";
pub const PROJECT_SEARCH_AGAIN_CALLBACK: &str = "projects:search:again";
pub const PROJECT_SEARCH_CANCEL_CALLBACK: &str = "projects:search:cancel";

/// Node `projects:search:page:<n>` pager-button prefix (search-results
/// paging; the query itself travels in interaction memory Node-side).
pub const PROJECT_SEARCH_PAGE_PREFIX: &str = "projects:search:page:";

/// Node `projects:search:result:<index>` result-button prefix (the index
/// is global within the filtered results and resolves via remembered ids).
pub const PROJECT_SEARCH_RESULT_PREFIX: &str = "projects:search:result:";

/// Parse a `projects:search:page:<n>` callback into the page number
/// (pure; digits-only, empty yields `None`).
pub fn parse_project_search_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(PROJECT_SEARCH_PAGE_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `projects:search:result:<n>` callback into the global result
/// index (pure; digits-only, empty yields `None`).
pub fn parse_project_search_result_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(PROJECT_SEARCH_RESULT_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `worktree:<n>` callback into the presented-list position (pure).
/// Page callbacks are excluded and anything but a non-negative integer
/// yields `None`.
pub fn parse_worktree_index_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(WORKTREE_CALLBACK_PREFIX)?;
    if data.starts_with(WORKTREE_PAGE_CALLBACK_PREFIX)
        || raw.is_empty()
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `worktree:page:<n>` callback into the page number (pure).
pub fn parse_worktree_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(WORKTREE_PAGE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `recent:<n>` callback into the presented-list position (pure).
/// Page callbacks are excluded and anything but a non-negative integer
/// yields `None`.
pub fn parse_recent_index_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(RECENT_CALLBACK_PREFIX)?;
    if data.starts_with(RECENT_PAGE_CALLBACK_PREFIX)
        || raw.is_empty()
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `recent:page:<n>` callback into the page number (pure).
pub fn parse_recent_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(RECENT_PAGE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Clamp a requested page into `0..total_pages` (pure; testable).
/// A zero `total_pages` (empty list) reads as page `0`; anything past the
/// last page pins to the last page.
pub fn clamp_page_index(page: usize, total_pages: usize) -> usize {
    if total_pages == 0 {
        return 0;
    }
    page.min(total_pages - 1)
}

/// Cap for a typed search query (Node
/// `SESSION_SEARCH_QUERY_MAX_LENGTH = 100` parity in
/// `session-selection-menu.ts`: "a session title never approaches this;
/// the cap keeps a typed query sendable").
pub const SESSION_SEARCH_QUERY_MAX_LENGTH: usize = 100;

/// Cap a typed search query to [`SESSION_SEARCH_QUERY_MAX_LENGTH`] chars
/// (pure; testable). Char-boundary safe; short queries pass through.
pub fn truncate_search_query(query: &str) -> String {
    if query.chars().count() <= SESSION_SEARCH_QUERY_MAX_LENGTH {
        return query.to_string();
    }
    query
        .chars()
        .take(SESSION_SEARCH_QUERY_MAX_LENGTH)
        .collect()
}

/// Whether a session row matches a typed query (pure; testable). Node
/// `searchSessions` parity: case-insensitive title-or-id contains over the
/// trimmed query; a blank query matches every row.
pub fn matches_session_query(item: &crate::commands::SessionListItem, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    item.title.to_lowercase().contains(&needle) || item.id.to_lowercase().contains(&needle)
}

/// Whether a project row matches a typed query (pure; testable). Node
/// `filterProjectsForSearch` parity, minus the active-project display-name
/// clause (stateless: the scaffold filters on the worktree folder name
/// only): case-insensitive folder-name contains over the trimmed query; a
/// blank query matches every row.
pub fn matches_project_query(item: &crate::commands::ProjectListItem, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    crate::commands::get_project_folder_name(&item.worktree)
        .to_lowercase()
        .contains(&needle)
}

/// Whether a model row matches a typed query (pure; testable). Node
/// `searchModels` parity: case-insensitive `provider/model` contains over
/// the trimmed query; a blank query matches nothing (Node returns `[]`).
pub fn matches_model_query(item: &crate::commands::ModelListItem, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return false;
    }
    format!("{}/{}", item.provider_id, item.model_id)
        .to_lowercase()
        .contains(&needle)
}

/// Parse a `messages:select:<n>` callback into the presented-list position
/// (pure).
pub fn parse_message_select_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(MESSAGES_SELECT_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `messages:page:<n>` callback into the page number (pure).
pub fn parse_message_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(MESSAGES_PAGE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Truncate an inline-button label to the 64-char Telegram limit (pure).
/// An active row keeps its `✅ ` prefix like the Node
/// `formatProjectButtonLabel`/`formatWorktreeButtonLabel` helpers.
pub fn truncate_button_label(label: &str, active: bool) -> String {
    const LIMIT: usize = 64;
    let prefix = if active { "✅ " } else { "" };
    let available = LIMIT.saturating_sub(prefix.chars().count());
    if label.chars().count() <= available {
        return format!("{prefix}{label}");
    }
    let kept: String = label.chars().take(available.saturating_sub(3)).collect();
    format!("{prefix}{kept}...")
}

/// One button row per session: `"{n}. [marker ]title"` with
/// `callback_data = "session:<id>"` (pure; testable). Titles go through
/// `format_session_title` so blank titles never produce empty buttons.
/// When `total_pages > 1` a trailing pager row carries `⬅`/`➡` buttons
/// with `callback_data = "session:page:<n>"` (Node
/// `session-selection-menu.ts` parity); a single page keeps the rows only.
pub fn build_sessions_keyboard(
    items: &[crate::commands::SessionListItem],
    current_id: Option<&str>,
    locale: &str,
    page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let marker = crate::locale::translate(locale, "sessions.current_marker");
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let title = crate::commands::format_session_title(&item.title, Some(locale));
            let label = if current_id == Some(item.id.as_str()) && !marker.is_empty() {
                format!("{}. {marker} {title}", index + 1)
            } else {
                format!("{}. {title}", index + 1)
            };
            vec![InlineKeyboardButton::callback(
                label,
                format!("{SESSION_CALLBACK_PREFIX}{}", item.id),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "sessions.button.prev_page"),
                format!("{SESSION_PAGE_CALLBACK_PREFIX}{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "sessions.button.next_page"),
                format!("{SESSION_PAGE_CALLBACK_PREFIX}{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    InlineKeyboardMarkup::new(rows)
}

/// Loaded `/sessions` context shared by the list reply and the select
/// callback (read-only except the remembered-settings read).
struct SessionsContext {
    worktree: String,
    current_id: Option<String>,
    items: Vec<crate::commands::SessionListItem>,
}

/// Load the current project's first session page, or the i18n key for the
/// failure reply (`sessions.project_not_selected` on no project,
/// `sessions.fetch_error` when the list cannot be fetched).
async fn load_sessions_context() -> Result<SessionsContext, &'static str> {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return Err("sessions.project_not_selected");
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let sessions = match client.list_sessions(Some(&worktree)).await {
        Ok(sessions) => sessions,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: sessions list failed: {err}");
            return Err("sessions.fetch_error");
        }
    };
    let items: Vec<crate::commands::SessionListItem> = sessions
        .iter()
        .filter_map(crate::commands::session_list_item_from_value)
        // Project scope, like the Node `directory` query: keep rows for this
        // worktree (blank-directory rows are legacy; keep them visible).
        // Paging slices per page in `sessions_view`; the full list is kept
        // here so later pages resolve.
        .filter(|item| item.directory.is_empty() || item.directory == worktree)
        .collect();
    Ok(SessionsContext {
        worktree,
        current_id: current.session_id,
        items,
    })
}

/// Comparison key for a worktree (Node `worktreeKey` parity: lowercase on
/// Windows only).
fn worktree_key(worktree: &str) -> String {
    if cfg!(windows) {
        worktree.to_lowercase()
    } else {
        worktree.to_string()
    }
}

/// True when `worktree/.git` is a pointer file into another repo's
/// `worktrees/` dir (Node `isLinkedGitWorktree` parity in
/// `project-service.ts`). A `.git` directory (main checkout), a missing
/// path, or an unreadable file all read as not-linked; only paths/error
/// kinds are ever logged, never values.
fn is_linked_worktree(worktree: &str) -> bool {
    if worktree == "/" {
        return false;
    }
    let content = match std::fs::read_to_string(std::path::Path::new(worktree).join(".git")) {
        Ok(content) => content,
        Err(_) => return false,
    };
    let Some(pointer) = content.lines().next().map(str::trim) else {
        return false;
    };
    let Some((key, target)) = pointer.split_once(':') else {
        return false;
    };
    if !key.trim().eq_ignore_ascii_case("gitdir") || target.trim().is_empty() {
        return false;
    }
    let resolved = std::path::Path::new(worktree).join(target.trim());
    let normalized = resolved.to_string_lossy().replace('\\', "/").to_lowercase();
    normalized.contains("/.git/worktrees/")
}

/// One button row per project: `"N. [✅ ]folder [worktree]"` with
/// `callback_data = "project:<id>"` (pure; testable). Labels are capped at
/// 64 chars like the Node `formatProjectButtonLabel` helper. When
/// `total_pages > 1` a trailing pager row carries `callback_data =
/// "projects:page:<n>"` (Node `project-selection-menu.ts` parity).
pub fn build_projects_keyboard(
    items: &[crate::commands::ProjectListItem],
    current_worktree: Option<&str>,
    locale: &str,
    page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .map(|item| {
            let label = format!(
                "{} [{}]",
                crate::commands::get_project_folder_name(&item.worktree),
                item.worktree
            );
            let active = current_worktree == Some(item.worktree.as_str());
            vec![InlineKeyboardButton::callback(
                truncate_button_label(&label, active),
                format!("{PROJECT_CALLBACK_PREFIX}{}", item.id),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "projects.prev_page"),
                format!("{PROJECT_PAGE_CALLBACK_PREFIX}{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "projects.next_page"),
                format!("{PROJECT_PAGE_CALLBACK_PREFIX}{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    InlineKeyboardMarkup::new(rows)
}

/// One button row per worktree: `"N. [✅ ]folder [path]"` with
/// `callback_data = "worktree:<position>"` (pure; testable). When
/// `total_pages > 1` a trailing pager row carries `callback_data =
/// "worktree:page:<n>"` (Node `worktree-selection-menu.ts` parity, whose
/// pager labels are the shared `projects.prev_page` / `projects.next_page`
/// keys).
pub fn build_worktree_keyboard(
    items: &[crate::commands::WorktreeListItem],
    locale: &str,
    page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let label = format!(
                "{} [{}]",
                crate::commands::get_project_folder_name(&item.path),
                item.path
            );
            vec![InlineKeyboardButton::callback(
                truncate_button_label(&label, item.is_current),
                format!("{WORKTREE_CALLBACK_PREFIX}{index}"),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "projects.prev_page"),
                format!("{WORKTREE_PAGE_CALLBACK_PREFIX}{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "projects.next_page"),
                format!("{WORKTREE_PAGE_CALLBACK_PREFIX}{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    InlineKeyboardMarkup::new(rows)
}

/// One button row per recent session: `"{glyph} [project] title"` with
/// `callback_data = "recent:<position>"` (pure; testable). Titles go
/// through `format_session_title` so blank titles never produce empty
/// buttons; over-long labels are capped at 64 chars like Node. When
/// `total_pages > 1` a trailing pager row carries `callback_data =
/// "recent:page:<n>"` (scaffold paging; the Node recent menu keeps a
/// single page).
pub fn build_recent_keyboard(
    items: &[crate::commands::RecentListItem],
    locale: &str,
    page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let title = crate::commands::format_session_title(&item.title, Some(locale));
            let label = format!(
                "{} [{}] {title}",
                crate::commands::recent_status_glyph(item.status),
                crate::commands::get_project_folder_name(&item.directory),
            );
            vec![InlineKeyboardButton::callback(
                truncate_button_label(&label, false),
                format!("{RECENT_CALLBACK_PREFIX}{index}"),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "projects.prev_page"),
                format!("{RECENT_PAGE_CALLBACK_PREFIX}{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "projects.next_page"),
                format!("{RECENT_PAGE_CALLBACK_PREFIX}{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    InlineKeyboardMarkup::new(rows)
}

/// One button row per user message: `"[HH:MM] text"` with `callback_data =
/// "messages:select:<position>"` (pure; testable). When `total_pages > 1`
/// a trailing pager row carries `callback_data = "messages:page:<n>"`
/// (Node `message-history-menu.ts` parity).
pub fn build_messages_keyboard(
    items: &[crate::commands::UserMessageItem],
    locale: &str,
    page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            vec![InlineKeyboardButton::callback(
                crate::commands::format_message_line(item),
                format!("{MESSAGES_SELECT_CALLBACK_PREFIX}{index}"),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "messages.button.prev_page"),
                format!("{MESSAGES_PAGE_CALLBACK_PREFIX}{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "messages.button.next_page"),
                format!("{MESSAGES_PAGE_CALLBACK_PREFIX}{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    InlineKeyboardMarkup::new(rows)
}

/// Detail keyboard for one user message (Node `buildMessageDetailKeyboard`
/// parity in `message-history-menu.ts`): `[revert]`, `[fork]`, then
/// `[back, cancel]` on one row. Labels come from the
/// `messages.button.*` i18n keys; callbacks are the exact
/// `messages:revert` / `messages:fork` / `messages:back` /
/// `messages:cancel` shapes (pure; testable).
pub fn build_message_detail_keyboard(locale: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "messages.button.revert"),
            MESSAGES_REVERT_CALLBACK.to_string(),
        )],
        vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "messages.button.fork"),
            MESSAGES_FORK_CALLBACK.to_string(),
        )],
        vec![
            InlineKeyboardButton::callback(
                crate::locale::translate(locale, "messages.button.back"),
                MESSAGES_BACK_CALLBACK.to_string(),
            ),
            InlineKeyboardButton::callback(
                crate::locale::translate(locale, "messages.button.cancel"),
                MESSAGES_CANCEL_CALLBACK.to_string(),
            ),
        ],
    ])
}

/// Resolve which list item a detail tap refers to by re-rendering each
/// candidate and comparing against the detail message text (pure;
/// testable).
///
/// SCAFFOLD DEVIATION: Node remembers the `selectedIndex` in its
/// interaction state (`MessagesDetailMetadata`), so its revert/fork taps
/// carry no payload. The scaffold keeps no per-chat interaction memory,
/// so the target is recovered from the detail message itself
/// (`render_message_detail` is deterministic). A follow-up with a
/// dispatcher should carry the remembered index instead. No match
/// (edited/foreign text) yields `None` and the caller falls back to the
/// `messages.inactive_callback` notice plus the list view.
pub fn resolve_message_detail_index(
    detail_text: &str,
    items: &[crate::commands::UserMessageItem],
) -> Option<usize> {
    items
        .iter()
        .position(|item| crate::commands::render_message_detail(item) == detail_text)
}

/// Loaded `/projects` context shared by the list reply and the select
/// callback (read-only except the remembered-settings read).
struct ProjectsContext {
    items: Vec<crate::commands::ProjectListItem>,
    current_worktree: Option<String>,
    current_display: Option<String>,
}

/// Load the project list: API projects merged with the isolated session
/// directory cache (JS `getResolvedProjects` parity, minus linked-worktree
/// hiding details the scaffold keeps: linked checkouts *are* hidden, but
/// `excludedProjectPaths` filtering is a dispatcher follow-up).
/// Missing folders are dropped like `getListedProjects`. Failure key:
/// `projects.fetch_error`.
async fn load_projects_context() -> Result<ProjectsContext, &'static str> {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let mut merged: std::collections::HashMap<String, crate::commands::ProjectListItem> =
        std::collections::HashMap::new();
    match client.list_projects().await {
        Ok(projects) => {
            for item in projects
                .iter()
                .filter_map(crate::commands::project_list_item_from_value)
            {
                merged.insert(worktree_key(&item.worktree), item);
            }
        }
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: projects list failed: {err}");
            return Err("projects.fetch_error");
        }
    }
    // Cached virtual projects fill gaps the API list does not cover (Node
    // `getCachedSessionProjects` merge parity); the cache DB is the
    // isolated one, never a live path.
    let db_path = crate::store::resolve_db_path(&app_home);
    if let Ok(store) = crate::store::SessionCacheStore::open(&db_path) {
        if let Ok(cached) = store.get_projects() {
            for project in cached {
                let key = worktree_key(&project.worktree);
                if let Some(existing) = merged.get_mut(&key) {
                    if project.last_updated > existing.last_updated {
                        existing.last_updated = project.last_updated;
                    }
                } else {
                    merged.insert(
                        key,
                        crate::commands::ProjectListItem {
                            id: project.id,
                            worktree: project.worktree,
                            name: project.name,
                            last_updated: project.last_updated,
                        },
                    );
                }
            }
        }
    }
    let mut items: Vec<crate::commands::ProjectListItem> = merged
        .into_values()
        // `getListedProjects` parity: folders the server cannot confirm are
        // left out; linked checkouts stay hidden like Node.
        .filter(|item| {
            !item.worktree.trim().is_empty()
                && item.worktree != "/"
                && std::path::Path::new(&item.worktree).exists()
                && !is_linked_worktree(&item.worktree)
        })
        .collect();
    items.sort_by(|left, right| right.last_updated.cmp(&left.last_updated));
    // Paging slices per page in `projects_view`; the full sorted list is
    // kept here so later pages resolve.
    let current_display = current
        .project_name
        .clone()
        .filter(|name| !name.is_empty())
        .or_else(|| current.project_worktree.clone());
    Ok(ProjectsContext {
        items,
        current_worktree: current.project_worktree,
        current_display,
    })
}

/// Live `/projects` view: the JS-parity text plus an inline keyboard with
/// one `project:<id>` button per row (read-only except the
/// remembered-settings read). Fetch failure → `projects.fetch_error`; no
/// rows → `projects.empty`; otherwise the numbered `render_projects` list
/// for the requested page plus `projects:page:<n>` pager buttons when more
/// than one page exists (Node `project-selection-menu.ts` parity).
async fn projects_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    let context = match load_projects_context().await {
        Ok(context) => context,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let total = context.items.len();
    let limit = crate::config::PROJECTS_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &context.items[start.min(total)..end];
    let text = crate::commands::render_projects(
        page_items,
        context.current_worktree.as_deref(),
        context.current_display.as_deref(),
        page,
        total_pages,
        Some(locale),
    );
    // Leading search row (Node `buildProjectsKeyboard` parity): opens the
    // stateless prompt-then-filter flow. `build_projects_keyboard` stays
    // shared with the results view, which carries again/cancel instead.
    let keyboard = if page_items.is_empty() {
        None
    } else {
        let mut keyboard = build_projects_keyboard(
            page_items,
            context.current_worktree.as_deref(),
            locale,
            page,
            total_pages,
        );
        keyboard.inline_keyboard.insert(
            0,
            vec![InlineKeyboardButton::callback(
                crate::locale::translate(locale, "projects.search.button"),
                PROJECT_SEARCH_CALLBACK.to_string(),
            )],
        );
        Some(keyboard)
    };
    (text, keyboard)
}

/// Filtered `/projects <query>` view (stateless Batch E1 search follow-up).
///
/// Same stateless assumption as [`sessions_search_view`]: no remembered
/// `project-search` interaction state, so the query travels as the
/// `/projects <query>` command argument and matching filters the live
/// merged list in memory on every call (Node `filterProjectsForSearch`
/// parity via [`matches_project_query`], folder-name clause only).
/// Result rows carry direct `project:<id>` taps (no remembered `ids`
/// vector, so `projects:search:result:<index>` cannot resolve); the list
/// is capped to the first `PROJECTS_LIST_LIMIT` matches with a
/// `[search again, cancel]` row. Stale `projects:search:page:<n>` /
/// `projects:search:result:<n>` taps re-prompt instead.
async fn projects_search_view(locale: &str, query: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let query = truncate_search_query(query);
    let context = match load_projects_context().await {
        Ok(context) => context,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let again_cancel = || {
        search_again_cancel_row(
            locale,
            "projects.search.again",
            "projects.search.cancel",
            PROJECT_SEARCH_AGAIN_CALLBACK,
            PROJECT_SEARCH_CANCEL_CALLBACK,
        )
    };
    let matched: Vec<crate::commands::ProjectListItem> = context
        .items
        .iter()
        .filter(|item| matches_project_query(item, &query))
        .cloned()
        .collect();
    if matched.is_empty() {
        let text = crate::locale::translate(locale, "projects.search.no_results")
            .replace("{query}", &query);
        return (text, Some(InlineKeyboardMarkup::new(vec![again_cancel()])));
    }
    let limit = crate::config::PROJECTS_LIST_LIMIT.max(1);
    let page_items: Vec<crate::commands::ProjectListItem> =
        matched.into_iter().take(limit).collect();
    let text =
        crate::locale::translate(locale, "projects.search.results").replace("{query}", &query);
    let mut keyboard = build_projects_keyboard(
        &page_items,
        context.current_worktree.as_deref(),
        locale,
        0,
        1,
    );
    keyboard.inline_keyboard.push(again_cancel());
    (text, Some(keyboard))
}

/// Parse `git worktree list --porcelain` output into `(path, branch)` rows
/// (pure; testable). A `branch refs/heads/<name>` line yields `Some(name)`;
/// `detached`/missing branch yields `None`. Blocks without a `worktree`
/// line are skipped.
pub fn parse_git_worktree_porcelain(output: &str) -> Vec<(String, Option<String>)> {
    let mut rows: Vec<(String, Option<String>)> = Vec::new();
    let mut path: Option<String> = None;
    let mut branch: Option<String> = None;
    for line in output.lines().chain(std::iter::once("")) {
        let line = line.trim_end();
        if line.is_empty() {
            if let Some(path) = path.take() {
                rows.push((path, branch.take()));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            if path.is_some() {
                rows.push((path.take().expect("path set"), branch.take()));
            }
            if !rest.trim().is_empty() {
                path = Some(rest.trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("branch ") {
            let name = rest.strip_prefix("refs/heads/").unwrap_or(rest);
            if !name.is_empty() {
                branch = Some(name.to_string());
            }
        }
    }
    rows
}

/// Run `git -C <worktree> worktree list --porcelain` off the async runtime
/// (blocking). Non-zero exit means "not a git repository"
/// (`worktree.not_git_repo`); a spawn failure or undecodable output means
/// `worktree.fetch_error`. Only the exit status is ever logged.
async fn git_worktree_list(worktree: &str) -> Result<String, &'static str> {
    let worktree = worktree.to_string();
    tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&worktree)
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .output()
            .map_err(|_| "worktree.fetch_error")
            .and_then(|output| {
                if !output.status.success() {
                    return Err("worktree.not_git_repo");
                }
                String::from_utf8(output.stdout).map_err(|_| "worktree.fetch_error")
            })
    })
    .await
    .map_err(|_| "worktree.fetch_error")?
}

/// Loaded `/worktree` context shared by the list reply and the select
/// callback.
struct WorktreeContext {
    items: Vec<crate::commands::WorktreeListItem>,
}

/// Load the current project's worktree list (JS `listPresentWorktrees`
/// parity: missing folders are left out). No project →
/// `worktree.project_not_selected`; not a git repo → `worktree.not_git_repo`.
async fn load_worktree_context() -> Result<WorktreeContext, &'static str> {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return Err("worktree.project_not_selected");
    };
    let output = git_worktree_list(&worktree).await?;
    let items: Vec<crate::commands::WorktreeListItem> = parse_git_worktree_porcelain(&output)
        .into_iter()
        // `listPresentWorktrees` parity: gone folders are left out, while
        // each row keeps its presented-list position for selection.
        // Paging slices per page in `worktree_view`; the full list is kept
        // here so later pages resolve.
        .filter(|(path, _)| std::path::Path::new(path).exists())
        .map(|(path, branch)| crate::commands::WorktreeListItem {
            is_current: worktree_key(&path) == worktree_key(&worktree),
            path,
            branch,
        })
        .collect();
    Ok(WorktreeContext { items })
}

/// Live `/worktree` view: the JS-parity text plus an inline keyboard with
/// one `worktree:<position>` button per row. No rows →
/// `worktree.empty`; otherwise the numbered `render_worktree` list for the
/// requested page plus `worktree:page:<n>` pager buttons when more than one
/// page exists (Node `worktree-selection-menu.ts` parity).
async fn worktree_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    let context = match load_worktree_context().await {
        Ok(context) => context,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let total = context.items.len();
    let limit = crate::config::PROJECTS_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &context.items[start.min(total)..end];
    let text = crate::commands::render_worktree(page_items, page, total_pages, Some(locale));
    let keyboard = if page_items.is_empty() {
        None
    } else {
        Some(build_worktree_keyboard(
            page_items,
            locale,
            page,
            total_pages,
        ))
    };
    (text, keyboard)
}

/// Load recent sessions across the known project directories, newest-first
/// (JS `loadRecentSessions` shape, minus liveness: statuses all read
/// `Idle` until the status/question/permission lookups land as a
/// follow-up). One bad directory never fails the list. Empty when no
/// project directory is known.
async fn load_recent_context() -> Vec<crate::commands::RecentListItem> {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let mut directories: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for worktree in current.project_worktree.iter().chain(
        crate::store::SessionCacheStore::open(&crate::store::resolve_db_path(&app_home))
            .ok()
            .and_then(|store| store.get_directories().ok())
            .unwrap_or_default()
            .iter()
            .map(|dir| &dir.worktree),
    ) {
        let worktree = worktree.trim();
        if worktree.is_empty() || !seen.insert(worktree_key(worktree)) {
            continue;
        }
        directories.push(worktree.to_string());
        if directories.len() >= 10 {
            break;
        }
    }
    if directories.is_empty() {
        return Vec::new();
    }
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let mut items: Vec<crate::commands::RecentListItem> = Vec::new();
    for directory in &directories {
        match client.list_sessions(Some(directory)).await {
            Ok(sessions) => {
                items.extend(sessions.iter().filter_map(|value| {
                    crate::commands::recent_item_from_value(
                        value,
                        crate::commands::RecentStatus::Idle,
                    )
                }));
            }
            Err(err) => {
                tracing::warn!("[otb-bot] telegram: recent list failed: {err}");
            }
        }
    }
    // Paging slices per page in `recent_view`; the full sorted list is kept
    // here so later pages resolve.
    crate::commands::sort_recent_items(items)
}

/// Live `/recent` view: the JS-parity heading + legend plus an inline
/// keyboard with one `recent:<position>` button per row. No rows →
/// `recent.empty`; otherwise the requested page plus `recent:page:<n>`
/// pager buttons when more than one page exists (scaffold paging; the Node
/// recent menu keeps a single page).
async fn recent_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    let items = load_recent_context().await;
    let total = items.len();
    let limit = crate::config::SESSIONS_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &items[start.min(total)..end];
    let text = crate::commands::render_recent(page_items, Some(locale));
    let keyboard = if page_items.is_empty() {
        None
    } else {
        Some(build_recent_keyboard(page_items, locale, page, total_pages))
    };
    (text, keyboard)
}

/// Loaded `/messages` context shared by the list reply and the select
/// callback.
/// Latest assistant text across raw `session.messages` entries (JS
/// `loadLatestAssistantResponse` parity in `message-history-service.ts`):
/// non-`assistant` and summary entries are skipped, `text` parts join, and
/// the entry with the greatest `info.time.created` wins (`>=` keeps the
/// later entry on ties). Returns `None` when nothing assistant-like
/// carries text. Pure; testable.
pub fn latest_assistant_text(messages: &[serde_json::Value]) -> Option<String> {
    let mut latest: Option<(i64, String)> = None;
    for message in messages {
        let info = message.get("info")?;
        if info.get("role")?.as_str() != Some("assistant") {
            continue;
        }
        if info
            .get("summary")
            .and_then(|summary| summary.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        let text = message
            .get("parts")?
            .as_array()?
            .iter()
            .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("text"))
            .filter_map(|part| part.get("text")?.as_str())
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();
        if text.is_empty() {
            continue;
        }
        let created = info
            .get("time")
            .and_then(|time| time.get("created"))
            .and_then(|created| created.as_i64())
            .unwrap_or(0);
        if match &latest {
            None => true,
            Some((at, _)) => created >= *at,
        } {
            latest = Some((created, text));
        }
    }
    latest.map(|(_, text)| text)
}

struct MessagesContext {
    items: Vec<crate::commands::UserMessageItem>,
    session_id: String,
    directory: String,
}

/// Load the current session's user messages, newest-first (JS
/// `loadUserMessages` parity, including the `revert.messageID` cutoff; a
/// failed session fetch only drops the cutoff, never the list).
async fn load_messages_context() -> Result<MessagesContext, &'static str> {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(project) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return Err("messages.project_not_selected");
    };
    let (Some(session_id), Some(session_dir)) = (
        current.session_id.clone().filter(|id| !id.is_empty()),
        current
            .session_directory
            .clone()
            .filter(|dir| !dir.is_empty()),
    ) else {
        return Err("messages.session_not_selected");
    };
    if session_dir != project {
        return Err("messages.session_project_mismatch");
    }
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.session_messages(&session_id, &session_dir).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: messages list failed: {err}");
            return Err("messages.fetch_error");
        }
    };
    let mut items: Vec<crate::commands::UserMessageItem> = raw
        .iter()
        .filter_map(crate::commands::user_message_item_from_value)
        .collect();
    items = crate::commands::sort_user_messages(items);
    if let Ok(session) = client.get_session(&session_id, &session_dir).await {
        if let Some(revert_id) = session
            .get("revert")
            .and_then(|revert| revert.get("messageID"))
            .and_then(|id| id.as_str())
        {
            if let Some(index) = items.iter().position(|item| item.id == revert_id) {
                items = items.into_iter().skip(index + 1).collect();
            }
        }
    }
    // Paging slices per page in `messages_view`; the full post-cutoff list
    // is kept here so later pages resolve.
    Ok(MessagesContext {
        items,
        session_id,
        directory: session_dir,
    })
}

/// Live `/messages` view: the JS-parity text plus an inline keyboard with
/// one `messages:select:<position>` button per row (read-only). Renders the
/// requested page plus `messages:page:<n>` pager buttons when more than one
/// page exists (Node `message-history-menu.ts` parity).
async fn messages_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    let context = match load_messages_context().await {
        Ok(context) => context,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let total = context.items.len();
    let limit = crate::config::MESSAGES_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &context.items[start.min(total)..end];
    let text = crate::commands::render_messages(page_items, page, Some(locale));
    let keyboard = if page_items.is_empty() {
        None
    } else {
        Some(build_messages_keyboard(
            page_items,
            locale,
            page,
            total_pages,
        ))
    };
    (text, keyboard)
}

/// Live `/sessions` view: the JS-parity text plus an inline keyboard with one
/// `session:<id>` button per row (read-only except the remembered-settings
/// read). Mirrors `sessions-command.ts`: no project → `project_not_selected`;
/// fetch failure → `fetch_error`; no rows → `sessions.empty`; otherwise the
/// numbered `render_sessions` list for the requested page plus
/// `session:page:<n>` pager buttons when more than one page exists (Node
/// `session-selection-menu.ts` parity).
async fn sessions_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    let context = match load_sessions_context().await {
        Ok(context) => context,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let total = context.items.len();
    let limit = crate::config::SESSIONS_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &context.items[start.min(total)..end];
    let text = crate::commands::render_sessions(
        page_items,
        context.current_id.as_deref(),
        page,
        Some(locale),
    );
    // Leading search row (Node `buildSessionsKeyboard` parity): opens the
    // stateless prompt-then-filter flow. `build_sessions_keyboard` stays
    // shared with the results view, which carries again/cancel instead.
    let keyboard = if page_items.is_empty() {
        None
    } else {
        let mut keyboard = build_sessions_keyboard(
            page_items,
            context.current_id.as_deref(),
            locale,
            page,
            total_pages,
        );
        keyboard.inline_keyboard.insert(
            0,
            vec![InlineKeyboardButton::callback(
                crate::locale::translate(locale, "sessions.search.button"),
                SESSION_SEARCH_CALLBACK.to_string(),
            )],
        );
        Some(keyboard)
    };
    (text, keyboard)
}

/// One `[search again, cancel]` row carrying the Node-exact
/// `...:search:again` / `...:search:cancel` callback shapes (pure).
fn search_again_cancel_row(
    locale: &str,
    again_key: &str,
    cancel_key: &str,
    again_data: &str,
    cancel_data: &str,
) -> Vec<InlineKeyboardButton> {
    vec![
        InlineKeyboardButton::callback(
            crate::locale::translate(locale, again_key),
            again_data.to_string(),
        ),
        InlineKeyboardButton::callback(
            crate::locale::translate(locale, cancel_key),
            cancel_data.to_string(),
        ),
    ]
}

/// Filtered `/sessions <query>` view (stateless Batch E1 search follow-up).
///
/// ASSUMPTION (stateless deviation): the scaffold keeps no per-chat
/// interaction memory, so the search mode Node holds in
/// `interactionManager` (`session-search` flow with `stage`/`query`/`ids`)
/// cannot be remembered. The typed query therefore travels as the
/// `/sessions <query>` command argument (the search-button prompt asks for
/// the query; the user's next text message carries it as command args),
/// and matching filters the live list in memory on every call (Node
/// `searchSessions` filter parity via [`matches_session_query`]).
/// Result rows carry direct `session:<id>` taps (no remembered `ids`
/// vector, so the Node `session:search:result:<index>` indirection cannot
/// resolve); tapping selects immediately like the Node result tap's end
/// effect. The list is capped to the first `SESSIONS_LIST_LIMIT` matches
/// with a `[search again, cancel]` row (Node
/// `buildSessionSearchView` parity minus paging: the query cannot
/// round-trip through `session:search:page:<n>` taps statelessly, so stale
/// page/result taps re-prompt instead). Fetch failure and no-project keep
/// the `sessions_view` error keys.
async fn sessions_search_view(locale: &str, query: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let query = truncate_search_query(query);
    let context = match load_sessions_context().await {
        Ok(context) => context,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let again_cancel = || {
        search_again_cancel_row(
            locale,
            "sessions.search.again",
            "sessions.search.cancel",
            SESSION_SEARCH_AGAIN_CALLBACK,
            SESSION_SEARCH_CANCEL_CALLBACK,
        )
    };
    let matched: Vec<crate::commands::SessionListItem> = context
        .items
        .iter()
        .filter(|item| matches_session_query(item, &query))
        .cloned()
        .collect();
    if matched.is_empty() {
        let text = crate::locale::translate(locale, "sessions.search.no_results")
            .replace("{query}", &query);
        return (text, Some(InlineKeyboardMarkup::new(vec![again_cancel()])));
    }
    let limit = crate::config::SESSIONS_LIST_LIMIT.max(1);
    let page_items: Vec<crate::commands::SessionListItem> =
        matched.into_iter().take(limit).collect();
    let text =
        crate::locale::translate(locale, "sessions.search.results").replace("{query}", &query);
    let mut keyboard =
        build_sessions_keyboard(&page_items, context.current_id.as_deref(), locale, 0, 1);
    keyboard.inline_keyboard.push(again_cancel());
    (text, Some(keyboard))
}

/// Send a text message with an inline keyboard (thin `sendMessage` parity).
async fn send_message_with_keyboard(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    text: &str,
    keyboard: InlineKeyboardMarkup,
) -> Result<(), teloxide_core::RequestError> {
    let (chat, body) = build_send_payload(chat_id, text);
    bot.send_message(chat, body).reply_markup(keyboard).await?;
    Ok(())
}

/// Answer a callback query to clear the client spinner (Node
/// `answerCallbackQuery` parity). `text` is only set for failures; only the
/// error is ever logged.
async fn answer_callback(
    bot: &teloxide_core::Bot,
    query: &CallbackQuery,
    text: Option<String>,
    show_alert: bool,
) {
    let mut request = bot.answer_callback_query(query.id.clone());
    if let Some(text) = text {
        request = request.text(text);
    }
    if show_alert {
        request = request.show_alert(true);
    }
    if let Err(err) = request.await {
        tracing::debug!("[otb-bot] telegram: answer callback failed: {err}");
    }
}

/// Chat the callback's message belongs to, when it is still accessible.
fn callback_chat_id(query: &CallbackQuery) -> Option<i64> {
    match &query.message {
        Some(MaybeInaccessibleMessage::Regular(message)) => Some(message.chat.id.0),
        _ => None,
    }
}

/// Chat plus message id the callback's message carries, when it is still
/// accessible (needed for the cancel delete and the detail target match).
fn callback_message_ids(query: &CallbackQuery) -> Option<(ChatId, MessageId)> {
    match &query.message {
        Some(MaybeInaccessibleMessage::Regular(message)) => Some((message.chat.id, message.id)),
        _ => None,
    }
}

/// Text of the callback's message, when it is still accessible (the detail
/// target match reads the rendered detail body back).
fn callback_message_text(query: &CallbackQuery) -> Option<&str> {
    match &query.message {
        Some(MaybeInaccessibleMessage::Regular(message)) => message.text(),
        _ => None,
    }
}

/// Handle a `session:<id>` tap: resolve the title from the project session
/// list, persist it via `write_current_session` (JS `setCurrentSession`
/// parity), ack the callback, and confirm with the `sessions.selected`
/// template. Only the id prefix and length are ever logged — never values.
async fn handle_session_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    // Page taps re-render the requested sessions page (Node
    // `session-selection-menu.ts` pager parity); the ack clears the spinner.
    if let Some(page) = parse_session_page_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = sessions_view(locale, page).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    let Some(session_id) = parse_session_id_callback(data) else {
        return;
    };
    tracing::debug!(
        "[otb-bot] telegram: session callback id_prefix={} id_len={}",
        session_id.chars().take(8).collect::<String>(),
        session_id.len()
    );
    let context = match load_sessions_context().await {
        Ok(context) => context,
        Err(key) => {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, key)),
                true,
            )
            .await;
            return;
        }
    };
    let Some(item) = context.items.iter().find(|item| item.id == session_id) else {
        tracing::warn!(
            "[otb-bot] telegram: session callback for unknown id (id_len={})",
            session_id.len()
        );
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "sessions.select_error")),
            true,
        )
        .await;
        return;
    };
    let app_home = crate::config::resolve_app_home();
    if let Err(err) = crate::settings_current::write_current_session(
        &app_home,
        &item.id,
        &item.title,
        &context.worktree,
    ) {
        tracing::warn!("[otb-bot] telegram: cannot remember selected session: {err}");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "sessions.select_error")),
            true,
        )
        .await;
        return;
    }
    tracing::info!(
        "[otb-bot] telegram: session selected id_prefix={} id_len={}",
        item.id.chars().take(8).collect::<String>(),
        item.id.len()
    );
    answer_callback(bot, query, None, false).await;
    // RC-1 (TS `resetInteractions("session_switched")` parity): clear the
    // stale interaction slot before following the newly selected session so
    // the old session's held question/permission never blocks its prompts.
    crate::attach::release_interaction_slot();
    // Live path (TS `selectSessionById` → `attachToSession` core chain
    // parity): follow the session so its event stream pumps and the
    // assistant answer arrives over SSE. Non-fatal by construction.
    crate::attach::attach_session(&item.id, &context.worktree);
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let confirmation = crate::commands::render_session_selected(&item.title, Some(locale));
    if let Err(err) = send_message(bot, chat_id, &confirmation).await {
        tracing::warn!("[otb-bot] telegram: select confirmation failed for chat {chat_id}: {err}");
    }
    // Last user input as quote, last assistant reply in full, plus the
    // newest still-waiting input (TS `sendSessionPickTranscript` parity).
    send_attach_transcript(bot, chat_id, &item.id, &context.worktree, locale).await;
}

/// Handle a `session:search*` tap (JS
/// `handleSessionSearchCallback` parity, stateless Batch E1 follow-up).
/// `session:search` / `session:search:again` ack the spinner and send the
/// `sessions.search.prompt` text (the typed query returns as
/// `/sessions <query>` command args — see [`sessions_search_view`]);
/// `session:search:cancel` deletes the prompt message (Node `cancelMenu`
/// parity). Stale `session:search:page:<n>` /
/// `session:search:result:<n>` taps cannot resolve without the remembered
/// query/ids, so they ack and re-prompt instead of guessing.
async fn handle_session_search_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if data == SESSION_SEARCH_CALLBACK || data == SESSION_SEARCH_AGAIN_CALLBACK {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "sessions.search.prompt"),
                None,
            )
            .await;
        }
        return;
    }
    if data == SESSION_SEARCH_CANCEL_CALLBACK {
        answer_callback(bot, query, None, false).await;
        if let Some((chat, message)) = callback_message_ids(query) {
            if let Err(err) = bot.delete_message(chat, message).await {
                tracing::debug!("[otb-bot] telegram: session search cancel delete failed: {err}");
            }
        }
        return;
    }
    if parse_session_search_page_callback(data).is_some()
        || parse_session_search_result_callback(data).is_some()
    {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "sessions.search.prompt"),
                None,
            )
            .await;
        }
    }
}

/// Best-effort `resolveProjectAgent` fallback (TS
/// `agent-selection-service.ts:53-76`): after a project switch, when the
/// stored agent is not listed for the new worktree, persist the
/// build-preferring fallback. Empty catalogs and write failures only log.
async fn apply_project_agent_fallback(worktree: &str, app_home: &std::path::Path) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_agents(worktree).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: agents list failed: {err}");
            return;
        }
    };
    let items = crate::commands::agent_list_from_values(&raw);
    if items.is_empty() {
        return;
    }
    let current = crate::settings_current::read_settings_current(app_home);
    let stored = current.agent.as_deref().unwrap_or("build");
    let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
    if otb_core::agent_model_variant_gates::is_agent_listed(&names, stored) {
        return;
    }
    let fallback = otb_core::agent_model_variant_gates::pick_fallback_agent(&names);
    if let Err(err) = crate::settings_current::write_current_agent(app_home, &fallback) {
        tracing::warn!("[otb-bot] telegram: cannot remember fallback agent: {err}");
    } else {
        tracing::info!("[otb-bot] telegram: project agent fallback applied");
    }
}

/// Handle a `project:<id>` tap: persist it as `currentProject` (dropping
/// the session, JS `setCurrentProject` parity) and confirm with the
/// `projects.selected` template. Only the id prefix and length are ever
/// logged — never values.
async fn handle_project_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    // Page taps re-render the requested projects page (Node
    // `project-selection-menu.ts` pager parity); the ack clears the spinner.
    if let Some(page) = parse_project_page_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = projects_view(locale, page).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    let Some(project_id) = parse_project_id_callback(data) else {
        return;
    };
    tracing::debug!(
        "[otb-bot] telegram: project callback id_prefix={} id_len={}",
        project_id.chars().take(8).collect::<String>(),
        project_id.len()
    );
    let context = match load_projects_context().await {
        Ok(context) => context,
        Err(key) => {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, key)),
                true,
            )
            .await;
            return;
        }
    };
    let Some(item) = context.items.iter().find(|item| item.id == project_id) else {
        tracing::warn!(
            "[otb-bot] telegram: project callback for unknown id (id_len={})",
            project_id.len()
        );
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "projects.select_error")),
            true,
        )
        .await;
        return;
    };
    let app_home = crate::config::resolve_app_home();
    if let Err(err) = crate::settings_current::write_current_project(
        &app_home,
        &item.id,
        &item.worktree,
        &item.name,
    ) {
        tracing::warn!("[otb-bot] telegram: cannot remember selected project: {err}");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "projects.select_error")),
            true,
        )
        .await;
        return;
    }
    tracing::info!(
        "[otb-bot] telegram: project selected id_prefix={} id_len={}",
        item.id.chars().take(8).collect::<String>(),
        item.id.len()
    );
    apply_project_agent_fallback(&item.worktree, &app_home).await;
    answer_callback(bot, query, None, false).await;
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let confirmation = crate::locale::translate(locale, "projects.selected").replace(
        "{project}",
        if item.name.trim().is_empty() {
            &item.worktree
        } else {
            &item.name
        },
    );
    if let Err(err) = send_message(bot, chat_id, &confirmation).await {
        tracing::warn!("[otb-bot] telegram: select confirmation failed for chat {chat_id}: {err}");
    }
}

/// Handle a `projects:search*` tap (JS `handleProjectSearchCallback`
/// parity, stateless Batch E1 follow-up). `projects:search` /
/// `projects:search:again` ack the spinner and send the
/// `projects.search.prompt` text (the typed query returns as
/// `/projects <query>` command args — see [`projects_search_view`]);
/// `projects:search:cancel` deletes the prompt message (Node `cancelMenu`
/// parity). Stale `projects:search:page:<n>` /
/// `projects:search:result:<n>` taps cannot resolve without the remembered
/// query/ids, so they ack and re-prompt instead of guessing.
async fn handle_project_search_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if data == PROJECT_SEARCH_CALLBACK || data == PROJECT_SEARCH_AGAIN_CALLBACK {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "projects.search.prompt"),
                None,
            )
            .await;
        }
        return;
    }
    if data == PROJECT_SEARCH_CANCEL_CALLBACK {
        answer_callback(bot, query, None, false).await;
        if let Some((chat, message)) = callback_message_ids(query) {
            if let Err(err) = bot.delete_message(chat, message).await {
                tracing::debug!("[otb-bot] telegram: project search cancel delete failed: {err}");
            }
        }
        return;
    }
    if parse_project_search_page_callback(data).is_some()
        || parse_project_search_result_callback(data).is_some()
    {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "projects.search.prompt"),
                None,
            )
            .await;
        }
    }
}

/// Handle a `worktree:<position>` tap: switch `currentProject.worktree` to
/// the tapped row (dropping the session, JS worktree-select parity) and
/// confirm with the `worktree.selected` template.
async fn handle_worktree_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    // Page taps re-render the requested worktree page (Node
    // `worktree-selection-menu.ts` pager parity); the ack clears the spinner.
    if let Some(page) = parse_worktree_page_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = worktree_view(locale, page).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    let Some(position) = parse_worktree_index_callback(data) else {
        return;
    };
    let context = match load_worktree_context().await {
        Ok(context) => context,
        Err(key) => {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, key)),
                true,
            )
            .await;
            return;
        }
    };
    let Some(item) = context.items.get(position) else {
        tracing::warn!("[otb-bot] telegram: worktree callback for unknown position");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "worktree.selection_missing_callback",
            )),
            true,
        )
        .await;
        return;
    };
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    // Already on this worktree: ack like the Node `already_selected` path.
    if current.project_worktree.as_deref() == Some(item.path.as_str()) {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "worktree.already_selected_callback",
            )),
            true,
        )
        .await;
        return;
    }
    if let Err(err) = crate::settings_current::write_current_project(
        &app_home,
        current.project_id.as_deref().unwrap_or(""),
        &item.path,
        current.project_name.as_deref().unwrap_or(""),
    ) {
        tracing::warn!("[otb-bot] telegram: cannot remember selected worktree: {err}");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "worktree.select_error")),
            true,
        )
        .await;
        return;
    }
    tracing::info!("[otb-bot] telegram: worktree selected");
    answer_callback(bot, query, None, false).await;
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let confirmation =
        crate::locale::translate(locale, "worktree.selected").replace("{worktree}", &item.path);
    if let Err(err) = send_message(bot, chat_id, &confirmation).await {
        tracing::warn!("[otb-bot] telegram: select confirmation failed for chat {chat_id}: {err}");
    }
}

/// Handle a `recent:<position>` tap: resolve the row from a fresh recent
/// list, persist it as the current session (JS recent-select parity), and
/// confirm with the `sessions.selected` template.
async fn handle_recent_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    // Page taps re-render the requested recent page (scaffold paging); the
    // ack clears the spinner.
    if let Some(page) = parse_recent_page_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = recent_view(locale, page).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    let Some(position) = parse_recent_index_callback(data) else {
        return;
    };
    let items = load_recent_context().await;
    let Some(item) = items.get(position) else {
        tracing::warn!("[otb-bot] telegram: recent callback for unknown position");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "sessions.select_error")),
            true,
        )
        .await;
        return;
    };
    let app_home = crate::config::resolve_app_home();
    if let Err(err) = crate::settings_current::write_current_session(
        &app_home,
        &item.id,
        &item.title,
        &item.directory,
    ) {
        tracing::warn!("[otb-bot] telegram: cannot remember recent session: {err}");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "sessions.select_error")),
            true,
        )
        .await;
        return;
    }
    tracing::info!(
        "[otb-bot] telegram: recent session selected id_prefix={} id_len={}",
        item.id.chars().take(8).collect::<String>(),
        item.id.len()
    );
    answer_callback(bot, query, None, false).await;
    // RC-1 (TS `resetInteractions("session_switched")` parity): clear the
    // stale interaction slot before following the newly selected session so
    // the old session's held question/permission never blocks its prompts.
    crate::attach::release_interaction_slot();
    // Live path (TS background/`recent` select → `attachToSession` core
    // chain parity): follow the session so its event stream pumps.
    // Non-fatal by construction.
    crate::attach::attach_session(&item.id, &item.directory);
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let confirmation = crate::commands::render_session_selected(&item.title, Some(locale));
    if let Err(err) = send_message(bot, chat_id, &confirmation).await {
        tracing::warn!("[otb-bot] telegram: select confirmation failed for chat {chat_id}: {err}");
    }
    // Last user input as quote, last assistant reply in full, plus the
    // newest still-waiting input (TS `sendSessionPickTranscript` parity).
    send_attach_transcript(bot, chat_id, &item.id, &item.directory, locale).await;
}

/// Handle a `messages:*` tap: page taps re-render the requested list page
/// (Node `message-history-menu.ts` pager parity); select taps re-fetch the
/// current session's user messages and show the tapped one with the detail
/// keyboard (`messages:revert` / `messages:fork` / `messages:back` /
/// `messages:cancel`); revert posts the versioned revert endpoint and
/// confirms with `messages.revert_success` (a rejected call renders the
/// local `messages.revert_error` notice plus the list view); fork creates a
/// forked session, persists it like `setCurrentSession`, and confirms with
/// `messages.fork_success` (failures render `messages.fork_error`); back
/// re-renders the list's first page; cancel acks and deletes the prompt
/// message (Node `cancelMenu` parity). The spinner is always acked first.
async fn handle_messages_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    // Detail actions carry the exact Node `MESSAGES_CALLBACK_*` shapes; the
    // ack clears the spinner before the live fetch.
    if is_messages_revert(data) {
        answer_callback(bot, query, None, false).await;
        handle_messages_revert(bot, locale, query).await;
        return;
    }
    if is_messages_fork(data) {
        answer_callback(bot, query, None, false).await;
        handle_messages_fork(bot, locale, query).await;
        return;
    }
    if is_messages_back(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = messages_view(locale, 0).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if is_messages_cancel(data) {
        answer_callback(bot, query, None, false).await;
        // Node `cancelMenu` deletes the prompt message; a gone message is
        // not an error worth surfacing. Only ids are ever logged.
        if let Some((chat, message)) = callback_message_ids(query) {
            if let Err(err) = bot.delete_message(chat, message).await {
                tracing::debug!("[otb-bot] telegram: cancel delete failed: {err}");
            }
        }
        return;
    }
    // Page taps re-render the requested messages page (Node
    // `message-history-menu.ts` pager parity); the ack clears the spinner.
    if let Some(page) = parse_message_page_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = messages_view(locale, page).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    let Some(position) = parse_message_select_callback(data) else {
        return;
    };
    let context = match load_messages_context().await {
        Ok(context) => context,
        Err(key) => {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, key)),
                true,
            )
            .await;
            return;
        }
    };
    let Some(item) = context.items.get(position) else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "messages.page_empty_callback",
            )),
            true,
        )
        .await;
        return;
    };
    answer_callback(bot, query, None, false).await;
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let detail = crate::commands::render_message_detail(item);
    let keyboard = build_message_detail_keyboard(locale);
    if let Err(err) = send_message_with_keyboard(bot, chat_id, &detail, keyboard).await {
        tracing::warn!("[otb-bot] telegram: message detail failed for chat {chat_id}: {err}");
    }
}

/// Handle a `messages:revert` tap: re-fetch the list, recover the detail
/// target from the detail message text, and post the versioned revert
/// endpoint (JS `session.revert` parity). Success confirms with
/// `messages.revert_success`; a missing target answers
/// `messages.inactive_callback` plus the list view; a rejected endpoint
/// (missing route or server error) renders the local
/// `messages.revert_error` notice plus the list view. Only the message id
/// length is ever logged — never values.
async fn handle_messages_revert(bot: &teloxide_core::Bot, locale: &str, query: &CallbackQuery) {
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let context = match load_messages_context().await {
        Ok(context) => context,
        Err(key) => {
            reply_list_view(bot, chat_id, &crate::locale::translate(locale, key), None).await;
            return;
        }
    };
    let target = callback_message_text(query)
        .and_then(|text| resolve_message_detail_index(text, &context.items))
        .and_then(|index| context.items.get(index));
    let Some(target) = target else {
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, "messages.inactive_callback"),
            None,
        )
        .await;
        let (text, keyboard) = messages_view(locale, 0).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    };
    tracing::debug!(
        "[otb-bot] telegram: messages revert tap (id_len={})",
        target.id.len()
    );
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    match client
        .revert_session(&context.session_id, &context.directory, &target.id)
        .await
    {
        Ok(true) => {
            let confirmation = crate::locale::translate(locale, "messages.revert_success")
                .replace("{text}", &target.text);
            reply_list_view(bot, chat_id, &confirmation, None).await;
        }
        Ok(false) | Err(_) => {
            tracing::warn!("[otb-bot] telegram: messages revert failed");
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "messages.revert_error"),
                None,
            )
            .await;
            let (text, keyboard) = messages_view(locale, 0).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
    }
}

/// Handle a `messages:fork` tap: re-fetch the list, recover the detail
/// target from the detail message text, and create a forked session
/// (JS `session.fork` parity). The forked session is persisted like
/// `setCurrentSession` and confirmed with `messages.fork_success`; a
/// missing target answers `messages.inactive_callback` plus the list view;
/// failures render `messages.fork_error`. Only id prefixes and lengths are
/// ever logged — never values.
async fn handle_messages_fork(bot: &teloxide_core::Bot, locale: &str, query: &CallbackQuery) {
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let context = match load_messages_context().await {
        Ok(context) => context,
        Err(key) => {
            reply_list_view(bot, chat_id, &crate::locale::translate(locale, key), None).await;
            return;
        }
    };
    let target = callback_message_text(query)
        .and_then(|text| resolve_message_detail_index(text, &context.items))
        .and_then(|index| context.items.get(index));
    let Some(target) = target else {
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, "messages.inactive_callback"),
            None,
        )
        .await;
        let (text, keyboard) = messages_view(locale, 0).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let forked = match client
        .fork_session(&context.session_id, &context.directory, &target.id)
        .await
    {
        Ok(forked) => forked,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: messages fork failed: {err}");
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "messages.fork_error"),
                None,
            )
            .await;
            return;
        }
    };
    let (id, title) = session_identity_from_value(&forked);
    if id.is_empty() {
        tracing::warn!("[otb-bot] telegram: messages fork returned no id");
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, "messages.fork_error"),
            None,
        )
        .await;
        return;
    }
    let app_home = crate::config::resolve_app_home();
    if let Err(err) =
        crate::settings_current::write_current_session(&app_home, &id, &title, &context.directory)
    {
        tracing::warn!("[otb-bot] telegram: cannot remember forked session: {err}");
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, "messages.fork_error"),
            None,
        )
        .await;
        return;
    }
    tracing::info!(
        "[otb-bot] telegram: messages fork created id_prefix={} id_len={}",
        id.chars().take(8).collect::<String>(),
        id.len()
    );
    let confirmation =
        crate::locale::translate(locale, "messages.fork_success").replace("{text}", &target.text);
    reply_list_view(bot, chat_id, &confirmation, None).await;
    // `messages.sendLatestAssistantResponse` parity
    // (`message-history-callback-handler.ts:314-323`): the forked session's
    // latest assistant answer follows in the background
    // (`safeBackgroundTask`). Best-effort: a quiet stream or a missing
    // answer only logs, never edits the confirm above.
    let bot_owned = bot.clone();
    let forked_id = id.clone();
    let forked_dir = context.directory.clone();
    tokio::spawn(async move {
        let client = crate::config::AppConfig::load()
            .map(|config| config.opencode_client())
            .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
        let raw = match client.session_messages(&forked_id, &forked_dir).await {
            Ok(raw) => raw,
            Err(err) => {
                tracing::debug!("[otb-bot] telegram: fork follow-up messages failed: {err}");
                return;
            }
        };
        let Some(answer) = latest_assistant_text(&raw) else {
            return;
        };
        if let Err(err) = send_message(&bot_owned, chat_id, &answer).await {
            tracing::debug!("[otb-bot] telegram: fork follow-up send failed: {err}");
        }
    });
}

/// Live `/new` reply: create a session in the current project directory and
/// remember it as `currentSession` (JS `new-command.ts` parity, minus the
/// keyboard side effects that need the dispatcher). Plain-text
/// prompts reuse this remembered session via `ensure_execute_session`.
/// No project → `new.project_not_selected`; create failure (or a bad write
/// of the remembered session) → `new.create_error`; otherwise `new.created`.
async fn new_reply(locale: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return crate::locale::translate(locale, "new.project_not_selected");
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let session = match client.create_session(&worktree, None).await {
        Ok(session) => session,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: session create failed: {err}");
            return crate::locale::translate(locale, "new.create_error");
        }
    };
    let (id, title) = session_identity_from_value(&session);
    if id.is_empty() {
        tracing::warn!("[otb-bot] telegram: session create returned no id");
        return crate::locale::translate(locale, "new.create_error");
    }
    if let Err(err) =
        crate::settings_current::write_current_session(&app_home, &id, &title, &worktree)
    {
        tracing::warn!("[otb-bot] telegram: cannot remember new session: {err}");
        return crate::locale::translate(locale, "new.create_error");
    }
    // RC-1 (TS `resetInteractions("session_created")` parity): drop any stale
    // held question/permission slot from the previous session so the next
    // prompt is not wedged by `prompt_blocked_hint`.
    crate::attach::release_interaction_slot();
    // RC-2 (TS `new-command.ts:79-84` `attachToSession` parity): follow the
    // new session so its event stream pumps. Best-effort and non-fatal by
    // construction (the SSE pump itself stays scaffold-gated until polling
    // is enabled).
    crate::attach::attach_session(&id, &worktree);
    crate::commands::render_new_created(&title, Some(locale))
}

/// Live `/status` reply: fresh health probe rendered through the JS-parity
/// template (read-only; the token value is never logged).
async fn status_reply(locale: &str) -> String {
    // Credentials travel with the probe: without them a password-protected
    // server answers 401 and the snapshot below correctly reads unavailable.
    // `AppConfig::load` also pulls in the App Home `.env` first so the probe
    // sees the same credentials Node uses; only status codes are ever logged.
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let probe = match client.health().await {
        Ok(probe) => probe,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: status probe failed: {err}");
            crate::opencode::HealthProbe {
                ok: false,
                version: None,
            }
        }
    };
    crate::commands::render_status(&build_status_snapshot(&probe), Some(locale))
}

/// Live `/tasklist` reply: sorted local-store list rendered through the
/// JS-parity template (read-only; a missing or invalid store reads empty).
/// Keeps the scaffold text-only: the open/delete/toggle keyboard needs a
/// dispatcher and stays a follow-up.
fn tasklist_reply(locale: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    crate::commands::render_tasklist(&load_scheduled_task_items(&app_home), Some(locale))
}

/// Load scheduled tasks from `<app_home>/settings.json` (`scheduledTasks`,
/// settings-store parity), tolerantly: a missing, unreadable, or invalid
/// file (or a non-array value) yields an empty list. Sorts with the JS
/// `sortTasks` parity helper. Only paths/error kinds are ever logged —
/// never task values.
fn load_scheduled_task_items(
    app_home: &std::path::Path,
) -> Vec<crate::commands::ScheduledTaskItem> {
    let path = crate::settings_current::settings_file_path(app_home);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            tracing::debug!(
                "[otb-bot] telegram: no task store at {} ({})",
                path.display(),
                err.kind()
            );
            return Vec::new();
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            tracing::debug!(
                "[otb-bot] telegram: ignoring unparseable task store ({} bytes): {err}",
                content.len()
            );
            return Vec::new();
        }
    };
    let tasks = match value
        .get("scheduledTasks")
        .and_then(|tasks| tasks.as_array())
    {
        Some(tasks) => tasks,
        None => return Vec::new(),
    };
    let items: Vec<crate::commands::ScheduledTaskItem> = tasks
        .iter()
        .filter_map(crate::commands::scheduled_task_item_from_value)
        .collect();
    crate::commands::sort_scheduled_tasks(items)
}

/// Live `/commands` reply: the current project's custom-command catalog
/// (JS `commands-command.ts` parity: no project → `bot.project_not_selected`;
/// fetch failure → `commands.fetch_error`; empty → `commands.empty`;
/// otherwise the numbered text list for the requested page plus a keyboard
/// with one `commands:select:<index>` button per row, optional
/// `commands:page:<n>` pager buttons, and a `commands:cancel` button (Node
/// `command-catalog-menu.ts` parity). Read-only.
async fn commands_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    use otb_core::session_pick_catalog_gates::filter_command_catalog_items;
    let (worktree, items) = match load_commands_catalog().await {
        Ok(loaded) => loaded,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let _ = worktree;
    let total = items.len();
    let limit = crate::config::CATALOG_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &items[start.min(total)..end];
    let text = crate::commands::render_catalog("commands", page_items, page, Some(locale));
    let keyboard = if page_items.is_empty() {
        None
    } else {
        Some(build_catalog_list_keyboard(
            page_items,
            start,
            CatalogKind::Commands,
            locale,
            page,
            total_pages,
        ))
    };
    (text, keyboard)
}

/// Live `/skills` reply: same shape as [`commands_view`] over the skill
/// catalog (JS `skills-catalog-command.ts` parity).
async fn skills_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    use otb_core::session_pick_catalog_gates::filter_skills_catalog_items;
    let (worktree, items) = match load_skills_catalog().await {
        Ok(loaded) => loaded,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let _ = worktree;
    let total = items.len();
    let limit = crate::config::CATALOG_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &items[start.min(total)..end];
    let text = crate::commands::render_catalog("skills", page_items, page, Some(locale));
    let keyboard = if page_items.is_empty() {
        None
    } else {
        Some(build_catalog_list_keyboard(
            page_items,
            start,
            CatalogKind::Skills,
            locale,
            page,
            total_pages,
        ))
    };
    (text, keyboard)
}

/// Convert a `serde_json::Value` into the no-serde
/// [`otb_core::session_pick_catalog_gates::CatalogJson`] shape for
/// `parseMcpCatalogServers` (pure; testable). Numbers and bools collapse to
/// `Null` (only `Str` payloads are ever inspected).
fn catalog_json_from_value(
    value: &serde_json::Value,
) -> otb_core::session_pick_catalog_gates::CatalogJson {
    use otb_core::session_pick_catalog_gates::CatalogJson as Json;
    match value {
        serde_json::Value::Null => Json::Null,
        serde_json::Value::String(text) => Json::Str(text.clone()),
        serde_json::Value::Array(items) => {
            Json::Arr(items.iter().map(catalog_json_from_value).collect())
        }
        serde_json::Value::Object(map) => Json::Obj(
            map.iter()
                .map(|(key, item)| (key.clone(), catalog_json_from_value(item)))
                .collect(),
        ),
        _ => Json::Null,
    }
}

/// Live `/mcps` reply: the current project's MCP server statuses
/// (JS `mcp-catalog-command.ts` parity: no project → `bot.project_not_selected`;
/// fetch failure or an invalid shape → `mcps.fetch_error`; empty →
/// `mcps.empty`; otherwise the numbered text list plus a keyboard with one
/// `mcps:select:<index>` button per server and a cancel button (Node
/// `mcp-catalog-menu.ts` parity). Read-only.
async fn mcps_view(locale: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let servers = match load_mcps_catalog().await {
        Ok((_, servers)) => servers,
        Err(key) => return (crate::locale::translate(locale, key), None),
    };
    let text = crate::commands::render_mcps(&servers, Some(locale));
    let keyboard = if servers.is_empty() {
        None
    } else {
        Some(build_mcps_list_keyboard(&servers, locale))
    };
    (text, keyboard)
}

/// Node `COMMANDS_CALLBACK_*` parity (`command-catalog-menu.ts`):
/// list rows carry `commands:select:<globalIndex>`, pager rows carry
/// `commands:page:<n>`, and the confirm row carries the exact
/// `commands:execute` / `commands:cancel` shapes.
pub const COMMANDS_CALLBACK_PREFIX: &str = "commands:";
pub const COMMANDS_CALLBACK_SELECT_PREFIX: &str = "commands:select:";
pub const COMMANDS_CALLBACK_PAGE_PREFIX: &str = "commands:page:";
pub const COMMANDS_CALLBACK_CANCEL: &str = "commands:cancel";
pub const COMMANDS_CALLBACK_EXECUTE: &str = "commands:execute";

/// Node `SKILLS_CALLBACK_*` parity (`skills-catalog-menu.ts`): same shapes
/// under the `skills:` prefix.
pub const SKILLS_CALLBACK_PREFIX: &str = "skills:";
pub const SKILLS_CALLBACK_SELECT_PREFIX: &str = "skills:select:";
pub const SKILLS_CALLBACK_PAGE_PREFIX: &str = "skills:page:";
pub const SKILLS_CALLBACK_CANCEL: &str = "skills:cancel";
pub const SKILLS_CALLBACK_EXECUTE: &str = "skills:execute";

/// Node `MCPS_CALLBACK_*` parity (`mcp-catalog-menu.ts`): detail rows carry
/// `mcps:select:<index>`; the detail keyboard carries the exact
/// `mcps:toggle` / `mcps:back` / `mcps:cancel` shapes.
pub const MCPS_CALLBACK_PREFIX: &str = "mcps:";
pub const MCPS_CALLBACK_SELECT_PREFIX: &str = "mcps:select:";
pub const MCPS_CALLBACK_TOGGLE: &str = "mcps:toggle";
pub const MCPS_CALLBACK_BACK: &str = "mcps:back";
pub const MCPS_CALLBACK_CANCEL: &str = "mcps:cancel";

/// Model/agent/variant callback prefixes (Node parity: the `MODEL_*`
/// constants in `model-selection-menu.ts`, `"agent:"` in
/// `agent-selection-menu.ts`, `"variant:"` in
/// `variant-selection-menu.ts`). Values reuse
/// `otb_core::agent_model_variant_gates` so the shapes cannot drift.
pub const MODEL_LIST_CALLBACK_PREFIX: &str =
    otb_core::agent_model_variant_gates::MODEL_LIST_CALLBACK_PREFIX;
pub const MODEL_PROVIDERS_CALLBACK_PREFIX: &str =
    otb_core::agent_model_variant_gates::MODEL_PROVIDERS_CALLBACK_PREFIX;
pub const MODEL_PROVIDER_CALLBACK_PREFIX: &str =
    otb_core::agent_model_variant_gates::MODEL_PROVIDER_CALLBACK_PREFIX;
pub const MODEL_PROVIDER_MODEL_CALLBACK_PREFIX: &str =
    otb_core::agent_model_variant_gates::MODEL_PROVIDER_MODEL_CALLBACK_PREFIX;
pub const MODEL_SEARCH_CALLBACK: &str = otb_core::agent_model_variant_gates::MODEL_SEARCH_CALLBACK;
pub const MODEL_SEARCH_AGAIN_CALLBACK: &str =
    otb_core::agent_model_variant_gates::MODEL_SEARCH_AGAIN_CALLBACK;
pub const MODEL_SEARCH_CANCEL_CALLBACK: &str =
    otb_core::agent_model_variant_gates::MODEL_SEARCH_CANCEL_CALLBACK;
pub const MODEL_ROOT_CALLBACK: &str = otb_core::agent_model_variant_gates::MODEL_ROOT_CALLBACK;
/// Node `MODEL_SEARCH_RESULT_CALLBACK_PREFIX` parity
/// (`model-selection-callback-handler.ts`): search-result picks carry
/// `callback_data = "model:result:<index>"` — note the prefix is *not*
/// `model:search:result:` (unlike the session/project search shapes).
pub const MODEL_SEARCH_RESULT_CALLBACK_PREFIX: &str =
    otb_core::agent_model_variant_gates::MODEL_SEARCH_RESULT_CALLBACK_PREFIX;
pub const AGENT_CALLBACK_PREFIX: &str = otb_core::agent_model_variant_gates::AGENT_CALLBACK_PREFIX;
pub const VARIANT_CALLBACK_PREFIX: &str =
    otb_core::agent_model_variant_gates::VARIANT_CALLBACK_PREFIX;

/// Node `settings:*` callback shapes (parity with
/// `src/bot/menus/settings-menu.ts` + `settings-callback-handler.ts`).
/// Values reuse `otb_core::settings_callback_gates` so the shapes cannot
/// drift.
pub const SETTINGS_CALLBACK_PREFIX: &str =
    otb_core::settings_callback_gates::SETTINGS_CALLBACK_PREFIX;
pub const SETTINGS_COMPACT_OUTPUT_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_COMPACT_OUTPUT_CALLBACK;
pub const SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK;
pub const SETTINGS_THINKING_CONTENT_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_THINKING_CONTENT_CALLBACK;
pub const SETTINGS_RESPONSE_STREAMING_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_RESPONSE_STREAMING_CALLBACK;
pub const SETTINGS_DIFF_FILES_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_DIFF_FILES_CALLBACK;
pub const SETTINGS_ASSISTANT_FOOTER_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_ASSISTANT_FOOTER_CALLBACK;
pub const SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK;
pub const SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK;
pub const SETTINGS_TTS_CALLBACK: &str = otb_core::settings_callback_gates::SETTINGS_TTS_CALLBACK;
pub const SETTINGS_PROMPT_QUEUE_CALLBACK: &str =
    otb_core::settings_callback_gates::SETTINGS_PROMPT_QUEUE_CALLBACK;

/// Node `task:*` / `tasklist:*` callback shapes (parity with
/// `src/bot/menus/scheduled-task-menu.ts` +
/// `scheduled-task-callback-handler.ts`). Values reuse
/// `otb_core::recent_scheduled_gates` so the shapes cannot drift.
pub const TASK_RETRY_SCHEDULE_CALLBACK: &str =
    otb_core::recent_scheduled_gates::TASK_RETRY_SCHEDULE_CALLBACK;
pub const TASK_CANCEL_CALLBACK: &str = otb_core::recent_scheduled_gates::TASK_CANCEL_CALLBACK;
pub const TASKLIST_CALLBACK_PREFIX: &str =
    otb_core::recent_scheduled_gates::TASKLIST_CALLBACK_PREFIX;
pub const TASKLIST_OPEN_PREFIX: &str = otb_core::recent_scheduled_gates::TASKLIST_OPEN_PREFIX;
pub const TASKLIST_DELETE_PREFIX: &str = otb_core::recent_scheduled_gates::TASKLIST_DELETE_PREFIX;
pub const TASKLIST_TOGGLE_PREFIX: &str = otb_core::recent_scheduled_gates::TASKLIST_TOGGLE_PREFIX;
pub const TASKLIST_CANCEL_CALLBACK: &str =
    otb_core::recent_scheduled_gates::TASKLIST_CANCEL_CALLBACK;

/// Node `compact:*` callback shapes (parity with
/// `src/bot/menus/context-control-menu.ts` +
/// `context-control-callback-handler.ts`). Values reuse
/// `otb_core::context_inline_gates` so the shapes cannot drift.
pub const COMPACT_DETAILS_CALLBACK: &str = otb_core::context_inline_gates::COMPACT_DETAILS_CALLBACK;
pub const COMPACT_CONFIRM_CALLBACK: &str = otb_core::context_inline_gates::COMPACT_CONFIRM_CALLBACK;

/// Node `inline:cancel:*` + legacy `compact:cancel` shapes (parity with
/// `src/bot/menus/inline-menu.ts` +
/// `inline-menu-cancel-callback-handler.ts`). Values reuse
/// `otb_core::inline_menu_gates` so the shapes cannot drift.
pub const INLINE_MENU_CANCEL_PREFIX: &str = otb_core::inline_menu_gates::INLINE_MENU_CANCEL_PREFIX;
pub const LEGACY_CONTEXT_CANCEL_CALLBACK: &str =
    otb_core::inline_menu_gates::LEGACY_CONTEXT_CANCEL_CALLBACK;

/// Node `rename:cancel` shape (parity with `src/bot/menus/rename-menu.ts`
/// + `rename-callback-handler.ts`). Value reuses
/// `otb_core::rename_callback_gates` so the shape cannot drift.
pub const RENAME_CANCEL_CALLBACK: &str = otb_core::rename_callback_gates::RENAME_CANCEL_CALLBACK;

/// `true` for any `settings:*` payload (Node `startsWith(SETTINGS_CALLBACK_PREFIX)`
/// guard in `settings-callback-handler.ts`).
pub fn is_settings_callback(data: &str) -> bool {
    otb_core::settings_callback_gates::handles_settings_callback(Some(data))
}

/// `true` for the exact `task:retry-schedule` payload (Node
/// `data === TASK_RETRY_SCHEDULE_CALLBACK` branch).
pub fn is_task_retry_schedule_callback(data: &str) -> bool {
    data == TASK_RETRY_SCHEDULE_CALLBACK
}

/// `true` for the exact `task:cancel` payload (Node
/// `data === TASK_CANCEL_CALLBACK` branch).
pub fn is_task_cancel_callback(data: &str) -> bool {
    data == TASK_CANCEL_CALLBACK
}

/// `true` for any `tasklist:*` payload (Node
/// `startsWith(TASKLIST_CALLBACK_PREFIX)` guard in
/// `handleTaskListCallback`).
pub fn is_tasklist_callback(data: &str) -> bool {
    otb_core::task_callback_gates::handles_tasklist_callback(Some(data))
}

/// Id behind a `tasklist:open:<id>` callback; `None` for a wrong prefix
/// (Node open-slice parity; the store lookup rejects unknown ids).
pub fn parse_tasklist_open_callback(data: &str) -> Option<String> {
    otb_core::recent_scheduled_gates::parse_tasklist_open_callback(data)
}

/// Id behind a `tasklist:delete:<id>` callback; `None` for a wrong prefix
/// (Node delete-slice parity).
pub fn parse_tasklist_delete_callback(data: &str) -> Option<String> {
    otb_core::recent_scheduled_gates::parse_tasklist_delete_callback(data)
}

/// Action + id behind a `tasklist:toggle:<on|off>:<id>` callback; `None`
/// for a wrong prefix, a non-`on`/`off` action, or an empty id (Node
/// toggle-gate parity).
pub fn parse_tasklist_toggle_callback(
    data: &str,
) -> Option<otb_core::recent_scheduled_gates::TasklistToggle> {
    otb_core::recent_scheduled_gates::parse_tasklist_toggle_callback(data)
}

/// `true` for the exact `tasklist:cancel` payload (Node
/// `data === TASKLIST_CANCEL_CALLBACK` branch).
pub fn is_tasklist_cancel_callback(data: &str) -> bool {
    otb_core::recent_scheduled_gates::is_tasklist_cancel_callback(data)
}

/// Exact `compact:details` match (Node `data !== "compact:details"`
/// return-false guard in `handleCompactDetails`).
pub fn is_compact_details_callback(data: &str) -> bool {
    data == COMPACT_DETAILS_CALLBACK
}

/// Exact `compact:confirm` match (Node `data !== "compact:confirm"`
/// return-false guard in `handleCompactConfirm`).
pub fn is_compact_confirm_callback(data: &str) -> bool {
    data == COMPACT_CONFIRM_CALLBACK
}

/// Resolved menu kind behind an inline-menu cancel payload: the legacy
/// `compact:cancel` reads as `context`, `inline:cancel:<kind>` reads as
/// that kind when known, anything else yields `None` (Node
/// `handleInlineMenuCancel` parity).
pub fn parse_inline_cancel_callback(data: &str) -> Option<&str> {
    otb_core::inline_menu_gates::resolve_inline_cancel_kind(Some(data))
}

/// Exact `rename:cancel` match (Node `data !== RENAME_CANCEL_CALLBACK`
/// return-false guard in `handleRenameCancel`).
pub fn is_rename_cancel_callback(data: &str) -> bool {
    otb_core::rename_callback_gates::handles_rename_cancel_callback(Some(data))
}

/// Node `question:*` / `permission:*` callback prefixes (parity with
/// `src/bot/menus/question-menu.ts` builders and
/// `src/bot/menus/permission-menu.ts` buttons). Values reuse
/// `otb_core::question_permission_inline_gates` so the shapes cannot
/// drift.
pub const QUESTION_CALLBACK_PREFIX: &str =
    otb_core::question_permission_inline_gates::QUESTION_CALLBACK_PREFIX;
pub const PERMISSION_CALLBACK_PREFIX: &str =
    otb_core::question_permission_inline_gates::PERMISSION_CALLBACK_PREFIX;
pub const PERMISSION_CALLBACK_ONCE: &str =
    otb_core::question_permission_inline_gates::PERMISSION_CALLBACK_ONCE;
pub const PERMISSION_CALLBACK_ALWAYS: &str =
    otb_core::question_permission_inline_gates::PERMISSION_CALLBACK_ALWAYS;
pub const PERMISSION_CALLBACK_REJECT: &str =
    otb_core::question_permission_inline_gates::PERMISSION_CALLBACK_REJECT;

/// Poll-card text from the pump's prepped [`crate::attach::PollCardData`]
/// (pure; testable).
///
/// Node `formatQuestionDetailsPart` fallback-text parity
/// (`src/bot/menus/question-menu.ts`): one `❓ n/total header` line per
/// prepped header, joined with `\n\n` through
/// `question_fallback_text`, then markdown-normalized and capped at
/// [`TELEGRAM_TEXT_LIMIT`].
///
/// SUBSET: the prep carries headers only (no per-question options,
/// `multiple`/`custom` flags, or selection state), so the card lists the
/// poll's questions without option rows or the multi-select hint. Those
/// need the dispatcher `questionManager` and stay a follow-up. The
/// `_locale` param is kept for that follow-up (status lines and the
/// multi-select hint are localized in TS); today's segments carry agent
/// text only.
pub fn build_poll_card_text(card: &crate::attach::PollCardData, _locale: &str) -> String {
    use otb_core::question_permission_inline_gates as gates;
    let total = card.question_count.max(card.headers.len()).max(1);
    let mut segments: Vec<gates::QuestionSegment> = card
        .headers
        .iter()
        .enumerate()
        .filter(|(_, header)| !header.trim().is_empty())
        .map(|(index, header)| {
            let progress = gates::question_progress_text(index as u64, total as u64);
            let title = if progress.is_empty() {
                format!("{} {header}", gates::QUESTION_EMOJI)
            } else {
                format!("{} {progress} {header}", gates::QUESTION_EMOJI)
            };
            gates::QuestionSegment {
                label: Some(title),
                rest: String::new(),
            }
        })
        .collect();
    if segments.is_empty() {
        segments.push(gates::QuestionSegment {
            label: Some(gates::QUESTION_EMOJI.to_string()),
            rest: String::new(),
        });
    }
    let visible = gates::truncate_question_segments(&segments, gates::QUESTION_MESSAGE_LIMIT);
    let fallback = gates::question_fallback_text(&visible);
    let normalized =
        otb_core::markdown_normalizer::normalize_markdown_for_telegram_rendering(&fallback);
    truncate_text(&normalized, TELEGRAM_TEXT_LIMIT).to_string()
}

/// Poll-card keyboard (pure; testable): custom-answer + cancel rows with
/// the `question:*` callbacks and localized labels.
///
/// SUBSET: per-option select rows and the submit row need the live option
/// list plus selection state (`questionManager`), so they stay a
/// dispatcher follow-up. Taps the scaffold already answers: cancel rejects
/// the held request, a custom answer arrives as plain text while the slot
/// is held, anything else only acks the spinner.
pub fn build_poll_card_keyboard(locale: &str) -> InlineKeyboardMarkup {
    use otb_core::question_permission_inline_gates as gates;
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "question.button.custom"),
            gates::build_question_custom_callback(0),
        )],
        vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "question.button.cancel"),
            gates::build_question_cancel_callback(0),
        )],
    ])
}

/// Localized display name for a permission kind (pure; testable): the
/// `permission.name.*` key, falling back to the raw id for unknown kinds
/// (Node `PERMISSION_NAME_KEYS` lookup parity in `permission-menu.ts`).
pub fn permission_kind_display_name(permission: &str, locale: &str) -> String {
    let key = format!("permission.name.{permission}");
    let translated = crate::locale::translate(locale, &key);
    if translated == key {
        permission.to_string()
    } else {
        translated
    }
}

/// Permission-card text from the pump's prepped
/// [`crate::attach::PermissionCardData`] (pure; testable).
///
/// Node `formatPermissionText` header parity
/// (`src/bot/menus/permission-menu.ts`): emoji (`🔐` fallback) plus the
/// localized kind name through the `permission.header` template, with the
/// `permission.grouped_count` line when the prompt answers more than one
/// pending request. Then markdown-normalized and capped at
/// [`TELEGRAM_TEXT_LIMIT`].
///
/// SUBSET: the prep carries the pattern count only, not the patterns
/// themselves, so the `• pattern` list is omitted. Carrying the patterns
/// (V1 `properties.patterns` / V2 `data.resources`) stays a follow-up.
pub fn build_permission_card_text(
    card: &crate::attach::PermissionCardData,
    locale: &str,
) -> String {
    use otb_core::question_permission_inline_gates as gates;
    let emoji = gates::permission_emoji_for(&card.permission);
    let name = permission_kind_display_name(&card.permission, locale);
    let header = crate::locale::translate(locale, "permission.header")
        .replace("{emoji}", emoji)
        .replace("{name}", &name);
    let grouped = if card.request_ids.len() > 1 {
        crate::locale::translate(locale, "permission.grouped_count")
            .replace("{count}", &card.request_ids.len().to_string())
    } else {
        String::new()
    };
    let normalized = otb_core::markdown_normalizer::normalize_markdown_for_telegram_rendering(
        &format!("{header}{grouped}"),
    );
    truncate_text(&normalized, TELEGRAM_TEXT_LIMIT).to_string()
}

/// Permission-card keyboard (pure; testable): allow-once / allow-always /
/// reject rows with the exact `permission:*` callbacks and localized
/// labels (Node `buildPermissionKeyboard` parity in
/// `permission-menu.ts`).
pub fn build_permission_card_keyboard(locale: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "permission.button.allow"),
            PERMISSION_CALLBACK_ONCE.to_string(),
        )],
        vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "permission.button.always"),
            PERMISSION_CALLBACK_ALWAYS.to_string(),
        )],
        vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "permission.button.reject"),
            PERMISSION_CALLBACK_REJECT.to_string(),
        )],
    ])
}

/// Native rich-message budget for one attach reply chunk (TS Bot API 10.1
/// parity: native messages hold 32768 characters instead of 4096, so a
/// reply is split rarely; bot-composed quote/waiting/empty lines stay at
/// [`TELEGRAM_TEXT_LIMIT`]).
pub const ATTACH_REPLY_CHUNK_LIMIT: usize = 32_768;

/// Read one raw `session.messages` entry into the session-pick shape the
/// [`otb_core::session_pick_catalog_gates`] pickers inspect (pure;
/// testable).
///
/// TS `SessionPickMessage` parity (`session-pick-reply.ts`): `info.id`,
/// `info.role`, `info.summary`, `info.parentID`, `info.error` truthiness,
/// `info.time.{created,completed}`, and `parts[]` with `type` plus
/// `text?` / `filename?` (a `name` field is accepted as a filename
/// fallback). Entries without an `info` object yield `None`; untyped
/// parts are skipped.
pub fn session_pick_message_from_value(
    value: &serde_json::Value,
) -> Option<otb_core::session_pick_catalog_gates::SessionPickMessage> {
    use otb_core::session_pick_catalog_gates as pick;
    let info = value.get("info")?;
    if !info.is_object() {
        return None;
    }
    let text_field = |key: &str| {
        info.get(key)
            .and_then(|field| field.as_str())
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .map(str::to_string)
    };
    let mut parts = Vec::new();
    if let Some(raw_parts) = value.get("parts").and_then(|parts| parts.as_array()) {
        for part in raw_parts {
            let kind = part
                .get("type")
                .and_then(|kind| kind.as_str())
                .unwrap_or("")
                .to_string();
            if kind.is_empty() {
                continue;
            }
            parts.push(pick::SessionPickPart {
                kind,
                text: part
                    .get("text")
                    .and_then(|text| text.as_str())
                    .map(str::to_string),
                filename: part
                    .get("filename")
                    .or_else(|| part.get("name"))
                    .and_then(|name| name.as_str())
                    .map(str::to_string),
            });
        }
    }
    Some(pick::SessionPickMessage {
        info: pick::SessionPickInfo {
            id: text_field("id"),
            role: info
                .get("role")
                .and_then(|role| role.as_str())
                .map(str::to_string),
            summary: info
                .get("summary")
                .and_then(|summary| summary.as_bool())
                .unwrap_or(false),
            parent_id: text_field("parentID"),
            has_error: info
                .get("error")
                .is_some_and(|error| !error.is_null() && error != &serde_json::Value::Bool(false)),
            created: info
                .get("time")
                .and_then(|time| time.get("created"))
                .and_then(|created| created.as_i64()),
            completed: info
                .get("time")
                .and_then(|time| time.get("completed"))
                .and_then(|completed| completed.as_i64()),
        },
        parts,
    })
}

/// Whether the session counts as busy in a raw `session.status` body
/// (pure; testable).
///
/// TS `readSessionBusy` parity (`session-callback-handler.ts`):
/// `data[sessionId]?.type === "busy"`. A bare `"busy"` entry and an id
/// list under `active`/`busy`/`sessions` (or a bare id array) are
/// tolerated for version-shape drift; anything else reads as idle.
pub fn attach_session_is_busy(status: &serde_json::Value, session_id: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    if let Some(entry) = status.get(session_id) {
        if entry.get("type").and_then(|kind| kind.as_str()) == Some("busy") {
            return true;
        }
        if entry.as_str() == Some("busy") {
            return true;
        }
    }
    for key in ["active", "busy", "sessions"] {
        if let Some(ids) = status.get(key).and_then(|ids| ids.as_array()) {
            if ids.iter().any(|id| id.as_str() == Some(session_id)) {
                return true;
            }
        }
    }
    if let Some(ids) = status.as_array() {
        if ids.iter().any(|id| id.as_str() == Some(session_id)) {
            return true;
        }
    }
    false
}

/// The shown reply's parent input when the loaded transcript no longer
/// carries it (pure; testable). TS `sendSessionPickTranscript` parity:
/// the reply's `parentId` is reloaded (`loadParentMessage`) only when no
/// loaded message has that id; otherwise there is nothing to fetch.
pub fn missing_attach_parent_id(
    messages: &[otb_core::session_pick_catalog_gates::SessionPickMessage],
    busy: bool,
) -> Option<String> {
    use otb_core::session_pick_catalog_gates as pick;
    let reply = pick::find_eligible_reply(messages, busy)?;
    let parent = reply.parent_id.filter(|id| !id.is_empty())?;
    if messages
        .iter()
        .any(|message| message.info.id.as_deref() == Some(parent.as_str()))
    {
        return None;
    }
    Some(parent)
}

/// Build the quote text for one picked user input (pure; testable).
///
/// TS `sendPickQuote` parity (`session-callback-handler.ts` plus
/// `session-pick-reply.ts` `buildSessionPickQuote` over
/// `buildQuotedNotification`): the localized `👤` title over the input
/// text (or its `📎` file lines), CRLF-normalized and truncated like
/// external input, rendered as a `>` blockquote. The scaffold sends
/// plain text (no MarkdownV2 parse-mode transport), so the normalized
/// raw fallback is used; native blockquote nesting survives through the
/// `>` prefix. Capped at [`TELEGRAM_TEXT_LIMIT`]; `None` when the
/// message carries no quotable text.
pub fn build_attach_quote_text(
    message: &otb_core::session_pick_catalog_gates::SessionPickMessage,
    locale: &str,
) -> Option<String> {
    use otb_core::session_pick_catalog_gates as pick;
    let title = crate::locale::translate(locale, "sessions.last_input.title");
    let attachment = crate::locale::translate(locale, "sessions.last_input.attachment");
    let quote = pick::build_session_pick_quote(message, &title, &attachment)?;
    let notice =
        otb_core::outage_ready_gates::build_quoted_notification(&quote.title, &quote.body, None);
    let normalized = otb_core::markdown_normalizer::normalize_markdown_for_telegram_rendering(
        &notice.raw_fallback_text,
    );
    Some(truncate_text(&normalized, TELEGRAM_TEXT_LIMIT).to_string())
}

/// Render one assistant reply into sendable native-budget chunks (pure;
/// testable).
///
/// TS `renderAssistantFinalPartsSafe` parity
/// (`assistant-rendering.ts`): markdown-normalized (headings, rules,
/// checklists, quote continuations) and split rarely — one chunk unless
/// the reply exceeds [`ATTACH_REPLY_CHUNK_LIMIT`] — splitting cleanly on
/// line breaks, then whitespace, then char boundaries (the
/// `splitTextIntoChunks` contract reused from
/// [`otb_core::render::chunker::chunk_plain_text`]). Blank text yields
/// no chunks. The native rich-blocks transport stays a dispatcher
/// follow-up; these chunks carry the normalized fallback text.
pub fn build_attach_reply_chunks(text: &str) -> Vec<String> {
    use otb_core::render::chunker::{self, PlainChunkerOptions};
    let normalized = otb_core::markdown_normalizer::normalize_markdown_for_telegram_rendering(text);
    if normalized.trim().is_empty() {
        return Vec::new();
    }
    chunker::chunk_plain_text(
        &normalized,
        Some(PlainChunkerOptions {
            max_chars: Some(ATTACH_REPLY_CHUNK_LIMIT),
        }),
    )
    .into_iter()
    .map(|part| part.fallback_text)
    .filter(|chunk| !chunk.is_empty())
    .collect()
}

/// Build the empty-transcript notice (pure; testable). TS
/// `sendSessionPickTranscript` parity: `sessions.preview.empty` when the
/// transcript has neither quote nor reply. Capped at
/// [`TELEGRAM_TEXT_LIMIT`].
pub fn build_attach_empty_text(locale: &str) -> String {
    truncate_text(
        &crate::locale::translate(locale, "sessions.preview.empty"),
        TELEGRAM_TEXT_LIMIT,
    )
    .to_string()
}

/// The attach transcript: last user input as quote, last assistant reply
/// in full, plus the newest still-waiting input (pure; testable).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AttachTranscript {
    pub quote: Option<String>,
    pub reply_chunks: Vec<String>,
    pub waiting: Option<String>,
}

/// Build the attach transcript from loaded session messages (pure;
/// testable).
///
/// TS `sendSessionPickTranscript` parity
/// (`session-callback-handler.ts`): the eligible reply's parent input is
/// the quote (falling back to the newest user prompt only when there is
/// no reply — the single-message reload itself stays with the caller via
/// [`missing_attach_parent_id`]); the reply renders in full; a user
/// input newer than the shown reply is quoted again as the waiting line.
pub fn build_attach_transcript(
    messages: &[otb_core::session_pick_catalog_gates::SessionPickMessage],
    busy: bool,
    locale: &str,
) -> AttachTranscript {
    use otb_core::session_pick_catalog_gates as pick;
    let attachment = crate::locale::translate(locale, "sessions.last_input.attachment");
    let reply = pick::find_eligible_reply(messages, busy);
    let quote_source = match &reply {
        Some(reply) => reply
            .parent_id
            .as_deref()
            .and_then(|id| pick::find_message_by_id(messages, id)),
        None => pick::find_latest_user_prompt(messages, &attachment),
    };
    let quote = quote_source.and_then(|message| build_attach_quote_text(message, locale));
    let reply_chunks = reply
        .as_ref()
        .map(|reply| build_attach_reply_chunks(&reply.text))
        .unwrap_or_default();
    let waiting = match &reply {
        Some(reply) => pick::find_pending_user_prompt(messages, reply.created, &attachment)
            .and_then(|message| build_attach_quote_text(message, locale)),
        None => None,
    };
    AttachTranscript {
        quote,
        reply_chunks,
        waiting,
    }
}

/// Whether the transcript degrades to the empty notice (pure; testable).
/// TS gate parity: `!reply && !quote` sends `sessions.preview.empty`
/// (via [`otb_core::session_select_callback_gates`]).
pub fn should_send_attach_empty(transcript: &AttachTranscript) -> bool {
    otb_core::session_select_callback_gates::should_send_empty_session_preview(
        !transcript.reply_chunks.is_empty(),
        transcript.quote.is_some(),
    )
}

/// Whether a waiting line follows the reply (pure; testable). TS gate
/// parity: after the reply parts, a pending newer-than-reply prompt plus
/// its quote must both exist (via
/// [`otb_core::session_select_callback_gates`]).
pub fn should_show_attach_waiting(transcript: &AttachTranscript) -> bool {
    otb_core::session_select_callback_gates::should_show_pending_prompt(
        !transcript.reply_chunks.is_empty(),
        transcript.waiting.is_some(),
    )
}

/// Send bot-composed text with no link preview (TS
/// `withBotLinkPreviewDefault` parity in `src/bot/index.ts`: only agent
/// text keeps the preview). Text is capped at [`TELEGRAM_TEXT_LIMIT`]
/// via [`build_send_payload`]; failures only trace.
async fn send_bot_composed_text(bot: &teloxide_core::Bot, chat_id: i64, text: &str) {
    let (chat, body) = build_send_payload(chat_id, text);
    let result = bot
        .send_message(chat, body)
        .link_preview_options(LinkPreviewOptions {
            is_disabled: true,
            url: None,
            prefer_small_media: false,
            prefer_large_media: false,
            show_above_text: false,
        })
        .await;
    if let Err(err) = result {
        tracing::warn!("[otb-bot] telegram: composed send failed for chat {chat_id}: {err}");
    }
}

/// Send agent text keeping Telegram's link preview (TS
/// `AGENT_TEXT_LINK_PREVIEW_OPTIONS` parity in
/// `src/bot/messages/telegram-text.ts`). No 4096 truncation: callers
/// pass native-budget chunks (see [`ATTACH_REPLY_CHUNK_LIMIT`]); the
/// native rich transport stays a dispatcher follow-up.
async fn send_agent_text(bot: &teloxide_core::Bot, chat_id: i64, text: &str) {
    if text.is_empty() {
        return;
    }
    let result = bot
        .send_message(ChatId(chat_id), text.to_string())
        .link_preview_options(LinkPreviewOptions {
            is_disabled: false,
            url: None,
            prefer_small_media: false,
            prefer_large_media: false,
            show_above_text: false,
        })
        .await;
    if let Err(err) = result {
        tracing::warn!("[otb-bot] telegram: agent send failed for chat {chat_id}: {err}");
    }
}

/// Live attach transcript (TS `sendSessionPickTranscript` parity in
/// `session-callback-handler.ts`): last user input as quote, last
/// assistant reply in full, plus the newest still-waiting input.
///
/// Order and preview flags mirror TS: quote (no preview), reply chunks
/// (preview kept), waiting quote (no preview); neither quote nor reply
/// degrades to `sessions.preview.empty` (no preview). Busy comes from
/// `session.status` — a failed read assumes busy like TS
/// `readSessionBusy` (a partial run is never shown as the full reply).
/// A shown reply whose parent input fell out of the loaded transcript is
/// reloaded once by ID. Every failure stays inside `tracing`; the attach
/// itself never fails by construction.
async fn send_attach_transcript(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    session_id: &str,
    directory: &str,
    locale: &str,
) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let busy = match client.session_status(directory).await {
        Ok(status) => attach_session_is_busy(&status, session_id),
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: attach status read failed: {err}");
            true
        }
    };
    let raw = match client.session_messages(session_id, directory).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: attach transcript list failed: {err}");
            send_bot_composed_text(bot, chat_id, &build_attach_empty_text(locale)).await;
            return;
        }
    };
    let mut messages: Vec<otb_core::session_pick_catalog_gates::SessionPickMessage> = raw
        .iter()
        .filter_map(session_pick_message_from_value)
        .collect();
    if let Some(parent_id) = missing_attach_parent_id(&messages, busy) {
        match client
            .session_message(session_id, &parent_id, directory)
            .await
        {
            Ok(value) => {
                if let Some(message) = session_pick_message_from_value(&value) {
                    messages.push(message);
                }
            }
            Err(err) => {
                tracing::warn!("[otb-bot] telegram: attach parent reload failed: {err}");
            }
        }
    }
    let transcript = build_attach_transcript(&messages, busy, locale);
    if should_send_attach_empty(&transcript) {
        send_bot_composed_text(bot, chat_id, &build_attach_empty_text(locale)).await;
        return;
    }
    if let Some(quote) = transcript.quote.as_deref() {
        send_bot_composed_text(bot, chat_id, quote).await;
    }
    for chunk in &transcript.reply_chunks {
        send_agent_text(bot, chat_id, chunk).await;
    }
    if should_show_attach_waiting(&transcript) {
        if let Some(waiting) = transcript.waiting.as_deref() {
            send_bot_composed_text(bot, chat_id, waiting).await;
        }
    }
    tracing::info!("[otb-bot] telegram: attach transcript sent");
}

/// Send a card with an inline keyboard and no link preview (bot-composed
/// prompt parity with TS `withBotLinkPreviewDefault` in
/// `src/bot/index.ts`: only agent text keeps the preview). Text is capped
/// at [`TELEGRAM_TEXT_LIMIT`] via [`build_send_payload`].
async fn send_card_with_keyboard(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    text: &str,
    keyboard: InlineKeyboardMarkup,
) -> Result<(), teloxide_core::RequestError> {
    let (chat, body) = build_send_payload(chat_id, text);
    bot.send_message(chat, body)
        .reply_markup(keyboard)
        .link_preview_options(LinkPreviewOptions {
            is_disabled: true,
            url: None,
            prefer_small_media: false,
            prefer_large_media: false,
            show_above_text: false,
        })
        .await?;
    Ok(())
}

/// Send one poll card for a prepped question ask (live path only; never
/// called from tests). Failures stay inside `tracing`: the slot hold is
/// the prompt and a retry tap still lands.
async fn send_poll_card(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    card: &crate::attach::PollCardData,
    locale: &str,
) {
    let text = build_poll_card_text(card, locale);
    let keyboard = build_poll_card_keyboard(locale);
    if let Err(err) = send_card_with_keyboard(bot, chat_id, &text, keyboard).await {
        tracing::warn!("[otb-bot] telegram: poll card send failed for chat {chat_id}: {err}");
    }
}

/// Send one permission card for a prepped permission ask (live path only;
/// never called from tests). Failures stay inside `tracing`: the slot
/// hold is the prompt and a retry tap still lands.
async fn send_permission_card(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    card: &crate::attach::PermissionCardData,
    locale: &str,
) {
    let text = build_permission_card_text(card, locale);
    let keyboard = build_permission_card_keyboard(locale);
    if let Err(err) = send_card_with_keyboard(bot, chat_id, &text, keyboard).await {
        tracing::warn!("[otb-bot] telegram: permission card send failed for chat {chat_id}: {err}");
    }
}

/// Install the telegram-ux poll/permission card hooks behind the pump's
/// [`crate::attach::set_poll_card_hook`] /
/// [`crate::attach::set_permission_card_hook`] wiring (live path only).
///
/// Each hook clones the bot plus the target chat and locale, then spawns
/// the send: the pump hook is sync while sending is async. A zero
/// `chat_id` (the deny-all sentinel, no DM target) installs nothing.
///
/// ASSUMPTION: single-user bot, so the card target is the allowed user's
/// DM (`chat_id == allowed user id`). The TS destination policy
/// (`policy.getDestination`) has no scaffold equivalent; a dispatcher
/// with per-session chat routing replaces this.
pub fn install_card_hooks(bot: &teloxide_core::Bot, chat_id: i64, locale: &str) {
    if chat_id == 0 {
        tracing::debug!("[otb-bot] telegram: card hooks not installed (no chat target)");
        return;
    }
    let poll_bot = bot.clone();
    let poll_locale = locale.to_string();
    crate::attach::set_poll_card_hook(Some(Box::new(move |card| {
        let bot = poll_bot.clone();
        let locale = poll_locale.clone();
        tokio::spawn(async move {
            send_poll_card(&bot, chat_id, &card, &locale).await;
        });
    })));
    let perm_bot = bot.clone();
    let perm_locale = locale.to_string();
    crate::attach::set_permission_card_hook(Some(Box::new(move |card| {
        let bot = perm_bot.clone();
        let locale = perm_locale.clone();
        tokio::spawn(async move {
            send_permission_card(&bot, chat_id, &card, &locale).await;
        });
    })));
    tracing::info!("[otb-bot] telegram: poll/permission card hooks installed");
}

/// Current wall-clock time in ms since the Unix epoch for footer `elapsed`
/// math (fail-open `0` when the clock reads before the epoch).
fn live_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Install the telegram-ux idle-footer sender behind the pump's
/// [`crate::attach::set_session_idle_footer_hook`] wiring (live path only).
///
/// Each hook fire spawns the send: the pump hook is sync while sending is
/// async. A zero `chat_id` (the deny-all sentinel, no DM target) installs
/// nothing. Completed-run hydration (agent/model/steps plus the suppressed
/// and interrupted-without-response flags) stays a dispatcher follow-up, so
/// `completed` is `None` and every idle stays footerless until that lands;
/// the live call chain (idle signal -> idle gate -> run-close tail ->
/// [`send_footer_line`]) is what this wires. `show_footer` mirrors the TS
/// `getShowAssistantRunFooter` setting read at install time.
pub fn install_session_idle_footer_hook(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    locale: &str,
    show_footer: bool,
) {
    if chat_id == 0 {
        tracing::debug!("[otb-bot] telegram: idle footer hook not installed (no chat target)");
        return;
    }
    let footer_bot = bot.clone();
    let footer_locale = locale.to_string();
    crate::attach::set_session_idle_footer_hook(Some(Box::new(move |signal| {
        let bot = footer_bot.clone();
        let locale = footer_locale.clone();
        tokio::spawn(async move {
            send_session_idle_footer(
                &bot,
                chat_id,
                show_footer,
                None,
                live_now_ms(),
                &locale,
                signal.shutdown_idle,
                false,
                false,
            )
            .await;
        });
    })));
    tracing::info!("[otb-bot] telegram: session-idle footer hook installed");
}

/// Streaming footers / pinned dashboard / compact progress (telegram-ux
/// slice 0.71: pure render plus the live `session.idle` footer hook; the
/// dispatcher still owns the completed-run snapshot).
///
/// TS parity:
/// - `src/app/formatters/assistant-run-footer-formatter.ts`
///   (`assistantRunTokensPerSecond`, `formatAssistantRunFooter`; PRODUCT.md
///   #82 — every answered turn ends with its own footer, none on abort /
///   error / reject / cancel / restart / stop)
/// - `src/bot/pinned/pinned-message-manager.ts` (`formatMessage`: title,
///   goal, project, worktree, model, context, cost, files; PRODUCT.md
///   #102-106 — live dashboard, empty diff removes the list, failed
///   refresh keeps the last one by not calling the renderer)
/// - `src/bot/streaming/compact-progress-streamer.ts` + `formatCompactSummary`
///   (PRODUCT.md #88 — one `⏳ Working` line per stretch, `✅ ...` count
///   line when it closes, removed when there is nothing to count)
/// - `src/bot/streaming/stream-throttle.ts`
///   (`resolveProgressiveThrottleMs`; PRODUCT.md #96 — 1s edits the first
///   minute, then 2s / 5s / 10s; the interval resets when the run stops)
///
/// Preview policy (PRODUCT.md result-delivery rule): every send here goes
/// through [`send_bot_composed_text`] (no link preview on first send);
/// dispatcher edits of the same messages must reuse
/// [`bot_composed_preview_options`] (no preview on edit either). Agent
/// replies keep the preview via `send_agent_text` and are untouched.
/// Link-preview options shared by bot-composed streaming surfaces
/// (footer, pinned dashboard, compact progress): disabled, URLs stay
/// clickable. Edits (dispatcher follow-up) must pass the same value.
pub fn bot_composed_preview_options() -> LinkPreviewOptions {
    LinkPreviewOptions {
        is_disabled: true,
        url: None,
        prefer_small_media: false,
        prefer_large_media: false,
        show_above_text: false,
    }
}

/// Substitute one `{name}` placeholder in an i18n template (pure).
fn fill_stream_template(template: &str, name: &str, value: &str) -> String {
    template.replace(&format!("{{{name}}}"), value)
}

/// Whether an answered turn gets its footer line (PRODUCT.md #82).
/// Aborted / errored turns (`completed_ok == false`), rejected
/// permissions, cancelled polls, restarts and stops get none; a disabled
/// footer setting suppresses the rest.
pub fn should_append_assistant_footer(show_footer: bool, completed_ok: bool) -> bool {
    show_footer && completed_ok
}

/// Build the assistant-run footer line for one answered turn
/// (`agent · 🧠 provider/model · 🕒 elapsed [· ⚡ x.x tok/s]
/// [· ⏹ interrupted]`). `steps` feeds OpenCode's speed formula (each step
/// weighted by its own stream span; `None` until every step streamed).
/// Pure; the caller decides delivery via [`should_append_assistant_footer`].
pub fn build_assistant_footer(
    agent: &str,
    provider_id: &str,
    model_id: &str,
    elapsed_ms: i64,
    steps: &[otb_core::duration_footer_display_gates::AssistantRunStep],
    interrupted: bool,
    locale: &str,
) -> String {
    let agent_display = otb_core::types_config::get_agent_display_name(agent);
    let speed = otb_core::duration_footer_display_gates::assistant_run_tokens_per_second(steps);
    let interrupted_label = crate::locale::translate(locale, "assistant.footer.interrupted");
    otb_core::duration_footer_display_gates::format_assistant_run_footer(
        &agent_display,
        provider_id,
        model_id,
        elapsed_ms,
        speed,
        interrupted,
        &interrupted_label,
    )
}

/// Completed-run snapshot for one closing foreground run (TS
/// `AssistantRunInfo` footer subset in
/// `src/bot/events/handlers/run-close.ts:1-11`). The dispatcher owns the
/// tool flush and the compact-progress finalize; this struct carries only
/// what the footer needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCloseCompletedRun {
    /// TS `hasCompletedResponse`: a reply landed before the close.
    pub has_completed_response: bool,
    /// TS `suppressFooter`: a permission reject or a cancelled question
    /// marked the turn footerless before the close.
    pub suppress_footer: bool,
    /// TS `configuredAgent` / `actualAgent` (actual wins when set).
    pub configured_agent: Option<String>,
    /// TS `configuredProviderID` / `actualProviderID` (actual wins).
    pub configured_provider_id: Option<String>,
    /// TS `configuredModelID` / `actualModelID` (actual wins).
    pub configured_model_id: Option<String>,
    pub actual_agent: Option<String>,
    pub actual_provider_id: Option<String>,
    pub actual_model_id: Option<String>,
    /// TS `startedAt` (ms epoch) for the `🕒 elapsed` part.
    pub started_at_ms: i64,
    /// TS `steps` for the `⚡ x.x tok/s` part.
    pub steps: Vec<otb_core::duration_footer_display_gates::AssistantRunStep>,
    /// TS `interrupted`: a client stop after the reply completed.
    pub interrupted: bool,
}

/// Pick the effective run identity (`actual || configured`, trimmed).
/// Empty identities read as missing (TS `run-close.ts:39-44` early return).
fn run_close_identity(run: &RunCloseCompletedRun) -> Option<(String, String, String)> {
    let pick = |actual: &Option<String>, configured: &Option<String>| {
        actual
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                configured
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .map(str::to_string)
    };
    let agent = pick(&run.actual_agent, &run.configured_agent)?;
    let provider = pick(&run.actual_provider_id, &run.configured_provider_id)?;
    let model = pick(&run.actual_model_id, &run.configured_model_id)?;
    Some((agent, provider, model))
}

/// Resolve the run-close footer for one finished turn (pure; testable).
/// TS `run-close.ts:31-44` parity: no footer when the setting is off, when
/// no reply completed (abort / error / restart / stop / scheduled error),
/// when the turn is suppressed (rejected permission / cancelled question),
/// or when the agent/model identity is incomplete. Otherwise builds via
/// [`build_assistant_footer`] with `elapsed = now_ms - started_at_ms`.
/// Scheduled-task success shares this shape with `interrupted == false`;
/// a scheduled error has no completed run, so it reads as `None`.
pub fn resolve_run_close_footer(
    show_footer: bool,
    completed: Option<&RunCloseCompletedRun>,
    now_ms: i64,
    locale: &str,
) -> Option<String> {
    let run = completed?;
    if !should_append_assistant_footer(show_footer, run.has_completed_response) {
        return None;
    }
    if run.suppress_footer {
        return None;
    }
    let (agent, provider, model) = run_close_identity(run)?;
    let elapsed_ms = now_ms.saturating_sub(run.started_at_ms).max(0);
    Some(build_assistant_footer(
        &agent,
        &provider,
        &model,
        elapsed_ms,
        &run.steps,
        run.interrupted,
        locale,
    ))
}

/// Resolve the `session.idle` footer (pure; testable). TS
/// `session-lifecycle-handler.ts:70-91` parity: a server-stop shutdown idle,
/// a suppressed turn, and an interrupted turn with no completed reply stay
/// footerless even when a snapshot is passed. The surviving path is the
/// [`resolve_run_close_footer`] gate above.
pub fn resolve_session_idle_footer(
    show_footer: bool,
    completed: Option<&RunCloseCompletedRun>,
    now_ms: i64,
    locale: &str,
    shutdown_idle: bool,
    footer_suppressed: bool,
    interrupted_without_response: bool,
) -> Option<String> {
    if shutdown_idle || footer_suppressed || interrupted_without_response {
        return None;
    }
    resolve_run_close_footer(show_footer, completed, now_ms, locale)
}

/// One changed file on the pinned dashboard (TS `FileChange` subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedChangedFile {
    pub file: String,
    pub additions: u64,
    pub deletions: u64,
}

/// Pinned dashboard values for one render (TS `formatMessage` inputs).
/// `tokens_limit` is `None` until the context limit loads; `cost` is
/// `None` until the first cost arrives; `changed_files` is OpenCode's
/// session diff (empty removes the list).
#[derive(Debug, Clone, PartialEq)]
pub struct PinnedDashboardData {
    pub session_title: String,
    pub goal_status: Option<String>,
    pub goal_objective: Option<String>,
    pub project_path: String,
    pub branch: Option<String>,
    pub worktree_path: Option<String>,
    pub is_linked_worktree: bool,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub variant: Option<String>,
    pub tokens_used: u64,
    pub tokens_limit: Option<u64>,
    pub cost: Option<f64>,
    pub changed_files: Vec<PinnedChangedFile>,
}

/// Whether the dashboard may draw (TS `should_render_dashboard` gate:
/// enabled AND a session with message state). Pure; the dispatcher owns
/// the send/pin/edit.
pub fn pinned_dashboard_visible(
    dashboard_enabled: bool,
    has_session: bool,
    has_message_state: bool,
) -> bool {
    otb_core::pinned_models::should_render_dashboard(
        dashboard_enabled,
        has_session,
        has_message_state,
    )
}

/// Render the pinned dashboard text (TS `formatMessage` parity).
/// A blank title falls back to `pinned.default_session_title`; the goal
/// line appears only when status and objective are both known; the
/// worktree line appears only for linked worktrees; unknown model/limit
/// use `pinned.unknown`; cost renders only when known (`$x.xx`); the file
/// section lists at most 10 items plus a `... and N more` overflow line.
/// Pure; i18n via `translate` + `{placeholder}` fill.
pub fn render_pinned_dashboard(data: &PinnedDashboardData, locale: &str) -> String {
    let t = |key: &str| crate::locale::translate(locale, key);
    let title = {
        let trimmed = data.session_title.trim();
        if trimmed.is_empty() {
            t("pinned.default_session_title")
        } else {
            trimmed.to_string()
        }
    };
    let mut lines = vec![title];
    let goal_status = data.goal_status.as_deref().map(str::trim).unwrap_or("");
    let goal_objective = data.goal_objective.as_deref().map(str::trim).unwrap_or("");
    if !goal_status.is_empty() && !goal_objective.is_empty() {
        let line = fill_stream_template(&t("pinned.line.goal"), "status", goal_status);
        lines.push(fill_stream_template(&line, "objective", goal_objective));
    }
    let project_display = match data.branch.as_deref().map(str::trim) {
        Some(branch) if !branch.is_empty() => format!("{}: {branch}", data.project_path),
        _ => data.project_path.clone(),
    };
    lines.push(fill_stream_template(
        &t("pinned.line.project"),
        "project",
        &project_display,
    ));
    if data.is_linked_worktree {
        if let Some(worktree) = data.worktree_path.as_deref().map(str::trim) {
            if !worktree.is_empty() {
                lines.push(fill_stream_template(
                    &t("pinned.line.worktree"),
                    "worktree",
                    worktree,
                ));
            }
        }
    }
    let model_name = match (
        data.provider_id.as_deref().map(str::trim),
        data.model_id.as_deref().map(str::trim),
    ) {
        (Some(provider), Some(model)) if !provider.is_empty() && !model.is_empty() => {
            let base = format!("{provider}/{model}");
            match data.variant.as_deref().map(str::trim) {
                Some(variant) if !variant.is_empty() => format!("{base} ({variant})"),
                _ => base,
            }
        }
        _ => t("pinned.unknown"),
    };
    lines.push(fill_stream_template(
        &t("pinned.line.model"),
        "model",
        &model_name,
    ));
    let used = otb_core::pinned_models::format_token_count(data.tokens_used);
    let limit = data
        .tokens_limit
        .filter(|limit| *limit > 0)
        .map(otb_core::pinned_models::format_token_count)
        .unwrap_or_else(|| t("pinned.unknown"));
    let percent = otb_core::pinned_models::context_percent(data.tokens_used, data.tokens_limit);
    let context = fill_stream_template(&t("pinned.line.context"), "used", &used);
    let context = fill_stream_template(&context, "limit", &limit);
    lines.push(fill_stream_template(
        &context,
        "percent",
        &percent.to_string(),
    ));
    if let Some(cost) = data.cost {
        lines.push(fill_stream_template(
            &t("pinned.line.cost"),
            "cost",
            &format!("${cost:.2}"),
        ));
    }
    if !data.changed_files.is_empty() {
        lines.push(String::new());
        lines.push(fill_stream_template(
            &t("pinned.files.title"),
            "count",
            &data.changed_files.len().to_string(),
        ));
        for change in data
            .changed_files
            .iter()
            .take(otb_core::pinned_models::MAX_LISTED_FILES)
        {
            let path =
                otb_core::pinned_models::display_path_for_project(&change.file, &data.project_path);
            let diff = otb_core::pinned_models::format_file_diff_suffix(
                change.additions,
                change.deletions,
            );
            let item = fill_stream_template(&t("pinned.files.item"), "path", &path);
            lines.push(fill_stream_template(&item, "diff", &diff));
        }
        if data.changed_files.len() > otb_core::pinned_models::MAX_LISTED_FILES {
            lines.push(fill_stream_template(
                &t("pinned.files.more"),
                "count",
                &(data.changed_files.len() - otb_core::pinned_models::MAX_LISTED_FILES).to_string(),
            ));
        }
    }
    lines.join("\n")
}

/// Render the compact `⏳ Working` progress line for the current stretch
/// (TS `t("progress.compact.activity", { header, activity })` parity).
/// Pure; blank activities fall back to the thinking line.
pub fn render_compact_progress(activity: &str, locale: &str) -> String {
    let activity = {
        let trimmed = activity.trim();
        if trimmed.is_empty() {
            crate::locale::translate(locale, "progress.compact.thinking")
        } else {
            trimmed.to_string()
        }
    };
    let template = crate::locale::translate(locale, "progress.compact.activity");
    let header = crate::locale::translate(locale, "progress.compact.working_header");
    let filled = fill_stream_template(&template, "header", &header);
    fill_stream_template(&filled, "activity", &activity)
}

/// Render the finished compact card's `✅ ...` count line, or `""` when
/// there is nothing to count (the card is then removed). Non-zero counts
/// in fixed order via the `progress.compact.count.*` i18n keys. Pure.
pub fn render_compact_summary(
    commands: u32,
    edits: u32,
    reads: u32,
    tools: u32,
    changed_files: usize,
    locale: &str,
) -> String {
    let entries: [(&str, usize); 5] = [
        ("progress.compact.count.commands", commands as usize),
        ("progress.compact.count.edits", edits as usize),
        ("progress.compact.count.reads", reads as usize),
        ("progress.compact.count.tools", tools as usize),
        ("progress.compact.count.changed_files", changed_files),
    ];
    let line = entries
        .iter()
        .filter(|entry| entry.1 > 0)
        .map(|(key, count)| {
            fill_stream_template(
                &crate::locale::translate(locale, key),
                "count",
                &count.to_string(),
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    if line.is_empty() {
        String::new()
    } else {
        format!("✅ {line}")
    }
}

/// Next stream-edit interval for a run of `run_elapsed_ms` (PRODUCT.md
/// #96: 1s the first minute, then 2s / 5s / 10s). Pure wrapper over the
/// `stream-throttle.ts` progressive tiers.
pub fn next_stream_interval_ms(run_elapsed_ms: u64) -> u64 {
    otb_core::streaming::resolve_progressive_throttle_ms(run_elapsed_ms)
}

/// Whether a stream edit (assistant/tool/thinking/pinned) is due:
/// the first edit always sends (`last_edit_ms == None`); later edits
/// wait for the progressive interval. Pure; the caller resets the clock
/// (elapsed back to zero) when the run stops so the next run restarts
/// at 1s.
pub fn should_send_stream_edit(
    last_edit_ms: Option<u64>,
    now_ms: u64,
    run_elapsed_ms: u64,
) -> bool {
    match last_edit_ms {
        None => true,
        Some(last) => {
            if now_ms < last {
                return true;
            }
            now_ms.saturating_sub(last) >= next_stream_interval_ms(run_elapsed_ms)
        }
    }
}

/// Send an assistant-run footer line (live path only; never called from
/// tests). Bot-composed: no link preview. Failures stay in `tracing`.
pub async fn send_footer_line(bot: &teloxide_core::Bot, chat_id: i64, footer: &str) {
    if footer.trim().is_empty() {
        return;
    }
    send_bot_composed_text(bot, chat_id, footer).await;
    tracing::info!("[otb-bot] telegram: assistant footer sent");
}

/// Live run-close send (TS `run-close.ts:16-61` delivery tail). Resolves
/// via [`resolve_run_close_footer`] and sends through [`send_footer_line`]
/// (bot-composed, no link preview). Tool flush / compact finalize stay with
/// the dispatcher; footerless turns log at debug and send nothing.
pub async fn send_run_close_footer(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    show_footer: bool,
    completed: Option<&RunCloseCompletedRun>,
    now_ms: i64,
    locale: &str,
) {
    let Some(footer) = resolve_run_close_footer(show_footer, completed, now_ms, locale) else {
        tracing::debug!("[otb-bot] telegram: run-close footer skipped (footerless turn)");
        return;
    };
    send_footer_line(bot, chat_id, &footer).await;
}

/// Live `session.idle` send (TS `session-lifecycle-handler.ts:108-111`
/// `closeForegroundRun` call). Gates through [`resolve_session_idle_footer`]
/// (shutdown / suppressed / interrupted-without-response turns log at debug
/// and send nothing), then shares the run-close tail
/// [`send_run_close_footer`] so the single send stays bot-composed with no
/// link preview. The gate string is discarded and re-resolved by the tail
/// (one small pure format per idle, same inputs so the same result);
/// completed-run hydration stays a dispatcher follow-up.
pub async fn send_session_idle_footer(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    show_footer: bool,
    completed: Option<&RunCloseCompletedRun>,
    now_ms: i64,
    locale: &str,
    shutdown_idle: bool,
    footer_suppressed: bool,
    interrupted_without_response: bool,
) {
    if resolve_session_idle_footer(
        show_footer,
        completed,
        now_ms,
        locale,
        shutdown_idle,
        footer_suppressed,
        interrupted_without_response,
    )
    .is_none()
    {
        tracing::debug!("[otb-bot] telegram: session-idle footer skipped (footerless turn)");
        return;
    }
    send_run_close_footer(bot, chat_id, show_footer, completed, now_ms, locale).await;
}

/// Send a pinned-dashboard snapshot (live path only; never called from
/// tests). Bot-composed: no link preview. The dashboard pin and the
/// failed-refresh-keeps-last merge stay with the dispatcher; see
/// [`apply_pinned_patch`] (pure merge) plus [`edit_pinned_dashboard`] and
/// [`install_pinned_dashboard_hook`] (live transport).
pub async fn send_pinned_dashboard(bot: &teloxide_core::Bot, chat_id: i64, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    send_bot_composed_text(bot, chat_id, text).await;
    tracing::info!("[otb-bot] telegram: pinned dashboard sent");
}

/// Incremental SSE/tool-event patch merged into the held dashboard snapshot
/// before rendering (pure inputs; the pump builds one per event and the
/// dispatcher holds the snapshot — telegram-ux owns only this merge).
///
/// TS parity (`pinned-message-manager.ts`):
/// - `onMessageComplete` / `updateTokensSilent`: `tokens_used` replaces
///   (`input + cacheRead`); `None` keeps the held value.
/// - `fetchContextLimit` failure: `tokens_limit` `None` keeps the held
///   limit (PRODUCT.md #104 never shows a placeholder zero); `Some(v)`
///   replaces (`0` still renders as `pinned.unknown` via
///   [`render_pinned_dashboard`]).
/// - `onCostUpdate`: `cost_delta` is additive and ignored when it cannot
///   change the total (via `should_apply_cost_update`); `cost_total`
///   replaces absolutely and wins when both are set (fresh load beats
///   in-flight deltas).
/// - `onSessionDiff` (`session.diff` event): `changed_files` `Some`
///   replaces (an empty list removes the section); `None` is a failed
///   refresh and keeps the last good list (PRODUCT.md #103).
/// - `onSessionTitleUpdate`: `session_title` applies only when non-empty
///   and different (via `should_update_title`).
///
/// Returns `true` when the snapshot changed (the caller re-renders only
/// then; unchanged diffs skip the edit like TS `areFileDiffsEqual`).
#[derive(Debug, Clone, Default)]
pub struct PinnedDashboardPatch {
    pub session_title: Option<String>,
    pub tokens_used: Option<u64>,
    pub tokens_limit: Option<u64>,
    pub cost_delta: Option<f64>,
    pub cost_total: Option<f64>,
    pub changed_files: Option<Vec<PinnedChangedFile>>,
}

/// Merge one [`PinnedDashboardPatch`] into the held snapshot (pure;
/// testable). Failed-refresh-keeps-last throughout: every `None` field
/// leaves the held value untouched. Additive cost; title gated.
pub fn apply_pinned_patch(data: &mut PinnedDashboardData, patch: &PinnedDashboardPatch) -> bool {
    let mut changed = false;
    if let Some(title) = patch.session_title.as_deref() {
        if otb_core::pinned_models::should_update_title(&data.session_title, title) {
            data.session_title = title.to_string();
            changed = true;
        }
    }
    if let Some(tokens) = patch.tokens_used {
        if data.tokens_used != tokens {
            data.tokens_used = tokens;
            changed = true;
        }
    }
    if let Some(limit) = patch.tokens_limit {
        if data.tokens_limit != Some(limit) {
            data.tokens_limit = Some(limit);
            changed = true;
        }
    }
    if let Some(total) = patch.cost_total {
        if data.cost != Some(total) {
            data.cost = Some(total);
            changed = true;
        }
    } else if let Some(delta) = patch.cost_delta {
        if otb_core::pinned_models::should_apply_cost_update(data.cost.unwrap_or(0.0), delta) {
            let next = data.cost.unwrap_or(0.0) + delta;
            if data.cost != Some(next) {
                data.cost = Some(next);
                changed = true;
            }
        }
    }
    if let Some(files) = patch.changed_files.as_ref() {
        if !otb_core::pinned_models::diffs_equal(
            &files
                .iter()
                .map(|change| otb_core::pinned_models::FileChange {
                    file: change.file.clone(),
                    additions: change.additions,
                    deletions: change.deletions,
                })
                .collect::<Vec<_>>(),
            &data
                .changed_files
                .iter()
                .map(|change| otb_core::pinned_models::FileChange {
                    file: change.file.clone(),
                    additions: change.additions,
                    deletions: change.deletions,
                })
                .collect::<Vec<_>>(),
        ) {
            data.changed_files = files.clone();
            changed = true;
        }
    }
    changed
}

/// Resolve the dashboard text for one merged snapshot (pure; testable).
/// TS `updatePinnedMessage` gate parity: `None` while the dashboard is
/// disabled or the session has no message state yet (a snapshot arriving
/// before the session is known never draws); otherwise the
/// [`render_pinned_dashboard`] text. Blank renders stay `None` so the
/// caller sends nothing.
pub fn resolve_pinned_text(
    data: &PinnedDashboardData,
    dashboard_enabled: bool,
    has_message_state: bool,
    locale: &str,
) -> Option<String> {
    if !pinned_dashboard_visible(dashboard_enabled, true, has_message_state) {
        return None;
    }
    let text = render_pinned_dashboard(data, locale);
    if text.trim().is_empty() {
        return None;
    }
    Some(text)
}

/// Whether a pinned edit is due (pure; testable). TS
/// `lastRenderedMessageText` parity: an unchanged re-render skips the edit
/// (Telegram would answer `message is not modified`), as does a blank
/// render.
pub fn should_send_pinned_edit(last_text: Option<&str>, next_text: &str) -> bool {
    if next_text.trim().is_empty() {
        return false;
    }
    last_text != Some(next_text)
}

/// Outcome of one live pinned-dashboard edit (testable classification over
/// `pinned_models` Telegram-error gates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinnedEditOutcome {
    /// Edit applied (or answered `message is not modified`, which counts as
    /// delivered per TS `flushPendingPinnedUpdates`).
    Updated,
    /// The dashboard was deleted (`message to edit not found`): the caller
    /// recreates it via [`send_pinned_dashboard`] (TS recreate path).
    Deleted,
    /// Any other failure (network, rate limit): ownership stays, no
    /// recreate (TS logs and keeps the id for retry).
    Failed,
}

/// Edit the live pinned dashboard in place (live path only; never called
/// from tests). Bot-composed: the edit reuses
/// [`bot_composed_preview_options`] so no preview appears on edit either.
/// Failures stay in `tracing`; the return tells the caller whether a
/// recreate (`Deleted`) is due.
pub async fn edit_pinned_dashboard(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    message_id: MessageId,
    text: &str,
) -> PinnedEditOutcome {
    if text.trim().is_empty() {
        return PinnedEditOutcome::Updated;
    }
    let result = bot
        .edit_message_text(ChatId(chat_id), message_id, text.to_string())
        .link_preview_options(bot_composed_preview_options())
        .await;
    match result {
        Ok(_) => {
            tracing::debug!("[otb-bot] telegram: pinned dashboard edited");
            PinnedEditOutcome::Updated
        }
        Err(err) => {
            let message = err.to_string();
            if otb_core::pinned_models::is_not_modified_error(&message) {
                tracing::debug!("[otb-bot] telegram: pinned edit unchanged (already current)");
                PinnedEditOutcome::Updated
            } else if otb_core::pinned_models::is_message_deleted_error(&message) {
                tracing::warn!("[otb-bot] telegram: pinned message deleted, recreate due");
                PinnedEditOutcome::Deleted
            } else {
                tracing::warn!("[otb-bot] telegram: pinned edit failed: {err}");
                PinnedEditOutcome::Failed
            }
        }
    }
}

/// Merged snapshot plus the draw gate for one hook fire (the pump merges
/// SSE/tool events via [`apply_pinned_patch`], then emits this; the
/// install below renders and sends/edits).
#[derive(Debug, Clone)]
pub struct PinnedDashboardSignal {
    pub data: PinnedDashboardData,
    pub has_message_state: bool,
}

/// Install the telegram-ux pinned-dashboard sender behind the pump's
/// [`crate::attach::set_pinned_dashboard_hook`] wiring (live path only).
///
/// Each hook fire spawns the send: the pump hook is sync while sending is
/// async. A zero `chat_id` (the deny-all sentinel, no DM target) installs
/// nothing. The hook holds the dashboard message id plus the last rendered
/// text and edit clock: the first fire sends ([`send_pinned_dashboard`]
/// shape, no preview) and remembers the id; later fires skip unchanged
/// re-renders ([`should_send_pinned_edit`]), wait out the stream-throttle
/// floor ([`should_send_stream_edit`] at the 1s tier — the pump's debounce
/// owns the progressive tiers), then edit in place
/// ([`edit_pinned_dashboard`], no preview); a deleted dashboard falls back
/// to a fresh send on the next fire. Pin/unpin and snapshot hydration stay
/// dispatcher follow-ups.
///
/// ASSUMPTION: single-user bot, so the target is the allowed user's DM
/// (`chat_id == allowed user id`), like
/// [`install_session_idle_footer_hook`].
pub fn install_pinned_dashboard_hook(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    locale: &str,
    dashboard_enabled: bool,
) {
    if chat_id == 0 {
        tracing::debug!("[otb-bot] telegram: pinned hook not installed (no chat target)");
        return;
    }
    let hook_bot = bot.clone();
    let hook_locale = locale.to_string();
    let message_id = std::sync::Arc::new(std::sync::Mutex::new(None::<MessageId>));
    let last_text = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let last_edit_ms = std::sync::Arc::new(std::sync::Mutex::new(None::<u64>));
    crate::attach::set_pinned_dashboard_hook(Some(Box::new(move |signal| {
        let bot = hook_bot.clone();
        let locale = hook_locale.clone();
        let message_id = message_id.clone();
        let last_text = last_text.clone();
        let last_edit_ms = last_edit_ms.clone();
        tokio::spawn(async move {
            let Some(text) = resolve_pinned_text(
                &signal.data,
                dashboard_enabled,
                signal.has_message_state,
                &locale,
            ) else {
                tracing::debug!("[otb-bot] telegram: pinned update skipped (dashboard gated)");
                return;
            };
            let unchanged = !should_send_pinned_edit(
                last_text
                    .lock()
                    .ok()
                    .as_ref()
                    .and_then(|text| text.as_deref()),
                &text,
            );
            if unchanged {
                tracing::debug!("[otb-bot] telegram: pinned update skipped (content unchanged)");
                return;
            }
            let now_ms = live_now_ms() as u64;
            let due = last_edit_ms
                .lock()
                .ok()
                .map(|last| should_send_stream_edit(*last, now_ms, 0))
                .unwrap_or(true);
            if !due {
                tracing::debug!("[otb-bot] telegram: pinned edit throttled");
                return;
            }
            let known_id = message_id.lock().ok().and_then(|id| *id);
            if let Some(known) = known_id {
                match edit_pinned_dashboard(&bot, chat_id, known, &text).await {
                    PinnedEditOutcome::Updated => {
                        if let Ok(mut last) = last_text.lock() {
                            *last = Some(text);
                        }
                        if let Ok(mut last) = last_edit_ms.lock() {
                            *last = Some(live_now_ms() as u64);
                        }
                    }
                    PinnedEditOutcome::Deleted => {
                        if let Ok(mut id) = message_id.lock() {
                            *id = None;
                        }
                        tracing::warn!(
                            "[otb-bot] telegram: pinned dashboard lost, next fire re-sends"
                        );
                    }
                    PinnedEditOutcome::Failed => {}
                }
                return;
            }
            let (chat, body) = build_send_payload(chat_id, &text);
            match bot
                .send_message(chat, body)
                .link_preview_options(bot_composed_preview_options())
                .await
            {
                Ok(sent) => {
                    if let Ok(mut id) = message_id.lock() {
                        *id = Some(sent.id);
                    }
                    if let Ok(mut last) = last_text.lock() {
                        *last = Some(text);
                    }
                    if let Ok(mut last) = last_edit_ms.lock() {
                        *last = Some(live_now_ms() as u64);
                    }
                    tracing::info!("[otb-bot] telegram: pinned dashboard sent");
                }
                Err(err) => {
                    tracing::warn!("[otb-bot] telegram: pinned dashboard send failed: {err}");
                }
            }
        });
    })));
    tracing::info!("[otb-bot] telegram: pinned dashboard hook installed");
}

/// Send a compact progress/summary line (live path only; never called
/// from tests). Bot-composed: no link preview. Card edits/deletes stay
/// with the dispatcher.
pub async fn send_compact_line(bot: &teloxide_core::Bot, chat_id: i64, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    send_bot_composed_text(bot, chat_id, text).await;
    tracing::debug!("[otb-bot] telegram: compact line sent");
}

/// Whether a compact progress edit should go out (pure; testable).
/// TS `lastRenderedMessageText` parity (same gate as
/// [`should_send_pinned_edit`]): a blank render or an unchanged re-render
/// skips the edit (Telegram would answer `message is not modified`).
pub fn should_send_compact_edit(last_text: Option<&str>, next_text: &str) -> bool {
    if next_text.trim().is_empty() {
        return false;
    }
    last_text != Some(next_text)
}

/// Whether a closing compact card should be deleted instead of edited
/// (pure; testable). TS `closeState:277-294` parity: the card is removed
/// when the run asks for it (`deleteOnFinish`, the "Delete progress on
/// finish" setting) or when the summary is empty (nothing to count);
/// otherwise the `✅ ...` line edits the card in place.
pub fn should_delete_compact_card(summary: &str, delete_on_finish: bool) -> bool {
    delete_on_finish || summary.trim().is_empty()
}

/// Additive summary render from aggregated counts (pure; testable).
/// The dispatcher accumulates one [`otb_core::streaming::ToolCallCounts`]
/// per finished call id (counted once) via
/// [`otb_core::streaming::add_tool_call_counts`] plus the distinct
/// changed-file count, then renders here through
/// [`render_compact_summary`] (fixed order, zero counts left out, `""`
/// when there is nothing to count). TS
/// `formatCompactSummary:73-89` parity.
pub fn render_compact_summary_from_counts(
    counts: &otb_core::streaming::ToolCallCounts,
    changed_files: usize,
    locale: &str,
) -> String {
    render_compact_summary(
        counts.commands,
        counts.edits,
        counts.reads,
        counts.tools,
        changed_files,
        locale,
    )
}

/// Whether a stretch-start progress update is due (pure; testable).
/// Wires the compact card onto the shared stream throttle: the content
/// must change ([`should_send_compact_edit`]) AND the progressive
/// interval must have elapsed ([`should_send_stream_edit`] over
/// [`next_stream_interval_ms`], PRODUCT.md #96 — 1s the first minute,
/// then 2s / 5s / 10s). The caller resets the edit clock when the run
/// stops so the next stretch restarts at 1s. TS
/// `compact-progress-streamer.ts:451,458-473` parity (`updateActivityState`
/// render plus the `ensureTimer` throttle gate).
pub fn should_send_compact_progress(
    last_text: Option<&str>,
    last_edit_ms: Option<u64>,
    now_ms: u64,
    run_elapsed_ms: u64,
    next_text: &str,
) -> bool {
    should_send_compact_edit(last_text, next_text)
        && should_send_stream_edit(last_edit_ms, now_ms, run_elapsed_ms)
}

/// Outcome of one live compact-card edit/delete (testable classification
/// over the shared Telegram-error gates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactEditOutcome {
    /// Edit applied (or answered `message is not modified`, which counts
    /// as delivered per TS `syncState`).
    Updated,
    /// The card was deleted (`message to edit not found`): the caller
    /// drops the held id; the next stretch sends fresh.
    Deleted,
    /// Any other failure (network, rate limit): ownership stays, no
    /// recreate (TS logs and keeps the id for retry).
    Failed,
}

/// Edit the live compact card in place (live path only; never called from
/// tests). Bot-composed: the edit reuses
/// [`bot_composed_preview_options`] so no preview appears on edit either.
/// Blank text never edits (the caller deletes instead via
/// [`should_delete_compact_card`]). Failures stay in `tracing`; the
/// return tells the caller whether the held id is stale (`Deleted`).
pub async fn edit_compact_line(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    message_id: MessageId,
    text: &str,
) -> CompactEditOutcome {
    if text.trim().is_empty() {
        return CompactEditOutcome::Updated;
    }
    let result = bot
        .edit_message_text(ChatId(chat_id), message_id, text.to_string())
        .link_preview_options(bot_composed_preview_options())
        .await;
    match result {
        Ok(_) => {
            tracing::debug!("[otb-bot] telegram: compact card edited");
            CompactEditOutcome::Updated
        }
        Err(err) => {
            let message = err.to_string();
            if otb_core::pinned_models::is_not_modified_error(&message) {
                tracing::debug!("[otb-bot] telegram: compact edit unchanged (already current)");
                CompactEditOutcome::Updated
            } else if otb_core::pinned_models::is_message_deleted_error(&message) {
                tracing::warn!("[otb-bot] telegram: compact card deleted, next stretch re-sends");
                CompactEditOutcome::Deleted
            } else {
                tracing::warn!("[otb-bot] telegram: compact edit failed: {err}");
                CompactEditOutcome::Failed
            }
        }
    }
}

/// Delete the live compact card (live path only; never called from
/// tests). Best-effort: failures stay in `tracing`. Returns `true` when
/// the card is gone (deleted or already unknown); `false` keeps the held
/// id for retry (rate limit / network).
pub async fn delete_compact_line(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    message_id: MessageId,
) -> bool {
    match bot.delete_message(ChatId(chat_id), message_id).await {
        Ok(_) => {
            tracing::debug!("[otb-bot] telegram: compact card deleted");
            true
        }
        Err(err) => {
            let message = err.to_string();
            if otb_core::pinned_models::is_message_deleted_error(&message) {
                tracing::debug!("[otb-bot] telegram: compact card already gone");
                true
            } else {
                tracing::warn!("[otb-bot] telegram: compact delete failed: {err}");
                false
            }
        }
    }
}

/// Live compact finalize: summary-or-remove for one closing stretch
/// (live path only; never called from tests). Renders nothing here: the
/// caller passes the [`render_compact_summary`] / [`render_compact_summary_from_counts`]
/// line (or `""` when there is nothing to count). TS
/// `closeState:277-302` parity: `(deleteOnFinish || !summary)` deletes the
/// card, otherwise the summary edits it in place; a never-sent card sends
/// the summary instead (TS `syncState` send branch) and an empty summary
/// with no card sends nothing. The single close edit/delete bypasses the
/// stream throttle (TS finalizes via `syncState` directly); stretch
/// updates stay throttled via [`should_send_compact_progress`].
/// Bot-composed throughout: no link preview on send or edit.
pub async fn finalize_compact_card(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    message_id: Option<MessageId>,
    summary: &str,
    delete_on_finish: bool,
) -> CompactEditOutcome {
    if should_delete_compact_card(summary, delete_on_finish) {
        if let Some(known) = message_id {
            if delete_compact_line(bot, chat_id, known).await {
                return CompactEditOutcome::Updated;
            }
            return CompactEditOutcome::Failed;
        }
        tracing::debug!("[otb-bot] telegram: compact finalize skipped (nothing to count)");
        return CompactEditOutcome::Updated;
    }
    match message_id {
        Some(known) => edit_compact_line(bot, chat_id, known, summary).await,
        None => {
            send_compact_line(bot, chat_id, summary).await;
            CompactEditOutcome::Updated
        }
    }
}

/// File-browser callback prefixes (Node `file-browser-menu.ts` parity).
/// Values reuse `otb_core::file_browser_menu_gates` so the shapes cannot
/// drift.
pub const OPEN_CALLBACK_PREFIX: &str = otb_core::file_browser_menu_gates::OPEN_CALLBACK_PREFIX;
pub const OPEN_CALLBACK_NAV_PREFIX: &str =
    otb_core::file_browser_menu_gates::OPEN_CALLBACK_NAV_PREFIX;
pub const OPEN_CALLBACK_SELECT_PREFIX: &str =
    otb_core::file_browser_menu_gates::OPEN_CALLBACK_SELECT_PREFIX;
pub const OPEN_CALLBACK_PAGE_PREFIX: &str =
    otb_core::file_browser_menu_gates::OPEN_CALLBACK_PAGE_PREFIX;
pub const OPEN_CALLBACK_ROOTS: &str = otb_core::file_browser_menu_gates::OPEN_CALLBACK_ROOTS;
pub const LS_CALLBACK_PREFIX: &str = otb_core::file_browser_menu_gates::LS_CALLBACK_PREFIX;
pub const LS_CALLBACK_NAV_PREFIX: &str = otb_core::file_browser_menu_gates::LS_CALLBACK_NAV_PREFIX;
pub const LS_CALLBACK_FILE_PREFIX: &str =
    otb_core::file_browser_menu_gates::LS_CALLBACK_FILE_PREFIX;
pub const LS_CALLBACK_DOWNLOAD_PREFIX: &str =
    otb_core::file_browser_menu_gates::LS_CALLBACK_DOWNLOAD_PREFIX;
pub const LS_CALLBACK_ATTACH_PREFIX: &str =
    otb_core::file_browser_menu_gates::LS_CALLBACK_ATTACH_PREFIX;
pub const LS_CALLBACK_BACK_PREFIX: &str =
    otb_core::file_browser_menu_gates::LS_CALLBACK_BACK_PREFIX;
pub const LS_CALLBACK_PAGE_PREFIX: &str =
    otb_core::file_browser_menu_gates::LS_CALLBACK_PAGE_PREFIX;

/// Exact `open:roots` match (Node `data === OPEN_CALLBACK_ROOTS`).
pub fn is_open_roots(data: &str) -> bool {
    data == OPEN_CALLBACK_ROOTS
}

/// Parse an `open:nav:<path>` callback (pure; Node
/// `decodeOpenPathFromCallback(OPEN_CALLBACK_NAV_PREFIX, ...)` parity,
/// stateless: the full path travels inline, empty yields `None`).
pub fn parse_open_nav_callback(data: &str) -> Option<&str> {
    let path = data.strip_prefix(OPEN_CALLBACK_NAV_PREFIX)?;
    if path.is_empty() {
        return None;
    }
    Some(path)
}

/// Parse an `open:sel:<path>` callback (pure; same shape as the nav
/// parser over the select prefix).
pub fn parse_open_select_callback(data: &str) -> Option<&str> {
    let path = data.strip_prefix(OPEN_CALLBACK_SELECT_PREFIX)?;
    if path.is_empty() {
        return None;
    }
    Some(path)
}

/// Split a `<path>|<page>` payload at the last `|` (pure; Node
/// `decodePathWithPageCallback` `lastIndexOf` parity). The page must be
/// digits-only; an empty path or a missing separator yields `None`.
fn split_path_page_payload(rest: &str) -> Option<(String, usize)> {
    let (path, page_text) = rest.rsplit_once('|')?;
    if path.is_empty() || page_text.is_empty() || !page_text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((path.to_string(), page_text.parse::<usize>().ok()?))
}

/// Parse an `open:pg:<path>|<page>` callback (pure; Node
/// `decodeOpenPaginationCallback` parity, digits-only page).
pub fn parse_open_page_callback(data: &str) -> Option<(String, usize)> {
    let rest = data.strip_prefix(OPEN_CALLBACK_PAGE_PREFIX)?;
    split_path_page_payload(rest)
}

/// Parse an `ls:nav:<path>` callback (pure; Node
/// `decodeLsPathFromCallback(LS_CALLBACK_NAV_PREFIX, ...)` parity).
pub fn parse_ls_nav_callback(data: &str) -> Option<&str> {
    let path = data.strip_prefix(LS_CALLBACK_NAV_PREFIX)?;
    if path.is_empty() {
        return None;
    }
    Some(path)
}

/// Parse an `ls:attach:<path>` callback (pure; Node
/// `decodeLsAttachCallback` parity).
pub fn parse_ls_attach_callback(data: &str) -> Option<&str> {
    let path = data.strip_prefix(LS_CALLBACK_ATTACH_PREFIX)?;
    if path.is_empty() {
        return None;
    }
    Some(path)
}

/// Parse an `ls:download:<path>` callback (pure; same non-page shape
/// over the download prefix).
pub fn parse_ls_download_callback(data: &str) -> Option<&str> {
    let path = data.strip_prefix(LS_CALLBACK_DOWNLOAD_PREFIX)?;
    if path.is_empty() {
        return None;
    }
    Some(path)
}

/// Parse an `ls:pg:<path>|<page>` callback (pure; Node
/// `decodeLsPaginationCallback` parity).
pub fn parse_ls_page_callback(data: &str) -> Option<(String, usize)> {
    let rest = data.strip_prefix(LS_CALLBACK_PAGE_PREFIX)?;
    split_path_page_payload(rest)
}

/// Parse an `ls:file:<path>|<page>` callback (pure; Node
/// `decodeLsFileCallback` parity).
pub fn parse_ls_file_callback(data: &str) -> Option<(String, usize)> {
    let rest = data.strip_prefix(LS_CALLBACK_FILE_PREFIX)?;
    split_path_page_payload(rest)
}

/// Parse an `ls:back:<path>|<page>` callback (pure; Node
/// `decodeLsBackCallback` parity).
pub fn parse_ls_back_callback(data: &str) -> Option<(String, usize)> {
    let rest = data.strip_prefix(LS_CALLBACK_BACK_PREFIX)?;
    split_path_page_payload(rest)
}

/// Truncate an inline-button label to the 64-char Telegram budget (pure;
/// Node `truncateLabel` parity: keep head + `...`).
fn truncate_browser_label(label: &str) -> String {
    const LIMIT: usize = 64;
    if label.chars().count() <= LIMIT {
        return label.to_string();
    }
    let kept: String = label.chars().take(LIMIT - 3).collect();
    format!("{kept}...")
}

/// One button row per browser root: `"📂 display"` with
/// `callback_data = "open:nav:<root>"`, plus a lone
/// `inline:cancel:open` row (pure; Node `buildOpenRootsKeyboard`
/// parity).
pub fn build_open_roots_keyboard(
    roots: &[String],
    home: &str,
    locale: &str,
) -> InlineKeyboardMarkup {
    use otb_core::file_browser_service_gates::path_to_display_path;
    let mut rows: Vec<Vec<InlineKeyboardButton>> = roots
        .iter()
        .map(|root| {
            let label = truncate_browser_label(&format!("📂 {}", path_to_display_path(root, home)));
            vec![InlineKeyboardButton::callback(
                label,
                format!("{OPEN_CALLBACK_NAV_PREFIX}{root}"),
            )]
        })
        .collect();
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "inline.button.cancel"),
        otb_core::inline_menu_gates::inline_cancel_callback_data("open"),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Browse keyboard for one `/open` page (pure; Node
/// `buildOpenBrowseKeyboard` parity): one `open:nav:<full>` row per
/// subdirectory, an up (`open:nav:<parent>`) / roots (`open:roots`) row
/// when applicable, an `open:pg:<current>|<n>` pager row when more than
/// one page exists, then the `open:sel:<current>` select row and the
/// `inline:cancel:open` row.
#[allow(clippy::too_many_arguments)]
pub fn build_open_browse_keyboard(
    entries: &[String],
    current_path: &str,
    page: usize,
    total_pages: usize,
    has_parent: bool,
    is_allowed_root: bool,
    show_roots: bool,
    locale: &str,
) -> InlineKeyboardMarkup {
    use otb_core::file_browser_service_gates::{join_path, parent_path};
    let mut rows: Vec<Vec<InlineKeyboardButton>> = entries
        .iter()
        .map(|name| {
            let full = join_path(current_path, name);
            let label = truncate_browser_label(
                &otb_core::file_browser_service_gates::build_entry_label(name),
            );
            vec![InlineKeyboardButton::callback(
                label,
                format!("{OPEN_CALLBACK_NAV_PREFIX}{full}"),
            )]
        })
        .collect();
    let at_root = is_allowed_root;
    let show_up = has_parent && !at_root;
    if show_up || show_roots {
        let mut nav: Vec<InlineKeyboardButton> = Vec::new();
        if show_up {
            nav.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "open.back"),
                format!("{OPEN_CALLBACK_NAV_PREFIX}{}", parent_path(current_path)),
            ));
        }
        if show_roots {
            nav.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "open.roots"),
                OPEN_CALLBACK_ROOTS.to_string(),
            ));
        }
        if !nav.is_empty() {
            rows.push(nav);
        }
    }
    if total_pages > 1 {
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "open.prev_page"),
                format!("{OPEN_CALLBACK_PAGE_PREFIX}{current_path}|{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "open.next_page"),
                format!("{OPEN_CALLBACK_PAGE_PREFIX}{current_path}|{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "open.select_current"),
        format!("{OPEN_CALLBACK_SELECT_PREFIX}{current_path}"),
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "inline.button.cancel"),
        otb_core::inline_menu_gates::inline_cancel_callback_data("open"),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Browse keyboard for one `/ls` page (pure; Node `buildLsBrowseKeyboard`
/// parity): directories carry `ls:nav:<full>`, files carry
/// `ls:file:<full>|<page>`; an up row (`ls:nav:<parent>`) appears for a
/// non-root parent, then the `ls:pg:<current>|<n>` pager and the
/// `inline:cancel:ls` row.
pub fn build_ls_browse_keyboard(
    entries: &[otb_core::file_browser_service_gates::LsEntry],
    current_path: &str,
    page: usize,
    total_pages: usize,
    has_parent: bool,
    is_project_root: bool,
    locale: &str,
) -> InlineKeyboardMarkup {
    use otb_core::file_browser_service_gates::parent_path;
    let mut rows: Vec<Vec<InlineKeyboardButton>> = entries
        .iter()
        .map(|entry| {
            let glyph = if entry.is_dir { "📁" } else { "📄" };
            let label = truncate_browser_label(&format!("{glyph} {}", entry.name));
            let data = if entry.is_dir {
                format!("{LS_CALLBACK_NAV_PREFIX}{}", entry.full_path)
            } else {
                format!("{LS_CALLBACK_FILE_PREFIX}{}|{page}", entry.full_path)
            };
            vec![InlineKeyboardButton::callback(label, data)]
        })
        .collect();
    if has_parent && !is_project_root {
        rows.push(vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "open.back"),
            format!("{LS_CALLBACK_NAV_PREFIX}{}", parent_path(current_path)),
        )]);
    }
    if total_pages > 1 {
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "open.prev_page"),
                format!("{LS_CALLBACK_PAGE_PREFIX}{current_path}|{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "open.next_page"),
                format!("{LS_CALLBACK_PAGE_PREFIX}{current_path}|{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "inline.button.cancel"),
        otb_core::inline_menu_gates::inline_cancel_callback_data("ls"),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Detail keyboard for one `/ls` file (pure; Node
/// `buildLsFileDetailsKeyboard` parity): an attach row only when the
/// file may be attached, then the download row, the
/// `ls:back:<parent>|<page>` row, and the `inline:cancel:ls` row.
pub fn build_ls_file_keyboard(
    file_path: &str,
    parent_path_value: &str,
    page: usize,
    can_attach: bool,
    locale: &str,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    if can_attach {
        rows.push(vec![InlineKeyboardButton::callback(
            crate::locale::translate(locale, "ls.file.attach"),
            format!("{LS_CALLBACK_ATTACH_PREFIX}{file_path}"),
        )]);
    }
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "ls.file.download"),
        format!("{LS_CALLBACK_DOWNLOAD_PREFIX}{file_path}"),
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "ls.file.back"),
        format!("{LS_CALLBACK_BACK_PREFIX}{parent_path_value}|{page}"),
    )]);
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "inline.button.cancel"),
        otb_core::inline_menu_gates::inline_cancel_callback_data("ls"),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Parse a `commands:select:<n>` callback into the global catalog position
/// (pure; Node `parseCommandSelectCallback` parity: digits-only,
/// non-negative; page callbacks never leak in through this prefix).
pub fn parse_command_select_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(COMMANDS_CALLBACK_SELECT_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `commands:page:<n>` callback into the page number (pure; Node
/// `parseCommandPageCallback` parity).
pub fn parse_command_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(COMMANDS_CALLBACK_PAGE_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Exact `commands:cancel` match (Node `data === COMMANDS_CALLBACK_CANCEL`).
pub fn is_commands_cancel(data: &str) -> bool {
    data == COMMANDS_CALLBACK_CANCEL
}

/// Exact `commands:execute` match (Node `data === COMMANDS_CALLBACK_EXECUTE`).
pub fn is_commands_execute(data: &str) -> bool {
    data == COMMANDS_CALLBACK_EXECUTE
}

/// Parse a `skills:select:<n>` callback (pure; Node
/// `parseSkillSelectCallback` parity).
pub fn parse_skill_select_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(SKILLS_CALLBACK_SELECT_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Parse a `skills:page:<n>` callback (pure; Node `parseSkillPageCallback`
/// parity).
pub fn parse_skill_page_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(SKILLS_CALLBACK_PAGE_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Exact `skills:cancel` match.
pub fn is_skills_cancel(data: &str) -> bool {
    data == SKILLS_CALLBACK_CANCEL
}

/// Exact `skills:execute` match.
pub fn is_skills_execute(data: &str) -> bool {
    data == SKILLS_CALLBACK_EXECUTE
}

/// Parse a `mcps:select:<n>` callback (pure; Node `parseMcpSelectCallback`
/// parity).
pub fn parse_mcp_select_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(MCPS_CALLBACK_SELECT_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Exact `mcps:toggle` match.
pub fn is_mcp_toggle(data: &str) -> bool {
    data == MCPS_CALLBACK_TOGGLE
}

/// Exact `mcps:back` match.
pub fn is_mcp_back(data: &str) -> bool {
    data == MCPS_CALLBACK_BACK
}

/// Exact `mcps:cancel` match.
pub fn is_mcp_cancel(data: &str) -> bool {
    data == MCPS_CALLBACK_CANCEL
}

/// Parse an `agent:<name>` callback into the agent name (pure; Node
/// `agent-selection-callback-handler.ts` `startsWith("agent:")` parity; an
/// empty name yields `None`).
pub fn parse_agent_callback(data: &str) -> Option<&str> {
    let name = data.strip_prefix(AGENT_CALLBACK_PREFIX)?;
    if name.is_empty() {
        return None;
    }
    Some(name)
}

/// Parse a `variant:<id>` callback into the variant id (pure; Node
/// `variant-selection-callback-handler.ts` parity; an empty id yields
/// `None`).
pub fn parse_variant_callback(data: &str) -> Option<&str> {
    let id = data.strip_prefix(VARIANT_CALLBACK_PREFIX)?;
    if id.is_empty() {
        return None;
    }
    Some(id)
}

/// `true` for any `question:*` payload (Node `startsWith("question:")`
/// guard in `question-callback-handler.ts`).
pub fn is_question_callback(data: &str) -> bool {
    otb_core::question_permission_inline_gates::is_question_callback(data)
}

/// Parse a `question:*` callback into its action + indexes (pure; Node
/// `handleQuestionCallback` dispatch parity:
/// `select:<q>:<o>`, `submit:<q>`, `custom:<q>`, `toggle_custom:<q>`,
/// `cancel:<q>`; anything else yields `None`).
pub fn parse_question_callback(
    data: &str,
) -> Option<otb_core::question_permission_inline_gates::QuestionCallback> {
    otb_core::question_permission_inline_gates::parse_question_callback(data)
}

/// `true` for any `permission:*` payload (Node `startsWith("permission:")`
/// guard in `permission-callback-handler.ts`).
pub fn is_permission_callback(data: &str) -> bool {
    otb_core::question_permission_inline_gates::is_permission_callback(data)
}

/// Which reply a `permission:*` payload carries; `None` for anything but
/// `once` / `always` / `reject` (Node `isPermissionReply` action gate).
pub fn classify_permission_callback(
    data: &str,
) -> Option<otb_core::question_permission_inline_gates::PermissionReply> {
    otb_core::question_permission_inline_gates::classify_permission_callback(data)
}

/// Parse a `model:list:<kind>:<n>` callback into `(kind, index)` (pure; Node
/// `resolveModelListCallback` shape parity: `favorites` or `recent`,
/// digits-only index).
pub fn parse_model_list_callback(data: &str) -> Option<(&str, usize)> {
    let rest = data.strip_prefix(MODEL_LIST_CALLBACK_PREFIX)?;
    let mut parts = rest.split(':');
    let kind = parts.next()?;
    let index_text = parts.next()?;
    if parts.next().is_some() || index_text.is_empty() {
        return None;
    }
    if kind != "favorites" && kind != "recent" {
        return None;
    }
    if !index_text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((kind, index_text.parse::<usize>().ok()?))
}

/// Parse a `model:providers:<n>` callback (pure; delegates to the core
/// `parseProvidersPageCallback` gate).
pub fn parse_model_providers_page_callback(data: &str) -> Option<usize> {
    otb_core::agent_model_variant_gates::parse_providers_page_callback(data)
        .and_then(|page| usize::try_from(page).ok())
}

/// Parse a `model:provider:<providerIndex>:<page>` callback (pure;
/// delegates to the core `parseProviderCallback` gate).
pub fn parse_model_provider_callback(data: &str) -> Option<(usize, usize)> {
    let (provider_index, page) =
        otb_core::agent_model_variant_gates::parse_provider_callback(data)?;
    Some((
        usize::try_from(provider_index).ok()?,
        usize::try_from(page).ok()?,
    ))
}

/// Parse a `model:pick:<n>` callback (pure; delegates to the core
/// `parseProviderModelCallback` gate).
pub fn parse_model_pick_callback(data: &str) -> Option<usize> {
    otb_core::agent_model_variant_gates::parse_provider_model_callback(data)
        .and_then(|index| usize::try_from(index).ok())
}

/// Parse a `model:result:<n>` search-result pick into the result index
/// (pure; Node `parseCallbackIndex(data,
/// MODEL_SEARCH_RESULT_CALLBACK_PREFIX)` parity: digits-only, empty
/// yields `None`). `"model:search:result:<n>"` is *not* a Node shape and
/// yields `None`.
pub fn parse_model_search_result_callback(data: &str) -> Option<usize> {
    let raw = data.strip_prefix(MODEL_SEARCH_RESULT_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<usize>().ok()
}

/// Build a self-contained legacy model pick `model:<provider>:<model>`
/// (pure; Node `parseLegacyModelCallback` shape parity: the provider is the
/// segment after `model:`, the model is the `:`-joined tail so ids
/// containing colons round-trip). Search-result buttons use this shape so
/// taps resolve without remembered search state (see [`model_search_view`]).
pub fn model_legacy_callback(provider_id: &str, model_id: &str) -> String {
    format!("model:{provider_id}:{model_id}")
}

/// Which catalog a list keyboard belongs to (select/cancel/execute shapes
/// differ only by prefix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatalogKind {
    Commands,
    Skills,
}

/// One button row per catalog entry: `"/{name} - {desc}"` capped at 64
/// chars with `callback_data = "<prefix>:select:<globalIndex>"` (pure;
/// Node `formatCommandButtonLabel` / `formatSkillButtonLabel` parity: the
/// description falls back to the `no_description` template). A trailing
/// pager row carries `<prefix>:page:<n>` buttons when more than one page
/// exists, then a lone cancel row carries the exact `<prefix>cancel` shape.
pub fn build_catalog_list_keyboard(
    items: &[otb_core::session_pick_catalog_gates::CatalogEntry],
    global_offset: usize,
    kind: CatalogKind,
    locale: &str,
    page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let (select_prefix, page_prefix, cancel) = match kind {
        CatalogKind::Commands => (
            COMMANDS_CALLBACK_SELECT_PREFIX,
            COMMANDS_CALLBACK_PAGE_PREFIX,
            COMMANDS_CALLBACK_CANCEL,
        ),
        CatalogKind::Skills => (
            SKILLS_CALLBACK_SELECT_PREFIX,
            SKILLS_CALLBACK_PAGE_PREFIX,
            SKILLS_CALLBACK_CANCEL,
        ),
    };
    let no_description = match kind {
        CatalogKind::Commands => crate::locale::translate(locale, "commands.no_description"),
        CatalogKind::Skills => crate::locale::translate(locale, "skills.no_description"),
    };
    let button_text = match kind {
        CatalogKind::Commands => crate::locale::translate(locale, "commands.button.cancel"),
        CatalogKind::Skills => crate::locale::translate(locale, "skills.button.cancel"),
    };
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let description = item
                .description
                .as_deref()
                .map(str::trim)
                .filter(|description| !description.is_empty())
                .unwrap_or(&no_description);
            let raw_label = format!("/{name} - {description}", name = item.name);
            let label = if raw_label.chars().count() <= 64 {
                raw_label
            } else {
                let kept: String = raw_label.chars().take(61).collect();
                format!("{kept}...")
            };
            vec![InlineKeyboardButton::callback(
                label,
                format!("{select_prefix}{}", global_offset + index),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        let (prev_label, next_label) = match kind {
            CatalogKind::Commands => (
                crate::locale::translate(locale, "commands.button.prev_page"),
                crate::locale::translate(locale, "commands.button.next_page"),
            ),
            CatalogKind::Skills => (
                crate::locale::translate(locale, "skills.button.prev_page"),
                crate::locale::translate(locale, "skills.button.next_page"),
            ),
        };
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                prev_label,
                format!("{page_prefix}{}", page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                next_label,
                format!("{page_prefix}{}", page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    rows.push(vec![InlineKeyboardButton::callback(
        button_text,
        cancel.to_string(),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Confirm keyboard for a selected command/skill (Node
/// `buildCommandsConfirmKeyboard` / `buildSkillsConfirmKeyboard` parity):
/// `[Execute, Cancel]` sharing one row with the exact
/// `<prefix>execute` / `<prefix>cancel` shapes (pure).
pub fn build_catalog_confirm_keyboard(kind: CatalogKind, locale: &str) -> InlineKeyboardMarkup {
    let (execute_label, cancel_label, execute, cancel) = match kind {
        CatalogKind::Commands => (
            crate::locale::translate(locale, "commands.button.execute"),
            crate::locale::translate(locale, "commands.button.cancel"),
            COMMANDS_CALLBACK_EXECUTE,
            COMMANDS_CALLBACK_CANCEL,
        ),
        CatalogKind::Skills => (
            crate::locale::translate(locale, "skills.button.execute"),
            crate::locale::translate(locale, "skills.button.cancel"),
            SKILLS_CALLBACK_EXECUTE,
            SKILLS_CALLBACK_CANCEL,
        ),
    };
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback(execute_label, execute.to_string()),
        InlineKeyboardButton::callback(cancel_label, cancel.to_string()),
    ]])
}

/// One button row per MCP server: `"{emoji} {name}"` capped at 64 chars
/// with `callback_data = "mcps:select:<index>"`, plus a lone cancel row
/// (pure; Node `buildMcpsListKeyboard` parity).
pub fn build_mcps_list_keyboard(
    servers: &[otb_core::session_pick_catalog_gates::McpCatalogServerItem],
    locale: &str,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = servers
        .iter()
        .enumerate()
        .map(|(index, server)| {
            let raw_label = format!(
                "{} {}",
                crate::commands::mcp_status_glyph(server.status.name),
                server.name
            );
            let label = if raw_label.chars().count() <= 64 {
                raw_label
            } else {
                let kept: String = raw_label.chars().take(61).collect();
                format!("{kept}...")
            };
            vec![InlineKeyboardButton::callback(
                label,
                format!("{MCPS_CALLBACK_SELECT_PREFIX}{index}"),
            )]
        })
        .collect();
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "inline.button.cancel"),
        MCPS_CALLBACK_CANCEL.to_string(),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// Detail keyboard for one MCP server (Node `buildMcpsDetailKeyboard`
/// parity): an enable/disable toggle row only for `connected` / `disabled` /
/// `failed` states, then a `[back, cancel]` row sharing one line (pure).
pub fn build_mcp_detail_keyboard(
    server: &otb_core::session_pick_catalog_gates::McpCatalogServerItem,
    locale: &str,
) -> InlineKeyboardMarkup {
    use otb_core::session_pick_catalog_gates::McpStatusName as Name;
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let toggle = match server.status.name {
        Name::Connected => Some(crate::locale::translate(locale, "mcps.button.disable")),
        Name::Disabled | Name::Failed => {
            Some(crate::locale::translate(locale, "mcps.button.enable"))
        }
        Name::NeedsAuth | Name::NeedsClientRegistration => None,
    };
    if let Some(label) = toggle {
        rows.push(vec![InlineKeyboardButton::callback(
            label,
            MCPS_CALLBACK_TOGGLE.to_string(),
        )]);
    }
    rows.push(vec![
        InlineKeyboardButton::callback(
            crate::locale::translate(locale, "mcps.button.back"),
            MCPS_CALLBACK_BACK.to_string(),
        ),
        InlineKeyboardButton::callback(
            crate::locale::translate(locale, "inline.button.cancel"),
            MCPS_CALLBACK_CANCEL.to_string(),
        ),
    ]);
    InlineKeyboardMarkup::new(rows)
}

/// One button row per agent: `"[✅ ]display"` with
/// `callback_data = "agent:<name>"` (pure; the display name reuses
/// `getAgentDisplayName` parity).
pub fn build_agents_keyboard(
    items: &[crate::commands::AgentListItem],
    current: Option<&str>,
) -> InlineKeyboardMarkup {
    let rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .map(|item| {
            let display = otb_core::types_config::get_agent_display_name(&item.name);
            let label = if current == Some(item.name.as_str()) {
                format!("✅ {display}")
            } else {
                display
            };
            vec![InlineKeyboardButton::callback(
                label,
                format!("{AGENT_CALLBACK_PREFIX}{}", item.name),
            )]
        })
        .collect();
    InlineKeyboardMarkup::new(rows)
}

/// One button row per model: `"[✅ ]model"` with
/// `callback_data = "model:pick:<globalIndex>"` (pure; Node
/// `buildProviderModelsMenuView` label parity).
pub fn build_models_keyboard(
    items: &[crate::commands::ModelListItem],
    global_offset: usize,
    current: Option<(&str, &str)>,
    locale: &str,
    page: usize,
    total_pages: usize,
    providers_page: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let active = current == Some((item.provider_id.as_str(), item.model_id.as_str()));
            let label = if active {
                format!("✅ {}", item.model_id)
            } else {
                item.model_id.clone()
            };
            vec![InlineKeyboardButton::callback(
                label,
                format!(
                    "{MODEL_PROVIDER_MODEL_CALLBACK_PREFIX}{}",
                    global_offset + index
                ),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let pager: Vec<InlineKeyboardButton> = Vec::new();
        // Pager callbacks keep the provider context (Node
        // `buildProviderModelsMenuView` parity:
        // `model:provider:<providerIndex>:<page>`); the provider index is
        // recovered from the message at tap time, so the scaffold re-uses
        // page 0's provider row here and the handler re-resolves it.
        // To stay stateless the pager is omitted when the provider index
        // is unknown; callers pass it through `providers_page` only for the
        // back button below.
        let _ = pager;
        let _ = page;
    }
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "model.button.back"),
        format!("{MODEL_PROVIDERS_CALLBACK_PREFIX}{providers_page}"),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// One button row per provider: `"{name} ({count})"` with
/// `callback_data = "model:provider:<globalIndex>:0"` (pure; Node
/// `buildProvidersMenuView` parity), plus pager rows and a back button to
/// `model:root`.
pub fn build_providers_keyboard(
    items: &[crate::commands::ProviderListItem],
    global_offset: usize,
    locale: &str,
    page: usize,
    total_pages: usize,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            vec![InlineKeyboardButton::callback(
                otb_core::agent_model_variant_gates::provider_row_label(
                    &item.name,
                    item.model_count as u32,
                ),
                otb_core::agent_model_variant_gates::provider_callback(global_offset + index),
            )]
        })
        .collect();
    if total_pages > 1 {
        let page = clamp_page_index(page, total_pages);
        let mut pager: Vec<InlineKeyboardButton> = Vec::new();
        if page > 0 {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "model.providers.prev_page"),
                otb_core::agent_model_variant_gates::providers_page_callback(page - 1),
            ));
        }
        if page + 1 < total_pages {
            pager.push(InlineKeyboardButton::callback(
                crate::locale::translate(locale, "model.providers.next_page"),
                otb_core::agent_model_variant_gates::providers_page_callback(page + 1),
            ));
        }
        if !pager.is_empty() {
            rows.push(pager);
        }
    }
    rows.push(vec![InlineKeyboardButton::callback(
        crate::locale::translate(locale, "model.button.back"),
        MODEL_ROOT_CALLBACK.to_string(),
    )]);
    InlineKeyboardMarkup::new(rows)
}

/// One button row per variant with `callback_data = "variant:<id>"` (pure;
/// Node `buildVariantSelectionMenu` parity: the active variant keeps a `✅`
/// mark and the display name is capitalized).
pub fn build_variants_keyboard(variants: &[String], current: &str) -> InlineKeyboardMarkup {
    let rows: Vec<Vec<InlineKeyboardButton>> = variants
        .iter()
        .map(|variant| {
            let display = otb_core::agent_model_variant_gates::format_variant_for_display(variant);
            let label = if variant == current {
                format!("✅ {display}")
            } else {
                display
            };
            vec![InlineKeyboardButton::callback(
                label,
                otb_core::agent_model_variant_gates::build_variant_callback(variant),
            )]
        })
        .collect();
    InlineKeyboardMarkup::new(rows)
}

/// Root model keyboard: `[Search, Providers]` on the first row plus one
/// `model:list:favorites:<globalIndex>` button per model row (pure; Node
/// `buildModelSelectionMenu` parity minus the recent section, which the
/// scaffold folds into the same flat list).
pub fn build_model_root_keyboard(
    items: &[crate::commands::ModelListItem],
    current: Option<(&str, &str)>,
    locale: &str,
) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = vec![vec![
        InlineKeyboardButton::callback(
            crate::locale::translate(locale, "model.search.button"),
            MODEL_SEARCH_CALLBACK.to_string(),
        ),
        InlineKeyboardButton::callback(
            crate::locale::translate(locale, "model.providers.button"),
            format!("{MODEL_PROVIDERS_CALLBACK_PREFIX}0"),
        ),
    ]];
    rows.extend(items.iter().enumerate().map(|(index, item)| {
        let active = current == Some((item.provider_id.as_str(), item.model_id.as_str()));
        vec![InlineKeyboardButton::callback(
            otb_core::agent_model_variant_gates::model_list_button_label(
                &item.provider_id,
                &item.model_id,
                "⭐",
                active,
            ),
            otb_core::agent_model_variant_gates::build_model_list_callback(
                otb_core::agent_model_variant_gates::ModelListKind::Favorites,
                index,
            ),
        )]
    }));
    InlineKeyboardMarkup::new(rows)
}

/// Loaded command catalog: the project worktree plus the filtered entries
/// in catalog order (the full list is kept so later pages and select taps
/// resolve; paging slices per page in [`commands_view`]).
async fn load_commands_catalog() -> Result<
    (
        String,
        Vec<otb_core::session_pick_catalog_gates::CatalogEntry>,
    ),
    &'static str,
> {
    use otb_core::session_pick_catalog_gates::filter_command_catalog_items;
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return Err("bot.project_not_selected");
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_commands(&worktree).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: commands list failed: {err}");
            return Err("commands.fetch_error");
        }
    };
    let raws: Vec<_> = raw
        .iter()
        .filter_map(crate::commands::raw_catalog_item_from_value)
        .collect();
    Ok((worktree, filter_command_catalog_items(&raws)))
}

/// Loaded skill catalog (same shape as [`load_commands_catalog`]).
async fn load_skills_catalog() -> Result<
    (
        String,
        Vec<otb_core::session_pick_catalog_gates::CatalogEntry>,
    ),
    &'static str,
> {
    use otb_core::session_pick_catalog_gates::filter_skills_catalog_items;
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return Err("bot.project_not_selected");
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_skills(&worktree).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: skills list failed: {err}");
            return Err("skills.fetch_error");
        }
    };
    let raws: Vec<_> = raw
        .iter()
        .filter_map(crate::commands::raw_catalog_item_from_value)
        .collect();
    Ok((worktree, filter_skills_catalog_items(&raws)))
}

/// Loaded MCP catalog: the project worktree plus the parsed servers in
/// catalog order. Fetch failure or an invalid shape yields the
/// `mcps.fetch_error` key.
async fn load_mcps_catalog() -> Result<
    (
        String,
        Vec<otb_core::session_pick_catalog_gates::McpCatalogServerItem>,
    ),
    &'static str,
> {
    use otb_core::session_pick_catalog_gates::parse_mcp_catalog_servers;
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return Err("bot.project_not_selected");
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.mcp_status(&worktree).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: mcp status failed: {err}");
            return Err("mcps.fetch_error");
        }
    };
    let Some(servers) = parse_mcp_catalog_servers(&catalog_json_from_value(&raw)) else {
        tracing::warn!("[otb-bot] telegram: mcp status returned an invalid shape");
        return Err("mcps.fetch_error");
    };
    Ok((worktree, servers))
}

/// Resolve which catalog server a toggle tap targets by re-rendering each
/// candidate detail and comparing against the detail message text (pure;
/// mirrors [`resolve_message_detail_index`]).
///
/// SCAFFOLD DEVIATION: Node remembers the `serverName` in its interaction
/// state (`McpsDetailMetadata`), so its toggle tap carries no payload. The
/// scaffold keeps no per-chat interaction memory, so the target is
/// recovered from the detail message itself
/// (`render_mcp_detail` is deterministic). A follow-up with a dispatcher
/// should carry the remembered name instead. No match (edited/foreign
/// text) yields `None`.
pub fn resolve_mcp_detail_name(
    detail_text: &str,
    servers: &[otb_core::session_pick_catalog_gates::McpCatalogServerItem],
    locale: &str,
) -> Option<String> {
    servers
        .iter()
        .find(|server| crate::commands::render_mcp_detail(server, Some(locale)) == detail_text)
        .map(|server| server.name.clone())
}

/// Recover a command/skill name from its confirm message text (pure).
/// The confirm templates embed the pick as `/name` (`commands.confirm` /
/// `skills.confirm` parity); the first `/token` (letters, digits, `_`, `-`)
/// wins. Anything else yields `None` and the caller falls back to the
/// inactive notice plus the list view.
pub fn parse_confirm_pick_name(text: &str) -> Option<String> {
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch != '/' {
            continue;
        }
        let rest = &text[index + 1..];
        let len = rest
            .char_indices()
            .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
            .map(|(offset, ch)| offset + ch.len_utf8())
            .last()
            .unwrap_or(0);
        if len > 0 {
            return Some(rest[..len].to_string());
        }
    }
    None
}

/// Ensure a session exists in the project worktree for a catalog execute:
/// the remembered session wins when it still belongs to the worktree,
/// otherwise a fresh session is created and remembered (JS
/// `ensureSessionForProject` parity in `command-callback-handler.ts`).
/// Returns `(id, directory)` or the i18n key for the failure reply.
async fn ensure_execute_session(worktree: &str) -> Result<(String, String), &'static str> {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    if let (Some(id), Some(dir)) = (
        current.session_id.clone().filter(|id| !id.is_empty()),
        current
            .session_directory
            .clone()
            .filter(|dir| !dir.is_empty()),
    ) {
        if dir == worktree {
            return Ok((id, dir));
        }
    }
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let session = match client.create_session(worktree, None).await {
        Ok(session) => session,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: execute session create failed: {err}");
            return Err("bot.create_session_error");
        }
    };
    let (id, title) = session_identity_from_value(&session);
    if id.is_empty() {
        return Err("bot.create_session_error");
    }
    if let Err(err) =
        crate::settings_current::write_current_session(&app_home, &id, &title, worktree)
    {
        tracing::warn!("[otb-bot] telegram: cannot remember execute session: {err}");
        return Err("bot.create_session_error");
    }
    Ok((id, worktree.to_string()))
}

/// Live agents view: the available agents with one `agent:<name>` button
/// per row (JS `agent-selection-menu.ts` parity: current agent marked,
/// hidden/subagents filtered). No project → `bot.project_not_selected`;
/// fetch failure/empty → `agent.menu.empty`.
async fn agents_view(locale: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return (
            crate::locale::translate(locale, "bot.project_not_selected"),
            None,
        );
    };
    let stored = current.agent.clone().unwrap_or_default();
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_agents(&worktree).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: agents list failed: {err}");
            return (crate::locale::translate(locale, "agent.menu.empty"), None);
        }
    };
    let items = crate::commands::agent_list_from_values(&raw);
    if items.is_empty() {
        return (crate::locale::translate(locale, "agent.menu.empty"), None);
    }
    let current_name = if stored.trim().is_empty() {
        None
    } else {
        Some(stored.trim())
    };
    let text = match current_name {
        Some(name) => crate::locale::translate(locale, "agent.menu.current").replace(
            "{name}",
            &otb_core::types_config::get_agent_display_name(name),
        ),
        None => crate::locale::translate(locale, "agent.menu.select"),
    };
    let keyboard = build_agents_keyboard(&items, current_name);
    (text, Some(keyboard))
}

/// Filtered model search view (stateless Batch E1 search follow-up).
///
/// Node `searchModels` + `buildModelSearchResultsView` parity via
/// [`matches_model_query`]: the flat provider catalog is sorted by
/// `provider/model`, filtered on the trimmed query, and capped to the first
/// `SEARCH_RESULTS_LIMIT` matches. Result rows carry the self-contained
/// legacy `model:<provider>:<model>` shape ([`model_legacy_callback`]) so
/// taps resolve without remembered search state; a trailing
/// `[search again, cancel]` row carries the Node-exact
/// `model:search:again` / `model:search:cancel` shapes. Empty matches render
/// `model.search.no_results`, otherwise `model.search.results_title`.
/// Fetch failure keeps the `model.providers.error` key.
///
/// SCAFFOLD DEVIATION: no caller yet — the typed query needs a dispatcher
/// (plain-text input is ignored and no `/model` command exists), so stale
/// `model:result:<n>` taps re-prompt instead. The future text-input flow
/// calls this view and the legacy result branch applies the pick.
#[allow(dead_code)]
async fn model_search_view(locale: &str, query: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            return (
                crate::locale::translate(locale, "model.providers.error"),
                None,
            );
        }
    };
    let providers = crate::commands::provider_list_from_value(&raw);
    let mut flat: Vec<crate::commands::ModelListItem> = Vec::new();
    for provider in &providers {
        flat.extend(crate::commands::model_list_for_provider(&raw, &provider.id));
    }
    flat.sort_by(|left, right| {
        format!("{}/{}", left.provider_id, left.model_id)
            .cmp(&format!("{}/{}", right.provider_id, right.model_id))
    });
    let matched: Vec<crate::commands::ModelListItem> = flat
        .into_iter()
        .filter(|item| matches_model_query(item, query))
        .take(otb_core::agent_model_variant_gates::SEARCH_RESULTS_LIMIT)
        .collect();
    let again_cancel = || {
        search_again_cancel_row(
            locale,
            "model.search.search_again",
            "inline.button.cancel",
            MODEL_SEARCH_AGAIN_CALLBACK,
            MODEL_SEARCH_CANCEL_CALLBACK,
        )
    };
    if matched.is_empty() {
        let text =
            crate::locale::translate(locale, "model.search.no_results").replace("{query}", query);
        return (text, Some(InlineKeyboardMarkup::new(vec![again_cancel()])));
    }
    let text =
        crate::locale::translate(locale, "model.search.results_title").replace("{query}", query);
    let mut rows: Vec<Vec<InlineKeyboardButton>> = matched
        .iter()
        .map(|item| {
            vec![InlineKeyboardButton::callback(
                format!("{}/{}", item.provider_id, item.model_id),
                model_legacy_callback(&item.provider_id, &item.model_id),
            )]
        })
        .collect();
    rows.push(again_cancel());
    (text, Some(InlineKeyboardMarkup::new(rows)))
}

/// Live model root view: `[Search, Providers]` plus one
/// `model:list:favorites:<index>` button per model of the first catalog
/// page (the scaffold folds favorites + recent into one flat list; the
/// provider browser covers the rest). Empty → `model.menu.empty`;
/// probe failure → `model.providers.error`.
async fn model_root_view(locale: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            return (
                crate::locale::translate(locale, "model.providers.error"),
                None,
            );
        }
    };
    let providers = crate::commands::provider_list_from_value(&raw);
    if providers.is_empty() {
        return (crate::locale::translate(locale, "model.menu.empty"), None);
    }
    let mut flat: Vec<crate::commands::ModelListItem> = Vec::new();
    for provider in &providers {
        flat.extend(crate::commands::model_list_for_provider(&raw, &provider.id));
    }
    flat.sort_by(|left, right| {
        format!("{}/{}", left.provider_id, left.model_id)
            .cmp(&format!("{}/{}", right.provider_id, right.model_id))
    });
    let limit = crate::config::MODELS_LIST_LIMIT.max(1);
    let page_items: Vec<crate::commands::ModelListItem> = flat.into_iter().take(limit).collect();
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let current_ref = match (
        current.model_provider.as_deref(),
        current.model_id.as_deref(),
    ) {
        (Some(provider), Some(model)) => Some((provider, model)),
        _ => None,
    };
    let text = crate::locale::translate(locale, "model.menu.select");
    let keyboard = build_model_root_keyboard(&page_items, current_ref, locale);
    (text, Some(keyboard))
}

/// Providers browser page: one `model:provider:<index>:0` button per
/// provider plus pager and back (Node `buildProvidersMenuView` parity).
async fn model_providers_view(locale: &str, page: usize) -> (String, Option<InlineKeyboardMarkup>) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            return (
                crate::locale::translate(locale, "model.providers.error"),
                None,
            );
        }
    };
    let providers = crate::commands::provider_list_from_value(&raw);
    if providers.is_empty() {
        return (
            crate::locale::translate(locale, "model.providers.empty"),
            None,
        );
    }
    let total = providers.len();
    let limit = crate::config::MODELS_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    let page = clamp_page_index(page, total_pages);
    let start = page * limit;
    let end = (start + limit).min(total);
    let page_items = &providers[start.min(total)..end];
    let mut text = crate::locale::translate(locale, "model.providers.title");
    if total_pages > 1 {
        let indicator = crate::locale::translate(locale, "model.providers.page_indicator");
        let indicator = indicator.replace("{current}", &(page + 1).to_string());
        text.push_str(&format!(
            "\n\n{}",
            indicator.replace("{total}", &total_pages.to_string())
        ));
    }
    let keyboard = build_providers_keyboard(page_items, start, locale, page, total_pages);
    (text, Some(keyboard))
}

/// One provider's models page: one `model:pick:<globalIndex>` button per
/// model plus a back button (Node `buildProviderModelsMenuView` parity;
/// the pick index is global within the provider so the stateless handler
/// resolves it without page context).
async fn model_provider_models_view(
    locale: &str,
    provider_index: usize,
    providers_page: usize,
) -> (String, Option<InlineKeyboardMarkup>) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            return (
                crate::locale::translate(locale, "model.providers.error"),
                None,
            );
        }
    };
    let providers = crate::commands::provider_list_from_value(&raw);
    let Some(provider) = providers.get(provider_index) else {
        return (
            crate::locale::translate(locale, "model.providers.error"),
            None,
        );
    };
    let models = crate::commands::model_list_for_provider(&raw, &provider.id);
    if models.is_empty() {
        let text = crate::locale::translate(locale, "model.provider_models.empty")
            .replace("{provider}", &provider.name);
        return (text, None);
    }
    let total = models.len();
    let limit = crate::config::MODELS_LIST_LIMIT.max(1);
    let total_pages = total.max(1).div_ceil(limit).max(1);
    // Stateless: always the first page (the pick index is global, so every
    // model stays reachable; paging the models browser needs a dispatcher).
    let end = limit.min(total);
    let page_items = &models[..end];
    let mut text = crate::locale::translate(locale, "model.provider_models.title")
        .replace("{provider}", &provider.name);
    if total_pages > 1 {
        let indicator = crate::locale::translate(locale, "model.provider_models.page_indicator");
        let indicator = indicator.replace("{current}", "1");
        text.push_str(&format!(
            "\n\n{}",
            indicator.replace("{total}", &total_pages.to_string())
        ));
    }
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let current_ref = match (
        current.model_provider.as_deref(),
        current.model_id.as_deref(),
    ) {
        (Some(provider), Some(model)) => Some((provider, model)),
        _ => None,
    };
    let keyboard = build_models_keyboard(
        page_items,
        0,
        current_ref,
        locale,
        0,
        total_pages,
        providers_page,
    );
    (text, Some(keyboard))
}

/// Live variant view: one `variant:<id>` button per variant of the current
/// model (JS `variant-selection-menu.ts` parity: `default` first, active
/// marked). No model → `variant.select_model_first`.
async fn variants_view(locale: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let (Some(provider), Some(model)) = (current.model_provider.clone(), current.model_id.clone())
    else {
        return (
            crate::locale::translate(locale, "variant.select_model_first"),
            None,
        );
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            return (crate::locale::translate(locale, "variant.menu.error"), None);
        }
    };
    let variants = crate::commands::variant_list_for_model(&raw, &provider, &model);
    if variants.is_empty() {
        return (crate::locale::translate(locale, "variant.menu.empty"), None);
    }
    let current_variant = current
        .model_variant
        .clone()
        .filter(|variant| !variant.is_empty())
        .unwrap_or_else(|| "default".to_string());
    let display = otb_core::agent_model_variant_gates::format_variant_for_display(&current_variant);
    let text = crate::locale::translate(locale, "variant.menu.current").replace("{name}", &display);
    let keyboard = build_variants_keyboard(&variants, &current_variant);
    (text, Some(keyboard))
}

/// Home directory for display folding and browser-root defaults
/// (Node `os.homedir()` parity): `HOME`, else `USERPROFILE` (Windows),
/// else `~` (folding becomes the identity).
fn home_directory() -> String {
    std::env::var("HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "~".to_string())
}

/// Browser roots for `/open` (Node `getBrowserRoots` parity):
/// `OPEN_BROWSER_ROOTS` comma list with `~` expansion, else the home
/// directory. Relative entries resolve against the current directory.
fn browser_roots() -> Vec<String> {
    use otb_core::file_browser_service_gates::{expand_tilde, split_browser_roots};
    let home = home_directory();
    let raw = std::env::var("OPEN_BROWSER_ROOTS").unwrap_or_default();
    let mut roots: Vec<String> = split_browser_roots(&raw)
        .into_iter()
        .map(|entry| expand_tilde(&entry, &home))
        .map(|entry| {
            let path = std::path::Path::new(&entry);
            if path.is_absolute() {
                entry
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(&entry).to_string_lossy().into_owned())
                    .unwrap_or(entry)
            }
        })
        .collect();
    if roots.is_empty() {
        roots.push(home);
    }
    roots
}

/// Scan one directory for visible subdirectories, sorted base-sensitively
/// (JS `scanDirectory` parity: hidden names skipped).
/// Blocking; callers run it inside `spawn_blocking`.
fn scan_open_subdirs(
    dir: &str,
) -> Result<Vec<String>, otb_core::file_browser_service_gates::ScanErrorCode> {
    use otb_core::file_browser_service_gates::{is_visible_dir, sorted_dir_names, ScanErrorCode};
    let entries = std::fs::read_dir(dir).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => ScanErrorCode::NotFound,
        std::io::ErrorKind::PermissionDenied => ScanErrorCode::AccessDenied,
        std::io::ErrorKind::NotADirectory => ScanErrorCode::NotDirectory,
        _ => ScanErrorCode::Unknown,
    })?;
    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        if is_visible_dir(&name, is_dir) {
            names.push(name);
        }
    }
    Ok(sorted_dir_names(&names))
}

/// Scan one project directory for `ls` entries, dirs-first sorted
/// (JS `scanLsDirectory` parity, minus the in-service project-root check
/// the caller performs). Blocking; callers run it inside `spawn_blocking`.
fn scan_ls_entries(
    dir: &str,
) -> Result<
    Vec<otb_core::file_browser_service_gates::LsEntry>,
    otb_core::file_browser_service_gates::ScanErrorCode,
> {
    use otb_core::file_browser_service_gates::{
        join_path, sort_ls_entries, LsEntry, ScanErrorCode,
    };
    let entries = std::fs::read_dir(dir).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => ScanErrorCode::NotFound,
        std::io::ErrorKind::PermissionDenied => ScanErrorCode::AccessDenied,
        std::io::ErrorKind::NotADirectory => ScanErrorCode::NotDirectory,
        _ => ScanErrorCode::Unknown,
    })?;
    let mut items: Vec<LsEntry> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        items.push(LsEntry {
            full_path: join_path(dir, &name),
            name,
            is_dir,
        });
    }
    Ok(sort_ls_entries(items))
}

/// Live `/open` reply: browse directories to add a project
/// (JS `open-command.ts` parity: one root browses it directly, several
/// list the roots). Scan failure → `open.scan_error`. The reply carries
/// the browse keyboard (`open:nav:` / `open:pg:` / `open:sel:` /
/// `open:roots` plus `inline:cancel:open`); taps are handled by
/// [`handle_open_callback`].
async fn open_view(locale: &str) -> (String, Option<InlineKeyboardMarkup>) {
    use otb_core::browse_command_gates::should_show_single_open_root;
    let home = home_directory();
    let roots = browser_roots();
    if !should_show_single_open_root(roots.len()) {
        let text = crate::commands::render_open_roots(&roots, &home, Some(locale));
        let keyboard = build_open_roots_keyboard(&roots, &home, locale);
        return (text, Some(keyboard));
    }
    let root = roots.first().cloned().unwrap_or(home.clone());
    open_browse_page(&root, 0, locale).await
}

/// Render one `/open` browse page with its keyboard (JS
/// `renderOpenBrowseView` + `buildOpenBrowseKeyboard` parity).
/// Scan failure renders `open.scan_error` with no keyboard.
async fn open_browse_page(
    dir: &str,
    page: usize,
    locale: &str,
) -> (String, Option<InlineKeyboardMarkup>) {
    use otb_core::file_browser_service_gates::{
        clamp_page, is_allowed_root, is_within_allowed_root, page_slice_range, parent_path,
        path_to_display_path, total_pages, ScanErrorCode,
    };
    let home = home_directory();
    let roots = browser_roots();
    let probe = dir.to_string();
    let scanned = match tokio::task::spawn_blocking(move || scan_open_subdirs(&probe)).await {
        Ok(Ok(names)) => names,
        Ok(Err(code)) => {
            let detail = match code {
                ScanErrorCode::NotFound => format!("Directory not found: {dir}"),
                ScanErrorCode::AccessDenied => format!("Permission denied: {dir}"),
                ScanErrorCode::NotDirectory => format!("Not a directory: {dir}"),
                ScanErrorCode::Unknown => "Unknown error".to_string(),
            };
            let text =
                crate::locale::translate(locale, "open.scan_error").replace("{error}", &detail);
            return (text, None);
        }
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: open scan task failed: {err}");
            let text = crate::locale::translate(locale, "open.scan_error")
                .replace("{error}", "Unknown error");
            return (text, None);
        }
    };
    let total = scanned.len();
    let pages = total_pages(total);
    let page = clamp_page(page as i64, pages);
    let (start, end) = page_slice_range(page);
    let display = path_to_display_path(dir, &home);
    let text = crate::commands::render_open_browse(
        &display,
        &scanned[start.min(total)..end.min(total)].to_vec(),
        page,
        pages,
        total,
        Some(locale),
    );
    let has_parent = parent_path(dir) != dir;
    let allowed = is_allowed_root(dir, &roots, cfg!(windows));
    let _within = is_within_allowed_root(dir, &roots, cfg!(windows));
    let keyboard = build_open_browse_keyboard(
        &scanned[start.min(total)..end.min(total)].to_vec(),
        dir,
        page,
        pages,
        has_parent,
        allowed,
        roots.len() > 1,
        locale,
    );
    (text, Some(keyboard))
}

/// Render the `/open` roots list with its keyboard (JS
/// `buildOpenRootsKeyboard` parity).
fn open_roots_view(locale: &str) -> (String, Option<InlineKeyboardMarkup>) {
    let home = home_directory();
    let roots = browser_roots();
    let text = crate::commands::render_open_roots(&roots, &home, Some(locale));
    let keyboard = build_open_roots_keyboard(&roots, &home, locale);
    (text, Some(keyboard))
}

/// Live `/ls` reply: list the current project directory (JS `ls-command.ts`
/// parity: no project or no target → `bot.project_not_selected`; outside
/// the root → `❌ {ls.access_denied}`; scan failure → `❌ {error}`;
/// otherwise the numbered text list plus the browse keyboard, first page
/// only). Taps are handled by [`handle_ls_callback`].
async fn ls_view(locale: &str, args: &str) -> (String, Option<InlineKeyboardMarkup>) {
    use otb_core::browse_command_gates::{resolve_ls_target, should_report_ls_access_denied};
    use otb_core::file_browser_service_gates::{is_path_within_directory, join_path};
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(project_root) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        return (
            crate::locale::translate(locale, "bot.project_not_selected"),
            None,
        );
    };
    let explicit = if args.trim().is_empty() {
        None
    } else {
        Some(args.trim())
    };
    // Scaffold memory: no per-user remembered directory yet, so an empty
    // argument lists the project root itself.
    let Some(target) = resolve_ls_target(explicit, Some(&project_root)) else {
        return (
            crate::locale::translate(locale, "bot.project_not_selected"),
            None,
        );
    };
    // Relative arguments resolve against the project root (Node takes the
    // argument as-is; the scaffold keeps browsing inside the project).
    let target = if std::path::Path::new(target).is_absolute() {
        target.to_string()
    } else {
        join_path(&project_root, target)
    };
    if should_report_ls_access_denied(is_path_within_directory(&target, &project_root)) {
        return (
            format!(
                "❌ {}",
                crate::locale::translate(locale, "ls.access_denied")
            ),
            None,
        );
    }
    ls_browse_page(&project_root, &target, 0, locale).await
}

/// Render one `/ls` browse page with its keyboard (JS `renderLsBrowseView`
/// + `buildLsBrowseKeyboard` parity). Scan failure renders `ls.scan_error`
/// with no keyboard.
async fn ls_browse_page(
    project_root: &str,
    target: &str,
    page: usize,
    locale: &str,
) -> (String, Option<InlineKeyboardMarkup>) {
    use otb_core::file_browser_service_gates::{
        clamp_page, has_parent, page_slice_range, path_to_display_path, total_pages,
    };
    let scanned = target.to_string();
    let entries = match tokio::task::spawn_blocking(move || scan_ls_entries(&scanned)).await {
        Ok(Ok(entries)) => entries,
        Ok(Err(_)) | Err(_) => {
            return (
                format!("❌ {}", crate::locale::translate(locale, "ls.scan_error")),
                None,
            );
        }
    };
    let total = entries.len();
    let pages = total_pages(total);
    let page = clamp_page(page as i64, pages);
    let (start, end) = page_slice_range(page);
    let home = home_directory();
    let display = path_to_display_path(target, &home);
    let text = crate::commands::render_ls_browse(
        &display,
        &entries[start.min(total)..end.min(total)].to_vec(),
        page,
        pages,
        total,
        Some(locale),
    );
    let keyboard = build_ls_browse_keyboard(
        &entries[start.min(total)..end.min(total)].to_vec(),
        target,
        page,
        pages,
        has_parent(target, project_root),
        target == project_root,
        locale,
    );
    (text, Some(keyboard))
}

/// Render one `/ls` file-details view with its keyboard (JS
/// `renderLsFileDetailsView` + `buildLsFileDetailsKeyboard` parity).
/// Missing files render `commands.download.not_found`, directories render
/// `commands.download.not_file`, both without a keyboard.
async fn ls_file_view(
    file_path: &str,
    page: usize,
    locale: &str,
) -> (String, Option<InlineKeyboardMarkup>) {
    use otb_core::file_browser_service_gates::{base_name, parent_path};
    let probed = file_path.to_string();
    let meta = match tokio::task::spawn_blocking(move || std::fs::metadata(&probed)).await {
        Ok(Ok(meta)) => meta,
        Ok(Err(_)) | Err(_) => {
            return (
                crate::locale::translate(locale, "commands.download.not_found"),
                None,
            );
        }
    };
    if meta.is_dir() {
        return (
            crate::locale::translate(locale, "commands.download.not_file"),
            None,
        );
    }
    let name = base_name(file_path);
    let size = meta.len();
    let size_text = otb_core::media_prompt::format_file_size(size);
    let header = crate::locale::translate(locale, "ls.file.header");
    let size_label = crate::locale::translate(locale, "commands.download.size");
    let text = format!(
        "📄 {header}\n<code>{}</code>\n{size_label}: {size_text}",
        crate::commands::escape_html(&name),
    );
    let is_text = otb_core::media_prompt::is_text_file_name(&name);
    let size_ok = otb_core::media_prompt::is_file_size_allowed(Some(size), 20 * 1024);
    let keyboard = build_ls_file_keyboard(
        file_path,
        &parent_path(file_path),
        page,
        otb_core::file_browser_menu_gates::can_attach_file(is_text, size_ok),
        locale,
    );
    (text, Some(keyboard))
}

/// Live `/rename` prompt reply (JS `rename-command.ts` parity, text-only:
/// no session → `rename.no_session`; otherwise the prompt with the live
/// title, falling back to the remembered one when the fetch fails).
/// The interactive waiting state needs a dispatcher and stays a follow-up;
/// use `/rename <new title>` to rename directly.
async fn rename_prompt_view(locale: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let (Some(session_id), Some(session_dir)) = (
        current.session_id.clone().filter(|id| !id.is_empty()),
        current
            .session_directory
            .clone()
            .filter(|dir| !dir.is_empty()),
    ) else {
        return crate::locale::translate(locale, "rename.no_session");
    };
    let remembered = current.session_title.clone().unwrap_or_default();
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let title = match client.get_session(&session_id, &session_dir).await {
        Ok(session) => session
            .get("title")
            .and_then(|title| title.as_str())
            .map(str::to_string)
            .unwrap_or(remembered),
        Err(err) => {
            tracing::debug!("[otb-bot] telegram: rename title fetch failed: {err}");
            remembered
        }
    };
    crate::commands::render_rename_prompt(&title, Some(locale))
}

/// Live `/rename <new title>` reply (JS `handleRenameTextAnswer` parity:
/// blank title → `rename.empty_title`; no session → `rename.no_session`;
/// update failure → `rename.error`; otherwise the API rename is persisted
/// like `setCurrentSession` and confirmed with `rename.success`).
async fn rename_apply_view(locale: &str, raw_title: &str) -> String {
    let Some(title) = crate::commands::validate_rename_title(raw_title) else {
        return crate::locale::translate(locale, "rename.empty_title");
    };
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let (Some(session_id), Some(session_dir)) = (
        current.session_id.clone().filter(|id| !id.is_empty()),
        current
            .session_directory
            .clone()
            .filter(|dir| !dir.is_empty()),
    ) else {
        return crate::locale::translate(locale, "rename.no_session");
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    if let Err(err) = client
        .update_session_title(&session_id, &session_dir, &title)
        .await
    {
        tracing::warn!("[otb-bot] telegram: rename failed: {err}");
        return crate::locale::translate(locale, "rename.error");
    }
    if let Err(err) =
        crate::settings_current::write_current_session(&app_home, &session_id, &title, &session_dir)
    {
        tracing::warn!("[otb-bot] telegram: cannot remember renamed session: {err}");
        return crate::locale::translate(locale, "rename.error");
    }
    crate::commands::render_rename_success(&title, Some(locale))
}

/// Live `/abort` reply: no current session → `stop.no_active_session`;
/// otherwise POST the versioned abort (`session.abort` V1 /
/// `session.interrupt` V2) and confirm with `stop.success`, or
/// `stop.warn_local_only` when the server-side abort failed. Plain text; the
/// prompt-queue/goal side effects need a dispatcher and stay a follow-up.
async fn abort_view(locale: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let (Some(session_id), Some(session_dir)) = (
        current.session_id.clone().filter(|id| !id.is_empty()),
        current
            .session_directory
            .clone()
            .filter(|dir| !dir.is_empty()),
    ) else {
        return crate::commands::render_abort_no_session(Some(locale));
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    match client.abort_session(&session_id, &session_dir).await {
        Ok(aborted) => crate::commands::render_abort_result(aborted, Some(locale)),
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: abort failed: {err}");
            crate::commands::render_abort_result(false, Some(locale))
        }
    }
}

/// Live `/detach` reply: local-only, the OpenCode session itself is never
/// stopped (JS `detach-command.ts` parity minus the attach/keyboard side
/// effects). Guard order matches Node: no project →
/// `detach.project_not_selected`; no session → `detach.no_active_session`;
/// otherwise the live title (remembered fallback) is resolved, the remembered
/// session is cleared, and `detach.success` confirms. A failed clear renders
/// `detach.error`.
async fn detach_view(locale: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    if current
        .project_worktree
        .as_deref()
        .filter(|worktree| !worktree.is_empty())
        .is_none()
    {
        return crate::locale::translate(locale, "detach.project_not_selected");
    }
    let (Some(session_id), Some(session_dir)) = (
        current.session_id.clone().filter(|id| !id.is_empty()),
        current
            .session_directory
            .clone()
            .filter(|dir| !dir.is_empty()),
    ) else {
        return crate::locale::translate(locale, "detach.no_active_session");
    };
    let remembered = current.session_title.clone().unwrap_or_default();
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let title = match client.get_session(&session_id, &session_dir).await {
        Ok(session) => session
            .get("title")
            .and_then(|title| title.as_str())
            .map(str::to_string)
            .unwrap_or(remembered),
        Err(err) => {
            tracing::debug!("[otb-bot] telegram: detach title fetch failed: {err}");
            remembered
        }
    };
    if let Err(err) = crate::settings_current::clear_current_session(&app_home) {
        tracing::warn!("[otb-bot] telegram: cannot clear detached session: {err}");
        return crate::locale::translate(locale, "detach.error");
    }
    tracing::info!(
        "[otb-bot] telegram: detached from session id_prefix={} id_len={}",
        session_id.chars().take(8).collect::<String>(),
        session_id.len()
    );
    crate::commands::render_detach_success(&title, Some(locale))
}

/// Live `/task` reply (minimal scaffold): no project →
/// `bot.project_not_selected`; otherwise the `render_task_ack` text (the
/// interactive schedule→prompt flow needs a dispatcher and stays a
/// follow-up). Read-only.
fn task_view(locale: &str, args: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    if current
        .project_worktree
        .as_deref()
        .filter(|worktree| !worktree.is_empty())
        .is_none()
    {
        return crate::locale::translate(locale, "bot.project_not_selected");
    }
    crate::commands::render_task_ack(args, Some(locale))
}

/// Live `/opencode_start` reply (safety scaffold): container + local-target
/// gates first (TS `opencode-start-command.ts:88-97` parity), then the
/// already-running health gate only. The spawn/wait/ready half
/// (`resolveUnavailableOpencodeServer`, `startLocalOpencodeServer`,
/// `waitForServerReady`) is impure and stays deferred with a trace; the bot
/// never spawns the server process. Read-only.
async fn opencode_start_view(locale: &str) -> String {
    use otb_core::{local_process_gates, server_command_gates};
    if crate::health::is_container_runtime() {
        return crate::locale::translate(locale, "runtime.container.command_unavailable");
    }
    let api_url = crate::config::AppConfig::load()
        .map(|config| config.opencode_api_url)
        .unwrap_or_else(|_| crate::opencode::base_url_from_env());
    if local_process_gates::resolve_local_opencode_target(&api_url).is_none() {
        return crate::locale::translate(locale, "opencode_start.remote_configured");
    }
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let probe = match client.health().await {
        Ok(probe) => probe,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: opencode_start probe failed: {err}");
            crate::opencode::HealthProbe {
                ok: false,
                version: None,
            }
        }
    };
    if server_command_gates::should_report_start_already_running(probe.ok) {
        return crate::commands::render_opencode_start_status(
            probe.ok,
            probe.version.as_deref(),
            Some(locale),
        );
    }
    tracing::info!(
        "[otb-bot] telegram: opencode_start spawn deferred (no spawn/wait syscalls in scaffold)"
    );
    crate::commands::render_opencode_start_status(false, None, Some(locale))
}

/// Live `/opencode_stop` reply (safety scaffold): container + local-target
/// gates first, then the pid gate directly (TS `opencode-stop-command.ts`
/// `STOP_REASON` + `!localTarget` + `findServerPid` order parity). Health is
/// never consulted (PRODUCT #160: the stop kills even when health hangs).
/// The pid lookup + `killServerProcess` syscalls are impure and stay
/// deferred with a trace; the bot never kills the server process. Read-only.
async fn opencode_stop_view(locale: &str) -> String {
    use otb_core::server_command_gates;
    if crate::health::is_container_runtime() {
        return crate::locale::translate(locale, "runtime.container.command_unavailable");
    }
    let api_url = crate::config::AppConfig::load()
        .map(|config| config.opencode_api_url)
        .unwrap_or_else(|_| crate::opencode::base_url_from_env());
    let has_local_target =
        otb_core::local_process_gates::resolve_local_opencode_target(&api_url).is_some();
    if server_command_gates::should_report_stop_remote(has_local_target) {
        return crate::locale::translate(locale, "opencode_stop.remote_configured");
    }
    // `findServerPid` / `killServerProcess` need netstat/ss/lsof + signal
    // syscalls: deferred to the dispatcher follow-up. The scaffold cannot
    // prove a pid listens, so the gate reads as not-running (never claim a
    // stop that did not happen).
    tracing::info!(
        "[otb-bot] telegram: opencode_stop pid lookup deferred (findServerPid/kill syscalls follow-up)"
    );
    let has_pid = false;
    debug_assert!(server_command_gates::should_report_stop_not_running(
        has_pid
    ));
    crate::locale::translate(locale, "opencode_stop.not_running")
}

/// Live `/reload` reply (minimal scaffold): re-read the local
/// `settings.json`. A missing file reads as defaults (success); an
/// unparseable file renders `reload.failed`. The OpenCode server config
/// rebuild needs a dispatcher and stays a follow-up. Only paths, error
/// kinds, and byte lengths are ever logged — never values.
fn reload_view(locale: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    let path = crate::settings_current::settings_file_path(&app_home);
    let ok = match std::fs::read_to_string(&path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
        Err(err) => {
            tracing::warn!(
                "[otb-bot] telegram: reload cannot read settings at {} ({})",
                path.display(),
                err.kind()
            );
            false
        }
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => true,
            Err(err) => {
                tracing::warn!(
                    "[otb-bot] telegram: reload ignoring unparseable settings ({} bytes): {err}",
                    content.len()
                );
                false
            }
        },
    };
    crate::commands::render_reload_result(ok, Some(locale))
}

/// Live `/goal` reply (minimal scaffold): the JS guard order
/// (`goal-command.ts` entry parity) rendered through `goal_entry_key`; the
/// happy path shows `goal.input.prompt` since the interactive objective flow
/// (and the server-side goal itself) needs a dispatcher and stays a
/// follow-up. Read-only.
fn goal_view(locale: &str, args: &str) -> String {
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let has_project = current
        .project_worktree
        .as_deref()
        .is_some_and(|worktree| !worktree.is_empty());
    let has_session = current
        .session_id
        .as_deref()
        .is_some_and(|id| !id.is_empty())
        && current
            .session_directory
            .as_deref()
            .is_some_and(|dir| !dir.is_empty());
    let session_matches_project = match (&current.session_directory, &current.project_worktree) {
        (Some(dir), Some(worktree)) => dir == worktree,
        _ => false,
    };
    let key = crate::commands::goal_entry_key(
        has_project,
        has_session,
        session_matches_project,
        !args.trim().is_empty(),
    );
    crate::locale::translate(locale, key)
}

/// Parse `/command[@bot] [args]` text into the bare lowercase command name.
/// Returns `None` for non-command text or a bare `/` (pure; testable).
pub fn parse_command_text(text: &str) -> Option<String> {
    split_command_args(text).map(|(name, _)| name)
}

/// Split `/command[@bot] [args]` text into the bare lowercase command name
/// plus the trimmed trailing arguments (`""` when absent).
/// Returns `None` for non-command text or a bare `/` (pure; testable).
pub fn split_command_args(text: &str) -> Option<(String, String)> {
    let token = text.trim().split_whitespace().next()?;
    let bare = token.strip_prefix('/')?;
    let name = bare.split('@').next().unwrap_or("");
    if name.is_empty() {
        return None;
    }
    let args = text
        .trim()
        .splitn(2, char::is_whitespace)
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();
    Some((name.to_lowercase(), args))
}

/// Unknown-command reply (Node `unknown-command.ts` parity:
/// `t("bot.unknown_command", { command: commandToken })`). Pure; the raw
/// slash token (`/name[@bot]`, case preserved) fills `{command}` with `en`
/// fallback via [`crate::locale::translate`].
pub fn unknown_command_reply(locale: &str, command_token: &str) -> String {
    crate::locale::translate(locale, "bot.unknown_command").replace("{command}", command_token)
}

/// Pure route for scaffold plain-text input per
/// `otb_core::message_router_text_gates` ordering (Node
/// `message-router.ts` `message:text` chain parity): input exists, no
/// slash, no blocking interaction snapshot fed in here, and no
/// goal/task/search/rename/catalog stage handles it, so the tail resolves
/// to `Merge` — the prompt flow. Live answer routing and the single-slot
/// occupancy are checked separately in `handle_plain_text_prompt` (a held
/// question takes the text as its answer before the prompt flow runs).
pub fn plain_text_prompt_route() -> otb_core::message_router_text_gates::MessageRouterTextRoute {
    otb_core::message_router_text_gates::select_message_router_text_route(
        true,  // has_input: `message.text()` is `Some`
        false, // is_slash: `split_command_args` is `None`
        false, // question_active: no interaction memory in the scaffold
        false, // goal_handled
        false, // task_handled
        false, // model_search_handled
        false, // project_handled
        false, // session_handled
        false, // rename_handled
        false, // catalog_handled
        false, // queued_busy: no busy-session tracking in the scaffold
    )
}

/// Whether scaffold plain text dispatches a prompt (Node `prompt.ts`
/// empty-prompt guard parity via
/// `otb_core::prompt_request_gates::should_dispatch_prompt`): blank text
/// with no files/photos/attachments sends nothing.
pub fn should_dispatch_plain_text(text: &str) -> bool {
    otb_core::prompt_request_gates::should_dispatch_prompt(text, 0, 0, false)
}

/// Whether plain text is a bottom-keyboard agent press (pure; testable).
/// Node `AGENT_MODE_BUTTON_TEXT_PATTERN` parity in `message-router.ts`
/// (`bot.hears(AGENT_MODE_BUTTON_TEXT_PATTERN)` -> `showAgentSelectionMenu`)
/// via `otb_core::keyboards::is_agent_mode_button_text`.
pub fn is_agent_button_press(text: &str) -> bool {
    otb_core::keyboards::is_agent_mode_button_text(text)
}

/// Voice/audio `file_id` plus size from a message, if it carries either
/// (TS `voice-handler.ts:187-190`: `voice?.file_id ?? audio?.file_id`).
/// The `u32::MAX` teloxide fallback (missing `file_size`) reads as unknown
/// (`None`), so the TS `if (file.file_size && ...)` pass-through holds.
/// Blank ids and non-voice/audio kinds yield `None` (silent ignore).
pub fn voice_audio_intake_from_message(message: &Message) -> Option<(String, Option<u64>)> {
    let (file_id, file_size) = match &message.kind {
        MessageKind::Common(common) => match &common.media_kind {
            MediaKind::Voice(voice) => (
                voice.voice.file.id.clone(),
                voice_file_size_opt(voice.voice.file.size),
            ),
            MediaKind::Audio(audio) => (
                audio.audio.file.id.clone(),
                voice_file_size_opt(audio.audio.file.size),
            ),
            _ => return None,
        },
        _ => return None,
    };
    if file_id.trim().is_empty() {
        return None;
    }
    Some((file_id, file_size))
}

/// Map a teloxide `file_size` to the intake gate input: the `u32::MAX`
/// missing-size fallback reads as unknown (`None`).
fn voice_file_size_opt(size: u32) -> Option<u64> {
    if size == u32::MAX {
        None
    } else {
        Some(u64::from(size))
    }
}

/// Stubbed STT transcript (P0-7 wiring step): the live `getFile` download +
/// STT `transcriptions` POST (with the secret key on the wire) stays a
/// dispatcher follow-up, so this always reports the deferred detail and the
/// caller answers `stt.error`. Swapping in the live call keeps the
/// recognizing/empty/transcript chain below untouched.
async fn stub_voice_transcript() -> Result<String, String> {
    tracing::info!("[otb-bot] telegram: voice transcribe deferred (live STT POST follow-up)");
    Err("transcription deferred".to_string())
}

/// Live voice/audio intake (TS `voice-handler.ts:187-204` guard order):
/// single-slot busy first (PRODUCT.md single-slot rule), then the
/// `decide_voice_intake` verdict (missing id ignores, unconfigured STT
/// answers `stt.not_configured`, over-cap answers `bot.file_too_large`),
/// then the `stt.recognizing` status. The transcript tail
/// ([`handle_voice_transcript`]) is shared with the `processUserPrompt`
/// prompt flow. No secret is ever logged; only the configured flag, the id
/// length, and the size travel to logs.
///
/// SCAFFOLD DEVIATION: TS edits the `stt.recognizing` status message in
/// place; the scaffold has no status-message tracking, so recognizing,
/// deferred-error, and transcript follow-ups arrive as new messages.
async fn handle_voice_message(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    locale: &str,
    file_id: &str,
    file_size: Option<u64>,
) {
    if let Some(hint_key) = crate::attach::prompt_blocked_hint() {
        tracing::info!("[otb-bot] telegram: voice prompt blocked by interaction slot");
        if let Err(err) =
            send_message(bot, chat_id, &crate::locale::translate(locale, &hint_key)).await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    let api_url = crate::media::stt_api_url_from_env();
    let api_key = crate::media::stt_api_key_from_env();
    let configured = crate::media::is_stt_configured(&api_url, &api_key);
    tracing::debug!(
        "[otb-bot] telegram: voice intake (configured={configured} id_len={} size={:?})",
        file_id.len(),
        file_size,
    );
    let intake = crate::media::decide_voice_intake(Some(file_id), file_size, configured);
    match intake {
        crate::media::VoiceIntake::Ignore => {
            tracing::debug!("[otb-bot] telegram: ignoring voice message with no file id");
            return;
        }
        crate::media::VoiceIntake::Reject { .. } => {
            if let Some(reply) = crate::media::voice_intake_error_text(locale, intake) {
                tracing::info!("[otb-bot] telegram: voice intake rejected");
                if let Err(err) = send_message(bot, chat_id, &reply).await {
                    tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
                }
            }
            return;
        }
        crate::media::VoiceIntake::Accept => {}
    }
    if let Err(err) =
        send_message(bot, chat_id, &crate::media::voice_recognizing_text(locale)).await
    {
        tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        return;
    }
    match stub_voice_transcript().await {
        Ok(transcript) => handle_voice_transcript(bot, chat_id, locale, &transcript).await,
        Err(detail) => {
            tracing::warn!("[otb-bot] telegram: voice transcribe stub reported: {detail}");
            if let Err(err) = send_message(
                bot,
                chat_id,
                &crate::media::voice_error_text(locale, &detail),
            )
            .await
            {
                tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
            }
        }
    }
}

/// Voice transcript tail (TS `voice-handler.ts:224-277` parity): blank
/// transcripts answer `stt.empty_result`, otherwise the `STT_NOTE_PROMPT`
/// preface folds into the LLM text and the result reuses the plain-text
/// prompt tail (slot re-check, project gate, `ensure_execute_session`,
/// `attach_session`, versioned `prompt_session`). A sent prompt gets no
/// immediate chat reply; the assistant answer arrives over SSE.
async fn handle_voice_transcript(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    locale: &str,
    recognized_text: &str,
) {
    let transcript = recognized_text.trim();
    if otb_core::voice_media_intake_gates::transcript_reject_key(transcript).is_some() {
        tracing::info!("[otb-bot] telegram: voice transcript empty");
        if let Err(err) =
            send_message(bot, chat_id, &crate::media::voice_empty_result_text(locale)).await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    let note_prompt = crate::media::stt_note_prompt_from_env();
    let llm_text = crate::media::build_voice_llm_text(&note_prompt, transcript);
    if !crate::media::is_voice_prompt_sendable(&llm_text) {
        tracing::debug!("[otb-bot] telegram: voice transcript not sendable");
        return;
    }
    if let Some(hint_key) = crate::attach::prompt_blocked_hint() {
        tracing::info!("[otb-bot] telegram: voice transcript blocked by interaction slot");
        if let Err(err) =
            send_message(bot, chat_id, &crate::locale::translate(locale, &hint_key)).await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        if let Err(err) = send_message(
            bot,
            chat_id,
            &crate::locale::translate(locale, "bot.project_not_selected"),
        )
        .await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    };
    let (session_id, session_dir) = match ensure_execute_session(&worktree).await {
        Ok(session) => session,
        Err(key) => {
            if let Err(err) =
                send_message(bot, chat_id, &crate::locale::translate(locale, key)).await
            {
                tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
            }
            return;
        }
    };
    crate::attach::attach_session(&session_id, &session_dir);
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    if let Err(err) = client
        .prompt_session(&session_id, &session_dir, &llm_text)
        .await
    {
        tracing::warn!("[otb-bot] telegram: voice prompt failed: {err}");
        if let Err(err) = send_message(
            bot,
            chat_id,
            &crate::locale::translate(locale, "bot.prompt_send_error"),
        )
        .await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    tracing::info!("[otb-bot] telegram: voice transcript sent as prompt");
}

/// Live plain-text prompt (Node `processUserPrompt` session half parity):
/// reuse the remembered session for the project worktree or create and
/// remember a fresh one (`ensure_execute_session` parity with
/// `new-command.ts` `setCurrentSession`), then send the text through the
/// versioned `prompt_session` write path (V1 `parts[]`, V2
/// `{"text", "delivery"}` plus conditional `files[]` via `admitPrompt`
/// parity in `opencode.rs`).
///
/// Guard order matches `processUserPrompt`: blank text is ignored, a
/// bottom-keyboard agent press opens the agents view (`message-router.ts`
/// `AGENT_MODE_BUTTON_TEXT_PATTERN` -> `showAgentSelectionMenu` parity via
/// [`agents_view`] + [`build_agents_keyboard`]), a held
/// question takes the text as its custom answer (`question.reply`,
/// released on resolve), a held permission (or other) interaction replies
/// its blocked hint, no project replies `bot.project_not_selected`, a
/// failed ensure replies its key, a failed send replies
/// `bot.prompt_send_error`. A sent prompt gets no immediate chat reply —
/// the assistant answer still arrives over the SSE event subscription when
/// it lands.
///
/// SCAFFOLD FOLLOW-UPS (dispatcher): goal/task/search/rename/catalog text
/// stages per `message_router_text_gates` ordering; main-keyboard refresh
/// on a newly created session (`new-command.ts` parity, like `new_reply`
/// still skips).
async fn handle_plain_text_prompt(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    locale: &str,
    text: &str,
) {
    // Ordering gate first so the dispatch mapping cannot drift silently.
    let route = plain_text_prompt_route();
    tracing::debug!("[otb-bot] telegram: plain-text route={route:?}");
    // Bottom-keyboard agent entry (TS `message-router.ts` agent hears
    // parity): the button label itself is never a prompt. A held slot keeps
    // blocking the menu (`blockMenuWhileInteractionActive` parity), so the
    // press answers the blocked hint instead of opening the view.
    if is_agent_button_press(text) {
        tracing::debug!("[otb-bot] telegram: agent button pressed");
        if let Some(hint_key) = crate::attach::prompt_blocked_hint() {
            tracing::info!("[otb-bot] telegram: agent menu blocked by interaction slot");
            if let Err(err) =
                send_message(bot, chat_id, &crate::locale::translate(locale, &hint_key)).await
            {
                tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
            }
            return;
        }
        let (view_text, keyboard) = agents_view(locale).await;
        tracing::info!("[otb-bot] telegram: agent menu shown");
        reply_list_view(bot, chat_id, &view_text, keyboard).await;
        return;
    }
    if !should_dispatch_plain_text(text) {
        return;
    }
    // Single-slot guard (TS `message-router.ts` + `prompt.ts` busy check /
    // PRODUCT.md single-slot rule): a held interaction blocks the prompt
    // before attach/send, so no double prompt lands while one is on screen.
    // The hint copy stays with telegram-ux / i18n (this is only the hook).
    // A held question instead takes the text as its custom answer (TS
    // `handleQuestionTextAnswer` parity, minus the custom-input affordance:
    // with no poll cards rendered, the hold itself is the prompt).
    if let Some(pending) = crate::attach::held_question() {
        tracing::info!("[otb-bot] telegram: plain-text answer routed to held question");
        let client = crate::config::AppConfig::load()
            .map(|config| config.opencode_client())
            .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
        let answers = vec![vec![text.to_string()]];
        let bot_owned = bot.clone();
        let locale_owned = locale.to_string();
        // `submitPollAnswers` parity: no immediate chat reply — the slot
        // frees once OpenCode takes the answer.
        tokio::spawn(async move {
            match client
                .reply_question(
                    &pending.session_id,
                    &pending.request_id,
                    &pending.directory,
                    &answers,
                )
                .await
            {
                Ok(_) => {
                    tracing::info!("[otb-bot] telegram: question answered");
                    crate::attach::release_interaction_slot();
                }
                Err(err) if crate::opencode::is_not_found_error(&err) => {
                    tracing::info!("[otb-bot] telegram: question already settled");
                    crate::attach::release_interaction_slot();
                }
                Err(err) => {
                    tracing::warn!("[otb-bot] telegram: question answer failed: {err}");
                    // Keep the hold so a retry still lands (TS
                    // delivery-warning parity), and say so.
                    if let Err(err) = send_message(
                        &bot_owned,
                        chat_id,
                        &crate::locale::translate(&locale_owned, "permission.delivery_failed"),
                    )
                    .await
                    {
                        tracing::warn!(
                            "[otb-bot] telegram: reply failed for chat {chat_id}: {err}"
                        );
                    }
                }
            }
        });
        return;
    }
    if let Some(hint_key) = crate::attach::prompt_blocked_hint() {
        tracing::info!("[otb-bot] telegram: plain-text prompt blocked by interaction slot");
        if let Err(err) =
            send_message(bot, chat_id, &crate::locale::translate(locale, &hint_key)).await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        if let Err(err) = send_message(
            bot,
            chat_id,
            &crate::locale::translate(locale, "bot.project_not_selected"),
        )
        .await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    };
    let (session_id, session_dir) = match ensure_execute_session(&worktree).await {
        Ok(session) => session,
        Err(key) => {
            if let Err(err) =
                send_message(bot, chat_id, &crate::locale::translate(locale, key)).await
            {
                tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
            }
            return;
        }
    };
    // TS `processUserPrompt:219-226` mismatch reset parity:
    // `ensure_execute_session` already recreates the session when the
    // remembered directory drifts from the project worktree, so a hit
    // here only traces (a future drift surfaces instead of prompting the
    // wrong folder silently).
    if otb_core::prompt_request_gates::is_session_project_mismatch(
        Some(session_dir.as_str()),
        &worktree,
    ) {
        tracing::warn!("[otb-bot] telegram: ensured session directory mismatches project");
    }
    // Live path (TS `processUserPrompt` attach-before-send order):
    // follow the session and make sure its event stream pumps, so the
    // assistant answer arrives over SSE. Non-fatal by construction.
    crate::attach::attach_session(&session_id, &session_dir);
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    // Minimal busy guard (TS `prompt.ts:301-306` `isSessionBusy` parity;
    // a failed status read assumes busy like the attach transcript path).
    // TODO(prompt-queue-dispatch): route to the inbox/queue tail per
    // `prompt-queue-dispatch.ts:102-239` instead of sending into a busy
    // session. No queue system is invented here, so a busy session only
    // logs and still sends.
    match client.session_status(&session_dir).await {
        Ok(status) if !attach_session_is_busy(&status, &session_id) => {}
        Ok(_) => {
            tracing::info!(
                "[otb-bot] telegram: prompting a busy session (queue dispatch follow-up)"
            );
        }
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: prompt busy check failed, sending anyway: {err}");
        }
    }
    if let Err(err) = client.prompt_session(&session_id, &session_dir, text).await {
        tracing::warn!("[otb-bot] telegram: plain-text prompt failed: {err}");
        if let Err(err) = send_message(
            bot,
            chat_id,
            &crate::locale::translate(locale, "bot.prompt_send_error"),
        )
        .await
        {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    tracing::info!("[otb-bot] telegram: plain-text prompt sent");
}

/// `start` wiring: only constructs the bot when `OTB_ENABLE_POLLING=1`;
/// otherwise logs the scaffold notice. Never polls by default.
pub fn maybe_start_telegram() {
    if !polling_enabled() {
        tracing::info!("[otb-bot] telegram: scaffold only (set OTB_ENABLE_POLLING=1 to opt in)");
        return;
    }
    tracing::info!("[otb-bot] telegram: polling enabled (client starts in the async task)");
}

/// Send a list reply with an optional inline keyboard (thin parity for the
/// `replyWithInlineMenu` list views: text-only when no rows are selectable).
async fn reply_list_view(
    bot: &teloxide_core::Bot,
    chat_id: i64,
    text: &str,
    keyboard: Option<InlineKeyboardMarkup>,
) {
    let result = match keyboard {
        Some(markup) => send_message_with_keyboard(bot, chat_id, text, markup).await,
        None => send_message(bot, chat_id, text).await,
    };
    if let Err(err) = result {
        tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
    }
}

/// Handle a `commands:*` / `skills:*` tap (stateless scaffold of
/// `command-catalog-callback-handler.ts` / `skills-catalog-callback-handler.ts`):
/// page taps re-render the requested list page; select taps show the
/// `[Execute, Cancel]` confirm keyboard (`buildCommandsConfirmKeyboard` /
/// `buildSkillsConfirmKeyboard` parity); execute taps ensure a session for the
/// project and send `/name` as a prompt (the Node `session.command` endpoint
/// has no scaffold client yet; the prompt text is equivalent on the wire);
/// cancel taps delete the prompt message (Node `cancelMenu` parity). The
/// spinner is always acked first; unknown indexes answer the
/// `inactive_callback` notice plus the list view.
async fn handle_catalog_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
    kind: CatalogKind,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    let (is_cancel, is_execute, prefix) = match kind {
        CatalogKind::Commands => (
            is_commands_cancel(data),
            is_commands_execute(data),
            "commands",
        ),
        CatalogKind::Skills => (is_skills_cancel(data), is_skills_execute(data), "skills"),
    };
    if is_cancel {
        answer_callback(bot, query, None, false).await;
        // Node `cancelMenu` deletes the prompt message; a gone message is
        // not an error worth surfacing. Only ids are ever logged.
        if let Some((chat, message)) = callback_message_ids(query) {
            if let Err(err) = bot.delete_message(chat, message).await {
                tracing::debug!("[otb-bot] telegram: catalog cancel delete failed: {err}");
            }
        }
        return;
    }
    if is_execute {
        answer_callback(bot, query, None, false).await;
        handle_catalog_execute(bot, locale, query, kind, prefix).await;
        return;
    }
    // Page taps re-render the requested list page (Node pager parity).
    let page = match kind {
        CatalogKind::Commands => parse_command_page_callback(data),
        CatalogKind::Skills => parse_skill_page_callback(data),
    };
    if let Some(page) = page {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = match kind {
                CatalogKind::Commands => commands_view(locale, page).await,
                CatalogKind::Skills => skills_view(locale, page).await,
            };
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    let index = match kind {
        CatalogKind::Commands => parse_command_select_callback(data),
        CatalogKind::Skills => parse_skill_select_callback(data),
    };
    let Some(index) = index else {
        return;
    };
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let (worktree, items) = match kind {
        CatalogKind::Commands => match load_commands_catalog().await {
            Ok(loaded) => loaded,
            Err(key) => {
                reply_list_view(bot, chat_id, &crate::locale::translate(locale, key), None).await;
                return;
            }
        },
        CatalogKind::Skills => match load_skills_catalog().await {
            Ok(loaded) => loaded,
            Err(key) => {
                reply_list_view(bot, chat_id, &crate::locale::translate(locale, key), None).await;
                return;
            }
        },
    };
    let _ = worktree;
    let Some(item) = items.get(index) else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                &format!("{prefix}.inactive_callback"),
            )),
            true,
        )
        .await;
        let (text, keyboard) = match kind {
            CatalogKind::Commands => commands_view(locale, 0).await,
            CatalogKind::Skills => skills_view(locale, 0).await,
        };
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    };
    let confirm = match kind {
        CatalogKind::Commands => crate::commands::render_command_confirm(&item.name, Some(locale)),
        CatalogKind::Skills => crate::commands::render_skill_confirm(&item.name, Some(locale)),
    };
    answer_callback(bot, query, None, false).await;
    let keyboard = build_catalog_confirm_keyboard(kind, locale);
    if let Err(err) = send_message_with_keyboard(bot, chat_id, &confirm, keyboard).await {
        tracing::warn!("[otb-bot] telegram: catalog confirm failed for chat {chat_id}: {err}");
    }
}

/// Handle a `commands:execute` / `skills:execute` tap: recover the picked
/// name from the confirm message text, ensure a session for the project
/// worktree, and send `/name` as a prompt. Failures render the local
/// `execute_error` / `create_session_error` notices.
async fn handle_catalog_execute(
    bot: &teloxide_core::Bot,
    locale: &str,
    query: &CallbackQuery,
    kind: CatalogKind,
    prefix: &str,
) {
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let name = callback_message_text(query).and_then(parse_confirm_pick_name);
    let Some(name) = name else {
        let (text, keyboard) = match kind {
            CatalogKind::Commands => commands_view(locale, 0).await,
            CatalogKind::Skills => skills_view(locale, 0).await,
        };
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, &format!("{prefix}.inactive_callback")),
            None,
        )
        .await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    };
    let loaded = match kind {
        CatalogKind::Commands => load_commands_catalog().await.map(|(worktree, _)| worktree),
        CatalogKind::Skills => load_skills_catalog().await.map(|(worktree, _)| worktree),
    };
    let Ok(worktree) = loaded else {
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, &format!("{prefix}.fetch_error")),
            None,
        )
        .await;
        return;
    };
    let (session_id, session_dir) = match ensure_execute_session(&worktree).await {
        Ok(session) => session,
        Err(key) => {
            reply_list_view(bot, chat_id, &crate::locale::translate(locale, key), None).await;
            return;
        }
    };
    // Live path, like the plain-text prompt: follow the session so its event
    // stream pumps while the command runs. Non-fatal by construction.
    crate::attach::attach_session(&session_id, &session_dir);
    let executing =
        crate::commands::render_executing_notice(&format!("{prefix}.executing_prefix"), &name, "");
    reply_list_view(bot, chat_id, &executing, None).await;
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    if let Err(err) = client
        .prompt_session(&session_id, &session_dir, &format!("/{name}"))
        .await
    {
        tracing::warn!("[otb-bot] telegram: catalog execute prompt failed: {err}");
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, &format!("{prefix}.execute_error")),
            None,
        )
        .await;
    }
}

/// Handle an `mcps:*` tap (stateless scaffold of
/// `mcp-catalog-callback-handler.ts`): select taps show the server detail
/// with the toggle/back/cancel keyboard (`buildMcpsDetailKeyboard` parity);
/// toggle taps flip the server via `mcp_set_enabled` (`enable =
/// status !== "connected"` parity) and re-render the detail; back taps return
/// to the list; cancel taps delete the prompt message. The toggle target is
/// recovered from the detail message text (no interaction memory; see
/// [`resolve_mcp_detail_name`]).
async fn handle_mcps_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if is_mcp_cancel(data) {
        answer_callback(bot, query, None, false).await;
        if let Some((chat, message)) = callback_message_ids(query) {
            if let Err(err) = bot.delete_message(chat, message).await {
                tracing::debug!("[otb-bot] telegram: mcps cancel delete failed: {err}");
            }
        }
        return;
    }
    if is_mcp_back(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = mcps_view(locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if is_mcp_toggle(data) {
        handle_mcp_toggle(bot, locale, query).await;
        return;
    }
    let Some(index) = parse_mcp_select_callback(data) else {
        return;
    };
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let servers = match load_mcps_catalog().await {
        Ok((_, servers)) => servers,
        Err(key) => {
            reply_list_view(bot, chat_id, &crate::locale::translate(locale, key), None).await;
            return;
        }
    };
    let Some(server) = servers.get(index) else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "inline.inactive_callback")),
            true,
        )
        .await;
        let (text, keyboard) = mcps_view(locale).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    };
    let detail = crate::commands::render_mcp_detail(server, Some(locale));
    answer_callback(bot, query, None, false).await;
    let keyboard = build_mcp_detail_keyboard(server, locale);
    if let Err(err) = send_message_with_keyboard(bot, chat_id, &detail, keyboard).await {
        tracing::warn!("[otb-bot] telegram: mcp detail failed for chat {chat_id}: {err}");
    }
}

/// Handle an `mcps:toggle` tap: recover the server name from the detail
/// message text, flip it (`enable = status !== "connected"`), and re-render
/// the detail with the `enabling` / `disabling` toast. A missing target
/// answers `inline.inactive_callback` plus the list view; a rejected toggle
/// answers `mcps.toggle_error`.
async fn handle_mcp_toggle(bot: &teloxide_core::Bot, locale: &str, query: &CallbackQuery) {
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let (worktree, servers) = match load_mcps_catalog().await {
        Ok(loaded) => loaded,
        Err(key) => {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, key)),
                true,
            )
            .await;
            return;
        }
    };
    let name = callback_message_text(query)
        .and_then(|text| resolve_mcp_detail_name(text, &servers, locale));
    let Some(name) = name else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "inline.inactive_callback")),
            true,
        )
        .await;
        let (text, keyboard) = mcps_view(locale).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    };
    let Some(server) = servers.iter().find(|server| server.name == name) else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "inline.inactive_callback")),
            true,
        )
        .await;
        return;
    };
    let enable = !matches!(
        server.status.name,
        otb_core::session_pick_catalog_gates::McpStatusName::Connected
    );
    let toast = crate::locale::translate(
        locale,
        if enable {
            "mcps.enabling"
        } else {
            "mcps.disabling"
        },
    );
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    match client.mcp_set_enabled(&worktree, &name, enable).await {
        Ok(true) => {
            answer_callback(bot, query, Some(toast), false).await;
            // Re-read the catalog so the detail reflects the flipped state;
            // a gone server falls back to the list (Node parity).
            let servers = load_mcps_catalog()
                .await
                .map(|(_, servers)| servers)
                .unwrap_or(servers);
            match servers.iter().find(|server| server.name == name) {
                Some(updated) => {
                    let detail = crate::commands::render_mcp_detail(updated, Some(locale));
                    let keyboard = build_mcp_detail_keyboard(updated, locale);
                    if let Err(err) =
                        send_message_with_keyboard(bot, chat_id, &detail, keyboard).await
                    {
                        tracing::warn!(
                            "[otb-bot] telegram: mcp detail refresh failed for chat {chat_id}: {err}"
                        );
                    }
                }
                None => {
                    let (text, keyboard) = mcps_view(locale).await;
                    reply_list_view(bot, chat_id, &text, keyboard).await;
                }
            }
        }
        Ok(false) | Err(_) => {
            tracing::warn!("[otb-bot] telegram: mcp toggle failed");
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, "mcps.toggle_error")),
                true,
            )
            .await;
        }
    }
}

/// Best-effort `applyAgentConfiguredSettings` parity (TS
/// `agent-selection-service.ts:159-212`): when the tapped agent names a
/// model and/or variant, persist the offered ones and leave the rest as is.
/// A configured model OpenCode no longer offers falls back to the configured
/// `OPENCODE_MODEL_PROVIDER`/`OPENCODE_MODEL_ID` env default when offered,
/// else the selection stays. Failures only log (never values); the agent
/// confirm stands either way.
async fn apply_agent_configured_selection(
    client: &crate::opencode::OpencodeClient,
    raw_agents: &[serde_json::Value],
    app_home: &std::path::Path,
    name: &str,
) {
    use otb_core::project_model_persist_gates as persist;
    let Some(raw) = raw_agents.iter().find(|value| {
        value
            .get("name")
            .and_then(|item| item.as_str())
            .is_some_and(|item| item == name)
    }) else {
        return;
    };
    let (provider, model, variant) = crate::commands::agent_configured_selection(raw);
    let has_model = provider.is_some() && model.is_some();
    if !has_model {
        if let Some(variant) = variant.as_deref().filter(|item| !item.is_empty()) {
            if persist::should_apply_agent_variant_only(false, true) {
                let current = crate::settings_current::read_settings_current(app_home);
                if let (Some(provider), Some(model)) = (
                    current.model_provider.as_deref(),
                    current.model_id.as_deref(),
                ) {
                    if let Err(err) = crate::settings_current::write_current_model(
                        app_home,
                        provider,
                        model,
                        Some(variant),
                    ) {
                        tracing::warn!("[otb-bot] telegram: cannot remember agent variant: {err}");
                    } else {
                        tracing::info!("[otb-bot] telegram: agent variant applied");
                    }
                }
            }
        }
        return;
    }
    let (provider, model) = (provider.unwrap_or_default(), model.unwrap_or_default());
    let catalog = match client.list_providers().await {
        Ok(catalog) => catalog,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            return;
        }
    };
    let offered = crate::commands::model_list_for_provider(&catalog, &provider)
        .iter()
        .any(|item| item.model_id == model);
    let adopted = if offered {
        Some((provider.clone(), model.clone()))
    } else {
        let env_provider = std::env::var(MODEL_PROVIDER_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let env_model = std::env::var(MODEL_ID_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        match (env_provider, env_model) {
            (Some(env_provider), Some(env_model))
                if !(env_provider == provider && env_model == model)
                    && crate::commands::model_list_for_provider(&catalog, &env_provider)
                        .iter()
                        .any(|item| item.model_id == env_model) =>
            {
                Some((env_provider, env_model))
            }
            _ => None,
        }
    };
    let Some((adopted_provider, adopted_model)) = adopted else {
        tracing::warn!("[otb-bot] telegram: agent model not offered; selection unchanged");
        return;
    };
    let is_agent_model = adopted_provider == provider && adopted_model == model;
    let current = crate::settings_current::read_settings_current(app_home);
    let stored_variant = persist::resolve_agent_applied_variant(
        is_agent_model,
        variant.as_deref(),
        current.model_variant.as_deref(),
    );
    // A non-offered variant must not be stored (TS `validateVariantForModel`):
    // keep the stored/default variant instead.
    let variants =
        crate::commands::variant_list_for_model(&catalog, &adopted_provider, &adopted_model);
    let usable = otb_core::agent_model_variant_gates::is_variant_usable(
        &variants
            .iter()
            .map(|id| otb_core::agent_model_variant_gates::VariantEntry {
                id: id.clone(),
                disabled: false,
            })
            .collect::<Vec<_>>(),
        &stored_variant,
    );
    let final_variant = if usable || stored_variant == "default" {
        stored_variant
    } else {
        "default".to_string()
    };
    if let Err(err) = crate::settings_current::write_current_model(
        app_home,
        &adopted_provider,
        &adopted_model,
        Some(&final_variant),
    ) {
        tracing::warn!("[otb-bot] telegram: cannot remember agent model: {err}");
    } else {
        tracing::info!("[otb-bot] telegram: agent model applied");
    }
}

/// Handle an `agent:<name>` tap (JS `agent-selection-callback-handler.ts`
/// parity): persist the tapped agent (`setCurrentAgent` parity) and confirm
/// with `agent.changed_message`. Unknown names and write failures answer
/// `agent.change_error_callback`; no project replies
/// `bot.project_not_selected`.
async fn handle_agent_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    let Some(name) = parse_agent_callback(data) else {
        return;
    };
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(worktree) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        answer_callback(bot, query, None, false).await;
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, "bot.project_not_selected"),
            None,
        )
        .await;
        return;
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw_agents = match client.list_agents(&worktree).await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: agents list failed: {err}");
            answer_callback(bot, query, None, false).await;
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "agent.menu.empty"),
                None,
            )
            .await;
            return;
        }
    };
    let items = crate::commands::agent_list_from_values(&raw_agents);
    if !items.iter().any(|item| item.name == name) {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "agent.change_error_callback",
            )),
            true,
        )
        .await;
        return;
    }
    if let Err(err) = crate::settings_current::write_current_agent(&app_home, name) {
        tracing::warn!("[otb-bot] telegram: cannot remember selected agent: {err}");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "agent.change_error_callback",
            )),
            true,
        )
        .await;
        return;
    }
    tracing::info!("[otb-bot] telegram: agent selected");
    apply_agent_configured_selection(&client, &raw_agents, &app_home, name).await;
    answer_callback(bot, query, None, false).await;
    let confirmation = crate::commands::render_agent_changed(name, Some(locale));
    if let Err(err) = send_message(bot, chat_id, &confirmation).await {
        tracing::warn!("[otb-bot] telegram: agent confirm failed for chat {chat_id}: {err}");
    }
}

/// Handle a `variant:<id>` tap (JS `variant-selection-callback-handler.ts`
/// parity): persist the tapped variant for the current model
/// (`setCurrentModel` parity, provider/model preserved) and confirm with
/// `variant.changed_message`. No model replies `variant.select_model_first`;
/// unknown variants and write failures answer
/// `variant.change_error_callback`.
async fn handle_variant_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    let Some(id) = parse_variant_callback(data) else {
        return;
    };
    let Some(chat_id) = callback_chat_id(query) else {
        return;
    };
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let (Some(provider), Some(model)) = (
        current.model_provider.clone().filter(|p| !p.is_empty()),
        current.model_id.clone().filter(|m| !m.is_empty()),
    ) else {
        answer_callback(bot, query, None, false).await;
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, "variant.select_model_first"),
            None,
        )
        .await;
        return;
    };
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let variants = match client.list_providers().await {
        Ok(raw) => crate::commands::variant_list_for_model(&raw, &provider, &model),
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            answer_callback(bot, query, None, false).await;
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "variant.menu.error"),
                None,
            )
            .await;
            return;
        }
    };
    if !variants.iter().any(|variant| variant == id) {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "variant.change_error_callback",
            )),
            true,
        )
        .await;
        return;
    }
    if let Err(err) =
        crate::settings_current::write_current_model(&app_home, &provider, &model, Some(id))
    {
        tracing::warn!("[otb-bot] telegram: cannot remember selected variant: {err}");
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "variant.change_error_callback",
            )),
            true,
        )
        .await;
        return;
    }
    tracing::info!("[otb-bot] telegram: variant selected");
    answer_callback(bot, query, None, false).await;
    let confirmation = crate::commands::render_variant_changed(id, Some(locale));
    if let Err(err) = send_message(bot, chat_id, &confirmation).await {
        tracing::warn!("[otb-bot] telegram: variant confirm failed for chat {chat_id}: {err}");
    }
}

/// Handle a `question:*` tap (JS `question-callback-handler.ts` parity):
/// auth-gate, prefix guard, then the live-poll gates. A tap with no held
/// question answers `question.inactive_callback` (the poll already
/// resolved); a `cancel` tap answers `common.cancelled` and rejects the
/// held request in the background (`question.reject`), releasing the slot
/// once OpenCode takes it or reports it gone. Other taps
/// (select/submit/custom/toggle) only ack — the option-selection state
/// machine needs poll rendering (`questionManager` + cards) and stays a
/// dispatcher follow-up.
async fn handle_question_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if !is_question_callback(data) {
        return;
    }
    let Some(parsed) = parse_question_callback(data) else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "question.processing_error_callback",
            )),
            true,
        )
        .await;
        return;
    };
    tracing::debug!(
        "[otb-bot] telegram: question callback q={} action={:?}",
        parsed.question_index,
        parsed.action
    );
    let Some(pending) = crate::attach::held_question() else {
        // No live poll (TS `!questionManager.isActive()`): the tap is stale.
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "question.inactive_callback",
            )),
            true,
        )
        .await;
        return;
    };
    if parsed.action != otb_core::question_permission_inline_gates::QuestionCallbackAction::Cancel {
        // Selection-state taps need the poll cards; ack the spinner only.
        answer_callback(bot, query, None, false).await;
        return;
    }
    // The dismissal is marked as being sent: a lost toast must not keep it
    // from going out (TS `question-callback-handler.ts` cancel parity).
    answer_callback(
        bot,
        query,
        Some(crate::locale::translate(locale, "common.cancelled")),
        false,
    )
    .await;
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let chat_id = callback_chat_id(query);
    let locale_owned = locale.to_string();
    let bot_owned = bot.clone();
    // `safeBackgroundTask` parity: the update that took the tap never
    // blocks on the reject round-trip.
    tokio::spawn(async move {
        match client
            .reject_question(&pending.session_id, &pending.request_id, &pending.directory)
            .await
        {
            Ok(_) => {
                tracing::info!("[otb-bot] telegram: question rejected");
                crate::attach::release_interaction_slot();
            }
            Err(err) if crate::opencode::is_not_found_error(&err) => {
                // Already settled outside Telegram: take the dismissal as
                // done and free the slot (TS `gone` finish parity).
                tracing::info!("[otb-bot] telegram: question already settled");
                crate::attach::release_interaction_slot();
            }
            Err(err) => {
                tracing::warn!("[otb-bot] telegram: question reject failed: {err}");
                // The poll stays answerable: keep the hold so a retry can
                // still dismiss it (TS delivery-warning parity), and say so.
                if let Some(chat_id) = chat_id {
                    if let Err(err) = send_message(
                        &bot_owned,
                        chat_id,
                        &crate::locale::translate(&locale_owned, "permission.delivery_failed"),
                    )
                    .await
                    {
                        tracing::warn!(
                            "[otb-bot] telegram: reply failed for chat {chat_id}: {err}"
                        );
                    }
                }
            }
        }
    });
}

/// Handle a `permission:*` tap (JS
/// `permission-callback-handler.ts` parity): auth-gate, prefix guard, then
/// the live-request gates. A tap with no held permission answers
/// `permission.inactive_callback`; a known `once` / `always` / `reject`
/// tap answers its `permission.reply.*` toast at once and replies to every
/// request the held prompt covers in the background (`permission.reply`),
/// releasing the slot once OpenCode takes them or reports them gone. A
/// transport failure keeps the hold answerable and sends the
/// `permission.delivery_failed` notice (TS `failSending` parity: the
/// accepted-share nuance needs per-request prompt state and stays a
/// follow-up).
async fn handle_permission_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if !is_permission_callback(data) {
        return;
    }
    let Some(reply) = classify_permission_callback(data) else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "permission.processing_error_callback",
            )),
            true,
        )
        .await;
        return;
    };
    tracing::debug!("[otb-bot] telegram: permission callback reply={:?}", reply);
    let Some(pending) = crate::attach::held_permission() else {
        // No live request (TS `!permissionManager.isActive()`): stale tap.
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "permission.inactive_callback",
            )),
            true,
        )
        .await;
        return;
    };
    let wire = match reply {
        otb_core::question_permission_inline_gates::PermissionReply::Once => "once",
        otb_core::question_permission_inline_gates::PermissionReply::Always => "always",
        otb_core::question_permission_inline_gates::PermissionReply::Reject => "reject",
    };
    // The answer is marked as being sent: a lost toast must not keep it
    // from going out (TS `markSending` parity, minus the in-flight swallow
    // — a double tap replays idempotently and settles as `gone`).
    answer_callback(
        bot,
        query,
        Some(crate::locale::translate(
            locale,
            &format!("permission.reply.{wire}"),
        )),
        false,
    )
    .await;
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let chat_id = callback_chat_id(query);
    let locale_owned = locale.to_string();
    let bot_owned = bot.clone();
    // `safeBackgroundTask` parity: the update that took the tap never
    // blocks on the reply round-trip. On V2 a reject settles every pending
    // request of the session (TS `settlesSession`); the release below
    // already drops the whole hold, so no extra pass is needed.
    tokio::spawn(async move {
        let mut failed = false;
        for request_id in &pending.request_ids {
            match client
                .reply_permission(
                    &pending.session_id,
                    request_id,
                    &pending.directory,
                    wire,
                    None,
                )
                .await
            {
                Ok(_) => {
                    tracing::info!("[otb-bot] telegram: permission reply sent");
                }
                Err(err) if crate::opencode::is_not_found_error(&err) => {
                    tracing::info!("[otb-bot] telegram: permission already settled");
                }
                Err(err) => {
                    tracing::warn!("[otb-bot] telegram: permission reply failed: {err}");
                    failed = true;
                }
            }
        }
        if failed {
            // At least one answer did not reach OpenCode: keep the hold so
            // a retry still lands (TS `showPermissionDeliveryWarning`
            // parity), and say so.
            if let Some(chat_id) = chat_id {
                if let Err(err) = send_message(
                    &bot_owned,
                    chat_id,
                    &crate::locale::translate(&locale_owned, "permission.delivery_failed"),
                )
                .await
                {
                    tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
                }
            }
            return;
        }
        crate::attach::release_interaction_slot();
    });
}

/// Handle a `model:*` tap (JS `model-selection-callback-handler.ts` parity):
/// `model:root` re-renders the root menu; `model:search` / `model:search:again`
/// prompt for a search query (the text-input search flow needs a dispatcher
/// and stays a follow-up); `model:search:cancel` deletes the prompt message;
/// `model:providers:<n>` re-renders the providers page;
/// `model:provider:<i>:<p>` shows that provider's models (always the first
/// page; the back button returns to providers page 0 — stateless deviation,
/// see [`model_provider_models_view`]); `model:pick:<n>` and
/// `model:list:<kind>:<n>` apply the tapped model (variant resets to
/// `"default"`, Node `resolveModelListCallback` parity) and confirm, with the
/// variant follow-up when the new model offers more than one variant; a
/// self-contained legacy `model:<provider>:<model>` tap (the shape
/// [`model_search_view`] result buttons carry) resolves against the live
/// catalog and applies the same way (Node `parseLegacyModelCallback`
/// parity). Stale `model:result:<n>` taps without remembered search state
/// re-prompt instead.
async fn handle_model_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if data == MODEL_ROOT_CALLBACK {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = model_root_view(locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if data == MODEL_SEARCH_CALLBACK || data == MODEL_SEARCH_AGAIN_CALLBACK {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "model.search.prompt"),
                None,
            )
            .await;
        }
        return;
    }
    if data == MODEL_SEARCH_CANCEL_CALLBACK {
        answer_callback(bot, query, None, false).await;
        if let Some((chat, message)) = callback_message_ids(query) {
            if let Err(err) = bot.delete_message(chat, message).await {
                tracing::debug!("[otb-bot] telegram: model search cancel delete failed: {err}");
            }
        }
        return;
    }
    if let Some(page) = parse_model_providers_page_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = model_providers_view(locale, page).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if let Some((provider_index, _)) = parse_model_provider_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = model_provider_models_view(locale, provider_index, 0).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    // A search-result tap without remembered search state (stateless
    // scaffold: no `results` stage memory) re-prompts for a query instead of
    // guessing the target.
    if parse_model_search_result_callback(data).is_some() {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "model.search.prompt"),
                None,
            )
            .await;
        }
        return;
    }
    // `model:list:<kind>:<n>` resolves against the same flat first-page list
    // the root keyboard shows (favorites + recent are folded together, see
    // [`model_root_view`]).
    if let Some((_, index)) = parse_model_list_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            apply_model_root_pick(bot, locale, chat_id, index).await;
        }
        return;
    }
    if let Some(index) = parse_model_pick_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            apply_model_provider_pick(bot, locale, chat_id, index).await;
        }
        return;
    }
    // Self-contained legacy pick `model:<provider>:<model>` (Node
    // `parseLegacyModelCallback` parity): search-result buttons carry the
    // full identity inline, so they resolve against the live catalog without
    // remembered search state. Every known shape returned above, so anything
    // reaching here is either legacy or garbage (a catalog miss answers
    // `model.change_error_callback`, like Node's invalid path).
    if let Some(resolved) = otb_core::agent_model_variant_gates::parse_legacy_model_callback(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            apply_model_legacy_pick(
                bot,
                locale,
                chat_id,
                &resolved.provider_id,
                &resolved.model_id,
            )
            .await;
        }
    }
}

/// Apply a `model:list:<kind>:<n>` tap against the root menu's flat first-page
/// list. Unknown indexes answer `model.change_error_callback`.
async fn apply_model_root_pick(bot: &teloxide_core::Bot, locale: &str, chat_id: i64, index: usize) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "model.providers.error"),
                None,
            )
            .await;
            return;
        }
    };
    let providers = crate::commands::provider_list_from_value(&raw);
    let mut flat: Vec<crate::commands::ModelListItem> = Vec::new();
    for provider in &providers {
        flat.extend(crate::commands::model_list_for_provider(&raw, &provider.id));
    }
    flat.sort_by(|left, right| {
        format!("{}/{}", left.provider_id, left.model_id)
            .cmp(&format!("{}/{}", right.provider_id, right.model_id))
    });
    let limit = crate::config::MODELS_LIST_LIMIT.max(1);
    let Some(pick) = flat.into_iter().take(limit).nth(index) else {
        answer_no_query_model_error(bot, chat_id, locale).await;
        return;
    };
    apply_model_pick(bot, locale, chat_id, &pick.provider_id, &pick.model_id).await;
}

/// Apply a `model:pick:<n>` tap: the index is global within its provider page
/// (see [`build_models_keyboard`]). Stateless, the provider is unknown, so
/// the first provider offering that position wins; an index no provider
/// offers answers `model.change_error_callback`.
async fn apply_model_provider_pick(
    bot: &teloxide_core::Bot,
    locale: &str,
    chat_id: i64,
    index: usize,
) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "model.providers.error"),
                None,
            )
            .await;
            return;
        }
    };
    let providers = crate::commands::provider_list_from_value(&raw);
    let mut pick: Option<crate::commands::ModelListItem> = None;
    for provider in &providers {
        let models = crate::commands::model_list_for_provider(&raw, &provider.id);
        if let Some(model) = models.get(index) {
            pick = Some(model.clone());
            break;
        }
    }
    let Some(pick) = pick else {
        answer_no_query_model_error(bot, chat_id, locale).await;
        return;
    };
    apply_model_pick(bot, locale, chat_id, &pick.provider_id, &pick.model_id).await;
}

/// Apply a legacy `model:<provider>:<model>` tap: the pair resolves against
/// the live catalog (exact match; a gone pair answers
/// `model.change_error_callback`), then applies like any other pick.
async fn apply_model_legacy_pick(
    bot: &teloxide_core::Bot,
    locale: &str,
    chat_id: i64,
    provider_id: &str,
    model_id: &str,
) {
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    let raw = match client.list_providers().await {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[otb-bot] telegram: providers list failed: {err}");
            reply_list_view(
                bot,
                chat_id,
                &crate::locale::translate(locale, "model.providers.error"),
                None,
            )
            .await;
            return;
        }
    };
    let offered = crate::commands::model_list_for_provider(&raw, provider_id)
        .iter()
        .any(|item| item.model_id == model_id);
    if !offered {
        tracing::warn!("[otb-bot] telegram: model legacy pick not offered");
        answer_no_query_model_error(bot, chat_id, locale).await;
        return;
    }
    apply_model_pick(bot, locale, chat_id, provider_id, model_id).await;
}

/// Persist a model pick (variant resets to `"default"`) and confirm with
/// `model.changed_message`. When the new model offers more than one variant,
/// the variant menu follows (Node
/// `showVariantSelectionMenuAfterModelChange` parity). A failed write answers
/// `model.change_error_callback`.
async fn apply_model_pick(
    bot: &teloxide_core::Bot,
    locale: &str,
    chat_id: i64,
    provider_id: &str,
    model_id: &str,
) {
    let app_home = crate::config::resolve_app_home();
    if let Err(err) = crate::settings_current::write_current_model(
        &app_home,
        provider_id,
        model_id,
        Some("default"),
    ) {
        tracing::warn!("[otb-bot] telegram: cannot remember selected model: {err}");
        answer_no_query_model_error(bot, chat_id, locale).await;
        return;
    }
    tracing::info!("[otb-bot] telegram: model selected");
    let confirmation =
        crate::commands::render_model_changed(provider_id, model_id, None, Some(locale));
    reply_list_view(bot, chat_id, &confirmation, None).await;
    // Variant follow-up (Node parity): only when the model offers a real
    // choice; failures stay silent so the model confirmation stands.
    let client = crate::config::AppConfig::load()
        .map(|config| config.opencode_client())
        .unwrap_or_else(|_| crate::opencode::OpencodeClient::from_env());
    if let Ok(raw) = client.list_providers().await {
        let variants = crate::commands::variant_list_for_model(&raw, provider_id, model_id);
        if variants.len() > 1 {
            let (text, keyboard) = variants_view(locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
    }
}

/// Stateless error reply for model-apply failures without a live callback
/// query (the spinner was already acked): the `model.change_error_callback`
/// text as a plain message.
async fn answer_no_query_model_error(bot: &teloxide_core::Bot, chat_id: i64, locale: &str) {
    reply_list_view(
        bot,
        chat_id,
        &crate::locale::translate(locale, "model.change_error_callback"),
        None,
    )
    .await;
}

/// Handle an `open:*` tap (JS `file-browser-callback-handler.ts`
/// `handleOpenCallback` parity): `open:roots` re-renders the roots list;
/// `open:nav:<path>` / `open:pg:<path>|<page>` re-render the requested
/// browse page; `open:sel:<path>` persists the directory as the current
/// project (folder name as display, path as id/worktree) and confirms
/// with `open.selected`. Outside the allowed roots answers
/// `open.access_denied`; scan failures render `open.scan_error`. The
/// spinner is always acked first.
async fn handle_open_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if is_open_roots(data) {
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = open_roots_view(locale);
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if let Some(path) = parse_open_nav_callback(data) {
        let roots = browser_roots();
        if !otb_core::file_browser_service_gates::is_within_allowed_root(
            path,
            &roots,
            cfg!(windows),
        ) {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, "open.access_denied")),
                true,
            )
            .await;
            return;
        }
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = open_browse_page(path, 0, locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if let Some((path, page)) = parse_open_page_callback(data) {
        let roots = browser_roots();
        if !otb_core::file_browser_service_gates::is_within_allowed_root(
            &path,
            &roots,
            cfg!(windows),
        ) {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, "open.access_denied")),
                true,
            )
            .await;
            return;
        }
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = open_browse_page(&path, page, locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if let Some(path) = parse_open_select_callback(data) {
        let roots = browser_roots();
        if !otb_core::file_browser_service_gates::is_within_allowed_root(
            path,
            &roots,
            cfg!(windows),
        ) {
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, "open.access_denied")),
                true,
            )
            .await;
            return;
        }
        let Some(chat_id) = callback_chat_id(query) else {
            return;
        };
        let home = home_directory();
        let display = otb_core::file_browser_service_gates::path_to_display_path(path, &home);
        let folder = crate::commands::get_project_folder_name(path);
        let app_home = crate::config::resolve_app_home();
        if let Err(err) =
            crate::settings_current::write_current_project(&app_home, path, path, &folder)
        {
            tracing::warn!("[otb-bot] telegram: cannot remember selected open directory: {err}");
            answer_callback(
                bot,
                query,
                Some(crate::locale::translate(locale, "open.select_error")),
                true,
            )
            .await;
            return;
        }
        tracing::info!("[otb-bot] telegram: open directory selected");
        answer_callback(bot, query, None, false).await;
        let confirmation =
            crate::locale::translate(locale, "open.selected").replace("{project}", &display);
        reply_list_view(bot, chat_id, &confirmation, None).await;
    }
}

/// Handle an `ls:*` tap (JS `handleLsCallback` parity):
/// `ls:nav:<path>` / `ls:pg:<path>|<page>` / `ls:back:<path>|<page>`
/// re-render the requested browse page; `ls:file:<path>|<page>` shows the
/// file details; `ls:attach:<path>` confirms the attach (stateless: the
/// prompt-attachment interaction needs a dispatcher and stays a
/// follow-up); `ls:download:<path>` answers the downloading toast and
/// re-sends the file-details view (direct document sending needs a
/// dispatcher follow-up). Outside the project root answers
/// `ls.access_denied`. The spinner is always acked first.
async fn handle_ls_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    let app_home = crate::config::resolve_app_home();
    let current = crate::settings_current::read_settings_current(&app_home);
    let Some(project_root) = current.project_worktree.clone().filter(|w| !w.is_empty()) else {
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(locale, "bot.project_not_selected")),
            true,
        )
        .await;
        return;
    };
    let denied = || crate::locale::translate(locale, "ls.access_denied");
    if let Some(path) = parse_ls_nav_callback(data) {
        if !otb_core::file_browser_service_gates::is_path_within_directory(path, &project_root) {
            answer_callback(bot, query, Some(denied()), true).await;
            return;
        }
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = ls_browse_page(&project_root, path, 0, locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if let Some((path, page)) = parse_ls_page_callback(data) {
        if !otb_core::file_browser_service_gates::is_path_within_directory(&path, &project_root) {
            answer_callback(bot, query, Some(denied()), true).await;
            return;
        }
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = ls_browse_page(&project_root, &path, page, locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if let Some((path, page)) = parse_ls_file_callback(data) {
        if !otb_core::file_browser_service_gates::is_path_within_directory(&path, &project_root) {
            answer_callback(bot, query, Some(denied()), true).await;
            return;
        }
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = ls_file_view(&path, page, locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
        return;
    }
    if let Some(path) = parse_ls_attach_callback(data) {
        if !otb_core::file_browser_service_gates::is_path_within_directory(path, &project_root) {
            answer_callback(bot, query, Some(denied()), true).await;
            return;
        }
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let relative = path
                .strip_prefix(&format!("{}/", project_root.trim_end_matches(['/', '\\'])))
                .unwrap_or(path);
            let confirmation =
                crate::locale::translate(locale, "attachment.added").replace("{path}", relative);
            reply_list_view(bot, chat_id, &confirmation, None).await;
        }
        return;
    }
    if let Some(path) = parse_ls_download_callback(data) {
        if !otb_core::file_browser_service_gates::is_path_within_directory(path, &project_root) {
            answer_callback(bot, query, Some(denied()), true).await;
            return;
        }
        // Download toast first (Node `sendDownloadedFile` parity: the toast
        // clears the spinner, then the file follows; the scaffold keeps no
        // document-sending path, so the details view follows instead).
        answer_callback(
            bot,
            query,
            Some(crate::locale::translate(
                locale,
                "commands.download.downloading",
            )),
            false,
        )
        .await;
        if let Some(chat_id) = callback_chat_id(query) {
            let probed = path.to_string();
            let meta = match tokio::task::spawn_blocking(move || std::fs::metadata(&probed)).await {
                Ok(Ok(meta)) => Some(meta),
                _ => None,
            };
            match meta {
                Some(meta) if meta.is_file() => {
                    let name = otb_core::file_browser_service_gates::base_name(path);
                    let size_text = otb_core::media_prompt::format_file_size(meta.len());
                    let size_label = crate::locale::translate(locale, "commands.download.size");
                    let header = crate::locale::translate(locale, "ls.file.header");
                    let text = format!(
                        "📄 {header}\n<code>{}</code>\n{size_label}: {size_text}",
                        crate::commands::escape_html(&name),
                    );
                    reply_list_view(bot, chat_id, &text, None).await;
                }
                Some(_) => {
                    reply_list_view(
                        bot,
                        chat_id,
                        &crate::locale::translate(locale, "commands.download.not_file"),
                        None,
                    )
                    .await;
                }
                None => {
                    reply_list_view(
                        bot,
                        chat_id,
                        &crate::locale::translate(locale, "commands.download.not_found"),
                        None,
                    )
                    .await;
                }
            }
        }
        return;
    }
    if let Some((path, page)) = parse_ls_back_callback(data) {
        if !otb_core::file_browser_service_gates::is_path_within_directory(&path, &project_root) {
            answer_callback(bot, query, Some(denied()), true).await;
            return;
        }
        answer_callback(bot, query, None, false).await;
        if let Some(chat_id) = callback_chat_id(query) {
            let (text, keyboard) = ls_browse_page(&project_root, &path, page, locale).await;
            reply_list_view(bot, chat_id, &text, keyboard).await;
        }
    }
}

/// Handle a `settings:*` tap (minimal JS `settings-callback-handler.ts`
/// parity): auth-gate, prefix guard, then the spinner is always answered
/// with the Node toast key (`settings.saved` for every handled toggle,
/// `callback.processing_error` for the unknown fallback; the TTS branch
/// answers its `tts.*` saved key). The toggle store writes, the settings
/// menu re-render, and the pinned-dashboard apply stay a dispatcher
/// follow-up, so the tap only acks here and never hangs.
async fn handle_settings_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if !is_settings_callback(data) {
        return;
    }
    let action = otb_core::settings_callback_gates::classify_settings_callback_action(data);
    // TTS mode is unknown statelessly; the saved-key helper needs a mode,
    // so answer the generic saved toast for the TTS branch here (the
    // `tts.*` key selection lands with the dispatcher).
    let key = match action {
        otb_core::settings_callback_gates::SettingsCallbackAction::Unknown => {
            "callback.processing_error"
        }
        _ => "settings.saved",
    };
    tracing::debug!("[otb-bot] telegram: settings callback action={:?}", action);
    answer_callback(
        bot,
        query,
        Some(crate::locale::translate(locale, key)),
        matches!(
            action,
            otb_core::settings_callback_gates::SettingsCallbackAction::Unknown
        ),
    )
    .await;
}

/// Handle a `task:retry-schedule` / `task:cancel` tap (minimal JS
/// `scheduled-task-callback-handler.ts` `handleTaskCallback` parity):
/// auth-gate, exact-shape guard, then the spinner is always acked. The
/// flow-state / interaction / message-id gates, the schedule reset, and
/// the message deletes need a dispatcher and stay a follow-up.
async fn handle_task_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    _locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if !is_task_retry_schedule_callback(data) && !is_task_cancel_callback(data) {
        return;
    }
    tracing::debug!("[otb-bot] telegram: task callback data={data}");
    // Ack the spinner so the tap never hangs; full handling (active-flow
    // gates, schedule reset, message deletes) lands with the dispatcher.
    answer_callback(bot, query, None, false).await;
}

/// Handle a `tasklist:*` tap (minimal JS `handleTaskListCallback`
/// parity): auth-gate, prefix guard, then the spinner is always acked. An
/// unknown shape answers the `callback.processing_error` toast; a known
/// open/delete/toggle/cancel tap only acks — the metadata / store /
/// runtime round-trips need a dispatcher and stay a follow-up.
async fn handle_tasklist_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if !is_tasklist_callback(data) {
        return;
    }
    if is_tasklist_cancel_callback(data)
        || parse_tasklist_open_callback(data).is_some()
        || parse_tasklist_delete_callback(data).is_some()
        || parse_tasklist_toggle_callback(data).is_some()
    {
        tracing::debug!("[otb-bot] telegram: tasklist callback data={data}");
        // Ack the spinner so the tap never hangs; full handling (metadata
        // gates, store toggle/delete, detail re-render) lands with the
        // dispatcher.
        answer_callback(bot, query, None, false).await;
        return;
    }
    answer_callback(
        bot,
        query,
        Some(crate::locale::translate(
            locale,
            "callback.processing_error",
        )),
        false,
    )
    .await;
}

/// Handle a `compact:details` / `compact:confirm` tap (minimal JS
/// `context-control-callback-handler.ts` parity): auth-gate, exact-shape
/// guard, then the spinner is always acked. The active-menu / stage gates,
/// the confirm-menu edit, and the summarize call need a dispatcher and
/// stay a follow-up.
async fn handle_compact_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    _locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if !is_compact_details_callback(data) && !is_compact_confirm_callback(data) {
        return;
    }
    tracing::debug!("[otb-bot] telegram: compact callback data={data}");
    // Ack the spinner so the tap never hangs; full handling (stage gates,
    // confirm edit, summarize) lands with the dispatcher.
    answer_callback(bot, query, None, false).await;
}

/// Handle an `inline:cancel:*` tap plus the legacy `compact:cancel` tap
/// (JS `handleInlineMenuCancel` parity, stateless): auth-gate, kind guard,
/// then toast `common.cancelled` and delete the menu message (Node
/// `cancelMenu` parity). Unknown kinds are ignored so the router can fall
/// through to `callback.unknown_command`.
async fn handle_inline_cancel_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    let Some(kind) = parse_inline_cancel_callback(data) else {
        return;
    };
    tracing::debug!("[otb-bot] telegram: inline cancel kind={kind}");
    answer_callback(
        bot,
        query,
        Some(crate::locale::translate(locale, "common.cancelled")),
        false,
    )
    .await;
    if let Some((chat, message)) = callback_message_ids(query) {
        if let Err(err) = bot.delete_message(chat, message).await {
            tracing::debug!("[otb-bot] telegram: inline cancel delete failed: {err}");
        }
    }
}

/// Handle a `rename:cancel` tap (minimal JS `handleRenameCancel` parity,
/// stateless): auth-gate, exact-shape guard, then toast
/// `common.cancelled` and confirm with the `rename.cancelled` text. The
/// waiting-for-name / interaction / active-message gates and the prompt
/// edit need a dispatcher and stay a follow-up.
async fn handle_rename_cancel_callback(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    query: &CallbackQuery,
) {
    if !is_authorized(Some(query.from.id.0), allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring callback from unauthorized user");
        return;
    }
    let data = query.data.as_deref().unwrap_or("");
    if !is_rename_cancel_callback(data) {
        return;
    }
    tracing::debug!("[otb-bot] telegram: rename cancel callback");
    answer_callback(
        bot,
        query,
        Some(crate::locale::translate(locale, "common.cancelled")),
        false,
    )
    .await;
    if let Some(chat_id) = callback_chat_id(query) {
        reply_list_view(
            bot,
            chat_id,
            &crate::locale::translate(locale, "rename.cancelled"),
            None,
        )
        .await;
    }
}

/// Handle one update: auth-gate, parse command, dispatch, reply (T13 + live
/// `/status`, `/sessions`, `/projects`, `/worktree`, `/recent`,
/// `/messages`, `/new`, `/help`, `/settings`, `/tasklist`, `/commands`,
/// `/skills`, `/mcps`, `/open`, `/ls`, `/rename`, `/abort`, `/detach`,
/// `/task`, `/opencode_start`, `/opencode_stop`, `/reload`, `/goal`).
/// Callback queries carrying `session:<id>`, `project:<id>`,
/// `worktree:<position>`, `recent:<position>`, or `messages:*` resolve
/// against a fresh live fetch and persist the selection (JS select
/// parity); `session:page:<n>`, `projects:page:<n>`, `worktree:page:<n>`,
/// `recent:page:<n>`, and `messages:page:<n>` re-render the requested list
/// page (JS pager parity); `messages:revert` reverts to the tapped message,
/// `messages:fork` forks a session from it, `messages:back` returns to the
/// list, and `messages:cancel` deletes the prompt message;
/// `commands:*` / `skills:*` page, select, execute, and cancel taps drive the
/// catalog confirm flow; `mcps:*` select, toggle, back, and cancel taps drive
/// the server detail flow; `agent:<name>` and `variant:<id>` taps persist the
/// pick and confirm; `question:*` select/submit/custom/toggle/cancel taps
/// and `permission:once` / `permission:always` / `permission:reject` taps
/// ack the spinner (minimal parity; the stateful reply/reject round-trips
/// need a dispatcher and stay a follow-up); `model:*` root, providers, provider, list, pick,
/// legacy (`model:<provider>:<model>`), and search taps drive the model
/// browser (stale `model:result:<n>` taps without remembered search state
/// re-prompt); `session:search*` / `projects:search*` taps
/// drive the stateless prompt-then-filter flow (the list views carry the
/// search button row; search/again send the prompt, cancel deletes the
/// message, stale page/result taps re-prompt; the query itself travels as
/// `/sessions <query>` / `/projects <query>` command args); `open:*` nav/page/roots/select taps drive the
/// `/open` directory browser and `ls:*` nav/page/file/attach/download/back
/// taps drive the `/ls` project browser; `settings:*` toggle taps answer the
/// saved toast (store writes + menu re-render stay a dispatcher follow-up);
/// `compact:details` / `compact:confirm` taps ack the spinner (active-menu /
/// stage gates + summarize stay a follow-up); `task:retry-schedule` /
/// `task:cancel` and `tasklist:*` open/delete/toggle/cancel taps ack the
/// spinner (flow-state / store / runtime round-trips stay a follow-up);
/// `inline:cancel:*` taps (plus legacy `compact:cancel`) toast
/// `common.cancelled` and delete the menu message; `rename:cancel` toasts
/// `common.cancelled` and confirms with `rename.cancelled` (waiting /
/// interaction / message gates stay a follow-up); other callback
/// data and non-message updates are ignored. Unauthorized users are
/// ignored. Voice/audio notes enter the STT intake (configured gate,
/// `stt.recognizing` status, stubbed transcribe with the live POST
/// deferred, transcript tail shared with the prompt flow). Non-command
/// text prompts the project session unless the single
/// interaction slot is held (then it gets the slot's busy hint and no
/// prompt is sent); attach + event subscription go live with the prompt;
/// keyboard refresh stays a dispatcher follow-up).
/// Unknown `/commands` get a minimal English notice. `/status`
/// renders the live JS-parity template from a fresh health probe plus the
/// `settings.json` `current*` view (agent/project/session fall back to
/// not-set until their live fetches land); `/sessions` lists the current
/// project's sessions with one tap-to-select button per row (or the
/// in-memory matches for `/sessions <query>`) and `/new`
/// creates one (both through the configured OpenCode client); `/projects`
/// merges the API project list with the isolated directory cache (or the
/// in-memory matches for `/projects <query>`);
/// `/worktree` lists `git worktree list --porcelain` rows for the current
/// project; `/recent` lists sessions across the known project directories
/// (all `Idle` until liveness lookups land); `/messages` lists the current
/// session's user messages with tap-to-show detail plus the revert/fork/
/// back/cancel detail actions; `/commands` and
/// `/skills` list the current project's catalog entries through the
/// configured OpenCode client; `/mcps` lists its MCP server statuses;
/// `/open` browses filesystem roots for adding a project; `/ls` lists the
/// current project directory; `/rename` shows the prompt with the live
/// title, or renames directly with `/rename <new title>`; `/abort` signals
/// the current session's running turn via the versioned abort endpoint
/// (plain text); `/detach` clears the remembered session without stopping
/// the server session; `/task` acks the schedule text (the interactive flow
/// stays a follow-up); `/opencode_start` and `/opencode_stop` report the
/// probed server status without starting or stopping anything (safety);
/// `/reload` re-reads the local settings file; `/goal` renders the entry
/// gate (the interactive objective flow stays a follow-up); `/help` and
/// `/settings` render their JS-parity texts with no I/O; `/tasklist`
/// renders the sorted `scheduledTasks` store from `settings.json`
/// (missing/invalid reads empty); non-command text reuses the remembered
/// project session (or creates and remembers one, so a `/new` session is
/// prompted next) and sends it via `prompt_session`; every other command
/// keeps its scaffold ack.
async fn handle_update(
    bot: &teloxide_core::Bot,
    allowed_user_id: u64,
    locale: &str,
    update: &Update,
) {
    if let UpdateKind::CallbackQuery(query) = &update.kind {
        let data = query.data.as_deref().unwrap_or("");
        // Inline-cancel pre-hook first (Node `callback-router.ts` parity:
        // `handleInlineMenuCancel` runs before prefix dispatch, and the
        // legacy `compact:cancel` reads as the `context` menu).
        if parse_inline_cancel_callback(data).is_some() {
            handle_inline_cancel_callback(bot, allowed_user_id, locale, query).await;
        } else if data == SESSION_SEARCH_CALLBACK
            || data == SESSION_SEARCH_AGAIN_CALLBACK
            || data == SESSION_SEARCH_CANCEL_CALLBACK
            || data.starts_with(SESSION_SEARCH_PAGE_PREFIX)
            || data.starts_with(SESSION_SEARCH_RESULT_PREFIX)
        {
            handle_session_search_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(SESSION_CALLBACK_PREFIX)
            || data.starts_with(SESSION_PAGE_CALLBACK_PREFIX)
        {
            handle_session_callback(bot, allowed_user_id, locale, query).await;
        } else if data == PROJECT_SEARCH_CALLBACK
            || data == PROJECT_SEARCH_AGAIN_CALLBACK
            || data == PROJECT_SEARCH_CANCEL_CALLBACK
            || data.starts_with(PROJECT_SEARCH_PAGE_PREFIX)
            || data.starts_with(PROJECT_SEARCH_RESULT_PREFIX)
        {
            handle_project_search_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(PROJECT_CALLBACK_PREFIX)
            || data.starts_with(PROJECT_PAGE_CALLBACK_PREFIX)
        {
            handle_project_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(WORKTREE_CALLBACK_PREFIX)
            || data.starts_with(WORKTREE_PAGE_CALLBACK_PREFIX)
        {
            handle_worktree_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(RECENT_CALLBACK_PREFIX)
            || data.starts_with(RECENT_PAGE_CALLBACK_PREFIX)
        {
            handle_recent_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(MESSAGES_CALLBACK_PREFIX) {
            handle_messages_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(COMMANDS_CALLBACK_PREFIX) {
            handle_catalog_callback(bot, allowed_user_id, locale, query, CatalogKind::Commands)
                .await;
        } else if data.starts_with(SKILLS_CALLBACK_PREFIX) {
            handle_catalog_callback(bot, allowed_user_id, locale, query, CatalogKind::Skills).await;
        } else if data.starts_with(MCPS_CALLBACK_PREFIX) {
            handle_mcps_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(AGENT_CALLBACK_PREFIX) {
            handle_agent_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(VARIANT_CALLBACK_PREFIX) {
            handle_variant_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(QUESTION_CALLBACK_PREFIX) {
            handle_question_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(PERMISSION_CALLBACK_PREFIX) {
            handle_permission_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with("model:") {
            handle_model_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(OPEN_CALLBACK_PREFIX) {
            handle_open_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(LS_CALLBACK_PREFIX) {
            handle_ls_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(SETTINGS_CALLBACK_PREFIX) {
            handle_settings_callback(bot, allowed_user_id, locale, query).await;
        } else if is_compact_details_callback(data) || is_compact_confirm_callback(data) {
            handle_compact_callback(bot, allowed_user_id, locale, query).await;
        } else if is_task_retry_schedule_callback(data) || is_task_cancel_callback(data) {
            handle_task_callback(bot, allowed_user_id, locale, query).await;
        } else if data.starts_with(TASKLIST_CALLBACK_PREFIX) {
            handle_tasklist_callback(bot, allowed_user_id, locale, query).await;
        } else if is_rename_cancel_callback(data) {
            handle_rename_cancel_callback(bot, allowed_user_id, locale, query).await;
        }
        return;
    }
    let message = match &update.kind {
        UpdateKind::Message(message) => message,
        _ => return,
    };
    let user_id = message.from.as_ref().map(|user| user.id.0);
    if !is_authorized(user_id, allowed_user_id) {
        tracing::debug!("[otb-bot] telegram: ignoring update from unauthorized user");
        return;
    }
    let chat_id = message.chat.id.0;
    // Voice/audio notes enter the STT intake before the text guard (TS
    // `message-router.ts` voice/audio branch parity): configured gate,
    // `stt.recognizing` status, stubbed transcribe (live POST deferred),
    // then the shared transcript-to-prompt tail. Single-slot rule holds.
    if let Some((file_id, file_size)) = voice_audio_intake_from_message(message) {
        handle_voice_message(bot, chat_id, locale, &file_id, file_size).await;
        return;
    }
    let Some(text) = message.text() else {
        return;
    };
    let Some((command, args)) = split_command_args(text) else {
        // Non-command text is a prompt, never an early return (Node
        // `message-router.ts` parity: the slash guard skips commands, the
        // rest falls through the question/goal/task/search/rename/catalog
        // stages to the queue/merge tail). The single-slot busy guard lives
        // inside `handle_plain_text_prompt`, so every dispatchable text
        // reaches the prompt flow here.
        handle_plain_text_prompt(bot, chat_id, locale, text).await;
        return;
    };
    if command == "sessions" {
        // A trailing query filters in memory (stateless search follow-up:
        // the search-button prompt asks for the query, which returns as
        // `/sessions <query>` args); bare lists the first page.
        let (text, keyboard) = if args.trim().is_empty() {
            sessions_view(locale, 0).await
        } else {
            sessions_search_view(locale, &args).await
        };
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "projects" {
        // Same stateless search transport as `/sessions <query>`.
        let (text, keyboard) = if args.trim().is_empty() {
            projects_view(locale, 0).await
        } else {
            projects_search_view(locale, &args).await
        };
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "worktree" {
        let (text, keyboard) = worktree_view(locale, 0).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "recent" {
        let (text, keyboard) = recent_view(locale, 0).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "messages" {
        let (text, keyboard) = messages_view(locale, 0).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "commands" {
        let (text, keyboard) = commands_view(locale, 0).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "skills" {
        let (text, keyboard) = skills_view(locale, 0).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "mcps" {
        let (text, keyboard) = mcps_view(locale).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "open" {
        let (text, keyboard) = open_view(locale).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "ls" {
        let (text, keyboard) = ls_view(locale, &args).await;
        reply_list_view(bot, chat_id, &text, keyboard).await;
        return;
    }
    if command == "rename" {
        let reply = if args.trim().is_empty() {
            rename_prompt_view(locale).await
        } else {
            rename_apply_view(locale, &args).await
        };
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    if command == "abort" {
        let reply = abort_view(locale).await;
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    if command == "detach" {
        let reply = detach_view(locale).await;
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    if command == "task" {
        let reply = task_view(locale, &args);
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    if command == "opencode_start" {
        // P0-8: container + local-target + already-running gates, spawn deferred.
        let reply = opencode_start_view(locale).await;
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    if command == "opencode_stop" {
        // P0-8: container + local-target + pid gates, health never consulted.
        let reply = opencode_stop_view(locale).await;
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    if command == "reload" {
        let reply = reload_view(locale);
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    if command == "goal" {
        let reply = goal_view(locale, &args);
        if let Err(err) = send_message(bot, chat_id, &reply).await {
            tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
        }
        return;
    }
    // Raw slash token for the unknown-command notice (Node
    // `unknown-command.ts` parity: `text.trim().split(/\s+/)[0]`, case
    // preserved). `command` above is the bare lowercase name from
    // `split_command_args` (see also `parse_command_text`, which delegates
    // to the same split).
    let raw_token = text
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or(command.as_str());
    debug_assert_eq!(parse_command_text(text).as_deref(), Some(command.as_str()));
    let reply = if command == "status" {
        status_reply(locale).await
    } else if command == "new" {
        new_reply(locale).await
    } else if command == "help" {
        crate::commands::render_help(Some(locale))
    } else if command == "settings" {
        crate::commands::render_settings_view(Some(locale))
    } else if command == "tasklist" {
        tasklist_reply(locale)
    } else if crate::commands::is_known_command(&command) {
        crate::commands::dispatch(&command).unwrap_or_else(|| {
            tracing::debug!("[otb-bot] telegram: unknown slash command received: {raw_token}");
            unknown_command_reply(locale, raw_token)
        })
    } else {
        tracing::debug!("[otb-bot] telegram: unknown slash command received: {raw_token}");
        unknown_command_reply(locale, raw_token)
    };
    if let Err(err) = send_message(bot, chat_id, &reply).await {
        tracing::warn!("[otb-bot] telegram: reply failed for chat {chat_id}: {err}");
    }
}

/// Map a polling `get_updates` failure onto the startup fatal-code gate
/// (pure; testable). TS `TELEGRAM_FATAL_TOKEN_CODES = {401, 404}`
/// (`start-bot-app.ts`): teloxide surfaces both shapes as
/// `ApiError::InvalidToken` (`"Unauthorized"` / `"Not Found"`), reported
/// here as `401` so
/// [`otb_core::stores_runtime::is_fatal_token_code`] decides.
pub fn poll_error_code(err: &teloxide_core::RequestError) -> Option<u16> {
    match err {
        teloxide_core::RequestError::Api(teloxide_core::ApiError::InvalidToken) => Some(401),
        _ => None,
    }
}

/// `retry_after` hint carried by a polling failure, in millis (pure;
/// testable). `RetryAfter(seconds)` mirrors the TS `retry_after` seconds
/// field honoured by `nextTelegramStartupRetryDelayMs`.
pub fn poll_retry_after_ms(err: &teloxide_core::RequestError) -> Option<u64> {
    match err {
        teloxide_core::RequestError::RetryAfter(secs) => {
            Some(u64::from(secs.seconds()).saturating_mul(1000))
        }
        _ => None,
    }
}

/// Full source chain of a polling `get_updates` failure, joined
/// outwards-in (pure; testable). Token-safe by construction: teloxide rewrites
/// the token inside the stored `reqwest::Error` URL to `token:redacted` (or
/// strips the URL) before wrapping it, and the chained sources
/// (hyper/rustls/io) never carry the URL — only the token length is ever
/// logged (see [`run_telegram_from_env`]). Today's `{err}` Display only shows
/// the outer `error sending request for url (...)` line, which hides whether
/// the process is dying on DNS, TLS, proxy, or connect.
pub fn poll_error_chain(err: &teloxide_core::RequestError) -> String {
    use std::error::Error as _;
    const MAX_LINKS: usize = 8;
    let mut parts = vec![err.to_string()];
    let mut next = err.source();
    while let Some(source) = next {
        if parts.len() > MAX_LINKS {
            parts.push("…".to_string());
            break;
        }
        parts.push(source.to_string());
        next = source.source();
    }
    parts.join(" <- caused by: ")
}

/// Best-effort transport hint for a polling failure (pure; testable): the
/// lowercased [`poll_error_chain`] is scanned for DNS/TLS/proxy/timeout
/// markers so the WARN line names where egress dies. Unknown shapes stay
/// `"network"`.
pub fn poll_error_hint(err: &teloxide_core::RequestError) -> &'static str {
    match err {
        teloxide_core::RequestError::Api(teloxide_core::ApiError::InvalidToken) => "token-rejected",
        teloxide_core::RequestError::Api(_) => "api",
        teloxide_core::RequestError::RetryAfter(_) => "retry-after",
        teloxide_core::RequestError::MigrateToChatId(_) => "migrated",
        teloxide_core::RequestError::InvalidJson { .. } => "invalid-json",
        teloxide_core::RequestError::Io(_) => "io",
        teloxide_core::RequestError::Network(_) => {
            classify_network_hint(&poll_error_chain(err).to_lowercase())
        }
    }
}

/// Substring classifier over a lowercased error chain (pure; testable).
fn classify_network_hint(lowered: &str) -> &'static str {
    if lowered.contains("timed out") || lowered.contains("timeout") || lowered.contains("deadline")
    {
        "network-timeout"
    } else if lowered.contains("certificate")
        || lowered.contains("tls")
        || lowered.contains("ssl")
        || lowered.contains("verify")
        || lowered.contains("handshake")
    {
        "network-tls"
    } else if lowered.contains("proxy") {
        "network-proxy"
    } else if lowered.contains("dns")
        || lowered.contains("resolve")
        || lowered.contains("name or service not known")
        || lowered.contains("nodename nor servname")
        || lowered.contains("no such host")
    {
        "network-dns"
    } else if lowered.contains("refused")
        || lowered.contains("reset")
        || lowered.contains("unreachable")
        || lowered.contains("not permitted")
        || lowered.contains("permission denied")
    {
        "network-connect"
    } else {
        "network"
    }
}

/// Proxy env presence for the polling client (pure; testable). Only
/// set/unset is reported — values may carry credentials and are never
/// logged. `reqwest` (used inside `teloxide_core::Bot`) honours these
/// automatically; the summary exists so the startup line shows whether the
/// service process even sees a proxy (a shell-set proxy missing from the
/// service env is the classic "curl works, service fails" shape).
pub fn format_proxy_summary(https: bool, http: bool, all: bool, no_proxy: bool) -> String {
    fn flag(set: bool) -> &'static str {
        if set {
            "set"
        } else {
            "unset"
        }
    }
    format!(
        "https_proxy={} http_proxy={} all_proxy={} no_proxy={}",
        flag(https),
        flag(http),
        flag(all),
        flag(no_proxy)
    )
}

/// Thin env reader for [`format_proxy_summary`] (reads env only; the value
/// itself is never logged).
pub fn proxy_env_summary() -> String {
    fn set(names: &[&str]) -> bool {
        names
            .iter()
            .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
    }
    format_proxy_summary(
        set(&["HTTPS_PROXY", "https_proxy"]),
        set(&["HTTP_PROXY", "http_proxy"]),
        set(&["ALL_PROXY", "all_proxy"]),
        set(&["NO_PROXY", "no_proxy"]),
    )
}

/// Long-poll forever with `get_updates` (offset ack + 30s timeout).
/// Only called when [`polling_enabled`]; never called from tests.
///
/// Node parity (`start-bot-app.ts` `callTelegramAtStartup`): transient
/// failures retry with a growing delay capped at 60s
/// ([`otb_core::stores_runtime::next_startup_retry_delay_ms`]); a rejected
/// token (401/404, surfaced as `ApiError::InvalidToken`) logs and exits
/// the process with code 1 so a supervisor can restart it.
pub async fn run_polling(bot: teloxide_core::Bot, allowed_user_id: u64, locale: &str) {
    let mut offset: i32 = 0;
    let mut attempt: u32 = 0;
    loop {
        match bot.get_updates().offset(offset).timeout(30).await {
            Ok(batch) => {
                attempt = 0;
                for update in &batch {
                    offset = update.id.0.wrapping_add(1) as i32;
                    handle_update(&bot, allowed_user_id, locale, update).await;
                }
            }
            Err(err) => {
                if otb_core::stores_runtime::is_fatal_token_code(poll_error_code(&err)) {
                    tracing::error!(
                        "[otb-bot] telegram: getUpdates rejected the bot token; exiting"
                    );
                    std::process::exit(1);
                }
                attempt = attempt.saturating_add(1);
                let delay_ms = otb_core::stores_runtime::next_startup_retry_delay_ms(
                    attempt,
                    poll_retry_after_ms(&err),
                );
                tracing::warn!(
                    "[otb-bot] telegram: getUpdates failed (attempt {attempt}): {err}; hint={}; chain={}; retrying in {delay_ms}ms",
                    poll_error_hint(&err),
                    poll_error_chain(&err),
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// Build the client from the env token + allowed id, then poll forever.
/// Call only when [`polling_enabled`]; performs network I/O. The token
/// value is never logged (only its length); `start` awaits this so the
/// process stays alive instead of exiting 0.
///
/// The attach SSE pump shares the process-wide `Send` aggregator (see
/// `attach.rs`), so plain `tokio::spawn` suffices — no `LocalSet` needed.
pub async fn run_telegram_from_env() {
    let token = token_from_env();
    let allowed = allowed_user_id_from_env();
    // App Home `.env` may carry `BOT_LOCALE`; fall back to `en` on error.
    let locale = crate::config::AppConfig::load()
        .map(|config| config.bot_locale)
        .unwrap_or_else(|_| crate::locale::FALLBACK_LOCALE.to_string());
    let bot = create_bot(&token);
    tracing::info!(
        "[otb-bot] telegram: polling started (token_len={} allowed_user_id={allowed} proxy[{}])",
        token.len(),
        proxy_env_summary()
    );
    install_card_hooks(&bot, i64::try_from(allowed).unwrap_or(0), &locale);
    install_session_idle_footer_hook(
        &bot,
        i64::try_from(allowed).unwrap_or(0),
        &locale,
        otb_core::types_config::default_show_assistant_run_footer(),
    );
    run_polling(bot, allowed, &locale).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polling_is_opt_in_only() {
        assert!(polling_enabled_from(Some("1")));
        assert!(!polling_enabled_from(None));
        assert!(!polling_enabled_from(Some("")));
        assert!(!polling_enabled_from(Some("true")));
        // Default process env must keep polling off.
        assert!(!polling_enabled_from(
            std::env::var(POLLING_ENV).ok().as_deref()
        ));
    }

    #[test]
    fn poll_error_mapping_feeds_startup_fatal_gate() {
        // Invalid token (TS 401/404) is fatal; everything else retries.
        let fatal = teloxide_core::RequestError::Api(teloxide_core::ApiError::InvalidToken);
        assert_eq!(poll_error_code(&fatal), Some(401));
        assert!(otb_core::stores_runtime::is_fatal_token_code(
            poll_error_code(&fatal)
        ));
        let transient = teloxide_core::RequestError::Api(teloxide_core::ApiError::BotBlocked);
        assert_eq!(poll_error_code(&transient), None);
        assert!(!otb_core::stores_runtime::is_fatal_token_code(
            poll_error_code(&transient)
        ));
        // Backoff grows 1s, 2s, 4s … and caps at 60s (TS parity).
        assert_eq!(
            otb_core::stores_runtime::next_startup_retry_delay_ms(1, None),
            1000
        );
        assert_eq!(
            otb_core::stores_runtime::next_startup_retry_delay_ms(2, None),
            2000
        );
        assert_eq!(
            otb_core::stores_runtime::next_startup_retry_delay_ms(10, None),
            60_000
        );
    }

    #[test]
    fn poll_error_chain_starts_with_outer_display() {
        // Chain keeps the (already token-redacted) outer Display first, so
        // the WARN line stays greppable while gaining the hidden causes.
        let fatal = teloxide_core::RequestError::Api(teloxide_core::ApiError::InvalidToken);
        let chain = poll_error_chain(&fatal);
        assert!(chain.starts_with(&fatal.to_string()));
        assert_eq!(poll_error_hint(&fatal), "token-rejected");
        let retry_after =
            teloxide_core::RequestError::RetryAfter(teloxide_core::types::Seconds::from_seconds(3));
        assert_eq!(poll_error_hint(&retry_after), "retry-after");
    }

    #[test]
    fn poll_error_hint_classifies_transport_failures() {
        assert_eq!(
            classify_network_hint("error sending request <- caused by: certificate verify failed"),
            "network-tls"
        );
        assert_eq!(
            classify_network_hint("error sending request <- caused by: tls handshake failure"),
            "network-tls"
        );
        assert_eq!(
            classify_network_hint("error sending request <- caused by: operation timed out"),
            "network-timeout"
        );
        assert_eq!(
            classify_network_hint("error sending request <- caused by: proxy connect failed"),
            "network-proxy"
        );
        assert_eq!(
            classify_network_hint(
                "error sending request <- caused by: failed to resolve host: name or service not known"
            ),
            "network-dns"
        );
        assert_eq!(
            classify_network_hint("error sending request <- caused by: connection refused"),
            "network-connect"
        );
        assert_eq!(
            classify_network_hint("error sending request <- caused by: operation not permitted"),
            "network-connect"
        );
        assert_eq!(
            classify_network_hint("error sending request for url (https://api.telegram.org/...)"),
            "network"
        );
    }

    #[test]
    fn proxy_summary_reports_presence_only() {
        assert_eq!(
            format_proxy_summary(true, false, false, true),
            "https_proxy=set http_proxy=unset all_proxy=unset no_proxy=set"
        );
        assert_eq!(
            format_proxy_summary(false, false, false, false),
            "https_proxy=unset http_proxy=unset all_proxy=unset no_proxy=unset"
        );
    }

    #[test]
    fn token_never_requires_live_env() {
        assert_eq!(resolve_token(Some("test-token")), "test-token");
        assert_eq!(resolve_token(Some("  padded  ")), "padded");
        assert_eq!(resolve_token(Some("")), DUMMY_TOKEN);
        assert_eq!(resolve_token(None), DUMMY_TOKEN);
    }

    #[test]
    fn token_resolution_prefers_live_over_test() {
        assert_eq!(
            resolve_token_ordered(Some("live-token"), Some("test-token")),
            "live-token"
        );
        assert_eq!(
            resolve_token_ordered(None, Some("test-token")),
            "test-token"
        );
        assert_eq!(resolve_token_ordered(None, None), DUMMY_TOKEN);
        assert_eq!(
            resolve_token_ordered(Some("  "), Some("test-token")),
            "test-token"
        );
        // Env-backed check with dummy values only (no network, no live paths).
        let prev_live = std::env::var(LIVE_TOKEN_ENV).ok();
        let prev_test = std::env::var(TEST_TOKEN_ENV).ok();
        std::env::set_var(TEST_TOKEN_ENV, "dummy-test-token");
        std::env::set_var(LIVE_TOKEN_ENV, "dummy-live-token");
        assert_eq!(token_from_env(), "dummy-live-token");
        match prev_live {
            Some(value) => std::env::set_var(LIVE_TOKEN_ENV, value),
            None => std::env::remove_var(LIVE_TOKEN_ENV),
        }
        match prev_test {
            Some(value) => std::env::set_var(TEST_TOKEN_ENV, value),
            None => std::env::remove_var(TEST_TOKEN_ENV),
        }
    }

    #[test]
    fn auth_allows_only_the_configured_user() {
        assert!(is_authorized(Some(42), 42));
        assert!(!is_authorized(Some(43), 42));
        assert!(!is_authorized(None, 42));
    }

    #[test]
    fn send_payload_truncates_on_char_boundary() {
        let (chat, body) = build_send_payload(7, "hi");
        assert_eq!(chat, ChatId(7));
        assert_eq!(body, "hi");

        let long = "é".repeat(TELEGRAM_TEXT_LIMIT + 10);
        let (_, body) = build_send_payload(7, &long);
        assert!(body.chars().count() <= TELEGRAM_TEXT_LIMIT);
        assert!(body.len() <= TELEGRAM_TEXT_LIMIT);
    }

    #[test]
    fn bot_construction_with_dummy_performs_no_io() {
        let bot = create_bot(DUMMY_TOKEN);
        assert_eq!(bot.token(), DUMMY_TOKEN);
    }

    #[test]
    fn parse_command_text_extracts_bare_lowercase_name() {
        assert_eq!(parse_command_text("/status"), Some("status".to_string()));
        assert_eq!(
            parse_command_text("/opencode_start@mybot extra args"),
            Some("opencode_start".to_string())
        );
        assert_eq!(parse_command_text("  /Help  "), Some("help".to_string()));
        assert_eq!(parse_command_text("hello"), None);
        assert_eq!(parse_command_text("/"), None);
        assert_eq!(parse_command_text(""), None);
    }

    #[test]
    fn split_command_args_separates_name_and_trailing_text() {
        assert_eq!(
            split_command_args("/status"),
            Some(("status".to_string(), String::new()))
        );
        assert_eq!(
            split_command_args("/ls  /work/proj "),
            Some(("ls".to_string(), "/work/proj".to_string()))
        );
        assert_eq!(
            split_command_args("/rename@mybot New title here"),
            Some(("rename".to_string(), "New title here".to_string()))
        );
        assert_eq!(split_command_args("hello"), None);
        assert_eq!(split_command_args("/"), None);
    }

    #[test]
    fn plain_text_routes_to_merge_before_queue() {
        // Failing-test-first for the dispatch gap: non-command text must
        // not miss — the `message_router_text_gates` ordering (input, no
        // slash, no blocking interaction, no stage handled, no busy queue)
        // resolves to the prompt tail (`Merge`).
        assert_eq!(
            plain_text_prompt_route(),
            otb_core::message_router_text_gates::MessageRouterTextRoute::Merge
        );
        assert_eq!(
            otb_core::message_router_text_gates::select_message_router_text_route(
                true, false, false, false, false, false, false, false, false, false, false,
            ),
            otb_core::message_router_text_gates::MessageRouterTextRoute::Merge
        );
    }

    #[test]
    fn plain_text_dispatch_needs_non_blank_text() {
        // `prompt.ts` empty-prompt guard parity: blank text sends nothing,
        // dispatchable text prompts the remembered/new session.
        assert!(should_dispatch_plain_text("fix the login bug"));
        assert!(!should_dispatch_plain_text(""));
        assert!(!should_dispatch_plain_text("   "));
    }

    #[test]
    fn agent_button_press_matches_bottom_keyboard_only() {
        // `message-router.ts` agent hears parity: bottom-keyboard agent
        // labels open the agents view, typed prompts do not.
        assert!(is_agent_button_press("🛠️ Build Agent"));
        assert!(is_agent_button_press("🤖 Sisyphus (Ultraworker) Mode"));
        assert!(!is_agent_button_press("fix the login bug"));
        assert!(!is_agent_button_press(""));
    }

    #[test]
    fn catalog_json_conversion_feeds_mcp_parser() {
        use otb_core::session_pick_catalog_gates::{
            parse_mcp_catalog_servers, McpStatusName as Name,
        };
        let value = serde_json::json!({
            "gh": { "status": "connected" },
            "db": { "status": "failed", "error": "boom" },
        });
        let servers = parse_mcp_catalog_servers(&catalog_json_from_value(&value)).expect("parses");
        assert_eq!(servers.len(), 2);
        // `serde_json::Map` sorts keys (JS `Object.entries` preserves
        // insertion order), so look servers up by name.
        let gh = servers.iter().find(|s| s.name == "gh").expect("gh server");
        let db = servers.iter().find(|s| s.name == "db").expect("db server");
        assert_eq!(gh.status.name, Name::Connected);
        assert_eq!(db.status.error.as_deref(), Some("boom"));
        // Numbers/bools collapse (never inspected by the parser).
        let junk = serde_json::json!({ "x": { "status": 42 } });
        assert!(parse_mcp_catalog_servers(&catalog_json_from_value(&junk)).is_none());
    }

    #[test]
    fn filesystem_scans_match_js_shapes() {
        // Temp dirs only; removed afterwards (never a live path).
        let base = std::env::temp_dir().join(format!("otb-scan-test-{}", std::process::id()));
        let proj = base.join("proj");
        std::fs::create_dir_all(proj.join("src")).expect("temp dir");
        std::fs::create_dir_all(proj.join(".hidden")).expect("temp dir");
        std::fs::write(proj.join("note.txt"), "hi").expect("temp file");
        let proj_str = proj.to_string_lossy().into_owned();
        let subdirs = scan_open_subdirs(&proj_str).expect("scans");
        // Hidden directories are skipped, files are not listed.
        assert_eq!(subdirs, vec!["src".to_string()]);
        let entries = scan_ls_entries(&proj_str).expect("scans");
        // Dirs sort before files (JS `scanLsDirectory` keeps hidden names).
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_dir);
        assert!(entries[1].is_dir);
        assert!(!entries[2].is_dir);
        assert!(scan_open_subdirs(base.join("missing").to_str().unwrap()).is_err());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn browser_roots_default_to_home_and_split_env() {
        // Dummy values only; live env is restored afterwards.
        let prev = std::env::var("OPEN_BROWSER_ROOTS").ok();
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", "/tmp/fake-home");
        std::env::remove_var("OPEN_BROWSER_ROOTS");
        assert_eq!(browser_roots(), vec!["/tmp/fake-home".to_string()]);
        std::env::set_var("OPEN_BROWSER_ROOTS", " ~/a ,, ~/b ");
        assert_eq!(
            browser_roots(),
            vec![
                "/tmp/fake-home/a".to_string(),
                "/tmp/fake-home/b".to_string()
            ]
        );
        match prev {
            Some(value) => std::env::set_var("OPEN_BROWSER_ROOTS", value),
            None => std::env::remove_var("OPEN_BROWSER_ROOTS"),
        }
        match prev_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn parse_session_id_callback_supports_node_shapes() {
        assert_eq!(parse_session_id_callback("session:ses_1"), Some("ses_1"));
        // Page callbacks are excluded (Node `parseSessionIdCallback` parity).
        assert_eq!(parse_session_id_callback("session:page:2"), None);
        assert_eq!(parse_session_id_callback("session:"), None);
        assert_eq!(parse_session_id_callback("bogus"), None);
        assert_eq!(parse_session_id_callback(""), None);
    }

    #[test]
    fn parse_session_page_callback_reads_non_negative_pages() {
        assert_eq!(parse_session_page_callback("session:page:0"), Some(0));
        assert_eq!(parse_session_page_callback("session:page:2"), Some(2));
        assert_eq!(parse_session_page_callback("session:ses_1"), None);
        assert_eq!(parse_session_page_callback("session:page:"), None);
        assert_eq!(parse_session_page_callback("session:page:-1"), None);
        assert_eq!(parse_session_page_callback("session:page:2x"), None);
        assert_eq!(parse_session_page_callback("bogus"), None);
    }

    #[test]
    fn sessions_keyboard_has_one_callback_row_per_session() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        let items = vec![
            crate::commands::SessionListItem {
                id: "ses_1".to_string(),
                title: "Fix bug".to_string(),
                directory: "/work/proj".to_string(),
            },
            crate::commands::SessionListItem {
                id: "ses_2".to_string(),
                title: "".to_string(),
                directory: "/work/proj".to_string(),
            },
        ];
        let markup = build_sessions_keyboard(&items, Some("ses_1"), "en", 0, 1);
        assert_eq!(markup.inline_keyboard.len(), 2);
        for row in &markup.inline_keyboard {
            assert_eq!(row.len(), 1);
        }
        let first = &markup.inline_keyboard[0][0];
        assert!(first.text.contains("(current)"));
        assert!(first.text.contains("Fix bug"));
        match &first.kind {
            InlineKeyboardButtonKind::CallbackData(data) => {
                assert_eq!(data, "session:ses_1");
            }
            other => panic!("expected callback data, got {other:?}"),
        }
        // A blank title falls back instead of producing an empty button.
        let second = &markup.inline_keyboard[1][0];
        assert!(second.text.contains("new session"));
        match &second.kind {
            InlineKeyboardButtonKind::CallbackData(data) => {
                assert_eq!(data, "session:ses_2");
            }
            other => panic!("expected callback data, got {other:?}"),
        }
    }

    #[test]
    fn button_label_truncation_keeps_active_prefix() {
        assert_eq!(truncate_button_label("short", false), "short");
        assert_eq!(truncate_button_label("short", true), "✅ short");
        let long = "x".repeat(100);
        let truncated = truncate_button_label(&long, false);
        assert_eq!(truncated.chars().count(), 64);
        assert!(truncated.ends_with("..."));
        let truncated = truncate_button_label(&long, true);
        assert_eq!(truncated.chars().count(), 64);
        assert!(truncated.starts_with("✅ "));
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn project_and_worktree_callbacks_parse_node_shapes() {
        assert_eq!(
            parse_project_id_callback("project:dir_abc"),
            Some("dir_abc")
        );
        assert_eq!(parse_project_id_callback("projects:page:2"), None);
        assert_eq!(parse_project_id_callback("project:"), None);
        assert_eq!(parse_project_page_callback("projects:page:2"), Some(2));
        assert_eq!(parse_project_page_callback("project:dir_abc"), None);
        assert_eq!(parse_worktree_index_callback("worktree:3"), Some(3));
        assert_eq!(parse_worktree_index_callback("worktree:page:1"), None);
        assert_eq!(parse_worktree_page_callback("worktree:page:1"), Some(1));
        assert_eq!(parse_recent_index_callback("recent:0"), Some(0));
        assert_eq!(parse_recent_index_callback("recent:"), None);
        // Recent page taps round-trip through their own parser and never
        // leak into the index parser (Node `parseSessionIdCallback`-style
        // page exclusion parity).
        assert_eq!(parse_recent_page_callback("recent:page:0"), Some(0));
        assert_eq!(parse_recent_page_callback("recent:page:3"), Some(3));
        assert_eq!(parse_recent_page_callback("recent:0"), None);
        assert_eq!(parse_recent_page_callback("recent:page:"), None);
        assert_eq!(parse_recent_page_callback("recent:page:-1"), None);
        assert_eq!(parse_recent_index_callback("recent:page:1"), None);
        assert_eq!(parse_message_select_callback("messages:select:4"), Some(4));
        assert_eq!(parse_message_select_callback("messages:page:1"), None);
        assert_eq!(parse_message_page_callback("messages:page:1"), Some(1));
    }

    #[test]
    fn worktree_keyboard_marks_current_and_carries_positions() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        let items = vec![
            crate::commands::WorktreeListItem {
                path: "/work/main".to_string(),
                branch: Some("main".to_string()),
                is_current: true,
            },
            crate::commands::WorktreeListItem {
                path: "/work/feat".to_string(),
                branch: None,
                is_current: false,
            },
        ];
        let markup = build_worktree_keyboard(&items, "en", 0, 1);
        assert_eq!(markup.inline_keyboard.len(), 2);
        let first = &markup.inline_keyboard[0][0];
        assert!(first.text.starts_with("✅ "));
        match &first.kind {
            InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, "worktree:0"),
            other => panic!("expected callback data, got {other:?}"),
        }
        let second = &markup.inline_keyboard[1][0];
        assert!(!second.text.starts_with("✅ "));
        match &second.kind {
            InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, "worktree:1"),
            other => panic!("expected callback data, got {other:?}"),
        }
    }

    #[test]
    fn projects_keyboard_marks_active_and_carries_ids() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        let items = vec![crate::commands::ProjectListItem {
            id: "dir_abc".to_string(),
            worktree: "/work/proj".to_string(),
            name: "proj".to_string(),
            last_updated: 0,
        }];
        let markup = build_projects_keyboard(&items, Some("/work/proj"), "en", 0, 1);
        assert_eq!(markup.inline_keyboard.len(), 1);
        let button = &markup.inline_keyboard[0][0];
        assert!(button.text.starts_with("✅ "));
        match &button.kind {
            InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, "project:dir_abc"),
            other => panic!("expected callback data, got {other:?}"),
        }
    }

    #[test]
    fn recent_and_messages_keyboards_carry_positions() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        let recent = vec![crate::commands::RecentListItem {
            id: "ses_1".to_string(),
            title: "Fix".to_string(),
            directory: "/work/proj".to_string(),
            updated: 1,
            status: crate::commands::RecentStatus::Idle,
        }];
        let markup = build_recent_keyboard(&recent, "en", 0, 1);
        assert_eq!(markup.inline_keyboard.len(), 1);
        match &markup.inline_keyboard[0][0].kind {
            InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, "recent:0"),
            other => panic!("expected callback data, got {other:?}"),
        }
        assert!(markup.inline_keyboard[0][0].text.contains("○"));

        let messages = vec![crate::commands::UserMessageItem {
            id: "msg_1".to_string(),
            text: "hello".to_string(),
            created: 3_600_000,
        }];
        let markup = build_messages_keyboard(&messages, "en", 0, 1);
        assert_eq!(markup.inline_keyboard.len(), 1);
        match &markup.inline_keyboard[0][0].kind {
            InlineKeyboardButtonKind::CallbackData(data) => {
                assert_eq!(data, "messages:select:0")
            }
            other => panic!("expected callback data, got {other:?}"),
        }
        assert!(markup.inline_keyboard[0][0].text.contains("[01:00]"));
    }

    #[test]
    fn clamp_page_index_pins_to_valid_range() {
        assert_eq!(clamp_page_index(0, 0), 0);
        assert_eq!(clamp_page_index(5, 0), 0);
        assert_eq!(clamp_page_index(0, 3), 0);
        assert_eq!(clamp_page_index(2, 3), 2);
        assert_eq!(clamp_page_index(9, 3), 2);
    }

    #[test]
    fn pager_row_appears_only_across_pages() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        fn pager_data(markup: &InlineKeyboardMarkup) -> Vec<String> {
            markup
                .inline_keyboard
                .last()
                .map(|row| {
                    row.iter()
                        .filter_map(|button| match &button.kind {
                            InlineKeyboardButtonKind::CallbackData(data) => Some(data.clone()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        let sessions = vec![crate::commands::SessionListItem {
            id: "ses_1".to_string(),
            title: "Fix".to_string(),
            directory: "/work/proj".to_string(),
        }];
        // Single page: rows only, no pager.
        let single = build_sessions_keyboard(&sessions, None, "en", 0, 1);
        assert_eq!(single.inline_keyboard.len(), 1);
        // First of two pages: next only.
        let first = build_sessions_keyboard(&sessions, None, "en", 0, 2);
        assert_eq!(first.inline_keyboard.len(), 2);
        assert_eq!(pager_data(&first), vec!["session:page:1".to_string()]);
        // Last of two pages: prev only.
        let last = build_sessions_keyboard(&sessions, None, "en", 1, 2);
        assert_eq!(pager_data(&last), vec!["session:page:0".to_string()]);
        // Middle of three pages: prev + next share one row.
        let mid = build_sessions_keyboard(&sessions, None, "en", 1, 3);
        assert_eq!(
            pager_data(&mid),
            vec!["session:page:0".to_string(), "session:page:2".to_string()]
        );

        let projects = vec![crate::commands::ProjectListItem {
            id: "dir_abc".to_string(),
            worktree: "/work/proj".to_string(),
            name: "proj".to_string(),
            last_updated: 0,
        }];
        let single = build_projects_keyboard(&projects, None, "en", 0, 1);
        assert_eq!(single.inline_keyboard.len(), 1);
        let first = build_projects_keyboard(&projects, None, "en", 0, 2);
        assert_eq!(pager_data(&first), vec!["projects:page:1".to_string()]);
        let last = build_projects_keyboard(&projects, None, "en", 1, 2);
        assert_eq!(pager_data(&last), vec!["projects:page:0".to_string()]);

        let worktrees = vec![crate::commands::WorktreeListItem {
            path: "/work/main".to_string(),
            branch: None,
            is_current: false,
        }];
        let single = build_worktree_keyboard(&worktrees, "en", 0, 1);
        assert_eq!(single.inline_keyboard.len(), 1);
        let first = build_worktree_keyboard(&worktrees, "en", 0, 2);
        assert_eq!(pager_data(&first), vec!["worktree:page:1".to_string()]);
        let last = build_worktree_keyboard(&worktrees, "en", 1, 2);
        assert_eq!(pager_data(&last), vec!["worktree:page:0".to_string()]);

        let recent = vec![crate::commands::RecentListItem {
            id: "ses_1".to_string(),
            title: "Fix".to_string(),
            directory: "/work/proj".to_string(),
            updated: 1,
            status: crate::commands::RecentStatus::Idle,
        }];
        let single = build_recent_keyboard(&recent, "en", 0, 1);
        assert_eq!(single.inline_keyboard.len(), 1);
        let first = build_recent_keyboard(&recent, "en", 0, 2);
        assert_eq!(pager_data(&first), vec!["recent:page:1".to_string()]);
        let last = build_recent_keyboard(&recent, "en", 1, 2);
        assert_eq!(pager_data(&last), vec!["recent:page:0".to_string()]);

        let messages = vec![crate::commands::UserMessageItem {
            id: "msg_1".to_string(),
            text: "hello".to_string(),
            created: 3_600_000,
        }];
        let single = build_messages_keyboard(&messages, "en", 0, 1);
        assert_eq!(single.inline_keyboard.len(), 1);
        let first = build_messages_keyboard(&messages, "en", 0, 2);
        assert_eq!(pager_data(&first), vec!["messages:page:1".to_string()]);
        let last = build_messages_keyboard(&messages, "en", 1, 2);
        assert_eq!(pager_data(&last), vec!["messages:page:0".to_string()]);
    }

    #[test]
    fn messages_detail_actions_match_node_exact_shapes() {
        // Node `MESSAGES_CALLBACK_*` are exact matches with no payload.
        assert!(is_messages_revert("messages:revert"));
        assert!(is_messages_fork("messages:fork"));
        assert!(is_messages_back("messages:back"));
        assert!(is_messages_cancel("messages:cancel"));
        // No cross-talk between actions, and prefixed callbacks never
        // leak into the action matchers.
        assert!(!is_messages_revert("messages:fork"));
        assert!(!is_messages_fork("messages:back"));
        assert!(!is_messages_back("messages:cancel"));
        assert!(!is_messages_cancel("messages:revert"));
        assert!(!is_messages_revert("messages:select:0"));
        assert!(!is_messages_fork("messages:page:1"));
        assert!(!is_messages_back("messages:revert:0"));
        assert!(!is_messages_cancel("messages:cancel:"));
        assert!(!is_messages_revert(""));
        // The shared prefix routes every messages callback, exact or
        // positional, to the messages handler.
        for data in [
            "messages:select:0",
            "messages:page:1",
            "messages:revert",
            "messages:fork",
            "messages:back",
            "messages:cancel",
        ] {
            assert!(data.starts_with(MESSAGES_CALLBACK_PREFIX), "{data}");
        }
        assert!(!"session:page:1".starts_with(MESSAGES_CALLBACK_PREFIX));
    }

    #[test]
    fn message_detail_keyboard_matches_node_layout() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        // Node `buildMessageDetailKeyboard`: `[revert]`, `[fork]`, then
        // `[back, cancel]` sharing one row.
        let markup = build_message_detail_keyboard("en");
        assert_eq!(markup.inline_keyboard.len(), 3);
        assert_eq!(markup.inline_keyboard[2].len(), 2);
        let data: Vec<String> = markup
            .inline_keyboard
            .iter()
            .flat_map(|row| row.iter())
            .map(|button| match &button.kind {
                InlineKeyboardButtonKind::CallbackData(data) => data.clone(),
                other => panic!("expected callback data, got {other:?}"),
            })
            .collect();
        assert_eq!(
            data,
            vec![
                "messages:revert".to_string(),
                "messages:fork".to_string(),
                "messages:back".to_string(),
                "messages:cancel".to_string(),
            ]
        );
        // Labels come from the `messages.button.*` templates, never blank.
        for row in &markup.inline_keyboard {
            for button in row {
                assert!(!button.text.is_empty());
            }
        }
    }

    #[test]
    fn message_detail_target_resolves_from_rendered_text() {
        let items = vec![
            crate::commands::UserMessageItem {
                id: "msg_1".to_string(),
                text: "fix the login bug".to_string(),
                created: 3_600_000,
            },
            crate::commands::UserMessageItem {
                id: "msg_2".to_string(),
                text: "add tests".to_string(),
                created: 7_200_000,
            },
        ];
        let first = crate::commands::render_message_detail(&items[0]);
        let second = crate::commands::render_message_detail(&items[1]);
        assert_eq!(resolve_message_detail_index(&first, &items), Some(0));
        assert_eq!(resolve_message_detail_index(&second, &items), Some(1));
        // Edited or foreign text matches nothing, so the caller falls back
        // to the inactive notice plus the list view.
        assert_eq!(
            resolve_message_detail_index("Select a message", &items),
            None
        );
        assert_eq!(
            resolve_message_detail_index(&format!("{second} (edited)"), &items),
            None
        );
        assert_eq!(resolve_message_detail_index(&first, &[]), None);
    }

    #[test]
    fn latest_assistant_text_skips_non_assistant_and_summary() {
        // `loadLatestAssistantResponse` parity: user roles and summary
        // entries never win; text parts join; the greatest `created` wins
        // with `>=` keeping the later entry on ties.
        let user = serde_json::json!({
            "info": { "role": "user", "time": { "created": 99 } },
            "parts": [{ "type": "text", "text": "ignore me" }]
        });
        let summary = serde_json::json!({
            "info": { "role": "assistant", "summary": true, "time": { "created": 100 } },
            "parts": [{ "type": "text", "text": "compacted" }]
        });
        let old = serde_json::json!({
            "info": { "role": "assistant", "time": { "created": 3 } },
            "parts": [{ "type": "text", "text": "first " }, { "type": "text", "text": "draft" }]
        });
        let tie_a = serde_json::json!({
            "info": { "role": "assistant", "time": { "created": 7 } },
            "parts": [{ "type": "text", "text": "tie one" }]
        });
        let tie_b = serde_json::json!({
            "info": { "role": "assistant", "time": { "created": 7 } },
            "parts": [{ "type": "text", "text": "tie two" }]
        });
        let rows = vec![user, summary, old, tie_a, tie_b];
        assert_eq!(latest_assistant_text(&rows).as_deref(), Some("tie two"));
        assert_eq!(latest_assistant_text(&[]), None);
        let blank = vec![serde_json::json!({
            "info": { "role": "assistant", "time": { "created": 1 } },
            "parts": [{ "type": "text", "text": "   " }]
        })];
        assert_eq!(latest_assistant_text(&blank), None);
    }

    #[test]
    fn page_callbacks_round_trip_through_parsers() {
        // Every pager button's `callback_data` parses back to its page.
        assert_eq!(parse_session_page_callback("session:page:1"), Some(1));
        assert_eq!(parse_project_page_callback("projects:page:1"), Some(1));
        assert_eq!(parse_worktree_page_callback("worktree:page:1"), Some(1));
        assert_eq!(parse_recent_page_callback("recent:page:1"), Some(1));
        assert_eq!(parse_message_page_callback("messages:page:1"), Some(1));
        // And page callbacks never leak into the select parsers.
        assert_eq!(parse_session_id_callback("session:page:1"), None);
        assert_eq!(parse_project_id_callback("projects:page:1"), None);
        assert_eq!(parse_worktree_index_callback("worktree:page:1"), None);
        assert_eq!(parse_recent_index_callback("recent:page:1"), None);
        assert_eq!(parse_message_select_callback("messages:page:1"), None);
    }

    #[test]
    fn catalog_callbacks_match_node_shapes() {
        // Commands catalog (`command-catalog-menu.ts` parity).
        assert_eq!(parse_command_select_callback("commands:select:3"), Some(3));
        assert_eq!(parse_command_select_callback("commands:select:"), None);
        assert_eq!(parse_command_select_callback("commands:select:-1"), None);
        assert_eq!(parse_command_select_callback("commands:page:1"), None);
        assert_eq!(parse_command_page_callback("commands:page:2"), Some(2));
        assert_eq!(parse_command_page_callback("commands:select:2"), None);
        assert!(is_commands_cancel("commands:cancel"));
        assert!(!is_commands_cancel("commands:execute"));
        assert!(is_commands_execute("commands:execute"));
        assert!(!is_commands_execute("commands:cancel"));
        // Skills catalog (`skills-catalog-menu.ts` parity): same shapes under
        // the `skills:` prefix.
        assert_eq!(parse_skill_select_callback("skills:select:0"), Some(0));
        assert_eq!(parse_skill_select_callback("skills:select:"), None);
        assert_eq!(parse_skill_select_callback("skills:page:1"), None);
        assert_eq!(parse_skill_page_callback("skills:page:1"), Some(1));
        assert_eq!(parse_skill_page_callback("skills:select:1"), None);
        assert!(is_skills_cancel("skills:cancel"));
        assert!(is_skills_execute("skills:execute"));
        assert!(!is_skills_cancel("commands:cancel"));
        // MCP catalog (`mcp-catalog-menu.ts` parity).
        assert_eq!(parse_mcp_select_callback("mcps:select:2"), Some(2));
        assert_eq!(parse_mcp_select_callback("mcps:select:"), None);
        assert_eq!(parse_mcp_select_callback("mcps:toggle"), None);
        assert!(is_mcp_toggle("mcps:toggle"));
        assert!(is_mcp_back("mcps:back"));
        assert!(is_mcp_cancel("mcps:cancel"));
        assert!(!is_mcp_toggle("mcps:back"));
        // Agent / variant (`agent:` / `variant:` parity; empty names yield
        // `None`).
        assert_eq!(parse_agent_callback("agent:build"), Some("build"));
        assert_eq!(parse_agent_callback("agent:"), None);
        assert_eq!(parse_agent_callback("bogus"), None);
        assert_eq!(parse_variant_callback("variant:default"), Some("default"));
        assert_eq!(parse_variant_callback("variant:"), None);
        assert_eq!(parse_variant_callback("bogus"), None);
    }

    #[test]
    fn question_permission_callbacks_match_node_shapes() {
        // `question:*` (`question-menu.ts` builders + handler dispatch parity).
        assert_eq!(QUESTION_CALLBACK_PREFIX, "question:");
        assert_eq!(PERMISSION_CALLBACK_PREFIX, "permission:");
        assert_eq!(PERMISSION_CALLBACK_ONCE, "permission:once");
        assert_eq!(PERMISSION_CALLBACK_ALWAYS, "permission:always");
        assert_eq!(PERMISSION_CALLBACK_REJECT, "permission:reject");
        assert!(is_question_callback("question:select:0:1"));
        assert!(is_question_callback("question:cancel:2"));
        assert!(!is_question_callback("permission:once"));
        assert!(!is_question_callback("bogus"));
        let select = parse_question_callback("question:select:0:1").expect("select parses");
        assert_eq!((select.question_index, select.option_index), (0, Some(1)));
        assert!(parse_question_callback("question:select:0").is_none());
        assert!(parse_question_callback("question:bogus:0").is_none());
        assert!(parse_question_callback("permission:once").is_none());
        // `permission:*` (`permission-menu.ts` buttons + handler gate parity).
        assert!(is_permission_callback("permission:once"));
        assert!(is_permission_callback("permission:always"));
        assert!(is_permission_callback("permission:reject"));
        assert!(!is_permission_callback("question:cancel:0"));
        assert!(!is_permission_callback("bogus"));
        assert_eq!(
            classify_permission_callback("permission:once"),
            Some(otb_core::question_permission_inline_gates::PermissionReply::Once)
        );
        assert_eq!(
            classify_permission_callback("permission:always"),
            Some(otb_core::question_permission_inline_gates::PermissionReply::Always)
        );
        assert_eq!(
            classify_permission_callback("permission:reject"),
            Some(otb_core::question_permission_inline_gates::PermissionReply::Reject)
        );
        assert_eq!(classify_permission_callback("permission:bogus"), None);
        assert_eq!(classify_permission_callback("question:cancel:0"), None);
    }

    #[test]
    fn poll_card_text_lists_headers_with_progress() {
        // `formatQuestionDetailsPart` fallback-text parity: one `❓ n/total
        // header` line per prepped header, joined with a blank line.
        let card = crate::attach::PollCardData {
            request_id: "req_1".to_string(),
            session_id: "ses_1".to_string(),
            directory: "/work".to_string(),
            question_count: 2,
            headers: vec!["Pick a model".to_string(), "Pick an agent".to_string()],
        };
        assert_eq!(
            build_poll_card_text(&card, "en"),
            "❓ 1/2 Pick a model\n\n❓ 2/2 Pick an agent"
        );
    }

    #[test]
    fn poll_card_text_falls_back_to_emoji_without_headers() {
        // The prep can carry zero headers (no `questions`/`fields` in the
        // envelope): the card still renders instead of going out empty.
        let card = crate::attach::PollCardData {
            request_id: "req_1".to_string(),
            session_id: "ses_1".to_string(),
            directory: String::new(),
            question_count: 0,
            headers: Vec::new(),
        };
        assert_eq!(build_poll_card_text(&card, "en"), "❓");
    }

    #[test]
    fn poll_card_keyboard_carries_custom_and_cancel() {
        use teloxide_core::types::InlineKeyboardButtonKind;
        // Localized labels with the exact `question:*` callbacks; option
        // rows stay a dispatcher follow-up (no option state in the prep).
        let keyboard = build_poll_card_keyboard("en");
        let rows = &keyboard.inline_keyboard;
        assert_eq!(rows.len(), 2);
        match &rows[0][0].kind {
            InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, "question:custom:0"),
            other => panic!("expected callback data, got {other:?}"),
        }
        match &rows[1][0].kind {
            InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, "question:cancel:0"),
            other => panic!("expected callback data, got {other:?}"),
        }
        assert!(!rows[0][0].text.is_empty());
        assert!(!rows[1][0].text.is_empty());
    }

    #[test]
    fn permission_card_text_uses_localized_kind_name() {
        // `formatPermissionText` header parity: emoji + localized name
        // through the `permission.header` template.
        let card = crate::attach::PermissionCardData {
            request_ids: vec!["perm_1".to_string()],
            session_id: "ses_1".to_string(),
            directory: "/work".to_string(),
            permission: "bash".to_string(),
            pattern_count: 3,
        };
        assert_eq!(
            build_permission_card_text(&card, "en"),
            "⚡ Permission request: Bash\n\n"
        );
        // Unknown kinds fall back to the raw id (TS `|| request.permission`).
        let unknown = crate::attach::PermissionCardData {
            permission: "mystery-tool".to_string(),
            ..card
        };
        assert_eq!(
            build_permission_card_text(&unknown, "en"),
            "🔐 Permission request: mystery-tool\n\n"
        );
    }

    #[test]
    fn permission_card_text_groups_multiple_requests() {
        // One prompt answering several pending requests carries the
        // `grouped_count` line (TS parity), exactly like the visible prompt.
        let card = crate::attach::PermissionCardData {
            request_ids: vec!["perm_1".to_string(), "perm_2".to_string()],
            session_id: "ses_1".to_string(),
            directory: "/work".to_string(),
            permission: "edit".to_string(),
            pattern_count: 1,
        };
        let text = build_permission_card_text(&card, "en");
        assert!(text.starts_with("✏️ Permission request: Edit\n\n"));
        assert!(text.contains("2 identical requests"));
    }

    #[test]
    fn permission_card_keyboard_carries_localized_replies() {
        use teloxide_core::types::InlineKeyboardButtonKind;
        // `buildPermissionKeyboard` parity: the three exact callbacks with
        // localized labels.
        let keyboard = build_permission_card_keyboard("en");
        let rows = &keyboard.inline_keyboard;
        assert_eq!(rows.len(), 3);
        for (row, expected) in
            rows.iter()
                .zip(["permission:once", "permission:always", "permission:reject"])
        {
            match &row[0].kind {
                InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, expected),
                other => panic!("expected callback data, got {other:?}"),
            }
        }
        assert_eq!(rows[0][0].text, "✅ Allow once");
        assert_eq!(rows[1][0].text, "🔓 Allow always");
        assert_eq!(rows[2][0].text, "❌ Reject");
    }

    #[test]
    fn card_builders_cover_all_twelve_locales() {
        use teloxide_core::types::InlineKeyboardButtonKind;
        // Jev coverage gap: card builders were en-tested only. Every
        // supported locale renders non-empty text plus labeled buttons
        // (missing keys fall back to `en`, so labels stay non-empty);
        // callbacks stay locale-independent.
        assert_eq!(crate::locale::SUPPORTED_LOCALES.len(), 12);
        for locale in crate::locale::SUPPORTED_LOCALES {
            let poll = crate::attach::PollCardData {
                request_id: "req_1".to_string(),
                session_id: "ses_1".to_string(),
                directory: "/work".to_string(),
                question_count: 1,
                headers: vec!["Pick a model".to_string()],
            };
            assert!(
                !build_poll_card_text(&poll, locale).is_empty(),
                "poll text empty for {locale}"
            );
            let poll_keyboard = build_poll_card_keyboard(locale);
            assert_eq!(poll_keyboard.inline_keyboard.len(), 2);
            for (row, expected) in poll_keyboard
                .inline_keyboard
                .iter()
                .zip(["question:custom:0", "question:cancel:0"])
            {
                match &row[0].kind {
                    InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, expected),
                    other => panic!("expected callback data, got {other:?}"),
                }
                assert!(
                    !row[0].text.is_empty(),
                    "poll button unlabeled for {locale}"
                );
            }
            let perm = crate::attach::PermissionCardData {
                request_ids: vec!["perm_1".to_string()],
                session_id: "ses_1".to_string(),
                directory: "/work".to_string(),
                permission: "bash".to_string(),
                pattern_count: 1,
            };
            let perm_text = build_permission_card_text(&perm, locale);
            assert!(!perm_text.is_empty(), "permission text empty for {locale}");
            assert!(
                !perm_text.contains("{emoji}") && !perm_text.contains("{name}"),
                "unreplaced template in permission text for {locale}"
            );
            let perm_keyboard = build_permission_card_keyboard(locale);
            assert_eq!(perm_keyboard.inline_keyboard.len(), 3);
            for (row, expected) in perm_keyboard.inline_keyboard.iter().zip([
                "permission:once",
                "permission:always",
                "permission:reject",
            ]) {
                match &row[0].kind {
                    InlineKeyboardButtonKind::CallbackData(data) => assert_eq!(data, expected),
                    other => panic!("expected callback data, got {other:?}"),
                }
                assert!(
                    !row[0].text.is_empty(),
                    "permission button unlabeled for {locale}"
                );
            }
        }
    }

    #[test]
    fn install_card_hooks_zero_chat_id_installs_nothing() {
        // Guard parity: the deny-all sentinel (no DM target) installs
        // neither hook, so no card send can spawn without a target chat.
        crate::attach::set_poll_card_hook(None);
        crate::attach::set_permission_card_hook(None);
        let bot = create_bot(DUMMY_TOKEN);
        install_card_hooks(&bot, 0, "en");
        assert!(!crate::attach::has_poll_card_hook());
        assert!(!crate::attach::has_permission_card_hook());
    }

    #[test]
    fn card_long_text_truncates_on_char_boundary() {
        // Multibyte filler forces both the char-based question cut and the
        // byte-based Telegram cap to engage: the card must fit the byte
        // limit without splitting a character (walk-back is at most 3
        // bytes for a 4-byte char).
        let poll = crate::attach::PollCardData {
            request_id: "req_1".to_string(),
            session_id: "ses_1".to_string(),
            directory: "/work".to_string(),
            question_count: 1,
            headers: vec!["é".repeat(5000)],
        };
        let poll_text = build_poll_card_text(&poll, "en");
        assert!(poll_text.len() <= TELEGRAM_TEXT_LIMIT);
        assert!(
            TELEGRAM_TEXT_LIMIT - poll_text.len() < 4,
            "poll card not capped at a char boundary: {} bytes",
            poll_text.len()
        );
        // Unknown permission kinds fall back to the raw id, so a long id
        // exercises the same byte cap on the permission path.
        let perm = crate::attach::PermissionCardData {
            request_ids: vec!["perm_1".to_string()],
            session_id: "ses_1".to_string(),
            directory: "/work".to_string(),
            permission: "é".repeat(5000),
            pattern_count: 0,
        };
        let perm_text = build_permission_card_text(&perm, "en");
        assert!(perm_text.len() <= TELEGRAM_TEXT_LIMIT);
        assert!(
            TELEGRAM_TEXT_LIMIT - perm_text.len() < 4,
            "permission card not capped at a char boundary: {} bytes",
            perm_text.len()
        );
    }

    #[test]
    fn model_callbacks_match_node_shapes() {
        // `model:list:<kind>:<n>` (`resolveModelListCallback` parity).
        assert_eq!(
            parse_model_list_callback("model:list:favorites:2"),
            Some(("favorites", 2))
        );
        assert_eq!(
            parse_model_list_callback("model:list:recent:0"),
            Some(("recent", 0))
        );
        assert_eq!(parse_model_list_callback("model:list:bogus:0"), None);
        assert_eq!(parse_model_list_callback("model:list:favorites:"), None);
        assert_eq!(
            parse_model_list_callback("model:list:favorites:2:extra"),
            None
        );
        assert_eq!(parse_model_list_callback("model:pick:2"), None);
        // Providers browser (`model-selection-menu.ts` parity).
        assert_eq!(
            parse_model_providers_page_callback("model:providers:1"),
            Some(1)
        );
        assert_eq!(
            parse_model_providers_page_callback("model:provider:0:1"),
            None
        );
        assert_eq!(
            parse_model_provider_callback("model:provider:2:1"),
            Some((2, 1))
        );
        assert_eq!(parse_model_provider_callback("model:provider:2"), None);
        assert_eq!(parse_model_provider_callback("model:provider:2:1:3"), None);
        assert_eq!(parse_model_pick_callback("model:pick:4"), Some(4));
        assert_eq!(parse_model_pick_callback("model:pick:"), None);
        // Page callbacks never leak into the sibling parsers.
        assert_eq!(parse_model_list_callback("model:providers:1"), None);
        assert_eq!(parse_model_pick_callback("model:provider:0:1"), None);
    }

    #[test]
    fn search_callbacks_match_node_exact_shapes() {
        // Session search (`session-selection-menu.ts` parity).
        assert_eq!(SESSION_SEARCH_CALLBACK, "session:search");
        assert_eq!(SESSION_SEARCH_AGAIN_CALLBACK, "session:search:again");
        assert_eq!(SESSION_SEARCH_CANCEL_CALLBACK, "session:search:cancel");
        assert_eq!(SESSION_SEARCH_PAGE_PREFIX, "session:search:page:");
        assert_eq!(SESSION_SEARCH_RESULT_PREFIX, "session:search:result:");
        assert_eq!(
            parse_session_search_page_callback("session:search:page:2"),
            Some(2)
        );
        assert_eq!(
            parse_session_search_page_callback("session:search:page:"),
            None
        );
        assert_eq!(
            parse_session_search_page_callback("session:search:page:-1"),
            None
        );
        assert_eq!(
            parse_session_search_page_callback("session:search:page:2x"),
            None
        );
        assert_eq!(parse_session_search_page_callback("session:page:2"), None);
        assert_eq!(parse_session_search_page_callback("session:search"), None);
        assert_eq!(
            parse_session_search_result_callback("session:search:result:0"),
            Some(0)
        );
        assert_eq!(
            parse_session_search_result_callback("session:search:result:12"),
            Some(12)
        );
        assert_eq!(
            parse_session_search_result_callback("session:search:result:"),
            None
        );
        assert_eq!(
            parse_session_search_result_callback("session:search:result:x"),
            None
        );
        assert_eq!(
            parse_session_search_result_callback("session:search:page:1"),
            None
        );
        // Page/result taps never leak into the select/page parsers.
        assert_eq!(parse_session_id_callback("session:page:1"), None);
        assert_eq!(parse_session_page_callback("session:search:page:1"), None);
        assert_eq!(parse_session_page_callback("session:search:result:1"), None);

        // Project search (`project-selection-menu.ts` parity).
        assert_eq!(PROJECT_SEARCH_CALLBACK, "projects:search");
        assert_eq!(PROJECT_SEARCH_AGAIN_CALLBACK, "projects:search:again");
        assert_eq!(PROJECT_SEARCH_CANCEL_CALLBACK, "projects:search:cancel");
        assert_eq!(PROJECT_SEARCH_PAGE_PREFIX, "projects:search:page:");
        assert_eq!(PROJECT_SEARCH_RESULT_PREFIX, "projects:search:result:");
        assert_eq!(
            parse_project_search_page_callback("projects:search:page:1"),
            Some(1)
        );
        assert_eq!(
            parse_project_search_page_callback("projects:search:page:"),
            None
        );
        assert_eq!(
            parse_project_search_page_callback("projects:search:page:1x"),
            None
        );
        assert_eq!(parse_project_search_page_callback("projects:page:1"), None);
        assert_eq!(
            parse_project_search_result_callback("projects:search:result:3"),
            Some(3)
        );
        assert_eq!(
            parse_project_search_result_callback("projects:search:result:"),
            None
        );
        assert_eq!(
            parse_project_search_result_callback("projects:search:result:-1"),
            None
        );
        assert_eq!(
            parse_project_search_result_callback("projects:search:page:1"),
            None
        );
        assert_eq!(parse_project_page_callback("projects:search:page:1"), None);
        assert_eq!(
            parse_project_page_callback("projects:search:result:1"),
            None
        );

        // Model search (`model-selection-menu.ts` +
        // `model-selection-callback-handler.ts` parity): the result prefix is
        // `model:result:`, never `model:search:result:`.
        assert_eq!(MODEL_SEARCH_CALLBACK, "model:search");
        assert_eq!(MODEL_SEARCH_AGAIN_CALLBACK, "model:search:again");
        assert_eq!(MODEL_SEARCH_CANCEL_CALLBACK, "model:search:cancel");
        assert_eq!(MODEL_SEARCH_RESULT_CALLBACK_PREFIX, "model:result:");
        assert_eq!(
            parse_model_search_result_callback("model:result:4"),
            Some(4)
        );
        assert_eq!(
            parse_model_search_result_callback("model:result:0"),
            Some(0)
        );
        assert_eq!(parse_model_search_result_callback("model:result:"), None);
        assert_eq!(parse_model_search_result_callback("model:result:x"), None);
        assert_eq!(parse_model_search_result_callback("model:search"), None);
        assert_eq!(
            parse_model_search_result_callback("model:search:result:1"),
            None
        );
        assert_eq!(parse_model_search_result_callback("model:pick:4"), None);
        assert_eq!(parse_model_pick_callback("model:result:4"), None);
    }

    #[test]
    fn search_query_matching_filters_like_node() {
        let session = crate::commands::SessionListItem {
            id: "ses_abc123".to_string(),
            title: "Fix login bug".to_string(),
            directory: "/work/proj".to_string(),
        };
        // Title match is case-insensitive; id matches too (Node
        // `searchSessions` parity).
        assert!(matches_session_query(&session, "login"));
        assert!(matches_session_query(&session, "LOGIN"));
        assert!(matches_session_query(&session, "  fix  "));
        assert!(matches_session_query(&session, "ses_abc"));
        assert!(matches_session_query(&session, "SES_ABC123"));
        assert!(!matches_session_query(&session, "checkout"));
        // A blank query matches every row.
        assert!(matches_session_query(&session, ""));
        assert!(matches_session_query(&session, "   "));

        let project = crate::commands::ProjectListItem {
            id: "dir_abc".to_string(),
            worktree: "/work/opencode-telegram-bot".to_string(),
            name: "opencode-telegram-bot".to_string(),
            last_updated: 0,
        };
        // Folder-name match is case-insensitive (Node
        // `filterProjectsForSearch` folder-name clause parity).
        assert!(matches_project_query(&project, "telegram"));
        assert!(matches_project_query(&project, "TELEGRAM"));
        assert!(matches_project_query(&project, "  opencode  "));
        assert!(!matches_project_query(&project, "other"));
        assert!(matches_project_query(&project, ""));
    }

    #[test]
    fn model_search_matching_filters_like_node() {
        let item = crate::commands::ModelListItem {
            provider_id: "anthropic".to_string(),
            model_id: "claude-sonnet-4".to_string(),
        };
        // Case-insensitive substring on `provider/model` (Node `searchModels`
        // parity).
        assert!(matches_model_query(&item, "claude"));
        assert!(matches_model_query(&item, "CLAUDE"));
        assert!(matches_model_query(&item, "anthropic/claude"));
        assert!(matches_model_query(&item, "  sonnet  "));
        assert!(!matches_model_query(&item, "openai"));
        assert!(!matches_model_query(&item, "claude-sonnet-5"));
        // Blank matches nothing (Node returns `[]` for a blank query),
        // unlike the session/project matchers where blank matches every row.
        assert!(!matches_model_query(&item, ""));
        assert!(!matches_model_query(&item, "   "));
    }

    #[test]
    fn legacy_model_pick_callbacks_bypass_index_parsers() {
        // Self-contained result picks carry the full identity inline, so no
        // remembered search state is needed to resolve them.
        let data = model_legacy_callback("anthropic", "claude-sonnet-4");
        assert_eq!(data, "model:anthropic:claude-sonnet-4");
        let resolved = otb_core::agent_model_variant_gates::parse_legacy_model_callback(&data)
            .expect("legacy shape parses");
        assert_eq!(resolved.provider_id, "anthropic");
        assert_eq!(resolved.model_id, "claude-sonnet-4");
        // Model ids containing colons survive the `:`-joined tail.
        let colon = model_legacy_callback("openai", "gpt:a");
        let resolved = otb_core::agent_model_variant_gates::parse_legacy_model_callback(&colon)
            .expect("colon model parses");
        assert_eq!(resolved.model_id, "gpt:a");
        // The indexed parsers never claim a legacy pick, so the handler's
        // legacy branch is the one that resolves it.
        assert_eq!(parse_model_pick_callback(&data), None);
        assert_eq!(parse_model_list_callback(&data), None);
        assert_eq!(parse_model_search_result_callback(&data), None);
        assert_eq!(parse_model_providers_page_callback(&data), None);
        assert_eq!(parse_model_provider_callback(&data), None);
    }

    #[test]
    fn search_query_truncates_to_sendable_cap() {
        assert_eq!(SESSION_SEARCH_QUERY_MAX_LENGTH, 100);
        assert_eq!(truncate_search_query("fix bug"), "fix bug");
        let long = "é".repeat(SESSION_SEARCH_QUERY_MAX_LENGTH + 10);
        let truncated = truncate_search_query(&long);
        assert_eq!(truncated.chars().count(), SESSION_SEARCH_QUERY_MAX_LENGTH);
    }

    #[test]
    fn catalog_keyboards_carry_node_callback_data() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        fn callback_data(markup: &InlineKeyboardMarkup) -> Vec<String> {
            markup
                .inline_keyboard
                .iter()
                .flat_map(|row| row.iter())
                .map(|button| match &button.kind {
                    InlineKeyboardButtonKind::CallbackData(data) => data.clone(),
                    other => panic!("expected callback data, got {other:?}"),
                })
                .collect()
        }

        let entries = vec![
            otb_core::session_pick_catalog_gates::CatalogEntry {
                name: "review".to_string(),
                description: Some("Review code".to_string()),
            },
            otb_core::session_pick_catalog_gates::CatalogEntry {
                name: "empty-desc".to_string(),
                description: None,
            },
        ];
        // List rows carry global indexes; the trailing row is the cancel
        // button (Node `buildCommandsListKeyboard` parity).
        let markup = build_catalog_list_keyboard(&entries, 10, CatalogKind::Commands, "en", 0, 1);
        assert_eq!(markup.inline_keyboard.len(), 3);
        assert_eq!(
            callback_data(&markup),
            vec![
                "commands:select:10".to_string(),
                "commands:select:11".to_string(),
                "commands:cancel".to_string(),
            ]
        );
        let markup = build_catalog_list_keyboard(&entries, 0, CatalogKind::Skills, "en", 0, 1);
        assert_eq!(
            callback_data(&markup),
            vec![
                "skills:select:0".to_string(),
                "skills:select:1".to_string(),
                "skills:cancel".to_string(),
            ]
        );
        // Confirm keyboards share one `[Execute, Cancel]` row (Node
        // `buildCommandsConfirmKeyboard` / `buildSkillsConfirmKeyboard`
        // parity).
        let markup = build_catalog_confirm_keyboard(CatalogKind::Commands, "en");
        assert_eq!(markup.inline_keyboard.len(), 1);
        assert_eq!(
            callback_data(&markup),
            vec![
                "commands:execute".to_string(),
                "commands:cancel".to_string()
            ]
        );
        let markup = build_catalog_confirm_keyboard(CatalogKind::Skills, "en");
        assert_eq!(
            callback_data(&markup),
            vec!["skills:execute".to_string(), "skills:cancel".to_string()]
        );
        // Labels never produce empty buttons (missing descriptions fall
        // back to the `no_description` template).
        for row in &markup.inline_keyboard {
            for button in row {
                assert!(!button.text.is_empty());
            }
        }
    }

    #[test]
    fn mcp_agent_variant_model_keyboards_carry_node_callback_data() {
        use otb_core::session_pick_catalog_gates::{
            McpCatalogServerItem, McpServerStatus, McpStatusName,
        };
        use teloxide_core::types::InlineKeyboardButtonKind;

        fn callback_data(markup: &InlineKeyboardMarkup) -> Vec<String> {
            markup
                .inline_keyboard
                .iter()
                .flat_map(|row| row.iter())
                .map(|button| match &button.kind {
                    InlineKeyboardButtonKind::CallbackData(data) => data.clone(),
                    other => panic!("expected callback data, got {other:?}"),
                })
                .collect()
        }

        // MCP list: one `mcps:select:<index>` row per server plus the cancel
        // row (Node `buildMcpsListKeyboard` parity).
        let servers = vec![
            McpCatalogServerItem {
                name: "gh".to_string(),
                status: McpServerStatus {
                    name: McpStatusName::Connected,
                    error: None,
                },
            },
            McpCatalogServerItem {
                name: "db".to_string(),
                status: McpServerStatus {
                    name: McpStatusName::Disabled,
                    error: None,
                },
            },
        ];
        let markup = build_mcps_list_keyboard(&servers, "en");
        assert_eq!(markup.inline_keyboard.len(), 3);
        assert_eq!(
            callback_data(&markup),
            vec![
                "mcps:select:0".to_string(),
                "mcps:select:1".to_string(),
                "mcps:cancel".to_string(),
            ]
        );
        // MCP detail: toggle row plus `[back, cancel]` sharing one row (Node
        // `buildMcpsDetailKeyboard` parity); auth-locked servers hide the
        // toggle row.
        let markup = build_mcp_detail_keyboard(&servers[0], "en");
        assert_eq!(markup.inline_keyboard.len(), 2);
        assert_eq!(markup.inline_keyboard[1].len(), 2);
        assert_eq!(
            callback_data(&markup),
            vec![
                "mcps:toggle".to_string(),
                "mcps:back".to_string(),
                "mcps:cancel".to_string(),
            ]
        );
        let locked = McpCatalogServerItem {
            name: "auth".to_string(),
            status: McpServerStatus {
                name: McpStatusName::NeedsAuth,
                error: None,
            },
        };
        let markup = build_mcp_detail_keyboard(&locked, "en");
        assert_eq!(markup.inline_keyboard.len(), 1);
        assert_eq!(
            callback_data(&markup),
            vec!["mcps:back".to_string(), "mcps:cancel".to_string()]
        );
        // Agents: one `agent:<name>` row per agent with the active mark (Node
        // `buildAgentSelectionMenu` parity).
        let agents = vec![
            crate::commands::AgentListItem {
                name: "build".to_string(),
            },
            crate::commands::AgentListItem {
                name: "plan".to_string(),
            },
        ];
        let markup = build_agents_keyboard(&agents, Some("build"));
        assert_eq!(
            callback_data(&markup),
            vec!["agent:build".to_string(), "agent:plan".to_string()]
        );
        assert!(markup.inline_keyboard[0][0].text.starts_with("✅ "));
        assert!(!markup.inline_keyboard[1][0].text.starts_with("✅ "));
        // Variants: one `variant:<id>` row per variant (Node
        // `buildVariantSelectionMenu` parity).
        let markup =
            build_variants_keyboard(&["default".to_string(), "thinking".to_string()], "thinking");
        assert_eq!(
            callback_data(&markup),
            vec![
                "variant:default".to_string(),
                "variant:thinking".to_string()
            ]
        );
        assert!(markup.inline_keyboard[1][0].text.starts_with("✅ "));
        // Model root: `[Search, Providers]` first row plus one
        // `model:list:favorites:<index>` row per model (Node
        // `buildModelSelectionMenu` parity).
        let models = vec![crate::commands::ModelListItem {
            provider_id: "meta".to_string(),
            model_id: "m1".to_string(),
        }];
        let markup = build_model_root_keyboard(&models, None, "en");
        assert_eq!(markup.inline_keyboard.len(), 2);
        assert_eq!(markup.inline_keyboard[0].len(), 2);
        assert_eq!(
            callback_data(&markup),
            vec![
                "model:search".to_string(),
                "model:providers:0".to_string(),
                "model:list:favorites:0".to_string(),
            ]
        );
    }

    #[test]
    fn git_porcelain_parses_branches_and_detached() {
        let output = "worktree /work/main\nHEAD abc123\nbranch refs/heads/main\n\nworktree /work/feat\nHEAD def456\ndetached\n\n";
        let rows = parse_git_worktree_porcelain(output);
        assert_eq!(
            rows,
            vec![
                ("/work/main".to_string(), Some("main".to_string())),
                ("/work/feat".to_string(), None),
            ]
        );
        assert!(parse_git_worktree_porcelain("").is_empty());
        assert!(parse_git_worktree_porcelain("bare\n\n").is_empty());
    }

    #[test]
    fn linked_worktree_detection_matches_js_shape() {
        // Temp dirs only; removed afterwards (never a live path).
        let base = std::env::temp_dir().join(format!("otb-wt-test-{}", std::process::id()));
        let main = base.join("main");
        let linked = base.join("linked");
        std::fs::create_dir_all(&main).expect("temp dir");
        std::fs::create_dir_all(&linked).expect("temp dir");
        // A real `.git` directory means a main checkout, never linked.
        std::fs::create_dir_all(main.join(".git")).expect("temp dir");
        assert!(!is_linked_worktree(main.to_str().unwrap()));
        // A `.git` pointer file into another repo's `worktrees/` dir is linked.
        std::fs::write(
            linked.join(".git"),
            "gitdir: /work/main/.git/worktrees/linked",
        )
        .expect("temp file");
        assert!(is_linked_worktree(linked.to_str().unwrap()));
        // Not a repo at all: not linked (the `/worktree` view reports
        // `not_git_repo` instead).
        assert!(!is_linked_worktree(base.join("missing").to_str().unwrap()));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn session_identity_supports_bare_and_enveloped_shapes() {
        // Bare session (SDK-style).
        let (id, title) =
            session_identity_from_value(&serde_json::json!({ "id": "ses_1", "title": "hi" }));
        assert_eq!((id.as_str(), title.as_str()), ("ses_1", "hi"));

        // Live V2 envelope from `POST api/session`.
        let (id, title) = session_identity_from_value(&serde_json::json!({
            "data": { "id": "ses_abc", "title": "hello" }
        }));
        assert_eq!((id.as_str(), title.as_str()), ("ses_abc", "hello"));

        // Missing/blank ids stay empty so `new_reply` keeps its
        // `new.create_error` path.
        let (id, _) = session_identity_from_value(&serde_json::json!({ "title": "x" }));
        assert!(id.is_empty());
        let (id, _) = session_identity_from_value(&serde_json::json!({ "data": {} }));
        assert!(id.is_empty());
    }

    #[test]
    fn status_snapshot_maps_probe_health_with_placeholders() {
        let empty = crate::settings_current::SettingsCurrent::default();
        let healthy = crate::opencode::HealthProbe {
            ok: true,
            version: Some("2.0.0".to_string()),
        };
        let snapshot = build_status_snapshot_from(&healthy, &empty);
        assert!(snapshot.healthy);
        assert_eq!(snapshot.opencode_version.as_deref(), Some("2.0.0"));
        assert_eq!(snapshot.bot_version, env!("CARGO_PKG_VERSION"));
        // Model display always carries the JS-parity `🧠` prefix, whatever
        // the `OPENCODE_MODEL_*` env holds; the rest stays `None` without
        // a settings file.
        assert!(snapshot.model_display.starts_with("🧠 "));
        assert!(snapshot.agent_display.is_none());
        assert!(snapshot.project_display.is_none());
        assert!(snapshot.worktree_path.is_none());
        assert!(snapshot.session_title.is_none());

        let sick = crate::opencode::HealthProbe {
            ok: false,
            version: None,
        };
        let snapshot = build_status_snapshot_from(&sick, &empty);
        assert!(!snapshot.healthy);
        assert!(snapshot.opencode_version.is_none());
        let rendered = crate::commands::render_status(&snapshot, Some("en"));
        assert!(rendered.starts_with("🔴 OpenCode Server is unavailable"));
    }

    #[test]
    fn status_snapshot_prefers_settings_current() {
        // Temp-dir settings file: no live paths, removed afterwards.
        let dir = std::env::temp_dir().join(format!("otb-status-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join(crate::settings_current::SETTINGS_FILE_NAME),
            serde_json::json!({
                "currentAgent": "orchestrator",
                "currentModel": {
                    "providerID": "meta",
                    "modelID": "muse-spark-1.3-contributor",
                    "variant": "default"
                },
                "currentProject": {
                    "id": "dir_abc123",
                    "worktree": "/work/opencode-telegram-bot"
                },
                "currentSession": {
                    "id": "ses_1",
                    "title": "Fix status placeholders",
                    "directory": "/work/opencode-telegram-bot"
                }
            })
            .to_string(),
        )
        .expect("seed settings");
        let current = crate::settings_current::read_settings_current(&dir);
        std::fs::remove_dir_all(&dir).ok();

        let snapshot = build_status_snapshot_from(
            &crate::opencode::HealthProbe {
                ok: true,
                version: Some("2.0.0".to_string()),
            },
            &current,
        );
        // Agent through `getAgentDisplayName` parity (unknown name: 🤖 fallback).
        assert_eq!(snapshot.agent_display.as_deref(), Some("🤖 Orchestrator"));
        assert_eq!(
            snapshot.model_display,
            "🧠 meta/muse-spark-1.3-contributor (default)"
        );
        assert_eq!(
            snapshot.project_display.as_deref(),
            Some("/work/opencode-telegram-bot")
        );
        assert_eq!(
            snapshot.session_title.as_deref(),
            Some("Fix status placeholders")
        );
        let rendered = crate::commands::render_status(&snapshot, Some("en"));
        assert!(rendered.contains("Agent: 🤖 Orchestrator"));
        assert!(rendered.contains("Model: 🧠 meta/muse-spark-1.3-contributor (default)"));
        assert!(rendered.contains("Project: /work/opencode-telegram-bot"));
        assert!(rendered.contains("Current session: Fix status placeholders"));
    }

    #[test]
    fn status_model_display_formats_variant() {
        assert_eq!(
            format_status_model_display("meta", "m1", None),
            "🧠 meta/m1"
        );
        assert_eq!(
            format_status_model_display("meta", "m1", Some("default")),
            "🧠 meta/m1 (default)"
        );
        assert_eq!(
            format_status_model_display("meta", "m1", Some("  ")),
            "🧠 meta/m1"
        );
    }

    #[test]
    fn status_render_goes_through_send_payload_truncation() {
        let empty = crate::settings_current::SettingsCurrent::default();
        let snapshot = build_status_snapshot_from(
            &crate::opencode::HealthProbe {
                ok: true,
                version: None,
            },
            &empty,
        );
        let text = crate::commands::render_status(&snapshot, Some("en"));
        let (_, body) = build_send_payload(7, &text);
        assert_eq!(body, text);
        assert!(body.starts_with("🟢 OpenCode Server is running"));
    }

    /// Serializes the App Home override below: the tests run in threads and
    /// the override is process-global.
    static ISOLATED_HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Run `body` with `OPENCODE_TELEGRAM_HOME` pointed at a fresh temp dir
    /// (removed afterwards; the previous value is restored).
    fn with_isolated_home(body: impl FnOnce(&std::path::Path)) {
        let _guard = ISOLATED_HOME_LOCK.lock().unwrap();
        let prev_home = std::env::var("OPENCODE_TELEGRAM_HOME").ok();
        let home = std::env::temp_dir().join(format!(
            "otb-batch4-views-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&home).expect("temp home");
        std::env::set_var("OPENCODE_TELEGRAM_HOME", &home);
        body(&home);
        match prev_home {
            Some(value) => std::env::set_var("OPENCODE_TELEGRAM_HOME", value),
            None => std::env::remove_var("OPENCODE_TELEGRAM_HOME"),
        }
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn goal_view_follows_js_entry_gates() {
        with_isolated_home(|home| {
            let settings = home.join(crate::settings_current::SETTINGS_FILE_NAME);
            // No settings file: no project.
            assert_eq!(
                goal_view("en", ""),
                "🏗 Project is not selected.\n\nFirst select a project with /projects."
            );
            // Project but no session.
            std::fs::write(
                &settings,
                serde_json::json!({
                    "currentProject": { "id": "dir_1", "worktree": "/work/proj" },
                })
                .to_string(),
            )
            .expect("seed settings");
            assert_eq!(
                goal_view("en", ""),
                "💬 Session is not selected.\n\nFirst choose a session with /sessions or create one with /new."
            );
            // Session in another project.
            std::fs::write(
                &settings,
                serde_json::json!({
                    "currentProject": { "id": "dir_1", "worktree": "/work/proj" },
                    "currentSession": { "id": "ses_1", "title": "Old", "directory": "/work/other" },
                })
                .to_string(),
            )
            .expect("seed settings");
            assert_eq!(
                goal_view("en", ""),
                "⚠️ The selected session does not match the current project. Choose the session again via /sessions."
            );
            // Matching session: arguments are rejected, bare input prompts.
            std::fs::write(
                &settings,
                serde_json::json!({
                    "currentProject": { "id": "dir_1", "worktree": "/work/proj" },
                    "currentSession": { "id": "ses_1", "title": "Old", "directory": "/work/proj" },
                })
                .to_string(),
            )
            .expect("seed settings");
            assert_eq!(
                goal_view("en", "do things"),
                "Use /goal without arguments, then follow the prompts or use the goal buttons."
            );
            assert_eq!(
                goal_view("en", ""),
                "Send the objective for this session in your next text message. Describe the result to achieve."
            );
        });
    }

    #[test]
    fn task_and_reload_views_read_isolated_settings() {
        with_isolated_home(|home| {
            let settings = home.join(crate::settings_current::SETTINGS_FILE_NAME);
            // No settings file: no project; reload reads defaults (success).
            assert_eq!(
                task_view("en", ""),
                "🏗 Project is not selected.\n\nFirst select a project with /projects."
            );
            assert_eq!(reload_view("en"), "✅ OpenCode configuration reloaded");
            std::fs::write(
                &settings,
                serde_json::json!({
                    "currentProject": { "id": "dir_1", "worktree": "/work/proj" },
                })
                .to_string(),
            )
            .expect("seed settings");
            assert!(task_view("en", "").starts_with("⏰ Send the task schedule"));
            let ack = task_view("en", "every day at 17:00 run tests");
            assert!(ack.starts_with("⏰ Task noted: every day at 17:00 run tests"));
            assert_eq!(reload_view("en"), "✅ OpenCode configuration reloaded");
            // Unparseable settings fail the reload.
            std::fs::write(&settings, "{not json").expect("seed invalid");
            assert_eq!(
                reload_view("en"),
                "🔴 Failed to reload OpenCode configuration"
            );
        });
    }

    #[test]
    fn open_ls_callbacks_match_node_shapes() {
        // `open:*` (`file-browser-menu.ts` parity).
        assert!(is_open_roots("open:roots"));
        assert!(!is_open_roots("open:nav:/a"));
        assert!(!is_open_roots("open:roots:extra"));
        assert_eq!(
            parse_open_nav_callback("open:nav:/work/proj"),
            Some("/work/proj")
        );
        assert_eq!(parse_open_nav_callback("open:nav:"), None);
        assert_eq!(parse_open_nav_callback("open:sel:/a"), None);
        assert_eq!(
            parse_open_select_callback("open:sel:/work/proj"),
            Some("/work/proj")
        );
        assert_eq!(parse_open_select_callback("open:sel:"), None);
        assert_eq!(parse_open_select_callback("open:nav:/a"), None);
        assert_eq!(
            parse_open_page_callback("open:pg:/work/proj|2"),
            Some(("/work/proj".to_string(), 2))
        );
        assert_eq!(parse_open_page_callback("open:pg:/work/proj"), None);
        assert_eq!(parse_open_page_callback("open:pg:|2"), None);
        assert_eq!(parse_open_page_callback("open:pg:/a|x"), None);
        assert_eq!(parse_open_page_callback("open:nav:/a"), None);
        // `ls:*` parity.
        assert_eq!(
            parse_ls_nav_callback("ls:nav:/work/proj/src"),
            Some("/work/proj/src")
        );
        assert_eq!(parse_ls_nav_callback("ls:nav:"), None);
        assert_eq!(parse_ls_nav_callback("ls:pg:/a|0"), None);
        assert_eq!(
            parse_ls_attach_callback("ls:attach:/work/proj/note.txt"),
            Some("/work/proj/note.txt")
        );
        assert_eq!(parse_ls_attach_callback("ls:attach:"), None);
        assert_eq!(
            parse_ls_download_callback("ls:download:/work/proj/note.txt"),
            Some("/work/proj/note.txt")
        );
        assert_eq!(parse_ls_download_callback("ls:download:"), None);
        assert_eq!(
            parse_ls_page_callback("ls:pg:/work/proj|1"),
            Some(("/work/proj".to_string(), 1))
        );
        assert_eq!(parse_ls_page_callback("ls:pg:/work/proj"), None);
        assert_eq!(
            parse_ls_file_callback("ls:file:/work/proj/note.txt|1"),
            Some(("/work/proj/note.txt".to_string(), 1))
        );
        assert_eq!(parse_ls_file_callback("ls:file:/a"), None);
        assert_eq!(
            parse_ls_back_callback("ls:back:/work/proj|0"),
            Some(("/work/proj".to_string(), 0))
        );
        assert_eq!(parse_ls_back_callback("ls:back:/work/proj"), None);
        assert_eq!(parse_ls_back_callback("ls:nav:/a"), None);
        // Page/file/back payloads never leak into the plain nav parsers.
        assert_eq!(parse_ls_nav_callback("ls:pg:/work/proj|1"), None);
        assert_eq!(parse_open_nav_callback("open:pg:/work/proj|1"), None);
        // The shared prefixes route every browser callback to its handler.
        for data in ["open:nav:/a", "open:sel:/a", "open:pg:/a|0", "open:roots"] {
            assert!(data.starts_with(OPEN_CALLBACK_PREFIX), "{data}");
        }
        for data in [
            "ls:nav:/a",
            "ls:pg:/a|0",
            "ls:file:/a|0",
            "ls:download:/a",
            "ls:attach:/a",
            "ls:back:/a|0",
        ] {
            assert!(data.starts_with(LS_CALLBACK_PREFIX), "{data}");
        }
        assert!(!"session:page:1".starts_with(OPEN_CALLBACK_PREFIX));
        assert!(!"session:page:1".starts_with(LS_CALLBACK_PREFIX));
    }

    #[test]
    fn open_ls_keyboards_carry_node_callback_data() {
        use teloxide_core::types::InlineKeyboardButtonKind;

        fn callback_data(markup: &InlineKeyboardMarkup) -> Vec<String> {
            markup
                .inline_keyboard
                .iter()
                .flat_map(|row| row.iter())
                .map(|button| match &button.kind {
                    InlineKeyboardButtonKind::CallbackData(data) => data.clone(),
                    other => panic!("expected callback data, got {other:?}"),
                })
                .collect()
        }

        // Roots: one `open:nav:<root>` row per root plus the cancel row
        // (Node `buildOpenRootsKeyboard` parity).
        let markup = build_open_roots_keyboard(
            &["/work/a".to_string(), "/work/b".to_string()],
            "/home/user",
            "en",
        );
        assert_eq!(markup.inline_keyboard.len(), 3);
        assert_eq!(
            callback_data(&markup),
            vec![
                "open:nav:/work/a".to_string(),
                "open:nav:/work/b".to_string(),
                "inline:cancel:open".to_string(),
            ]
        );

        // Browse: one `open:nav:<full>` row per entry, an up+roots row, a
        // pager row, then select + cancel (Node `buildOpenBrowseKeyboard`
        // parity).
        let markup = build_open_browse_keyboard(
            &["src".to_string()],
            "/work/proj",
            1,
            2,
            true,
            false,
            true,
            "en",
        );
        let data = callback_data(&markup);
        assert!(data.contains(&"open:nav:/work/proj/src".to_string()));
        assert!(data.contains(&"open:nav:/work".to_string()));
        assert!(data.contains(&"open:roots".to_string()));
        assert!(data.contains(&"open:pg:/work/proj|0".to_string()));
        assert!(data.contains(&"open:sel:/work/proj".to_string()));
        assert_eq!(data.last(), Some(&"inline:cancel:open".to_string()));

        // Ls browse: dirs carry `ls:nav:<full>`, files carry
        // `ls:file:<full>|<page>`, then up / pager / cancel rows (Node
        // `buildLsBrowseKeyboard` parity).
        let entries = vec![
            otb_core::file_browser_service_gates::LsEntry {
                full_path: "/work/proj/src".to_string(),
                name: "src".to_string(),
                is_dir: true,
            },
            otb_core::file_browser_service_gates::LsEntry {
                full_path: "/work/proj/note.txt".to_string(),
                name: "note.txt".to_string(),
                is_dir: false,
            },
        ];
        let markup = build_ls_browse_keyboard(&entries, "/work/proj/sub", 0, 2, true, false, "en");
        let data = callback_data(&markup);
        assert!(data.contains(&"ls:nav:/work/proj/src".to_string()));
        assert!(data.contains(&"ls:file:/work/proj/note.txt|0".to_string()));
        assert!(data.contains(&"ls:nav:/work/proj".to_string()));
        assert!(data.contains(&"ls:pg:/work/proj/sub|1".to_string()));
        assert_eq!(data.last(), Some(&"inline:cancel:ls".to_string()));

        // Ls file details: attach, download, back, cancel (Node
        // `buildLsFileDetailsKeyboard` parity); without attach the row is
        // hidden rather than erroring.
        let markup = build_ls_file_keyboard("/work/proj/note.txt", "/work/proj", 3, true, "en");
        assert_eq!(
            callback_data(&markup),
            vec![
                "ls:attach:/work/proj/note.txt".to_string(),
                "ls:download:/work/proj/note.txt".to_string(),
                "ls:back:/work/proj|3".to_string(),
                "inline:cancel:ls".to_string(),
            ]
        );
        let markup = build_ls_file_keyboard("/work/proj/note.txt", "/work/proj", 3, false, "en");
        assert_eq!(
            callback_data(&markup),
            vec![
                "ls:download:/work/proj/note.txt".to_string(),
                "ls:back:/work/proj|3".to_string(),
                "inline:cancel:ls".to_string(),
            ]
        );
    }

    // -- Attach quote/reply surface (telegram-ux slice) --

    fn attach_user_value(id: &str, text: &str, created: i64) -> serde_json::Value {
        serde_json::json!({
            "info": { "id": id, "role": "user", "time": { "created": created } },
            "parts": [{ "type": "text", "text": text }],
        })
    }

    fn attach_assistant_value(
        id: &str,
        text: &str,
        created: i64,
        parent: Option<&str>,
        completed: bool,
    ) -> serde_json::Value {
        let mut info =
            serde_json::json!({ "id": id, "role": "assistant", "time": { "created": created } });
        if let Some(parent) = parent {
            info["parentID"] = serde_json::Value::String(parent.to_string());
        }
        if completed {
            info["time"]["completed"] = serde_json::json!(created + 1);
        }
        serde_json::json!({ "info": info, "parts": [{ "type": "text", "text": text }] })
    }

    fn attach_pick_messages(
        values: &[serde_json::Value],
    ) -> Vec<otb_core::session_pick_catalog_gates::SessionPickMessage> {
        values
            .iter()
            .filter_map(session_pick_message_from_value)
            .collect()
    }

    #[test]
    fn attach_pick_converter_reads_pick_shape() {
        let values = vec![
            attach_user_value("user_1", "do the thing", 1),
            attach_assistant_value("asst_1", "done", 2, Some("user_1"), true),
            serde_json::json!({
                "info": { "id": "user_2", "role": "user", "time": { "created": 3 } },
                "parts": [{ "type": "file", "filename": "pic.png" }],
            }),
        ];
        let messages = attach_pick_messages(&values);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].info.role.as_deref(), Some("user"));
        assert_eq!(messages[1].info.parent_id.as_deref(), Some("user_1"));
        assert_eq!(messages[1].info.completed, Some(3));
        assert_eq!(messages[2].parts[0].kind, "file");
        // Entries without an `info` object never reach the pickers.
        assert!(session_pick_message_from_value(&serde_json::json!({})).is_none());
        assert!(
            session_pick_message_from_value(&serde_json::json!({ "info": "user_1" })).is_none()
        );
        // Untyped parts are skipped; the message itself still converts.
        let untyped = serde_json::json!({
            "info": { "id": "user_3", "role": "user" },
            "parts": [{ "text": "hi" }, { "type": "text", "text": "hi" }],
        });
        let message = session_pick_message_from_value(&untyped).expect("converts");
        assert_eq!(message.parts.len(), 1);
    }

    #[test]
    fn attach_busy_reads_map_and_list_shapes() {
        let busy_map = serde_json::json!({ "ses_1": { "type": "busy" } });
        assert!(attach_session_is_busy(&busy_map, "ses_1"));
        let idle_map = serde_json::json!({ "ses_1": { "type": "idle" } });
        assert!(!attach_session_is_busy(&idle_map, "ses_1"));
        assert!(!attach_session_is_busy(&busy_map, "ses_9"));
        assert!(!attach_session_is_busy(&busy_map, ""));
        let array = serde_json::json!(["ses_1"]);
        assert!(attach_session_is_busy(&array, "ses_1"));
        assert!(!attach_session_is_busy(&array, "ses_9"));
        let active_key = serde_json::json!({ "active": ["ses_1"] });
        assert!(attach_session_is_busy(&active_key, "ses_1"));
    }

    #[test]
    fn attach_missing_parent_only_when_absent() {
        let with_parent = attach_pick_messages(&[
            attach_user_value("user_1", "do the thing", 1),
            attach_assistant_value("asst_1", "done", 2, Some("user_1"), true),
        ]);
        assert_eq!(missing_attach_parent_id(&with_parent, false), None);
        let orphan = attach_pick_messages(&[attach_assistant_value(
            "asst_1",
            "done",
            2,
            Some("user_9"),
            true,
        )]);
        assert_eq!(
            missing_attach_parent_id(&orphan, false),
            Some("user_9".to_string())
        );
        let no_reply = attach_pick_messages(&[attach_user_value("user_1", "hi", 1)]);
        assert_eq!(missing_attach_parent_id(&no_reply, false), None);
    }

    #[test]
    fn attach_quote_carries_title_and_blockquote() {
        let messages = attach_pick_messages(&[attach_user_value("user_1", "do the thing", 1)]);
        let quote = build_attach_quote_text(&messages[0], "en").expect("quotes text input");
        assert!(quote.contains("Last user input:"));
        assert!(quote.contains("> do the thing"));
        assert!(quote.chars().count() <= TELEGRAM_TEXT_LIMIT);
        // A file-only input quotes its attachment line instead of text.
        let file_only = attach_pick_messages(&[serde_json::json!({
            "info": { "id": "user_2", "role": "user", "time": { "created": 2 } },
            "parts": [{ "type": "file", "filename": "pic.png" }],
        })]);
        let file_quote = build_attach_quote_text(&file_only[0], "en").expect("quotes file input");
        assert!(file_quote.contains("pic.png"));
        // Blank inputs quote nothing.
        let blank = attach_pick_messages(&[attach_user_value("user_3", "   ", 3)]);
        assert_eq!(build_attach_quote_text(&blank[0], "en"), None);
    }

    #[test]
    fn attach_reply_chunks_split_rarely_and_cleanly() {
        assert!(build_attach_reply_chunks("   ").is_empty());
        let short = build_attach_reply_chunks("done");
        assert_eq!(short, vec!["done".to_string()]);
        // Markdown normalization applies (headings render bold).
        let headed = build_attach_reply_chunks("# Title\n\nbody");
        assert_eq!(headed, vec!["**Title**\n\nbody".to_string()]);
        // Over the native budget the reply splits, never inside a word, and
        // rejoins exactly.
        let long = "word ".repeat(7_000);
        assert!(long.chars().count() > ATTACH_REPLY_CHUNK_LIMIT);
        let chunks = build_attach_reply_chunks(&long);
        assert_eq!(chunks.len(), 2);
        for chunk in &chunks {
            assert!(chunk.chars().count() <= ATTACH_REPLY_CHUNK_LIMIT);
        }
        let normalized =
            otb_core::markdown_normalizer::normalize_markdown_for_telegram_rendering(&long);
        assert_eq!(chunks.join(""), normalized);
    }

    #[test]
    fn attach_transcript_quotes_reply_and_waiting_input() {
        let messages = attach_pick_messages(&[
            attach_user_value("user_1", "do the thing", 1),
            attach_assistant_value("asst_1", "done", 2, Some("user_1"), true),
            attach_user_value("user_2", "and another", 3),
        ]);
        let transcript = build_attach_transcript(&messages, false, "en");
        let quote = transcript
            .quote
            .as_deref()
            .expect("quotes the parent input");
        assert!(quote.contains("> do the thing"));
        assert_eq!(transcript.reply_chunks, vec!["done".to_string()]);
        let waiting = transcript
            .waiting
            .as_deref()
            .expect("quotes the waiting input");
        assert!(waiting.contains("> and another"));
        assert!(!should_send_attach_empty(&transcript));
        assert!(should_show_attach_waiting(&transcript));
    }

    #[test]
    fn attach_transcript_without_reply_quotes_latest_prompt() {
        let messages = attach_pick_messages(&[attach_user_value("user_1", "hello?", 1)]);
        let transcript = build_attach_transcript(&messages, false, "en");
        assert!(transcript
            .quote
            .as_deref()
            .is_some_and(|quote| quote.contains("> hello?")));
        assert!(transcript.reply_chunks.is_empty());
        assert_eq!(transcript.waiting, None);
        assert!(!should_send_attach_empty(&transcript));
        assert!(!should_show_attach_waiting(&transcript));
    }

    #[test]
    fn attach_transcript_empty_without_messages() {
        let transcript = build_attach_transcript(&[], false, "en");
        assert_eq!(transcript.quote, None);
        assert!(transcript.reply_chunks.is_empty());
        assert!(should_send_attach_empty(&transcript));
        assert!(!should_show_attach_waiting(&transcript));
        assert_eq!(
            build_attach_empty_text("en"),
            crate::locale::translate("en", "sessions.preview.empty")
        );
    }

    #[test]
    fn attach_busy_session_skips_incomplete_reply() {
        let messages = attach_pick_messages(&[
            attach_user_value("user_1", "do the thing", 1),
            attach_assistant_value("asst_1", "partial…", 2, Some("user_1"), false),
        ]);
        // Busy: the still-running reply is not shown as the full reply.
        let busy = build_attach_transcript(&messages, true, "en");
        assert!(busy.reply_chunks.is_empty());
        // Idle: the same transcript resolves normally.
        let idle = build_attach_transcript(&messages, false, "en");
        assert_eq!(idle.reply_chunks, vec!["partial…".to_string()]);
    }

    fn footer_completed_run() -> RunCloseCompletedRun {
        RunCloseCompletedRun {
            has_completed_response: true,
            suppress_footer: false,
            configured_agent: Some("build".to_string()),
            configured_provider_id: Some("openai".to_string()),
            configured_model_id: Some("gpt-5".to_string()),
            actual_agent: None,
            actual_provider_id: None,
            actual_model_id: None,
            started_at_ms: 1_000,
            steps: vec![],
            interrupted: false,
        }
    }

    #[test]
    fn run_close_footer_needs_setting_response_and_identity() {
        // Failing-test-first for the unwired footer: setting on, answered
        // turn, full identity builds the line (TS `run-close.ts:31-61`).
        let footer = resolve_run_close_footer(true, Some(&footer_completed_run()), 5_000, "en")
            .expect("footer");
        assert!(footer.contains("Build"), "agent display missing: {footer}");
        assert!(footer.contains("openai/gpt-5"), "model missing: {footer}");
        // Disabled setting, missing run, aborted turn, suppressed turn, and
        // incomplete identity all stay footerless (TS `run-close.ts:31-44`).
        assert_eq!(
            resolve_run_close_footer(false, Some(&footer_completed_run()), 5_000, "en"),
            None
        );
        assert_eq!(resolve_run_close_footer(true, None, 5_000, "en"), None);
        let mut aborted = footer_completed_run();
        aborted.has_completed_response = false;
        assert_eq!(
            resolve_run_close_footer(true, Some(&aborted), 5_000, "en"),
            None
        );
        let mut suppressed = footer_completed_run();
        suppressed.suppress_footer = true;
        assert_eq!(
            resolve_run_close_footer(true, Some(&suppressed), 5_000, "en"),
            None
        );
        let mut nameless = footer_completed_run();
        nameless.configured_agent = Some("  ".to_string());
        assert_eq!(
            resolve_run_close_footer(true, Some(&nameless), 5_000, "en"),
            None
        );
    }

    #[test]
    fn run_close_footer_prefers_actual_identity_and_marks_interruption() {
        // `actual || configured` wins when set (TS `run-close.ts:39-41`);
        // a client stop after the reply marks the interruption.
        let mut run = footer_completed_run();
        run.actual_agent = Some("plan".to_string());
        run.actual_provider_id = Some("anthropic".to_string());
        run.actual_model_id = Some("claude-x".to_string());
        run.interrupted = true;
        let footer = resolve_run_close_footer(true, Some(&run), 5_000, "en").expect("footer");
        assert!(footer.contains("Plan"), "actual agent should win: {footer}");
        assert!(
            footer.contains("anthropic/claude-x"),
            "actual model should win: {footer}"
        );
        assert!(
            footer.contains("interrupted"),
            "interrupted mark missing: {footer}"
        );
        // Elapsed saturates instead of going negative on clock skew.
        let zeroed = resolve_run_close_footer(true, Some(&run), 500, "en").expect("footer");
        assert!(zeroed.contains("0s"), "elapsed should saturate: {zeroed}");
    }

    #[test]
    fn session_idle_footer_keeps_shutdown_and_suppressed_turns_footerless() {
        // Failing-test-first: TS `session-lifecycle-handler.ts:70-91` keeps
        // shutdown, suppressed, and interrupted-without-response idles
        // footerless even with a completed snapshot; the surviving path
        // matches the run-close gate.
        let run = footer_completed_run();
        assert_eq!(
            resolve_session_idle_footer(true, Some(&run), 5_000, "en", true, false, false),
            None
        );
        assert_eq!(
            resolve_session_idle_footer(true, Some(&run), 5_000, "en", false, true, false),
            None
        );
        assert_eq!(
            resolve_session_idle_footer(true, Some(&run), 5_000, "en", false, false, true),
            None
        );
        assert!(
            resolve_session_idle_footer(true, Some(&run), 5_000, "en", false, false, false)
                .is_some()
        );
        assert_eq!(
            resolve_session_idle_footer(false, Some(&run), 5_000, "en", false, false, false),
            None
        );
    }

    #[test]
    fn install_idle_footer_hook_zero_chat_id_installs_nothing() {
        // Guard parity with `install_card_hooks`: the deny-all sentinel (no
        // DM target) installs no hook, so no footer send can spawn without
        // a target chat.
        crate::attach::set_session_idle_footer_hook(None);
        let bot = create_bot(DUMMY_TOKEN);
        install_session_idle_footer_hook(&bot, 0, "en", true);
        assert!(!crate::attach::has_session_idle_footer_hook());
        install_session_idle_footer_hook(&bot, 7, "en", true);
        assert!(crate::attach::has_session_idle_footer_hook());
        crate::attach::set_session_idle_footer_hook(None);
    }

    fn pinned_test_data() -> PinnedDashboardData {
        PinnedDashboardData {
            session_title: "Fix login".to_string(),
            goal_status: None,
            goal_objective: None,
            project_path: "/repo".to_string(),
            branch: None,
            worktree_path: None,
            is_linked_worktree: false,
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-5".to_string()),
            variant: None,
            tokens_used: 100,
            tokens_limit: Some(200_000),
            cost: Some(1.5),
            changed_files: vec![PinnedChangedFile {
                file: "/repo/a.ts".to_string(),
                additions: 3,
                deletions: 1,
            }],
        }
    }

    #[test]
    fn pinned_patch_is_additive_and_keeps_last_on_failed_refresh() {
        // Tokens replace (`input + cacheRead`); cost deltas add; a failed
        // diff refresh (`None`) keeps the last good list while an empty one
        // removes the section (TS `onSessionDiff` / `applyDirtyFileRefresh`).
        let mut data = pinned_test_data();
        let changed = apply_pinned_patch(
            &mut data,
            &PinnedDashboardPatch {
                tokens_used: Some(500),
                cost_delta: Some(2.25),
                changed_files: None,
                ..Default::default()
            },
        );
        assert!(changed);
        assert_eq!(data.tokens_used, 500);
        assert_eq!(data.cost, Some(3.75));
        assert_eq!(data.changed_files.len(), 1);
        // Zero and NaN deltas cannot change the total (TS cost gate).
        let files = data.changed_files.clone();
        let unchanged = apply_pinned_patch(
            &mut data,
            &PinnedDashboardPatch {
                cost_delta: Some(0.0),
                changed_files: Some(files),
                session_title: Some(String::new()),
                ..Default::default()
            },
        );
        assert!(!unchanged);
        assert_eq!(data.cost, Some(3.75));
        assert_eq!(data.session_title, "Fix login");
        let cleared = apply_pinned_patch(
            &mut data,
            &PinnedDashboardPatch {
                changed_files: Some(vec![]),
                session_title: Some("Fix login v2".to_string()),
                ..Default::default()
            },
        );
        assert!(cleared);
        assert!(data.changed_files.is_empty());
        assert_eq!(data.session_title, "Fix login v2");
    }

    #[test]
    fn resolve_pinned_text_gates_disabled_and_stateless() {
        // Disabled dashboard or no message state never draws (TS
        // `updatePinnedMessage` early return); a merged snapshot renders.
        let data = pinned_test_data();
        assert_eq!(resolve_pinned_text(&data, false, true, "en"), None);
        assert_eq!(resolve_pinned_text(&data, true, false, "en"), None);
        let text = resolve_pinned_text(&data, true, true, "en").expect("text");
        assert!(text.contains("Fix login"), "title missing: {text}");
        assert!(!should_send_pinned_edit(Some(&text), &text));
        assert!(should_send_pinned_edit(None, &text));
        assert!(!should_send_pinned_edit(Some("old"), "  "));
    }

    #[test]
    fn install_pinned_hook_zero_chat_id_installs_nothing() {
        // Guard parity with `install_session_idle_footer_hook`: the
        // deny-all sentinel installs no hook, so no dashboard send can
        // spawn without a target chat.
        crate::attach::set_pinned_dashboard_hook(None);
        let bot = create_bot(DUMMY_TOKEN);
        install_pinned_dashboard_hook(&bot, 0, "en", true);
        assert!(!crate::attach::has_pinned_dashboard_hook());
        install_pinned_dashboard_hook(&bot, 7, "en", true);
        assert!(crate::attach::has_pinned_dashboard_hook());
        crate::attach::set_pinned_dashboard_hook(None);
    }
}
