//! Slice 103 parity: attach-restore orchestration pure gates (RED).
//!
//! Mirrors the pure TS contract in `src/app/services/attach-service.ts`
//! (`attachToSession` restore gate, busy-vs-idle mark choice,
//! `restorePendingQuestions` show-vs-wait gate,
//! `restorePendingPermissions` question-active wait branch,
//! `listPendingQuestions` / `listPendingPermissions` error-or-no-data
//! skip, `belongsToFollowedSession` missing-chain gate,
//! `isQuestionTracked` / `isPermissionTracked` OR gates,
//! `settleRequestsGoneWhileDisconnected` waiting-drop, shown-question
//! close and restart-vs-outside choice, shown-permission settle and
//! apply gates, `restoreAttachedCurrentSession` join and failed-join
//! retry gates, `performAttachedCurrentSessionRestore` missing-state and
//! directory-mismatch skips, `dropSavedSessionIfMissing` not-found gate,
//! `restorePendingInteractionsAfterReconnect` detached skip and
//! restart-only missed-turn gate, `startMissedLiveTurn`
//! no-error-plus-busy gate, `detachAttachedSession` detached skip,
//! `markAttachedSessionBusy` / `markAttachedSessionIdle` unchanged-mark
//! skip, `syncPinnedAttachState` missing-presentation skip).
//! `getAttachBusyStatus` already lives in `attach_foreground_gates` and
//! is NOT re-tested here; live question/permission flow and event-handler
//! dispatch live in `question_permission_flow_gates` /
//! `event_handler_gates` and are NOT re-tested here (this slice takes
//! their outcomes as bools).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `attach_restore_gates` is ported.

use otb_core::attach_restore_gates::*;

// ---- restore gate (5) ----

#[test]
fn restores_pending_when_fresh_and_idle_parity() {
    assert!(should_restore_pending(false, false, false, false));
}

#[test]
fn restores_pending_when_forced_on_attached_parity() {
    assert!(should_restore_pending(true, true, false, false));
}

#[test]
fn skips_restore_when_attached_without_force_parity() {
    assert!(!should_restore_pending(true, false, false, false));
}

#[test]
fn skips_restore_when_question_active_parity() {
    assert!(!should_restore_pending(false, false, true, false));
}

#[test]
fn skips_restore_when_permission_active_parity() {
    assert!(!should_restore_pending(false, false, false, true));
}

// ---- restored question show-vs-wait (3) ----

#[test]
fn shows_restored_question_when_idle_and_started_parity() {
    assert!(should_show_restored_question(false, true));
}

#[test]
fn waits_restored_question_when_active_parity() {
    assert!(!should_show_restored_question(true, true));
}

#[test]
fn waits_restored_question_when_start_failed_parity() {
    assert!(!should_show_restored_question(false, false));
}

// ---- restored permission wait branch (2) ----

#[test]
fn waits_restored_permission_when_question_active_parity() {
    assert!(should_wait_restored_permission(true));
}

#[test]
fn shows_restored_permission_when_no_question_parity() {
    assert!(!should_wait_restored_permission(false));
}

// ---- pending-list skip (3) ----

#[test]
fn skips_pending_restore_on_error_parity() {
    assert!(should_skip_pending_restore(true, true));
}

#[test]
fn skips_pending_restore_on_no_data_parity() {
    assert!(should_skip_pending_restore(false, false));
}

#[test]
fn keeps_pending_restore_on_data_parity() {
    assert!(!should_skip_pending_restore(false, true));
}

// ---- followed-session chain (2) ----

#[test]
fn follows_request_with_chain_parity() {
    assert!(is_followed_request(true));
}

#[test]
fn drops_request_without_chain_parity() {
    assert!(!is_followed_request(false));
}

// ---- question tracked (3) ----

#[test]
fn question_tracked_when_shown_matches_parity() {
    assert!(is_question_tracked(true, false));
}

#[test]
fn question_tracked_when_waiting_parity() {
    assert!(is_question_tracked(false, true));
}

#[test]
fn question_untracked_when_neither_parity() {
    assert!(!is_question_tracked(false, false));
}

// ---- permission tracked (4) ----

#[test]
fn permission_tracked_when_has_request_parity() {
    assert!(is_permission_tracked(true, false, false));
}

#[test]
fn permission_tracked_when_resolved_parity() {
    assert!(is_permission_tracked(false, true, false));
}

#[test]
fn permission_tracked_when_waiting_parity() {
    assert!(is_permission_tracked(false, false, true));
}

#[test]
fn permission_untracked_when_none_parity() {
    assert!(!is_permission_tracked(false, false, false));
}

// ---- waiting drop while gone (2) ----

#[test]
fn drops_waiting_missing_from_pending_parity() {
    assert!(should_drop_waiting_while_gone(false));
}

#[test]
fn keeps_waiting_still_pending_parity() {
    assert!(!should_drop_waiting_while_gone(true));
}

// ---- shown-question close (6) ----

#[test]
fn closes_shown_question_gone_and_current_and_quiet_parity() {
    assert!(should_close_shown_question(true, false, true, false, true));
}

#[test]
fn keeps_shown_question_still_pending_parity() {
    assert!(!should_close_shown_question(true, true, true, false, true));
}

#[test]
fn keeps_shown_question_when_none_on_screen_parity() {
    assert!(!should_close_shown_question(false, false, false, false, true));
}

