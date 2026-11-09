//! Russian streaming-footer gates (slice 178, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/ru.ts` for the
//! streaming summary surface NOT covered by `FULL_KEYS` in
//! `crates/otb-core/src/i18n_full.rs` (only `progress.compact.activity`
//! and `pinned.line.goal` are embedded there):
//! - `progress.compact.*` (12 keys: working_header, thinking,
//!   responding, waiting_question, waiting_permission, retrying, task,
//!   count.commands/edits/reads/tools/changed_files)
//! - `pinned.*` (13 keys: default_session_title, unknown, line.project,
//!   line.worktree, line.model, line.attach, attach.status.idle/busy,
//!   line.context, line.cost, files.title/item/more)
//! - `subagent.*` (15 keys: header, line.status/task/agent, starting,
//!   starting_untitled, working, working_with_details, completed,
//!   interrupted, failed, status.pending/running/completed/error)
//! - footer labels: `assistant.footer.interrupted`,
//!   `settings.assistant_footer.label`,
//!   `settings.pin_session_dashboard.label`
//!
//! AUDIT (2026-11-09, branch `rust-rewrite`):
//! - `src/i18n/ru.ts` lines 74-85 carry the 12 `progress.compact.*`
//!   templates (line 73 `progress.compact.activity` is already in
//!   `i18n_full`); lines 477-486 carry the `pinned.line.*` /
//!   `pinned.attach.*` templates (line 478 `pinned.line.goal` is already
//!   in `i18n_full`); lines 487-501 carry the 15 `subagent.*`
//!   templates; lines 502-504 carry `pinned.files.*`; lines 178-181
//!   carry the footer/dashboard labels. `src/i18n/en.ts` lines 75-86,
//!   475-484, 533-550 and 179-181 are the placeholder reference.
//! - `crate::i18n` resolves every key through the build-time full table,
//!   so `t("ru", key)` already returns these templates; no Rust owner
//!   pins the `ru` streaming verbatim templates yet.
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `streaming.rs` owns only the length/throttle/message
//!   budgets. `duration_footer_display_gates` (slice 76) owns only the
//!   elapsed/speed icon thresholds. `subagent_card_gates` (slice 77)
//!   owns only the locale-agnostic card composition (label templates
//!   arrive as resolved strings). `pinned_models` owns only the context
//!   limits and search caps. `summary_format_gates` owns only the
//!   locale-agnostic summary aggregation.
//! - This slice owns only the `ru` streaming lookup +
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
//!   (`crate::i18n_full` owner, including `progress.compact.activity`
//!   and `pinned.line.goal`)
//! - placeholder set-equality / missing / extra helpers
//!   (`locale_placeholder_gates` owner, slice 122)
//! - registry labels / `getLocaleOptions` / `getLocale` precedence
//!   (`locale_registry_gates` owner, slice 121)
//! - stream budgets, card composition, footer thresholds, pinned caps
//!   (`streaming`, `subagent_card_gates`, `duration_footer_display_gates`,
//!   `pinned_models` owners)
//! - `ru` STT lookup + formatting (`locale_ru_voice_gates` owner,
//!   slice 175), `ru` task-detail + goal-remainder lookup + formatting
//!   (`locale_ru_task_goal_gates` owner, slice 176), `ru` start/stop
//!   lookup + formatting (`locale_ru_start_stop_gates` owner, slice 177).
//!
//! Deferred (NOT in this slice):
//! - remaining `settings.*` toggle labels (settings-menu surface,
//!   callers localize via `crate::i18n`)
//! - `tool.todo.overflow` / `tool.file_header.*` (tool-render surface,
//!   locale-agnostic composition in the formatter owners).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice178_locale_ru_streaming_parity.rs`.

/// Locale code for this table (TS: `ru` in `SUPPORTED_LOCALES`).
pub const RU_STREAMING_LOCALE: &str = "ru";

