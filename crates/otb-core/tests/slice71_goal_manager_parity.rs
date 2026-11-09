//! Slice 71 parity: goal-manager + rename-flow gates (RED).
//!
//! Mirrors the pure TS contract in:
//! - `src/app/managers/goal-manager.ts` (`GoalManager` follow/clear/apply
//!   once-only notices, pause suppression, control-token claim, `noticeKey`)
//! - `src/app/managers/rename-manager.ts` (rename wait payload, message id,
//!   active-message match, session-info reads, clear)
//! - `src/app/formatters/goal-text.ts` (`formatGoalStatus` label select)
//!
//! Every test MUST FAIL until `goal_manager_gates` is ported (step 2).

use otb_core::goal_manager_gates::*;

fn goal(id: &str, status: GoalStatus, updated_at: i64) -> GoalState {
    GoalState {
        id: id.to_string(),
        objective: "Ship it".to_string(),
        status,
        reason: None,
        rounds: 1,
        max_rounds: 10,
        updated_at,
    }
}

fn paused_goal(reason: Option<&str>) -> GoalState {
    GoalState {
        reason: reason.map(|r| r.to_string()),
        ..goal("g1", GoalStatus::Paused, 100)
    }
}

// --- GoalManager lifecycle ---

#[test]
fn slice71_new_snapshot_empty_parity() {
    let mgr = GoalManager::new();
    let (session, goal) = mgr.snapshot();
    assert_eq!(session, None);
    assert_eq!(goal, None);
}

#[test]
fn slice71_claim_empty_false_parity() {
    let mut mgr = GoalManager::new();
    assert!(!mgr.claim_control("whatever"));
}

#[test]
fn slice71_claim_wrong_token_false_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    assert!(!mgr.claim_control("not-the-token"));
}

#[test]
fn slice71_follow_sets_session_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert_eq!(mgr.snapshot().0, Some("s1"));
}

#[test]
fn slice71_follow_same_session_keeps_goal_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    mgr.follow("s1");
    assert_eq!(mgr.snapshot().1, Some(&goal("g1", GoalStatus::Active, 1)));
}

#[test]
fn slice71_follow_same_session_keeps_token_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    let token = mgr.control_token().to_string();
    mgr.follow("s1");
    assert_eq!(mgr.control_token(), token);
}

#[test]
fn slice71_follow_different_clears_goal_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    mgr.follow("s2");
    let (session, goal) = mgr.snapshot();
    assert_eq!(session, Some("s2"));
    assert_eq!(goal, None);
}

#[test]
fn slice71_follow_different_rotates_token_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    let token = mgr.control_token().to_string();
    mgr.follow("s2");
    assert_ne!(mgr.control_token(), token);
    assert!(!mgr.claim_control(&token));
}

#[test]
fn slice71_clear_resets_snapshot_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    mgr.clear();
    let (session, goal) = mgr.snapshot();
    assert_eq!(session, None);
    assert_eq!(goal, None);
}

#[test]
fn slice71_clear_rotates_token_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    let token = mgr.control_token().to_string();
    mgr.clear();
    assert_ne!(mgr.control_token(), token);
}

#[test]
fn slice71_claim_after_clear_false_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    let token = mgr.control_token().to_string();
    mgr.clear();
    assert!(!mgr.claim_control(&token));
}

// --- apply: session routing and terminal gating ---

#[test]
fn slice71_apply_wrong_session_null_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    assert_eq!(mgr.apply("s2", Some(paused_goal(None)), true), None);
    assert_eq!(mgr.snapshot().1, Some(&goal("g1", GoalStatus::Active, 1)));
}

#[test]
fn slice71_apply_wrong_session_empty_null_parity() {
    let mut mgr = GoalManager::new();
    assert_eq!(mgr.apply("s9", Some(paused_goal(None)), true), None);
}

#[test]
fn slice71_apply_active_null_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert_eq!(
        mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), true),
        None
    );
}

#[test]
fn slice71_apply_null_goal_null_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert_eq!(mgr.apply("s1", None, true), None);
}

#[test]
fn slice71_apply_paused_with_reason_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert_eq!(
        mgr.apply("s1", Some(paused_goal(Some("Paused by the user"))), true),
        Some(GoalNotice {
            status: GoalNoticeStatus::Paused,
            reason: Some("Paused by the user".to_string()),
        })
    );
}

