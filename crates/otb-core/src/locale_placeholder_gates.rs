//! Locale placeholder-consistency gates (slice 122, RED).
//!
//! Rust port of the pure remainder in
//! `tests/i18n/placeholders.test.ts`:
//! - `PLACEHOLDER_PATTERN` (`/\{([a-zA-Z0-9_]+)\}/g`) shared with
//!   `src/i18n/index.ts` `interpolate`
//! - `extractPlaceholders` (`match` + `Set`, `(none)` rendering is
//!   test-only formatting and not ported)
//! - per-locale loop: for every `I18nKey` in `src/i18n/en.ts`, the
//!   placeholder set of `dictionary[key]` (`src/i18n/ar.ts`,
//!   `src/i18n/de.ts`, `src/i18n/es.ts`, `src/i18n/fr.ts`,
//!   `src/i18n/id.ts`, `src/i18n/it.ts`, `src/i18n/ko.ts`,
//!   `src/i18n/pt.ts`, `src/i18n/ru.ts`, `src/i18n/tr.ts`,
//!   `src/i18n/zh.ts`) must equal the `en` set
//!   (`expected.size !== actual.size || some missing` mismatch gate,
//!   `mismatches` collected per locale)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `find src -type f -name "*.ts"` lists 282 TS files; every `src/*.ts`
//!   path except the 10 locale tables is referenced from
//!   `crates/otb-core/{src,tests}` (`comm -23` gap is exactly
//!   `src/i18n/{ar,de,es,fr,id,it,ko,pt,tr,zh}.ts`; `en`/`ru` are covered
//!   by `crate::i18n` + `crate::i18n_full`).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only.
//! - `tests/i18n/placeholders.test.ts` (11 locales x every `I18nKey`) has
//!   no Rust owner. This slice owns only its pure predicate core; full
//!   12-locale x 654-key table codegen stays future work.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback (`crate::i18n` owner)
//! - `LOCALE_DEFINITIONS` labels, `getLocaleOptions`, `getLocale`
//!   precedence (`locale_registry_gates` owner, slice 121)
//! - en/ru representative tables + interpolation
//!   (`crate::i18n_full` owner)
//! - `extract_placeholders` sorted-`Vec` helper (`crate::i18n` owner;
//!   this slice returns `Set`-ordered `Vec`s for the mismatch report)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - locale dictionary file reads (`src/i18n/*.ts` arrive as explicit
//!   `en_template` / `locale_template` args), per-locale test looping,
//!   `mismatches` aggregation.
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; empty `{}` and
//! `{with-dash}` never match in either language.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice122_locale_placeholder_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locales whose placeholder parity has no Rust owner yet
/// (`en` is the reference, `ru` is covered via `crate::i18n_full`).
pub const REMAINING_LOCALES: [&str; 10] = [
    "ar", "de", "es", "fr", "id", "it", "ko", "pt", "tr", "zh",
];

/// Source of the shared placeholder pattern
/// (TS: `/\{([a-zA-Z0-9_]+)\}/g` in `tests/i18n/placeholders.test.ts`
/// and `src/i18n/index.ts`).
pub const PLACEHOLDER_PATTERN_SOURCE: &str = r"\{([a-zA-Z0-9_]+)\}";

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

fn format_placeholder_set(set: &std::collections::BTreeSet<String>) -> String {
    if set.is_empty() {
        "(none)".to_string()
    } else {
        set.iter().cloned().collect::<Vec<_>>().join(", ")
    }
}

/// `true` when both templates carry the same placeholder set
/// (TS: `expected.size === actual.size && every(p => actual.has(p))`).
pub fn placeholder_sets_equal(en_template: &str, locale_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(locale_template)
}

/// Placeholders in `en_template` missing from `locale_template`, sorted
/// (TS: `[...expected].filter((p) => !actual.has(p))` subset of the
/// mismatch gate).
pub fn missing_placeholders(
    en_template: &str,
    locale_template: &str,
) -> Vec<String> {
    let expected = placeholder_set(en_template);
    let actual = placeholder_set(locale_template);
    expected.difference(&actual).cloned().collect()
}

/// Placeholders in `locale_template` absent from `en_template`, sorted
/// (TS: the size/equality gate catches these together with missing ones).
pub fn extra_placeholders(
    en_template: &str,
    locale_template: &str,
) -> Vec<String> {
    let expected = placeholder_set(en_template);
    let actual = placeholder_set(locale_template);
    actual.difference(&expected).cloned().collect()
}

/// `true` when the translation keeps the `en` placeholder set exactly
/// (TS: zero `mismatches` for the key).
pub fn is_translation_placeholder_ok(
    en_template: &str,
    locale_template: &str,
) -> bool {
    placeholder_sets_equal(en_template, locale_template)
}

/// One mismatch line (TS: `` `${key}: en has ${expected}, ${code} has
/// ${actual}` `` with `sort().join(", ") || "(none)"` formatting).
pub fn format_placeholder_mismatch(
    key: &str,
    en_template: &str,
    locale_template: &str,
    locale: &str,
) -> String {
    let expected = placeholder_set(en_template);
    let actual = placeholder_set(locale_template);
    format!(
        "{}: en has {}, {} has {}",
        key,
        format_placeholder_set(&expected),
        locale,
        format_placeholder_set(&actual)
    )
}