/// Compact-progress keys covered here (the `progress.compact.*` keys in
/// `src/i18n/ru.ts` lines 74-85, outside the 72 `FULL_KEYS` in
/// `crate::i18n_full`).
pub const RU_COMPACT_KEYS: [&str; 12] = [
    "progress.compact.working_header",
    "progress.compact.thinking",
    "progress.compact.responding",
    "progress.compact.waiting_question",
    "progress.compact.waiting_permission",
    "progress.compact.retrying",
    "progress.compact.task",
    "progress.compact.count.commands",
    "progress.compact.count.edits",
    "progress.compact.count.reads",
    "progress.compact.count.tools",
    "progress.compact.count.changed_files",
];

/// Pinned-dashboard keys covered here (the `pinned.*` keys in
/// `src/i18n/ru.ts` lines 477-486 + 502-504, outside `pinned.line.goal`
/// in `crate::i18n_full`).
pub const RU_PINNED_KEYS: [&str; 13] = [
    "pinned.default_session_title",
    "pinned.unknown",
    "pinned.line.project",
    "pinned.line.worktree",
    "pinned.line.model",
    "pinned.line.attach",
    "pinned.attach.status.idle",
    "pinned.attach.status.busy",
    "pinned.line.context",
    "pinned.line.cost",
    "pinned.files.title",
    "pinned.files.item",
    "pinned.files.more",
];

/// Subagent-card keys covered here (the 15 `subagent.*` keys in
/// `src/i18n/ru.ts` lines 487-501).
pub const RU_SUBAGENT_KEYS: [&str; 15] = [
    "subagent.header",
    "subagent.line.status",
    "subagent.line.task",
    "subagent.line.agent",
    "subagent.starting",
    "subagent.starting_untitled",
    "subagent.working",
    "subagent.working_with_details",
    "subagent.completed",
    "subagent.interrupted",
    "subagent.failed",
    "subagent.status.pending",
    "subagent.status.running",
    "subagent.status.completed",
    "subagent.status.error",
];

/// Footer-label keys covered here (`src/i18n/ru.ts` lines 178-181).
pub const RU_FOOTER_KEYS: [&str; 3] = [
    "assistant.footer.interrupted",
    "settings.assistant_footer.label",
    "settings.pin_session_dashboard.label",
];

/// Verbatim `ru` compact-progress templates (TS: `src/i18n/ru.ts` lines
/// 74-85).
pub const RU_COMPACT_WORKING_HEADER: &str = "⏳ Работаю";
pub const RU_COMPACT_THINKING: &str = "💭 Думаю...";
pub const RU_COMPACT_RESPONDING: &str = "✍️ Пишу ответ...";
pub const RU_COMPACT_WAITING_QUESTION: &str = "❓ Жду ваш ответ...";
pub const RU_COMPACT_WAITING_PERMISSION: &str = "🔐 Жду разрешение...";
pub const RU_COMPACT_RETRYING: &str = "🔁 Повторяю запрос...";
pub const RU_COMPACT_TASK: &str = "🤖 Выполняю задачу";
pub const RU_COMPACT_COUNT_COMMANDS: &str = "команды: {count}";
pub const RU_COMPACT_COUNT_EDITS: &str = "правки: {count}";
pub const RU_COMPACT_COUNT_READS: &str = "чтения: {count}";
pub const RU_COMPACT_COUNT_TOOLS: &str = "инструменты: {count}";
pub const RU_COMPACT_COUNT_CHANGED_FILES: &str = "изменённые файлы: {count}";

/// Verbatim `ru` pinned-dashboard templates (TS: `src/i18n/ru.ts` lines
/// 477-486 + 502-504).
pub const RU_PINNED_DEFAULT_SESSION_TITLE: &str = "новая сессия";
pub const RU_PINNED_UNKNOWN: &str = "Неизвестно";
pub const RU_PINNED_LINE_PROJECT: &str = "Проект: {project}";
pub const RU_PINNED_LINE_WORKTREE: &str = "Worktree: {worktree}";
pub const RU_PINNED_LINE_MODEL: &str = "Модель: {model}";
pub const RU_PINNED_LINE_ATTACH: &str = "Tracking: {status}";
pub const RU_PINNED_ATTACH_STATUS_IDLE: &str = "активен, idle";
pub const RU_PINNED_ATTACH_STATUS_BUSY: &str = "активен, busy";
pub const RU_PINNED_LINE_CONTEXT: &str = "Контекст: {used} / {limit} ({percent}%)";
pub const RU_PINNED_LINE_COST: &str = "Стоимость: {cost} потрачено";
pub const RU_PINNED_FILES_TITLE: &str = "Файлы ({count}):";
pub const RU_PINNED_FILES_ITEM: &str = "  {path}{diff}";
pub const RU_PINNED_FILES_MORE: &str = "  ... и еще {count}";

