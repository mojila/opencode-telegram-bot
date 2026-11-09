//! French status-table gates (slice 127, RED).
//!
//! Rust port of the pure remainder in `src/i18n/fr.ts` for the status
//! command surface (`STATUS_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `status.line.health` (`Statut : {health}`)
//! - `status.line.version` (`Version OpenCode : {version}`)
//! - `status.line.pid` (`PID : {pid}`)
//! - `status.line.uptime_sec` (`Temps de fonctionnement : {seconds} sec`)
//! - `status.line.mode` (`Agent : {mode}`)
//! - `status.line.model` (`Modèle : {model}`)
//! - `status.project_selected` (`Projet : {project}`)
//! - `status.session_selected` (`Session actuelle : {title}`)
//! - `status.header_running` (`🟢 Le serveur OpenCode est en cours d'exécution`)
//! - `status.health.healthy` (`Sain`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/fr.ts` lines 148-149 + 151-152 + 156-159 + 165 + 171 carry
//!   the templates above verbatim; `src/i18n/en.ts` is the placeholder
//!   reference (`en` keeps `Status: {health}`, `OpenCode version:
//!   {version}`, `Uptime: {seconds} sec`, `Model: {model}`, `Healthy`, ...).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only;
//!   `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates; `locale_de_status_gates` (slice 123) owns only the `de`
//!   status table. No Rust owner embeds the `fr` status templates yet.
//! - This slice owns only the `fr` status lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `fr.ts` file reads (arrive as explicit `key` args here),
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
//!   slice 123).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice127_locale_fr_status_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locale code for this table (TS: `fr` in `SUPPORTED_LOCALES`).
pub const FR_LOCALE: &str = "fr";

/// Status keys covered here (same 10-key subset as `DE_STATUS_KEYS`,
/// slice 123).
pub const FR_STATUS_KEYS: [&str; 10] = [
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

/// Verbatim `fr` templates (TS: `src/i18n/fr.ts` lines 148-171).
pub const FR_STATUS_LINE_HEALTH: &str = "Statut : {health}";
/// Verbatim `fr` version line (TS: `status.line.version`).
pub const FR_STATUS_LINE_VERSION: &str = "Version OpenCode : {version}";
/// Verbatim `fr` pid line (TS: `status.line.pid`).
pub const FR_STATUS_LINE_PID: &str = "PID : {pid}";
/// Verbatim `fr` uptime line (TS: `status.line.uptime_sec`).
pub const FR_STATUS_LINE_UPTIME_SEC: &str = "Temps de fonctionnement : {seconds} sec";
/// Verbatim `fr` agent-mode line (TS: `status.line.mode`).
pub const FR_STATUS_LINE_MODE: &str = "Agent : {mode}";
/// Verbatim `fr` model line (TS: `status.line.model`).
pub const FR_STATUS_LINE_MODEL: &str = "Modèle : {model}";
/// Verbatim `fr` project line (TS: `status.project_selected`).
pub const FR_STATUS_PROJECT_SELECTED: &str = "Projet : {project}";
/// Verbatim `fr` session line (TS: `status.session_selected`).
pub const FR_STATUS_SESSION_SELECTED: &str = "Session actuelle : {title}";
/// Verbatim `fr` running header (TS: `status.header_running`).
pub const FR_STATUS_HEADER_RUNNING: &str = "🟢 Le serveur OpenCode est en cours d'exécution";
/// Verbatim `fr` healthy word (TS: `status.health.healthy`).
pub const FR_STATUS_HEALTH_HEALTHY: &str = "Sain";

/// Verbatim `fr` status template for a key, or `None` outside this slice
/// (TS: `fr[key]` for the 10 status keys above).
pub fn lookup_fr_status_template(key: &str) -> Option<&'static str> {
    match key {
        "status.line.health" => Some(FR_STATUS_LINE_HEALTH),
        "status.line.version" => Some(FR_STATUS_LINE_VERSION),
        "status.line.pid" => Some(FR_STATUS_LINE_PID),
        "status.line.uptime_sec" => Some(FR_STATUS_LINE_UPTIME_SEC),
        "status.line.mode" => Some(FR_STATUS_LINE_MODE),
        "status.line.model" => Some(FR_STATUS_LINE_MODEL),
        "status.project_selected" => Some(FR_STATUS_PROJECT_SELECTED),
        "status.session_selected" => Some(FR_STATUS_SESSION_SELECTED),
        "status.header_running" => Some(FR_STATUS_HEADER_RUNNING),
        "status.health.healthy" => Some(FR_STATUS_HEALTH_HEALTHY),
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

fn interpolate_fr(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `fr` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_fr_status_placeholder_ok(en_template: &str, fr_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(fr_template)
}

/// `Statut : {health}` line with the health word filled in
/// (TS: `t("status.line.health", { health })` call site in
/// `src/bot/commands/status-command.ts`).
pub fn format_fr_status_health_line(health: &str) -> String {
    interpolate_fr(FR_STATUS_LINE_HEALTH, &[("health", health)])
}

/// `Modèle : {model}` line with the model filled in
/// (TS: `t("status.line.model", { model })` call site).
pub fn format_fr_status_model_line(model: &str) -> String {
    interpolate_fr(FR_STATUS_LINE_MODEL, &[("model", model)])
}

/// `Projet : {project}` line with the project filled in
/// (TS: `t("status.project_selected", { project })` call site).
pub fn format_fr_status_project_line(project: &str) -> String {
    interpolate_fr(FR_STATUS_PROJECT_SELECTED, &[("project", project)])
}
