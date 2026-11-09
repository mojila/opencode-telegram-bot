//! Slice 131 parity: Spanish status-table gates (RED).
//!
//! Mirrors the pure TS contract in `src/i18n/es.ts` (lines 145-166) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `es` templates for the 10 status keys
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
//! `fr` status lookup + formatting (`locale_fr_status_gates`, slice 127).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_es_status_gates` is ported (only the constant-value tests pass).

use otb_core::locale_es_status_gates::*;

// ---- constants (3) ----

#[test]
fn es_locale_value_parity() {
    assert_eq!(ES_LOCALE, "es");
}

#[test]
fn es_status_keys_value_parity() {
    assert_eq!(ES_STATUS_KEYS.len(), 10);
    assert!(ES_STATUS_KEYS.contains(&"status.line.health"));
    assert!(ES_STATUS_KEYS.contains(&"status.line.model"));
    assert!(ES_STATUS_KEYS.contains(&"status.header_running"));
    assert!(ES_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn es_status_templates_value_parity() {
    assert_eq!(ES_STATUS_LINE_HEALTH, "Estado: {health}");
    assert_eq!(ES_STATUS_LINE_VERSION, "Versión de OpenCode: {version}");
    assert_eq!(ES_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(ES_STATUS_LINE_UPTIME_SEC, "Tiempo activo: {seconds} s");
    assert_eq!(ES_STATUS_LINE_MODE, "Agente: {mode}");
    assert_eq!(ES_STATUS_LINE_MODEL, "Modelo: {model}");
    assert_eq!(ES_STATUS_PROJECT_SELECTED, "Proyecto: {project}");
    assert_eq!(ES_STATUS_SESSION_SELECTED, "Sesión actual: {title}");
    assert_eq!(
        ES_STATUS_HEADER_RUNNING,
        "🟢 OpenCode Server está en ejecución"
    );
    assert_eq!(ES_STATUS_HEALTH_HEALTHY, "Saludable");
}

// ---- lookup (2) ----

#[test]
fn es_status_lookup_health_parity() {
    assert_eq!(
        lookup_es_status_template("status.line.health"),
        Some("Estado: {health}")
    );
}

#[test]
fn es_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_es_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_es_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn es_status_placeholders_match_en_parity() {
    assert!(is_es_status_placeholder_ok(
        "Status: {health}",
        "Estado: {health}"
    ));
    assert!(is_es_status_placeholder_ok(
        "Model: {model}",
        "Modelo: {model}"
    ));
}

#[test]
fn es_status_placeholders_reject_mismatch_parity() {
    assert!(!is_es_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "Tiempo activo: ok"
    ));
    assert!(!is_es_status_placeholder_ok(
        "Project: {project}",
        "Proyecto: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn es_status_health_line_parity() {
    assert_eq!(
        format_es_status_health_line("Saludable"),
        "Estado: Saludable"
    );
}

#[test]
fn es_status_model_line_parity() {
    assert_eq!(format_es_status_model_line("gpt-5"), "Modelo: gpt-5");
}

#[test]
fn es_status_project_line_parity() {
    assert_eq!(format_es_status_project_line("bot"), "Proyecto: bot");
}
