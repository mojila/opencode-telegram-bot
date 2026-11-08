//! Slice 123 parity: German status-table gates (RED).
//!
//! Mirrors the pure TS contract in `src/i18n/de.ts` (lines 145-166) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `de` templates for the 10 status keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind `src/bot/commands/status-command.ts`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_de_status_gates` is ported (only the constant-value tests pass).

use otb_core::locale_de_status_gates::*;

// ---- constants (3) ----

#[test]
fn de_locale_value_parity() {
    assert_eq!(DE_LOCALE, "de");
}

#[test]
fn de_status_keys_value_parity() {
    assert_eq!(DE_STATUS_KEYS.len(), 10);
    assert!(DE_STATUS_KEYS.contains(&"status.line.health"));
    assert!(DE_STATUS_KEYS.contains(&"status.line.model"));
    assert!(DE_STATUS_KEYS.contains(&"status.header_running"));
    assert!(DE_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn de_status_templates_value_parity() {
    assert_eq!(DE_STATUS_LINE_HEALTH, "Status: {health}");
    assert_eq!(DE_STATUS_LINE_VERSION, "OpenCode-Version: {version}");
    assert_eq!(DE_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(DE_STATUS_LINE_UPTIME_SEC, "Betriebszeit: {seconds} s");
    assert_eq!(DE_STATUS_LINE_MODE, "Agent: {mode}");
    assert_eq!(DE_STATUS_LINE_MODEL, "Modell: {model}");
    assert_eq!(DE_STATUS_PROJECT_SELECTED, "Projekt: {project}");
    assert_eq!(DE_STATUS_SESSION_SELECTED, "Aktuelle Sitzung: {title}");
    assert_eq!(DE_STATUS_HEADER_RUNNING, "🟢 OpenCode-Server läuft");
    assert_eq!(DE_STATUS_HEALTH_HEALTHY, "OK");
}

// ---- lookup (2) ----

#[test]
fn de_status_lookup_health_parity() {
    assert_eq!(
        lookup_de_status_template("status.line.health"),
        Some("Status: {health}")
    );
}

#[test]
fn de_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_de_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_de_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn de_status_placeholders_match_en_parity() {
    assert!(is_de_status_placeholder_ok(
        "Status: {health}",
        "Status: {health}"
    ));
    assert!(is_de_status_placeholder_ok(
        "Model: {model}",
        "Modell: {model}"
    ));
}

#[test]
fn de_status_placeholders_reject_mismatch_parity() {
    assert!(!is_de_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "Betriebszeit: ok"
    ));
    assert!(!is_de_status_placeholder_ok(
        "Project: {project}",
        "Projekt: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn de_status_health_line_parity() {
    assert_eq!(format_de_status_health_line("OK"), "Status: OK");
}

#[test]
fn de_status_model_line_parity() {
    assert_eq!(format_de_status_model_line("gpt-5"), "Modell: gpt-5");
}

#[test]
fn de_status_project_line_parity() {
    assert_eq!(format_de_status_project_line("bot"), "Projekt: bot");
}
