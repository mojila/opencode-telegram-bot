//! `recent_callback_gates` (slice 92, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/recent-callback-handler.ts`
//!   (`handleRecentSelect` dispatch: the `!data?.startsWith("recent:")`
//!   guard, the digits-only index gate (`/^\d+$/`, otherwise `-1` ->
//!   inactive), the `sessionIds`/`directories` array gates plus the
//!   `typeof sessionId !== "string" || typeof directory !== "string"`
//!   `inactive_callback` gate, the session-fetch usability gate (`error`
//!   / missing session / `session.directory !== directory` /
//!   `session.parentID` present answers `inactive_callback`), the
//!   foreground-busy `replyBusyBlocked` gate).
//! - `src/bot/menus/recent-selection-menu.ts` (the `recent:` callback
//!   prefix; already classified in `recent_scheduled_gates`, repeated
//!   here only as the dispatch table the handler matches).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `recent_scheduled_gates::RECENT_CALLBACK_PREFIX`,
//!   `parse_recent_index_callback`, `build_recent_callback`,
//!   `recent_status_glyph`, `shorten_project`, `project_labels`,
//!   `project_names`, `build_recent_button_label` (the menu label math
//!   and the digits-only index parse behind the dispatch below).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, transitions, `clear`).
//! - `opencodeClient.session.get` fetch, `getCurrentProject` reads,
//!   `switchToProject` / `selectSessionById` calls,
//!   `ensureActiveInlineMenu` / `resetInteractions`.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `messageId` values are not compared here
//! (the handler relies on the active-inline-menu guard, not a message-id
//! check). The digits-only index parse lives in `recent_scheduled_gates`;
//! this slice takes the already-parsed `Option<u64>` (`None` = the TS
//! `-1` inactive path) so the resolve gate stays self-contained. `Symbol`
//! tokens are not modelled here (staleness arrives as explicit bools).
//!
//! Step 2 (GREEN): pure sync logic ported from the TS sources above.
//! Verified by `crates/otb-core/tests/slice92_recent_callback_parity.rs`.

/// `recent:<index>` callback prefix.
pub const RECENT_CB_PREFIX: &str = "recent:";
/// Foreground-busy answer key (TS: `replyBusyBlocked` -> `t("bot.session_busy")`).
pub const RECENT_BUSY_KEY: &str = "bot.session_busy";
/// Stale/mismatched selection answer key (TS: `t("inline.inactive_callback")`).
pub const RECENT_INACTIVE_KEY: &str = "inline.inactive_callback";
/// Catch-all select error key (TS: `failure(ctx, "sessions.select_error")`).
pub const RECENT_SELECT_ERROR_KEY: &str = "sessions.select_error";

/// Resolved recent selection (TS: the `sessionId` + `directory` pair read
/// off `metadata.sessionIds` / `metadata.directories`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentSelection {
    pub session_id: String,
    pub directory: String,
}

/// Recent-callback dispatch guard (TS: `if (!data?.startsWith("recent:"))
/// return false` in `handleRecentSelect`).
pub fn handles_recent_callback(_data: Option<&str>) -> bool {
    match _data {
        Some(data) => data.starts_with(RECENT_CB_PREFIX),
        None => false,
    }
}

/// Recent index selection behind `recent:<n>` (TS: `indexText` digits-only
/// gate then `Array.isArray(ids) ? ids[index] : undefined` plus the
/// `Array.isArray(directories) ? directories[index] : undefined` read;
/// a missing array, an out-of-bounds index, or a `None` index (the TS `-1`
/// path) reads as `None`, and the caller answers `inactive_callback`).
pub fn resolve_recent_selection(
    _ids: Option<&[String]>,
    _directories: Option<&[String]>,
    _index: Option<u64>,
) -> Option<RecentSelection> {
    let index: usize = usize::try_from(_index?).ok()?;
    let ids = _ids?;
    let directories = _directories?;
    let session_id = ids.get(index)?.clone();
    let directory = directories.get(index)?.clone();
    Some(RecentSelection {
        session_id,
        directory,
    })
}

/// Fetched-session usability gate (TS: `error || !session ||
/// session.directory !== directory || session.parentID` answers
/// `inactive_callback`; only an existing leaf session in the same
/// directory proceeds).
pub fn is_recent_session_usable(
    _session_exists: bool,
    _directory_matches: bool,
    _has_parent: bool,
) -> bool {
    _session_exists && _directory_matches && !_has_parent
}

/// Combined inactive gate for the recent select (TS: the `typeof
/// sessionId !== "string" || typeof directory !== "string"` check plus
/// the fetched-session usability check above).
pub fn should_answer_recent_inactive(_has_selection: bool, _session_usable: bool) -> bool {
    !_has_selection || !_session_usable
}

/// Foreground-busy gate (TS: `isForegroundBusy(deps)` answers
/// `replyBusyBlocked` before the menu guard).
pub fn should_block_recent_when_busy(_is_busy: bool) -> bool {
    _is_busy
}
