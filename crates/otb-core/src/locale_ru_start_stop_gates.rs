//! Russian start/stop gates (slice 177, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/ru.ts` for the
//! start/stop lifecycle surface NOT covered by `FULL_KEYS` in
//! `crates/otb-core/src/i18n_full.rs`:
//! - `start.welcome` (multiline `/new` onboarding text)
//! - `stop.no_active_session`, `stop.in_progress`,
//!   `stop.warn_unconfirmed`, `stop.warn_maybe_finished`, `stop.success`,
//!   `stop.success_background_running` (`{count}`),
//!   `stop.warn_still_busy`, `stop.warn_timeout`, `stop.warn_local_only`,
//!   `stop.error`
//! - `opencode_start.*` (10 keys: already-running, connected, starting,
//!   start_error, started_not_ready, success, error, remote_configured)
//! - `opencode_stop.*` (9 keys: external_running, remote_configured,
//!   not_running, pid_not_found, stopping, stop_error, still_running,
//!   success, error)
//!
//! AUDIT (2026-11-09, branch `rust-rewrite`):
//! - `src/i18n/ru.ts` lines 65-66 carry `start.welcome`; lines 288-307
//!   carry the 10 `stop.*` templates; lines 309-324 carry the 10
//!   `opencode_start.*` templates; lines 325-337 carry the 9
//!   `opencode_stop.*` templates. `src/i18n/en.ts` lines 66-68 carry
//!   `start.welcome`, lines 290-305 the `stop.*` reference, lines
//!   307-321 the `opencode_start.*` reference, lines 323-333 the
//!   `opencode_stop.*` reference.
//! - `crate::i18n` resolves every key through the build-time full table,
//!   so `t("ru", key)` already returns these templates; no Rust owner
//!   pins the `ru` start/stop verbatim templates yet.
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `server_command_gates` (slice 104) owns only the
//!   locale-agnostic stop-dispatch gates (`STOP_REASON`, kill budget,
//!   remote/not-running/error gates). `local_process_gates` owns only
//!   the serve-spawn / pid-file / kill-escalation gates.
//!   `session_context_gates` + `server_target_gates` own only the
//!   folder-presence / registered-server gates (slice 174).
//! - This slice owns only the `ru` start/stop lookup +
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
//!   (`crate::i18n_full` owner, including `bot.project_folder_missing`
//!   and `bot.project_folder_missing_worktree` used by the folder-gone
//!   verdict in slice 174)
//! - placeholder set-equality / missing / extra helpers
//!   (`locale_placeholder_gates` owner, slice 122)
//! - registry labels / `getLocaleOptions` / `getLocale` precedence
//!   (`locale_registry_gates` owner, slice 121)
//! - stop dispatch, ready-lifecycle transitions, serve-spawn args,
//!   pid-file record/clear, exit-wait escalation, registered-server skips
//!   (`server_command_gates` slice 104, `local_process_gates`,
//!   `server_target_gates`, `session_context_gates` owners; slice 174)
//! - `ru` STT lookup + formatting (`locale_ru_voice_gates` owner,
//!   slice 175) and `ru` task-detail + goal-remainder lookup +
//!   formatting (`locale_ru_task_goal_gates` owner, slice 176).
//!
//! Deferred (NOT in this slice):
//! - `reload.*` templates (reload-command surface, slice 171 owner
//!   locale-agnostic; callers localize via `crate::i18n`)
//! - `help.*` / `cmd.description.*` static labels (command registry
//!   surface in `otb-bot`, structure-only).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice177_locale_ru_start_stop_parity.rs`.

/// Locale code for this table (TS: `ru` in `SUPPORTED_LOCALES`).
pub const RU_START_STOP_LOCALE: &str = "ru";

