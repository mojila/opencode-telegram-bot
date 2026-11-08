//! Slice 85 parity: rename callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/rename-callback-handler.ts` (`handleRenameCancel`
//! dispatch plus the not-waiting / wrong-interaction / stale-message
//! gates; `handleRenameTextAnswer` routing plus the wrong-interaction /
//! missing-session / empty-title gates; `getCallbackMessageId`;
//! `clearRenameInteraction`), `src/bot/menus/rename-menu.ts`
//! (`RENAME_CANCEL_CALLBACK`), and
//! `src/app/managers/rename-manager.ts` (`isActiveMessage`,
//! `isWaitingForName`, `getSessionInfo`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `rename_callback_gates` is ported (only the constant-value tests
//! pass).

use otb_core::rename_callback_gates::*;

// ---- constants ----

#[test]
fn rename_cancel_callback_value_parity() {
    assert_eq!(RENAME_CANCEL_CALLBACK, "rename:cancel");
}

#[test]
fn rename_interaction_kind_value_parity() {
    assert_eq!(RENAME_INTERACTION_KIND, "rename");
}

// ---- cancel dispatch ----

#[test]
fn handles_cancel_none_parity() {
    assert!(!handles_rename_cancel_callback(None));
}

#[test]
fn handles_cancel_empty_parity() {
    assert!(!handles_rename_cancel_callback(Some("")));
}

#[test]
fn handles_cancel_exact_parity() {
    assert!(handles_rename_cancel_callback(Some("rename:cancel")));
}

#[test]
fn handles_cancel_rejects_other_suffix_parity() {
    assert!(!handles_rename_cancel_callback(Some("rename:confirm")));
    assert!(!handles_rename_cancel_callback(Some("rename:")));
}

#[test]
fn handles_cancel_rejects_case_variant_parity() {
    assert!(!handles_rename_cancel_callback(Some("Rename:cancel")));
}

// ---- callback message id ----

#[test]
fn message_id_none_without_message_parity() {
    assert_eq!(callback_message_id(false, MessageIdRaw::Number(7)), None);
}

#[test]
fn message_id_none_without_field_parity() {
    assert_eq!(callback_message_id(true, MessageIdRaw::Missing), None);
}

#[test]
fn message_id_none_for_non_number_parity() {
    assert_eq!(callback_message_id(true, MessageIdRaw::NonNumber), None);
}

#[test]
fn message_id_some_for_number_parity() {
    assert_eq!(
        callback_message_id(true, MessageIdRaw::Number(42)),
        Some(42)
    );
}

#[test]
fn message_id_zero_passthrough_parity() {
    assert_eq!(callback_message_id(true, MessageIdRaw::Number(0)), Some(0));
}

// ---- clear guard ----

#[test]
fn clear_on_rename_kind_parity() {
    assert!(should_clear_rename_interaction(Some("rename")));
}

#[test]
fn no_clear_on_other_kind_parity() {
    assert!(!should_clear_rename_interaction(Some("task")));
    assert!(!should_clear_rename_interaction(Some("question")));
}

#[test]
fn no_clear_without_interaction_parity() {
    assert!(!should_clear_rename_interaction(None));
}

#[test]
fn no_clear_on_empty_kind_parity() {
    assert!(!should_clear_rename_interaction(Some("")));
}

// ---- cancel branches ----

#[test]
fn cancel_inactive_when_not_waiting_parity() {
    assert_eq!(
        classify_rename_cancel(false, Some("rename"), true),
        RenameCancelOutcome::InactiveNotWaiting
    );
    assert_eq!(
        classify_rename_cancel(false, Some("task"), false),
        RenameCancelOutcome::InactiveNotWaiting
    );
}

#[test]
fn cancel_wrong_interaction_when_active_elsewhere_parity() {
    assert_eq!(
        classify_rename_cancel(true, Some("task"), true),
        RenameCancelOutcome::InactiveWrongInteraction
    );
}

#[test]
fn cancel_wrong_interaction_without_snapshot_parity() {
    assert_eq!(
        classify_rename_cancel(true, None, false),
        RenameCancelOutcome::InactiveWrongInteraction
    );
}

#[test]
fn cancel_stale_when_message_mismatch_parity() {
    assert_eq!(
        classify_rename_cancel(true, Some("rename"), false),
        RenameCancelOutcome::InactiveStaleMessage
    );
}

#[test]
fn cancel_proceeds_when_active_parity() {
    assert_eq!(
        classify_rename_cancel(true, Some("rename"), true),
        RenameCancelOutcome::ProceedCancel
    );
}

