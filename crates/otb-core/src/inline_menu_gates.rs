//! Inline-menu gates (slice 172, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/bot/menus/inline-menu.test.ts` (the unified inline-menu flow over
//! `src/bot/menus/inline-menu.ts` plus
//! `src/bot/callbacks/inline-menu-cancel-callback-handler.ts`):
//! - the cancel callback data (`inline:cancel:<menuKind>`; the legacy
//!   `compact:cancel` payload reads as the `context` menu)
//! - the cancel-button label key (`inline.button.close` for `settings`,
//!   `inline.button.cancel` everywhere else)
//! - the trailing-empty-row trim (pop empty rows before appending the
//!   cancel button, then add a separator row only onto a non-empty
//!   keyboard)
//! - the cancel-callback route (a missing/empty payload, an unknown kind,
//!   and any non-cancel payload are not handled)
//! - the active-menu gate (the interaction state is `inline`, its
//!   `menuKind` is a known kind, its `messageId` is a number, and both
//!   match the expected kind and the callback's message; everything else
//!   is stale)
//! - the clear gate (only an `inline` state is cleared on cancel)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/bot/menus/inline-menu.ts` lines 7-8 (cancel prefix + legacy
//!   callback constants), 10-22 (`INLINE_MENU_KINDS`), 42-44
//!   (`isInlineMenuKind`), 80-82 (cancel callback data build), 84-103
//!   (`appendInlineMenuCancelButton`: trailing-empty-row trim, separator
//!   row, settings/close label default), 145-172
//!   (`ensureActiveInlineMenu`: active triple-match, otherwise the stale
//!   `inline.inactive_callback` alert), 174-179 (`clearActiveInlineMenu`:
//!   clear only an `inline` state).
//! - `src/bot/callbacks/inline-menu-cancel-callback-handler.ts` lines
//!   18-36 (cancel route: no data -> unhandled, legacy -> `context`,
//!   prefixed + known kind -> kind, prefixed + unknown kind -> unhandled,
//!   other -> unhandled; then the `ensureActiveInlineMenu` check, clear,
//!   and `cancelMenu`).
//! - `src/bot/callbacks/feedback.ts` lines 65-75 (`cancelMenu` answers
//!   `common.cancelled`).
//! - `tests/bot/menus/inline-menu.test.ts` lines 57-67 (cancel button on a
//!   project keyboard), 69-79 (close label for settings), 81-88 (trailing
//!   separator trimmed, no empty rows, cancel lands on row 1), 90-124
//!   (reply appends the `inline:cancel:model` callback and registers the
//!   inline interaction), 126-139 (caller-supplied close label keeps the
//!   `inline:cancel:context` callback), 141-157 (active callback accepted
//!   with no answer), 159-178 (kind mismatch is stale with the inactive
//!   alert), 180-198 (unified cancel handled and state cleared), 200-216
//!   (legacy compact cancel handled and state cleared).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`ctx.reply`, `answerCallbackQuery`,
//!   `deleteMessage`, the `.catch(() => {})` swallow, `logger.debug`
//!   emission).
//! - `interactionManager` I/O (`start` with the reply's `message_id`,
//!   `getSnapshot`, `clear`) and the `InlineKeyboard` assembly
//!   (`.text().row()` mutation; rows travel here as row lengths).
//! - `t(key)` resolution (keys travel here as literal key strings) and
//!   the caller-supplied `cancelButtonText` override (the settings/close
//!   *default* is the gate; an explicit override bypasses it).
//! - `cancelMenu` delivery (answers `common.cancelled`, caller-owned).
//!
//! Paged delete+resend delivery (`resendPagedMenu`,
//! `src/bot/menus/inline-menu.ts:118-137`) is out of scope here and lives
//! with the future dispatcher/polling caller in `otb-bot/src/telegram.rs`,
//! gated by `crate::paged_resend_gates` (`DeleteOldMessage` ->
//! `ReplyFreshPage` -> `TransitionToSentId`). Scope guardrail: page turns
//! (`:page:` + back-to-list + model/open/ls page navigations) resend,
//! while detail / confirm / file-detail / toggle screens stay on edit.
//!
//! Already covered elsewhere and NOT re-tested here:
//! - locale strings for the `inline.*` / `common.cancelled` keys (i18n
//!   owners).
//! - the `InteractionState` / `InteractionManager` primitives (the
//!   interaction-state owner; this slice takes scalar snapshots as input).
//!
//! Deviations: none intended. The TS `!data` guard treats both a missing
//! callback payload and `""` as unhandled, mirroring the
//! `Option<&str>`-with-`Some("")`-as-absent precedent from the reload
//! slice; the cancel data is a fixed-prefix strip rather than a split, so
//! no `parseInt`/`Number()` semantics apply.
//!
//! Step 2 (GREEN): pure sync logic ported from the TS contract asserted
//! by the parity tests in
//! `crates/otb-core/tests/slice172_inline_menu_parity.rs`.

/// Unified inline-menu cancel callback prefix
/// (TS: `INLINE_MENU_CANCEL_PREFIX`).
pub const INLINE_MENU_CANCEL_PREFIX: &str = "inline:cancel:";

