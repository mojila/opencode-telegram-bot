//! Slice 111 parity: TTS response-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/handlers/tts-response-handler.ts`
//! (`sendTtsResponseForSession` send/skip/fail branching) and its cases
//! in `tests/bot/handlers/tts-response-handler.test.ts`:
//! - `!prepared.shouldSend` resolves `false` silently (no audio, no
//!   error notice, synthesis never attempted)
//! - prepared + `sendAudio` resolves -> `true`, no error notice
//! - prepared + `sendAudio` rejects -> `false` + `t("tts.failed")`
//!   user-facing notice
//! - the response mode persists across calls until cleared (registry
//!   owned by `prompt_request_gates`, noted here only as the reason two
//!   consecutive prepared sends both resolve `true`)
//!
//! Every test MUST FAIL with `unimplemented!()` panics until
//! `tts_response_gates` is ported.

use otb_core::tts_response_gates::*;

// --- failure key (1) ---

#[test]
fn slice111_01_failure_notice_key_is_tts_failed() {
    assert_eq!(TTS_FAILED_MESSAGE_KEY, "tts.failed");
}

// --- should_send_tts_audio gate (2) ---

#[test]
fn slice111_02_prepared_response_sends_audio() {
    assert!(should_send_tts_audio(true));
}

#[test]
fn slice111_03_unprepared_response_skips_silently() {
    assert!(!should_send_tts_audio(false));
}

// --- classify_tts_send_outcome (4) ---

#[test]
fn slice111_04_prepared_and_sent_is_sent() {
    assert_eq!(classify_tts_send_outcome(true, true), TtsSendOutcome::Sent);
}

#[test]
fn slice111_05_unprepared_never_sends_even_when_transport_would_succeed() {
    assert_eq!(classify_tts_send_outcome(false, true), TtsSendOutcome::Skipped);
    assert_eq!(
        classify_tts_send_outcome(false, false),
        TtsSendOutcome::Skipped
    );
}

#[test]
fn slice111_06_prepared_but_send_failed_is_failed() {
    assert_eq!(
        classify_tts_send_outcome(true, false),
        TtsSendOutcome::Failed
    );
}

// --- tts_handler_result (2) ---

#[test]
fn slice111_07_only_sent_resolves_true() {
    assert!(tts_handler_result(TtsSendOutcome::Sent));
    assert!(!tts_handler_result(TtsSendOutcome::Skipped));
    assert!(!tts_handler_result(TtsSendOutcome::Failed));
}

// --- tts_failure_notice_key / should_report_tts_failure (3) ---

#[test]
fn slice111_08_only_failed_carries_the_failure_notice() {
    assert_eq!(
        tts_failure_notice_key(TtsSendOutcome::Failed),
        Some("tts.failed")
    );
    assert_eq!(tts_failure_notice_key(TtsSendOutcome::Sent), None);
    assert_eq!(tts_failure_notice_key(TtsSendOutcome::Skipped), None);
}

#[test]
fn slice111_09_only_failed_reports_the_failure() {
    assert!(should_report_tts_failure(TtsSendOutcome::Failed));
    assert!(!should_report_tts_failure(TtsSendOutcome::Sent));
    assert!(!should_report_tts_failure(TtsSendOutcome::Skipped));
}

// --- end-to-end decision table mirroring the TS handler tests (3) ---

#[test]
fn slice111_10_text_only_mode_skips_without_synthesis_or_notice() {
    // TS: `getResponseMode: () => "text_only"` -> `prepare` yields
    // `shouldSend: false` -> resolves `false`, synth/audio/notice untouched.
    let outcome = classify_tts_send_outcome(should_send_tts_audio(false), true);
    assert_eq!(outcome, TtsSendOutcome::Skipped);
    assert!(!tts_handler_result(outcome));
    assert!(!should_report_tts_failure(outcome));
}

#[test]
fn slice111_11_unconfigured_tts_skips_without_synthesis_or_notice() {
    // TS: `isTtsConfigured: () => false` -> `shouldSend: false` -> silent `false`.
    let outcome = classify_tts_send_outcome(should_send_tts_audio(false), true);
    assert_eq!(outcome, TtsSendOutcome::Skipped);
    assert!(!tts_handler_result(outcome));
    assert_eq!(tts_failure_notice_key(outcome), None);
}

#[test]
fn slice111_12_send_failure_reports_tts_failed_and_resolves_false() {
    // TS: `sendAudio` rejects -> `sendMessage(chatId, t("tts.failed"))`,
    // resolves `false`.
    let outcome = classify_tts_send_outcome(should_send_tts_audio(true), false);
    assert_eq!(outcome, TtsSendOutcome::Failed);
    assert!(!tts_handler_result(outcome));
    assert_eq!(tts_failure_notice_key(outcome), Some(TTS_FAILED_MESSAGE_KEY));
}
