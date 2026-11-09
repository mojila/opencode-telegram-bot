//! Build-time i18n codegen.
//!
//! The single source of truth for bot strings is `src/i18n/*.ts`
//! (`I18nDictionary` per locale, keyed by `I18nKey` from `en.ts`).
//! This script parses those dictionaries (no TS toolchain needed: the
//! tables are plain quoted-string literals) and emits
//! `i18n_generated.rs` with:
//! - `ALL_KEYS`: every `I18nKey`, sorted (drives full-table parity tests).
//! - `CALLBACK_ANSWER_KEYS`: literals passed to `notify`/`alert`/`failure`/
//!   `cancelPrompt` or direct `answerCallbackQuery({ text: t("...") })` in
//!   `src/bot`, found with the same static scan the TS
//!   `tests/i18n/callback-answer-length.test.ts` performs (scanned keys
//!   unknown to `en` are dropped; the TS test still reports those as
//!   violations).
//! - `generated_lookup(locale, key)`: verbatim template table for all 12
//!   locales, used by `crate::i18n::t`.
//!
//! `en` defines the key universe. The build panics when any other locale
//! drifts from it, enforcing the `Record<I18nKey, string>` contract at
//! codegen time instead of letting a missing key hide behind the `en`
//! fallback in `t()`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const LOCALES: [&str; 12] = [
    "en", "ar", "de", "es", "fr", "id", "it", "ko", "pt", "ru", "tr", "zh",
];

const HELPER_PREFIXES: [&str; 4] = [
    "notify(ctx, \"",
    "alert(ctx, \"",
    "failure(ctx, \"",
    "cancelPrompt(ctx, \"",
];

const DIRECT_ANSWER_MARKER: &str = "answerCallbackQuery({";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Skip whitespace and `//` line comments.
fn skip_gap(mut rest: &str) -> &str {
    loop {
        rest = rest.trim_start();
        if rest.starts_with("//") {
            match rest.find('\n') {
                Some(i) => rest = &rest[i + 1..],
                None => return "",
            }
            continue;
        }
        return rest;
    }
}

/// Convert TS escape sequences (`\n`, `\"`, `\'`, `\\`, `\uXXXX`, ...) to text.
fn unescape_ts(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('b') => out.push('\u{0008}'),
            Some('f') => out.push('\u{000C}'),
            Some('v') => out.push('\u{000B}'),
            Some('0') => out.push('\0'),
            Some('\n') => {} // line continuation: emit nothing
            Some('u') => {
                let mut hex = String::new();
                if chars.peek() == Some(&'{') {
                    chars.next();
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                        hex.push(c);
                    }
                } else {
                    for _ in 0..4 {
                        match chars.next() {
                            Some(h) => hex.push(h),
                            None => break,
                        }
                    }
                }
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    None => out.push_str(&format!("\\u{hex}")),
                }
            }
            Some(other) => out.push(other), // covers \", \', \\, \`
            None => out.push('\\'),
        }
    }
    out
}