#[test]
fn slice71_apply_paused_without_reason_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert_eq!(
        mgr.apply("s1", Some(paused_goal(None)), true),
        Some(GoalNotice {
            status: GoalNoticeStatus::Paused,
            reason: None,
        })
    );
}

#[test]
fn slice71_apply_complete_with_reason_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    let done = GoalState {
        reason: Some("done".to_string()),
        ..goal("g1", GoalStatus::Complete, 7)
    };
    assert_eq!(
        mgr.apply("s1", Some(done), true),
        Some(GoalNotice {
            status: GoalNoticeStatus::Complete,
            reason: Some("done".to_string()),
        })
    );
}

#[test]
fn slice71_apply_same_twice_second_null_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert!(mgr.apply("s1", Some(paused_goal(None)), true).is_some());
    assert_eq!(mgr.apply("s1", Some(paused_goal(None)), true), None);
}

#[test]
fn slice71_apply_active_then_paused_reannounces_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert!(mgr.apply("s1", Some(paused_goal(None)), true).is_some());
    assert_eq!(
        mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 2)), true),
        None
    );
    assert!(mgr.apply("s1", Some(paused_goal(None)), true).is_some());
}

#[test]
fn slice71_apply_announce_false_records_key_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert_eq!(mgr.apply("s1", Some(paused_goal(None)), false), None);
    assert_eq!(mgr.apply("s1", Some(paused_goal(None)), true), None);
}

#[test]
fn slice71_mark_announced_silences_apply_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.mark_announced(&paused_goal(None));
    assert_eq!(mgr.apply("s1", Some(paused_goal(None)), true), None);
}

#[test]
fn slice71_updated_at_change_reannounces_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert!(mgr.apply("s1", Some(paused_goal(None)), true).is_some());
    let later = GoalState {
        updated_at: 101,
        ..paused_goal(None)
    };
    assert!(mgr.apply("s1", Some(later), true).is_some());
}

#[test]
fn slice71_reason_only_change_silent_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    assert!(mgr
        .apply("s1", Some(paused_goal(Some("a"))), true)
        .is_some());
    assert_eq!(mgr.apply("s1", Some(paused_goal(Some("b"))), true), None);
}

// --- pause suppression ---

#[test]
fn slice71_suppress_swallows_paused_once_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.suppress_pause_notice();
    assert_eq!(mgr.apply("s1", Some(paused_goal(None)), true), None);
}

#[test]
fn slice71_suppress_leaves_complete_alone_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.suppress_pause_notice();
    let done = goal("g1", GoalStatus::Complete, 7);
    assert!(mgr.apply("s1", Some(done), true).is_some());
}

#[test]
fn slice71_clear_suppression_restores_notice_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.suppress_pause_notice();
    mgr.clear_pause_suppression();
    assert!(mgr.apply("s1", Some(paused_goal(None)), true).is_some());
}

#[test]
fn slice71_clear_resets_suppression_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.suppress_pause_notice();
    mgr.clear();
    mgr.follow("s1");
    assert!(mgr.apply("s1", Some(paused_goal(None)), true).is_some());
}

// --- control token rotation ---

#[test]
fn slice71_token_rotates_on_id_change_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    let token = mgr.control_token().to_string();
    mgr.apply("s1", Some(goal("g2", GoalStatus::Active, 1)), false);
    assert_ne!(mgr.control_token(), token);
}

#[test]
fn slice71_token_rotates_on_status_change_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    let token = mgr.control_token().to_string();
    mgr.apply("s1", Some(goal("g1", GoalStatus::Paused, 2)), false);
    assert_ne!(mgr.control_token(), token);
}

#[test]
fn slice71_token_rotates_on_objective_change_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    let token = mgr.control_token().to_string();
    let changed = GoalState {
        objective: "Something else".to_string(),
        ..goal("g1", GoalStatus::Active, 1)
    };
    mgr.apply("s1", Some(changed), false);
    assert_ne!(mgr.control_token(), token);
}

#[test]
fn slice71_token_stable_on_identical_reapply_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    let token = mgr.control_token().to_string();
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    assert_eq!(mgr.control_token(), token);
}

