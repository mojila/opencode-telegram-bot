//! Paged-menu delete+resend gates (`resendPagedMenu` parity).
//!
//! Rust port of the pure sync remainder in
//! `src/bot/menus/inline-menu.ts` lines 105-137 (`ResendPagedMenuOptions`
//! / `resendPagedMenu`):
//! - the delivery order (delete the stale list message *before* replying
//!   the fresh page, then point the stored interaction at the reply's
//!   `message_id` so the next page click still passes the active-menu
//!   check)
//! - the page-turn gate (page turns resend; detail / confirm / file /
//!   toggle screens stay on `editMessageText`)
//! - the metadata merge (`{ ...snapshot.metadata, ...metadataPatch,
//!   messageId: sent.message_id }`: the patch wins on `page` / `ids`,
//!   everything else survives, `messageId` always re-points)
//! - the snapshot guard (the transition only runs when a snapshot exists)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/bot/menus/inline-menu.ts` lines 118-137 (`resendPagedMenu`:
//!   `deleteMessage`, then `ctx.reply`, then `transition` with the spread
//!   merge above; the caller answers the callback query first).
//! - Page-turn call sites (all twelve go through `resendPagedMenu`):
//!   `session-callback-handler.ts` list page (~507) + search page (~188),
//!   `project-callback-handler.ts` list page (~77) + search page (~258),
//!   `message-history-callback-handler.ts` list page (~389) + back to
//!   list (~347), `command-catalog-callback-handler.ts` list page (~421),
//!   `skills-catalog-callback-handler.ts` list page (~243),
//!   `worktree-callback-handler.ts` list page (~101),
//!   `file-browser-callback-handler.ts` open `navigateOpenTo` (~182),
//!   ls `navigateLsTo` (~314), open roots (~162); plus the model
//!   `renderModelMenuScreen` manual delete+reply equivalent.
//! - Screens that must stay `editMessageText` (never resend): the message
//!   detail view (~410-413), the command / skill confirm views, the ls
//!   file-detail view, the settings toggles, the MCP detail view.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx` I/O (`deleteMessage` with the `.catch(() => {})`
//!   swallow, `ctx.reply` with the optional `parse_mode`, the preceding
//!   `answerCallbackQuery`).
//! - `interactionManager` I/O (`getSnapshot`, `transition`); the delete
//!   and the reply carry no interaction write, and a missing snapshot
//!   skips the transition without failing the resend.
//! - `InlineKeyboard` assembly and `t(key)` resolution (keys travel here
//!   as literal key strings).
//!
//! Already covered elsewhere and NOT re-tested here:
//! - the open/ls pagination codec (path-index encode/decode, `|<page>`
//!   suffix, page-range math): ported in `file_browser_menu_gates`
//!   (slice 56, GREEN).
//! - the active-menu gate the re-pointed `messageId` feeds back into:
//!   ported in `inline_menu_gates` (slice 172, GREEN).
//! - locale strings for the page-error / empty-page answers (i18n
//!   owners); the per-menu `metadataPatch` value sources (menu owners).
//!
//! Deviations: none intended. The TS `metadataPatch` is a
//! `Record<string, unknown>`; only the `page` / `ids` keys are ever
//! patched by the twelve call sites (`{ page }` on catalog pages,
//! `{ ids }` on search pages, absent elsewhere), so the port models
//! exactly those two optional patch keys while every other key
//! (`menuKind`, `flow`, `stage`, directories, item rows) is represented
//! by the surviving `menu_kind` carrier. `messageId` is `i64` here per
//! the inline-menu precedent.

/// Ordered delivery behind `resendPagedMenu` (TS: `deleteMessage`, then
/// `ctx.reply`, then `interactionManager.transition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagedResendStep {
    /// Drop the stale list message first (TS line 123).
    DeleteOldMessage,
    /// Send the fresh page as a new message (TS lines 124-129).
    ReplyFreshPage,
    /// Point the stored interaction at the reply (TS lines 130-135).
    TransitionToSentId,
}

/// The pinned delivery order: the delete lands before the reply, and the
/// transition lands last, so a page turn never edits a deleted message
/// and never leaves the interaction pointing at one.
pub const PAGED_RESEND_STEPS: [PagedResendStep; 3] = [
    PagedResendStep::DeleteOldMessage,
    PagedResendStep::ReplyFreshPage,
    PagedResendStep::TransitionToSentId,
];

/// Page-turn gate (TS: the twelve page-turn branches call
/// `resendPagedMenu`, while detail / confirm / file-detail / toggle
/// branches call `editMessageText`): only a page turn resends.
pub fn should_resend_page(is_page_turn: bool) -> bool {
    is_page_turn
}

/// Surviving paged-menu interaction metadata (TS: the
/// `snapshot.metadata` spread — `menuKind` stands in for the untouched
/// keys while `page` / `ids` are the two keys a `metadataPatch` may
/// overwrite).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagedMenuMetadata {
    /// Untouched menu key carried through the resend (TS: `menuKind`,
    /// plus `flow` / `stage` / directories / item rows).
    pub menu_kind: String,
    /// Active-menu message id, re-pointed at each resend (TS:
    /// `messageId`).
    pub message_id: i64,
    /// Current list page (TS: `page`, patched by catalog pages).
    pub page: Option<i64>,
    /// Current result ids (TS: `ids`, patched by search pages).
    pub ids: Vec<String>,
}

/// Optional per-resend overwrite (TS: `ResendPagedMenuOptions`
/// `metadataPatch` — `{ page }`, `{ ids }`, or absent).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PagedMetadataPatch {
    /// Replacement page; `None` keeps the old page.
    pub page: Option<i64>,
    /// Replacement ids; `None` keeps the old ids.
    pub ids: Option<Vec<String>>,
}

/// Next interaction metadata after a resend (TS:
/// `{ ...snapshot.metadata, ...metadataPatch, messageId: sent.message_id }`):
/// the patch wins on `page` / `ids`, `messageId` always re-points at the
/// fresh reply, everything else survives.
pub fn next_paged_message_metadata(
    old: &PagedMenuMetadata,
    sent_message_id: i64,
    patch: &PagedMetadataPatch,
) -> PagedMenuMetadata {
    PagedMenuMetadata {
        menu_kind: old.menu_kind.clone(),
        message_id: sent_message_id,
        page: patch.page.or(old.page),
        ids: patch.ids.clone().unwrap_or_else(|| old.ids.clone()),
    }
}

/// Snapshot guard (TS: `if (snapshot)` around the `transition`): the
/// interaction is only re-pointed when a snapshot exists; the fresh
/// reply itself is already sent either way.
pub fn should_transition_after_resend(has_snapshot: bool) -> bool {
    has_snapshot
}
