//! Voice/media intake gates: size caps, transcribe dispatch, prompt ingest.
//!
//! Pure composition over the existing pieces, in dependency order:
//! 1. Size caps (`src/app/services/file-download-service.ts:8`
//!    `MAX_FILE_SIZE_BYTES = 20 MiB`, enforced as
//!    `if (file.file_size && ...)` at `file-download-service.ts:85-88`).
//! 2. Transcribe dispatch guards (`src/bot/handlers/voice-handler.ts:187-228`:
//!    `file_id` presence, `isSttConfigured`
//!    (`src/app/services/stt-service.ts:24-26`), the `stt.not_configured` /
//!    `stt.error` / `stt.empty_result` replies).
//! 3. Prompt ingest reuse (`voice-handler.ts:251-277`: the `STT_NOTE_PROMPT`
//!    note prefix, the `all`/`auto` `responseMode`, `createIncomingPrompt`
//!    sendability before `processUserPrompt`).
//!
//! No network, no Telegram calls: download, transcribe, and send stay with
//! the caller (live Telegram download and dispatcher restores are deferred).
//! Failures surface as i18n keys; the bot layer localizes them.

/// Max inbound voice/audio bytes (TS: `MAX_FILE_SIZE_BYTES = 20 MiB`).
pub const VOICE_MAX_BYTES: u64 = 20 * 1024 * 1024;

/// Max inbound document bytes (same TS cap, document path).
pub const FILE_DOWNLOAD_MAX_BYTES: u64 = 20 * 1024 * 1024;

/// Default Whisper model (TS: `STT_MODEL` fallback in `src/config.ts:315`).
pub const STT_DEFAULT_MODEL: &str = "whisper-large-v3-turbo";

/// Ordered voice-intake verdict (`voice-handler.ts:187-204` guard order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceIntake {
    /// No `file_id` (`voice-handler.ts:192-195`): TS logs and returns.
    Ignore,
    /// Caller replies with the localized key (`stt.not_configured` when STT
    /// is off, `bot.file_too_large` past the 20 MiB cap).
    Reject { key: &'static str },
    /// Download and transcribe.
    Accept,
}

/// Whether an inbound voice/audio size passes the 20 MiB cap.
/// Unknown or zero size passes, mirroring `if (file.file_size && ...)` in TS.
pub fn is_voice_file_size_allowed(file_size: Option<u64>) -> bool {
    match file_size {
        None | Some(0) => true,
        Some(size) => size <= VOICE_MAX_BYTES,
    }
}

/// Whether an inbound document size passes the 20 MiB download cap
/// (same TS gate, document path).
pub fn is_file_download_size_allowed(file_size: Option<u64>) -> bool {
    match file_size {
        None | Some(0) => true,
        Some(size) => size <= FILE_DOWNLOAD_MAX_BYTES,
    }
}

/// Voice dispatch in TS guard order: missing `file_id` is ignored, an
/// unconfigured STT answers `stt.not_configured`, an over-cap file answers
/// `bot.file_too_large`; anything else proceeds to download + transcribe.
pub fn decide_voice_intake(
    file_id: Option<&str>,
    file_size: Option<u64>,
    stt_configured: bool,
) -> VoiceIntake {
    if file_id.is_none_or(|id| id.is_empty()) {
        return VoiceIntake::Ignore;
    }
    if !stt_configured {
        return VoiceIntake::Reject {
            key: "stt.not_configured",
        };
    }
    if !is_voice_file_size_allowed(file_size) {
        return VoiceIntake::Reject {
            key: "bot.file_too_large",
        };
    }
    VoiceIntake::Accept
}

/// Transcript gate: blank transcripts answer `stt.empty_result`
/// (`voice-handler.ts:224-228`); anything else proceeds to prompt ingest.
pub fn transcript_reject_key(text: &str) -> Option<&'static str> {
    if crate::voice_cron_gates::is_empty_transcript(text) {
        Some("stt.empty_result")
    } else {
        None
    }
}

