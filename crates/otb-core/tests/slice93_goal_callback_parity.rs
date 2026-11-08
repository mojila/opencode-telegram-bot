//! Slice 93 parity: goal callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/commands/goal-command.ts` (`goalConfirmData` /
//! `goalCancelData` / `goalDeleteData`) and
//! `src/bot/callbacks/goal-callback-handler.ts` (the `goal:delete:`
//! prefix route, the confirm/cancel exact-match route, the goal-flow
//! cancel-valid gate, `readConfirmation` kind/flow/stage/type gates with
//! nullable `goalId`, the confirmation/callback `messageId` mismatch
//! gate, the callback message-id resolve gate, the busy/session
//! invalidation gate, the stale-confirmation gate, `matchesShownGoal`
//! for `start` / `resume`, the resume claim gate, and the
//! pause/resume/clear status gates).
//! `GoalManager` / `GoalState`, `formatGoalConfirmation` /
//! `formatGoalStatusText`, and `goalService` read/command/refresh kinds
//! already live in `goal_manager_gates` / `goal_reason_gates` /
//! `goal_service_gates` and are NOT re-tested here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `goal_callback_gates` is ported (only the constant-value tests pass).

use otb_core::goal_callback_gates::*;

fn input(
    kind: &str,
    flow: Option<&str>,
    stage: Option<&str>,
    message_id: Option<i64>,
    action: Option<&str>,
    goal_id: Option<&str>,
) -> GoalConfirmationInput {
    GoalConfirmationInput {
        kind: kind.to_string(),
        flow: flow.map(str::to_string),
        stage: stage.map(str::to_string),
        message_id,
        session_id: Some("sess-1".to_string()),
        directory: Some("/repo".to_string()),
        goal_id: goal_id.map(str::to_string),
        shown_goal_json: goal_id.map(|g| format!("{{\"id\":\"{g}\"}}")),
        action: action.map(str::to_string),
        objective: Some("ship it".to_string()),
        control_token: Some("tok-1".to_string()),
    }
}

// ---- constants ----

#[test]
fn goal_confirm_value_parity() {
    assert_eq!(GOAL_CONFIRM_CALLBACK, "goal:confirm");
}

#[test]
fn goal_cancel_value_parity() {
    assert_eq!(GOAL_CANCEL_CALLBACK, "goal:cancel");
}

#[test]
fn goal_delete_prefix_value_parity() {
    assert_eq!(GOAL_DELETE_PREFIX, "goal:delete:");
}

// ---- builders ----

#[test]
fn goal_confirm_data_parity() {
    assert_eq!(goal_confirm_data(), "goal:confirm");
}

#[test]
fn goal_cancel_data_parity() {
    assert_eq!(goal_cancel_data(), "goal:cancel");
}

#[test]
fn goal_delete_data_parity() {
    assert_eq!(goal_delete_data("tok-1"), "goal:delete:tok-1");
    assert_eq!(goal_delete_data(""), "goal:delete:");
}

// ---- dispatch ----

#[test]
fn handles_goal_confirm_parity() {
    assert!(handles_goal_callback(Some("goal:confirm")));
}

#[test]
fn handles_goal_cancel_parity() {
    assert!(handles_goal_callback(Some("goal:cancel")));
}

#[test]
fn handles_goal_delete_with_token_parity() {
    assert!(handles_goal_callback(Some("goal:delete:tok-1")));
}

#[test]
fn handles_goal_bare_delete_prefix_parity() {
    assert!(handles_goal_callback(Some("goal:delete:")));
}

#[test]
fn handles_goal_rejects_none_and_empty_parity() {
    assert!(!handles_goal_callback(None));
    assert!(!handles_goal_callback(Some("")));
}

#[test]
fn handles_goal_rejects_bare_goal_parity() {
    assert!(!handles_goal_callback(Some("goal")));
    assert!(!handles_goal_callback(Some("goal:")));
}

#[test]
fn handles_goal_rejects_other_prefixes_parity() {
    assert!(!handles_goal_callback(Some("session:abc")));
    assert!(!handles_goal_callback(Some("project:abc")));
    assert!(!handles_goal_callback(Some("attach:cancel")));
    assert!(!handles_goal_callback(Some("compact:confirm")));
}

// ---- delete / exact matches ----

#[test]
fn is_goal_delete_prefix_parity() {
    assert!(is_goal_delete_callback("goal:delete:tok-1"));
    assert!(is_goal_delete_callback("goal:delete:"));
    assert!(!is_goal_delete_callback("goal:confirm"));
    assert!(!is_goal_delete_callback("goal:delete"));
}

#[test]
fn parse_goal_delete_token_parity() {
    assert_eq!(
        parse_goal_delete_token("goal:delete:tok-1"),
        Some("tok-1".to_string())
    );
    // TS slices without an emptiness check.
    assert_eq!(parse_goal_delete_token("goal:delete:"), Some("".to_string()));
}

