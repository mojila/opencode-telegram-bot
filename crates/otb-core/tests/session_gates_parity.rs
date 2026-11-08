//! Parity port of pure session/interaction gates (slice 26b, RED).
//!
//! Mirrors:
//! - `tests/app/managers/abort-suppression-manager.test.ts` (5 its)
//! - `tests/app/managers/external-input-suppression-manager.test.ts` (4 its)
//! - `tests/app/managers/assistant-run-state-manager.test.ts` (5 its)
//! - `tests/app/managers/background-session-manager.test.ts` (8 its)
//! - `tests/bot/message-patterns.test.ts` (9 its)
//! - `tests/app/goal.test.ts` objective/text subset (goal objective 2 its
//!   + truncate/dashboard/abort/notice its)
//! - `tests/bot/keyboards/queued-prompt-button.test.ts` label subset
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::session_gates::*;

// --- abort suppression (5 + size gate = 6) ---

#[test]
fn slice26b_01_abort_suppresses_one_aborted_error() {
    reset_user_abort_suppression();
    mark_user_abort_requested("session-1", 1_000);
    assert!(should_suppress_user_abort_session_error("session-1", " Aborted ", 1_000));
    assert!(!should_suppress_user_abort_session_error("session-1", "Aborted", 1_000));
}

#[test]
fn slice26b_02_abort_ignores_unrelated_errors() {
    reset_user_abort_suppression();
    mark_user_abort_requested("session-1", 1_000);
    assert!(!should_suppress_user_abort_session_error("session-1", "Model not found", 1_000));
    assert!(should_suppress_user_abort_session_error("session-1", "Aborted", 1_000));
}

#[test]
fn slice26b_03_abort_does_not_suppress_stale_errors() {
    reset_user_abort_suppression();
    mark_user_abort_requested("session-1", 1_000);
    assert!(!should_suppress_user_abort_session_error(
        "session-1",
        "Aborted",
        1_000 + USER_ABORT_SUPPRESSION_WINDOW_MS + 1
    ));
}

#[test]
fn slice26b_04_abort_stays_active_within_window() {
    reset_user_abort_suppression();
    mark_user_abort_requested("session-1", 1_000);
    assert!(should_suppress_user_abort_session_error(
        "session-1",
        "Aborted",
        1_000 + USER_ABORT_SUPPRESSION_WINDOW_MS - 1
    ));
}

#[test]
fn slice26b_05_abort_prunes_expired_markers_on_new_mark() {
    reset_user_abort_suppression();
    mark_user_abort_requested("session-1", 1_000);
    mark_user_abort_requested("session-2", 1_000 + USER_ABORT_SUPPRESSION_WINDOW_MS + 1);
    assert_eq!(user_abort_suppression_size(), 1);
    assert!(!should_suppress_user_abort_session_error(
        "session-1",
        "Aborted",
        1_000 + USER_ABORT_SUPPRESSION_WINDOW_MS + 1
    ));
    assert!(should_suppress_user_abort_session_error(
        "session-2",
        "Aborted",
        1_000 + USER_ABORT_SUPPRESSION_WINDOW_MS + 1
    ));
}

#[test]
fn slice26b_06_abort_case_insensitive_match() {
    reset_user_abort_suppression();
    mark_user_abort_requested("session-1", 1_000);
    assert!(should_suppress_user_abort_session_error("session-1", "ABORTED", 1_000));
}

// --- external input suppression (6) ---

#[test]
fn slice26b_07_external_consumes_matching_input_once() {
    let mut manager = ExternalUserInputSuppressionManager::new();
    manager.register("session-1", "Review README", 1_000);
    assert!(manager.consume("session-1", "Review README", 1_000));
    assert!(!manager.consume("session-1", "Review README", 1_000));
}

#[test]
fn slice26b_08_external_ignores_other_sessions() {
    let mut manager = ExternalUserInputSuppressionManager::new();
    manager.register("session-1", "Review README", 1_000);
    assert!(!manager.consume("session-2", "Review README", 1_000));
}

#[test]
fn slice26b_09_external_ignores_different_text() {
    let mut manager = ExternalUserInputSuppressionManager::new();
    manager.register("session-1", "Review README", 1_000);
    assert!(!manager.consume("session-1", "Review tests", 1_000));
}

#[test]
fn slice26b_10_external_expires_stale_entries() {
    let mut manager = ExternalUserInputSuppressionManager::new();
    manager.register("session-1", "Review README", 1_000);
    assert!(!manager.consume("session-1", "Review README", 1_000 + EXTERNAL_INPUT_SUPPRESSION_TTL_MS + 1));
}