/// Start/stop keys covered here (`start.welcome`, the 10 `stop.*` keys,
/// the 10 `opencode_start.*` keys and the 9 `opencode_stop.*` keys in
/// `src/i18n/ru.ts`, outside the 72 `FULL_KEYS` in `crate::i18n_full`).
pub const RU_START_STOP_KEYS: [&str; 30] = [
    "start.welcome",
    "stop.no_active_session",
    "stop.in_progress",
    "stop.warn_unconfirmed",
    "stop.warn_maybe_finished",
    "stop.success",
    "stop.success_background_running",
    "stop.warn_still_busy",
    "stop.warn_timeout",
    "stop.warn_local_only",
    "stop.error",
    "opencode_start.already_running_managed",
    "opencode_start.already_running_external",
    "opencode_start.already_running",
    "opencode_start.connected",
    "opencode_start.remote_configured",
    "opencode_start.starting",
    "opencode_start.start_error",
    "opencode_start.started_not_ready",
    "opencode_start.success",
    "opencode_start.error",
    "opencode_stop.external_running",
    "opencode_stop.remote_configured",
    "opencode_stop.not_running",
    "opencode_stop.pid_not_found",
    "opencode_stop.stopping",
    "opencode_stop.stop_error",
    "opencode_stop.still_running",
    "opencode_stop.success",
    "opencode_stop.error",
];

/// Verbatim `ru` welcome template (TS: `src/i18n/ru.ts` lines 65-66).
pub const RU_START_WELCOME: &str = "👋 Добро пожаловать в OpenCode Telegram Bot!\n\nИспользуйте команды:\n/projects — выбрать проект\n/sessions — список сессий\n/new — новая сессия\n/commands — пользовательские команды\n/skills — каталог скиллов\n/task — задача по расписанию\n/tasklist — список задач по расписанию\n/status — статус\n/help — справка\n\nАгент, модель и вариант выбираются кнопками внизу.";

/// Verbatim `ru` stop templates (TS: `src/i18n/ru.ts` lines 288-307).
pub const RU_STOP_NO_ACTIVE_SESSION: &str =
    "🛑 Агент не был запущен\n\nСначала создайте сессию командой /new или выберите существующую через /sessions.";
pub const RU_STOP_IN_PROGRESS: &str =
    "🛑 Отключил поток событий и отправляю сигнал прерывания...\n\nОжидание остановки агента.";
pub const RU_STOP_WARN_UNCONFIRMED: &str = "⚠️ Поток событий остановлен, но сервер не подтвердил прерывание.\n\nПроверьте /status и повторите /abort через пару секунд.";
pub const RU_STOP_WARN_MAYBE_FINISHED: &str =
    "⚠️ Поток событий остановлен, но агент мог уже завершиться к моменту запроса.";
pub const RU_STOP_SUCCESS: &str =
    "✅ Действие агента прервано. Новые сообщения от текущего запуска больше не придут.";
pub const RU_STOP_SUCCESS_BACKGROUND_RUNNING: &str = "✅ Действие агента прервано.\nФоновые операции продолжают работу: {count}\nИх ход и результаты по-прежнему появятся здесь.";
pub const RU_STOP_WARN_STILL_BUSY: &str = "⚠️ Сигнал отправлен, но агент еще busy.\n\nПоток событий уже отключен, поэтому бот не будет присылать промежуточные сообщения.";
pub const RU_STOP_WARN_TIMEOUT: &str =
    "⚠️ Таймаут запроса на прерывание.\n\nПоток событий уже отключен, повторите /abort через пару секунд.";
pub const RU_STOP_WARN_LOCAL_ONLY: &str =
    "⚠️ Поток событий остановлен локально, но при прерывании на сервере произошла ошибка.";
pub const RU_STOP_ERROR: &str =
    "🔴 Ошибка при прерывании действия.\n\nПоток событий остановлен, попробуйте /abort еще раз.";

/// Verbatim `ru` server-start templates (TS: `src/i18n/ru.ts` lines
/// 309-324).
pub const RU_OPENCODE_START_ALREADY_RUNNING_MANAGED: &str =
    "⚠️ OpenCode Server уже запущен\n\nPID: {pid}\nUptime: {seconds} секунд";
