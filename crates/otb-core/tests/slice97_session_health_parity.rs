//! Slice 97 parity: session / health-server / document-extractor / config-reload gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/app/services/session-service.ts` (`setCurrentSession` same-id keep
//! gate, `fetchSessionTitle` fetched-vs-error fallback),
//! `src/app/services/run-control-service.ts` (`isForegroundBusy` OR gate,
//! `getBusyDirectories` dedupe, reconcile early-return gate),
//! `src/app/services/health-server-service.ts` (`HEALTH_HOST` timeouts,
//! `GET`-only 405 gate, `split("?")[0]` path dispatch, ready 200-vs-503,
//! payload healthy-vs-degraded, `getUptimeSeconds`),
//! `src/app/services/document-extractor-service.ts` (`REQUEST_TIMEOUT_MS`,
//! configured gate, bearer header gate, timeout/HTTP/missing-text messages)
//! and `src/app/services/config-reload-service.ts`
//! (`CONFIG_RELOAD_TIMEOUT_MS`, in-flight join, success inversion, timeout
//! detach gate).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `session_health_gates` is ported (only the constant-value tests pass).

use otb_core::session_health_gates::*;

// ---- constants (7) ----

#[test]
fn health_host_value_parity() {
    assert_eq!(HEALTH_HOST, "127.0.0.1");
}

#[test]
fn opencode_check_timeout_value_parity() {
    assert_eq!(OPENCODE_CHECK_TIMEOUT_MS, 3000);
}

#[test]
fn close_connections_timeout_value_parity() {
    assert_eq!(CLOSE_CONNECTIONS_TIMEOUT_MS, 2000);
}

#[test]
fn doc_extractor_timeout_value_parity() {
    assert_eq!(DOC_EXTRACTOR_REQUEST_TIMEOUT_MS, 60_000);
}

#[test]
fn config_reload_timeout_value_parity() {
    assert_eq!(CONFIG_RELOAD_TIMEOUT_MS, 60_000);
}

#[test]
fn doc_extractor_not_configured_message_value_parity() {
    assert_eq!(
        DOC_EXTRACTOR_NOT_CONFIGURED_MESSAGE,
        "Document extractor is not configured: DOC_EXTRACTOR_URL is required"
    );
}

#[test]
fn doc_extractor_missing_text_message_value_parity() {
    assert_eq!(
        DOC_EXTRACTOR_MISSING_TEXT_MESSAGE,
        "Document extractor API response does not contain a text field"
    );
}

// ---- session-service (7) ----

#[test]
fn session_set_resets_when_no_previous_parity() {
    assert!(should_reset_on_session_set(None, "abc"));
}

#[test]
fn session_set_resets_when_different_id_parity() {
    assert!(should_reset_on_session_set(Some("old"), "new"));
}

#[test]
fn session_set_keeps_when_same_id_parity() {
    assert!(!should_reset_on_session_set(Some("abc"), "abc"));
}

#[test]
fn session_title_uses_fetched_parity() {
    assert_eq!(
        resolve_session_title(Some("Live title"), false, "Remembered"),
        "Live title".to_string()
    );
}

#[test]
fn session_title_falls_back_on_error_parity() {
    assert_eq!(
        resolve_session_title(Some("Live title"), true, "Remembered"),
        "Remembered".to_string()
    );
}

#[test]
fn session_title_falls_back_when_missing_parity() {
    assert_eq!(
        resolve_session_title(None, false, "Remembered"),
        "Remembered".to_string()
    );
}

#[test]
fn session_title_empty_fetched_kept_parity() {
    assert_eq!(
        resolve_session_title(Some(""), false, "Remembered"),
        String::new()
    );
}

// ---- run-control (8) ----

#[test]
fn fg_busy_both_idle_parity() {
    assert!(!is_foreground_busy(false, false));
}

#[test]
fn fg_busy_fg_only_parity() {
    assert!(is_foreground_busy(true, false));
}

#[test]
fn fg_busy_attach_only_parity() {
    assert!(is_foreground_busy(false, true));
}

#[test]
fn fg_busy_both_busy_parity() {
    assert!(is_foreground_busy(true, true));
}

#[test]
fn reconcile_gate_parity() {
    assert!(!should_reconcile_busy(false));
    assert!(should_reconcile_busy(true));
}

#[test]
fn busy_dirs_fg_only_parity() {
    let dirs = collect_busy_directories(&["/a".to_string(), "/b".to_string()], Some("/c"), false);
    assert_eq!(dirs, vec!["/a".to_string(), "/b".to_string()]);
}

#[test]
fn busy_dirs_includes_busy_attached_parity() {
    let dirs = collect_busy_directories(&["/a".to_string()], Some("/b"), true);
    assert_eq!(dirs, vec!["/a".to_string(), "/b".to_string()]);
}

