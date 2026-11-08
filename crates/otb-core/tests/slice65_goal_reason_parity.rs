//! Slice 65 parity: goal reason + confirmation gates (RED).
//!
//! Mirrors the TS contracts in
//! `src/app/formatters/goal-text.ts` that `session_gates`
//! (`parseGoalObjective`, `flattenGoalObjective`, `truncateGoalObjective`)
//! does not cover: `localizeGoalReason` (as pure classification),
//! `isNoActiveGoalReason`, `fitConfirmation`, `formatGoalWarning`,
//! `formatGoalConfirmation`, `formatAbortWithGoalPause`, and
//! `formatGoalStatusText`.
//!
//! Every test MUST FAIL until `goal_reason_gates` is ported (step 2).

use otb_core::goal_reason_gates::*;

// --- classify_goal_reason: exact server sentences ---

#[test]
fn classify_paused_by_user_parity() {
    assert_eq!(
        classify_goal_reason("Paused by the user"),
        GoalReasonKind::PausedByUser
    );
}

#[test]
fn classify_stopped_by_user_parity() {
    assert_eq!(
        classify_goal_reason("Stopped by the user"),
        GoalReasonKind::StoppedByUser
    );
}

#[test]
fn classify_server_restarted_parity() {
    assert_eq!(
        classify_goal_reason("The server restarted while the goal was active"),
        GoalReasonKind::ServerRestarted
    );
}

#[test]
fn classify_stopped_after_error_parity() {
    assert_eq!(
        classify_goal_reason("The goal stopped after an error"),
        GoalReasonKind::StoppedAfterError
    );
}

#[test]
fn classify_no_verdict_parity() {
    assert_eq!(
        classify_goal_reason("The goal reviewer gave no usable verdict"),
        GoalReasonKind::NoVerdict
    );
}

#[test]
fn classify_review_too_large_parity() {
    assert_eq!(
        classify_goal_reason("The goal reviewer's request is too large for the model"),
        GoalReasonKind::ReviewTooLarge
    );
}

#[test]
fn classify_no_active_parity() {
    assert_eq!(
        classify_goal_reason("No active goal to pause."),
        GoalReasonKind::NoActive
    );
}

#[test]
fn classify_no_paused_parity() {
    assert_eq!(
        classify_goal_reason("No paused goal to resume."),
        GoalReasonKind::NoPaused
    );
}

#[test]
fn classify_none_set_parity() {
    assert_eq!(classify_goal_reason("No goal is set."), GoalReasonKind::NoneSet);
}

#[test]
fn classify_exact_match_is_untrimmed_parity() {
    assert_eq!(
        classify_goal_reason("  Paused by the user"),
        GoalReasonKind::Unknown
    );
}

// --- classify_goal_reason: round limit ---

#[test]
fn classify_round_limit_parity() {
    assert_eq!(
        classify_goal_reason("Reached the limit of 5 rounds"),
        GoalReasonKind::RoundLimit { rounds: "5" }
    );
}

#[test]
fn classify_round_limit_rejects_non_digits_parity() {
    assert_eq!(
        classify_goal_reason("Reached the limit of five rounds"),
        GoalReasonKind::Unknown
    );
}

#[test]
fn classify_round_limit_rejects_empty_digits_parity() {
    assert_eq!(
        classify_goal_reason("Reached the limit of  rounds"),
        GoalReasonKind::Unknown
    );
}

// --- classify_goal_reason: wait limit ---

#[test]
fn classify_wait_limit_parity() {
    assert_eq!(
        classify_goal_reason("Nothing ran for 30 minutes while the goal waited on background work"),
        GoalReasonKind::WaitLimit { minutes: "30" }
    );
}

#[test]
fn classify_wait_limit_rejects_altered_suffix_parity() {
    assert_eq!(
        classify_goal_reason("Nothing ran for 30 minutes while the goal waited"),
        GoalReasonKind::Unknown
    );
}