/// Verbatim `ru` subagent-card templates (TS: `src/i18n/ru.ts` lines
/// 487-501).
pub const RU_SUBAGENT_HEADER: &str = "Сабагент {agent}: {description}";
pub const RU_SUBAGENT_LINE_STATUS: &str = "Статус: {status}";
pub const RU_SUBAGENT_LINE_TASK: &str = "Задача: {task}";
pub const RU_SUBAGENT_LINE_AGENT: &str = "Агент: {agent}";
pub const RU_SUBAGENT_STARTING: &str = "Запуск задачи: {task}";
pub const RU_SUBAGENT_STARTING_UNTITLED: &str = "Запуск задачи...";
pub const RU_SUBAGENT_WORKING: &str = "В работе...";
pub const RU_SUBAGENT_WORKING_WITH_DETAILS: &str = "В работе: {details}";
pub const RU_SUBAGENT_COMPLETED: &str = "Завершена";
pub const RU_SUBAGENT_INTERRUPTED: &str = "Прервана";
pub const RU_SUBAGENT_FAILED: &str = "Ошибка задачи";
pub const RU_SUBAGENT_STATUS_PENDING: &str = "ожидание";
pub const RU_SUBAGENT_STATUS_RUNNING: &str = "в работе";
pub const RU_SUBAGENT_STATUS_COMPLETED: &str = "завершен";
pub const RU_SUBAGENT_STATUS_ERROR: &str = "ошибка";

/// Verbatim `ru` footer-label templates (TS: `src/i18n/ru.ts` lines
/// 178-181).
pub const RU_ASSISTANT_FOOTER_INTERRUPTED: &str = "прерван";
pub const RU_SETTINGS_ASSISTANT_FOOTER_LABEL: &str = "Футер ответа";
pub const RU_SETTINGS_PIN_SESSION_DASHBOARD_LABEL: &str = "Закреплять дашборд сессии";