// ---- text routing ----

#[test]
fn route_text_false_when_not_waiting_parity() {
    assert!(!should_route_rename_text(false, Some("New title")));
    assert!(!should_route_rename_text(false, None));
}

#[test]
fn route_text_false_without_text_parity() {
    assert!(!should_route_rename_text(true, None));
}

#[test]
fn route_text_false_for_slash_command_parity() {
    assert!(!should_route_rename_text(true, Some("/new")));
    assert!(!should_route_rename_text(true, Some("/")));
}

#[test]
fn route_text_true_for_plain_title_parity() {
    assert!(should_route_rename_text(true, Some("New title")));
}

#[test]
fn route_text_true_for_empty_string_parity() {
    // `""` is defined text that does not start with `/`: it routes and
    // fails later at the empty-title gate.
    assert!(should_route_rename_text(true, Some("")));
}

// ---- text branches ----

#[test]
fn text_wrong_interaction_takes_precedence_parity() {
    assert_eq!(
        classify_rename_text(Some("task"), false, ""),
        RenameTextOutcome::WrongInteraction
    );
    assert_eq!(
        classify_rename_text(None, false, ""),
        RenameTextOutcome::WrongInteraction
    );
}

#[test]
fn text_missing_session_without_info_parity() {
    assert_eq!(
        classify_rename_text(Some("rename"), false, "New title"),
        RenameTextOutcome::MissingSession
    );
}

#[test]
fn text_empty_title_rejected_parity() {
    assert_eq!(
        classify_rename_text(Some("rename"), true, ""),
        RenameTextOutcome::EmptyTitle
    );
}

#[test]
fn text_whitespace_title_rejected_parity() {
    // The caller passes the already-trimmed title; a blank input
    // arrives as `""` after `trim()`.
    assert_eq!(
        classify_rename_text(Some("rename"), true, normalize_rename_title("   ").as_str()),
        RenameTextOutcome::EmptyTitle
    );
}

#[test]
fn text_proceeds_when_valid_parity() {
    assert_eq!(
        classify_rename_text(Some("rename"), true, "New title"),
        RenameTextOutcome::ProceedRename
    );
}

// ---- title normalizer ----

#[test]
fn normalize_trims_spaces_parity() {
    assert_eq!(normalize_rename_title("  New title  "), "New title");
}

#[test]
fn normalize_trims_newlines_parity() {
    assert_eq!(normalize_rename_title("\n\tNew title\n"), "New title");
}

#[test]
fn normalize_keeps_inner_spacing_parity() {
    assert_eq!(normalize_rename_title("  a  b  "), "a  b");
}

#[test]
fn normalize_empty_stays_empty_parity() {
    assert_eq!(normalize_rename_title(""), "");
    assert_eq!(normalize_rename_title("   "), "");
}

// ---- slash guard ----

#[test]
fn command_slash_prefix_parity() {
    assert!(is_rename_command_text("/new"));
    assert!(is_rename_command_text("/cancel"));
}

#[test]
fn command_bare_slash_parity() {
    assert!(is_rename_command_text("/"));
}

#[test]
fn command_leading_space_not_command_parity() {
    assert!(!is_rename_command_text(" /new"));
}

#[test]
fn command_plain_text_not_command_parity() {
    assert!(!is_rename_command_text("New title"));
    assert!(!is_rename_command_text(""));
}

// ---- active message ----

#[test]
fn active_message_match_parity() {
    assert!(is_active_rename_message(Some(7), Some(7)));
}

#[test]
fn active_message_mismatch_parity() {
    assert!(!is_active_rename_message(Some(7), Some(8)));
}

#[test]
fn active_message_none_stored_parity() {
    assert!(!is_active_rename_message(None, Some(7)));
}

#[test]
fn active_message_none_candidate_parity() {
    assert!(!is_active_rename_message(Some(7), None));
}

#[test]
fn active_message_both_none_parity() {
    assert!(!is_active_rename_message(None, None));
}

// ---- session presence ----

#[test]
fn has_session_with_id_parity() {
    assert!(has_rename_session(Some("abc123")));
}

#[test]
fn has_session_none_missing_parity() {
    assert!(!has_rename_session(None));
}

#[test]
fn has_session_empty_missing_parity() {
    assert!(!has_rename_session(Some("")));
}

#[test]
fn has_session_whitespace_kept_parity() {
    // TS only checks `!state.sessionId`: a non-empty string, even
    // blank-looking, counts as present; trimming is the title's job.
    assert!(has_rename_session(Some("  ")));
}