/// Parse one quoted TS string literal at the start of `rest` (first char is
/// the opening `"` or `'`). Returns the unescaped value and the remainder.
/// Values in `src/i18n/*.ts` use `"` or `'` delimiters, never backticks.
fn parse_ts_string<'a>(rest: &'a str, file: &str) -> (String, &'a str) {
    assert!(!rest.is_empty(), "{file}: unexpected end of dictionary");
    let delim = rest.as_bytes()[0];
    assert!(
        delim == b'"' || delim == b'\'',
        "{file}: unexpected string delimiter"
    );
    let mut raw = String::new();
    let mut i = 1;
    while i < rest.len() {
        if rest.as_bytes()[i] == b'\\' {
            let next_len = rest[i + 1..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
            raw.push('\\');
            raw.push_str(&rest[i + 1..i + 1 + next_len]);
            i += 1 + next_len;
            continue;
        }
        if rest.as_bytes()[i] == delim {
            return (unescape_ts(&raw), &rest[i + 1..]);
        }
        let ch_len = rest[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        raw.push_str(&rest[i..i + ch_len]);
        i += ch_len;
    }
    panic!("{file}: unterminated string literal");
}

/// Parse the first `{...}` dictionary object after `export const` into
/// key/value pairs. Every value stays inside one literal, so the entry loop
/// naturally stops at the object's closing brace (this also keeps
/// `projectSearchFallback` in `en.ts` out of the table).
fn parse_dictionary(source: &str, file: &str) -> BTreeMap<String, String> {
    let start = source
        .find("export const")
        .unwrap_or_else(|| panic!("{file}: missing `export const`"));
    let brace = source[start..]
        .find('{')
        .unwrap_or_else(|| panic!("{file}: missing dictionary brace"));
    let mut rest = &source[start + brace + 1..];
    let mut out = BTreeMap::new();
    loop {
        rest = skip_gap(rest);
        if rest.starts_with('}') {
            break;
        }
        // Skip spreads like `...projectSearchFallback,` (locales reuse the
        // `en` fallback table verbatim; values are merged in `main`).
        if rest.starts_with("...") {
            let comma = rest
                .find(',')
                .unwrap_or_else(|| panic!("{file}: unterminated spread"));
            rest = &rest[comma + 1..];
            continue;
        }
        let (key, after_key) = parse_ts_string(rest, file);
        rest = skip_gap(after_key);
        assert!(
            rest.starts_with(':'),
            "{file}: expected `:` after key {key:?}"
        );
        rest = skip_gap(&rest[1..]);
        let (value, after_value) = parse_ts_string(rest, file);
        assert!(
            out.insert(key.clone(), value).is_none(),
            "{file}: duplicate key {key:?}"
        );
        rest = skip_gap(after_value);
        if rest.starts_with(',') {
            rest = &rest[1..];
        }
    }
    out
}

fn collect_ts_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(root).unwrap_or_else(|e| panic!("cannot read dir {}: {e}", root.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("cannot read entry: {e}"))
            .path();
        if path.is_dir() {
            collect_ts_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "ts") {
            out.push(path);
        }
    }
}

/// Mirror the TS callback-answer scan: helper-call literals plus direct
/// `answerCallbackQuery({ ... text: t("...") })` keys.
fn scan_bot_file(content: &str, known: &BTreeSet<String>, keys: &mut BTreeSet<String>) {
    for prefix in HELPER_PREFIXES {
        let mut search = content;
        while let Some(i) = search.find(prefix) {
            let after = &search[i + prefix.len()..];
            let end = after.find('"').unwrap_or(after.len());
            let key = &after[..end];
            if known.contains(key) {
                keys.insert(key.to_string());
            }
            search = &after[end.min(after.len())..];
        }
    }
    let mut search = content;
    while let Some(i) = search.find(DIRECT_ANSWER_MARKER) {
        let from = i + DIRECT_ANSWER_MARKER.len();
        let window_end = search[from..]
            .find('}')
            .map(|e| from + e)
            .unwrap_or(search.len());
        let window = &search[from..window_end];
        for (j, _) in window.match_indices("t(") {
            let before = window[..j].chars().next_back();
            if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
                continue;
            }
            let after = skip_gap(&window[j + 2..]);
            if let Some(literal) = after.strip_prefix('"') {
                let end = literal.find('"').unwrap_or(literal.len());
                let key = &literal[..end];
                if known.contains(key) {
                    keys.insert(key.to_string());
                }
                break;
            }
        }
        search = &search[from..];
    }
}

/// Keys of `export const projectSearchFallback` in `en.ts`. Values there are
/// `en["..."]` references (not string literals), so only the keys are read.
/// Non-`en` locales spread this table to reuse `en` values verbatim.
fn parse_fallback_keys(source: &str, file: &str) -> Vec<String> {
    let marker = "export const projectSearchFallback";
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("{file}: missing projectSearchFallback"));
    let brace = source[start..]
        .find('{')
        .unwrap_or_else(|| panic!("{file}: missing fallback brace"));
    let mut rest = &source[start + brace + 1..];
    let mut keys = Vec::new();
    loop {
        rest = skip_gap(rest);
        if rest.starts_with('}') {
            break;
        }
        let (key, after_key) = parse_ts_string(rest, file);
        keys.push(key);
        // Value is `en["..."]`; skip to the end of this entry.
        let comma = after_key.find(',');
        let close = after_key.find('}');
        match (comma, close) {
            (Some(c), Some(b)) if b < c => {
                rest = &after_key[b..];
            }
            (Some(c), _) => {
                rest = &after_key[c + 1..];
            }
            (None, Some(b)) => {
                rest = &after_key[b..];
            }
            (None, None) => panic!("{file}: unterminated fallback entry"),
        }
    }
    keys
}

