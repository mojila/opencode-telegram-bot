//! Slice 135 parity: Italian status-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/it.ts` (lines 149-170) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `it` templates for the 10 status keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind `src/bot/commands/status-command.ts`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `de` status lookup + formatting (`locale_de_status_gates`, slice 123),
//! `fr` status lookup + formatting (`locale_fr_status_gates`, slice 127),
//! `es` status lookup + formatting (`locale_es_status_gates`, slice 131).

use otb_core::locale_it_status_gates::*;

// ---- constants (3) ----

#[test]
fn it_locale_value_parity() {
    assert_eq!(IT_LOCALE, "it");
}

#[test]
fn it_status_keys_value_parity() {
    assert_eq!(IT_STATUS_KEYS.len(), 10);
    assert!(IT_STATUS_KEYS.contains(&"status.line.health"));
    assert!(IT_STATUS_KEYS.contains(&"status.line.model"));
    assert!(IT_STATUS_KEYS.contains(&"status.header_running"));
    assert!(IT_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn it_status_templates_value_parity() {
    assert_eq!(IT_STATUS_LINE_HEALTH, "Stato: {health}");
    assert_eq!(IT_STATUS_LINE_VERSION, "Versione OpenCode: {version}");
    assert_eq!(IT_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(
        IT_STATUS_LINE_UPTIME_SEC,
        "Tempo di attività: {seconds} sec"
    );
    assert_eq!(IT_STATUS_LINE_MODE, "Agente: {mode}");
    assert_eq!(IT_STATUS_LINE_MODEL, "Modello: {model}");
    assert_eq!(IT_STATUS_PROJECT_SELECTED, "Progetto: {project}");
    assert_eq!(IT_STATUS_SESSION_SELECTED, "Sessione corrente: {title}");
    assert_eq!(
        IT_STATUS_HEADER_RUNNING,
        "🟢 Il server OpenCode è in esecuzione"
    );
    assert_eq!(IT_STATUS_HEALTH_HEALTHY, "Integro");
}

// ---- lookup (2) ----

#[test]
fn it_status_lookup_health_parity() {
    assert_eq!(
        lookup_it_status_template("status.line.health"),
        Some("Stato: {health}")
    );
}

#[test]
fn it_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_it_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_it_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn it_status_placeholders_match_en_parity() {
    assert!(is_it_status_placeholder_ok(
        "Status: {health}",
        "Stato: {health}"
    ));
    assert!(is_it_status_placeholder_ok(
        "Model: {model}",
        "Modello: {model}"
    ));
}

#[test]
fn it_status_placeholders_reject_mismatch_parity() {
    assert!(!is_it_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "Tempo di attività: ok"
    ));
    assert!(!is_it_status_placeholder_ok(
        "Project: {project}",
        "Progetto: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn it_status_health_line_parity() {
    assert_eq!(format_it_status_health_line("Integro"), "Stato: Integro");
}

#[test]
fn it_status_model_line_parity() {
    assert_eq!(format_it_status_model_line("gpt-5"), "Modello: gpt-5");
}

#[test]
fn it_status_project_line_parity() {
    assert_eq!(format_it_status_project_line("bot"), "Progetto: bot");
}