pub const RU_OPENCODE_START_ALREADY_RUNNING_EXTERNAL: &str = "✅ OpenCode Server уже запущен внешним процессом\n\nВерсия: {version}\n\nЭтот сервер не был запущен через бота, поэтому команда /opencode-stop не сможет его остановить.";
pub const RU_OPENCODE_START_ALREADY_RUNNING: &str =
    "✅ OpenCode Server уже запущен\n\nВерсия: {version}";
pub const RU_OPENCODE_START_CONNECTED: &str =
    "✅ Подключено к запущенному OpenCode Server\n{url} · v{version}";
pub const RU_OPENCODE_START_REMOTE_CONFIGURED: &str =
    "⚠️ /opencode_start работает только с локальным OpenCode Server.";
pub const RU_OPENCODE_START_STARTING: &str = "🔄 Запускаю OpenCode Server...";
pub const RU_OPENCODE_START_START_ERROR: &str = "🔴 Не удалось запустить OpenCode Server\n\nОшибка: {error}\n\nПроверьте, что OpenCode CLI установлен и доступен в PATH или стандартном каталоге установки (OPENCODE_BIN_PATH позволяет указать свой исполняемый файл):\nopencode --version\nnpm install -g @opencode-ai/cli";
pub const RU_OPENCODE_START_STARTED_NOT_READY: &str = "⚠️ OpenCode Server запущен, но не отвечает\n\nPID: {pid}\n\nСервер может запускаться. Попробуйте /status через несколько секунд.";
pub const RU_OPENCODE_START_SUCCESS: &str =
    "✅ OpenCode Server успешно запущен\n\nPID: {pid}\nВерсия: {version}";
pub const RU_OPENCODE_START_ERROR: &str =
    "🔴 Произошла ошибка при запуске сервера.\n\nПроверьте логи приложения для подробностей.";

/// Verbatim `ru` server-stop templates (TS: `src/i18n/ru.ts` lines
/// 325-337).
pub const RU_OPENCODE_STOP_EXTERNAL_RUNNING: &str = "⚠️ OpenCode Server запущен внешним процессом\n\nЭтот сервер не был запущен через /opencode-start.\nОстановите его вручную или используйте /status для проверки состояния.";
pub const RU_OPENCODE_STOP_REMOTE_CONFIGURED: &str =
    "⚠️ /opencode_stop работает только с локальным OpenCode Server.";
pub const RU_OPENCODE_STOP_NOT_RUNNING: &str = "⚠️ OpenCode Server не запущен";
pub const RU_OPENCODE_STOP_PID_NOT_FOUND: &str =
    "⚠️ OpenCode Server отвечает на порту {port}, но локальный процесс для остановки найти не удалось.";
pub const RU_OPENCODE_STOP_STOPPING: &str = "🛑 Останавливаю OpenCode Server...\n\nPID: {pid}";
pub const RU_OPENCODE_STOP_STOP_ERROR: &str =
    "🔴 Не удалось остановить OpenCode Server\n\nОшибка: {error}";
pub const RU_OPENCODE_STOP_STILL_RUNNING: &str =
    "Сервер все еще отвечает после запроса на остановку.";
pub const RU_OPENCODE_STOP_SUCCESS: &str = "✅ OpenCode Server успешно остановлен";
pub const RU_OPENCODE_STOP_ERROR: &str =
    "🔴 Произошла ошибка при остановке сервера.\n\nПроверьте логи приложения для подробностей.";

