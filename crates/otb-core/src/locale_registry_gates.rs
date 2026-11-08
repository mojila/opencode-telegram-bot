//! Locale registry + runtime-locale override gates (slice 121, RED).
//!
//! Rust port of the unported remainder in `src/i18n/index.ts`:
//! - `LOCALE_DEFINITIONS` labels (`English`, `العربية`, `Deutsch`,
//!   `Español`, `Français`, `Bahasa Indonesia`, `Italiano`, `한국어`,
//!   `Português (Brasil)`, `Русский`, `Türkçe`, `简体中文`) in
//!   `SUPPORTED_LOCALES` order
//! - `getLocaleOptions` (`{ code, label }` pairs from the single registry)
//! - `getLocale` precedence (runtime override wins over `BOT_LOCALE` env)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `process.env.BOT_LOCALE` reads (arrive as explicit `env` args),
//!   module-level `runtimeLocaleOverride` cell (`setRuntimeLocale` /
//!   `resetRuntimeLocale`), `getLocale()` env wiring.
//!
//! Already ported elsewhere and NOT repeated here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback, placeholder extraction
//!   (`crate::i18n` owner); en/ru representative tables + interpolation
//!   (`crate::i18n_full` owner). Callers normalize `env` via
//!   `crate::i18n::normalize_locale` before asking for the effective code.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice121_locale_registry_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Default locale (TS: `DEFAULT_LOCALE` in `src/i18n/index.ts`).
pub const DEFAULT_LOCALE: &str = "en";

/// Registry table in `SUPPORTED_LOCALES` order: `(code, label)` pairs copied
/// verbatim from `LOCALE_DEFINITIONS` in `src/i18n/index.ts`.
pub const LOCALE_OPTIONS: [(&str, &str); 12] = [
    ("en", "English"),
    ("ar", "العربية"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("fr", "Français"),
    ("id", "Bahasa Indonesia"),
    ("it", "Italiano"),
    ("ko", "한국어"),
    ("pt", "Português (Brasil)"),
    ("ru", "Русский"),
    ("tr", "Türkçe"),
    ("zh", "简体中文"),
];

/// Locale option (TS: `LocaleOption` — `{ code, label }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocaleOption {
    pub code: &'static str,
    pub label: &'static str,
}

/// Label for a locale code from the registry (`LOCALE_DEFINITIONS` lookup);
/// `None` for unknown codes.
pub fn get_locale_label(code: &str) -> Option<&'static str> {
    LOCALE_OPTIONS
        .iter()
        .find(|(entry_code, _)| *entry_code == code)
        .map(|(_, label)| *label)
}

/// All locale options from the single registry in `SUPPORTED_LOCALES` order
/// (TS: `getLocaleOptions`).
pub fn get_locale_options() -> Vec<LocaleOption> {
    LOCALE_OPTIONS
        .iter()
        .map(|(code, label)| LocaleOption { code, label })
        .collect()
}

/// Whether a `(code, label)` pair matches the registry entry exactly
/// (TS: option identity behind `getLocaleOptions().map((o) => o.code)`).
pub fn is_valid_locale_option(code: &str, label: &str) -> bool {
    get_locale_label(code) == Some(label)
}

/// Effective locale code: the runtime override (already a valid `Locale`
/// from `setRuntimeLocale`) wins as-is, otherwise the caller's
/// already-normalized env/default code (TS: `getLocale` precedence —
/// `runtimeLocaleOverride ?? normalizeLocale(BOT_LOCALE)`).
pub fn resolve_effective_locale_code(
    override_locale: Option<&str>,
    normalized_env: &str,
) -> String {
    match override_locale {
        Some(active) => active.to_string(),
        None => normalized_env.to_string(),
    }
}
