//! Slice 127 parity: French status-table gates (RED).
//!
//! Mirrors the pure TS contract in `src/i18n/fr.ts` (lines 148-171) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `fr` templates for the 10 status keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind `src/bot/commands/status-command.ts`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `de` status lookup + formatting (`locale_de_status_gates`, slice 123).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_fr_status_gates` is ported (only the constant-value tests pass).

use otb_core::locale_fr_status_gates::*;

// ---- constants (3) ----

#[test]
fn fr_locale_value_parity() {
    assert_eq!(FR_LOCALE, "fr");
}

#[test]
fn fr_status_keys_value_parity() {
    assert_eq!(FR_STATUS_KEYS.len(), 10);
    assert!(FR_STATUS_KEYS.contains(&"status.line.health"));
    assert!(FR_STATUS_KEYS.contains(&"status.line.model"));
    assert!(FR_STATUS_KEYS.contains(&"status.header_running"));
    assert!(FR_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn fr_status_templates_value_parity() {
    assert_eq!(FR_STATUS_LINE_HEALTH, "Statut : {health}");
    assert_eq!(FR_STATUS_LINE_VERSION, "Version OpenCode : {version}");
    assert_eq!(FR_STATUS_LINE_PID, "PID : {pid}");
    assert_eq!(
        FR_STATUS_LINE_UPTIME_SEC,
        "Temps de fonctionnement : {seconds} sec"
    );
    assert_eq!(FR_STATUS_LINE_MODE, "Agent : {mode}");
    assert_eq!(FR_STATUS_LINE_MODEL, "Modèle : {model}");
    assert_eq!(FR_STATUS_PROJECT_SELECTED, "Projet : {project}");
    assert_eq!(FR_STATUS_SESSION_SELECTED, "Session actuelle : {title}");
    assert_eq!(
        FR_STATUS_HEADER_RUNNING,
        "🟢 Le serveur OpenCode est en cours d'exécution"
    );
    assert_eq!(FR_STATUS_HEALTH_HEALTHY, "Sain");
}

// ---- lookup (2) ----

#[test]
fn fr_status_lookup_health_parity() {
    assert_eq!(
        lookup_fr_status_template("status.line.health"),
        Some("Statut : {health}")
    );
}

#[test]
fn fr_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_fr_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_fr_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn fr_status_placeholders_match_en_parity() {
    assert!(is_fr_status_placeholder_ok(
        "Status: {health}",
        "Statut : {health}"
    ));
    assert!(is_fr_status_placeholder_ok(
        "Model: {model}",
        "Modèle : {model}"
    ));
}

#[test]
fn fr_status_placeholders_reject_mismatch_parity() {
    assert!(!is_fr_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "Temps de fonctionnement : ok"
    ));
    assert!(!is_fr_status_placeholder_ok(
        "Project: {project}",
        "Projet : {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn fr_status_health_line_parity() {
    assert_eq!(format_fr_status_health_line("Sain"), "Statut : Sain");
}

#[test]
fn fr_status_model_line_parity() {
    assert_eq!(format_fr_status_model_line("gpt-5"), "Modèle : gpt-5");
}

#[test]
fn fr_status_project_line_parity() {
    assert_eq!(format_fr_status_project_line("bot"), "Projet : bot");
}