/// LLM prompt text for a transcript (TS: `textForLLM` at
/// `voice-handler.ts:251-258`): note prefix plus transcript.
pub fn build_voice_llm_text(note_prompt: &str, recognized_text: &str) -> String {
    crate::voice_cron_gates::voice_llm_text(note_prompt, recognized_text)
}

/// Whether the transcribed text is worth sending (TS: `createIncomingPrompt`
/// sendability before `processUserPrompt`).
pub fn is_voice_prompt_sendable(text: &str) -> bool {
    crate::media_prompt::is_prompt_sendable(text, 0, 0)
}

/// Normalize the `STT_REQUEST_FORMAT` value (`src/config.ts:320-321`):
/// `"json"` selects the base64 `input_audio` body, anything else (blank
/// included) is the standard multipart upload.
pub fn normalize_stt_request_format(raw: &str) -> &'static str {
    if raw.trim().eq_ignore_ascii_case("json") {
        "json"
    } else {
        "multipart"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_caps_mirror_the_ts_file_size_gate() {
        assert!(is_voice_file_size_allowed(None));
        assert!(is_voice_file_size_allowed(Some(0)));
        assert!(is_voice_file_size_allowed(Some(VOICE_MAX_BYTES)));
        assert!(!is_voice_file_size_allowed(Some(VOICE_MAX_BYTES + 1)));
        assert!(is_file_download_size_allowed(None));
        assert!(is_file_download_size_allowed(Some(FILE_DOWNLOAD_MAX_BYTES)));
        assert!(!is_file_download_size_allowed(Some(
            FILE_DOWNLOAD_MAX_BYTES + 1
        )));
    }

    #[test]
    fn intake_follows_the_ts_guard_order() {
        // No file id: ignored even when STT is off and the file is huge.
        assert_eq!(
            decide_voice_intake(None, Some(VOICE_MAX_BYTES + 1), false),
            VoiceIntake::Ignore
        );
        assert_eq!(
            decide_voice_intake(Some(""), None, true),
            VoiceIntake::Ignore
        );
        // STT off beats the size cap.
        assert_eq!(
            decide_voice_intake(Some("id"), Some(VOICE_MAX_BYTES + 1), false),
            VoiceIntake::Reject {
                key: "stt.not_configured"
            }
        );
        // Over cap with STT on.
        assert_eq!(
            decide_voice_intake(Some("id"), Some(VOICE_MAX_BYTES + 1), true),
            VoiceIntake::Reject {
                key: "bot.file_too_large"
            }
        );
        // Happy path, including the unknown-size pass-through.
        assert_eq!(
            decide_voice_intake(Some("id"), None, true),
            VoiceIntake::Accept
        );
        assert_eq!(
            decide_voice_intake(Some("id"), Some(VOICE_MAX_BYTES), true),
            VoiceIntake::Accept
        );
    }

    #[test]
    fn transcript_gate_and_prompt_ingest_reuse_ts_shapes() {
        assert_eq!(transcript_reject_key("   "), Some("stt.empty_result"));
        assert_eq!(transcript_reject_key("hello"), None);
        assert_eq!(build_voice_llm_text("", "hi"), "hi");
        assert_eq!(
            build_voice_llm_text("speak slowly", "hi"),
            "[Note: speak slowly]\nhi"
        );
        assert_eq!(build_voice_llm_text("false", "hi"), "hi");
        assert!(is_voice_prompt_sendable("hi"));
        assert!(!is_voice_prompt_sendable("   "));
    }

    #[test]
    fn request_format_defaults_to_multipart() {
        assert_eq!(normalize_stt_request_format("json"), "json");
        assert_eq!(normalize_stt_request_format("  JSON  "), "json");
        assert_eq!(normalize_stt_request_format("multipart"), "multipart");
        assert_eq!(normalize_stt_request_format(""), "multipart");
        assert_eq!(normalize_stt_request_format("bogus"), "multipart");
    }
}