fn main() {
    let root = repo_root();
    let i18n_root = root.join("src").join("i18n");
    let bot_root = root.join("src").join("bot");

    let mut tables: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut sources: BTreeMap<String, String> = BTreeMap::new();
    for locale in LOCALES {
        let file = i18n_root.join(format!("{locale}.ts"));
        println!("cargo:rerun-if-changed={}", file.display());
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
        tables.insert(
            locale.to_string(),
            parse_dictionary(&source, &file.display().to_string()),
        );
        sources.insert(locale.to_string(), source);
    }

    // Expand `...projectSearchFallback` (skipped during parsing) with the
    // verbatim `en` values, matching the TS spread semantics.
    let fallback_keys = parse_fallback_keys(
        &sources["en"],
        &i18n_root.join("en.ts").display().to_string(),
    );
    let en_table = tables["en"].clone();
    for locale in LOCALES {
        if locale == "en" {
            continue;
        }
        let table = tables.get_mut(locale).expect("locale table exists");
        for key in &fallback_keys {
            if !table.contains_key(key) {
                let value = en_table
                    .get(key)
                    .unwrap_or_else(|| panic!("fallback key {key:?} missing from `en`"));
                table.insert(key.clone(), value.clone());
            }
        }
    }

    let en_keys: BTreeSet<String> = tables["en"].keys().cloned().collect();
    for locale in LOCALES {
        if locale == "en" {
            continue;
        }
        let keys: BTreeSet<String> = tables[locale].keys().cloned().collect();
        assert!(
            keys == en_keys,
            "locale `{locale}` drifts from the `en` key universe (missing: {:?}, extra: {:?})",
            en_keys.difference(&keys).collect::<Vec<_>>(),
            keys.difference(&en_keys).collect::<Vec<_>>(),
        );
    }

    let mut bot_files = Vec::new();
    collect_ts_files(&bot_root, &mut bot_files);
    for file in &bot_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    let mut callback_keys = BTreeSet::new();
    for file in &bot_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
        scan_bot_file(&content, &en_keys, &mut callback_keys);
    }

    let empty = BTreeMap::new();
    let mut code = String::from(
        "// Generated by `crates/otb-core/build.rs` from `src/i18n/*.ts`. Do not edit.\n\n",
    );
    code.push_str("/// Every `I18nKey` (the `en` key universe), sorted.\n");
    code.push_str("pub const ALL_KEYS: &[&str] = &[\n");
    for key in tables.get("en").unwrap_or(&empty).keys() {
        code.push_str(&format!("    {key:?},\n"));
    }
    code.push_str("];\n\n");
    code.push_str("/// Callback-answer keys statically scanned from `src/bot`\n");
    code.push_str("/// (same scan as `tests/i18n/callback-answer-length.test.ts`).\n");
    code.push_str("pub const CALLBACK_ANSWER_KEYS: &[&str] = &[\n");
    for key in &callback_keys {
        code.push_str(&format!("    {key:?},\n"));
    }
    code.push_str("];\n\n");
    code.push_str("/// Verbatim template lookup over the full 12-locale table.\n");
    code.push_str("pub fn generated_lookup(locale: &str, key: &str) -> Option<&'static str> {\n");
    code.push_str("    match locale {\n");
    for locale in LOCALES {
        let table = tables.get(locale).unwrap_or(&empty);
        code.push_str(&format!("        {locale:?} => match key {{\n"));
        for (key, value) in table {
            code.push_str(&format!("            {key:?} => Some({value:?}),\n"));
        }
        code.push_str("            _ => None,\n        },\n");
    }
    code.push_str("        _ => None,\n    }\n}\n");

    let out_path =
        PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set")).join("i18n_generated.rs");
    fs::write(&out_path, code)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", out_path.display()));
}
