//! Slice 64 parity: interaction-guard edge gates (RED).
//!
//! Mirrors the TS contracts in
//! `src/bot/middleware/interaction-guard-decision.ts`,
//! `src/app/services/local-command-registry.ts` (`allowsWhenBusy` / `has`),
//! `src/bot/keyboards/goal-button.ts` (`GOAL_BUTTON_TEXT_PATTERN`),
//! `src/bot/menus/scheduled-task-menu.ts` (`task:cancel`,
//! `task:retry-schedule`), and
//! `src/bot/messages/busy-blocked-renderer.ts`.
//!
//! Every test MUST FAIL until `interaction_guard_edge_gates` is ported
//! (step 2).

use otb_core::interaction_guard_edge_gates::*;

// --- BUSY_ALLOWED_OUTSIDE_INTERACTION_COMMANDS ---

#[test]
fn outside_commands_list_contents_parity() {
    assert_eq!(
        BUSY_ALLOWED_OUTSIDE_INTERACTION_COMMANDS,
        &["/reload", "/goal"]
    );
}

// --- is_busy_allowed_with_registry ---

#[test]
fn busy_allowed_abort_parity() {
    assert!(is_busy_allowed_with_registry(Some("/abort"), false));
}

#[test]
fn busy_allowed_detach_parity() {
    assert!(is_busy_allowed_with_registry(Some("/detach"), false));
}

#[test]
fn busy_allowed_status_parity() {
    assert!(is_busy_allowed_with_registry(Some("/status"), false));
}

#[test]
fn busy_allowed_help_parity() {
    assert!(is_busy_allowed_with_registry(Some("/help"), false));
}

#[test]
fn busy_allowed_opencode_stop_parity() {
    assert!(is_busy_allowed_with_registry(Some("/opencode_stop"), false));
}

#[test]
fn busy_blocked_new_parity() {
    assert!(!is_busy_allowed_with_registry(Some("/new"), false));
}

#[test]
fn busy_allowed_via_registry_fallback_parity() {
    assert!(is_busy_allowed_with_registry(Some("/deploy"), true));
}

#[test]
fn busy_blocked_registry_denies_parity() {
    assert!(!is_busy_allowed_with_registry(Some("/deploy"), false));
}

// --- is_busy_allowed_outside_interaction ---

#[test]
fn outside_interaction_reload_known_parity() {
    assert!(is_busy_allowed_outside_interaction(Some("/reload"), true));
}

#[test]
fn outside_interaction_goal_known_parity() {
    assert!(is_busy_allowed_outside_interaction(Some("/goal"), true));
}

#[test]
fn outside_interaction_reload_unknown_parity() {
    assert!(!is_busy_allowed_outside_interaction(Some("/reload"), false));
}

#[test]
fn outside_interaction_abort_not_listed_parity() {
    assert!(!is_busy_allowed_outside_interaction(Some("/abort"), true));
}

#[test]
fn outside_interaction_missing_command_parity() {
    assert!(!is_busy_allowed_outside_interaction(None, true));
}

// --- should_busy_block_local_command ---

#[test]
fn local_command_blocks_when_state_and_known_parity() {
    assert!(should_busy_block_local_command(true, true));
}

#[test]
fn local_command_passes_without_state_parity() {
    assert!(!should_busy_block_local_command(false, true));
}

#[test]
fn local_command_passes_when_unknown_parity() {
    assert!(!should_busy_block_local_command(true, false));
}

// --- is_goal_button_label ---

#[test]
fn goal_label_details_parity() {
    assert!(is_goal_button_label("◎ active · ship it"));
}

#[test]
fn goal_label_pause_parity() {
    assert!(is_goal_button_label("⏸ Pause"));
}

#[test]
fn goal_label_resume_parity() {
    assert!(is_goal_button_label("▶ Continue"));
}

#[test]
fn goal_label_clear_parity() {
    assert!(is_goal_button_label("× Delete"));
}

#[test]
fn goal_label_plain_text_parity() {
    assert!(!is_goal_button_label("hello"));
}

#[test]
fn goal_label_marker_only_parity() {
    assert!(!is_goal_button_label("◎"));
}

// --- goal_label_has_busy_bypass_prefix ---

#[test]
fn bypass_circle_prefix_parity() {
    assert!(goal_label_has_busy_bypass_prefix("◎ active · ship it"));
}

#[test]
fn bypass_pause_prefix_parity() {
    assert!(goal_label_has_busy_bypass_prefix("⏸ Pause"));
}

