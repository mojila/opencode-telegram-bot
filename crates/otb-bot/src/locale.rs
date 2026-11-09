//! Locale parity (T9, structure only).
//!
//! ASSUMPTION: "13 locales" counts the 13 files under `src/i18n/` (12
//! locale dictionaries + `index.ts`). The locale code universe is the 12
//! codes of `SUPPORTED_LOCALES` in `src/i18n/index.ts`
//! (`en ar de es fr id it ko pt ru tr zh`), already codegenned in full by
//! `crates/otb-core/build.rs` into `otb_core::i18n`.
//!
//! This module is a thin structural wrapper: the supported list re-exports
//! the codegen order, `en` is the fallback, and lookup delegates to
//! `otb_core::i18n::t`. No Telegram calls here.

/// Supported locale codes, mirroring `SUPPORTED_LOCALES` order in
/// `src/i18n/index.ts` (12 codes; `index.ts` itself is the 13th file).
pub use otb_core::i18n::SUPPORTED_LOCALES;

/// Fallback locale (TS: `DEFAULT_LOCALE = "en"`).
pub const FALLBACK_LOCALE: &str = "en";

/// Number of supported locale codes.
pub const LOCALE_COUNT: usize = SUPPORTED_LOCALES.len();

/// Whether `code` is a supported locale (TS: `isSupportedLocale`).
pub fn is_supported_locale(code: &str) -> bool {
    otb_core::i18n::resolve_supported_locale(Some(code)).is_some()
}

/// Resolve a raw locale tag with `en` fallback (TS: `normalizeLocale`).
/// Pure; the caller's `BOT_LOCALE` arrives as `raw`.
pub fn resolve_locale(raw: Option<&str>) -> String {
    otb_core::i18n::normalize_locale(raw, FALLBACK_LOCALE)
}

/// Translate `key` for `locale` with `en` fallback, then the key itself
/// (TS: `t`). Pure lookup over the generated full table.
pub fn translate(locale: &str, key: &str) -> String {
    otb_core::i18n::t(locale, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_list_matches_ts_universe_with_en_fallback() {
        assert_eq!(LOCALE_COUNT, 12);
        assert_eq!(SUPPORTED_LOCALES[0], "en");
        assert!(SUPPORTED_LOCALES.contains(&"ru"));
        assert!(SUPPORTED_LOCALES.contains(&"zh"));
        assert!(is_supported_locale("ru-RU"));
        assert!(!is_supported_locale("xx"));
        assert_eq!(resolve_locale(None), "en");
        assert_eq!(resolve_locale(Some("  RU  ")), "ru");
        assert_eq!(resolve_locale(Some("xx")), "en");
        // Pure lookup: known key renders the `en` template; unknown locale
        // and unknown key fall back without any Telegram call.
        assert_eq!(translate("en", "status.line.health"), "Status: {health}");
        assert_eq!(translate("xx", "status.line.health"), "Status: {health}");
        assert_eq!(translate("en", "no.such.key"), "no.such.key");
    }
}
