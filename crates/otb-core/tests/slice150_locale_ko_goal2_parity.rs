//! Slice 150 parity: Korean goal-remainder gates.
//!
//! Mirrors the pure TS contract in `src/i18n/ko.ts` (lines 740-785) for the
//! 26 goal keys outside `GOAL_KEYS` in `crate::i18n_full` (slice 149 covers
//! the first 22):
//! - verbatim `ko` templates for the 26 goal-remainder keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the goal call sites)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `ko` status lookup + formatting (`locale_ko_status_gates`, slice 147),
//! `ko` queue lookup + formatting (`locale_ko_queue_gates`, slice 148),
//! `ko` goal (first 22 keys) lookup + formatting (`locale_ko_goal_gates`,
//! slice 149).

use otb_core::locale_ko_goal2_gates::*;

// ---- constants (3) ----

#[test]
fn ko_goal2_locale_value_parity() {
    assert_eq!(KO_GOAL2_LOCALE, "ko");
}

#[test]
fn ko_goal2_keys_value_parity() {
    assert_eq!(KO_GOAL2_KEYS.len(), 26);
    assert!(KO_GOAL2_KEYS.contains(&"goal.button.clear"));
    assert!(KO_GOAL2_KEYS.contains(&"goal.confirm.warning"));
    assert!(KO_GOAL2_KEYS.contains(&"goal.reason.paused_by_user"));
    assert!(KO_GOAL2_KEYS.contains(&"goal.reason.objective_limit"));
}

#[test]
fn ko_goal2_templates_value_parity() {
    assert_eq!(
        KO_GOAL_ARGUMENTS_NOT_SUPPORTED,
        "Use /goal without arguments, then follow the prompts or use the goal buttons."
    );
    assert_eq!(KO_GOAL_CONFIRM_ROUND_LIMIT, "Round limit: {rounds}.");
    assert_eq!(
        KO_GOAL_REASON_OBJECTIVE_LIMIT,
        "A goal objective is limited to {limit} characters (got {count})."
    );
    assert_eq!(KO_GOAL_REASON_PAUSED_BY_USER, "Paused by the user");
    assert_eq!(
        KO_GOAL_REASON_SESSION_FAILED,
        "The session failed: {reason}"
    );
}

// ---- lookup (2) ----

#[test]
fn ko_goal2_lookup_session_failed_parity() {
    assert_eq!(
        lookup_ko_goal2_template("goal.reason.session_failed"),
        Some("The session failed: {reason}")
    );
}

#[test]
fn ko_goal2_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ko_goal2_template("goal.unknown_key"), None);
    assert_eq!(lookup_ko_goal2_template("pinned.line.goal"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ko_goal2_placeholders_match_en_parity() {
    assert!(is_ko_goal2_placeholder_ok(
        "Round limit: {rounds}.",
        "Round limit: {rounds}."
    ));
    assert!(is_ko_goal2_placeholder_ok(
        "The session failed: {reason}",
        "The session failed: {reason}"
    ));
}

#[test]
fn ko_goal2_placeholders_reject_mismatch_parity() {
    assert!(!is_ko_goal2_placeholder_ok(
        "A goal objective is limited to {limit} characters (got {count}).",
        "A goal objective is limited to {limit} characters."
    ));
    assert!(!is_ko_goal2_placeholder_ok(
        "Round limit: {rounds}.",
        "Round limit: {rounds}. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn ko_goal2_objective_limit_line_parity() {
    assert_eq!(
        format_ko_goal2_objective_limit(200, 250),
        "A goal objective is limited to 200 characters (got 250)."
    );
}

#[test]
fn ko_goal2_confirm_round_limit_line_parity() {
    assert_eq!(format_ko_goal2_confirm_round_limit(10), "Round limit: 10.");
}
