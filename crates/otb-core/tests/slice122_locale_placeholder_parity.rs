//! Slice 122 parity: locale placeholder-consistency gates (RED).
//!
//! Mirrors the pure TS contract in `tests/i18n/placeholders.test.ts`:
//! - `PLACEHOLDER_PATTERN` (`/\{([a-zA-Z0-9_]+)\}/g`) shared with
//!   `src/i18n/index.ts` `interpolate`
//! - per-locale loop over every `I18nKey` in `src/i18n/en.ts`: the
//!   placeholder set of each remaining locale dictionary
//!   (`src/i18n/{ar,de,es,fr,id,it,ko,pt,tr,zh}.ts`) must equal `en`
//!   (`expected.size !== actual.size || some missing` mismatch gate)
//!
//! Already covered elsewhere and NOT re-tested here:
//! `extract_placeholders` sorted-`Vec` helper (`crate::i18n` owner),
//! `SUPPORTED_LOCALES` / registry / `getLocale` precedence
//! (`locale_registry_gates`, slice 121), en/ru tables + interpolation
//! (`crate::i18n_full`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_placeholder_gates` is ported (only the constant-value tests pass).

use otb_core::locale_placeholder_gates::*;

// ---- constants (2) ----

#[test]
fn remaining_locales_value_parity() {
    assert_eq!(REMAINING_LOCALES.len(), 10);
    assert_eq!(
        REMAINING_LOCALES,
        ["ar", "de", "es", "fr", "id", "it", "ko", "pt", "tr", "zh"]
    );
}

#[test]
fn placeholder_pattern_source_value_parity() {
    assert_eq!(PLACEHOLDER_PATTERN_SOURCE, r"\{([a-zA-Z0-9_]+)\}");
}

// ---- set equality (3) ----

#[test]
fn placeholder_sets_equal_parity() {
    assert!(placeholder_sets_equal(
        "Status: {health}",
        "Status: {health}"
    ));
    assert!(placeholder_sets_equal(
        "no placeholders",
        "ohne Platzhalter"
    ));
}

#[test]
fn placeholder_sets_reject_missing_parity() {
    assert!(!placeholder_sets_equal("Status: {health}", "Status: ok"));
    assert!(!placeholder_sets_equal(
        "Quote {index}:\n{question}",
        "Quote {index}:"
    ));
}

#[test]
fn placeholder_sets_reject_extra_parity() {
    assert!(!placeholder_sets_equal("Done", "Done {extra}"));
    assert!(!placeholder_sets_equal("Hi {name}", "Hi {name} {surname}"));
}

// ---- missing / extra (3) ----

#[test]
fn missing_placeholders_parity() {
    assert_eq!(
        missing_placeholders("Status: {health}", "Status: ok"),
        vec!["{health}".to_string()]
    );
    assert!(missing_placeholders("Status: {health}", "Status: {health}").is_empty());
}

#[test]
fn extra_placeholders_parity() {
    assert_eq!(
        extra_placeholders("Done", "Done {extra}"),
        vec!["{extra}".to_string()]
    );
    assert!(extra_placeholders("Hi {name}", "Hi {name}").is_empty());
}

#[test]
fn non_identifier_braces_are_not_placeholders_parity() {
    assert!(placeholder_sets_equal(
        "empty {} braces",
        "leere {} Klammern"
    ));
    assert!(placeholder_sets_equal(
        "dash {with-dash}",
        "Bindestrich {with-dash}"
    ));
    assert!(!placeholder_sets_equal("Hello {name}", "Hallo {Name}"));
}

// ---- translation verdict (2) ----

#[test]
fn translation_ok_parity() {
    assert!(is_translation_placeholder_ok(
        "Question {index}:\n{question}",
        "Frage {index}:\n{question}"
    ));
    assert!(!is_translation_placeholder_ok(
        "Question {index}:\n{question}",
        "Frage {index}:"
    ));
}

#[test]
fn mismatch_line_names_key_and_sets_parity() {
    let line =
        format_placeholder_mismatch("status.line.health", "Status: {health}", "Status: ok", "de");
    assert!(
        line.contains("status.line.health"),
        "line must name the key: {line}"
    );
    assert!(
        line.contains("{health}"),
        "line must show the en set: {line}"
    );
    assert!(line.contains("de"), "line must name the locale: {line}");
}