#[test]
fn slice26b_11_external_normalizes_crlf_and_trims() {
    assert_eq!(normalize_external_user_input_text("Review\r\nREADME  "), "Review\nREADME");
    let mut manager = ExternalUserInputSuppressionManager::new();
    manager.register("session-1", "Review README", 1_000);
    assert!(manager.consume("session-1", "  Review README\r\n", 1_000));
}

#[test]
fn slice26b_12_external_clear_all_drops_entries() {
    let mut manager = ExternalUserInputSuppressionManager::new();
    manager.register("session-1", "Review README", 1_000);
    manager.clear_all();
    assert!(!manager.consume("session-1", "Review README", 1_000));
}

// --- assistant run state (9) ---

fn run_step(message_id: &str, streamed: u64, output: u64) -> AssistantRunStep {
    AssistantRunStep {
        message_id: message_id.to_string(),
        created: 10,
        streamed,
        output,
        reasoning: 0,
    }
}

#[test]
fn slice26b_13_run_adopts_pre_open_steps_and_drops_on_clear() {
    let mut state = AssistantRunState::new();
    state.record_step("ses-1", run_step("msg-1", 20, 3));
    state.start_observed_run("ses-1", 1);
    state.record_step("ses-1", run_step("msg-1", 40, 5));
    let finished = state.finish_run("ses-1", "test").expect("run");
    assert_eq!(finished.steps, vec![run_step("msg-1", 40, 5)]);
    state.record_step("ses-1", run_step("late", 4, 1));
    state.clear_run("ses-1", "test");
    state.start_observed_run("ses-1", 2);
    assert_eq!(
        state.finish_run("ses-1", "test").expect("run").steps,
        Vec::new()
    );
}

#[test]
fn slice26b_14_run_footerless_wins_over_interrupted() {
    let mut state = AssistantRunState::new();
    state.start_run("ses-1", AssistantRunStartInfo { started_at: 1 });
    state.suppress_footer("ses-1");
    state.mark_interrupted("ses-1");
    assert!(state.is_footer_suppressed("ses-1"));
    let finished = state.finish_run("ses-1", "test").expect("run");
    assert!(finished.suppress_footer);
    assert!(finished.interrupted);
}

#[test]
fn slice26b_15_run_sealed_steps_belong_to_next_run() {
    let mut state = AssistantRunState::new();
    state.start_run("ses-1", AssistantRunStartInfo { started_at: 1 });
    state.record_step("ses-1", run_step("old", 2, 1));
    state.seal_run("ses-1");
    state.record_step("ses-1", run_step("next", 4, 2));
    let finished = state.finish_run("ses-1", "idle").expect("run");
    assert_eq!(
        finished.steps.iter().map(|step| step.message_id.clone()).collect::<Vec<_>>(),
        vec!["old".to_string()]
    );
    state.start_run("ses-1", AssistantRunStartInfo { started_at: 5 });
    let finished = state.finish_run("ses-1", "next").expect("run");
    assert_eq!(
        finished.steps.iter().map(|step| step.message_id.clone()).collect::<Vec<_>>(),
        vec!["next".to_string()]
    );
}

#[test]
fn slice26b_16_run_sealed_footer_sends_despite_next_footerless() {
    let mut state = AssistantRunState::new();
    state.start_run("ses-1", AssistantRunStartInfo { started_at: 1 });
    state.mark_response_completed("ses-1");
    state.seal_run("ses-1");
    state.suppress_footer("ses-1");
    let finished = state.finish_run("ses-1", "idle").expect("run");
    assert!(!finished.suppress_footer);
    assert!(!state.is_footer_suppressed("ses-1"));
    state.start_run("ses-1", AssistantRunStartInfo { started_at: 2 });
    assert!(state.is_footer_suppressed("ses-1"));
}

#[test]
fn slice26b_17_run_adopts_footerless_mark_on_observed_open() {
    let mut state = AssistantRunState::new();
    state.suppress_footer("ses-1");
    state.start_observed_run("ses-1", 1);
    assert!(state.is_footer_suppressed("ses-1"));
}

#[test]
fn slice26b_18_run_start_ignores_empty_session() {
    let mut state = AssistantRunState::new();
    state.start_run("", AssistantRunStartInfo { started_at: 1 });
    assert!(state.finish_run("", "test").is_none());
}

#[test]
fn slice26b_19_run_observed_open_keeps_existing_run() {
    let mut state = AssistantRunState::new();
    state.start_run("ses-1", AssistantRunStartInfo { started_at: 1 });
    state.start_observed_run("ses-1", 2);
    let finished = state.finish_run("ses-1", "test").expect("run");
    assert!(!finished.interrupted);
}

