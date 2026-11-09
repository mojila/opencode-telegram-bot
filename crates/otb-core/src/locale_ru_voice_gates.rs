//! Russian voice/STT gates (slice 175, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/ru.ts` for the voice/media
//! intake surface NOT covered by `STATUS_KEYS` / `QUEUE_KEYS` / `GOAL_KEYS` /
//! `ERROR_KEYS` in `crates/otb-core/src/i18n_full.rs`:
//! - `stt.recognizing` (`🎤 Распознаю аудио...`)
//! - `stt.recognized` (`🎤 Распознано:`)
//! - `stt.not_configured` (multiline `.env` hint)
//! - `stt.error` (`🔴 Не удалось распознать аудио: {error}`)
//! - `stt.empty_result` (`🎤 В аудиосообщении не обнаружена речь.`)
//!
//! AUDIT (2026-11-09, branch `rust-rewrite`):
//! - `src/i18n/ru.ts` lines 697-702 carry the five `stt.*` templates;
//!   `src/i18n/en.ts` lines 736-741 are the placeholder reference (`en`
//!   keeps `🎤 Recognizing audio...`, `🎤 Recognized:`, the multiline
//!   `stt.not_configured` hint, `🔴 Failed to recognize audio: {error}`,
//!   `🎤 No speech detected in the audio message.`).
//! - `crate::i18n` resolves every key through the build-time full table
//!   (`crates/otb-core/build.rs` parses `src/i18n/*.ts`), so `t("ru", key)`
//!   already returns these templates; `crate::i18n_full` embeds 72
//!   representative en/ru templates but no `stt.*` key. No Rust owner pins
//!   the `ru` STT verbatim templates yet.
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `voice_media_intake_gates` owns only the locale-agnostic
//!   intake verdicts (`decide_voice_intake` returns the key names, the
//!   caller localizes). `tts_response_gates` (slice 111) and
//!   `edge_tts_gates` (slice 73) own only the TTS reply gates.
//! - This slice owns only the `ru` STT lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen already
//!   lives in `crate::i18n` (build-time).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `ru.ts` file reads (arrive as explicit `key` args here),
//!   per-locale test looping, `t()` fallback wiring.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback (`crate::i18n` owner)
//! - `FULL_KEYS` + en/ru verbatim tables + interpolation
//!   (`crate::i18n_full` owner)
//! - placeholder set-equality / missing / extra helpers
//!   (`locale_placeholder_gates` owner, slice 122)
//! - registry labels / `getLocaleOptions` / `getLocale` precedence
//!   (`locale_registry_gates` owner, slice 121)
//! - voice size caps + transcribe dispatch verdicts + prompt ingest reuse
//!   (`voice_media_intake_gates` owner)
//! - TTS reply keys and failure path (`tts_response_gates` owner,
//!   slice 111; `edge_tts_gates` owner, slice 73).
//!
//! Deferred (NOT in this slice):
//! - `tts.*` templates (TTS reply surface, same owners as above)
//! - `bot.photo_*` / `bot.file_*` / `bot.media_group_*` download notices
//!   (media-download surface, caller localizes via `crate::i18n`)
//! - `help.*` / `cmd.description.*` static labels (command registry
//!   surface in `otb-bot`, structure-only).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice175_locale_ru_voice_parity.rs`.

/// Locale code for this table (TS: `ru` in `SUPPORTED_LOCALES`).
pub const RU_VOICE_LOCALE: &str = "ru";

/// Voice/STT keys covered here (the `stt.*` keys in `src/i18n/ru.ts`
/// lines 697-702, outside the 72 `FULL_KEYS` in `crate::i18n_full`).
pub const RU_VOICE_KEYS: [&str; 5] = [
    "stt.recognizing",
    "stt.recognized",
    "stt.not_configured",
    "stt.error",
    "stt.empty_result",
];

/// Verbatim `ru` STT templates (TS: `src/i18n/ru.ts` lines 697-702).
pub const RU_STT_RECOGNIZING: &str = "🎤 Распознаю аудио...";
pub const RU_STT_RECOGNIZED: &str = "🎤 Распознано:";
pub const RU_STT_NOT_CONFIGURED: &str = "🎤 Распознавание голоса не настроено.\n\nУстановите STT_API_URL и STT_API_KEY в .env для включения.";
pub const RU_STT_ERROR: &str = "🔴 Не удалось распознать аудио: {error}";
pub const RU_STT_EMPTY_RESULT: &str = "🎤 В аудиосообщении не обнаружена речь.";

/// Verbatim `ru` voice/STT template for a key, or `None` outside this
/// slice (TS: `ru[key]` for the five `stt.*` keys above).
pub fn lookup_ru_voice_template(key: &str) -> Option<&'static str> {
    match key {
        "stt.recognizing" => Some(RU_STT_RECOGNIZING),
        "stt.recognized" => Some(RU_STT_RECOGNIZED),
        "stt.not_configured" => Some(RU_STT_NOT_CONFIGURED),
        "stt.error" => Some(RU_STT_ERROR),
        "stt.empty_result" => Some(RU_STT_EMPTY_RESULT),
        _ => None,
    }
}

fn is_placeholder_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn placeholder_set(template: &str) -> std::collections::BTreeSet<String> {
    let bytes = template.as_bytes();
    let mut out = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && is_placeholder_char(bytes[j]) {
                j += 1;
            }
            if j > i + 1 && j < bytes.len() && bytes[j] == b'}' {
                out.insert(template[i..=j].to_string());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn interpolate_ru(template: &str, params: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < template.len() {
        let rest = &template[i..];
        if rest.starts_with('{') {
            if let Some(end) = rest.find('}') {
                let name = &rest[1..end];
                if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    match params.iter().find(|(k, _)| *k == name) {
                        Some((_, value)) => out.push_str(value),
                        None => out.push_str(&rest[..=end]),
                    }
                    i += end + 1;
                    continue;
                }
            }
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `true` when the `ru` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_ru_voice_placeholder_ok(en_template: &str, ru_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(ru_template)
}

/// `🔴 Не удалось распознать аудио: {error}` with the recognizer error
/// filled in (TS: `t("stt.error", { error })` call site in
/// `src/bot/handlers/voice-handler.ts`).
pub fn format_ru_stt_error(error: &str) -> String {
    interpolate_ru(RU_STT_ERROR, &[("error", error)])
}
