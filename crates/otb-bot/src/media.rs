//! Media/TTS/STT parity helpers (T8, structure only).
//!
//! Node parity (pure subset only):
//! - `src/config.ts` (`TtsProvider = "openai" | "google" | "elevenlabs" |
//!   "edge"`, default `"openai"`, per-provider default voice, `stt`
//!   `apiUrl`/`apiKey` shape)
//! - `src/app/services/stt-service.ts` (`isSttConfigured`, `getAudioFormat`
//!   extension map)
//! - `src/app/services/file-download-service.ts` (`MAX_FILE_SIZE_BYTES` =
//!   20 MiB download cap)
//! - `src/app/services/prompt-queue-manager.ts` (`MAX_QUEUED_MEDIA_BYTES` =
//!   20 MiB)
//! - `src/bot/handlers/voice-handler.ts` (`.oga` -> `.ogg` rename,
//!   `audio.ogg` default)
//!
//! Pure predicates and value transforms only. No network, no ffmpeg, no
//! Telegram calls here: speech synthesis, file download, audio conversion,
//! and `api.sendAudio` stay with the caller. (There is no ffmpeg to port:
//! `grep -rn "ffmpeg" src` has no matches in the TS source.)

/// Supported TTS providers (TS: `TtsProvider` in `src/config.ts`).
pub const TTS_PROVIDERS: &[&str] = &["openai", "google", "elevenlabs", "edge"];

/// TTS provider fallback (TS: `getOptionalTtsProviderEnvVar(..., "openai")`).
pub const DEFAULT_TTS_PROVIDER: &str = "openai";

/// Max inbound Telegram file size (TS: `MAX_FILE_SIZE_BYTES = 20MiB` in
/// `file-download-service.ts`).
pub const MAX_VOICE_FILE_BYTES: u64 = 20 * 1024 * 1024;

/// Max queued media bytes (TS: `MAX_QUEUED_MEDIA_BYTES = 20MiB` in
/// `prompt-queue-manager.ts`).
pub const MAX_QUEUED_MEDIA_BYTES: u64 = 20 * 1024 * 1024;

/// Normalize a raw provider name; unknown/blank falls back to `"openai"`
/// (TS: `getOptionalTtsProviderEnvVar`).
pub fn normalize_tts_provider(raw: Option<&str>) -> &'static str {
    match raw.map(str::trim).unwrap_or("") {
        value if value.eq_ignore_ascii_case("openai") => "openai",
        value if value.eq_ignore_ascii_case("google") => "google",
        value if value.eq_ignore_ascii_case("elevenlabs") => "elevenlabs",
        value if value.eq_ignore_ascii_case("edge") => "edge",
        _ => DEFAULT_TTS_PROVIDER,
    }
}

/// Default voice per provider (TS: `defaultVoice` in `src/config.ts`).
pub fn default_tts_voice(provider: &str) -> &'static str {
    match normalize_tts_provider(Some(provider)) {
        "google" => "en-US-Studio-O",
        "elevenlabs" => "21m00Tcm4TlvDq8ikWAM",
        "edge" => "en-US-EmmaMultilingualNeural",
        _ => "alloy",
    }
}

/// TTS configured gate (TS: `isTtsConfigured`; google needs credentials,
/// edge is always configured, others need URL + key).
pub fn is_tts_configured(
    provider: &str,
    api_url: &str,
    api_key: &str,
    google_credentials_set: bool,
) -> bool {
    otb_core::media_prompt::is_tts_configured(provider, api_url, api_key, google_credentials_set)
}

/// STT configured gate (TS: `isSttConfigured`: URL + key are set).
pub fn is_stt_configured(api_url: &str, api_key: &str) -> bool {
    otb_core::voice_cron_gates::is_stt_configured(api_url, api_key)
}

/// Lower-cased audio format for a filename, defaulting to `"ogg"`
/// (TS: `getAudioFormat`).
pub fn audio_format_for_filename(filename: &str) -> String {
    otb_core::voice_cron_gates::audio_format_for_filename(filename)
}

/// Basename of a Telegram `file_path`, `.oga` renamed to `.ogg`,
/// defaulting to `audio.ogg` (TS: voice download filename gate).
pub fn normalize_voice_filename(file_path: &str) -> String {
    otb_core::voice_cron_gates::normalize_voice_download_filename(file_path)
}