#[test]
fn slice26b_20_run_finish_unknown_session_returns_none() {
    let mut state = AssistantRunState::new();
    assert!(state.finish_run("missing", "test").is_none());
}

#[test]
fn slice26b_21_run_interrupt_without_suppress_keeps_footer() {
    let mut state = AssistantRunState::new();
    state.start_run("ses-1", AssistantRunStartInfo { started_at: 1 });
    state.mark_interrupted("ses-1");
    assert!(!state.is_footer_suppressed("ses-1"));
}

// --- background sessions (8) ---

#[test]
fn slice26b_22_background_notifies_on_idle_after_assistant_done() {
    let mut tracker = BackgroundSessionTracker::new();
    tracker.note_session("session-2", "Background Task");
    tracker.note_assistant_message_completed("session-2", "message-1", "session-1");
    let notification = tracker
        .idle_notification("session-2", "session-1")
        .expect("notification");
    assert_eq!(notification.kind, BackgroundNotificationKind::AssistantResponse);
    assert_eq!(notification.session_id, "session-2");
    assert_eq!(notification.session_title.as_deref(), Some("Background Task"));
    assert_eq!(notification.message_id.as_deref(), Some("message-1"));
}

#[test]
fn slice26b_23_background_silent_for_current_session() {
    let mut tracker = BackgroundSessionTracker::new();
    tracker.note_assistant_message_completed("session-1", "message-1", "session-1");
    assert_eq!(tracker.idle_notification("session-1", "session-1"), None);
}

#[test]
fn slice26b_24_background_coalesces_messages_into_one_idle() {
    let mut tracker = BackgroundSessionTracker::new();
    tracker.note_session("session-2", "Background Task");
    tracker.note_assistant_message_completed("session-2", "message-1", "session-1");
    tracker.note_assistant_message_completed("session-2", "message-2", "session-1");
    let first = tracker.idle_notification("session-2", "session-1");
    assert!(first.is_some());
    assert_eq!(tracker.idle_notification("session-2", "session-1"), None);
}

#[test]
fn slice26b_25_background_notifies_questions_and_permissions() {
    let mut tracker = BackgroundSessionTracker::new();
    let question = tracker
        .question_notification("session-2", "req-1", "session-1")
        .expect("question");
    assert_eq!(question.kind, BackgroundNotificationKind::QuestionAsked);
    assert_eq!(question.request_id.as_deref(), Some("req-1"));
    let permission = tracker
        .permission_notification("session-2", "req-2", "session-1")
        .expect("permission");
    assert_eq!(permission.kind, BackgroundNotificationKind::PermissionAsked);
    assert_eq!(permission.request_id.as_deref(), Some("req-2"));
}

#[test]
fn slice26b_26_background_dedupes_request_ids() {
    let mut tracker = BackgroundSessionTracker::new();
    assert!(tracker.question_notification("session-2", "req-1", "session-1").is_some());
    assert_eq!(tracker.question_notification("session-2", "req-1", "session-1"), None);
    assert!(tracker.permission_notification("session-2", "req-2", "session-1").is_some());
    assert_eq!(tracker.permission_notification("session-2", "req-2", "session-1"), None);
}

#[test]
fn slice26b_27_background_silent_without_completed_message() {
    let mut tracker = BackgroundSessionTracker::new();
    tracker.note_session("session-2", "Background Task");
    assert_eq!(tracker.idle_notification("session-2", "session-1"), None);
}

#[test]
fn slice26b_28_background_clear_on_directory_change() {
    let mut tracker = BackgroundSessionTracker::new();
    tracker.note_session("session-2", "Background Task");
    tracker.note_assistant_message_completed("session-2", "message-1", "session-1");
    tracker.set_directory("/tmp/a");
    tracker.set_directory("/tmp/b");
    assert_eq!(tracker.idle_notification("session-2", "session-1"), None);
}

#[test]
fn slice26b_29_background_question_silent_for_current_session() {
    let mut tracker = BackgroundSessionTracker::new();
    assert_eq!(tracker.question_notification("session-1", "req-1", "session-1"), None);
}

// --- message patterns (8) ---

#[test]
fn slice26b_30_pattern_matches_single_line_model_button() {
    assert!(is_model_button_text("🧠 cliproxyapi2/gpt-5.3-codex"));
}

#[test]
fn slice26b_31_pattern_rejects_agent_label_as_model() {
    assert!(!is_model_button_text("🤖 Reviewer Agent"));
}

#[test]
fn slice26b_32_pattern_matches_current_and_legacy_variant_prefix() {
    assert!(is_variant_button_text("💡 Default"));
    assert!(is_variant_button_text("💭 Default"));
}

