//! Parity port of `tests/i18n/placeholders.test.ts`,
//! `tests/i18n/callback-answer-length.test.ts`, and the lookup smoke surface
//! of `tests/i18n/index.test.ts`.
//!
//! Step 3 (RED): these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (GREEN).
//!
//! String-unit mapping: TS `text.length` counts UTF-16 code units. Budget
//! assertions therefore use `text.encode_utf16().count()`, which matches TS
//! semantics exactly (unlike the chunker parity tests, where the budget unit
//! was deliberately remapped to `chars().count()`).
//!
//! SCOPE (RED): embedding the full 12-locale translation tables is out of
//! scope. The key lists below name just enough keys to express each contract:
//! placeholder-bearing keys plus one placeholder-free key for (a), and
//! representative `notify`/`alert`/`failure`/`cancelPrompt` callback-answer
//! keys for (b). GREEN fills the real tables behind `otb_core::i18n::t()` and
//! expands coverage toward the full `I18nKey` set (ideally by statically
//! scanning callback-answer keys as the TS test does, instead of this
//! hand-picked list).

use otb_core::i18n::{
    CALLBACK_ANSWER_MAX_LENGTH, SUPPORTED_LOCALES, extract_placeholders, get_date_locale,
    normalize_locale, resolve_supported_locale, t,
};
use std::collections::BTreeSet;

// Representative keys only (see SCOPE above). Each placeholder-bearing key
// must keep the same `{placeholder}` set in every locale; the last key
// pins the placeholder-free case.
const REPRESENTATIVE_KEYS: &[&str] = &[
    "bot.session_created", // {title}
    "status.line.health",  // {health}
    "progress.compact.count.tools", // {count}
    "common.cancelled",    // no placeholders
];

// Representative callback-answer keys only (see SCOPE above): literals passed
// to `notify`/`alert`/`failure`/`cancelPrompt` or direct
// `answerCallbackQuery({ text: t("key") })` in `src/bot/`.
const CALLBACK_ANSWER_KEYS: &[&str] = &[
    "common.cancelled",
    "agent.change_error_callback",
    "variant.change_error_callback",
    "question.inactive_callback",
    "permission.inactive_callback",
];

fn placeholder_set(template: &str) -> BTreeSet<String> {
    extract_placeholders(template).into_iter().collect()
}

// --- (a) placeholder-set equality per key across all 12 locales ---

#[test]
fn placeholder_sets_match_en_for_every_key_in_every_locale() {
    let mut mismatches: Vec<String> = Vec::new();

    for key in REPRESENTATIVE_KEYS {
        let expected = placeholder_set(&t("en", key));
        for locale in SUPPORTED_LOCALES {
            let actual = placeholder_set(&t(locale, key));
            if expected != actual {
                mismatches.push(format!(
                    "{key} ({locale}): en has {expected:?}, {locale} has {actual:?}"
                ));
            }
        }
    }

    assert!(mismatches.is_empty(), "placeholder mismatches: {mismatches:?}");
}

// --- (b) callback-answer length <= 200 in every locale ---

#[test]
fn callback_answer_keys_are_known() {
    assert!(
        !CALLBACK_ANSWER_KEYS.is_empty(),
        "expected at least one callback-answer key"
    );
    assert!(CALLBACK_ANSWER_KEYS.contains(&"common.cancelled"));
}

#[test]
fn callback_answers_stay_within_telegram_limit_in_every_locale() {
    let mut violations: Vec<String> = Vec::new();

    for key in CALLBACK_ANSWER_KEYS {
        for locale in SUPPORTED_LOCALES {
            let text = t(locale, key);
            let len = text.encode_utf16().count();
            if len > CALLBACK_ANSWER_MAX_LENGTH {
                violations.push(format!(
                    "{key} ({locale}): {len} chars > {CALLBACK_ANSWER_MAX_LENGTH}"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "callback answers exceeding the Telegram limit: {violations:?}"
    );
}

// --- (c) basic t() lookup + locale-helper smoke (from index.test.ts) ---

#[test]
fn t_resolves_known_keys_per_locale() {
    assert!(!t("en", "common.cancelled").is_empty());
    assert!(!t("ru", "bot.session_created").is_empty());
}

#[test]
fn resolves_exact_and_regional_locale_values() {
    assert_eq!(resolve_supported_locale(Some("ru")), Some("ru".to_string()));
    assert_eq!(
        resolve_supported_locale(Some("ru-RU")),
        Some("ru".to_string())
    );
    assert_eq!(
        resolve_supported_locale(Some("en-US")),
        Some("en".to_string())
    );
    assert_eq!(
        resolve_supported_locale(Some("pt-BR")),
        Some("pt".to_string())
    );
    assert_eq!(resolve_supported_locale(Some("xx")), None);
}

#[test]
fn normalizes_unsupported_locale_values_with_fallback() {
    assert_eq!(normalize_locale(Some("xx"), "en"), "en".to_string());
    assert_eq!(normalize_locale(None, "ru"), "ru".to_string());
}

#[test]
fn returns_date_locale_from_locale_definition() {
    assert_eq!(get_date_locale("ru"), "ru-RU".to_string());
    assert_eq!(get_date_locale("en"), "en-US".to_string());
    assert_eq!(get_date_locale("de"), "de-DE".to_string());
    assert_eq!(get_date_locale("fr"), "fr-FR".to_string());
}
