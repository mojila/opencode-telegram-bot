//! Slice 159 parity: Chinese status-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/zh.ts` (lines 127-148) for the
//! status surface (`STATUS_KEYS` in `crate::i18n_full`):
//! - verbatim `zh` templates for the 10 status keys
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
//! `tr` status lookup + formatting (`locale_tr_status_gates`, slice 151),
//! `ar` status lookup + formatting (`locale_ar_status_gates`, slice 155).

use otb_core::locale_zh_status_gates::*;

// ---- constants (3) ----

#[test]
fn zh_locale_value_parity() {
    assert_eq!(ZH_LOCALE, "zh");
}

#[test]
fn zh_status_keys_value_parity() {
    assert_eq!(ZH_STATUS_KEYS.len(), 10);
    assert!(ZH_STATUS_KEYS.contains(&"status.line.health"));
    assert!(ZH_STATUS_KEYS.contains(&"status.line.model"));
    assert!(ZH_STATUS_KEYS.contains(&"status.header_running"));
    assert!(ZH_STATUS_KEYS.contains(&"status.health.healthy"));
}

#[test]
fn zh_status_templates_value_parity() {
    assert_eq!(ZH_STATUS_LINE_HEALTH, "状态：{health}");
    assert_eq!(ZH_STATUS_LINE_VERSION, "OpenCode 版本：{version}");
    assert_eq!(ZH_STATUS_LINE_PID, "PID：{pid}");
    assert_eq!(ZH_STATUS_LINE_UPTIME_SEC, "运行时间：{seconds} 秒");
    assert_eq!(ZH_STATUS_LINE_MODE, "Agent：{mode}");
    assert_eq!(ZH_STATUS_LINE_MODEL, "模型：{model}");
    assert_eq!(ZH_STATUS_PROJECT_SELECTED, "项目：{project}");
    assert_eq!(ZH_STATUS_SESSION_SELECTED, "当前会话：{title}");
    assert_eq!(ZH_STATUS_HEADER_RUNNING, "🟢 OpenCode 服务器正在运行");
    assert_eq!(ZH_STATUS_HEALTH_HEALTHY, "健康");
}

// ---- lookup (2) ----

#[test]
fn zh_status_lookup_health_parity() {
    assert_eq!(
        lookup_zh_status_template("status.line.health"),
        Some("状态：{health}")
    );
}

#[test]
fn zh_status_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_zh_status_template("status.line.unknown_key"), None);
    assert_eq!(lookup_zh_status_template("queue.added"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn zh_status_placeholders_match_en_parity() {
    assert!(is_zh_status_placeholder_ok(
        "Status: {health}",
        "状态：{health}"
    ));
    assert!(is_zh_status_placeholder_ok(
        "Model: {model}",
        "模型：{model}"
    ));
}

#[test]
fn zh_status_placeholders_reject_mismatch_parity() {
    assert!(!is_zh_status_placeholder_ok(
        "Uptime: {seconds} sec",
        "运行时间： 秒"
    ));
    assert!(!is_zh_status_placeholder_ok(
        "Project: {project}",
        "项目：{project} {extra}"
    ));
}

// ---- formatting (3) ----

#[test]
fn zh_status_health_line_parity() {
    assert_eq!(format_zh_status_health_line("健康"), "状态：健康");
}

#[test]
fn zh_status_model_line_parity() {
    assert_eq!(format_zh_status_model_line("gpt-5"), "模型：gpt-5");
}

#[test]
fn zh_status_project_line_parity() {
    assert_eq!(format_zh_status_project_line("bot"), "项目：bot");
}
