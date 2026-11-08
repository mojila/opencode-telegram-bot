//! Slice 158 parity: Arabic goal-remainder gates.
//!
//! Mirrors the pure TS contract in `src/i18n/ar.ts` (lines 713-758) for the
//! 26 goal keys outside `GOAL_KEYS` in `crate::i18n_full` (slice 157 covers
//! the first 22):
//! - verbatim `ar` templates for the 26 goal-remainder keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the goal call sites)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `ar` status lookup + formatting (`locale_ar_status_gates`, slice 155),
//! `ar` queue lookup + formatting (`locale_ar_queue_gates`, slice 156),
//! `ar` goal (first 22 keys) lookup + formatting (`locale_ar_goal_gates`,
//! slice 157).

use otb_core::locale_ar_goal2_gates::*;

// ---- constants (3) ----

#[test]
fn ar_goal2_locale_value_parity() {
    assert_eq!(AR_GOAL2_LOCALE, "ar");
}

#[test]
fn ar_goal2_keys_value_parity() {
    assert_eq!(AR_GOAL2_KEYS.len(), 26);
    assert!(AR_GOAL2_KEYS.contains(&"goal.button.clear"));
    assert!(AR_GOAL2_KEYS.contains(&"goal.confirm.warning"));
    assert!(AR_GOAL2_KEYS.contains(&"goal.reason.paused_by_user"));
    assert!(AR_GOAL2_KEYS.contains(&"goal.reason.objective_limit"));
}

#[test]
fn ar_goal2_templates_value_parity() {
    assert_eq!(
        AR_GOAL_ARGUMENTS_NOT_SUPPORTED,
        "Use /goal without arguments, then follow the prompts or use the goal buttons."
    );
    assert_eq!(AR_GOAL_CONFIRM_ROUND_LIMIT, "Round limit: {rounds}.");
    assert_eq!(
        AR_GOAL_REASON_OBJECTIVE_LIMIT,
        "A goal objective is limited to {limit} characters (got {count})."
    );
    assert_eq!(AR_GOAL_REASON_PAUSED_BY_USER, "Paused by the user");
    assert_eq!(
        AR_GOAL_REASON_SESSION_FAILED,
        "The session failed: {reason}"
    );
}

// ---- lookup (2) ----

#[test]
fn ar_goal2_lookup_session_failed_parity() {
    assert_eq!(
        lookup_ar_goal2_template("goal.reason.session_failed"),
        Some("The session failed: {reason}")
    );
}

#[test]
fn ar_goal2_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ar_goal2_template("goal.unknown_key"), None);
    assert_eq!(lookup_ar_goal2_template("pinned.line.goal"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ar_goal2_placeholders_match_en_parity() {
    assert!(is_ar_goal2_placeholder_ok(
        "Round limit: {rounds}.",
        "Round limit: {rounds}."
    ));
    assert!(is_ar_goal2_placeholder_ok(
        "The session failed: {reason}",
        "The session failed: {reason}"
    ));
}

#[test]
fn ar_goal2_placeholders_reject_mismatch_parity() {
    assert!(!is_ar_goal2_placeholder_ok(
        "A goal objective is limited to {limit} characters (got {count}).",
        "A goal objective is limited to {limit} characters."
    ));
    assert!(!is_ar_goal2_placeholder_ok(
        "Round limit: {rounds}.",
        "Round limit: {rounds}. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn ar_goal2_objective_limit_line_parity() {
    assert_eq!(
        format_ar_goal2_objective_limit(200, 250),
        "A goal objective is limited to 200 characters (got 250)."
    );
}

#[test]
fn ar_goal2_confirm_round_limit_line_parity() {
    assert_eq!(
        format_ar_goal2_confirm_round_limit(10),
        "Round limit: 10."
    );
}