#[test]
fn keeps_shown_question_when_current_changed_parity() {
    assert!(!should_close_shown_question(true, false, false, false, true));
}

#[test]
fn keeps_shown_question_when_settling_from_telegram_parity() {
    assert!(!should_close_shown_question(true, false, true, true, true));
}

#[test]
fn keeps_shown_question_without_presentation_parity() {
    assert!(!should_close_shown_question(
        true, false, true, false, false
    ));
}

// ---- restart-vs-outside close choice (2) ----

#[test]
fn closes_question_not_answered_on_restart_parity() {
    assert!(should_close_question_as_not_answered(true));
}

#[test]
fn closes_question_settled_outside_otherwise_parity() {
    assert!(!should_close_question_as_not_answered(false));
}

// ---- shown-permission settle (3) ----

#[test]
fn settles_shown_permission_gone_but_settleable_parity() {
    assert!(should_settle_shown_permission(false, true));
}

#[test]
fn keeps_shown_permission_still_pending_parity() {
    assert!(!should_settle_shown_permission(true, true));
}

#[test]
fn keeps_shown_permission_no_longer_settleable_parity() {
    assert!(!should_settle_shown_permission(false, false));
}

// ---- settle-changes apply (3) ----

#[test]
fn applies_permission_settle_changes_with_presentation_parity() {
    assert!(should_apply_permission_settle_changes(2, true));
}

#[test]
fn skips_permission_settle_changes_without_changes_parity() {
    assert!(!should_apply_permission_settle_changes(0, true));
}

#[test]
fn skips_permission_settle_changes_without_presentation_parity() {
    assert!(!should_apply_permission_settle_changes(2, false));
}

// ---- restore join / retry (5) ----

#[test]
fn joins_restore_in_flight_parity() {
    assert!(should_join_restore(true));
}

#[test]
fn starts_restore_when_none_in_flight_parity() {
    assert!(!should_join_restore(false));
}

#[test]
fn retries_after_failed_join_with_no_retry_parity() {
    assert!(should_retry_joined_restore(false, false));
}

#[test]
fn no_retry_after_successful_join_parity() {
    assert!(!should_retry_joined_restore(true, false));
    assert!(!should_retry_joined_restore(true, true));
}

#[test]
fn no_retry_when_other_already_retried_parity() {
    assert!(!should_retry_joined_restore(false, true));
}

// ---- startup-restore skips (5) ----

#[test]
fn skips_startup_restore_without_project_parity() {
    assert!(should_skip_startup_restore(false, true));
}

#[test]
fn skips_startup_restore_without_session_parity() {
    assert!(should_skip_startup_restore(true, false));
}

#[test]
fn continues_startup_restore_with_both_parity() {
    assert!(!should_skip_startup_restore(true, true));
}

#[test]
fn skips_restore_on_directory_mismatch_parity() {
    assert!(should_skip_restore_on_mismatch("/a", "/b"));
}

#[test]
fn continues_restore_when_directories_match_parity() {
    assert!(!should_skip_restore_on_mismatch("/a", "/a"));
}

// ---- missing-session drop (2) ----

#[test]
fn drops_missing_session_on_not_found_parity() {
    assert!(should_drop_missing_session(true));
}

#[test]
fn keeps_session_when_found_parity() {
    assert!(!should_drop_missing_session(false));
}

// ---- reconnect restore (4) ----

#[test]
fn skips_reconnect_restore_when_detached_parity() {
    assert!(should_skip_reconnect_restore(false));
}

#[test]
fn continues_reconnect_restore_when_attached_parity() {
    assert!(!should_skip_reconnect_restore(true));
}

#[test]
fn calls_start_missed_live_turn_on_restart_parity() {
    assert!(should_call_start_missed_live_turn(true));
}

#[test]
fn skips_start_missed_live_turn_without_restart_parity() {
    assert!(!should_call_start_missed_live_turn(false));
}

// ---- missed live turn (3) ----

#[test]
fn starts_missed_live_turn_when_busy_without_error_parity() {
    assert!(should_start_missed_live_turn(false, true));
}

#[test]
fn skips_missed_live_turn_on_error_parity() {
    assert!(!should_start_missed_live_turn(true, true));
}

#[test]
fn skips_missed_live_turn_when_idle_parity() {
    assert!(!should_start_missed_live_turn(false, false));
}

// ---- detach / mark / sync (6) ----

#[test]
fn skips_detach_when_nothing_attached_parity() {
    assert!(should_skip_detach(false));
}

#[test]
fn continues_detach_when_attached_parity() {
    assert!(!should_skip_detach(true));
}

#[test]
fn syncs_attach_state_when_mark_changed_parity() {
    assert!(should_sync_attach_state(true));
}

#[test]
fn skips_attach_sync_when_mark_unchanged_parity() {
    assert!(!should_sync_attach_state(false));
}

#[test]
fn syncs_pinned_attach_with_presentation_parity() {
    assert!(should_sync_pinned_attach(true));
}

#[test]
fn skips_pinned_attach_sync_without_presentation_parity() {
    assert!(!should_sync_pinned_attach(false));
}

// ---- busy mark choice (2) ----

#[test]
fn marks_session_busy_when_busy_parity() {
    assert!(should_mark_session_busy(true));
}

#[test]
fn marks_session_idle_when_not_busy_parity() {
    assert!(!should_mark_session_busy(false));
}
