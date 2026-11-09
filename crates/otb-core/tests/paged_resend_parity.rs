//! Paged delete+resend parity: `resendPagedMenu` pure gates.
//!
//! Mirrors the pure TS contract in `src/bot/menus/inline-menu.ts` lines
//! 105-137:
//! - page turns resend (delete, then reply, then re-point the interaction
//!   at the reply's `message_id`)
//! - detail / confirm / file-detail / toggle screens stay on
//!   `editMessageText` (never resend)
//! - the merge re-points `messageId` at the sent id while the patch wins
//!   on `page` / `ids` and everything else survives
//! - the transition only runs when a snapshot exists
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `deleteMessage` / `ctx.reply` / `transition` delivery, the
//!   `.catch(() => {})` swallow, the preceding `answerCallbackQuery`
//!   (callers)
//! - the open/ls pagination codec (`file_browser_menu_gates`, slice 56)
//! - the active-menu gate the re-pointed id feeds (`inline_menu_gates`,
//!   slice 172)

use otb_core::paged_resend_gates::{
    next_paged_message_metadata, should_resend_page, should_transition_after_resend,
    PagedMenuMetadata, PagedMetadataPatch, PagedResendStep, PAGED_RESEND_STEPS,
};

fn session_list_metadata() -> PagedMenuMetadata {
    PagedMenuMetadata {
        menu_kind: "session".to_string(),
        message_id: 99,
        page: Some(0),
        ids: Vec::new(),
    }
}

// ---- delivery order (1) ----

#[test]
fn paged_resend_deletes_before_reply_before_transition_parity() {
    // TS: `deleteMessage` (line 123), then `ctx.reply` (lines 124-129),
    // then `transition` (lines 130-135) — a page turn never edits the
    // deleted message and never leaves the interaction behind.
    assert_eq!(
        PAGED_RESEND_STEPS,
        [
            PagedResendStep::DeleteOldMessage,
            PagedResendStep::ReplyFreshPage,
            PagedResendStep::TransitionToSentId,
        ]
    );
}

// ---- page-turn gate (2) ----

#[test]
fn paged_resend_page_turn_requires_resend_parity() {
    // TS: all twelve page-turn branches (session list/search, projects
    // list/search, messages list/back, commands list, skills list,
    // worktree list, open navigate, ls navigate) go through
    // `resendPagedMenu`.
    assert!(should_resend_page(true));
}

#[test]
fn paged_resend_detail_and_confirm_stay_on_edit_parity() {
    // TS guardrail: the message detail view, the command/skill confirm
    // views, the ls file-detail view, the settings toggles, and the MCP
    // detail view use `editMessageText` and must never resend.
    assert!(!should_resend_page(false));
}

// ---- metadata merge (5) ----

#[test]
fn paged_resend_message_id_repoints_to_sent_id_parity() {
    // TS: `messageId: sent.message_id` overwrites the stale id so the
    // next page click passes the active-menu check.
    let next = next_paged_message_metadata(
        &session_list_metadata(),
        100,
        &PagedMetadataPatch::default(),
    );
    assert_eq!(next.message_id, 100);
    assert_eq!(next.menu_kind, "session");
}

#[test]
fn paged_resend_catalog_page_patch_applies_parity() {
    // TS: `metadataPatch: { page: normalizedPage }` on the commands /
    // skills / messages list pages.
    let next = next_paged_message_metadata(
        &session_list_metadata(),
        100,
        &PagedMetadataPatch {
            page: Some(2),
            ids: None,
        },
    );
    assert_eq!(next.message_id, 100);
    assert_eq!(next.page, Some(2));
    assert!(next.ids.is_empty());
}

#[test]
fn paged_resend_search_ids_patch_applies_parity() {
    // TS: `metadataPatch: { ids: ... }` on the session / project search
    // pages preserves the fresh result set on the new message.
    let next = next_paged_message_metadata(
        &session_list_metadata(),
        101,
        &PagedMetadataPatch {
            page: None,
            ids: Some(vec!["s1".to_string(), "s2".to_string()]),
        },
    );
    assert_eq!(next.message_id, 101);
    assert_eq!(next.page, Some(0));
    assert_eq!(next.ids, vec!["s1".to_string(), "s2".to_string()]);
}

#[test]
fn paged_resend_absent_patch_keeps_old_page_and_ids_parity() {
    // TS: list pages with no `metadataPatch` (plain session / project /
    // worktree / open / ls turns) keep the old page and ids and only
    // re-point the message id.
    let old = PagedMenuMetadata {
        menu_kind: "worktree".to_string(),
        message_id: 50,
        page: Some(1),
        ids: vec!["w1".to_string()],
    };
    let next = next_paged_message_metadata(&old, 51, &PagedMetadataPatch::default());
    assert_eq!(
        next,
        PagedMenuMetadata {
            menu_kind: "worktree".to_string(),
            message_id: 51,
            page: Some(1),
            ids: vec!["w1".to_string()],
        }
    );
}

#[test]
fn paged_resend_menu_kind_survives_resend_parity() {
    // TS: the `{ ...snapshot.metadata, ... }` spread carries the untouched
    // keys (`menuKind`, `flow`, `stage`, directories, item rows) onto the
    // new message.
    let next = next_paged_message_metadata(
        &session_list_metadata(),
        100,
        &PagedMetadataPatch {
            page: Some(3),
            ids: Some(Vec::new()),
        },
    );
    assert_eq!(next.menu_kind, "session");
    assert_eq!(next.message_id, 100);
}

// ---- snapshot guard (1) ----

#[test]
fn paged_resend_transitions_only_with_snapshot_parity() {
    // TS: `if (snapshot)` — the transition is skipped when no snapshot
    // exists, while the fresh reply is already sent either way.
    assert!(should_transition_after_resend(true));
    assert!(!should_transition_after_resend(false));
}
