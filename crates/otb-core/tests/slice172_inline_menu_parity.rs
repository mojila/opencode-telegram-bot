//! Slice 172 parity: inline-menu gates.
//!
//! Mirrors the pure TS contract in
//! `tests/bot/menus/inline-menu.test.ts` (the unified inline-menu flow over
//! `src/bot/menus/inline-menu.ts` plus
//! `src/bot/callbacks/inline-menu-cancel-callback-handler.ts`):
//! - the cancel callback data (`inline:cancel:<menuKind>`, the legacy
//!   `compact:cancel` reading as the `context` menu)
//! - the cancel-button label key (`inline.button.close` for `settings`,
//!   `inline.button.cancel` everywhere else)
//! - the trailing-empty-row trim (pop empty rows, separator row only onto
//!   a non-empty keyboard, so no empty row is ever created)
//! - the cancel-callback route (missing/empty payloads, unknown kinds, and
//!   non-cancel payloads are not handled)
//! - the active-menu gate (inline state + known kind + numeric message id
//!   all matching the expected kind and callback message; everything else
//!   is stale)
//! - the clear gate (only an `inline` state is cleared on cancel)
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `ctx.reply` / `answerCallbackQuery` / `deleteMessage` delivery, the
//!   `.catch(() => {})` swallow, `logger.debug` emission,
//!   `interactionManager` `start`/`getSnapshot`/`clear` I/O, the
//!   `InlineKeyboard` assembly (callers)
//! - the reply's `message_id` -> interaction `messageId` plumbing and the
//!   caller-supplied `cancelButtonText` override (callers)
//! - `cancelMenu` delivery of the `common.cancelled` answer (callers;
//!   the key itself is asserted below)
//! - locale strings for the `inline.*` / `common.cancelled` keys (i18n
//!   owners)

use otb_core::inline_menu_gates::*;

// ---- constants (4) ----

#[test]
fn inline_cancel_prefix_value_parity() {
    assert_eq!(INLINE_MENU_CANCEL_PREFIX, "inline:cancel:");
}

#[test]
fn inline_legacy_cancel_value_parity() {
    assert_eq!(LEGACY_CONTEXT_CANCEL_CALLBACK, "compact:cancel");
}

#[test]
fn inline_menu_kinds_table_parity() {
    // TS: `INLINE_MENU_KINDS` — the eleven menus sharing the unified
    // cancel flow.
    assert_eq!(INLINE_MENU_KINDS.len(), 11);
    for kind in [
        "project", "session", "recent", "model", "agent", "variant", "context", "open", "ls",
        "worktree", "settings",
    ] {
        assert!(is_inline_menu_kind(kind), "missing kind: {kind}");
    }
    assert!(!is_inline_menu_kind("compact"));
    assert!(!is_inline_menu_kind(""));
}

#[test]
fn inline_key_values_parity() {
    assert_eq!(INLINE_BUTTON_CANCEL_KEY, "inline.button.cancel");
    assert_eq!(INLINE_BUTTON_CLOSE_KEY, "inline.button.close");
    assert_eq!(INLINE_INACTIVE_CALLBACK_KEY, "inline.inactive_callback");
    assert_eq!(COMMON_CANCELLED_KEY, "common.cancelled");
}

// ---- cancel callback data (2) ----

#[test]
fn inline_project_cancel_data_parity() {
    // TS: the cancel button on a project keyboard carries
    // `inline:cancel:project` (test lines 57-67).
    assert_eq!(
        inline_cancel_callback_data("project"),
        "inline:cancel:project"
    );
}

#[test]
fn inline_model_and_context_cancel_data_parity() {
    // TS: the reply appends `inline:cancel:model` (test line 113) and the
    // caller-supplied close label keeps `inline:cancel:context` (test line
    // 138).
    assert_eq!(inline_cancel_callback_data("model"), "inline:cancel:model");
    assert_eq!(
        inline_cancel_callback_data("context"),
        "inline:cancel:context"
    );
}

// ---- cancel-button label key (3) ----

#[test]
fn inline_project_cancel_reads_cancel_key_parity() {
    // TS: an ordinary menu reads `t("inline.button.cancel")` (test lines
    // 57-67).
    assert_eq!(cancel_button_key("project"), "inline.button.cancel");
}

#[test]
fn inline_settings_cancel_reads_close_key_parity() {
    // TS: the settings menu reads `t("inline.button.close")` instead
    // (test lines 69-79).
    assert_eq!(cancel_button_key("settings"), "inline.button.close");
}

#[test]
fn inline_other_menus_read_cancel_key_parity() {
    // TS: the settings branch is exact-match, so every other unified kind
    // reads the cancel key.
    for kind in ["session", "recent", "model", "open", "ls", "worktree"] {
        assert_eq!(cancel_button_key(kind), "inline.button.cancel");
    }
}

// ---- trailing-empty-row trim (4) ----

#[test]
fn inline_trailing_separator_row_is_trimmed_parity() {
    // TS: a keyboard ending in a `.row()` separator has that empty row
    // popped, so the cancel button lands on row 1 with no empty rows left
    // (test lines 81-88).
    assert_eq!(trailing_empty_row_count(&[1, 0]), 1);
    assert_eq!(trailing_empty_row_count(&[2, 1, 0, 0]), 2);
}

