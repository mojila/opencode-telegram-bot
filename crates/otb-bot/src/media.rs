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
    match file_size {
        None | Some(0) => true,
        Some(size) => size <= MAX_VOICE_FILE_BYTES,
    }
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
}
