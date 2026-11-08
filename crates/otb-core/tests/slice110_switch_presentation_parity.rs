//! Slice 110 parity: switch-presentation / model-views / safe-background-task gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/utils/safe-background-task.ts` (`[safeBackgroundTask]` prefix,
//! task-failed / hook-failed messages, `if (!hook)` gate),
//! `src/app/services/project-switch-service.ts` (`variant || "default"`
//! fallback, initial `tokensUsed: 0`, `trackBackgroundSessions &&
//! ensureEventSubscription` gate),
//! `src/bot/services/project-switch-presentation.ts` (`if (ctx.chat)` gate),
//! `src/bot/services/attach-presentation.ts` (same-session equality,
//! same-with-message vs restore-vs-change choice, `forceFullRestore` gate,
//! context-info gate, initialized gate) and
//! `src/bot/services/model-views.ts` (`isInitialized` keyboard gate).
//!
//! Already covered elsewhere and NOT re-tested here:
//! `formatVariantForButton` (`agent_model_variant_gates`),
//! `getAttachBusyStatus` (`attach_foreground_gates`), restore orchestration
//! (`attach_restore_gates`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `switch_presentation_gates` is ported (only the constant-value test passes).

use otb_core::switch_presentation_gates::*;

// ---- constant (1) ----

#[test]
fn safe_background_prefix_value_parity() {
    assert_eq!(SAFE_BACKGROUND_TASK_PREFIX, "[safeBackgroundTask]");
}

// ---- safe-background-task (5) ----

#[test]
fn task_error_message_parity() {
    assert_eq!(
        safe_background_task_error_message("sync"),
        "[safeBackgroundTask] sync failed:".to_string()
    );
}

#[test]
fn hook_error_message_parity() {
    assert_eq!(
        safe_background_hook_error_message("sync", "onSuccess"),
        "[safeBackgroundTask] sync: onSuccess failed:".to_string()
    );
    assert_eq!(
        safe_background_hook_error_message("sync", "onError"),
        "[safeBackgroundTask] sync: onError failed:".to_string()
    );
}

#[test]
fn hook_runs_when_present_parity() {
    assert!(should_run_hook(true));
}

#[test]
fn hook_skips_when_missing_parity() {
    assert!(!should_run_hook(false));
}

// ---- project-switch-service (4) ----

#[test]
fn switch_variant_falls_back_to_default_parity() {
    assert_eq!(resolve_switch_variant(None), "default");
    assert_eq!(resolve_switch_variant(Some("")), "default");
}

#[test]
fn switch_variant_keeps_present_parity() {
    assert_eq!(resolve_switch_variant(Some("pro")), "pro");
    assert_eq!(resolve_switch_variant(Some("   ")), "   ");
}

#[test]
fn switch_initial_tokens_are_zero_parity() {
    assert_eq!(switch_initial_tokens_used(), 0);
}

#[test]
fn ensure_subscription_needs_track_and_ensure_parity() {
    assert!(should_ensure_event_subscription(true, true));
    assert!(!should_ensure_event_subscription(true, false));
    assert!(!should_ensure_event_subscription(false, true));
    assert!(!should_ensure_event_subscription(false, false));
}

// ---- presentations / model-views (7) ----

#[test]
fn switch_keyboard_initializes_with_chat_parity() {
    assert!(should_initialize_switch_keyboard(true));
    assert!(!should_initialize_switch_keyboard(false));
}

#[test]
fn model_views_keyboard_follows_init_parity() {
    assert!(should_update_model_views_keyboard(true));
    assert!(!should_update_model_views_keyboard(false));
}

#[test]
fn attach_sync_needs_init_parity() {
    assert!(should_sync_attach_state(true));
    assert!(!should_sync_attach_state(false));
}

#[test]
fn same_pinned_session_equality_parity() {
    assert!(is_same_pinned_session(Some("s1"), "s1"));
    assert!(!is_same_pinned_session(Some("s1"), "s2"));
    assert!(!is_same_pinned_session(None, "s1"));
    assert!(!is_same_pinned_session(Some(""), "s1"));
}

#[test]
fn prepare_same_session_with_message_parity() {
    assert_eq!(
        select_prepare_pinned_action(true, true, false),
        PreparePinnedAction::SameSession
    );
}

#[test]
fn prepare_restore_when_message_with_null_session_parity() {
    assert_eq!(
        select_prepare_pinned_action(false, true, true),
        PreparePinnedAction::Restore
    );
}

#[test]
fn prepare_change_otherwise_parity() {
    assert_eq!(
        select_prepare_pinned_action(false, false, false),
        PreparePinnedAction::Change
    );
    assert_eq!(
        select_prepare_pinned_action(false, false, true),
        PreparePinnedAction::Change
    );
    assert_eq!(
        select_prepare_pinned_action(true, false, false),
        PreparePinnedAction::Change
    );
    assert_eq!(
        select_prepare_pinned_action(false, true, false),
        PreparePinnedAction::Change
    );
}

#[test]
fn full_restore_follows_flag_parity() {
    assert!(should_full_restore_on_same_session(true));
    assert!(!should_full_restore_on_same_session(false));
}

#[test]
fn keyboard_context_follows_info_parity() {
    assert!(should_update_keyboard_context(true));
    assert!(!should_update_keyboard_context(false));
}
