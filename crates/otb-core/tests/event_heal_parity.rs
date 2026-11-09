//! Parity port of the reselect-heal gates in `src/opencode/events.ts`
//! (`isEventSubscriptionActive` + same-directory early return) and the
//! `attach-service.ts` reselect guard.

use otb_core::event_subscription::{
    is_subscription_active_for_reselect, should_continue_reconnect,
    should_heal_same_directory_subscription, should_keep_same_directory_subscription,
    SSE_IDLE_TIMEOUT_MS,
};
use otb_core::event_transport_gates::{
    is_fatal_no_stream_message, should_keep_subscription_on_reconnect, FATAL_NO_STREAM_ERROR,
    SSE_IDLE_TIMEOUT_ERROR,
};

#[test]
fn heal_keeps_healthy_same_directory_subscription() {
    assert!(should_keep_same_directory_subscription(true, true, true));
    assert!(!should_keep_same_directory_subscription(true, true, false));
    assert!(!should_keep_same_directory_subscription(true, false, true));
    assert!(!should_keep_same_directory_subscription(false, true, true));
}

#[test]
fn heal_resubscribes_when_same_directory_stream_is_missing() {
    assert!(should_heal_same_directory_subscription(true, true, false));
    assert!(!should_heal_same_directory_subscription(true, true, true));
    assert!(!should_heal_same_directory_subscription(false, true, false));
    assert!(!should_heal_same_directory_subscription(true, false, false));
}

#[test]
fn heal_reports_active_only_for_live_same_directory_stream() {
    assert!(is_subscription_active_for_reselect(true, true, true));
    assert!(!is_subscription_active_for_reselect(true, true, false));
    assert!(!is_subscription_active_for_reselect(false, true, true));
    assert!(!is_subscription_active_for_reselect(true, false, true));
}

#[test]
fn heal_keeps_idle_timeout_budget_and_reconnect_wait() {
    assert_eq!(SSE_IDLE_TIMEOUT_MS, 30_000);
    assert_eq!(SSE_IDLE_TIMEOUT_ERROR, "SSE stream idle timeout");
    assert!(should_continue_reconnect(true));
    assert!(!should_continue_reconnect(false));
}

#[test]
fn heal_treats_missing_stream_as_fatal_not_reconnectable() {
    assert!(is_fatal_no_stream_message(FATAL_NO_STREAM_ERROR));
    assert!(!is_fatal_no_stream_message(SSE_IDLE_TIMEOUT_ERROR));
    assert!(!is_fatal_no_stream_message("boom"));
}

#[test]
fn heal_keeps_subscription_on_reconnect_only_when_listening_same_directory() {
    assert!(should_keep_subscription_on_reconnect(true, true));
    assert!(!should_keep_subscription_on_reconnect(true, false));
    assert!(!should_keep_subscription_on_reconnect(false, true));
}