#[test]
fn no_bypass_resume_prefix_parity() {
    assert!(!goal_label_has_busy_bypass_prefix("▶ Continue"));
}

// --- decide_goal_label_while_busy ---

#[test]
fn goal_label_blocks_over_interaction_parity() {
    assert_eq!(
        decide_goal_label_while_busy(true, false, false),
        GoalLabelBusyDecision::BlockExpectedCallback
    );
}

#[test]
fn goal_label_allows_when_idle_parity() {
    assert_eq!(
        decide_goal_label_while_busy(false, false, false),
        GoalLabelBusyDecision::Allow
    );
}

#[test]
fn goal_label_allows_bypass_while_busy_parity() {
    assert_eq!(
        decide_goal_label_while_busy(false, true, true),
        GoalLabelBusyDecision::Allow
    );
}

#[test]
fn goal_label_busy_blocks_without_bypass_parity() {
    assert_eq!(
        decide_goal_label_while_busy(false, true, false),
        GoalLabelBusyDecision::BlockBusyExpectedText
    );
}

// --- is_goal_flow_shortcut ---

#[test]
fn goal_shortcut_cancel_parity() {
    assert!(is_goal_flow_shortcut(Some("goal"), Some("goal:cancel")));
}

#[test]
fn goal_shortcut_confirm_parity() {
    assert!(is_goal_flow_shortcut(Some("goal"), Some("goal:confirm")));
}

#[test]
fn goal_shortcut_wrong_flow_parity() {
    assert!(!is_goal_flow_shortcut(Some("task"), Some("goal:cancel")));
}

#[test]
fn goal_shortcut_wrong_callback_parity() {
    assert!(!is_goal_flow_shortcut(
        Some("goal"),
        Some("goal:delete:abc")
    ));
}

// --- is_allowed_rename_cancel ---

#[test]
fn rename_cancel_allowed_parity() {
    assert!(is_allowed_rename_cancel(
        Some("rename"),
        Some("text"),
        Some("rename:cancel")
    ));
}

#[test]
fn rename_cancel_wrong_kind_parity() {
    assert!(!is_allowed_rename_cancel(
        Some("task"),
        Some("text"),
        Some("rename:cancel")
    ));
}

#[test]
fn rename_cancel_wrong_callback_parity() {
    assert!(!is_allowed_rename_cancel(
        Some("rename"),
        Some("text"),
        Some("project:abc")
    ));
}

// --- is_allowed_task_callback ---

#[test]
fn task_cancel_allowed_parity() {
    assert!(is_allowed_task_callback(Some("task"), Some("task:cancel")));
}

#[test]
fn task_retry_allowed_parity() {
    assert!(is_allowed_task_callback(
        Some("task"),
        Some("task:retry-schedule")
    ));
}

#[test]
fn task_callback_wrong_kind_parity() {
    assert!(!is_allowed_task_callback(
        Some("rename"),
        Some("task:cancel")
    ));
}

// --- is_queued_prompt_button_press ---

#[test]
fn queued_prompt_press_parity() {
    assert!(is_queued_prompt_button_press("❌ 2. fix the bug"));
}

#[test]
fn queued_prompt_plain_text_parity() {
    assert!(!is_queued_prompt_button_press("hello"));
}

#[test]
fn queued_prompt_missing_number_parity() {
    assert!(!is_queued_prompt_button_press("❌ fix the bug"));
}

// --- is_mixed_allowed_input ---

#[test]
fn mixed_allows_callback_parity() {
    assert!(is_mixed_allowed_input(true, false));
}

#[test]
fn mixed_allows_text_parity() {
    assert!(is_mixed_allowed_input(false, true));
}

#[test]
fn mixed_blocks_neither_parity() {
    assert!(!is_mixed_allowed_input(false, false));
}

// --- decide_busy_blocked_channel ---

#[test]
fn busy_blocked_answers_callback_parity() {
    assert_eq!(
        decide_busy_blocked_channel(true, true),
        BusyBlockedChannel::AnswerCallback
    );
}

#[test]
fn busy_blocked_replies_to_chat_parity() {
    assert_eq!(
        decide_busy_blocked_channel(false, true),
        BusyBlockedChannel::Reply
    );
}

#[test]
fn busy_blocked_stays_silent_parity() {
    assert_eq!(
        decide_busy_blocked_channel(false, false),
        BusyBlockedChannel::Silent
    );
}
