//! Full-table port of `tests/i18n/placeholders.test.ts`,
//! `tests/i18n/callback-answer-length.test.ts`, and the lookup smoke surface
//! of `tests/i18n/index.test.ts`.
//!
//! Unlike `tests/i18n_parity.rs` (representative keys only), these tests run
//! over the generated full table in `otb_core::i18n`: every `I18nKey` in
//! `ALL_KEYS` must resolve in all 12 locales with the same `{placeholder}`
//! set as `en`, and every statically scanned key in `CALLBACK_ANSWER_KEYS`
//! must stay within the Telegram callback-answer budget.
//!
//! String-unit mapping: TS `text.length` counts UTF-16 code units, so budget
//! assertions use `text.encode_utf16().count()`.

use otb_core::i18n::{
    extract_placeholders, t, ALL_KEYS, CALLBACK_ANSWER_KEYS, CALLBACK_ANSWER_MAX_LENGTH,
    SUPPORTED_LOCALES,
};
use std::collections::BTreeSet;

fn placeholder_set(template: &str) -> BTreeSet<String> {
    extract_placeholders(template).into_iter().collect()
}

#[test]
fn generated_key_lists_cover_the_contract_surface() {
    assert!(ALL_KEYS.len() > 500, "full table looks truncated");
    // Legacy hand-picked keys must survive the codegen switch.
    for key in [
        "bot.session_created",
        "status.line.health",
        "progress.compact.count.tools",
        "common.cancelled",
        "agent.change_error_callback",
        "variant.change_error_callback",
        "permission.inactive_callback",
        "question.inactive_callback",
    ] {
        assert!(ALL_KEYS.contains(&key), "{key} must be in ALL_KEYS");
    }
    assert!(!CALLBACK_ANSWER_KEYS.is_empty());
    assert!(CALLBACK_ANSWER_KEYS.contains(&"common.cancelled"));
    for key in CALLBACK_ANSWER_KEYS {
        assert!(ALL_KEYS.contains(key), "{key} must be a known key");
    }
}

#[test]
fn every_key_resolves_in_every_locale() {
    let mut missing: Vec<String> = Vec::new();
    for key in ALL_KEYS {
        for locale in SUPPORTED_LOCALES {
            let text = t(locale, key);
            if text.is_empty() || text == *key {
                missing.push(format!("{key} ({locale})"));
            }
        }
    }
    assert!(missing.is_empty(), "unresolved keys: {missing:?}");
}

#[test]
fn placeholder_sets_match_en_for_every_key_in_every_locale() {
    let mut mismatches: Vec<String> = Vec::new();
    for key in ALL_KEYS {
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
    assert!(
        mismatches.is_empty(),
        "placeholder mismatches: {mismatches:?}"
    );
}

#[test]
fn scanned_callback_answers_stay_within_telegram_limit_in_every_locale() {
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

#[test]
fn unknown_key_echoes_and_unknown_locale_falls_back() {
    assert_eq!(t("en", "no.such.key"), "no.such.key");
    assert_eq!(t("xx", "common.cancelled"), t("en", "common.cancelled"));
}