// --- classify_goal_reason: not-progress / session-failed ---

#[test]
fn classify_not_progress_parity() {
    assert_eq!(
        classify_goal_reason("Not making progress: same error twice"),
        GoalReasonKind::NotProgress {
            reason: "same error twice"
        }
    );
}

#[test]
fn classify_not_progress_rejects_empty_tail_parity() {
    assert_eq!(
        classify_goal_reason("Not making progress: "),
        GoalReasonKind::Unknown
    );
}

#[test]
fn classify_session_failed_parity() {
    assert_eq!(
        classify_goal_reason("The session failed: exit 1"),
        GoalReasonKind::SessionFailed { reason: "exit 1" }
    );
}

#[test]
fn classify_session_failed_multiline_tail_parity() {
    assert_eq!(
        classify_goal_reason("The session failed: boom\nbang"),
        GoalReasonKind::SessionFailed { reason: "boom\nbang" }
    );
}

// --- classify_goal_reason: objective limit / unknown ---

#[test]
fn classify_objective_limit_parity() {
    assert_eq!(
        classify_goal_reason("A goal objective is limited to 4000 characters (got 4123)."),
        GoalReasonKind::ObjectiveLimit {
            limit: "4000",
            count: "4123"
        }
    );
}

#[test]
fn classify_objective_limit_rejects_missing_period_parity() {
    assert_eq!(
        classify_goal_reason("A goal objective is limited to 4000 characters (got 4123)"),
        GoalReasonKind::Unknown
    );
}

#[test]
fn classify_unknown_passthrough_parity() {
    assert_eq!(
        classify_goal_reason("Something else entirely"),
        GoalReasonKind::Unknown
    );
}

#[test]
fn classify_unknown_empty_parity() {
    assert_eq!(classify_goal_reason(""), GoalReasonKind::Unknown);
}

// --- is_no_active_goal_reason ---

#[test]
fn is_no_active_server_english_parity() {
    assert!(is_no_active_goal_reason(
        "No active goal to pause.",
        "Нет активной цели."
    ));
}

#[test]
fn is_no_active_localized_parity() {
    assert!(is_no_active_goal_reason(
        "Нет активной цели.",
        "Нет активной цели."
    ));
}

#[test]
fn is_no_active_trims_whitespace_parity() {
    assert!(is_no_active_goal_reason(
        "  No active goal to pause.\n",
        "Нет активной цели."
    ));
}

#[test]
fn is_no_active_rejects_other_parity() {
    assert!(!is_no_active_goal_reason(
        "No paused goal to resume.",
        "Нет активной цели."
    ));
}

// --- fit_confirmation ---

#[test]
fn fit_short_combined_parity() {
    assert_eq!(fit_confirmation("warn", "body"), "warn\n\nbody");
}

#[test]
fn fit_combined_at_limit_parity() {
    let body = "x".repeat(3797);
    let expected = format!("W\n\n{body}");
    assert_eq!(expected.len(), 3800);
    assert_eq!(fit_confirmation("W", &body), expected);
}

#[test]
fn fit_truncates_long_body_parity() {
    let body = "x".repeat(5000);
    let expected = format!("W\n\n{}…", "x".repeat(3796));
    assert_eq!(fit_confirmation("W", &body), expected);
}

#[test]
fn fit_trims_trailing_space_before_ellipsis_parity() {
    let body = format!("{}{}{}", "a".repeat(3795), "   ", "b".repeat(100));
    let expected = format!("W\n\n{}…", "a".repeat(3795));
    assert_eq!(fit_confirmation("W", &body), expected);
}

#[test]
fn fit_returns_warning_when_no_room_parity() {
    let warning = "W".repeat(3797);
    assert_eq!(fit_confirmation(&warning, "abc"), warning);
}

#[test]
fn fit_single_char_body_at_room_two_parity() {
    let warning = "W".repeat(3796);
    let expected = format!("{warning}\n\na…");
    assert_eq!(fit_confirmation(&warning, "abcdef"), expected);
}