#[test]
fn slice26b_33_pattern_rejects_plain_prompt() {
    assert!(!is_model_button_text("Create a migration plan"));
    assert!(!is_agent_mode_button_text("Create a migration plan"));
    assert!(!is_variant_button_text("Create a migration plan"));
}

#[test]
fn slice26b_34_pattern_matches_agent_labels_with_descriptors() {
    assert!(is_agent_mode_button_text("🤖 Sisyphus (Ultraworker) Agent"));
    assert!(is_agent_mode_button_text("🤖 Sisyphus (Ultraworker) Mode"));
    assert!(!is_model_button_text("🤖 Sisyphus (Ultraworker) Agent"));
}

#[test]
fn slice26b_35_pattern_matches_queued_prompt_labels() {
    assert!(is_queued_prompt_button_text(&format_queued_prompt_button_label(1, "Fix the bug")));
    assert!(is_queued_prompt_button_text(&format_queued_prompt_button_label(12, "Fix the bug")));
}

#[test]
fn slice26b_36_pattern_rejects_non_queued_labels() {
    assert!(!is_queued_prompt_button_text("🛠️ Build Agent"));
    assert!(!is_queued_prompt_button_text("💡 Default"));
    assert!(!is_queued_prompt_button_text("Create a migration plan"));
    assert!(!is_queued_prompt_button_text("❌ do not do that"));
}

#[test]
fn slice26b_37_pattern_recognises_reply_keyboard_buttons() {
    assert!(is_reply_keyboard_button_text("🛠️ Build Agent"));
    assert!(is_reply_keyboard_button_text("💡 Default"));
    assert!(is_reply_keyboard_button_text(&format_queued_prompt_button_label(1, "queued")));
    assert!(!is_reply_keyboard_button_text("Create a migration plan"));
}

// --- goal objective + labels (11) ---

#[test]
fn slice26b_38_goal_rejects_empty_and_reserved_controls() {
    assert_eq!(parse_goal_objective("  "), GoalObjective::Empty);
    for value in ["PAUSE", "Resume", "clear", " \"PAUSE\" "] {
        assert_eq!(parse_goal_objective(value), GoalObjective::Reserved, "{value}");
    }
    assert_eq!(
        parse_goal_objective("pause now"),
        GoalObjective::Objective("pause now".to_string())
    );
}

#[test]
fn slice26b_39_goal_rejects_objective_past_server_limit() {
    let text = "x".repeat(GOAL_OBJECTIVE_LIMIT + 1);
    assert_eq!(parse_goal_objective(&text), GoalObjective::TooLong(text.len()));
}

#[test]
fn slice26b_40_goal_accepts_objective_at_limit() {
    let text = "x".repeat(GOAL_OBJECTIVE_LIMIT);
    assert_eq!(parse_goal_objective(&text), GoalObjective::Objective(text));
}

#[test]
fn slice26b_41_goal_quoted_reserved_is_reserved() {
    assert_eq!(parse_goal_objective("'pause'"), GoalObjective::Reserved);
    assert_eq!(parse_goal_objective("\"pause'"), GoalObjective::Objective("\"pause'".to_string()));
}

#[test]
fn slice26b_42_goal_truncates_long_objective_with_ellipsis() {
    assert_eq!(truncate_goal_objective("one   line"), "one line");
    let cut = truncate_goal_objective(&"word ".repeat(40));
    assert!(cut.ends_with('…'));
    assert!(cut.len() <= 120);
}

#[test]
fn slice26b_43_goal_flattens_whitespace() {
    assert_eq!(flatten_goal_objective("one \n  line"), "one line");
}

#[test]
fn slice26b_44_goal_button_pattern_matches() {
    assert!(is_goal_button_text("⏸ Pause goal"));
    assert!(!is_goal_button_text("Create a migration plan"));
}

#[test]
fn slice26b_45_queued_label_previews_long_text() {
    let label = format_queued_prompt_button_label(1, &"Fix the bug ".repeat(10));
    assert!(is_queued_prompt_button_text(&label));
    assert!(label.starts_with("❌ 1. "));
}

#[test]
fn slice26b_46_queued_label_keeps_short_text_whole() {
    let label = format_queued_prompt_button_label(2, "Fix the bug");
    assert!(label.contains("Fix the bug"));
}

#[test]
fn slice26b_47_queued_label_index_keeps_duplicates_unique() {
    let first = format_queued_prompt_button_label(1, "same text");
    let second = format_queued_prompt_button_label(2, "same text");
    assert_ne!(first, second);
    assert!(is_queued_prompt_button_text(&first));
    assert!(is_queued_prompt_button_text(&second));
}

#[test]
fn slice26b_48_goal_empty_quoted_is_empty() {
    assert_eq!(parse_goal_objective("\" \""), GoalObjective::Empty);
}
