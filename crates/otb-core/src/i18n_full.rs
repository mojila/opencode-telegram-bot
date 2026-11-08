//! Expanded i18n coverage: representative key tables + formatting-logic stubs.
//!
//! Mirrors the representative `t("key", params)` call sites of the TS bot
//! (`src/bot/` keyboards, prompt queue, goal flow, status command, error
//! paths, multi-param prompts) plus the `tests/i18n/*` contract surface.
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/en.ts` defines **654** `I18nKey` entries (key lines 2..787;
//!   `projectSearchFallback` re-exports 14 aliases and is not counted).
//! - `crates/otb-core/src/i18n/mod.rs` embeds only **8** unique keys for all
//!   12 locales (`bot.session_created`, `status.line.health`,
//!   `progress.compact.count.tools`, `common.cancelled`,
//!   `agent.change_error_callback`, `variant.change_error_callback`,
//!   `permission.inactive_callback`, `question.inactive_callback`).
//!   Coverage before this module: 8/654 (~1.2%).
//! - This module adds **71** representative keys (en + ru verbatim tables),
//!   raising table coverage to 79/654 (~12%), still scoped to en/ru.
//!
//! SCOPE (GREEN, cycle 33b): only `en` + `ru` templates are embedded verbatim
//! from `src/i18n/en.ts` / `src/i18n/ru.ts` so `{placeholder}` sets can be
//! asserted for two locales. The other 10 locales fall back to `en` in
//! [`t_full`]. The formatting functions below implement TS `t(key, params)`
//! interpolation call sites (keyboard labels, queued prompts, goal, status,
//! errors, prompts) via [`render_full`]; full 12-locale codegen over all 654
//! `I18nKey` entries remains future work.
//!
//! Every `slice33b_*` test in `tests/i18n_full_parity.rs` passes against the
//! verbatim en/ru tables plus the interpolation logic below.

/// Locales with verbatim tables in this module. The remaining
/// `SUPPORTED_LOCALES` entries fall back to `en` (see [`t_full`]).
pub const FULL_TABLE_LOCALES: [&str; 2] = ["en", "ru"];

/// Representative keyboard-label keys (`src/bot/keyboards/*`, reply markup).
pub const KEYBOARD_KEYS: &[&str] = &[
    "keyboard.agent_mode",
    "keyboard.context",
    "keyboard.context_empty",
    "keyboard.variant",
    "keyboard.variant_default",
    "keyboard.queued_prompt",
];

/// Prompt-queue keys (`src/bot/handlers/prompt-queue-dispatch.ts`, queue UI).
pub const QUEUE_KEYS: &[&str] = &[
    "queue.added",
    "queue.full",
    "queue.media_limit",
    "queue.removed",
    "queue.not_found",
    "queue.steer_added",
    "queue.disabled_hint",
    "keyboard.updated",
];

/// Goal-flow keys (`src/bot/callbacks/goal-callback-handler.ts`, `/goal`).
pub const GOAL_KEYS: &[&str] = &[
    "pinned.line.goal",
    "goal.none",
    "goal.input.prompt",
    "goal.button.details",
    "goal.button.pause",
    "goal.button.resume",
    "goal.start",
    "goal.too_long",
    "goal.status.active",
    "goal.status.paused",
    "goal.status.complete",
    "goal.status.rounds",
    "goal.status.updated",
    "goal.confirm.start",
    "goal.confirm.resume",
    "goal.notice.paused",
    "goal.notice.complete",
    "goal.abort.paused",
    "goal.abort.failed",
    "goal.reason.round_limit",
    "goal.reason.wait_limit",
    "goal.reason.not_progress",
];

/// Status-command keys (`src/bot/commands/status-command.ts`).
pub const STATUS_KEYS: &[&str] = &[
    "status.line.health",
    "status.line.version",
    "status.line.pid",
    "status.line.uptime_sec",
    "status.line.mode",
    "status.line.model",
    "status.project_selected",
    "status.session_selected",
    "status.header_running",
    "status.health.healthy",
];

/// Error-path keys (`bot.*` handlers, `error.generic`).
pub const ERROR_KEYS: &[&str] = &[
    "error.generic",
    "bot.session_error",
    "bot.unknown_command",
    "bot.photo_too_large",
    "bot.file_too_large",
    "bot.project_folder_missing",
    "bot.session_retry",
    "bot.prompt_send_error",
    "bot.create_session_error",
    "bot.text_file_too_large",
];

/// Multi-param prompt keys (rename, scheduled tasks, polls, permissions,
/// context compaction, progress rendering).
pub const PROMPT_KEYS: &[&str] = &[
    "rename.prompt",
    "rename.success",
    "task.schedule_preview",
    "task.created",
    "question.summary.question",
    "question.summary.answer",
    "permission.header",
    "context.confirm_text",
    "progress.compact.activity",
    "context.details.window",
];

/// Extra callback-answer keys covered here (Telegram 200-char budget, cf.
/// `tests/i18n/callback-answer-length.test.ts`).
pub const FULL_CALLBACK_KEYS: &[&str] = &[
    "rename.inactive_callback",
    "task.inactive_callback",
    "commands.execute_callback",
    "task.retry_schedule_callback",
    "question.cancelled",
];

/// Full representative key set for this module (71 keys).
pub const FULL_KEYS: &[&str] = &[
    // KEYBOARD_KEYS (6)
    "keyboard.agent_mode",
    "keyboard.context",
    "keyboard.context_empty",
    "keyboard.variant",
    "keyboard.variant_default",
    "keyboard.queued_prompt",
    // QUEUE_KEYS (8)
    "queue.added",
    "queue.full",
    "queue.media_limit",
    "queue.removed",
    "queue.not_found",
    "queue.steer_added",
    "queue.disabled_hint",
    "keyboard.updated",
    // GOAL_KEYS (22)
    "pinned.line.goal",
    "goal.none",
    "goal.input.prompt",
    "goal.button.details",
    "goal.button.pause",
    "goal.button.resume",
    "goal.start",
    "goal.too_long",
    "goal.status.active",
    "goal.status.paused",
    "goal.status.complete",
    "goal.status.rounds",
    "goal.status.updated",
    "goal.confirm.start",
    "goal.confirm.resume",
    "goal.notice.paused",
    "goal.notice.complete",
    "goal.abort.paused",
    "goal.abort.failed",
    "goal.reason.round_limit",
    "goal.reason.wait_limit",
    "goal.reason.not_progress",
    // STATUS_KEYS (10)
    "status.line.health",
    "status.line.version",
    "status.line.pid",
    "status.line.uptime_sec",
    "status.line.mode",
    "status.line.model",
    "status.project_selected",
    "status.session_selected",
    "status.header_running",
    "status.health.healthy",
    // ERROR_KEYS (10)
    "error.generic",
    "bot.session_error",
    "bot.unknown_command",
    "bot.photo_too_large",
    "bot.file_too_large",
    "bot.project_folder_missing",
    "bot.session_retry",
    "bot.prompt_send_error",
    "bot.create_session_error",
    "bot.text_file_too_large",
    // PROMPT_KEYS (10)
    "rename.prompt",
    "rename.success",
    "task.schedule_preview",
    "task.created",
    "question.summary.question",
    "question.summary.answer",
    "permission.header",
    "context.confirm_text",
    "progress.compact.activity",
    "context.details.window",
    // FULL_CALLBACK_KEYS (5)
    "rename.inactive_callback",
    "task.inactive_callback",
    "commands.execute_callback",
    "task.retry_schedule_callback",
    "question.cancelled",
];

/// Verbatim template lookup for the representative key set.
/// Strings are copied verbatim from `src/i18n/en.ts` / `src/i18n/ru.ts`.
/// Returns `None` for locales/keys outside this RED scope (other locales
/// fall back to `en` in [`t_full`]; full 12-locale coverage is GREEN work).
fn lookup_full(locale: &str, key: &str) -> Option<&'static str> {
    match (locale, key) {
        // --- en: keyboard ---
        ("en", "keyboard.agent_mode") => Some("{emoji} {name} Agent"),
        ("en", "keyboard.context") => Some("📊 {used} / {limit} ({percent}%)"),
        ("en", "keyboard.context_empty") => Some("📊 0"),
        ("en", "keyboard.variant") => Some("💭 {name}"),
        ("en", "keyboard.variant_default") => Some("💡 Default"),
        ("en", "keyboard.queued_prompt") => Some("❌ {index}. {text}"),
        // --- en: queue ---
        ("en", "queue.added") => Some(
            "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        ),
        ("en", "queue.full") => Some(
            "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        ),
        ("en", "queue.media_limit") => Some(
            "⚠️ Queued media is limited to {maxSizeMb} MiB. Wait for an item to send, then try again.",
        ),
        ("en", "queue.removed") => Some("🗑 Message removed from the queue."),
        ("en", "queue.not_found") => Some("This message is no longer in the queue."),
        ("en", "queue.steer_added") => Some("📥 Added to the current task ({count}/{max})."),
        ("en", "queue.disabled_hint") => {
            Some("The message queue can be enabled in /settings.")
        }
        ("en", "keyboard.updated") => Some("⌨️ Keyboard updated"),
        // --- en: goal ---
        ("en", "pinned.line.goal") => Some("◎ Goal: {status} — {objective}"),
        ("en", "goal.none") => Some(
            "No goal is set.\n\nUse /goal, then send the objective in the next message.",
        ),
        ("en", "goal.input.prompt") => Some(
            "Send the objective for this session in your next text message. Describe the result to achieve.",
        ),
        ("en", "goal.button.details") => Some("◎ {status} · {objective}"),
        ("en", "goal.button.pause") => Some("⏸ Pause"),
        ("en", "goal.button.resume") => Some("▶ Continue"),
        ("en", "goal.start") => Some("✅ Start"),
        ("en", "goal.too_long") => {
            Some("A goal is limited to {limit} characters (got {count}).")
        }
        ("en", "goal.status.active") => Some("active"),
        ("en", "goal.status.paused") => Some("paused"),
        ("en", "goal.status.complete") => Some("complete"),
        ("en", "goal.status.rounds") => Some("{rounds}/{maxRounds}"),
        ("en", "goal.status.updated") => Some("Updated {time}"),
        ("en", "goal.confirm.start") => Some("Start this goal?\n\n{objective}"),
        ("en", "goal.confirm.resume") => Some("Resume this goal?\n\n{objective}"),
        ("en", "goal.notice.paused") => Some("◎ Goal {status}\n{reason}"),
        ("en", "goal.notice.complete") => Some("◎ Goal {status}\n{reason}"),
        ("en", "goal.abort.paused") => Some("Goal paused: {reason}"),
        ("en", "goal.abort.failed") => Some("Goal pause failed: {reason}"),
        ("en", "goal.reason.round_limit") => Some("Reached the limit of {rounds} rounds"),
        ("en", "goal.reason.wait_limit") => Some(
            "Nothing ran for {minutes} minutes while the goal waited on background work",
        ),
        ("en", "goal.reason.not_progress") => Some("Not making progress: {reason}"),
        // --- en: status ---
        ("en", "status.line.health") => Some("Status: {health}"),
        ("en", "status.line.version") => Some("OpenCode version: {version}"),
        ("en", "status.line.pid") => Some("PID: {pid}"),
        ("en", "status.line.uptime_sec") => Some("Uptime: {seconds} sec"),
        ("en", "status.line.mode") => Some("Agent: {mode}"),
        ("en", "status.line.model") => Some("Model: {model}"),
        ("en", "status.project_selected") => Some("Project: {project}"),
        ("en", "status.session_selected") => Some("Current session: {title}"),
        ("en", "status.header_running") => Some("🟢 OpenCode Server is running"),
        ("en", "status.health.healthy") => Some("Healthy"),
        // --- en: errors ---
        ("en", "error.generic") => Some("🔴 Something went wrong."),
        ("en", "bot.session_error") => Some("🔴 OpenCode returned an error: {message}"),
        ("en", "bot.unknown_command") => Some(
            "⚠️ Unknown command: {command}. Use /help to see available commands.",
        ),
        ("en", "bot.photo_too_large") => Some("⚠️ Photo is too large (max {maxSizeMb}MB)"),
        ("en", "bot.file_too_large") => Some("⚠️ File is too large (max {maxSizeMb}MB)"),
        ("en", "bot.project_folder_missing") => Some(
            "🚫 The project folder no longer exists: {path}. Choose another project in /projects.",
        ),
        ("en", "bot.session_retry") => Some(
            "🔁 {message}\n\nProvider keeps returning the same error on repeated retries. Use /abort to abort.",
        ),
        ("en", "bot.prompt_send_error") => Some("Failed to send request to OpenCode."),
        ("en", "bot.create_session_error") => Some(
            "🔴 Failed to create session. Try /new or check server status with /status.",
        ),
        ("en", "bot.text_file_too_large") => {
            Some("⚠️ Text file is too large (max {maxSizeKb}KB)")
        }
        // --- en: prompts ---
        ("en", "rename.prompt") => {
            Some("📝 Enter new title for session:\n\nCurrent: {title}")
        }
        ("en", "rename.success") => Some("✅ Session renamed to: {title}"),
        ("en", "task.schedule_preview") => Some(
            "✅ Schedule parsed\n\nHow I understood it: {summary}\n{cronLine}Timezone: {timezone}\nType: {kind}\nNext run: {nextRunAt}",
        ),
        ("en", "task.created") => Some(
            "✅ Scheduled task created\n\nTask: {description}\nProject: {project}\nAgent: {agent}\nModel: {model}\nSchedule: {schedule}\n{cronLine}Next run: {nextRunAt}",
        ),
        ("en", "question.summary.question") => Some("Question {index}:\n{question}\n\n"),
        ("en", "question.summary.answer") => Some("Answer:\n{answer}\n\n"),
        ("en", "permission.header") => Some("{emoji} Permission request: {name}\n\n"),
        ("en", "context.confirm_text") => Some(
            "📊 Context compaction for session \"{title}\"\n\nThis will reduce context usage by removing old messages from history.\n\nContinue?",
        ),
        ("en", "progress.compact.activity") => Some("{header}\n{activity}"),
        ("en", "context.details.window") => Some("Window: {used} / {limit} ({percent}%)"),
        // --- en: extra callback answers ---
        ("en", "rename.inactive_callback") => Some("Rename request is inactive"),
        ("en", "task.inactive_callback") => Some("This scheduled task flow is inactive"),
        ("en", "commands.execute_callback") => Some("Executing command..."),
        ("en", "task.retry_schedule_callback") => Some("Re-entering schedule..."),
        ("en", "question.cancelled") => Some("❌ Poll cancelled"),
        // --- ru: keyboard ---
        ("ru", "keyboard.agent_mode") => Some("{emoji} {name} Agent"),
        ("ru", "keyboard.context") => Some("📊 {used} / {limit} ({percent}%)"),
        ("ru", "keyboard.context_empty") => Some("📊 0"),
        ("ru", "keyboard.variant") => Some("💭 {name}"),
        ("ru", "keyboard.variant_default") => Some("💡 Default"),
        ("ru", "keyboard.queued_prompt") => Some("❌ {index}. {text}"),
        // --- ru: queue ---
        ("ru", "queue.added") => Some(
            "📥 Добавлено в очередь ({count}/{max}). Сообщение уйдёт после завершения текущей задачи.",
        ),
        ("ru", "queue.full") => Some(
            "⚠️ Очередь заполнена ({max}). Удалите сообщение или дождитесь завершения текущей задачи.",
        ),
        ("ru", "queue.media_limit") => Some(
            "⚠️ Медиа в очереди ограничены {maxSizeMb} MiB. Дождитесь отправки элемента и повторите попытку.",
        ),
        ("ru", "queue.removed") => Some("🗑 Сообщение удалено из очереди."),
        ("ru", "queue.not_found") => Some("Этого сообщения больше нет в очереди."),
        ("ru", "queue.steer_added") => Some("📥 Добавлено в текущую задачу ({count}/{max})."),
        ("ru", "queue.disabled_hint") => Some("Очередь сообщений включается в /settings."),
        ("ru", "keyboard.updated") => Some("⌨️ Клавиатура обновлена"),
        // --- ru: goal ---
        ("ru", "pinned.line.goal") => Some("◎ Цель: {status} — {objective}"),
        ("ru", "goal.none") => Some(
            "Цель не задана.\n\nВыберите /goal, затем отправьте цель следующим сообщением.",
        ),
        ("ru", "goal.input.prompt") => Some(
            "Отправьте цель для этой сессии следующим текстовым сообщением. Опишите, какого результата нужно достичь.",
        ),
        ("ru", "goal.button.details") => Some("◎ {status} · {objective}"),
        ("ru", "goal.button.pause") => Some("⏸ Пауза"),
        ("ru", "goal.button.resume") => Some("▶ Продолжить"),
        ("ru", "goal.start") => Some("✅ Запустить"),
        ("ru", "goal.too_long") => Some("Цель ограничена {limit} символами (сейчас {count})."),
        ("ru", "goal.status.active") => Some("активна"),
        ("ru", "goal.status.paused") => Some("пауза"),
        ("ru", "goal.status.complete") => Some("достигнута"),
        ("ru", "goal.status.rounds") => Some("{rounds}/{maxRounds}"),
        ("ru", "goal.status.updated") => Some("Обновлено {time}"),
        ("ru", "goal.confirm.start") => Some("Запустить эту цель?\n\n{objective}"),
        ("ru", "goal.confirm.resume") => Some("Возобновить эту цель?\n\n{objective}"),
        ("ru", "goal.notice.paused") => Some("◎ Цель {status}\n{reason}"),
        ("ru", "goal.notice.complete") => Some("◎ Цель {status}\n{reason}"),
        ("ru", "goal.abort.paused") => Some("Цель на паузе: {reason}"),
        ("ru", "goal.abort.failed") => Some("Не удалось поставить цель на паузу: {reason}"),
        ("ru", "goal.reason.round_limit") => Some("Достигнут лимит в {rounds} раундов"),
        ("ru", "goal.reason.wait_limit") => Some(
            "За {minutes} минут ничего не выполнялось, пока цель ждала фоновую работу",
        ),
        ("ru", "goal.reason.not_progress") => Some("Нет прогресса: {reason}"),
        // --- ru: status ---
        ("ru", "status.line.health") => Some("Статус: {health}"),
        ("ru", "status.line.version") => Some("Версия OpenCode: {version}"),
        ("ru", "status.line.pid") => Some("PID: {pid}"),
        ("ru", "status.line.uptime_sec") => Some("Uptime: {seconds} сек"),
        ("ru", "status.line.mode") => Some("Агент: {mode}"),
        ("ru", "status.line.model") => Some("Модель: {model}"),
        ("ru", "status.project_selected") => Some("Проект: {project}"),
        ("ru", "status.session_selected") => Some("Текущая сессия: {title}"),
        ("ru", "status.header_running") => Some("🟢 OpenCode Server запущен"),
        ("ru", "status.health.healthy") => Some("Healthy"),
        // --- ru: errors ---
        ("ru", "error.generic") => Some("🔴 Произошла ошибка."),
        ("ru", "bot.session_error") => Some("🔴 OpenCode вернул ошибку: {message}"),
        ("ru", "bot.unknown_command") => Some(
            "⚠️ Неизвестная команда: {command}. Используйте /help для списка команд.",
        ),
        ("ru", "bot.photo_too_large") => Some("⚠️ Фото слишком большое (макс. {maxSizeMb}МБ)"),
        ("ru", "bot.file_too_large") => Some("⚠️ Файл слишком большой (макс. {maxSizeMb}МБ)"),
        ("ru", "bot.project_folder_missing") => Some(
            "🚫 Папка проекта больше не существует: {path}. Выберите другой проект в /projects.",
        ),
        ("ru", "bot.session_retry") => Some(
            "🔁 {message}\n\nПровайдер возвращает одну и ту же ошибку при повторных запросах. Используйте /abort для остановки.",
        ),
        ("ru", "bot.prompt_send_error") => Some("Не удалось отправить запрос в OpenCode."),
        ("ru", "bot.create_session_error") => Some(
            "🔴 Не удалось создать сессию. Попробуйте команду /new или проверьте статус сервера /status.",
        ),
        ("ru", "bot.text_file_too_large") => {
            Some("⚠️ Текстовый файл слишком большой (макс. {maxSizeKb}КБ)")
        }
        // --- ru: prompts ---
        ("ru", "rename.prompt") => {
            Some("📝 Введите новое название сессии:\n\nТекущее: {title}")
        }
        ("ru", "rename.success") => Some("✅ Сессия переименована в: {title}"),
        ("ru", "task.schedule_preview") => Some(
            "✅ Расписание распознано\n\nКак я понял: {summary}\n{cronLine}Часовой пояс: {timezone}\nТип: {kind}\nСледующий запуск: {nextRunAt}",
        ),
        ("ru", "task.created") => Some(
            "✅ Задача по расписанию создана\n\nЗадача: {description}\nПроект: {project}\nАгент: {agent}\nМодель: {model}\nРасписание: {schedule}\n{cronLine}Следующий запуск: {nextRunAt}",
        ),
        ("ru", "question.summary.question") => Some("Вопрос {index}:\n{question}\n\n"),
        ("ru", "question.summary.answer") => Some("Ответ:\n{answer}\n\n"),
        ("ru", "permission.header") => Some("{emoji} Запрос разрешения: {name}\n\n"),
        ("ru", "context.confirm_text") => Some(
            "📊 Сжатие контекста для сессии \"{title}\"\n\nЭто уменьшит использование контекста, удалив старые сообщения из истории.\n\nПродолжить?",
        ),
        ("ru", "progress.compact.activity") => Some("{header}\n{activity}"),
        ("ru", "context.details.window") => Some("Окно: {used} / {limit} ({percent}%)"),
        // --- ru: extra callback answers ---
        ("ru", "rename.inactive_callback") => Some("Запрос переименования неактивен"),
        ("ru", "task.inactive_callback") => Some("Этот сценарий создания задачи уже неактивен"),
        ("ru", "commands.execute_callback") => Some("Запускаю команду..."),
        ("ru", "task.retry_schedule_callback") => Some("Возвращаю ввод периода..."),
        ("ru", "question.cancelled") => Some("❌ Опрос отменен"),
        _ => None,
    }
}

/// Raw template lookup for the expanded key set (no interpolation).
/// Mirrors `t(key)` without `params` in `src/i18n/index.ts`.
pub fn lookup_template(locale: &str, key: &str) -> Option<&'static str> {
    lookup_full(locale, key)
}

/// Translate `key` for `locale` with `en` fallback, then key fallback.
/// Mirrors `t()` in `src/i18n/index.ts` (template only; interpolation is the
/// GREEN `render_full` stub below).
pub fn t_full(locale: &str, key: &str) -> String {
    if let Some(template) = lookup_full(locale, key) {
        return template.to_string();
    }
    if locale != "en" {
        if let Some(template) = lookup_full("en", key) {
            return template.to_string();
        }
    }
    key.to_string()
}

// Formatting helpers (GREEN): TS-faithful `t(key, params)` interpolation.
//
// Mirrors `interpolate` in `src/i18n/index.ts`: every `{name}` placeholder
// (`[A-Za-z0-9_]+`) is replaced by the matching param; placeholders without
// a param are left verbatim. Truncation mirrors the TS call sites
// (`buildPreview` in `queued-prompt-button.ts`, `truncateGoalObjective` in
// `app/formatters/goal-text.ts`); both leave short strings untouched.
fn interpolate_full(template: &str, params: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < template.len() {
        let rest = &template[i..];
        if rest.starts_with('{') {
            if let Some(end) = rest.find('}') {
                let name = &rest[1..end];
                if !name.is_empty()
                    && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
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

/// Collapse all whitespace runs to single spaces and trim, mirroring
/// `text.replace(/\s+/g, " ").trim()` at the TS call sites.
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Queued-prompt preview: collapse whitespace, keep at most `limit` chars
/// (UTF-16 code units in TS, scalar values here — identical for the
/// representative templates), appending `…`. Mirrors `buildPreview`.
fn truncate_preview(text: &str, limit: usize) -> String {
    let collapsed = collapse_whitespace(text);
    if collapsed.chars().count() <= limit {
        return collapsed;
    }
    let kept: String = collapsed.chars().take(limit.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// Goal-objective truncation: flatten, keep at most `limit` chars, trim
/// trailing spaces, append `…`. Mirrors `truncateGoalObjective`.
fn truncate_goal_objective(text: &str, limit: usize) -> String {
    const DASHBOARD_OBJECTIVE_LIMIT: usize = 120;
    let limit = if limit == 0 {
        DASHBOARD_OBJECTIVE_LIMIT
    } else {
        limit
    };
    let flat = collapse_whitespace(text);
    if flat.chars().count() <= limit {
        return flat;
    }
    let kept: String = flat.chars().take(limit.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end())
}

// ---------------------------------------------------------------------------
// Formatting logic (GREEN): each mirrors a TS `t(key, params)` call site
// with concrete params via `render_full` below.
// ---------------------------------------------------------------------------

/// Reply-keyboard agent-mode button (`keyboard.agent_mode` + params).
pub fn keyboard_agent_label(_locale: &str, _emoji: &str, _name: &str) -> String {
    render_full(_locale, "keyboard.agent_mode", &[("emoji", _emoji), ("name", _name)])
}

/// Reply-keyboard context button (`keyboard.context` + params).
pub fn keyboard_context_label(
    _locale: &str,
    _used: &str,
    _limit: &str,
    _percent: u32,
) -> String {
    let percent = _percent.to_string();
    render_full(
        _locale,
        "keyboard.context",
        &[("used", _used), ("limit", _limit), ("percent", &percent)],
    )
}

/// Reply-keyboard variant button (`keyboard.variant` + params).
pub fn keyboard_variant_label(_locale: &str, _name: &str) -> String {
    render_full(_locale, "keyboard.variant", &[("name", _name)])
}

/// Queued-prompt keyboard row (`keyboard.queued_prompt` + params).
pub fn queued_prompt_label(_locale: &str, _index: usize, _text: &str) -> String {
    let index = _index.to_string();
    let preview = truncate_preview(_text, 28);
    render_full(_locale, "keyboard.queued_prompt", &[("index", &index), ("text", &preview)])
}

/// Queue acknowledgement (`queue.added` + params).
pub fn queue_added(_locale: &str, _count: u32, _max: u32) -> String {
    let count = _count.to_string();
    let max = _max.to_string();
    render_full(_locale, "queue.added", &[("count", &count), ("max", &max)])
}

/// Queue-full warning (`queue.full` + params).
pub fn queue_full(_locale: &str, _max: u32) -> String {
    let max = _max.to_string();
    render_full(_locale, "queue.full", &[("max", &max)])
}

/// Queued-media limit warning (`queue.media_limit` + params).
pub fn queue_media_limit(_locale: &str, _max_size_mb: u32) -> String {
    let max = _max_size_mb.to_string();
    render_full(_locale, "queue.media_limit", &[("maxSizeMb", &max)])
}

/// Steer-mode acknowledgement (`queue.steer_added` + params).
pub fn queue_steer_added(_locale: &str, _count: u32, _max: u32) -> String {
    let count = _count.to_string();
    let max = _max.to_string();
    render_full(_locale, "queue.steer_added", &[("count", &count), ("max", &max)])
}

/// Pinned goal line (`pinned.line.goal` + params).
pub fn goal_status_line(_locale: &str, _status: &str, _objective: &str) -> String {
    let objective = truncate_goal_objective(_objective, 120);
    render_full(_locale, "pinned.line.goal", &[("status", _status), ("objective", &objective)])
}

/// Localised goal-status word (`goal.status.*` by status key).
pub fn goal_status_label(_locale: &str, _status: &str) -> String {
    // Mirrors `formatGoalStatus`: paused/complete map to their word, anything
    // else falls back to the active word.
    match _status {
        "paused" => t_full(_locale, "goal.status.paused"),
        "complete" => t_full(_locale, "goal.status.complete"),
        _ => t_full(_locale, "goal.status.active"),
    }
}

/// Goal start confirmation (`goal.confirm.start` + params).
pub fn goal_confirm_start(_locale: &str, _objective: &str) -> String {
    render_full(_locale, "goal.confirm.start", &[("objective", _objective)])
}

/// Goal resume confirmation (`goal.confirm.resume` + params).
pub fn goal_confirm_resume(_locale: &str, _objective: &str) -> String {
    render_full(_locale, "goal.confirm.resume", &[("objective", _objective)])
}

/// Goal length error (`goal.too_long` + params).
pub fn goal_too_long(_locale: &str, _limit: usize, _count: usize) -> String {
    let limit = _limit.to_string();
    let count = _count.to_string();
    render_full(_locale, "goal.too_long", &[("limit", &limit), ("count", &count)])
}

/// Goal notice (`goal.notice.paused` / `goal.notice.complete` + params).
pub fn goal_notice(_locale: &str, _completed: bool, _status: &str, _reason: &str) -> String {
    // Mirrors `formatGoalNotice`: complete picks `goal.notice.complete`,
    // anything else picks `goal.notice.paused`.
    let key = if _completed {
        "goal.notice.complete"
    } else {
        "goal.notice.paused"
    };
    render_full(_locale, key, &[("status", _status), ("reason", _reason)])
}

/// Goal abort outcome (`goal.abort.paused` / `goal.abort.failed` + params).
pub fn goal_abort(_locale: &str, _paused: bool, _reason: &str) -> String {
    let key = if _paused {
        "goal.abort.paused"
    } else {
        "goal.abort.failed"
    };
    render_full(_locale, key, &[("reason", _reason)])
}

/// Goal round-limit reason (`goal.reason.round_limit` + params).
pub fn goal_reason_round_limit(_locale: &str, _rounds: u32) -> String {
    let rounds = _rounds.to_string();
    render_full(_locale, "goal.reason.round_limit", &[("rounds", &rounds)])
}

/// Goal round counter (`goal.status.rounds` + params).
pub fn goal_rounds(_locale: &str, _rounds: u32, _max_rounds: u32) -> String {
    let rounds = _rounds.to_string();
    let max = _max_rounds.to_string();
    render_full(_locale, "goal.status.rounds", &[("rounds", &rounds), ("maxRounds", &max)])
}

/// Goal details button (`goal.button.details` + params).
pub fn goal_button_details(_locale: &str, _status: &str, _objective: &str) -> String {
    // Mirrors `getGoalButtonLabels`: the objective preview is capped at 28 chars.
    let objective = truncate_goal_objective(_objective, 28);
    render_full(
        _locale,
        "goal.button.details",
        &[("status", _status), ("objective", &objective)],
    )
}

/// Status health line (`status.line.health` + params).
pub fn status_health_line(_locale: &str, _health: &str) -> String {
    render_full(_locale, "status.line.health", &[("health", _health)])
}

/// Status version line (`status.line.version` + params).
pub fn status_version_line(_locale: &str, _version: &str) -> String {
    render_full(_locale, "status.line.version", &[("version", _version)])
}

/// Status uptime line (`status.line.uptime_sec` + params).
pub fn status_uptime_line(_locale: &str, _seconds: u64) -> String {
    let seconds = _seconds.to_string();
    render_full(_locale, "status.line.uptime_sec", &[("seconds", &seconds)])
}

/// Status agent-mode line (`status.line.mode` + params).
pub fn status_mode_line(_locale: &str, _mode: &str) -> String {
    render_full(_locale, "status.line.mode", &[("mode", _mode)])
}

/// Status model line (`status.line.model` + params).
pub fn status_model_line(_locale: &str, _model: &str) -> String {
    render_full(_locale, "status.line.model", &[("model", _model)])
}

/// Status project line (`status.project_selected` + params).
pub fn status_project_line(_locale: &str, _project: &str) -> String {
    render_full(_locale, "status.project_selected", &[("project", _project)])
}

/// Status session line (`status.session_selected` + params).
pub fn status_session_line(_locale: &str, _title: &str) -> String {
    render_full(_locale, "status.session_selected", &[("title", _title)])
}

/// OpenCode session error (`bot.session_error` + params).
pub fn error_session(_locale: &str, _message: &str) -> String {
    render_full(_locale, "bot.session_error", &[("message", _message)])
}

/// Unknown-command warning (`bot.unknown_command` + params).
pub fn error_unknown_command(_locale: &str, _command: &str) -> String {
    render_full(_locale, "bot.unknown_command", &[("command", _command)])
}

/// Oversized-photo warning (`bot.photo_too_large` + params).
pub fn error_photo_too_large(_locale: &str, _max_size_mb: u32) -> String {
    let max = _max_size_mb.to_string();
    render_full(_locale, "bot.photo_too_large", &[("maxSizeMb", &max)])
}

/// Oversized-file warning (`bot.file_too_large` + params).
pub fn error_file_too_large(_locale: &str, _max_size_mb: u32) -> String {
    let max = _max_size_mb.to_string();
    render_full(_locale, "bot.file_too_large", &[("maxSizeMb", &max)])
}

/// Missing project folder (`bot.project_folder_missing` + params).
pub fn error_project_folder_missing(_locale: &str, _path: &str) -> String {
    render_full(_locale, "bot.project_folder_missing", &[("path", _path)])
}

/// Provider-retry notice (`bot.session_retry` + params).
pub fn error_session_retry(_locale: &str, _message: &str) -> String {
    render_full(_locale, "bot.session_retry", &[("message", _message)])
}

/// Rename prompt (`rename.prompt` + params).
pub fn prompt_rename(_locale: &str, _title: &str) -> String {
    render_full(_locale, "rename.prompt", &[("title", _title)])
}

/// Schedule preview (`task.schedule_preview` + params).
#[allow(clippy::too_many_arguments)]
pub fn prompt_task_schedule_preview(
    _locale: &str,
    _summary: &str,
    _cron_line: &str,
    _timezone: &str,
    _kind: &str,
    _next_run_at: &str,
) -> String {
    render_full(
        _locale,
        "task.schedule_preview",
        &[
            ("summary", _summary),
            ("cronLine", _cron_line),
            ("timezone", _timezone),
            ("kind", _kind),
            ("nextRunAt", _next_run_at),
        ],
    )
}

/// Scheduled-task receipt (`task.created` + params).
#[allow(clippy::too_many_arguments)]
pub fn prompt_task_created(
    _locale: &str,
    _description: &str,
    _project: &str,
    _agent: &str,
    _model: &str,
    _schedule: &str,
    _cron_line: &str,
    _next_run_at: &str,
) -> String {
    render_full(
        _locale,
        "task.created",
        &[
            ("description", _description),
            ("project", _project),
            ("agent", _agent),
            ("model", _model),
            ("schedule", _schedule),
            ("cronLine", _cron_line),
            ("nextRunAt", _next_run_at),
        ],
    )
}

/// Poll summary block (`question.summary.question` + `question.summary.answer`).
pub fn question_summary(
    _locale: &str,
    _index: u32,
    _question: &str,
    _answer: &str,
) -> String {
    // Mirrors the poll summary block: question template + answer template.
    let index = _index.to_string();
    let question = render_full(
        _locale,
        "question.summary.question",
        &[("index", &index), ("question", _question)],
    );
    let answer = render_full(_locale, "question.summary.answer", &[("answer", _answer)]);
    format!("{question}{answer}")
}

/// Permission request header (`permission.header` + params).
pub fn permission_header(_locale: &str, _emoji: &str, _name: &str) -> String {
    render_full(_locale, "permission.header", &[("emoji", _emoji), ("name", _name)])
}

/// Progress activity line (`progress.compact.activity` + params).
pub fn progress_activity(_locale: &str, _header: &str, _activity: &str) -> String {
    render_full(
        _locale,
        "progress.compact.activity",
        &[("header", _header), ("activity", _activity)],
    )
}

/// Context-compaction confirmation (`context.confirm_text` + params).
pub fn context_confirm_text(_locale: &str, _title: &str) -> String {
    render_full(_locale, "context.confirm_text", &[("title", _title)])
}

/// Generic param interpolation over the expanded table: mirrors
/// `t(key, params, locale)` from `src/i18n/index.ts`.
pub fn render_full(_locale: &str, _key: &str, _params: &[(&str, &str)]) -> String {
    interpolate_full(&t_full(_locale, _key), _params)
}
