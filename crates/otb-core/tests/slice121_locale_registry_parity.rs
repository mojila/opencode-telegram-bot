//! Slice 121 parity: locale registry + runtime-override gates (RED).
//!
//! Mirrors the unported remainder of `src/i18n/index.ts`:
//! - `LOCALE_DEFINITIONS` labels in `SUPPORTED_LOCALES` order (12 locales)
//! - `getLocaleOptions` (`{ code, label }` pairs from the single registry)
//! - `getLocale` precedence (runtime override wins over `BOT_LOCALE` env)
//! - `tests/i18n/index.test.ts`: option codes equal `SUPPORTED_LOCALES`,
//!   runtime override preferred over env locale
//!
//! Already covered elsewhere and NOT re-tested here:
//! `resolveSupportedLocale` / `normalizeLocale` / `getDateLocale` / `t`
//! fallback / placeholder extraction (`crate::i18n` owner), en/ru
//! representative tables + interpolation (`crate::i18n_full` owner).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_registry_gates` is ported (only the constant-value tests pass).

use otb_core::locale_registry_gates::*;

// ---- constants (3) ----

#[test]
fn default_locale_value_parity() {
    assert_eq!(DEFAULT_LOCALE, "en");
}

#[test]
fn locale_options_table_value_parity() {
    assert_eq!(LOCALE_OPTIONS.len(), 12);
    assert_eq!(LOCALE_OPTIONS[0], ("en", "English"));
    assert_eq!(LOCALE_OPTIONS[9], ("ru", "Русский"));
    assert_eq!(LOCALE_OPTIONS[11], ("zh", "简体中文"));
}

#[test]
fn locale_labels_value_parity() {
    let labels: Vec<&str> = LOCALE_OPTIONS.iter().map(|(_, label)| *label).collect();
    assert_eq!(
        labels,
        vec![
            "English",
            "العربية",
            "Deutsch",
            "Español",
            "Français",
            "Bahasa Indonesia",
            "Italiano",
            "한국어",
            "Português (Brasil)",
            "Русский",
            "Türkçe",
            "简体中文",
        ]
    );
}

// ---- label lookup (2) ----

#[test]
fn locale_label_lookup_parity() {
    assert_eq!(get_locale_label("en"), Some("English"));
    assert_eq!(get_locale_label("ru"), Some("Русский"));
    assert_eq!(get_locale_label("pt"), Some("Português (Brasil)"));
}

#[test]
fn locale_label_rejects_unknown_parity() {
    assert_eq!(get_locale_label("xx"), None);
    assert_eq!(get_locale_label(""), None);
}

// ---- options (2) ----

#[test]
fn locale_options_order_parity() {
    let options = get_locale_options();
    let codes: Vec<&str> = options.iter().map(|option| option.code).collect();
    assert_eq!(
        codes,
        vec!["en", "ar", "de", "es", "fr", "id", "it", "ko", "pt", "ru", "tr", "zh"]
    );
}

#[test]
fn locale_options_labels_match_registry_parity() {
    let options = get_locale_options();
    assert_eq!(options.len(), 12);
    assert!(options
        .iter()
        .all(|option| get_locale_label(option.code) == Some(option.label)));
}

// ---- option identity (2) ----

#[test]
fn valid_locale_option_parity() {
    assert!(is_valid_locale_option("ru", "Русский"));
    assert!(is_valid_locale_option("en", "English"));
}

#[test]
fn invalid_locale_option_parity() {
    assert!(!is_valid_locale_option("ru", "English"));
    assert!(!is_valid_locale_option("xx", "Unknown"));
    assert!(!is_valid_locale_option("en", ""));
}

// ---- effective locale precedence (2) ----

#[test]
fn effective_locale_prefers_override_parity() {
    assert_eq!(
        resolve_effective_locale_code(Some("ru"), "en"),
        "ru".to_string()
    );
    assert_eq!(
        resolve_effective_locale_code(Some("zh"), "en"),
        "zh".to_string()
    );
}

#[test]
fn effective_locale_falls_back_to_env_parity() {
    assert_eq!(resolve_effective_locale_code(None, "en"), "en".to_string());
    assert_eq!(resolve_effective_locale_code(None, "ru"), "ru".to_string());
}
