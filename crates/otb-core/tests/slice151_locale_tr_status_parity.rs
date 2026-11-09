//! Slice 151 parity: Turkish status-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/tr.ts` (lines 140-161) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `tr` templates for the 10 status keys
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
//! `pt` status lookup + formatting (`locale_pt_status_gates`, slice 139),
//! `id` status lookup + formatting (`locale_id_status_gates`, slice 143),
//! `ko` status lookup + formatting (`locale_ko_status_gates`, slice 147).

use otb_core::locale_tr_status_gates::*;

// ---- constants (3) ----

#[test]
fn tr_locale_value_parity() {
    assert_eq!(TR_LOCALE, "tr");
}

#[test]
fn tr_status_keys_value_parity() {
    assert_eq!(TR_STATUS_KEYS.len(), 10);
    assert!(TR_STATUS_KEYS.contains(&"status.line.health"));
    assert!(TR_STATUS_KEYS.contains(&"status.line.model"));
    assert!(TR_STATUS_KEYS.contains(&"status.header_running"));
    assert!(TR_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn tr_status_templates_value_parity() {
    assert_eq!(TR_STATUS_LINE_HEALTH, "Durum: {health}");
    assert_eq!(TR_STATUS_LINE_VERSION, "OpenCode sürümü: {version}");
    assert_eq!(TR_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(TR_STATUS_LINE_UPTIME_SEC, "Çalışma süresi: {seconds} sn");
    assert_eq!(TR_STATUS_LINE_MODE, "Ajan: {mode}");
    assert_eq!(TR_STATUS_LINE_MODEL, "Model: {model}");
    assert_eq!(TR_STATUS_PROJECT_SELECTED, "Proje: {project}");
    assert_eq!(TR_STATUS_SESSION_SELECTED, "Geçerli oturum: {title}");
    assert_eq!(TR_STATUS_HEADER_RUNNING, "🟢 OpenCode Sunucusu çalışıyor");
    assert_eq!(TR_STATUS_HEALTH_HEALTHY, "Sağlıklı");
}

// ---- lookup (2) ----

#[test]
fn tr_status_lookup_health_parity() {
    assert_eq!(
        lookup_tr_status_template("status.line.health"),
        Some("Durum: {health}")
    );
}

#[test]
fn tr_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_tr_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_tr_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn tr_status_placeholders_match_en_parity() {
    assert!(is_tr_status_placeholder_ok(
        "Status: {health}",
        "Durum: {health}"
    ));
    assert!(is_tr_status_placeholder_ok(
        "Model: {model}",
        "Model: {model}"
    ));
}

#[test]
fn tr_status_placeholders_reject_mismatch_parity() {
    assert!(!is_tr_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "Çalışma süresi: ok"
    ));
    assert!(!is_tr_status_placeholder_ok(
        "Project: {project}",
        "Proje: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn tr_status_health_line_parity() {
    assert_eq!(format_tr_status_health_line("Sağlıklı"), "Durum: Sağlıklı");
}

#[test]
fn tr_status_model_line_parity() {
    assert_eq!(format_tr_status_model_line("gpt-5"), "Model: gpt-5");
}

#[test]
fn tr_status_project_line_parity() {
    assert_eq!(format_tr_status_project_line("bot"), "Proje: bot");
}
