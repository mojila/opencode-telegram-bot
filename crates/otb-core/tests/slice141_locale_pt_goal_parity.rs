//! Slice 141 parity: Portuguese goal-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/pt.ts` (lines 752-794) for the
//! goal surface (`GOAL_KEYS` in `crate::i18n_full`; currently English
//! fallback, untranslated in `pt` — same as `it` in slice 137, `es` in
//! slice 133 and `fr` in slice 129):
//! - verbatim `pt` templates for the 22 goal keys
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
//! `de` queue lookup + formatting (`locale_de_queue_gates`, slice 124),
//! `de` goal (first 22 keys) lookup + formatting (`locale_de_goal_gates`,
//! slice 125), `de` goal-remainder lookup + formatting
//! (`locale_de_goal2_gates`, slice 126), `fr` status lookup + formatting
//! (`locale_fr_status_gates`, slice 127), `fr` queue lookup + formatting
//! (`locale_fr_queue_gates`, slice 128), `fr` goal lookup + formatting
//! (`locale_fr_goal_gates`, slice 129), `fr` goal-remainder lookup +
//! formatting (`locale_fr_goal2_gates`, slice 130), `es` status lookup +
//! formatting (`locale_es_status_gates`, slice 131), `es` queue lookup +
//! formatting (`locale_es_queue_gates`, slice 132), `es` goal lookup +
//! formatting (`locale_es_goal_gates`, slice 133), `es` goal-remainder
//! lookup + formatting (`locale_es_goal2_gates`, slice 134), `it` status
//! lookup + formatting (`locale_it_status_gates`, slice 135), `it` queue
//! lookup + formatting (`locale_it_queue_gates`, slice 136), `it` goal
//! lookup + formatting (`locale_it_goal_gates`, slice 137), `it`
//! goal-remainder lookup + formatting (`locale_it_goal2_gates`, slice 138),
//! `pt` status lookup + formatting (`locale_pt_status_gates`, slice 139),
//! `pt` queue lookup + formatting (`locale_pt_queue_gates`, slice 140).

use otb_core::locale_pt_goal_gates::*;

// ---- constants (3) ----

#[test]
fn pt_goal_locale_value_parity() {
    assert_eq!(PT_GOAL_LOCALE, "pt");
}

#[test]
fn pt_goal_keys_value_parity() {
    assert_eq!(PT_GOAL_KEYS.len(), 22);
    assert!(PT_GOAL_KEYS.contains(&"pinned.line.goal"));
    assert!(PT_GOAL_KEYS.contains(&"goal.too_long"));
    assert!(PT_GOAL_KEYS.contains(&"goal.reason.round_limit"));
    assert!(PT_GOAL_KEYS.contains(&"goal.reason.not_progress"));
}

#[test]
fn pt_goal_templates_value_parity() {
    assert_eq!(PT_PINNED_LINE_GOAL, "◎ Goal: {status} — {objective}");
    assert_eq!(
        PT_GOAL_NONE,
        "No goal is set.\n\nUse /goal, then send the objective in the next message."
    );
    assert_eq!(
        PT_GOAL_TOO_LONG,
        "A goal is limited to {limit} characters (got {count})."
    );
    assert_eq!(
        PT_GOAL_REASON_ROUND_LIMIT,
        "Reached the limit of {rounds} rounds"
    );
    assert_eq!(
        PT_GOAL_REASON_WAIT_LIMIT,
        "Nothing ran for {minutes} minutes while the goal waited on background work"
    );
    assert_eq!(
        PT_GOAL_REASON_NOT_PROGRESS,
        "Not making progress: {reason}"
    );
}

// ---- lookup (2) ----

#[test]
fn pt_goal_lookup_pinned_line_parity() {
    assert_eq!(
        lookup_pt_goal_template("pinned.line.goal"),
        Some("◎ Goal: {status} — {objective}")
    );
}

#[test]
fn pt_goal_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_pt_goal_template("goal.unknown_key"), None);
    assert_eq!(lookup_pt_goal_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn pt_goal_placeholders_match_en_parity() {
    assert!(is_pt_goal_placeholder_ok(
        "◎ Goal: {status} — {objective}",
        "◎ Goal: {status} — {objective}"
    ));
    assert!(is_pt_goal_placeholder_ok("▶ Continue", "▶ Continue"));
}

#[test]
fn pt_goal_placeholders_reject_mismatch_parity() {
    assert!(!is_pt_goal_placeholder_ok(
        "A goal is limited to {limit} characters (got {count}).",
        "A goal is limited to {limit} characters."
    ));
    assert!(!is_pt_goal_placeholder_ok(
        "Reached the limit of {rounds} rounds",
        "Reached the limit of {rounds} rounds. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn pt_goal_too_long_line_parity() {
    assert_eq!(
        format_pt_goal_too_long(200, 250),
        "A goal is limited to 200 characters (got 250)."
    );
}

#[test]
fn pt_goal_round_limit_line_parity() {
    assert_eq!(
        format_pt_goal_round_limit(10),
        "Reached the limit of 10 rounds"
    );
}
