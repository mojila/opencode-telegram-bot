//! Slice 166 parity: context compaction flow gates.
//!
//! Mirrors the pure TS contract in
//! `tests/bot/callbacks/context-control-callback-handler.test.ts`
//! (the "context compaction flow" over
//! `src/bot/callbacks/context-control-callback-handler.ts` +
//! `src/bot/callbacks/inline-menu-cancel-callback-handler.ts` for the
//! `context` menu):
//! - exact dispatch data (`compact:details` / `compact:confirm` /
//!   `inline:cancel:context`; missing or foreign data is not this flow)
//! - details shows the confirmation only from the `details` stage, and an
//!   early `compact:confirm` stays inactive so nothing summarizes
//! - a callback from a different message is stale and edits nothing
//! - cancel clears the flow from either stage without compacting
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `context_inline_gates` (the `compact:details` / `compact:confirm`
//!   atoms, the details/confirm stage checks, the missing-session gate)
//! - `worktree_history_inline_gates` (the cancel-data kind table and the
//!   legacy `compact:cancel` mapping)
//! - `formatters::extra_formatters::format_session_title` (the empty
//!   title -> default-session-title fallback)
//! - grammy delivery, `interactionManager` effects, service reads, `t()`
//!   lookups, answer-before-edit await ordering (callers)

use otb_core::context_control_flow_gates::*;

// ---- dispatch data (6) ----

#[test]
fn compact_details_data_value_parity() {
    assert_eq!(COMPACT_DETAILS_DATA, "compact:details");
}

#[test]
fn compact_confirm_data_value_parity() {
    assert_eq!(COMPACT_CONFIRM_DATA, "compact:confirm");
}

#[test]
fn context_cancel_data_value_parity() {
    assert_eq!(CONTEXT_CANCEL_DATA, "inline:cancel:context");
}

#[test]
fn context_menu_kind_value_parity() {
    assert_eq!(CONTEXT_MENU_KIND, "context");
}

#[test]
fn compact_details_stage_value_parity() {
    assert_eq!(DETAILS_STAGE, "details");
}

#[test]
fn compact_confirm_stage_value_parity() {
    assert_eq!(CONFIRM_STAGE, "confirm");
}

// ---- dispatch guards (3) ----

#[test]
fn compact_details_guard_matches_only_details_parity() {
    assert!(handles_compact_details(Some("compact:details")));
    assert!(!handles_compact_details(Some("compact:confirm")));
    assert!(!handles_compact_details(Some("inline:cancel:context")));
    assert!(!handles_compact_details(None));
    assert!(!handles_compact_details(Some("")));
}

#[test]
fn compact_confirm_guard_matches_only_confirm_parity() {
    assert!(handles_compact_confirm(Some("compact:confirm")));
    assert!(!handles_compact_confirm(Some("compact:details")));
    assert!(!handles_compact_confirm(Some("inline:cancel:context")));
    assert!(!handles_compact_confirm(None));
    assert!(!handles_compact_confirm(Some("")));
}

#[test]
fn context_cancel_guard_matches_only_context_cancel_parity() {
    // Scoped cancel only: other menus and the legacy data are not this flow.
    assert!(handles_context_cancel(Some("inline:cancel:context")));
    assert!(!handles_context_cancel(Some("inline:cancel:session")));
    assert!(!handles_context_cancel(Some("compact:cancel")));
    assert!(!handles_context_cancel(Some("compact:details")));
    assert!(!handles_context_cancel(None));
    assert!(!handles_context_cancel(Some("")));
}

// ---- message freshness (2) ----

#[test]
fn compact_message_fresh_when_ids_match_parity() {
    assert!(is_compact_message_fresh(Some(7), Some(7)));
}

#[test]
fn compact_message_stale_or_missing_parity() {
    // Stale callback from a different message edits nothing.
    assert!(!is_compact_message_fresh(Some(8), Some(7)));
    // Missing callback message or no active menu is never fresh.
    assert!(!is_compact_message_fresh(None, Some(7)));
    assert!(!is_compact_message_fresh(Some(7), None));
    assert!(!is_compact_message_fresh(None, None));
}

// ---- flow decision (7) ----

#[test]
fn compact_flow_details_shows_confirm_parity() {
    assert_eq!(
        decide_compact_flow(Some("compact:details"), Some("details"), true),
        CompactFlowAction::ShowConfirm
    );
}

#[test]
fn compact_flow_early_confirm_stays_inactive_parity() {
    // Confirming from the details stage compacts nothing (no summarize).
    assert_eq!(
        decide_compact_flow(Some("compact:confirm"), Some("details"), true),
        CompactFlowAction::Inactive
    );
}

#[test]
fn compact_flow_repeat_details_stays_inactive_parity() {
    // Details again once the flow already moved to confirm: no second menu.
    assert_eq!(
        decide_compact_flow(Some("compact:details"), Some("confirm"), true),
        CompactFlowAction::Inactive
    );
}

#[test]
fn compact_flow_confirm_compacts_parity() {
    assert_eq!(
        decide_compact_flow(Some("compact:confirm"), Some("confirm"), true),
        CompactFlowAction::Compact
    );
}

#[test]
fn compact_flow_stale_message_is_inactive_parity() {
    assert_eq!(
        decide_compact_flow(Some("compact:details"), Some("details"), false),
        CompactFlowAction::Inactive
    );
    assert_eq!(
        decide_compact_flow(Some("compact:confirm"), Some("confirm"), false),
        CompactFlowAction::Inactive
    );
    assert_eq!(
        decide_compact_flow(Some("inline:cancel:context"), Some("details"), false),
        CompactFlowAction::Inactive
    );
}

#[test]
fn compact_flow_cancel_clears_either_stage_parity() {
    // Cancel closes details or confirmation without compacting.
    assert_eq!(
        decide_compact_flow(Some("inline:cancel:context"), Some("details"), true),
        CompactFlowAction::Cancel
    );
    assert_eq!(
        decide_compact_flow(Some("inline:cancel:context"), Some("confirm"), true),
        CompactFlowAction::Cancel
    );
}

#[test]
fn compact_flow_ignores_foreign_data_parity() {
    assert_eq!(
        decide_compact_flow(Some("recent:0"), Some("details"), true),
        CompactFlowAction::Ignore
    );
    assert_eq!(
        decide_compact_flow(Some(""), Some("details"), true),
        CompactFlowAction::Ignore
    );
    assert_eq!(
        decide_compact_flow(None, Some("details"), true),
        CompactFlowAction::Ignore
    );
    assert_eq!(
        decide_compact_flow(None, None, false),
        CompactFlowAction::Ignore
    );
}
