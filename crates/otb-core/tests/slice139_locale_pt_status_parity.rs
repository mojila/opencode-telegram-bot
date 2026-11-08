//! Slice 139 parity: Portuguese status-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/pt.ts` (lines 144-165) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `pt` templates for the 10 status keys
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
//! `es` status lookup + formatting (`locale_es_status_gates`, slice 131),
//! `it` status lookup + formatting (`locale_it_status_gates`, slice 135).

use otb_core::locale_pt_status_gates::*;

// ---- constants (3) ----

#[test]
fn pt_locale_value_parity() {
    assert_eq!(PT_LOCALE, "pt");
}

#[test]
fn pt_status_keys_value_parity() {
    assert_eq!(PT_STATUS_KEYS.len(), 10);
    assert!(PT_STATUS_KEYS.contains(&"status.line.health"));
    assert!(PT_STATUS_KEYS.contains(&"status.line.model"));
    assert!(PT_STATUS_KEYS.contains(&"status.header_running"));
    assert!(PT_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn pt_status_templates_value_parity() {
    assert_eq!(PT_STATUS_LINE_HEALTH, "Status: {health}");
    assert_eq!(PT_STATUS_LINE_VERSION, "Versão do OpenCode: {version}");
    assert_eq!(PT_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(PT_STATUS_LINE_UPTIME_SEC, "Tempo ativo: {seconds} s");
    assert_eq!(PT_STATUS_LINE_MODE, "Agente: {mode}");
    assert_eq!(PT_STATUS_LINE_MODEL, "Modelo: {model}");
    assert_eq!(PT_STATUS_PROJECT_SELECTED, "Projeto: {project}");
    assert_eq!(PT_STATUS_SESSION_SELECTED, "Sessão atual: {title}");
    assert_eq!(
        PT_STATUS_HEADER_RUNNING,
        "🟢 O OpenCode Server está em execução"
    );
    assert_eq!(PT_STATUS_HEALTH_HEALTHY, "Saudável");
}

// ---- lookup (2) ----

#[test]
fn pt_status_lookup_health_parity() {
    assert_eq!(
        lookup_pt_status_template("status.line.health"),
        Some("Status: {health}")
    );
}

#[test]
fn pt_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_pt_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_pt_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn pt_status_placeholders_match_en_parity() {
    assert!(is_pt_status_placeholder_ok(
        "Status: {health}",
        "Status: {health}"
    ));
    assert!(is_pt_status_placeholder_ok(
        "Model: {model}",
        "Modelo: {model}"
    ));
}

#[test]
fn pt_status_placeholders_reject_mismatch_parity() {
    assert!(!is_pt_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "Tempo ativo: ok"
    ));
    assert!(!is_pt_status_placeholder_ok(
        "Project: {project}",
        "Projeto: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn pt_status_health_line_parity() {
    assert_eq!(format_pt_status_health_line("Saudável"), "Status: Saudável");
}

#[test]
fn pt_status_model_line_parity() {
    assert_eq!(format_pt_status_model_line("gpt-5"), "Modelo: gpt-5");
}

#[test]
fn pt_status_project_line_parity() {
    assert_eq!(format_pt_status_project_line("bot"), "Projeto: bot");
}
