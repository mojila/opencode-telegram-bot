//! TTS response-handler pure gates (slice 111, RED).
//!
//! Rust port of the pure remainder in
//! `src/bot/handlers/tts-response-handler.ts`
//! (`sendTtsResponseForSession` send/skip/fail branching):
//! - the `if (!prepared.shouldSend) return false` gate (silent skip:
//!   no audio send, no error notice, synthesis never attempted)
//! - success path (`sendAudio` resolves -> `return true`, no error
//!   notice)
//! - failure path (`sendAudio` rejects -> user-facing `t("tts.failed")`
//!   notice with its own nested catch, `return false`)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice111_tts_response_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `prepareTtsResponseForSession` orchestration (service gates owned by
//!   `media_prompt`: mode/empty/configured/`MAX_TTS_INPUT_CHARS` gates)
//! - the `promptResponseModes` registry (`set` / `get` / `clear`, owned by
//!   `prompt_request_gates`)
//! - `synthesizeSpeech` audio generation, `InputFile` construction and
//!   `api.sendAudio` / `api.sendMessage` I/O, `logger` calls.
//!
//! Already ported elsewhere and NOT re-tested here:
//! `stripMarkdownForSpeech` / `extractLanguageCode` / `isTtsConfigured` /
//! `should_prepare_tts_response` (`media_prompt` owner), response-mode
//! resolve + registry (`prompt_request_gates` owner).
//!
//! Deviations: none intended. The handler's `boolean` result maps onto
//! [`TtsSendOutcome`]: only `Sent` returns `true`.

/// User-facing notice key sent when the audio send fails
/// (TS: `t("tts.failed")` in the `catch` of `sendTtsResponseForSession`).
pub const TTS_FAILED_MESSAGE_KEY: &str = "tts.failed";

/// Outcome of one `sendTtsResponseForSession` call
/// (TS: the `boolean` it resolves with, refined by whether the failure
/// notice goes out: `Sent(true)`, silent `Skipped(false)`,
/// `Failed(false` + `tts.failed` notice `)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsSendOutcome {
    Sent,
    Skipped,
    Failed,
}

/// `true` when the prepared response carries audio to send
/// (TS: `if (!prepared.shouldSend) return false;` — a `false` here
/// sends neither audio nor the failure notice).
pub fn should_send_tts_audio(prepared_should_send: bool) -> bool {
    prepared_should_send
}

/// Outcome from the prepare gate plus the audio-send result
/// (TS: `!shouldSend` -> silent `false`; send resolves -> `true`;
/// send rejects -> `false` with the `tts.failed` notice).
pub fn classify_tts_send_outcome(
    prepared_should_send: bool,
    send_succeeded: bool,
) -> TtsSendOutcome {
    if !prepared_should_send {
        return TtsSendOutcome::Skipped;
    }
    if send_succeeded {
        TtsSendOutcome::Sent
    } else {
        TtsSendOutcome::Failed
    }
}

/// Handler `boolean` result for an outcome
/// (TS: `sendTtsResponseForSession` resolves `true` only when the
/// audio was sent).
pub fn tts_handler_result(outcome: TtsSendOutcome) -> bool {
    outcome == TtsSendOutcome::Sent
}

/// i18n key of the user-facing notice for an outcome, or `None` when
/// nothing is reported (TS: only the `catch` path calls
/// `api.sendMessage(chatId, t("tts.failed"))`).
pub fn tts_failure_notice_key(outcome: TtsSendOutcome) -> Option<&'static str> {
    match outcome {
        TtsSendOutcome::Failed => Some(TTS_FAILED_MESSAGE_KEY),
        _ => None,
    }
}

/// `true` when the outcome reports the user-facing failure notice
/// (TS: only the send-failure path).
pub fn should_report_tts_failure(outcome: TtsSendOutcome) -> bool {
    outcome == TtsSendOutcome::Failed
}