#[test]
fn parse_goal_delete_token_rejects_unprefixed_parity() {
    assert_eq!(parse_goal_delete_token("goal:confirm"), None);
    assert_eq!(parse_goal_delete_token("goal:delete"), None);
    assert_eq!(parse_goal_delete_token(""), None);
}

#[test]
fn is_goal_confirm_exact_parity() {
    assert!(is_goal_confirm("goal:confirm"));
    assert!(!is_goal_confirm("goal:confirm:x"));
    assert!(!is_goal_confirm("goal:cancel"));
}

#[test]
fn is_goal_cancel_exact_parity() {
    assert!(is_goal_cancel("goal:cancel"));
    assert!(!is_goal_cancel("goal:cancel:x"));
    assert!(!is_goal_cancel("goal:confirm"));
}

// ---- cancel-valid gate ----

#[test]
fn goal_cancel_valid_when_flow_and_id_match_parity() {
    assert!(is_goal_cancel_valid(Some("goal"), Some(7), Some(7)));
}

#[test]
fn goal_cancel_invalid_when_flow_not_goal_parity() {
    assert!(!is_goal_cancel_valid(Some("commands"), Some(7), Some(7)));
    assert!(!is_goal_cancel_valid(None, Some(7), Some(7)));
}

#[test]
fn goal_cancel_invalid_on_id_mismatch_parity() {
    assert!(!is_goal_cancel_valid(Some("goal"), Some(7), Some(8)));
}

#[test]
fn goal_cancel_invalid_when_ids_missing_parity() {
    assert!(!is_goal_cancel_valid(Some("goal"), None, Some(7)));
    assert!(!is_goal_cancel_valid(Some("goal"), Some(7), None));
    assert!(!is_goal_cancel_valid(Some("goal"), None, None));
}

// ---- readConfirmation ----

#[test]
fn parse_goal_start_confirmation_parity() {
    let got = parse_goal_confirmation(Some(&input(
        "custom",
        Some("goal"),
        Some("confirm"),
        Some(7),
        Some("start"),
        None,
    )))
    .expect("start confirmation parses");
    assert_eq!(got.message_id, 7);
    assert_eq!(got.session_id, "sess-1");
    assert_eq!(got.directory, "/repo");
    assert_eq!(got.goal_id, None);
    assert_eq!(got.action, GoalAction::Start);
    assert_eq!(got.objective, "ship it");
    assert_eq!(got.control_token, "tok-1");
}

#[test]
fn parse_goal_resume_confirmation_parity() {
    let got = parse_goal_confirmation(Some(&input(
        "custom",
        Some("goal"),
        Some("confirm"),
        Some(9),
        Some("resume"),
        Some("g-1"),
    )))
    .expect("resume confirmation parses");
    assert_eq!(got.action, GoalAction::Resume);
    assert_eq!(got.goal_id, Some("g-1".to_string()));
}

#[test]
fn parse_goal_rejects_none_snapshot_parity() {
    assert_eq!(parse_goal_confirmation(None), None);
}

#[test]
fn parse_goal_rejects_wrong_kind_parity() {
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "inline",
            Some("goal"),
            Some("confirm"),
            Some(7),
            Some("start"),
            None
        ))),
        None
    );
}

#[test]
fn parse_goal_rejects_wrong_flow_parity() {
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "custom",
            Some("commands"),
            Some("confirm"),
            Some(7),
            Some("start"),
            None
        ))),
        None
    );
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "custom", None,
            Some("confirm"),
            Some(7),
            Some("start"),
            None
        ))),
        None
    );
}

#[test]
fn parse_goal_rejects_wrong_stage_parity() {
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "custom",
            Some("goal"),
            Some("objective"),
            Some(7),
            Some("start"),
            None
        ))),
        None
    );
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "custom",
            Some("goal"),
            None,
            Some(7),
            Some("start"),
            None
        ))),
        None
    );
}

#[test]
fn parse_goal_rejects_missing_message_id_parity() {
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "custom",
            Some("goal"),
            Some("confirm"),
            None,
            Some("start"),
            None
        ))),
        None
    );
}

#[test]
fn parse_goal_rejects_bad_action_parity() {
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "custom",
            Some("goal"),
            Some("confirm"),
            Some(7),
            Some("details"),
            None
        ))),
        None
    );
    assert_eq!(
        parse_goal_confirmation(Some(&input(
            "custom",
            Some("goal"),
            Some("confirm"),
            Some(7),
            None,
            None
        ))),
        None
    );
}

// ---- message mismatch / resolve ----

#[test]
fn confirmation_mismatch_when_ids_differ_parity() {
    assert!(is_confirmation_message_mismatch(Some(7), Some(8)));
}

#[test]
fn confirmation_no_mismatch_when_equal_parity() {
    assert!(!is_confirmation_message_mismatch(Some(7), Some(7)));
}

