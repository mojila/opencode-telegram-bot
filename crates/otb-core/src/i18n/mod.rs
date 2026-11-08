//! i18n layer: locale lookup, placeholder interpolation, callback-answer budget.
//!
//! Mirrors `src/i18n/index.ts` (plus the `src/i18n/*.ts` dictionary shape)
//! from the TypeScript source.
//!
//! The template table is generated at build time by
//! `crates/otb-core/build.rs`, which parses every `src/i18n/<locale>.ts`
//! dictionary and emits `ALL_KEYS`, `CALLBACK_ANSWER_KEYS`, and
//! `generated_lookup`. `src/i18n/en.ts` defines the key universe (`I18nKey`);
//! the build fails when any other locale drifts from it. Callback-answer
//! keys come from the same static scan the TS
//! `tests/i18n/callback-answer-length.test.ts` performs (`notify`/`alert`/
//! `failure`/`cancelPrompt` literals and direct
//! `answerCallbackQuery({ text: t("...") })` in `src/bot`).

/// Must stay in sync with `CALLBACK_ANSWER_MAX_LENGTH` in
/// `src/bot/callbacks/feedback.ts` (see
/// `tests/i18n/callback-answer-length.test.ts`).
pub const CALLBACK_ANSWER_MAX_LENGTH: usize = 200;

/// All supported locale codes, mirroring `SUPPORTED_LOCALES` derived from
/// `LOCALE_DEFINITIONS` in `src/i18n/index.ts`.
pub const SUPPORTED_LOCALES: [&str; 12] = [
    "en", "ar", "de", "es", "fr", "id", "it", "ko", "pt", "ru", "tr", "zh",
];

/// `dateLocale` per locale code, mirroring `LOCALE_DEFINITIONS` in
/// `src/i18n/index.ts`.
fn date_locale_for(locale: &str) -> Option<&'static str> {
    match locale {
        "en" => Some("en-US"),
        "ar" => Some("ar-SA"),
        "de" => Some("de-DE"),
        "es" => Some("es-ES"),
        "fr" => Some("fr-FR"),
        "id" => Some("id-ID"),
        "it" => Some("it-IT"),
        "ko" => Some("ko-KR"),
        "pt" => Some("pt-BR"),
        "ru" => Some("ru-RU"),
        "tr" => Some("tr-TR"),
        "zh" => Some("zh-CN"),
        _ => None,
    }
}

/// Full template table + key lists, generated at build time from
/// `src/i18n/*.ts` (see `crates/otb-core/build.rs`). `ALL_KEYS` is the `en`
/// key universe (`I18nKey`); `CALLBACK_ANSWER_KEYS` is the static scan of
/// callback-answer keys in `src/bot`.
include!(concat!(env!("OUT_DIR"), "/i18n_generated.rs"));

/// Translation lookup over the generated full table.
/// Strings are the verbatim TS locale templates, so `{placeholder}` sets
/// match `en` exactly where the TS `tests/i18n/placeholders.test.ts` says so.
fn lookup(locale: &str, key: &str) -> Option<&'static str> {
    generated_lookup(locale, key)
}

fn is_placeholder_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Translate `key` for `locale`, interpolating `{placeholders}`.
/// Falls back to `en`, then to the key itself when missing.
/// Mirrors `t()` in `src/i18n/index.ts`.
///
/// Note: unlike the TS signature `t(key, params?, locale?)`, this port
/// takes `(locale, key)` and returns the raw template without
/// interpolation (no `params` argument); callers keep `{placeholders}`
/// intact, which is what the parity tests assert.
pub fn t(locale: &str, key: &str) -> String {
    // Unknown locale falls back to `en`, mirroring `normalizeLocale()`
    // falling back to `DEFAULT_LOCALE`.
    let resolved = resolve_supported_locale(Some(locale));
    let active = resolved.as_deref().unwrap_or("en");
    if let Some(template) = lookup(active, key) {
        return template.to_string();
    }
    if active != "en" {
        if let Some(template) = lookup("en", key) {
            return template.to_string();
        }
    }
    key.to_string()
}

/// Extract the `{placeholder}` tokens of a template, mirroring the
/// `/\{([a-zA-Z0-9_]+)\}/g` pattern shared by `src/i18n/index.ts` and
/// `tests/i18n/placeholders.test.ts`. Returns full tokens (e.g. `"{title}"`),
/// sorted, so placeholder sets can be compared across locales.
pub fn extract_placeholders(template: &str) -> Vec<String> {
    let bytes = template.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && is_placeholder_char(bytes[j]) {
                j += 1;
            }
            // Non-empty inner run closed by `}`.
            if j > i + 1 && j < bytes.len() && bytes[j] == b'}' {
                out.push(template[i..=j].to_string());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out.sort();
    out
}

/// Resolve a BCP-47-ish tag to a supported locale (`"ru-RU"` -> `"ru"`),
/// or `None` when unsupported. Mirrors `resolveSupportedLocale()`.
pub fn resolve_supported_locale(locale: Option<&str>) -> Option<String> {
    let normalized = locale?.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if SUPPORTED_LOCALES.contains(&normalized.as_str()) {
        return Some(normalized);
    }
    let base = normalized.split('-').next().unwrap_or("");
    if !base.is_empty() && SUPPORTED_LOCALES.contains(&base) {
        return Some(base.to_string());
    }
    None
}

/// Normalize a locale tag with fallback. Mirrors `normalizeLocale()`.
pub fn normalize_locale(locale: Option<&str>, fallback: &str) -> String {
    resolve_supported_locale(locale).unwrap_or_else(|| fallback.to_string())
}

/// Date locale (e.g. `"ru-RU"`) for a supported locale code.
/// Mirrors `getDateLocale()`. Unknown codes fall back to `"en-US"`,
/// mirroring the `DEFAULT_LOCALE` fallback in the TS layer.
pub fn get_date_locale(locale: &str) -> String {
    date_locale_for(locale).unwrap_or("en-US").to_string()
}
