//! Slice 147 parity: Korean status-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/ko.ts` (lines 146-167) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `ko` templates for the 10 status keys
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
//! `id` status lookup + formatting (`locale_id_status_gates`, slice 143).

use otb_core::locale_ko_status_gates::*;

// ---- constants (3) ----

#[test]
fn ko_locale_value_parity() {
    assert_eq!(KO_LOCALE, "ko");
}

#[test]
fn ko_status_keys_value_parity() {
    assert_eq!(KO_STATUS_KEYS.len(), 10);
    assert!(KO_STATUS_KEYS.contains(&"status.line.health"));
    assert!(KO_STATUS_KEYS.contains(&"status.line.model"));
    assert!(KO_STATUS_KEYS.contains(&"status.header_running"));
    assert!(KO_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn ko_status_templates_value_parity() {
    assert_eq!(KO_STATUS_LINE_HEALTH, "상태: {health}");
    assert_eq!(KO_STATUS_LINE_VERSION, "OpenCode 버전: {version}");
    assert_eq!(KO_STATUS_LINE_PID, "PID: {pid}");
    assert_eq!(KO_STATUS_LINE_UPTIME_SEC, "가동 시간: {seconds}초");
    assert_eq!(KO_STATUS_LINE_MODE, "에이전트: {mode}");
    assert_eq!(KO_STATUS_LINE_MODEL, "모델: {model}");
    assert_eq!(KO_STATUS_PROJECT_SELECTED, "프로젝트: {project}");
    assert_eq!(KO_STATUS_SESSION_SELECTED, "현재 세션: {title}");
    assert_eq!(KO_STATUS_HEADER_RUNNING, "🟢 OpenCode 서버 실행 중");
    assert_eq!(KO_STATUS_HEALTH_HEALTHY, "정상");
}

// ---- lookup (2) ----

#[test]
fn ko_status_lookup_health_parity() {
    assert_eq!(
        lookup_ko_status_template("status.line.health"),
        Some("상태: {health}")
    );
}

#[test]
fn ko_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ko_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_ko_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ko_status_placeholders_match_en_parity() {
    assert!(is_ko_status_placeholder_ok(
        "Status: {health}",
        "상태: {health}"
    ));
    assert!(is_ko_status_placeholder_ok("Model: {model}", "모델: {model}"));
}

#[test]
fn ko_status_placeholders_reject_mismatch_parity() {
    assert!(!is_ko_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "가동 시간: ok"
    ));
    assert!(!is_ko_status_placeholder_ok(
        "Project: {project}",
        "프로젝트: {project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn ko_status_health_line_parity() {
    assert_eq!(format_ko_status_health_line("정상"), "상태: 정상");
}

#[test]
fn ko_status_model_line_parity() {
    assert_eq!(format_ko_status_model_line("gpt-5"), "모델: gpt-5");
}

#[test]
fn ko_status_project_line_parity() {
    assert_eq!(format_ko_status_project_line("bot"), "프로젝트: bot");
}
