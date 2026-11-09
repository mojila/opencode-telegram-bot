//! Slice 175 parity: Russian voice/STT gates.
//!
//! Mirrors the pure TS contract in `src/i18n/ru.ts` (lines 697-702) for the
//! five `stt.*` keys outside `FULL_KEYS` in `crate::i18n_full`:
//! - verbatim `ru` templates for the STT replies
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the voice-handler call sites)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! voice size caps + transcribe dispatch verdicts (`voice_media_intake_gates`),
//! TTS reply gates (`tts_response_gates`, slice 111; `edge_tts_gates`,
//! slice 73).

use otb_core::locale_ru_voice_gates::*;

// ---- constants (3) ----

#[test]
fn ru_voice_locale_value_parity() {
    assert_eq!(RU_VOICE_LOCALE, "ru");
}

#[test]
fn ru_voice_keys_value_parity() {
    assert_eq!(RU_VOICE_KEYS.len(), 5);
    assert!(RU_VOICE_KEYS.contains(&"stt.recognizing"));
    assert!(RU_VOICE_KEYS.contains(&"stt.not_configured"));
    assert!(RU_VOICE_KEYS.contains(&"stt.error"));
    assert!(RU_VOICE_KEYS.contains(&"stt.empty_result"));
}

#[test]
fn ru_voice_templates_value_parity() {
    assert_eq!(RU_STT_RECOGNIZING, "🎤 Распознаю аудио...");
    assert_eq!(RU_STT_RECOGNIZED, "🎤 Распознано:");
    assert_eq!(
        RU_STT_NOT_CONFIGURED,
        "🎤 Распознавание голоса не настроено.\n\nУстановите STT_API_URL и STT_API_KEY в .env для включения."
    );
    assert_eq!(RU_STT_ERROR, "🔴 Не удалось распознать аудио: {error}");
    assert_eq!(
        RU_STT_EMPTY_RESULT,
        "🎤 В аудиосообщении не обнаружена речь."
    );
}

// ---- lookup (2) ----

#[test]
fn ru_voice_lookup_error_parity() {
    assert_eq!(
        lookup_ru_voice_template("stt.error"),
        Some("🔴 Не удалось распознать аудио: {error}")
    );
}

#[test]
fn ru_voice_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ru_voice_template("stt.unknown_key"), None);
    assert_eq!(lookup_ru_voice_template("pinned.line.goal"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ru_voice_placeholders_match_en_parity() {
    assert!(is_ru_voice_placeholder_ok(
        "🔴 Failed to recognize audio: {error}",
        "🔴 Не удалось распознать аудио: {error}"
    ));
    assert!(is_ru_voice_placeholder_ok(
        "🎤 Recognizing audio...",
        "🎤 Распознаю аудио..."
    ));
}

#[test]
fn ru_voice_placeholders_reject_mismatch_parity() {
    assert!(!is_ru_voice_placeholder_ok(
        "🔴 Failed to recognize audio: {error}",
        "🔴 Не удалось распознать аудио"
    ));
    assert!(!is_ru_voice_placeholder_ok(
        "🎤 Recognizing audio...",
        "🎤 Распознаю аудио... {extra}"
    ));
}

// ---- formatting (1) ----

#[test]
fn ru_voice_stt_error_line_parity() {
    assert_eq!(
        format_ru_stt_error("timeout"),
        "🔴 Не удалось распознать аудио: timeout"
    );
}