/// Verbatim `ru` start/stop template for a key, or `None` outside this
/// slice (TS: `ru[key]` for the keys above).
pub fn lookup_ru_start_stop_template(key: &str) -> Option<&'static str> {
    match key {
        "start.welcome" => Some(RU_START_WELCOME),
        "stop.no_active_session" => Some(RU_STOP_NO_ACTIVE_SESSION),
        "stop.in_progress" => Some(RU_STOP_IN_PROGRESS),
        "stop.warn_unconfirmed" => Some(RU_STOP_WARN_UNCONFIRMED),
        "stop.warn_maybe_finished" => Some(RU_STOP_WARN_MAYBE_FINISHED),
        "stop.success" => Some(RU_STOP_SUCCESS),
        "stop.success_background_running" => Some(RU_STOP_SUCCESS_BACKGROUND_RUNNING),
        "stop.warn_still_busy" => Some(RU_STOP_WARN_STILL_BUSY),
        "stop.warn_timeout" => Some(RU_STOP_WARN_TIMEOUT),
        "stop.warn_local_only" => Some(RU_STOP_WARN_LOCAL_ONLY),
        "stop.error" => Some(RU_STOP_ERROR),
        "opencode_start.already_running_managed" => Some(RU_OPENCODE_START_ALREADY_RUNNING_MANAGED),
        "opencode_start.already_running_external" => {
            Some(RU_OPENCODE_START_ALREADY_RUNNING_EXTERNAL)
        }
        "opencode_start.already_running" => Some(RU_OPENCODE_START_ALREADY_RUNNING),
        "opencode_start.connected" => Some(RU_OPENCODE_START_CONNECTED),
        "opencode_start.remote_configured" => Some(RU_OPENCODE_START_REMOTE_CONFIGURED),
        "opencode_start.starting" => Some(RU_OPENCODE_START_STARTING),
        "opencode_start.start_error" => Some(RU_OPENCODE_START_START_ERROR),
        "opencode_start.started_not_ready" => Some(RU_OPENCODE_START_STARTED_NOT_READY),
        "opencode_start.success" => Some(RU_OPENCODE_START_SUCCESS),
        "opencode_start.error" => Some(RU_OPENCODE_START_ERROR),
        "opencode_stop.external_running" => Some(RU_OPENCODE_STOP_EXTERNAL_RUNNING),
        "opencode_stop.remote_configured" => Some(RU_OPENCODE_STOP_REMOTE_CONFIGURED),
        "opencode_stop.not_running" => Some(RU_OPENCODE_STOP_NOT_RUNNING),
        "opencode_stop.pid_not_found" => Some(RU_OPENCODE_STOP_PID_NOT_FOUND),
        "opencode_stop.stopping" => Some(RU_OPENCODE_STOP_STOPPING),
        "opencode_stop.stop_error" => Some(RU_OPENCODE_STOP_STOP_ERROR),
        "opencode_stop.still_running" => Some(RU_OPENCODE_STOP_STILL_RUNNING),
        "opencode_stop.success" => Some(RU_OPENCODE_STOP_SUCCESS),
        "opencode_stop.error" => Some(RU_OPENCODE_STOP_ERROR),
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
pub fn is_ru_start_stop_placeholder_ok(en_template: &str, ru_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(ru_template)
}

/// `✅ Действие агента прервано. Фоновые операции продолжают работу:
/// {count} ...` with the background-operation count filled in (TS:
/// `t("stop.success_background_running", { count })` call site in
/// `src/bot/commands/stop-command.ts`).
pub fn format_ru_stop_background_running(count: usize) -> String {
    let count = count.to_string();
    interpolate_ru(RU_STOP_SUCCESS_BACKGROUND_RUNNING, &[("count", &count)])
}

/// `✅ Подключено к запущенному OpenCode Server ...` with URL and version
/// filled in (TS: `t("opencode_start.connected", { url, version })` call
/// site in `src/opencode/local-start.ts`).
pub fn format_ru_start_connected(url: &str, version: &str) -> String {
    interpolate_ru(
        RU_OPENCODE_START_CONNECTED,
        &[("url", url), ("version", version)],
    )
}

/// `✅ OpenCode Server успешно запущен ...` with PID and version filled
/// in (TS: `t("opencode_start.success", { pid, version })` call site in
/// `src/opencode/local-start.ts`).
pub fn format_ru_start_success(pid: &str, version: &str) -> String {
    interpolate_ru(
        RU_OPENCODE_START_SUCCESS,
        &[("pid", pid), ("version", version)],
    )
}

/// `🛑 Останавливаю OpenCode Server... PID: {pid}` with the PID filled
/// in (TS: `t("opencode_stop.stopping", { pid })` call site in
/// `src/opencode/local-stop.ts`).
pub fn format_ru_stop_stopping(pid: &str) -> String {
    interpolate_ru(RU_OPENCODE_STOP_STOPPING, &[("pid", pid)])
}
