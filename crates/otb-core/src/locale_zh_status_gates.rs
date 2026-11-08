//! Chinese status-table gates (slice 159, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/zh.ts` for the status
//! command surface (`STATUS_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `status.line.health` (`状态：{health}`)
//! - `status.line.version` (`OpenCode 版本：{version}`)
//! - `status.line.pid` (`PID：{pid}`)
//! - `status.line.uptime_sec` (`运行时间：{seconds} 秒`)
//! - `status.line.mode` (`Agent：{mode}`)
//! - `status.line.model` (`模型：{model}`)
//! - `status.project_selected` (`项目：{project}`)
//! - `status.session_selected` (`当前会话：{title}`)
//! - `status.header_running` (`🟢 OpenCode 服务器正在运行`)
//! - `status.health.healthy` (`健康`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/zh.ts` line 127 carries `status.header_running`;
//!   line 128 carries `status.health.healthy`; line 130 carries
//!   `status.line.health`; line 131 carries `status.line.version`;
//!   lines 135-138 carry `status.line.pid` / `status.line.uptime_sec` /
//!   `status.line.mode` / `status.line.model`; line 144 carries
//!   `status.project_selected`; line 148 carries
//!   `status.session_selected`. `src/i18n/en.ts` lines 137-138 + 140-141
//!   are the placeholder reference (`en` keeps `Status: {health}`,
//!   `OpenCode version: {version}`, `Healthy`, ...).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only;
//!   `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates; `locale_de_status_gates` (slice 123) owns only the `de`
//!   status table; `locale_fr_status_gates` (slice 127) owns only the `fr`
//!   status table; `locale_es_status_gates` (slice 131) owns only the `es`
//!   status table; `locale_it_status_gates` (slice 135) owns only the
//!   `it` status table; `locale_pt_status_gates` (slice 139) owns only
//!   the `pt` status table; `locale_id_status_gates` (slice 143) owns
//!   only the `id` status table; `locale_ko_status_gates` (slice 147)
//!   owns only the `ko` status table; `locale_tr_status_gates`
//!   (slice 151) owns only the `tr` status table; `locale_ar_status_gates`
//!   (slice 155) owns only the `ar` status table. No Rust owner embeds
//!   the `zh` status templates yet.
//! - This slice owns only the `zh` status lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `zh.ts` file reads (arrive as explicit `key` args here),
//!   per-locale test looping, `t()` fallback wiring.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback (`crate::i18n` owner)
//! - `FULL_KEYS` / `STATUS_KEYS` category lists + en/ru verbatim tables +
//!   interpolation (`crate::i18n_full` owner)
//! - placeholder set-equality / missing / extra helpers
//!   (`locale_placeholder_gates` owner, slice 122)
//! - registry labels / `getLocaleOptions` / `getLocale` precedence
//!   (`locale_registry_gates` owner, slice 121)
//! - `de` status lookup + formatting (`locale_de_status_gates` owner,
//!   slice 123)
//! - `fr` status lookup + formatting (`locale_fr_status_gates` owner,
//!   slice 127)
//! - `es` status lookup + formatting (`locale_es_status_gates` owner,
//!   slice 131)
//! - `it` status lookup + formatting (`locale_it_status_gates` owner,
//!   slice 135)
//! - `pt` status lookup + formatting (`locale_pt_status_gates` owner,
//!   slice 139)
//! - `id` status lookup + formatting (`locale_id_status_gates` owner,
//!   slice 143)
//! - `ko` status lookup + formatting (`locale_ko_status_gates` owner,
//!   slice 147)
//! - `tr` status lookup + formatting (`locale_tr_status_gates` owner,
//!   slice 151)
//! - `ar` status lookup + formatting (`locale_ar_status_gates` owner,
//!   slice 155).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice159_locale_zh_status_parity.rs`.

/// Locale code for this table (TS: `zh` in `SUPPORTED_LOCALES`).
pub const ZH_LOCALE: &str = "zh";