/// Whether an inbound file fits the 20 MiB Telegram download cap.
/// Unknown size passes, mirroring `if (file.file_size && ...)` in TS.
pub fn is_voice_file_size_allowed(file_size: Option<u64>) -> bool {
    otb_core::voice_media_intake_gates::is_voice_file_size_allowed(file_size)
}

/// Whether an inbound document fits the 20 MiB download cap (same TS gate,
/// document path).
pub fn is_file_download_size_allowed(file_size: Option<u64>) -> bool {
    otb_core::voice_media_intake_gates::is_file_download_size_allowed(file_size)
}

/// Env var carrying the STT API base URL (Node `STT_API_URL`).
pub const STT_API_URL_ENV: &str = "STT_API_URL";

/// Env var carrying the STT API key (Node `STT_API_KEY`).
pub const STT_API_KEY_ENV: &str = "STT_API_KEY";

/// Env var carrying the STT model (Node `STT_MODEL`).
pub const STT_MODEL_ENV: &str = "STT_MODEL";

/// Env var carrying the STT language hint (Node `STT_LANGUAGE`).
pub const STT_LANGUAGE_ENV: &str = "STT_LANGUAGE";

/// Env var carrying the STT note preface (Node `STT_NOTE_PROMPT`).
pub const STT_NOTE_PROMPT_ENV: &str = "STT_NOTE_PROMPT";

/// Env var carrying the STT request format (Node `STT_REQUEST_FORMAT`).
pub const STT_REQUEST_FORMAT_ENV: &str = "STT_REQUEST_FORMAT";

/// STT request timeout (Node `STT_REQUEST_TIMEOUT_MS = 60_000` in
/// `stt-service.ts`; the live transcribe call stays a dispatcher follow-up).
pub const STT_REQUEST_TIMEOUT_MS: u64 = 60_000;