#[test]
fn fit_empty_body_parity() {
    assert_eq!(fit_confirmation("W", ""), "W\n\n");
}

// --- format_goal_warning ---

#[test]
fn warning_without_rounds_parity() {
    assert_eq!(format_goal_warning("warn", None), "warn");
}

#[test]
fn warning_with_round_limit_parity() {
    assert_eq!(
        format_goal_warning("warn", Some("Up to 5 rounds.")),
        "warn\nUp to 5 rounds."
    );
}

// --- format_goal_confirmation ---

#[test]
fn confirmation_short_parity() {
    assert_eq!(format_goal_confirmation("W", "B"), "W\n\nB");
}

#[test]
fn confirmation_truncates_parity() {
    let result = format_goal_confirmation("W", &"y".repeat(5000));
    assert!(result.starts_with("W\n\n"));
    assert!(result.ends_with('…'));
    assert_eq!(result, format!("W\n\n{}…", "y".repeat(3796)));
}

#[test]
fn confirmation_empty_body_parity() {
    assert_eq!(format_goal_confirmation("W", ""), "W\n\n");
}

// --- format_abort_with_goal_pause ---

#[test]
fn abort_silent_returns_text_parity() {
    assert_eq!(
        format_abort_with_goal_pause("done", &AbortPauseOutcome::Silent, "P", "F"),
        "done"
    );
}

#[test]
fn abort_paused_prepends_clause_parity() {
    assert_eq!(
        format_abort_with_goal_pause(
            "done",
            &AbortPauseOutcome::Paused { reason: "user" },
            "Goal paused: user",
            "Goal pause failed: user",
        ),
        "Goal paused: user\n\ndone"
    );
}

#[test]
fn abort_failed_prepends_clause_parity() {
    assert_eq!(
        format_abort_with_goal_pause(
            "done",
            &AbortPauseOutcome::Failed { reason: "oops" },
            "Goal paused: oops",
            "Goal pause failed: oops",
        ),
        "Goal pause failed: oops\n\ndone"
    );
}

#[test]
fn abort_silent_ignores_clauses_parity() {
    assert_eq!(
        format_abort_with_goal_pause("", &AbortPauseOutcome::Silent, "P", "F"),
        ""
    );
}

#[test]
fn abort_paused_empty_text_parity() {
    assert_eq!(
        format_abort_with_goal_pause(
            "",
            &AbortPauseOutcome::Paused { reason: "user" },
            "Goal paused: user",
            "F",
        ),
        "Goal paused: user\n\n"
    );
}

// --- format_goal_status_text ---

#[test]
fn status_text_without_reason_parity() {
    assert_eq!(
        format_goal_status_text("Write tests", "active", "1/5", None, "today"),
        "Write tests\n\n◎ active\n1/5\ntoday"
    );
}

#[test]
fn status_text_with_reason_parity() {
    assert_eq!(
        format_goal_status_text("Write tests", "paused", "1/5", Some("tired"), "today"),
        "Write tests\n\n◎ paused\n1/5\ntired\ntoday"
    );
}

#[test]
fn status_text_empty_objective_parity() {
    assert_eq!(
        format_goal_status_text("", "active", "0/5", None, "today"),
        "\n\n◎ active\n0/5\ntoday"
    );
}

#[test]
fn status_text_multiline_objective_kept_parity() {
    assert_eq!(
        format_goal_status_text("line1\nline2", "active", "0/5", None, "today"),
        "line1\nline2\n\n◎ active\n0/5\ntoday"
    );
}

// --- constants ---

#[test]
fn confirmation_limit_value_parity() {
    assert_eq!(CONFIRMATION_LIMIT, 3800);
}

#[test]
fn server_no_active_reason_value_parity() {
    assert_eq!(SERVER_NO_ACTIVE_GOAL_REASON, "No active goal to pause.");
}