/// Status keys covered here (same 10-key subset as `DE_STATUS_KEYS`,
/// slice 123, `FR_STATUS_KEYS`, slice 127, `ES_STATUS_KEYS`, slice 131,
/// `IT_STATUS_KEYS`, slice 135, `PT_STATUS_KEYS`, slice 139,
/// `ID_STATUS_KEYS`, slice 143, `KO_STATUS_KEYS`, slice 147,
/// `TR_STATUS_KEYS`, slice 151, and `AR_STATUS_KEYS`, slice 155).
pub const ZH_STATUS_KEYS: [&str; 10] = [
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

/// Verbatim `zh` templates (TS: `src/i18n/zh.ts` lines 127-148).
pub const ZH_STATUS_LINE_HEALTH: &str = "状态：{health}";
/// Verbatim `zh` version line (TS: `status.line.version`).
pub const ZH_STATUS_LINE_VERSION: &str = "OpenCode 版本：{version}";
/// Verbatim `zh` pid line (TS: `status.line.pid`).
pub const ZH_STATUS_LINE_PID: &str = "PID：{pid}";
/// Verbatim `zh` uptime line (TS: `status.line.uptime_sec`).
pub const ZH_STATUS_LINE_UPTIME_SEC: &str = "运行时间：{seconds} 秒";
/// Verbatim `zh` agent-mode line (TS: `status.line.mode`).
pub const ZH_STATUS_LINE_MODE: &str = "Agent：{mode}";
/// Verbatim `zh` model line (TS: `status.line.model`).
pub const ZH_STATUS_LINE_MODEL: &str = "模型：{model}";
/// Verbatim `zh` project line (TS: `status.project_selected`).
pub const ZH_STATUS_PROJECT_SELECTED: &str = "项目：{project}";
/// Verbatim `zh` session line (TS: `status.session_selected`).
pub const ZH_STATUS_SESSION_SELECTED: &str = "当前会话：{title}";
/// Verbatim `zh` running header (TS: `status.header_running`).
pub const ZH_STATUS_HEADER_RUNNING: &str = "🟢 OpenCode 服务器正在运行";
/// Verbatim `zh` healthy word (TS: `status.health.healthy`).
pub const ZH_STATUS_HEALTH_HEALTHY: &str = "健康";

/// Verbatim `zh` status template for a key, or `None` outside this slice
/// (TS: `zh[key]` for the 10 status keys above).
pub fn lookup_zh_status_template(key: &str) -> Option<&'static str> {
    match key {
        "status.line.health" => Some(ZH_STATUS_LINE_HEALTH),
        "status.line.version" => Some(ZH_STATUS_LINE_VERSION),
        "status.line.pid" => Some(ZH_STATUS_LINE_PID),
        "status.line.uptime_sec" => Some(ZH_STATUS_LINE_UPTIME_SEC),
        "status.line.mode" => Some(ZH_STATUS_LINE_MODE),
        "status.line.model" => Some(ZH_STATUS_LINE_MODEL),
        "status.project_selected" => Some(ZH_STATUS_PROJECT_SELECTED),
        "status.session_selected" => Some(ZH_STATUS_SESSION_SELECTED),
        "status.header_running" => Some(ZH_STATUS_HEADER_RUNNING),
        "status.health.healthy" => Some(ZH_STATUS_HEALTH_HEALTHY),
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

fn interpolate_zh(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `zh` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_zh_status_placeholder_ok(_en_template: &str, _zh_template: &str) -> bool {
    placeholder_set(_en_template) == placeholder_set(_zh_template)
}

/// `状态：{health}` line with the health word filled in
/// (TS: `t("status.line.health", { health })` call site in
/// `src/bot/commands/status-command.ts`).
pub fn format_zh_status_health_line(_health: &str) -> String {
    interpolate_zh(ZH_STATUS_LINE_HEALTH, &[("health", _health)])
}

/// `模型：{model}` line with the model filled in
/// (TS: `t("status.line.model", { model })` call site).
pub fn format_zh_status_model_line(_model: &str) -> String {
    interpolate_zh(ZH_STATUS_LINE_MODEL, &[("model", _model)])
}

/// `项目：{project}` line with the project filled in
/// (TS: `t("status.project_selected", { project })` call site).
pub fn format_zh_status_project_line(_project: &str) -> String {
    interpolate_zh(ZH_STATUS_PROJECT_SELECTED, &[("project", _project)])
}
