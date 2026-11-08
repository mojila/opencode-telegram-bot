//! Slice 63 parity: router / middleware / callback-dispatch gates (RED).
//!
//! Mirrors the TS contracts in `src/bot/routers/command-utils.ts`,
//! `src/bot/routers/command-router.ts`, `src/bot/middleware/auth.ts`,
//! `src/bot/middleware/stale-update.ts`,
//! `src/bot/middleware/unknown-command.ts`,
//! `src/bot/commands/goal-command.ts`,
//! `src/bot/callbacks/goal-callback-handler.ts`, and
//! `src/bot/callbacks/prompt-attachment-callback-handler.ts`.
//!
//! Every test MUST FAIL until `router_middleware_gates` is ported (step 2).

use otb_core::router_middleware_gates::*;

// --- extract_command_name (command-utils.ts) ---

#[test]
fn extract_simple_command_parity() {
    assert_eq!(extract_command_name("/status"), Some("status".to_string()));
}

#[test]
fn extract_command_with_args_parity() {
    assert_eq!(
        extract_command_name("/new hello world"),
        Some("new".to_string())
    );
}

#[test]
fn extract_command_strips_mention_parity() {
    assert_eq!(
        extract_command_name("/status@mybot"),
        Some("status".to_string())
    );
}

#[test]
fn extract_command_lowercases_parity() {
    assert_eq!(
        extract_command_name("/STATUS@MyBot extra"),
        Some("status".to_string())
    );
}

#[test]
fn extract_plain_text_has_no_command_parity() {
    assert_eq!(extract_command_name("hello"), None);
}

#[test]
fn extract_bare_slash_has_no_command_parity() {
    assert_eq!(extract_command_name("/"), None);
}

// --- is_known_command ---

#[test]
fn known_command_hit_parity() {
    assert!(is_known_command("status", &["start", "status", "help"]));
}

#[test]
fn known_command_miss_parity() {
    assert!(!is_known_command("frobnicate", &["start", "status", "help"]));
}

// --- should_flush_pending_prompt ---

#[test]
fn flush_slash_text_parity() {
    assert!(should_flush_pending_prompt(Some("/status")));
}

#[test]
fn flush_plain_text_parity() {
    assert!(!should_flush_pending_prompt(Some("hello")));
}

#[test]
fn flush_no_text_parity() {
    assert!(!should_flush_pending_prompt(None));
}

// --- decide_ensure_commands_init ---

#[test]
fn init_skips_when_already_initialized_parity() {
    assert_eq!(
        decide_ensure_commands_init(true, Some(7), 7, true),
        EnsureCommandsDecision::SkipAlreadyInitialized
    );
}

#[test]
fn init_skips_for_unauthorized_user_parity() {
    assert_eq!(
        decide_ensure_commands_init(false, Some(9), 7, true),
        EnsureCommandsDecision::SkipUnauthorized
    );
}

#[test]
fn init_skips_without_chat_parity() {
    assert_eq!(
        decide_ensure_commands_init(false, Some(7), 7, false),
        EnsureCommandsDecision::SkipNoChat
    );
}

#[test]
fn init_proceeds_for_authorized_chat_parity() {
    assert_eq!(
        decide_ensure_commands_init(false, Some(7), 7, true),
        EnsureCommandsDecision::Initialize
    );
}

// --- local_command_reply_key / failed_exit_code_label ---

#[test]
fn local_reply_success_sends_raw_text_parity() {
    assert_eq!(local_command_reply_key(LocalCommandKind::Success), None);
}

#[test]
fn local_reply_empty_key_parity() {
    assert_eq!(
        local_command_reply_key(LocalCommandKind::Empty),
        Some("local_command.empty_output")
    );
}

#[test]
fn local_reply_timeout_key_parity() {
    assert_eq!(
        local_command_reply_key(LocalCommandKind::Timeout),
        Some("local_command.timeout")
    );
}

#[test]
fn local_reply_failed_key_parity() {
    assert_eq!(
        local_command_reply_key(LocalCommandKind::Failed),
        Some("local_command.failed")
    );
}

#[test]
fn failed_exit_code_number_parity() {
    assert_eq!(failed_exit_code_label(Some(1)), "1".to_string());
}

#[test]
fn failed_exit_code_missing_parity() {
    assert_eq!(failed_exit_code_label(None), "unknown".to_string());
}

// --- decide_auth ---

#[test]
fn auth_allows_authorized_user_parity() {
    assert_eq!(decide_auth(Some(7), 7, 99, Some(7)), AuthVerdict::Allow);
}

#[test]
fn auth_ignores_bot_itself_parity() {
    assert_eq!(
        decide_auth(Some(99), 7, 99, Some(7)),
        AuthVerdict::IgnoreSelf
    );
}

