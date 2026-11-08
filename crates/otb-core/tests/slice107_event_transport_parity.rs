//! Slice 107 parity: OpenCode event-transport gates (RED).
//!
//! Mirrors the pure TS contract in `src/opencode/events.ts`
//! (`RECONNECT_BASE_DELAY_MS`, `RECONNECT_MAX_DELAY_MS`,
//! `FATAL_NO_STREAM_ERROR`, `SSE_IDLE_TIMEOUT_ERROR`,
//! `getReconnectDelayMs` exponential backoff with the 15000ms cap,
//! `normalizeDirectoryForComparison` backslash / trailing-slash /
//! drive-letter rule, `isSameDirectory` equality,
//! `isEventStreamIdleTimeoutError` message gate, `isEventLike` shape
//! gate, `normalizeGlobalEvent` mismatched-directory drop and
//! `eventDirectory || directory` fallback, `normalizeEvent` source
//! dispatch, `getReconnectInfo` restart-flag rule).
//! Subscription lifecycle, busy/idle routing, footer gating and compact
//! fallback already live in `event_subscription` and are NOT re-tested
//! here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `event_transport_gates` is ported (only the constant-value tests pass).

use otb_core::event_transport_gates::*;

// ---- constants (4) ----

#[test]
fn reconnect_base_delay_value_parity() {
    assert_eq!(RECONNECT_BASE_DELAY_MS, 1000);
}

#[test]
fn reconnect_max_delay_value_parity() {
    assert_eq!(RECONNECT_MAX_DELAY_MS, 15000);
}

#[test]
fn fatal_no_stream_error_value_parity() {
    assert_eq!(
        FATAL_NO_STREAM_ERROR,
        "No stream returned from event subscription"
    );
}

#[test]
fn idle_timeout_error_value_parity() {
    assert_eq!(SSE_IDLE_TIMEOUT_ERROR, "SSE stream idle timeout");
}

// ---- getReconnectDelayMs (4) ----

#[test]
fn reconnect_delay_first_attempt_parity() {
    assert_eq!(get_reconnect_delay_ms(1), 1000);
}

#[test]
fn reconnect_delay_backs_off_exponentially_parity() {
    assert_eq!(get_reconnect_delay_ms(2), 2000);
    assert_eq!(get_reconnect_delay_ms(3), 4000);
    assert_eq!(get_reconnect_delay_ms(4), 8000);
}

#[test]
fn reconnect_delay_caps_at_max_parity() {
    assert_eq!(get_reconnect_delay_ms(5), 15000);
    assert_eq!(get_reconnect_delay_ms(10), 15000);
}

#[test]
fn reconnect_delay_non_positive_attempt_stays_at_base_parity() {
    assert_eq!(get_reconnect_delay_ms(0), 1000);
    assert_eq!(get_reconnect_delay_ms(-3), 1000);
}

// ---- normalizeDirectoryForComparison (3) ----

#[test]
fn directory_normalizes_separators_and_trailing_slash_parity() {
    assert_eq!(
        normalize_directory_for_comparison("C:\\Repo\\"),
        "c:/repo".to_string()
    );
    assert_eq!(
        normalize_directory_for_comparison("/repo///"),
        "/repo".to_string()
    );
}

#[test]
fn directory_lowercases_drive_letter_only_parity() {
    assert_eq!(
        normalize_directory_for_comparison("D:/Work/Repo"),
        "d:/work/repo".to_string()
    );
    assert_eq!(
        normalize_directory_for_comparison("/Work/Repo"),
        "/Work/Repo".to_string()
    );
}

#[test]
fn directory_empty_and_root_strip_to_empty_parity() {
    assert_eq!(normalize_directory_for_comparison(""), "".to_string());
    assert_eq!(normalize_directory_for_comparison("/"), "".to_string());
}

// ---- isSameDirectory (2) ----

#[test]
fn same_directory_matches_across_separators_parity() {
    assert!(is_same_directory("C:\\Repo\\", "c:/repo"));
    assert!(is_same_directory("/repo", "/repo/"));
}

#[test]
fn same_directory_rejects_other_folder_parity() {
    assert!(!is_same_directory("/repo-a", "/repo-b"));
    assert!(!is_same_directory("C:/repo", "D:/repo"));
}

// ---- isEventStreamIdleTimeoutError message gate (2) ----

#[test]
fn idle_timeout_message_matches_parity() {
    assert!(is_idle_timeout_message("SSE stream idle timeout"));
}

#[test]
fn idle_timeout_message_rejects_other_parity() {
    assert!(!is_idle_timeout_message(
        "No stream returned from event subscription"
    ));
    assert!(!is_idle_timeout_message(""));
}

// ---- isEventLike (2) ----

#[test]
fn event_like_needs_record_type_and_properties_parity() {
    assert!(is_event_like(true, true, true));
}

#[test]
fn event_like_rejects_misshapen_values_parity() {
    assert!(!is_event_like(false, true, true));
    assert!(!is_event_like(true, false, true));
    assert!(!is_event_like(true, true, false));
}

// ---- normalizeGlobalEvent directory gates (3) ----

#[test]
fn global_event_drops_mismatched_directory_parity() {
    assert!(should_drop_global_event_on_directory(Some("C:/other"), false));
}

#[test]
fn global_event_keeps_same_or_missing_directory_parity() {
    assert!(!should_drop_global_event_on_directory(
        Some("C:/repo"),
        true
    ));
    assert!(!should_drop_global_event_on_directory(None, false));
    assert!(!should_drop_global_event_on_directory(None, true));
}

#[test]
fn global_event_directory_falls_back_parity() {
    assert_eq!(
        resolve_global_event_directory(Some("C:/repo"), "/default"),
        "C:/repo".to_string()
    );
    assert_eq!(
        resolve_global_event_directory(None, "/default"),
        "/default".to_string()
    );
    assert_eq!(
        resolve_global_event_directory(Some(""), "/default"),
        "/default".to_string()
    );
}

// ---- getReconnectInfo (3) ----

#[test]
fn reconnect_info_null_unless_connected_parity() {
    assert_eq!(resolve_reconnect_restarted(false, Some(true)), None);
    assert_eq!(resolve_reconnect_restarted(false, None), None);
}

#[test]
fn reconnect_info_passes_boolean_restarted_parity() {
    assert_eq!(resolve_reconnect_restarted(true, Some(true)), Some(true));
    assert_eq!(resolve_reconnect_restarted(true, Some(false)), Some(false));
}

#[test]
fn reconnect_info_null_when_restarted_not_boolean_parity() {
    assert_eq!(resolve_reconnect_restarted(true, None), None);
}
