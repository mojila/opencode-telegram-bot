//! Slice 176 parity: Russian task/goal-detail gates.
//!
//! Mirrors the pure TS contract in `src/i18n/ru.ts` (task lines 573/578/
//! 591-613, goal lines 754-799) for the 18 task-detail keys and the 26
//! translated goal-remainder keys outside `FULL_KEYS` in
//! `crate::i18n_full`:
//! - verbatim `ru` templates for task detail/run/status/cron lines and
//!   goal-remainder lines
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the task-runner and goal call sites)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! locale-agnostic detail/notice/ignore gates (`task_goal_gates`, slice 173),
//! `ru` STT lookup + formatting (`locale_ru_voice_gates`, slice 175).

use otb_core::locale_ru_task_goal_gates::*;

// ---- constants (3) ----

#[test]
fn ru_task_goal_locale_value_parity() {
    assert_eq!(RU_TASK_GOAL_LOCALE, "ru");
}

#[test]
fn ru_task_goal_keys_value_parity() {
    assert_eq!(RU_TASK_DETAIL_KEYS.len(), 18);
    assert!(RU_TASK_DETAIL_KEYS.contains(&"tasklist.details"));
    assert!(RU_TASK_DETAIL_KEYS.contains(&"task.run.error.folder_missing"));
    assert!(RU_TASK_DETAIL_KEYS.contains(&"task.kind.cron"));
    assert_eq!(RU_GOAL_DETAIL_KEYS.len(), 26);
    assert!(RU_GOAL_DETAIL_KEYS.contains(&"goal.button.clear"));
    assert!(RU_GOAL_DETAIL_KEYS.contains(&"goal.confirm.warning"));
    assert!(RU_GOAL_DETAIL_KEYS.contains(&"goal.reason.paused_by_user"));
    assert!(RU_GOAL_DETAIL_KEYS.contains(&"goal.reason.objective_limit"));
}

#[test]
fn ru_task_goal_templates_value_parity() {
    assert_eq!(RU_TASK_KIND_CRON, "повторяющаяся");
    assert_eq!(RU_TASK_KIND_ONCE, "однократная");
    assert_eq!(
        RU_TASK_RUN_SUCCESS,
        "⏰ Задача по расписанию выполнена: {description}"
    );
    assert_eq!(
        RU_TASK_RUN_ERROR_FOLDER_MISSING,
        "Папка проекта больше не существует: {path}"
    );
    assert_eq!(
        RU_GOAL_ARGUMENTS_NOT_SUPPORTED,
        "Выберите /goal без аргументов, затем следуйте подсказкам или используйте кнопки цели."
    );
    assert_eq!(RU_GOAL_CONFIRM_ROUND_LIMIT, "Лимит раундов: {rounds}.");
    assert_eq!(
        RU_GOAL_REASON_OBJECTIVE_LIMIT,
        "Цель ограничена {limit} символами (сейчас {count})."
    );
    assert_eq!(
        RU_GOAL_REASON_PAUSED_BY_USER,
        "Пауза по запросу пользователя"
    );
    assert_eq!(
        RU_GOAL_REASON_SESSION_FAILED,
        "Сессия завершилась ошибкой: {reason}"
    );
}

// ---- lookup (3) ----

#[test]
fn ru_task_goal_lookup_run_error_parity() {
    assert_eq!(
        lookup_ru_task_goal_template("task.run.error.folder_missing"),
        Some("Папка проекта больше не существует: {path}")
    );
}

#[test]
fn ru_task_goal_lookup_goal_session_failed_parity() {
    assert_eq!(
        lookup_ru_task_goal_template("goal.reason.session_failed"),
        Some("Сессия завершилась ошибкой: {reason}")
    );
}

#[test]
fn ru_task_goal_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ru_task_goal_template("task.unknown_key"), None);
    assert_eq!(lookup_ru_task_goal_template("pinned.line.goal"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ru_task_goal_placeholders_match_en_parity() {
    assert!(is_ru_task_goal_placeholder_ok(
        "Round limit: {rounds}.",
        "Лимит раундов: {rounds}."
    ));
    assert!(is_ru_task_goal_placeholder_ok(
        "The session failed: {reason}",
        "Сессия завершилась ошибкой: {reason}"
    ));
    assert!(is_ru_task_goal_placeholder_ok(
        "⏰ Scheduled task\n\nTask: {prompt}\nProject: {project}\nSchedule: {schedule}\n{cronLine}Timezone: {timezone}\nNext run: {nextRunAt}\nLast run: {lastRunAt}\nRun count: {runCount}",
        RU_TASKLIST_DETAILS
    ));
}

#[test]
fn ru_task_goal_placeholders_reject_mismatch_parity() {
    assert!(!is_ru_task_goal_placeholder_ok(
        "A goal objective is limited to {limit} characters (got {count}).",
        "Цель ограничена {limit} символами."
    ));
    assert!(!is_ru_task_goal_placeholder_ok(
        "Cron: {cron}",
        "Cron: {cron} {extra}"
    ));
}

// ---- formatting (5) ----

#[test]
fn ru_task_goal_run_success_line_parity() {
    assert_eq!(
        format_ru_task_run_success("nightly backup"),
        "⏰ Задача по расписанию выполнена: nightly backup"
    );
}

#[test]
fn ru_task_goal_run_error_line_parity() {
    assert_eq!(
        format_ru_task_run_error("nightly backup", "boom"),
        "🔴 Ошибка выполнения задачи по расписанию: nightly backup\n\nОшибка: boom"
    );
}

#[test]
fn ru_task_goal_details_cron_line_parity() {
    assert_eq!(
        format_ru_tasklist_details_cron("0 9 * * *"),
        "Cron: 0 9 * * *"
    );
}

#[test]
fn ru_task_goal_objective_limit_line_parity() {
    assert_eq!(
        format_ru_goal_objective_limit(200, 250),
        "Цель ограничена 200 символами (сейчас 250)."
    );
}

#[test]
fn ru_task_goal_confirm_round_limit_line_parity() {
    assert_eq!(format_ru_goal_confirm_round_limit(10), "Лимит раундов: 10.");
}