#[test]
fn busy_dirs_dedupes_parity() {
    let dirs = collect_busy_directories(&["/a".to_string(), "/a".to_string()], Some("/a"), true);
    assert_eq!(dirs, vec!["/a".to_string()]);
}

// ---- health-server (13) ----

#[test]
fn health_method_get_allowed_parity() {
    assert!(is_health_method_allowed("GET"));
}

#[test]
fn health_method_post_rejected_parity() {
    assert!(!is_health_method_allowed("POST"));
    assert!(!is_health_method_allowed("get"));
}

#[test]
fn health_route_live_parity() {
    assert_eq!(
        resolve_health_route("GET", Some("/health/live")),
        HealthRoute::Live
    );
}

#[test]
fn health_route_live_strips_query_parity() {
    assert_eq!(
        resolve_health_route("GET", Some("/health/live?x=1")),
        HealthRoute::Live
    );
}

#[test]
fn health_route_ready_parity() {
    assert_eq!(
        resolve_health_route("GET", Some("/health/ready")),
        HealthRoute::Ready
    );
}

#[test]
fn health_route_root_parity() {
    assert_eq!(
        resolve_health_route("GET", Some("/health")),
        HealthRoute::Root
    );
    assert_eq!(resolve_health_route("GET", None), HealthRoute::Root);
}

#[test]
fn health_route_unknown_404_parity() {
    assert_eq!(
        resolve_health_route("GET", Some("/nope")),
        HealthRoute::NotFound
    );
}

#[test]
fn health_route_method_not_allowed_parity() {
    assert_eq!(
        resolve_health_route("POST", Some("/health/live")),
        HealthRoute::MethodNotAllowed
    );
}

#[test]
fn health_ready_status_healthy_200_parity() {
    assert_eq!(health_ready_status(true), 200);
}

#[test]
fn health_ready_status_degraded_503_parity() {
    assert_eq!(health_ready_status(false), 503);
}

#[test]
fn health_payload_status_parity() {
    assert_eq!(health_payload_status(true), "healthy");
    assert_eq!(health_payload_status(false), "degraded");
}

#[test]
fn uptime_zero_without_start_parity() {
    assert_eq!(uptime_seconds(None, 1_700_000_000_000), 0);
}

#[test]
fn uptime_counts_seconds_parity() {
    assert_eq!(uptime_seconds(Some(1_000), 61_000), 60);
    assert_eq!(uptime_seconds(Some(5_000), 5_900), 0);
}

// ---- document-extractor (8) ----

#[test]
fn doc_extractor_unconfigured_when_none_parity() {
    assert!(!is_doc_extractor_configured(None));
}

#[test]
fn doc_extractor_unconfigured_when_empty_parity() {
    assert!(!is_doc_extractor_configured(Some("")));
}

#[test]
fn doc_extractor_configured_when_url_parity() {
    assert!(is_doc_extractor_configured(Some("http://localhost:8000")));
    assert!(is_doc_extractor_configured(Some("   ")));
}

#[test]
fn doc_extractor_auth_header_parity() {
    assert_eq!(
        doc_extractor_auth_header(Some(" secret ")),
        Some("Bearer  secret ".to_string())
    );
}

#[test]
fn doc_extractor_auth_missing_parity() {
    assert_eq!(doc_extractor_auth_header(None), None);
    assert_eq!(doc_extractor_auth_header(Some("")), None);
}

#[test]
fn doc_extractor_timeout_message_parity() {
    let message = doc_extractor_timeout_message();
    assert!(message.contains("60000"));
    assert!(message.contains("timed out"));
}

#[test]
fn doc_extractor_http_error_parity() {
    assert_eq!(
        doc_extractor_http_error(500, "boom", "Internal Error"),
        "Document extractor API returned HTTP 500: boom".to_string()
    );
    assert_eq!(
        doc_extractor_http_error(503, "", "Service Unavailable"),
        "Document extractor API returned HTTP 503: Service Unavailable".to_string()
    );
}

#[test]
fn doc_extractor_missing_text_parity() {
    assert_eq!(
        extracted_text_or_error(Some("hello")),
        Ok("hello".to_string())
    );
    assert_eq!(
        extracted_text_or_error(None),
        Err(DOC_EXTRACTOR_MISSING_TEXT_MESSAGE)
    );
}

// ---- config-reload (5) ----

#[test]
fn config_reload_joins_inflight_parity() {
    assert!(should_join_inflight(true));
}

#[test]
fn config_reload_starts_fresh_parity() {
    assert!(!should_join_inflight(false));
}

#[test]
fn config_reload_success_changed_parity() {
    assert!(reload_success_model_changed(false));
}

#[test]
fn config_reload_success_unchanged_parity() {
    assert!(!reload_success_model_changed(true));
}

#[test]
fn config_reload_clear_on_timeout_parity() {
    assert!(should_clear_inflight_on_timeout(true, true));
    assert!(!should_clear_inflight_on_timeout(true, false));
    assert!(!should_clear_inflight_on_timeout(false, true));
}
