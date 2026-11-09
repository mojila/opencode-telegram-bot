//! Russian task/goal-detail gates (slice 176, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/ru.ts` for the scheduled
//! task detail/run surface and the translated goal-remainder surface, NOT
//! covered by `FULL_KEYS` in `crates/otb-core/src/i18n_full.rs`:
//! - task detail block: `task.scheduling`, `task.enabled`, `task.disabled`,
//!   `task.button.enable`, `task.button.disable`, `task.toggle.saved`,
//!   `task.toggle.error`, `tasklist.details`, `tasklist.details.cron`,
//!   `task.created.cron`, `task.schedule_preview.cron`, `task.run.success`,
//!   `task.run.error`, `task.run.error.folder_missing`,
//!   `task.run.error.interactive_question`,
//!   `task.run.error.interactive_permission`, `task.kind.cron`,
//!   `task.kind.once`
//! - goal-remainder block (same 26 keys as the other locales'
//!   `*_goal2_gates`, here translated): `goal.arguments_not_supported`,
//!   `goal.input.empty`, `goal.input.reserved`, `goal.button.clear`,
//!   `goal.continue`, `goal.delete`, `goal.deleted`, `goal.unsupported`,
//!   `goal.read_error`, `goal.action_error`, `goal.confirm.warning`,
//!   `goal.confirm.round_limit`, `goal.confirm.confirm`,
//!   `goal.confirm.cancel`, `goal.notice.no_reason`,
//!   `goal.reason.paused_by_user`, `goal.reason.stopped_by_user`,
//!   `goal.reason.server_restarted`, `goal.reason.stopped_after_error`,
//!   `goal.reason.no_verdict`, `goal.reason.review_too_large`,
//!   `goal.reason.session_failed`, `goal.reason.no_active`,
//!   `goal.reason.no_paused`, `goal.reason.none_set`,
//!   `goal.reason.objective_limit`
//!
//! AUDIT (2026-11-09, branch `rust-rewrite`):
//! - `src/i18n/ru.ts` lines 573/578 carry the `task.*.cron` lines;
//!   lines 591-599 carry `task.kind.*` + `task.run.*`; lines 604-613 carry
//!   `task.scheduling` / `task.enabled` / `task.disabled` /
//!   `task.button.enable` / `task.button.disable` / `task.toggle.saved` /
//!   `task.toggle.error` / `tasklist.details` / `tasklist.details.cron`;
//!   lines 754-799 carry the 26 translated goal-remainder templates.
//!   `src/i18n/en.ts` lines 609-661 carry the task placeholder reference
//!   and lines 487-532 the goal placeholder reference.
//! - `crate::i18n` resolves every key through the build-time full table,
//!   so `t("ru", key)` already returns these templates;
//!   `crate::i18n_full` embeds `task.schedule_preview`, `task.created`,
//!   `task.retry_schedule_callback`, `task.inactive_callback` and the first
//!   22 goal keys only. No Rust owner pins the `ru` task-detail or
//!   goal-remainder verbatim templates yet.
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `task_goal_gates` (slice 173) owns only the
//!   locale-agnostic detail/notice/ignore gates (templates arrive as args).
//!   `task_callback_gates` owns only the toggle/delete allow gates and
//!   prompt truncation. The other locales' `*_goal2_gates` (slices 126,
//!   130, 134, 138, 142, 146, 150, 154, 158, 162) own only their own
//!   locale's 26 goal-remainder keys.
//! - This slice owns only the `ru` task-detail + goal-remainder lookup +
//!   placeholder-equality + line-format gates; full 12-locale x 654-key
//!   table codegen already lives in `crate::i18n` (build-time).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `ru.ts` file reads (arrive as explicit `key` args here),
//!   per-locale test looping, `t()` fallback wiring.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback (`crate::i18n` owner)
//! - `FULL_KEYS` + en/ru verbatim tables + interpolation
//!   (`crate::i18n_full` owner)
//! - placeholder set-equality / missing / extra helpers
//!   (`locale_placeholder_gates` owner, slice 122)
//! - registry labels / `getLocaleOptions` / `getLocale` precedence
//!   (`locale_registry_gates` owner, slice 121)
//! - toggle-missing outcomes, refreshed-task pick, date-label fallback,
//!   model/variant labels, cron/project blocks, dashboard goal line,
//!   notice-key pick, ignore-list guards (`task_goal_gates` owner,
//!   slice 173)
//! - `ru` STT lookup + formatting (`locale_ru_voice_gates` owner,
//!   slice 175).
//!
//! Deferred (NOT in this slice):
//! - task creation-flow prompts (`task.prompt.*`, `task.parse.*`,
//!   `task.schedule_empty`, `task.prompt_empty`, `task.parse_error`,
//!   `task.blocked.*`, `task.inactive`, `task.limit_reached`,
//!   `task.schedule_too_frequent`, `task.button.*`) and tasklist menu
//!   chrome (`tasklist.empty/select/button.*/deleted_callback/
//!   inactive_callback/load_error`) — menu-handler surface, callers
//!   localize via `crate::i18n`.
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice176_locale_ru_task_goal_parity.rs`.

/// Locale code for this table (TS: `ru` in `SUPPORTED_LOCALES`).
pub const RU_TASK_GOAL_LOCALE: &str = "ru";

/// Task-detail keys covered here (the `task.*` / `tasklist.details*` keys
/// in `src/i18n/ru.ts` outside the four task keys in `crate::i18n_full`).
pub const RU_TASK_DETAIL_KEYS: [&str; 18] = [
    "task.scheduling",
    "task.enabled",
    "task.disabled",
    "task.button.enable",
    "task.button.disable",
    "task.toggle.saved",
    "task.toggle.error",
    "tasklist.details",
    "tasklist.details.cron",
    "task.created.cron",
    "task.schedule_preview.cron",
    "task.run.success",
    "task.run.error",
    "task.run.error.folder_missing",
    "task.run.error.interactive_question",
    "task.run.error.interactive_permission",
    "task.kind.cron",
    "task.kind.once",
];

/// Goal-remainder keys covered here (the same 26 `goal.*` keys as the
/// other locales' `*_goal2_gates`, translated in `src/i18n/ru.ts`
/// lines 754-799).
pub const RU_GOAL_DETAIL_KEYS: [&str; 26] = [
    "goal.arguments_not_supported",
    "goal.input.empty",
    "goal.input.reserved",
    "goal.button.clear",
    "goal.continue",
    "goal.delete",
    "goal.deleted",
    "goal.unsupported",
    "goal.read_error",
    "goal.action_error",
    "goal.confirm.warning",
    "goal.confirm.round_limit",
    "goal.confirm.confirm",
    "goal.confirm.cancel",
    "goal.notice.no_reason",
    "goal.reason.paused_by_user",
    "goal.reason.stopped_by_user",
    "goal.reason.server_restarted",
    "goal.reason.stopped_after_error",
    "goal.reason.no_verdict",
    "goal.reason.review_too_large",
    "goal.reason.session_failed",
    "goal.reason.no_active",
    "goal.reason.no_paused",
    "goal.reason.none_set",
    "goal.reason.objective_limit",
];

/// Verbatim `ru` task-detail templates (TS: `src/i18n/ru.ts` lines
/// 573/578/591-613).
pub const RU_TASK_SCHEDULING: &str = "Расписание";
pub const RU_TASK_ENABLED: &str = "Вкл";
pub const RU_TASK_DISABLED: &str = "Выкл";
pub const RU_TASK_BUTTON_ENABLE: &str = "Включить";
pub const RU_TASK_BUTTON_DISABLE: &str = "Выключить";
pub const RU_TASK_TOGGLE_SAVED: &str = "Сохранено";
pub const RU_TASK_TOGGLE_ERROR: &str = "Не удалось сохранить";
pub const RU_TASKLIST_DETAILS: &str = "⏰ Задача по расписанию\n\nЗадача: {prompt}\nПроект: {project}\nРасписание: {schedule}\n{cronLine}Часовой пояс: {timezone}\nСледующий запуск: {nextRunAt}\nПоследний запуск: {lastRunAt}\nКоличество запусков: {runCount}";
pub const RU_TASKLIST_DETAILS_CRON: &str = "Cron: {cron}";
pub const RU_TASK_CREATED_CRON: &str = "Cron: {cron}";
pub const RU_TASK_SCHEDULE_PREVIEW_CRON: &str = "Cron: {cron}";
pub const RU_TASK_RUN_SUCCESS: &str = "⏰ Задача по расписанию выполнена: {description}";
pub const RU_TASK_RUN_ERROR: &str =
    "🔴 Ошибка выполнения задачи по расписанию: {description}\n\nОшибка: {error}";
pub const RU_TASK_RUN_ERROR_FOLDER_MISSING: &str = "Папка проекта больше не существует: {path}";
pub const RU_TASK_RUN_ERROR_INTERACTIVE_QUESTION: &str = "Задача по расписанию задала интерактивный вопрос и не может продолжить выполнение без участия пользователя.";
pub const RU_TASK_RUN_ERROR_INTERACTIVE_PERMISSION: &str = "Задача по расписанию запросила интерактивное разрешение и не может продолжить выполнение без участия пользователя.";
pub const RU_TASK_KIND_CRON: &str = "повторяющаяся";
pub const RU_TASK_KIND_ONCE: &str = "однократная";

/// Verbatim `ru` goal-remainder templates (TS: `src/i18n/ru.ts` lines
/// 754-799).
pub const RU_GOAL_ARGUMENTS_NOT_SUPPORTED: &str =
    "Выберите /goal без аргументов, затем следуйте подсказкам или используйте кнопки цели.";
pub const RU_GOAL_INPUT_EMPTY: &str =
    "Цель не может быть пустой. Отправьте её текст или нажмите «Отмена».";
pub const RU_GOAL_INPUT_RESERVED: &str =
    "Это слово зарезервировано OpenCode для управления целью. Опишите цель подробнее или нажмите «Отмена».";
pub const RU_GOAL_BUTTON_CLEAR: &str = "× Удалить";
pub const RU_GOAL_CONTINUE: &str = "▶ Продолжить";
pub const RU_GOAL_DELETE: &str = "🗑 Удалить цель";
pub const RU_GOAL_DELETED: &str = "Цель удалена. Выберите /goal, чтобы создать новую.";
pub const RU_GOAL_UNSUPPORTED: &str = "Этот сервер OpenCode не поддерживает цели сессии.";
pub const RU_GOAL_READ_ERROR: &str = "Не удалось прочитать цель сессии.";
pub const RU_GOAL_ACTION_ERROR: &str = "Не удалось изменить цель сессии.";
pub const RU_GOAL_CONFIRM_WARNING: &str = "Эта цель продолжается сама после каждого шага. Продолжение и оценка тратят токены. Потолка стоимости нет. Сервер останавливает её по своему лимиту раундов.";
pub const RU_GOAL_CONFIRM_ROUND_LIMIT: &str = "Лимит раундов: {rounds}.";
pub const RU_GOAL_CONFIRM_CONFIRM: &str = "Подтвердить";
pub const RU_GOAL_CONFIRM_CANCEL: &str = "❌ Отмена";
pub const RU_GOAL_NOTICE_NO_REASON: &str = "Причина не указана.";
pub const RU_GOAL_REASON_PAUSED_BY_USER: &str = "Пауза по запросу пользователя";
pub const RU_GOAL_REASON_STOPPED_BY_USER: &str = "Остановлено пользователем";
pub const RU_GOAL_REASON_SERVER_RESTARTED: &str = "Сервер перезапустился, пока цель была активна";
pub const RU_GOAL_REASON_STOPPED_AFTER_ERROR: &str = "Цель остановилась после ошибки";
pub const RU_GOAL_REASON_NO_VERDICT: &str = "Оценщик цели не дал пригодного вердикта";
pub const RU_GOAL_REASON_REVIEW_TOO_LARGE: &str = "Запрос оценщика цели слишком велик для модели";
pub const RU_GOAL_REASON_SESSION_FAILED: &str = "Сессия завершилась ошибкой: {reason}";
pub const RU_GOAL_REASON_NO_ACTIVE: &str = "Нет активной цели, чтобы поставить на паузу.";
pub const RU_GOAL_REASON_NO_PAUSED: &str = "Нет цели на паузе, чтобы возобновить.";
pub const RU_GOAL_REASON_NONE_SET: &str = "Цель не задана.";
pub const RU_GOAL_REASON_OBJECTIVE_LIMIT: &str =
    "Цель ограничена {limit} символами (сейчас {count}).";

/// Verbatim `ru` task-detail / goal-remainder template for a key, or
/// `None` outside this slice (TS: `ru[key]` for the keys above).
pub fn lookup_ru_task_goal_template(key: &str) -> Option<&'static str> {
    match key {
        "task.scheduling" => Some(RU_TASK_SCHEDULING),
        "task.enabled" => Some(RU_TASK_ENABLED),
        "task.disabled" => Some(RU_TASK_DISABLED),
        "task.button.enable" => Some(RU_TASK_BUTTON_ENABLE),
        "task.button.disable" => Some(RU_TASK_BUTTON_DISABLE),
        "task.toggle.saved" => Some(RU_TASK_TOGGLE_SAVED),
        "task.toggle.error" => Some(RU_TASK_TOGGLE_ERROR),
        "tasklist.details" => Some(RU_TASKLIST_DETAILS),
        "tasklist.details.cron" => Some(RU_TASKLIST_DETAILS_CRON),
        "task.created.cron" => Some(RU_TASK_CREATED_CRON),
        "task.schedule_preview.cron" => Some(RU_TASK_SCHEDULE_PREVIEW_CRON),
        "task.run.success" => Some(RU_TASK_RUN_SUCCESS),
        "task.run.error" => Some(RU_TASK_RUN_ERROR),
        "task.run.error.folder_missing" => Some(RU_TASK_RUN_ERROR_FOLDER_MISSING),
        "task.run.error.interactive_question" => Some(RU_TASK_RUN_ERROR_INTERACTIVE_QUESTION),
        "task.run.error.interactive_permission" => Some(RU_TASK_RUN_ERROR_INTERACTIVE_PERMISSION),
        "task.kind.cron" => Some(RU_TASK_KIND_CRON),
        "task.kind.once" => Some(RU_TASK_KIND_ONCE),
        "goal.arguments_not_supported" => Some(RU_GOAL_ARGUMENTS_NOT_SUPPORTED),
        "goal.input.empty" => Some(RU_GOAL_INPUT_EMPTY),
        "goal.input.reserved" => Some(RU_GOAL_INPUT_RESERVED),
        "goal.button.clear" => Some(RU_GOAL_BUTTON_CLEAR),
        "goal.continue" => Some(RU_GOAL_CONTINUE),
        "goal.delete" => Some(RU_GOAL_DELETE),
        "goal.deleted" => Some(RU_GOAL_DELETED),
        "goal.unsupported" => Some(RU_GOAL_UNSUPPORTED),
        "goal.read_error" => Some(RU_GOAL_READ_ERROR),
        "goal.action_error" => Some(RU_GOAL_ACTION_ERROR),
        "goal.confirm.warning" => Some(RU_GOAL_CONFIRM_WARNING),
        "goal.confirm.round_limit" => Some(RU_GOAL_CONFIRM_ROUND_LIMIT),
        "goal.confirm.confirm" => Some(RU_GOAL_CONFIRM_CONFIRM),
        "goal.confirm.cancel" => Some(RU_GOAL_CONFIRM_CANCEL),
        "goal.notice.no_reason" => Some(RU_GOAL_NOTICE_NO_REASON),
        "goal.reason.paused_by_user" => Some(RU_GOAL_REASON_PAUSED_BY_USER),
        "goal.reason.stopped_by_user" => Some(RU_GOAL_REASON_STOPPED_BY_USER),
        "goal.reason.server_restarted" => Some(RU_GOAL_REASON_SERVER_RESTARTED),
        "goal.reason.stopped_after_error" => Some(RU_GOAL_REASON_STOPPED_AFTER_ERROR),
        "goal.reason.no_verdict" => Some(RU_GOAL_REASON_NO_VERDICT),
        "goal.reason.review_too_large" => Some(RU_GOAL_REASON_REVIEW_TOO_LARGE),
        "goal.reason.session_failed" => Some(RU_GOAL_REASON_SESSION_FAILED),
        "goal.reason.no_active" => Some(RU_GOAL_REASON_NO_ACTIVE),
        "goal.reason.no_paused" => Some(RU_GOAL_REASON_NO_PAUSED),
        "goal.reason.none_set" => Some(RU_GOAL_REASON_NONE_SET),
        "goal.reason.objective_limit" => Some(RU_GOAL_REASON_OBJECTIVE_LIMIT),
        _ => None,
    }
}

fn is_placeholder_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn placeholder_set(template: &str) -> std::collections::BTreeSet<String> {
    let bytes = template.as_bytes();
    let mut out = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && is_placeholder_char(bytes[j]) {
                j += 1;
            }
            if j > i + 1 && j < bytes.len() && bytes[j] == b'}' {
                out.insert(template[i..=j].to_string());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn interpolate_ru(template: &str, params: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < template.len() {
        let rest = &template[i..];
        if rest.starts_with('{') {
            if let Some(end) = rest.find('}') {
                let name = &rest[1..end];
                if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    match params.iter().find(|(k, _)| *k == name) {
                        Some((_, value)) => out.push_str(value),
                        None => out.push_str(&rest[..=end]),
                    }
                    i += end + 1;
                    continue;
                }
            }
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `true` when the `ru` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_ru_task_goal_placeholder_ok(en_template: &str, ru_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(ru_template)
}

/// `⏰ Задача по расписанию выполнена: {description}` with the task
/// description filled in (TS: `t("task.run.success", { description })`
/// call site in the scheduled-task runner).
pub fn format_ru_task_run_success(description: &str) -> String {
    interpolate_ru(RU_TASK_RUN_SUCCESS, &[("description", description)])
}

/// `🔴 Ошибка выполнения задачи по расписанию: ...` with description and
/// error filled in (TS: `t("task.run.error", { description, error })`
/// call site in the scheduled-task runner).
pub fn format_ru_task_run_error(description: &str, error: &str) -> String {
    interpolate_ru(
        RU_TASK_RUN_ERROR,
        &[("description", description), ("error", error)],
    )
}

/// `Cron: {cron}` line appended under task detail blocks for recurring
/// schedules (TS: `t("tasklist.details.cron", { cron })` call site in
/// `formatTaskDetails`).
pub fn format_ru_tasklist_details_cron(cron: &str) -> String {
    interpolate_ru(RU_TASKLIST_DETAILS_CRON, &[("cron", cron)])
}

/// `Цель ограничена {limit} символами (сейчас {count}).` with counts
/// filled in (TS: `t("goal.reason.objective_limit", { limit, count })`
/// call site).
pub fn format_ru_goal_objective_limit(limit: usize, count: usize) -> String {
    let limit = limit.to_string();
    let count = count.to_string();
    interpolate_ru(
        RU_GOAL_REASON_OBJECTIVE_LIMIT,
        &[("limit", &limit), ("count", &count)],
    )
}

/// `Лимит раундов: {rounds}.` with the round limit filled in
/// (TS: `t("goal.confirm.round_limit", { rounds })` call site).
pub fn format_ru_goal_confirm_round_limit(rounds: u32) -> String {
    let rounds = rounds.to_string();
    interpolate_ru(RU_GOAL_CONFIRM_ROUND_LIMIT, &[("rounds", &rounds)])
}
