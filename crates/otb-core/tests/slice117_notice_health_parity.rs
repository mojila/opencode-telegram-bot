//! Slice 117 parity: notice-delivery + runtime-health pure gates (RED).
//!
//! Mirrors the pure TS contract in:
//! - `src/bot/messages/thinking-message.ts` (`deliverThinkingMessage`
//!   sends `t("bot.thinking")` via `sendTextNow` with the
//!   `"thinking_started"` tag and never touches `enqueue`)
//! - `src/bot/telegram-outage-notices.ts` (`NOTICE_TEXT`: `undelivered`
//!   maps to `"bot.assistant_reply_undelivered"`, `skipped` maps to
//!   `"bot.stale_messages_skipped"`)
//! - `src/runtime/bot-version.ts` (`getBotVersion` falls back to
//!   `"unknown"` when the version is missing or unreadable)
//! - `src/runtime/healthcheck.ts` (5000ms probe timeout, `"/health/live"`
//!   probe path, port `0` passes without a request, probe `200` passes,
//!   any other status or a failed fetch fails)
//! - `src/runtime/service/types.ts` (`"running" | "stopped"` status
//!   union, `"stale" | "invalid" | null` cleanup-reason union)
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `notice_health_gates` is ported (only constant-value tests pass).

use otb_core::notice_health_gates::*;

// --- thinking indicator constants (2) ---

#[test]
fn slice117_01_thinking_key_is_bot_thinking() {
    assert_eq!(THINKING_MESSAGE_KEY, "bot.thinking");
}

#[test]
fn slice117_02_thinking_tag_is_thinking_started() {
    assert_eq!(THINKING_STARTED_TAG, "thinking_started");
}

// --- should_send_thinking_now (1) ---

#[test]
fn slice117_03_thinking_sends_immediately_never_enqueues() {
    assert!(should_send_thinking_now());
}

// --- outage_notice_text_key (2) ---

#[test]
fn slice117_04_undelivered_maps_to_reply_undelivered_key() {
    assert_eq!(
        outage_notice_text_key(OutageNoticeKind::Undelivered),
        "bot.assistant_reply_undelivered"
    );
}

#[test]
fn slice117_05_skipped_maps_to_stale_messages_skipped_key() {
    assert_eq!(
        outage_notice_text_key(OutageNoticeKind::Skipped),
        "bot.stale_messages_skipped"
    );
}

// --- bot version (3) ---

#[test]
fn slice117_06_version_constant_is_unknown() {
    assert_eq!(UNKNOWN_BOT_VERSION, "unknown");
}

#[test]
fn slice117_07_present_version_is_kept() {
    assert_eq!(resolve_bot_version_text(Some("0.27.0")), "0.27.0");
}

#[test]
fn slice117_08_missing_version_falls_back_to_unknown() {
    assert_eq!(resolve_bot_version_text(None), "unknown");
    // TS `??` is nullish-only: an empty string is kept, not replaced.
    assert_eq!(resolve_bot_version_text(Some("")), "");
}

// --- healthcheck probe constants (2) ---

#[test]
fn slice117_09_probe_timeout_is_5000ms() {
    assert_eq!(HEALTH_PROBE_TIMEOUT_MS, 5000);
}

#[test]
fn slice117_10_probe_path_is_health_live() {
    assert_eq!(HEALTH_LIVE_PATH, "/health/live");
}

// --- should_skip_health_probe (2) ---

#[test]
fn slice117_11_zero_port_passes_without_a_request() {
    assert!(should_skip_health_probe(0));
}

#[test]
fn slice117_12_nonzero_port_probes() {
    assert!(!should_skip_health_probe(3100));
    assert!(!should_skip_health_probe(80));
}

// --- health_probe_exit_code (3) ---

#[test]
fn slice117_13_probe_200_passes() {
    assert_eq!(health_probe_exit_code(Some(200)), 0);
}

#[test]
fn slice117_14_probe_non_200_fails() {
    assert_eq!(health_probe_exit_code(Some(500)), 1);
    assert_eq!(health_probe_exit_code(Some(404)), 1);
}

#[test]
fn slice117_15_failed_probe_fails() {
    assert_eq!(health_probe_exit_code(None), 1);
}

// --- service status (3) ---

#[test]
fn slice117_16_parses_running_and_stopped() {
    assert_eq!(
        parse_bot_service_status("running"),
        Some(BotServiceStatusValue::Running)
    );
    assert_eq!(
        parse_bot_service_status("stopped"),
        Some(BotServiceStatusValue::Stopped)
    );
}

#[test]
fn slice117_17_rejects_unknown_status() {
    assert_eq!(parse_bot_service_status("banana"), None);
    assert_eq!(parse_bot_service_status(""), None);
}

#[test]
fn slice117_18_only_running_counts_as_running() {
    assert!(is_bot_service_running(BotServiceStatusValue::Running));
    assert!(!is_bot_service_running(BotServiceStatusValue::Stopped));
}

// --- service cleanup reason (3) ---

#[test]
fn slice117_19_parses_stale_and_invalid() {
    assert_eq!(
        parse_service_cleanup_reason(Some("stale")),
        Some(ServiceCleanupReason::Stale)
    );
    assert_eq!(
        parse_service_cleanup_reason(Some("invalid")),
        Some(ServiceCleanupReason::Invalid)
    );
}

#[test]
fn slice117_20_null_cleanup_reason_stays_none() {
    assert_eq!(parse_service_cleanup_reason(None), None);
}

#[test]
fn slice117_21_unknown_cleanup_reason_stays_none() {
    assert_eq!(parse_service_cleanup_reason(Some("other")), None);
    assert_eq!(parse_service_cleanup_reason(Some("")), None);
}
