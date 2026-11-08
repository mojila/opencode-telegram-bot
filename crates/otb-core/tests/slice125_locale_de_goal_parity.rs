//! Slice 125 parity: German goal-table gates (RED).
//!
//! Mirrors the pure TS contract in `src/i18n/de.ts` (lines 758-800) for the
//! goal surface (`GOAL_KEYS` in `crate::i18n_full`; currently English
//! fallback, untranslated in `de`):
//! - verbatim `de` templates for the 22 goal keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the goal call sites)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `de` status lookup + formatting (`locale_de_status_gates`, slice 123),
//! `de` queue lookup + formatting (`locale_de_queue_gates`, slice 124).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_de_goal_gates` is ported (only the constant-value tests pass).

use otb_core::locale_de_goal_gates::*;

// ---- constants (3) ----

#[test]
fn de_goal_locale_value_parity() {
    assert_eq!(DE_GOAL_LOCALE, "de");
}

#[test]
fn de_goal_keys_value_parity() {
    assert_eq!(DE_GOAL_KEYS.len(), 22);
    assert!(DE_GOAL_KEYS.contains(&"pinned.line.goal"));
    assert!(DE_GOAL_KEYS.contains(&"goal.too_long"));
    assert!(DE_GOAL_KEYS.contains(&"goal.reason.round_limit"));
    assert!(DE_GOAL_KEYS.contains(&"goal.reason.not_progress"));
}

#[test]
fn de_goal_templates_value_parity() {
    assert_eq!(DE_PINNED_LINE_GOAL, "◎ Goal: {status} — {objective}");
    assert_eq!(
        DE_GOAL_NONE,
        "No goal is set.\n\nUse /goal, then send the objective in the next message."
    );
    assert_eq!(
        DE_GOAL_TOO_LONG,
        "A goal is limited to {limit} characters (got {count})."
    );
    assert_eq!(
        DE_GOAL_REASON_ROUND_LIMIT,
        "Reached the limit of {rounds} rounds"
    );
    assert_eq!(
        DE_GOAL_REASON_WAIT_LIMIT,
        "Nothing ran for {minutes} minutes while the goal waited on background work"
    );
    assert_eq!(
        DE_GOAL_REASON_NOT_PROGRESS,
        "Not making progress: {reason}"
    );
}

// ---- lookup (2) ----

#[test]
fn de_goal_lookup_pinned_line_parity() {
    assert_eq!(
        lookup_de_goal_template("pinned.line.goal"),
        Some("◎ Goal: {status} — {objective}")
    );
}

#[test]
fn de_goal_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_de_goal_template("goal.unknown_key"), None);
    assert_eq!(lookup_de_goal_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn de_goal_placeholders_match_en_parity() {
    assert!(is_de_goal_placeholder_ok(
        "◎ Goal: {status} — {objective}",
        "◎ Goal: {status} — {objective}"
    ));
    assert!(is_de_goal_placeholder_ok("▶ Continue", "▶ Continue"));
}

#[test]
fn de_goal_placeholders_reject_mismatch_parity() {
    assert!(!is_de_goal_placeholder_ok(
        "A goal is limited to {limit} characters (got {count}).",
        "A goal is limited to {limit} characters."
    ));
    assert!(!is_de_goal_placeholder_ok(
        "Reached the limit of {rounds} rounds",
        "Reached the limit of {rounds} rounds. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn de_goal_too_long_line_parity() {
    assert_eq!(
        format_de_goal_too_long(200, 250),
        "A goal is limited to 200 characters (got 250)."
    );
}

#[test]
fn de_goal_round_limit_line_parity() {
    assert_eq!(
        format_de_goal_round_limit(10),
        "Reached the limit of 10 rounds"
    );
}