#[test]
fn confirmation_mismatch_when_either_missing_parity() {
    assert!(is_confirmation_message_mismatch(None, Some(7)));
    assert!(is_confirmation_message_mismatch(Some(7), None));
    assert!(is_confirmation_message_mismatch(None, None));
}

#[test]
fn resolve_callback_message_id_parity() {
    assert_eq!(resolve_callback_message_id(true, Some(7)), Some(7));
    assert_eq!(resolve_callback_message_id(false, Some(7)), None);
    assert_eq!(resolve_callback_message_id(true, None), None);
}

// ---- busy / session invalidation ----

#[test]
fn invalidates_when_busy_parity() {
    assert!(should_invalidate_goal_on_session(true, Some("sess-1"), "sess-1"));
}

#[test]
fn invalidates_on_session_mismatch_parity() {
    assert!(should_invalidate_goal_on_session(
        false,
        Some("sess-2"),
        "sess-1"
    ));
    assert!(should_invalidate_goal_on_session(false, None, "sess-1"));
}

#[test]
fn keeps_when_idle_and_same_session_parity() {
    assert!(!should_invalidate_goal_on_session(
        false,
        Some("sess-1"),
        "sess-1"
    ));
}

// ---- stale confirmation ----

#[test]
fn stale_when_not_still_active_parity() {
    assert!(is_goal_confirmation_stale(
        false, true, true, true, true, true, false
    ));
}

#[test]
fn stale_on_message_mismatch_parity() {
    assert!(is_goal_confirmation_stale(
        true, false, true, true, true, true, false
    ));
}

#[test]
fn stale_on_session_or_directory_mismatch_parity() {
    assert!(is_goal_confirmation_stale(
        true, true, false, true, true, true, false
    ));
    assert!(is_goal_confirmation_stale(
        true, true, true, false, true, true, false
    ));
}

#[test]
fn stale_on_token_or_goal_mismatch_parity() {
    assert!(is_goal_confirmation_stale(
        true, true, true, true, false, true, false
    ));
    assert!(is_goal_confirmation_stale(
        true, true, true, true, true, false, false
    ));
}

#[test]
fn stale_when_busy_parity() {
    assert!(is_goal_confirmation_stale(
        true, true, true, true, true, true, true
    ));
}

#[test]
fn fresh_when_all_match_and_idle_parity() {
    assert!(!is_goal_confirmation_stale(
        true, true, true, true, true, true, false
    ));
}

// ---- matchesShownGoal ----

#[test]
fn matches_start_when_none_parity() {
    assert!(matches_shown_goal(
        GoalAction::Start,
        GoalReadKind::None,
        false,
        false,
        false
    ));
}

#[test]
fn start_rejects_goal_or_error_parity() {
    assert!(!matches_shown_goal(
        GoalAction::Start,
        GoalReadKind::Goal,
        true,
        true,
        true
    ));
    assert!(!matches_shown_goal(
        GoalAction::Start,
        GoalReadKind::Unsupported,
        false,
        false,
        false
    ));
    assert!(!matches_shown_goal(
        GoalAction::Start,
        GoalReadKind::Error,
        false,
        false,
        false
    ));
}

#[test]
fn matches_resume_when_paused_and_same_parity() {
    assert!(matches_shown_goal(
        GoalAction::Resume,
        GoalReadKind::Goal,
        true,
        true,
        true
    ));
}

#[test]
fn resume_rejects_active_status_parity() {
    assert!(!matches_shown_goal(
        GoalAction::Resume,
        GoalReadKind::Goal,
        true,
        true,
        false
    ));
}

#[test]
fn resume_rejects_id_or_json_mismatch_parity() {
    assert!(!matches_shown_goal(
        GoalAction::Resume,
        GoalReadKind::Goal,
        false,
        true,
        true
    ));
    assert!(!matches_shown_goal(
        GoalAction::Resume,
        GoalReadKind::Goal,
        true,
        false,
        true
    ));
    assert!(!matches_shown_goal(
        GoalAction::Resume,
        GoalReadKind::None,
        true,
        true,
        true
    ));
}

// ---- claim + action status ----

#[test]
fn rejects_resume_when_claim_fails_parity() {
    assert!(should_reject_goal_resume_claim(GoalAction::Resume, false));
    assert!(!should_reject_goal_resume_claim(GoalAction::Resume, true));
    assert!(!should_reject_goal_resume_claim(GoalAction::Start, false));
    assert!(!should_reject_goal_resume_claim(GoalAction::Start, true));
}

#[test]
fn goal_action_status_parity() {
    assert!(is_goal_action_allowed("pause", "active"));
    assert!(!is_goal_action_allowed("pause", "paused"));
    assert!(is_goal_action_allowed("resume", "paused"));
    assert!(!is_goal_action_allowed("resume", "active"));
    assert!(is_goal_action_allowed("clear", "paused"));
    assert!(!is_goal_action_allowed("clear", "active"));
}