/// Legacy context-menu cancel callback data, read as the `context` menu
/// (TS: `LEGACY_CONTEXT_CANCEL_CALLBACK`).
pub const LEGACY_CONTEXT_CANCEL_CALLBACK: &str = "compact:cancel";

/// Cancel-button label key for ordinary menus
/// (TS: `t("inline.button.cancel")`, the `❌ Cancel` line).
pub const INLINE_BUTTON_CANCEL_KEY: &str = "inline.button.cancel";

/// Cancel-button label key for the settings menu
/// (TS: `t("inline.button.close")`, the `❌ Close` line).
pub const INLINE_BUTTON_CLOSE_KEY: &str = "inline.button.close";

/// Stale-callback alert key (TS: `t("inline.inactive_callback")`).
pub const INLINE_INACTIVE_CALLBACK_KEY: &str = "inline.inactive_callback";

/// Cancelled-answer key (TS: `cancelMenu` answers `common.cancelled`).
pub const COMMON_CANCELLED_KEY: &str = "common.cancelled";

/// Menu kinds sharing the unified cancel flow
/// (TS: `INLINE_MENU_KINDS`).
pub const INLINE_MENU_KINDS: [&str; 11] = [
    "project", "session", "recent", "model", "agent", "variant", "context", "open", "ls",
    "worktree", "settings",
];

/// Membership over the unified menu kinds (TS: `isInlineMenuKind`).
pub fn is_inline_menu_kind(value: &str) -> bool {
    INLINE_MENU_KINDS.contains(&value)
}

/// Cancel callback data for one menu kind
/// (TS: `` `${INLINE_MENU_CANCEL_PREFIX}${menuKind}` ``).
pub fn inline_cancel_callback_data(menu_kind: &str) -> String {
    format!("{INLINE_MENU_CANCEL_PREFIX}{menu_kind}")
}

/// Default cancel-button label key for one menu kind (TS: the
/// `appendInlineMenuCancelButton` default — `settings` reads the close
/// key, every other kind reads the cancel key; an explicit
/// `cancelButtonText` override bypasses this and stays with the caller).
pub fn cancel_button_key(menu_kind: &str) -> &'static str {
    if menu_kind == "settings" {
        INLINE_BUTTON_CLOSE_KEY
    } else {
        INLINE_BUTTON_CANCEL_KEY
    }
}

/// Resolved menu kind behind an inline-menu cancel payload (TS: the
/// `handleInlineMenuCancel` route — the legacy payload reads as
/// `context`, a prefixed payload reads as its kind when known; a
/// missing/empty payload, a prefixed-but-unknown kind, and any other
/// payload are not handled).
pub fn resolve_inline_cancel_kind(data: Option<&str>) -> Option<&str> {
    match data {
        None => None,
        Some(payload) if payload.is_empty() => None,
        Some(payload) if payload == LEGACY_CONTEXT_CANCEL_CALLBACK => Some("context"),
        Some(payload) => match payload.strip_prefix(INLINE_MENU_CANCEL_PREFIX) {
            Some(kind) if is_inline_menu_kind(kind) => Some(kind),
            _ => None,
        },
    }
}

/// Active-menu gate (TS: `ensureActiveInlineMenu` — the interaction state
/// is `inline`, its `menuKind` is a known kind, its `messageId` is a
/// number, and both match the expected kind and the callback's message;
/// everything else is stale and the caller answers
/// `inline.inactive_callback` with an alert).
pub fn is_active_inline_menu(
    state_is_inline: bool,
    active_kind: Option<&str>,
    active_message_id: Option<i64>,
    expected_kind: &str,
    callback_message_id: Option<i64>,
) -> bool {
    if !state_is_inline {
        return false;
    }
    let kind = match active_kind {
        Some(kind) if is_inline_menu_kind(kind) => kind,
        _ => return false,
    };
    if kind != expected_kind {
        return false;
    }
    match (active_message_id, callback_message_id) {
        (Some(active), Some(callback)) => active == callback,
        _ => false,
    }
}

/// Trailing empty rows `appendInlineMenuCancelButton` pops before
/// appending the cancel button (TS: the `while` loop over
/// `keyboard.inline_keyboard`, modelled here as row lengths).
pub fn trailing_empty_row_count(row_lens: &[usize]) -> usize {
    let mut count = 0;
    for len in row_lens.iter().rev() {
        if *len != 0 {
            break;
        }
        count += 1;
    }
    count
}

/// Separator gate (TS: `if (keyboard.inline_keyboard.length > 0)
/// keyboard.row()` — a separator row is inserted only onto a non-empty
/// keyboard after the trim, so the cancel button never creates an empty
/// row on its own).
pub fn needs_cancel_separator_row(remaining_rows: usize) -> bool {
    remaining_rows > 0
}

/// Clear gate (TS: `clearActiveInlineMenu` clears only an `inline` state;
/// the cancel route calls it after the active-menu check, so it normally
/// fires).
pub fn should_clear_inline_menu(state_kind: Option<&str>) -> bool {
    matches!(state_kind, Some("inline"))
}