/// STT API base URL from the environment (blank counts as missing).
pub fn stt_api_url_from_env() -> String {
    std::env::var(STT_API_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

/// STT API key from the environment (blank counts as missing; the value
/// itself is never logged).
pub fn stt_api_key_from_env() -> String {
    std::env::var(STT_API_KEY_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

/// STT model from the environment (Node `STT_MODEL` fallback to
/// `whisper-large-v3-turbo`).
pub fn stt_model_from_env() -> String {
    std::env::var(STT_MODEL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| otb_core::voice_media_intake_gates::STT_DEFAULT_MODEL.to_string())
}

/// STT language hint from the environment (`None` when blank; sent only
/// when set, like the TS multipart/json branches).
pub fn stt_language_from_env() -> Option<String> {
    std::env::var(STT_LANGUAGE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// STT note preface from the environment (blank/`"false"`/`"0"` is gated
/// out by [`otb_core::voice_cron_gates::should_include_stt_note`]).
pub fn stt_note_prompt_from_env() -> String {
    std::env::var(STT_NOTE_PROMPT_ENV).ok().unwrap_or_default()
}

/// STT request format from the environment (`"json"` or `"multipart"`,
/// default `"multipart"`).
pub fn stt_request_format_from_env() -> &'static str {
    match std::env::var(STT_REQUEST_FORMAT_ENV).ok().as_deref() {
        Some(raw)
            if otb_core::voice_media_intake_gates::normalize_stt_request_format(raw) == "json" =>
        {
            "json"
        }
        _ => "multipart",
    }
}

/// `{apiUrl}/audio/transcriptions` for an explicit base URL
/// (TS: `transcribeAudio` URL build in `stt-service.ts:50`).
pub fn stt_transcriptions_url_for(api_url: &str) -> String {
    otb_core::voice_cron_gates::stt_transcriptions_url(api_url)
}

/// `Bearer` auth value for an STT key (TS: `stt-service.ts:77`);
/// the key itself must never be logged.
pub fn stt_bearer_header(api_key: &str) -> String {
    format!("Bearer {api_key}")
}

/// Ordered voice-intake verdict (TS guard order in
/// `src/bot/handlers/voice-handler.ts:187-204`).
pub use otb_core::voice_media_intake_gates::VoiceIntake;

/// Voice dispatch from its parts: missing `file_id` is ignored, an
/// unconfigured STT answers `stt.not_configured`, an over-cap file answers
/// `bot.file_too_large`; anything else proceeds to download + transcribe.
pub fn decide_voice_intake(
    file_id: Option<&str>,
    file_size: Option<u64>,
    stt_configured: bool,
) -> VoiceIntake {
    otb_core::voice_media_intake_gates::decide_voice_intake(file_id, file_size, stt_configured)
}

/// Localized reply for a voice-intake rejection (`None` when there is
/// nothing to say: ignored or accepted). `{maxSizeMb}` is filled with the
/// 20 MiB cap; keys without placeholders pass through.
pub fn voice_intake_error_text(locale: &str, intake: VoiceIntake) -> Option<String> {
    match intake {
        VoiceIntake::Ignore | VoiceIntake::Accept => None,
        VoiceIntake::Reject { key } => {
            let template = crate::locale::translate(locale, key);
            Some(template.replace(
                "{maxSizeMb}",
                &(otb_core::voice_media_intake_gates::VOICE_MAX_BYTES / (1024 * 1024)).to_string(),
            ))
        }
    }
}

/// Localized `stt.recognizing` status text (edited in place once the
/// transcript lands, like the TS status message).
pub fn voice_recognizing_text(locale: &str) -> String {
    crate::locale::translate(locale, "stt.recognizing")
}

/// Localized `stt.empty_result` text for a blank transcript.
pub fn voice_empty_result_text(locale: &str) -> String {
    crate::locale::translate(locale, "stt.empty_result")
}

/// Localized `stt.error` text with the `{error}` detail filled in.
/// Callers pass a redacted detail (never the STT key or token).
pub fn voice_error_text(locale: &str, error: &str) -> String {
    crate::locale::translate(locale, "stt.error").replace("{error}", error)
}

/// Localized `bot.file_too_large` text with the 20 MiB cap filled in.
pub fn file_too_large_text(locale: &str) -> String {
    crate::locale::translate(locale, "bot.file_too_large").replace(
        "{maxSizeMb}",
        &(otb_core::voice_media_intake_gates::FILE_DOWNLOAD_MAX_BYTES / (1024 * 1024)).to_string(),
    )
}

/// Localized `queue.media_limit` text with the queued-media cap filled in.
pub fn queued_media_limit_text(locale: &str) -> String {
    crate::locale::translate(locale, "queue.media_limit").replace(
        "{maxSizeMb}",
        &otb_core::message_ingest::format_queued_media_limit_mb(),
    )
}

/// LLM prompt text for a transcript: note prefix plus transcript
/// (TS: `textForLLM` in `voice-handler.ts:251-258`).
pub fn build_voice_llm_text(note_prompt: &str, recognized_text: &str) -> String {
    otb_core::voice_media_intake_gates::build_voice_llm_text(note_prompt, recognized_text)
}

/// Whether the transcribed text is worth sending as a prompt.
pub fn is_voice_prompt_sendable(text: &str) -> bool {
    otb_core::voice_media_intake_gates::is_voice_prompt_sendable(text)
}

/// Whether queued media fits the 20 MiB queue cap (TS:
/// `rejectQueuedMediaBeforePreparation` budget).
pub fn is_queued_media_size_allowed(bytes: u64) -> bool {
    bytes <= MAX_QUEUED_MEDIA_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_falls_back_to_openai() {
        assert_eq!(normalize_tts_provider(None), "openai");
        assert_eq!(normalize_tts_provider(Some("")), "openai");
        assert_eq!(normalize_tts_provider(Some("  EDGE  ")), "edge");
        assert_eq!(normalize_tts_provider(Some("unknown")), "openai");
        assert_eq!(TTS_PROVIDERS.len(), 4);
    }

    #[test]
    fn default_voice_matches_ts_config() {
        assert_eq!(default_tts_voice("google"), "en-US-Studio-O");
        assert_eq!(default_tts_voice("elevenlabs"), "21m00Tcm4TlvDq8ikWAM");
        assert_eq!(default_tts_voice("edge"), "en-US-EmmaMultilingualNeural");
        assert_eq!(default_tts_voice("openai"), "alloy");
        assert_eq!(default_tts_voice("bogus"), "alloy");
    }

    #[test]
    fn size_and_format_gates_mirror_ts() {
        assert!(is_voice_file_size_allowed(None));
        assert!(is_voice_file_size_allowed(Some(0)));
        assert!(is_voice_file_size_allowed(Some(MAX_VOICE_FILE_BYTES)));
        assert!(!is_voice_file_size_allowed(Some(MAX_VOICE_FILE_BYTES + 1)));
        assert!(is_file_download_size_allowed(None));
        assert!(!is_file_download_size_allowed(Some(
            MAX_VOICE_FILE_BYTES + 1
        )));
        assert!(is_queued_media_size_allowed(MAX_QUEUED_MEDIA_BYTES));
        assert!(!is_queued_media_size_allowed(MAX_QUEUED_MEDIA_BYTES + 1));
        assert_eq!(audio_format_for_filename("voice.oga"), "ogg");
        assert_eq!(audio_format_for_filename("a.MP3"), "mp3");
        assert_eq!(audio_format_for_filename("noext"), "ogg");
        assert_eq!(normalize_voice_filename("music/file.oga"), "file.ogg");
        assert_eq!(normalize_voice_filename(""), "audio.ogg");
        assert!(is_stt_configured("https://stt.example", "key"));
        assert!(!is_stt_configured("", "key"));
        assert!(is_tts_configured("edge", "", "", false));
        assert!(!is_tts_configured("openai", "", "", false));
    }

    #[test]
    fn intake_guards_and_localized_errors_follow_ts_order() {
        // Guard order: missing id ignores, STT-off rejects, over-cap
        // rejects, the rest accepts.
        assert_eq!(decide_voice_intake(None, None, false), VoiceIntake::Ignore);
        assert_eq!(
            decide_voice_intake(Some("file_id"), None, false),
            VoiceIntake::Reject {
                key: "stt.not_configured"
            }
        );
        assert_eq!(
            decide_voice_intake(Some("file_id"), Some(MAX_VOICE_FILE_BYTES + 1), true),
            VoiceIntake::Reject {
                key: "bot.file_too_large"
            }
        );
        assert_eq!(
            decide_voice_intake(Some("file_id"), None, true),
            VoiceIntake::Accept
        );
        // Ignored/accepted intakes stay silent.
        assert_eq!(voice_intake_error_text("en", VoiceIntake::Ignore), None);
        assert_eq!(voice_intake_error_text("en", VoiceIntake::Accept), None);
        // Rejections localize with the 20 MiB cap filled in.
        let not_configured = voice_intake_error_text(
            "en",
            VoiceIntake::Reject {
                key: "stt.not_configured",
            },
        )
        .expect("rejection localizes");
        assert!(not_configured.contains("STT_API_URL"));
        let too_large = voice_intake_error_text(
            "en",
            VoiceIntake::Reject {
                key: "bot.file_too_large",
            },
        )
        .expect("rejection localizes");
        assert!(too_large.contains("20"));
        assert!(!too_large.contains("{maxSizeMb}"));
        // Transcript gate, error detail, and prompt ingest reuse.
        assert!(voice_empty_result_text("en").contains("No speech"));
        assert!(voice_error_text("en", "boom").contains("boom"));
        assert!(!voice_error_text("en", "boom").contains("{error}"));
        assert_eq!(build_voice_llm_text("", "hi"), "hi");
        assert!(is_voice_prompt_sendable("hi"));
        assert!(!is_voice_prompt_sendable("  "));
        // Read-only STT client pieces: URL build, bearer shape, format.
        assert_eq!(
            stt_transcriptions_url_for("https://stt.example"),
            "https://stt.example/audio/transcriptions"
        );
        assert_eq!(stt_bearer_header("key"), "Bearer key");
        // Env-dependent: clear the override, then restore it.
        let prev_format = std::env::var(STT_REQUEST_FORMAT_ENV).ok();
        std::env::remove_var(STT_REQUEST_FORMAT_ENV);
        assert_eq!(stt_request_format_from_env(), "multipart");
        match prev_format {
            Some(value) => std::env::set_var(STT_REQUEST_FORMAT_ENV, value),
            None => std::env::remove_var(STT_REQUEST_FORMAT_ENV),
        }
        assert!(!stt_model_from_env().is_empty());
        assert_eq!(STT_REQUEST_TIMEOUT_MS, 60_000);
    }
}