#[test]
fn inline_non_empty_tail_trims_nothing_parity() {
    // TS: without a trailing separator the `while` loop breaks
    // immediately.
    assert_eq!(trailing_empty_row_count(&[1]), 0);
    assert_eq!(trailing_empty_row_count(&[1, 2]), 0);
}

#[test]
fn inline_empty_keyboard_trims_nothing_parity() {
    assert_eq!(trailing_empty_row_count(&[]), 0);
}

#[test]
fn inline_separator_row_only_on_non_empty_keyboard_parity() {
    // TS: `if (keyboard.inline_keyboard.length > 0) keyboard.row()` — a
    // non-empty keyboard (after the trim) gets a separator row; an empty
    // keyboard takes the cancel button directly.
    assert!(needs_cancel_separator_row(1));
    assert!(needs_cancel_separator_row(2));
    assert!(!needs_cancel_separator_row(0));
}

// ---- cancel-callback route (5) ----

#[test]
fn inline_missing_and_empty_payloads_are_unhandled_parity() {
    // TS: `if (!data) return false` covers both a missing payload and
    // `""`.
    assert_eq!(resolve_inline_cancel_kind(None), None);
    assert_eq!(resolve_inline_cancel_kind(Some("")), None);
}

#[test]
fn inline_legacy_compact_cancel_reads_context_parity() {
    // TS: `compact:cancel` resolves the `context` menu (test lines
    // 200-216).
    assert_eq!(
        resolve_inline_cancel_kind(Some("compact:cancel")),
        Some("context")
    );
}

#[test]
fn inline_prefixed_known_kind_resolves_parity() {
    // TS: `inline:cancel:variant` resolves `variant` (test lines 180-198).
    assert_eq!(
        resolve_inline_cancel_kind(Some("inline:cancel:variant")),
        Some("variant")
    );
    assert_eq!(
        resolve_inline_cancel_kind(Some("inline:cancel:settings")),
        Some("settings")
    );
}

#[test]
fn inline_prefixed_unknown_kind_is_unhandled_parity() {
    // TS: a prefixed payload with an unknown kind returns false.
    assert_eq!(
        resolve_inline_cancel_kind(Some("inline:cancel:compact")),
        None
    );
    assert_eq!(resolve_inline_cancel_kind(Some("inline:cancel:")), None);
}

#[test]
fn inline_non_cancel_payload_is_unhandled_parity() {
    // TS: any other payload (e.g. a menu's own callback) returns false.
    assert_eq!(resolve_inline_cancel_kind(Some("session:abc")), None);
    assert_eq!(resolve_inline_cancel_kind(Some("project:1")), None);
}

// ---- active-menu gate (5) ----

#[test]
fn inline_active_callback_accepted_parity() {
    // TS: inline state for the `session` menu with message 99 accepts the
    // callback on message 99 with no answer (test lines 141-157).
    assert!(is_active_inline_menu(
        true,
        Some("session"),
        Some(99),
        "session",
        Some(99)
    ));
}

#[test]
fn inline_kind_mismatch_is_stale_parity() {
    // TS: inline state for `project` rejects a `session` callback with the
    // inactive alert (test lines 159-178).
    assert!(!is_active_inline_menu(
        true,
        Some("project"),
        Some(10),
        "session",
        Some(10)
    ));
}

#[test]
fn inline_message_mismatch_is_stale_parity() {
    // TS: the gate triple-matches kind *and* message id, so a callback
    // from another message is stale even with the right kind.
    assert!(!is_active_inline_menu(
        true,
        Some("session"),
        Some(99),
        "session",
        Some(100)
    ));
}

#[test]
fn inline_non_inline_state_is_stale_parity() {
    // TS: `getActiveInlineMenuMetadata` returns null unless the state
    // kind is `inline`.
    assert!(!is_active_inline_menu(
        false,
        Some("session"),
        Some(99),
        "session",
        Some(99)
    ));
}

#[test]
fn inline_unknown_kind_and_missing_ids_are_stale_parity() {
    // TS: the active `menuKind` must itself be a known kind and the
    // `messageId` a number, and the callback must carry a message.
    assert!(!is_active_inline_menu(
        true,
        Some("compact"),
        Some(99),
        "compact",
        Some(99)
    ));
    assert!(!is_active_inline_menu(
        true,
        None,
        Some(99),
        "session",
        Some(99)
    ));
    assert!(!is_active_inline_menu(
        true,
        Some("session"),
        None,
        "session",
        Some(99)
    ));
    assert!(!is_active_inline_menu(
        true,
        Some("session"),
        Some(99),
        "session",
        None
    ));
}

// ---- clear gate (2) ----

#[test]
fn inline_state_clears_on_cancel_parity() {
    // TS: after the cancel route accepts, an `inline` state is cleared
    // (test lines 180-198 and 200-216 assert a null snapshot).
    assert!(should_clear_inline_menu(Some("inline")));
}

#[test]
fn inline_non_inline_state_never_clears_parity() {
    // TS: `clearActiveInlineMenu` leaves any other state alone.
    assert!(!should_clear_inline_menu(Some("question")));
    assert!(!should_clear_inline_menu(None));
}
