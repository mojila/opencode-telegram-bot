//! Slice 143 parity: Indonesian status-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/id.ts` (lines 146-167) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `id` templates for the 10 status keys
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
//! `it` status lookup + formatting (`locale_it_status_gates`, slice 135),
//! `pt` status lookup + formatting (`locale_pt_status_gates`, slice 139).

use otb_core::locale_id_status_gates::*;

// ---- constants (3) ----

#[test]
fn id_locale_value_parity() {
    assert_eq!(ID_LOCALE, "id");
}

#[test]
fn id_status_keys_value_parity() {
    assert_eq!(ID_STATUS_KEYS.len(), 10);
    assert!(ID_STATUS_KEYS.contains(&"status.line.health"));
    assert!(ID_STATUS_KEYS.contains(&"status.line.model"));
    assert!(ID_STATUS_KEYS.contains(&"status.header_running"));
    assert!(ID_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn id_status_templates_value_parity() {
    assert_eq!(ID_STATUS_LINE_HEALTH, "Status: {health}");
    assert_eq!(ID_STATUS_LINE_VERSION, "Versi OpenCode: {version}");
    assert_eq!(ID_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(ID_STATUS_LINE_UPTIME_SEC, "Waktu aktif: {seconds} detik");
    assert_eq!(ID_STATUS_LINE_MODE, "Agent: {mode}");
    assert_eq!(ID_STATUS_LINE_MODEL, "Model: {model}");
    assert_eq!(ID_STATUS_PROJECT_SELECTED, "Proyek: {project}");
    assert_eq!(ID_STATUS_SESSION_SELECTED, "Sesi saat ini: {title}");
    assert_eq!(
        ID_STATUS_HEADER_RUNNING,
        "🟢 Server OpenCode sedang berjalan"
    );
    assert_eq!(ID_STATUS_HEALTH_HEALTHY, "Sehat");
}

// ---- lookup (2) ----

#[test]
fn id_status_lookup_health_parity() {
    assert_eq!(
        lookup_id_status_template("status.line.health"),
        Some("Status: {health}")
    );
}

#[test]
fn id_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_id_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_id_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn id_status_placeholders_match_en_parity() {
    assert!(is_id_status_placeholder_ok(
        "Status: {health}",
        "Status: {health}"
    ));
    assert!(is_id_status_placeholder_ok(
        "Model: {model}",
        "Model: {model}"
    ));
}

#[test]
fn id_status_placeholders_reject_mismatch_parity() {
    assert!(!is_id_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "Waktu aktif: ok"
    ));
    assert!(!is_id_status_placeholder_ok(
        "Project: {project}",
        "Proyek: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn id_status_health_line_parity() {
    assert_eq!(format_id_status_health_line("Sehat"), "Status: Sehat");
}

#[test]
fn id_status_model_line_parity() {
    assert_eq!(format_id_status_model_line("gpt-5"), "Model: gpt-5");
}

#[test]
fn id_status_project_line_parity() {
    assert_eq!(format_id_status_project_line("bot"), "Proyek: bot");
}
