//! Turkish status-table gates (slice 151, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/tr.ts` for the status
//! command surface (`STATUS_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `status.line.health` (`Durum: {health}`)
//! - `status.line.version` (`OpenCode sürümü: {version}`)
//! - `status.line.pid` (`PID: {pid}`)
//! - `status.line.uptime_sec` (`Çalışma süresi: {seconds} sn`)
//! - `status.line.mode` (`Ajan: {mode}`)
//! - `status.line.model` (`Model: {model}`)
//! - `status.project_selected` (`Proje: {project}`)
//! - `status.session_selected` (`Geçerli oturum: {title}`)
//! - `status.header_running` (`🟢 OpenCode Sunucusu çalışıyor`)
//! - `status.health.healthy` (`Sağlıklı`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/tr.ts` line 140 carries `status.header_running`;
//!   line 141 carries `status.health.healthy`; line 143 carries
//!   `status.line.health`; line 144 carries `status.line.version`;
//!   lines 148-151 carry `status.line.pid` / `status.line.uptime_sec` /
//!   `status.line.mode` / `status.line.model`; line 157 carries
//!   `status.project_selected`; line 161 carries
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
//!   owns only the `ko` status table. No Rust owner embeds the `tr`
//!   status templates yet.
//! - This slice owns only the `tr` status lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `tr.ts` file reads (arrive as explicit `key` args here),
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
//!   slice 147).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice151_locale_tr_status_parity.rs`.

/// Locale code for this table (TS: `tr` in `SUPPORTED_LOCALES`).
pub const TR_LOCALE: &str = "tr";

/// Status keys covered here (same 10-key subset as `DE_STATUS_KEYS`,
/// slice 123, `FR_STATUS_KEYS`, slice 127, `ES_STATUS_KEYS`, slice 131,
/// `IT_STATUS_KEYS`, slice 135, `PT_STATUS_KEYS`, slice 139,
/// `ID_STATUS_KEYS`, slice 143, and `KO_STATUS_KEYS`, slice 147).
pub const TR_STATUS_KEYS: [&str; 10] = [
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

/// Verbatim `tr` templates (TS: `src/i18n/tr.ts` lines 140-161).
pub const TR_STATUS_LINE_HEALTH: &str = "Durum: {health}";
/// Verbatim `tr` version line (TS: `status.line.version`).
pub const TR_STATUS_LINE_VERSION: &str = "OpenCode sürümü: {version}";
/// Verbatim `tr` pid line (TS: `status.line.pid`).
pub const TR_STATUS_LINE_PID: &str = "PID: {pid}";
/// Verbatim `tr` uptime line (TS: `status.line.uptime_sec`).
pub const TR_STATUS_LINE_UPTIME_SEC: &str = "Çalışma süresi: {seconds} sn";
/// Verbatim `tr` agent-mode line (TS: `status.line.mode`).
pub const TR_STATUS_LINE_MODE: &str = "Ajan: {mode}";
/// Verbatim `tr` model line (TS: `status.line.model`).
pub const TR_STATUS_LINE_MODEL: &str = "Model: {model}";
/// Verbatim `tr` project line (TS: `status.project_selected`).
pub const TR_STATUS_PROJECT_SELECTED: &str = "Proje: {project}";
/// Verbatim `tr` session line (TS: `status.session_selected`).
pub const TR_STATUS_SESSION_SELECTED: &str = "Geçerli oturum: {title}";
/// Verbatim `tr` running header (TS: `status.header_running`).
pub const TR_STATUS_HEADER_RUNNING: &str = "🟢 OpenCode Sunucusu çalışıyor";
/// Verbatim `tr` healthy word (TS: `status.health.healthy`).
pub const TR_STATUS_HEALTH_HEALTHY: &str = "Sağlıklı";

/// Verbatim `tr` status template for a key, or `None` outside this slice
/// (TS: `tr[key]` for the 10 status keys above).
pub fn lookup_tr_status_template(key: &str) -> Option<&'static str> {
    match key {
        "status.line.health" => Some(TR_STATUS_LINE_HEALTH),
        "status.line.version" => Some(TR_STATUS_LINE_VERSION),
        "status.line.pid" => Some(TR_STATUS_LINE_PID),
        "status.line.uptime_sec" => Some(TR_STATUS_LINE_UPTIME_SEC),
        "status.line.mode" => Some(TR_STATUS_LINE_MODE),
        "status.line.model" => Some(TR_STATUS_LINE_MODEL),
        "status.project_selected" => Some(TR_STATUS_PROJECT_SELECTED),
        "status.session_selected" => Some(TR_STATUS_SESSION_SELECTED),
        "status.header_running" => Some(TR_STATUS_HEADER_RUNNING),
        "status.health.healthy" => Some(TR_STATUS_HEALTH_HEALTHY),
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

fn interpolate_tr(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `tr` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_tr_status_placeholder_ok(_en_template: &str, _tr_template: &str) -> bool {
    placeholder_set(_en_template) == placeholder_set(_tr_template)
}

/// `Durum: {health}` line with the health word filled in
/// (TS: `t("status.line.health", { health })` call site in
/// `src/bot/commands/status-command.ts`).
pub fn format_tr_status_health_line(_health: &str) -> String {
    interpolate_tr(TR_STATUS_LINE_HEALTH, &[("health", _health)])
}

/// `Model: {model}` line with the model filled in
/// (TS: `t("status.line.model", { model })` call site).
pub fn format_tr_status_model_line(_model: &str) -> String {
    interpolate_tr(TR_STATUS_LINE_MODEL, &[("model", _model)])
}

/// `Proje: {project}` line with the project filled in
/// (TS: `t("status.project_selected", { project })` call site).
pub fn format_tr_status_project_line(_project: &str) -> String {
    interpolate_tr(TR_STATUS_PROJECT_SELECTED, &[("project", _project)])
}
