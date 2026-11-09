//! Slice 177 parity: Russian start/stop gates.
//!
//! Mirrors the pure TS contract in `src/i18n/ru.ts` (welcome lines 65-66,
//! `stop.*` lines 288-307, `opencode_start.*` lines 309-324,
//! `opencode_stop.*` lines 325-337) for the 30 start/stop keys outside
//! `FULL_KEYS` in `crate::i18n_full`:
//! - verbatim `ru` templates for the welcome text, stop outcomes and
//!   server start/stop outcomes
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind the stop-command and local-start/stop call sites)
//! - the folder-gone notice pair (`bot.project_folder_missing` /
//!   `bot.project_folder_missing_worktree`) resolves in `en` and `ru`
//!   through `crate::i18n_full` (slice 174 asserted the worktree
//!   variant; this slice asserts the plain variant)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! stop dispatch / ready lifecycle / serve-spawn / pid-file / exit-wait /
//! registered-server / folder-presence gates (`server_command_gates`,
//! `local_process_gates`, `server_target_gates`, `session_context_gates`;
//! slice 174), `ru` STT lookup + formatting (`locale_ru_voice_gates`,
//! slice 175), `ru` task-detail + goal-remainder lookup + formatting
//! (`locale_ru_task_goal_gates`, slice 176).

use otb_core::i18n_full::error_project_folder_missing;
use otb_core::locale_ru_start_stop_gates::*;

// ---- constants (3) ----

#[test]
fn ru_start_stop_locale_value_parity() {
    assert_eq!(RU_START_STOP_LOCALE, "ru");
}

#[test]
fn ru_start_stop_keys_value_parity() {
    assert_eq!(RU_START_STOP_KEYS.len(), 30);
    assert!(RU_START_STOP_KEYS.contains(&"start.welcome"));
    assert!(RU_START_STOP_KEYS.contains(&"stop.success_background_running"));
    assert!(RU_START_STOP_KEYS.contains(&"opencode_start.connected"));
    assert!(RU_START_STOP_KEYS.contains(&"opencode_stop.stopping"));
}

#[test]
fn ru_start_stop_templates_value_parity() {
    assert_eq!(
        RU_STOP_SUCCESS,
        "✅ Действие агента прервано. Новые сообщения от текущего запуска больше не придут."
    );
    assert_eq!(
        RU_STOP_SUCCESS_BACKGROUND_RUNNING,
        "✅ Действие агента прервано.\nФоновые операции продолжают работу: {count}\nИх ход и результаты по-прежнему появятся здесь."
    );
    assert_eq!(
        RU_OPENCODE_START_CONNECTED,
        "✅ Подключено к запущенному OpenCode Server\n{url} · v{version}"
    );
    assert_eq!(
        RU_OPENCODE_STOP_STOPPING,
        "🛑 Останавливаю OpenCode Server...\n\nPID: {pid}"
    );
    assert_eq!(
        RU_OPENCODE_STOP_SUCCESS,
        "✅ OpenCode Server успешно остановлен"
    );
}

// ---- lookup (2) ----

#[test]
fn ru_start_stop_lookup_stopping_parity() {
    assert_eq!(
        lookup_ru_start_stop_template("opencode_stop.stopping"),
        Some("🛑 Останавливаю OpenCode Server...\n\nPID: {pid}")
    );
}

#[test]
fn ru_start_stop_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ru_start_stop_template("stop.unknown_key"), None);
    assert_eq!(lookup_ru_start_stop_template("pinned.line.goal"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ru_start_stop_placeholders_match_en_parity() {
    assert!(is_ru_start_stop_placeholder_ok(
        "✅ Connected to a running OpenCode Server\n{url} · v{version}",
        "✅ Подключено к запущенному OpenCode Server\n{url} · v{version}"
    ));
    assert!(is_ru_start_stop_placeholder_ok(
        "🛑 Stopping OpenCode Server...\n\nPID: {pid}",
        "🛑 Останавливаю OpenCode Server...\n\nPID: {pid}"
    ));
}

#[test]
fn ru_start_stop_placeholders_reject_mismatch_parity() {
    assert!(!is_ru_start_stop_placeholder_ok(
        "✅ OpenCode Server started successfully\n\nPID: {pid}\nVersion: {version}",
        "✅ OpenCode Server успешно запущен\n\nPID: {pid}"
    ));
    assert!(!is_ru_start_stop_placeholder_ok(
        "⚠️ OpenCode Server responds on port {port}, but no local process was found to stop.",
        "⚠️ OpenCode Server отвечает на порту {port}, но локальный процесс для остановки найти не удалось. {extra}"
    ));
}

// ---- formatting (4) ----

#[test]
fn ru_start_stop_background_running_line_parity() {
    assert_eq!(
        format_ru_stop_background_running(2),
        "✅ Действие агента прервано.\nФоновые операции продолжают работу: 2\nИх ход и результаты по-прежнему появятся здесь."
    );
}

#[test]
fn ru_start_stop_connected_line_parity() {
    assert_eq!(
        format_ru_start_connected("http://127.0.0.1:4096", "1.2.3"),
        "✅ Подключено к запущенному OpenCode Server\nhttp://127.0.0.1:4096 · v1.2.3"
    );
}

#[test]
fn ru_start_stop_success_line_parity() {
    assert_eq!(
        format_ru_start_success("4242", "1.2.3"),
        "✅ OpenCode Server успешно запущен\n\nPID: 4242\nВерсия: 1.2.3"
    );
}

#[test]
fn ru_start_stop_stopping_line_parity() {
    assert_eq!(
        format_ru_stop_stopping("4242"),
        "🛑 Останавливаю OpenCode Server...\n\nPID: 4242"
    );
}

// ---- folder-gone notice pair (1, via i18n_full) ----

#[test]
fn ru_start_stop_folder_missing_points_to_projects_in_en_and_ru() {
    assert_eq!(
        error_project_folder_missing("en", "/work/gone"),
        "🚫 The project folder no longer exists: /work/gone. Choose another project in /projects."
    );
    assert_eq!(
        error_project_folder_missing("ru", "/work/gone"),
        "🚫 Папка проекта больше не существует: /work/gone. Выберите другой проект в /projects."
    );
    assert_eq!(
        error_project_folder_missing("xx", "/work/gone"),
        error_project_folder_missing("en", "/work/gone")
    );
}