/// Verbatim `ru` streaming template for a key, or `None` outside this
/// slice (TS: `ru[key]` for the keys above).
pub fn lookup_ru_streaming_template(key: &str) -> Option<&'static str> {
    match key {
        "progress.compact.working_header" => Some(RU_COMPACT_WORKING_HEADER),
        "progress.compact.thinking" => Some(RU_COMPACT_THINKING),
        "progress.compact.responding" => Some(RU_COMPACT_RESPONDING),
        "progress.compact.waiting_question" => Some(RU_COMPACT_WAITING_QUESTION),
        "progress.compact.waiting_permission" => Some(RU_COMPACT_WAITING_PERMISSION),
        "progress.compact.retrying" => Some(RU_COMPACT_RETRYING),
        "progress.compact.task" => Some(RU_COMPACT_TASK),
        "progress.compact.count.commands" => Some(RU_COMPACT_COUNT_COMMANDS),
        "progress.compact.count.edits" => Some(RU_COMPACT_COUNT_EDITS),
        "progress.compact.count.reads" => Some(RU_COMPACT_COUNT_READS),
        "progress.compact.count.tools" => Some(RU_COMPACT_COUNT_TOOLS),
        "progress.compact.count.changed_files" => Some(RU_COMPACT_COUNT_CHANGED_FILES),
        "pinned.default_session_title" => Some(RU_PINNED_DEFAULT_SESSION_TITLE),
        "pinned.unknown" => Some(RU_PINNED_UNKNOWN),
        "pinned.line.project" => Some(RU_PINNED_LINE_PROJECT),
        "pinned.line.worktree" => Some(RU_PINNED_LINE_WORKTREE),
        "pinned.line.model" => Some(RU_PINNED_LINE_MODEL),
        "pinned.line.attach" => Some(RU_PINNED_LINE_ATTACH),
        "pinned.attach.status.idle" => Some(RU_PINNED_ATTACH_STATUS_IDLE),
        "pinned.attach.status.busy" => Some(RU_PINNED_ATTACH_STATUS_BUSY),
        "pinned.line.context" => Some(RU_PINNED_LINE_CONTEXT),
        "pinned.line.cost" => Some(RU_PINNED_LINE_COST),
        "pinned.files.title" => Some(RU_PINNED_FILES_TITLE),
        "pinned.files.item" => Some(RU_PINNED_FILES_ITEM),
        "pinned.files.more" => Some(RU_PINNED_FILES_MORE),
        "subagent.header" => Some(RU_SUBAGENT_HEADER),
        "subagent.line.status" => Some(RU_SUBAGENT_LINE_STATUS),
        "subagent.line.task" => Some(RU_SUBAGENT_LINE_TASK),
        "subagent.line.agent" => Some(RU_SUBAGENT_LINE_AGENT),
        "subagent.starting" => Some(RU_SUBAGENT_STARTING),
        "subagent.starting_untitled" => Some(RU_SUBAGENT_STARTING_UNTITLED),
        "subagent.working" => Some(RU_SUBAGENT_WORKING),
        "subagent.working_with_details" => Some(RU_SUBAGENT_WORKING_WITH_DETAILS),
        "subagent.completed" => Some(RU_SUBAGENT_COMPLETED),
        "subagent.interrupted" => Some(RU_SUBAGENT_INTERRUPTED),
        "subagent.failed" => Some(RU_SUBAGENT_FAILED),
        "subagent.status.pending" => Some(RU_SUBAGENT_STATUS_PENDING),
        "subagent.status.running" => Some(RU_SUBAGENT_STATUS_RUNNING),
        "subagent.status.completed" => Some(RU_SUBAGENT_STATUS_COMPLETED),
        "subagent.status.error" => Some(RU_SUBAGENT_STATUS_ERROR),
        "assistant.footer.interrupted" => Some(RU_ASSISTANT_FOOTER_INTERRUPTED),
        "settings.assistant_footer.label" => Some(RU_SETTINGS_ASSISTANT_FOOTER_LABEL),
        "settings.pin_session_dashboard.label" => Some(RU_SETTINGS_PIN_SESSION_DASHBOARD_LABEL),
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
pub fn is_ru_streaming_placeholder_ok(en_template: &str, ru_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(ru_template)
}

/// `{kind-word}: {count}` compact counter line for a tool-count kind
/// (`commands` / `edits` / `reads` / `tools` / `changed_files`), with the
/// count filled in (TS: `t("progress.compact.count.*", { count })` call
/// sites in the progress renderer). Returns `None` for unknown kinds.
pub fn format_ru_compact_count(kind: &str, count: usize) -> Option<String> {
    let template = match kind {
        "commands" => RU_COMPACT_COUNT_COMMANDS,
        "edits" => RU_COMPACT_COUNT_EDITS,
        "reads" => RU_COMPACT_COUNT_READS,
        "tools" => RU_COMPACT_COUNT_TOOLS,
        "changed_files" => RU_COMPACT_COUNT_CHANGED_FILES,
        _ => return None,
    };
    let count = count.to_string();
    Some(interpolate_ru(template, &[("count", &count)]))
}

/// `Запуск задачи: {task}` with the task title filled in (TS:
/// `t("subagent.starting", { task })` call site in the subagent card
/// renderer).
pub fn format_ru_subagent_starting(task: &str) -> String {
    interpolate_ru(RU_SUBAGENT_STARTING, &[("task", task)])
}

/// `Стоимость: {cost} потрачено` with the cost filled in (TS:
/// `t("pinned.line.cost", { cost })` call site in the pinned dashboard
/// renderer).
pub fn format_ru_pinned_cost(cost: &str) -> String {
    interpolate_ru(RU_PINNED_LINE_COST, &[("cost", cost)])
}

/// `  ... и еще {count}` overflow line with the hidden-file count
/// filled in (TS: `t("pinned.files.more", { count })` call site in the
/// pinned dashboard renderer).
pub fn format_ru_pinned_files_more(count: usize) -> String {
    let count = count.to_string();
    interpolate_ru(RU_PINNED_FILES_MORE, &[("count", &count)])
}