#[test]
fn slice71_reason_only_change_keeps_token_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(paused_goal(Some("a"))), false);
    let token = mgr.control_token().to_string();
    mgr.apply("s1", Some(paused_goal(Some("b"))), false);
    assert_eq!(mgr.control_token(), token);
}

#[test]
fn slice71_claim_valid_rotates_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    mgr.apply("s1", Some(goal("g1", GoalStatus::Active, 1)), false);
    let token = mgr.control_token().to_string();
    assert!(mgr.claim_control(&token));
    assert!(!mgr.claim_control(&token));
}

#[test]
fn slice71_claim_without_goal_false_parity() {
    let mut mgr = GoalManager::new();
    mgr.follow("s1");
    let token = mgr.control_token().to_string();
    assert!(!mgr.claim_control(&token));
}

// --- notice_key ---

#[test]
fn slice71_notice_key_format_parity() {
    assert_eq!(notice_key(&paused_goal(None)), "g1:paused:100");
}

#[test]
fn slice71_notice_key_differs_by_status_parity() {
    let complete = goal("g1", GoalStatus::Complete, 100);
    assert_ne!(notice_key(&paused_goal(None)), notice_key(&complete));
}

#[test]
fn slice71_notice_key_differs_by_updated_at_parity() {
    let later = GoalState {
        updated_at: 200,
        ..paused_goal(None)
    };
    assert_ne!(notice_key(&paused_goal(None)), notice_key(&later));
}

// --- status label select ---

#[test]
fn slice71_status_label_paused_parity() {
    assert_eq!(
        select_goal_status_label(GoalStatus::Paused, "paused", "complete", "active"),
        "paused"
    );
}

#[test]
fn slice71_status_label_complete_parity() {
    assert_eq!(
        select_goal_status_label(GoalStatus::Complete, "paused", "complete", "active"),
        "complete"
    );
}

#[test]
fn slice71_status_label_active_parity() {
    assert_eq!(
        select_goal_status_label(GoalStatus::Active, "paused", "complete", "active"),
        "active"
    );
}

// --- RenameFlow ---

#[test]
fn slice71_rename_new_idle_parity() {
    let flow = RenameFlow::new();
    assert!(!flow.is_waiting_for_name());
    assert_eq!(flow.session_info(), None);
    assert_eq!(flow.message_id(), None);
}

#[test]
fn slice71_rename_start_waiting_sets_info_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/work/proj", "Old title");
    assert!(flow.is_waiting_for_name());
    assert_eq!(flow.session_info(), Some(("s1", "/work/proj", "Old title")));
    assert_eq!(flow.message_id(), None);
}

#[test]
fn slice71_rename_start_waiting_overwrites_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/a", "One");
    flow.start_waiting("s2", "/b", "Two");
    assert_eq!(flow.session_info(), Some(("s2", "/b", "Two")));
}

#[test]
fn slice71_rename_set_message_without_state_noop_parity() {
    let mut flow = RenameFlow::new();
    flow.set_message_id(42);
    assert_eq!(flow.message_id(), None);
    assert!(!flow.is_waiting_for_name());
}

#[test]
fn slice71_rename_set_and_get_message_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/work/proj", "Old title");
    flow.set_message_id(42);
    assert_eq!(flow.message_id(), Some(42));
}

#[test]
fn slice71_rename_is_active_match_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/work/proj", "Old title");
    flow.set_message_id(42);
    assert!(flow.is_active_message(Some(42)));
}

#[test]
fn slice71_rename_is_active_mismatch_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/work/proj", "Old title");
    flow.set_message_id(42);
    assert!(!flow.is_active_message(Some(43)));
}

#[test]
fn slice71_rename_is_active_unset_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/work/proj", "Old title");
    assert!(!flow.is_active_message(Some(42)));
}

#[test]
fn slice71_rename_is_active_none_arg_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/work/proj", "Old title");
    flow.set_message_id(42);
    assert!(!flow.is_active_message(None));
}

#[test]
fn slice71_rename_clear_empties_parity() {
    let mut flow = RenameFlow::new();
    flow.start_waiting("s1", "/work/proj", "Old title");
    flow.set_message_id(42);
    flow.clear();
    assert!(!flow.is_waiting_for_name());
    assert_eq!(flow.session_info(), None);
    assert_eq!(flow.message_id(), None);
}
