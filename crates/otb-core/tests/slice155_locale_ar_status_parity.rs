//! Slice 155 parity: Arabic status-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/ar.ts` (lines 139-160) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `ar` templates for the 10 status keys
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
//! `ko` status lookup + formatting (`locale_ko_status_gates`, slice 147),
//! `tr` status lookup + formatting (`locale_tr_status_gates`, slice 151).

use otb_core::locale_ar_status_gates::*;

// ---- constants (3) ----

#[test]
fn ar_locale_value_parity() {
    assert_eq!(AR_LOCALE, "ar");
}

#[test]
fn ar_status_keys_value_parity() {
    assert_eq!(AR_STATUS_KEYS.len(), 10);
    assert!(AR_STATUS_KEYS.contains(&"status.line.health"));
    assert!(AR_STATUS_KEYS.contains(&"status.line.model"));
    assert!(AR_STATUS_KEYS.contains(&"status.header_running"));
    assert!(AR_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn ar_status_templates_value_parity() {
    assert_eq!(AR_STATUS_LINE_HEALTH, "الحالة: {health}");
    assert_eq!(AR_STATUS_LINE_VERSION, "إصدار OpenCode: {version}");
    assert_eq!(AR_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(AR_STATUS_LINE_UPTIME_SEC, "مدة التشغيل: {seconds} ثانية");
    assert_eq!(AR_STATUS_LINE_MODE, "الوكيل: {mode}");
    assert_eq!(AR_STATUS_LINE_MODEL, "النموذج: {model}");
    assert_eq!(AR_STATUS_PROJECT_SELECTED, "المشروع: {project}");
    assert_eq!(AR_STATUS_SESSION_SELECTED, "الجلسة الحالية: {title}");
    assert_eq!(AR_STATUS_HEADER_RUNNING, "🟢 خادم OpenCode يعمل");
    assert_eq!(AR_STATUS_HEALTH_HEALTHY, "يعمل بشكل طبيعي");
}

// ---- lookup (2) ----

#[test]
fn ar_status_lookup_health_parity() {
    assert_eq!(
        lookup_ar_status_template("status.line.health"),
        Some("الحالة: {health}")
    );
}

#[test]
fn ar_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ar_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_ar_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ar_status_placeholders_match_en_parity() {
    assert!(is_ar_status_placeholder_ok(
        "Status: {health}",
        "الحالة: {health}"
    ));
    assert!(is_ar_status_placeholder_ok(
        "Model: {model}",
        "النموذج: {model}"
    ));
}

#[test]
fn ar_status_placeholders_reject_mismatch_parity() {
    assert!(!is_ar_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "مدة التشغيل: ثانية"
    ));
    assert!(!is_ar_status_placeholder_ok(
        "Project: {project}",
        "المشروع: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn ar_status_health_line_parity() {
    assert_eq!(
        format_ar_status_health_line("يعمل بشكل طبيعي"),
        "الحالة: يعمل بشكل طبيعي"
    );
}

#[test]
fn ar_status_model_line_parity() {
    assert_eq!(format_ar_status_model_line("gpt-5"), "النموذج: gpt-5");
}

#[test]
fn ar_status_project_line_parity() {
    assert_eq!(format_ar_status_project_line("bot"), "المشروع: bot");
}
