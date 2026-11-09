//! Slice 134 parity: Spanish goal-remainder gates.
//!
//! Mirrors the pure TS contract in `src/i18n/es.ts` (lines 757-802) for the
//! 26 goal keys outside `GOAL_KEYS` in `crate::i18n_full` (slice 133 covers
//! the first 22):
//! - verbatim `es` templates for the 26 goal-remainder keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the goal call sites)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `es` status lookup + formatting (`locale_es_status_gates`, slice 131),
//! `es` queue lookup + formatting (`locale_es_queue_gates`, slice 132),
//! `es` goal (first 22 keys) lookup + formatting (`locale_es_goal_gates`,
//! slice 133).

use otb_core::locale_es_goal2_gates::*;

// ---- constants (3) ----

#[test]
fn es_goal2_locale_value_parity() {
    assert_eq!(ES_GOAL2_LOCALE, "es");
}

#[test]
fn es_goal2_keys_value_parity() {
    assert_eq!(ES_GOAL2_KEYS.len(), 26);
    assert!(ES_GOAL2_KEYS.contains(&"goal.button.clear"));
    assert!(ES_GOAL2_KEYS.contains(&"goal.confirm.warning"));
    assert!(ES_GOAL2_KEYS.contains(&"goal.reason.paused_by_user"));
    assert!(ES_GOAL2_KEYS.contains(&"goal.reason.objective_limit"));
}

#[test]
fn es_goal2_templates_value_parity() {
    assert_eq!(
        ES_GOAL_ARGUMENTS_NOT_SUPPORTED,
        "Use /goal without arguments, then follow the prompts or use the goal buttons."
    );
    assert_eq!(ES_GOAL_CONFIRM_ROUND_LIMIT, "Round limit: {rounds}.");
    assert_eq!(
        ES_GOAL_REASON_OBJECTIVE_LIMIT,
        "A goal objective is limited to {limit} characters (got {count})."
    );
    assert_eq!(ES_GOAL_REASON_PAUSED_BY_USER, "Paused by the user");
    assert_eq!(
        ES_GOAL_REASON_SESSION_FAILED,
        "The session failed: {reason}"
    );
}

// ---- lookup (2) ----

#[test]
fn es_goal2_lookup_session_failed_parity() {
    assert_eq!(
        lookup_es_goal2_template("goal.reason.session_failed"),
        Some("The session failed: {reason}")
    );
}

#[test]
fn es_goal2_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_es_goal2_template("goal.unknown_key"), None);
    assert_eq!(lookup_es_goal2_template("pinned.line.goal"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn es_goal2_placeholders_match_en_parity() {
    assert!(is_es_goal2_placeholder_ok(
        "Round limit: {rounds}.",
        "Round limit: {rounds}."
    ));
    assert!(is_es_goal2_placeholder_ok(
        "The session failed: {reason}",
        "The session failed: {reason}"
    ));
}

#[test]
fn es_goal2_placeholders_reject_mismatch_parity() {
    assert!(!is_es_goal2_placeholder_ok(
        "A goal objective is limited to {limit} characters (got {count}).",
        "A goal objective is limited to {limit} characters."
    ));
    assert!(!is_es_goal2_placeholder_ok(
        "Round limit: {rounds}.",
        "Round limit: {rounds}. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn es_goal2_objective_limit_line_parity() {
    assert_eq!(
        format_es_goal2_objective_limit(200, 250),
        "A goal objective is limited to 200 characters (got 250)."
    );
}

#[test]
fn es_goal2_confirm_round_limit_line_parity() {
    assert_eq!(format_es_goal2_confirm_round_limit(10), "Round limit: 10.");
}