#[test]
fn auth_denies_stranger_and_hides_commands_parity() {
    assert_eq!(
        decide_auth(Some(9), 7, 99, Some(1234)),
        AuthVerdict::DenyHideCommands
    );
}

#[test]
fn auth_denies_stranger_in_allowed_chat_without_hide_parity() {
    assert_eq!(
        decide_auth(Some(9), 7, 99, Some(7)),
        AuthVerdict::Deny
    );
}

#[test]
fn auth_denies_stranger_without_chat_parity() {
    assert_eq!(decide_auth(Some(9), 7, 99, None), AuthVerdict::Deny);
}

#[test]
fn auth_denies_missing_user_parity() {
    assert_eq!(decide_auth(None, 7, 99, Some(1234)), AuthVerdict::Deny);
}

// --- stale-update age ---

#[test]
fn stale_budget_is_sixty_seconds_parity() {
    assert_eq!(MAX_MESSAGE_AGE_SECONDS, 60);
}

#[test]
fn message_age_subtracts_date_parity() {
    assert_eq!(message_age_seconds(1_700_000_000_000, 1_699_999_950), 50);
}

#[test]
fn fresh_message_at_boundary_parity() {
    assert!(!is_stale_message(60));
}

#[test]
fn stale_message_past_budget_parity() {
    assert!(is_stale_message(61));
}

// --- decide_unknown_command ---

#[test]
fn unknown_command_no_text_passes_parity() {
    assert_eq!(
        decide_unknown_command(None, &["start", "status"]),
        UnknownCommandDecision::PassThrough
    );
}

#[test]
fn unknown_command_plain_text_passes_parity() {
    assert_eq!(
        decide_unknown_command(Some("hello there"), &["start", "status"]),
        UnknownCommandDecision::PassThrough
    );
}

#[test]
fn unknown_command_known_passes_parity() {
    assert_eq!(
        decide_unknown_command(Some("/status"), &["start", "status"]),
        UnknownCommandDecision::PassThrough
    );
}

#[test]
fn unknown_command_unknown_replies_with_token_parity() {
    assert_eq!(
        decide_unknown_command(Some("/frobnicate"), &["start", "status"]),
        UnknownCommandDecision::ReplyUnknown {
            token: "/frobnicate".to_string()
        }
    );
}

#[test]
fn unknown_command_uses_first_token_parity() {
    assert_eq!(
        decide_unknown_command(Some("/frobnicate now please"), &["start", "status"]),
        UnknownCommandDecision::ReplyUnknown {
            token: "/frobnicate".to_string()
        }
    );
}

// --- goal callback data ---

#[test]
fn goal_confirm_data_value_parity() {
    assert_eq!(GOAL_CONFIRM_DATA, "goal:confirm");
}

#[test]
fn goal_cancel_data_value_parity() {
    assert_eq!(GOAL_CANCEL_DATA, "goal:cancel");
}

#[test]
fn goal_delete_data_builds_token_parity() {
    assert_eq!(build_goal_delete_data("abc"), "goal:delete:abc".to_string());
}

#[test]
fn goal_route_delete_parity() {
    assert_eq!(
        route_goal_callback(Some("goal:delete:abc")),
        GoalCallbackRoute::Delete {
            token: "abc".to_string()
        }
    );
}

#[test]
fn goal_route_confirm_parity() {
    assert_eq!(
        route_goal_callback(Some("goal:confirm")),
        GoalCallbackRoute::Confirm
    );
}

#[test]
fn goal_route_cancel_parity() {
    assert_eq!(
        route_goal_callback(Some("goal:cancel")),
        GoalCallbackRoute::Cancel
    );
}

#[test]
fn goal_route_other_parity() {
    assert_eq!(
        route_goal_callback(Some("goal:bogus")),
        GoalCallbackRoute::Other
    );
}

#[test]
fn goal_route_missing_parity() {
    assert_eq!(route_goal_callback(None), GoalCallbackRoute::Other);
}

#[test]
fn goal_cancel_valid_flow_parity() {
    assert!(is_valid_goal_cancel(Some("goal"), Some(11), Some(11)));
}

#[test]
fn goal_cancel_wrong_flow_parity() {
    assert!(!is_valid_goal_cancel(Some("task"), Some(11), Some(11)));
}

#[test]
fn goal_cancel_message_mismatch_parity() {
    assert!(!is_valid_goal_cancel(Some("goal"), Some(11), Some(12)));
}

// --- prompt-attachment cancel ---

#[test]
fn attachment_cancel_value_parity() {
    assert_eq!(ATTACHMENT_CANCEL_CALLBACK, "attach:cancel");
}

#[test]
fn attachment_cancel_exact_match_parity() {
    assert!(is_attachment_cancel_callback(Some("attach:cancel")));
}

#[test]
fn attachment_cancel_other_data_parity() {
    assert!(!is_attachment_cancel_callback(Some("attach:other")));
}
