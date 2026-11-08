//! Runtime bootstrap env-file gates (slice 102, GREEN).
//!
//! Rust port of the pure remainder in `src/runtime/bootstrap.ts`:
//! - `DEFAULT_API_URLS` (`v1` `"http://localhost:4096"`, `v2`
//!   `"http://127.0.0.1:49374"`), `DEFAULT_SERVER_USERNAME` `"opencode"`,
//!   `FALLBACK_MODEL_PROVIDER` `"opencode"`, `FALLBACK_MODEL_ID`
//!   `"big-pickle"`)
//! - `isPositiveInteger` (`/^[1-9]\d*$/`)
//! - `isValidHttpUrl` (`new URL` + `http:`/`https:` protocol gate)
//! - `validateRuntimeEnvValues` (missing/blank token, non-positive-integer
//!   user id, missing/blank provider, missing/blank model id, trimmed
//!   non-empty `OPENCODE_API_URL` URL gate; first failure wins, in that
//!   order)
//! - `normalizeEnvLineEndings` (`\r?\n` split, trailing-`\r` strip, drop
//!   trailing empty lines)
//! - `removeEnvKey` (`^\s*(?:export\s+)?KEY\s*=` line filter)
//! - `parseEnvAssignmentLine`
//!   (`/^(\s*#\s*)?(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$/`,
//!   commented flag set when group 1 matched, raw value is everything
//!   after the first `=`)
//! - `buildTemplateKeySet` (every parsed template line counts, commented
//!   lines included)
//! - `collectActiveEnvAssignments` (non-commented lines only, last value
//!   wins per key)
//! - `collectCustomEnvAssignments` (reverse walk, skip non-assignments /
//!   commented / template keys / already-seen keys, result back in file
//!   order)
//! - `renderEnvAssignment` (`` `${key}=${rawValue}` ``)
//! - `finalizeEnvContent` (`` `${lines.join("\n")}\n` ``)
//! - `buildFlatEnvFileContent` (remove-then-append each wizard key in
//!   `WIZARD_ENV_KEYS` order, skip blank values, keep custom lines)
//! - `buildEnvFileContent` (no/empty template falls back to flat; wizard
//!   overrides win per template line, blank overrides keep the template
//!   line, non-wizard keys fill from existing assignments, custom lines
//!   appended after one blank separator)
//! - `getWizardServerVersionDefault` (no existing file -> `v2`; saved
//!   `OPENCODE_SERVER_VERSION` trimmed + lowercased `=== "v2"` -> `v2`,
//!   else `v1`)
//! - `pickServerPassword` (`answer || saved || undefined`)
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `parseCliArgs` / `normalizeMode` / Telegram client wiring (see
//!   `bot_bootstrap_gates`), health-port / node-version / service-manager
//!   gates (see `runtime_service_gates`), `paths` / `container` (see
//!   `stores_runtime`).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `fs` env-file reads/writes, `readline` wizard prompts,
//!   `process.stdin/stdout.isTTY`, `process.env` reads, `dotenv.parse`
//!   (env content arrives as explicit `&str` args; the `dotenv`-compatible
//!   lookup for the wizard default is a minimal `KEY=value` scan).
//!
//! Deviations: none intended. `dotenv.parse` quote handling is out of
//! scope: parity fixtures use plain `KEY=value` lines where the minimal
//! scan and `dotenv` agree.

/// Default OpenCode URL offered for V1 servers (TS: `DEFAULT_API_URLS.v1`).
pub const DEFAULT_API_URL_V1: &str = "http://localhost:4096";

/// Default OpenCode URL offered for V2 servers (TS: `DEFAULT_API_URLS.v2`).
pub const DEFAULT_API_URL_V2: &str = "http://127.0.0.1:49374";

/// Username offered when the wizard asks for one (TS: `DEFAULT_SERVER_USERNAME`).
pub const DEFAULT_SERVER_USERNAME: &str = "opencode";

/// Model provider fallback when `.env.example` has none (TS: `FALLBACK_MODEL_PROVIDER`).
pub const FALLBACK_MODEL_PROVIDER: &str = "opencode";

/// Model id fallback when `.env.example` has none (TS: `FALLBACK_MODEL_ID`).
pub const FALLBACK_MODEL_ID: &str = "big-pickle";

/// OpenCode server version chosen in the wizard (TS: `OpencodeServerVersion`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerVersion {
    V1,
    V2,
}

impl ServerVersion {
    fn as_str(self) -> &'static str {
        match self {
            ServerVersion::V1 => "v1",
            ServerVersion::V2 => "v2",
        }
    }
}

/// Wizard `.env` values written by the setup flow (TS: `WizardEnvValues`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizardEnvValues {
    pub locale: String,
    pub token: String,
    pub allowed_user_id: String,
    pub server_version: ServerVersion,
    pub api_url: Option<String>,
    pub server_username: String,
    pub server_password: Option<String>,
    pub model_provider: String,
    pub model_id: String,
}

/// Result of `validateRuntimeEnvValues` (TS: `EnvValidationResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvValidation {
    pub is_valid: bool,
    pub reason: Option<String>,
}

/// One parsed `.env` assignment line (TS: `ParsedEnvAssignmentLine`
/// without the echo of the source line, which the caller already has).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEnvAssignment {
    pub key: String,
    pub raw_value: String,
    pub is_commented: bool,
}

/// `true` for `1`, `2`, ... with no leading zero or extras
/// (TS: `isPositiveInteger`: `/^[1-9]\d*$/`).
pub fn is_positive_integer(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    if !(b'1'..=b'9').contains(&bytes[0]) {
        return false;
    }
    bytes[1..].iter().all(|b| b.is_ascii_digit())
}

/// `true` when `value` parses as a URL with an `http:`/`https:` protocol
/// (TS: `isValidHttpUrl`).
pub fn is_valid_http_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let rest = if lower.starts_with("http://") {
        &value["http://".len()..]
    } else if lower.starts_with("https://") {
        &value["https://".len()..]
    } else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    if rest.bytes().any(|b| b.is_ascii_whitespace()) {
        return false;
    }
    let host_end = rest
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(rest.len());
    !rest[..host_end].is_empty()
}

/// First-failure validation in TS order: token, user id, provider, model
/// id, then the trimmed API URL gate (TS: `validateRuntimeEnvValues`).
pub fn validate_runtime_env(
    token: Option<&str>,
    allowed_user_id: Option<&str>,
    model_provider: Option<&str>,
    model_id: Option<&str>,
    api_url: Option<&str>,
) -> EnvValidation {
    let invalid = |reason: &str| EnvValidation {
        is_valid: false,
        reason: Some(reason.to_string()),
    };
    if token.map(str::trim).unwrap_or("").is_empty() {
        return invalid("Missing TELEGRAM_BOT_TOKEN");
    }
    if !is_positive_integer(allowed_user_id.unwrap_or("")) {
        return invalid("Invalid TELEGRAM_ALLOWED_USER_ID");
    }
    if model_provider.map(str::trim).unwrap_or("").is_empty() {
        return invalid("Missing OPENCODE_MODEL_PROVIDER");
    }
    if model_id.map(str::trim).unwrap_or("").is_empty() {
        return invalid("Missing OPENCODE_MODEL_ID");
    }
    let api = api_url.map(str::trim).unwrap_or("");
    if !api.is_empty() && !is_valid_http_url(api) {
        return invalid("Invalid OPENCODE_API_URL");
    }
    EnvValidation {
        is_valid: true,
        reason: None,
    }
}

/// Split on line breaks and drop trailing empty lines
/// (TS: `normalizeEnvLineEndings`).
pub fn normalize_env_line_endings(content: &str) -> Vec<String> {
    let mut lines: Vec<String> = content
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

fn line_assigns_key(line: &str, key: &str) -> bool {
    let mut rest = line.trim_start();
    if let Some(after_export) = rest.strip_prefix("export") {
        if let Some(first) = after_export.chars().next() {
            if first.is_whitespace() {
                rest = after_export.trim_start();
            }
        }
    }
    let Some(after_key) = rest.strip_prefix(key) else {
        return false;
    };
    after_key.trim_start().starts_with('=')
}

/// Drop every line assigning `key`, with or without `export`
/// (TS: `removeEnvKey`).
pub fn remove_env_key(lines: &[String], key: &str) -> Vec<String> {
    lines
        .iter()
        .filter(|line| !line_assigns_key(line, key))
        .cloned()
        .collect()
}

fn is_env_key_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_env_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Parse a `KEY=value` line, including `export` and `#`-commented forms
/// (TS: `parseEnvAssignmentLine`).
pub fn parse_env_assignment_line(line: &str) -> Option<ParsedEnvAssignment> {
    // G1: /^(\s*#\s*)?/ — only consumes when a `#` follows leading blanks.
    let (mut rest, is_commented) = match consume_comment_prefix(line) {
        Some(remainder) => (remainder, true),
        None => (line, false),
    };
    // `(?:export\s+)?`
    if let Some(after_export) = rest.strip_prefix("export") {
        if let Some(first) = after_export.chars().next() {
            if first.is_whitespace() {
                rest = after_export.trim_start();
            }
        }
    }
    // `([A-Za-z_][A-Za-z0-9_]*)`
    let mut chars = rest.char_indices();
    let Some((_, first)) = chars.next() else {
        return None;
    };
    if !is_env_key_start(first) {
        return None;
    }
    let mut key_end = first.len_utf8();
    for (index, c) in chars {
        if !is_env_key_char(c) {
            key_end = index;
            break;
        }
        key_end = index + c.len_utf8();
        if key_end == rest.len() {
            break;
        }
    }
    if key_end > rest.len() {
        return None;
    }
    let key = &rest[..key_end];
    // `\s*=(.*)$`
    let after_key = rest[key_end..].trim_start();
    let raw_value = after_key.strip_prefix('=')?;
    Some(ParsedEnvAssignment {
        key: key.to_string(),
        raw_value: raw_value.to_string(),
        is_commented,
    })
}

/// Consume a leading `^\s*#\s*` prefix, returning the remainder.
/// Returns `None` when the line has no `#` after leading whitespace,
/// mirroring the optional `(\s*#\s*)?` group failing to match.
fn consume_comment_prefix(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let after_hash = trimmed.strip_prefix('#')?;
    Some(after_hash.trim_start())
}

/// Keys assigned anywhere in the template, commented lines included
/// (TS: `buildTemplateKeySet`).
pub fn build_template_key_set(template_content: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for line in normalize_env_line_endings(template_content) {
        if let Some(parsed) = parse_env_assignment_line(&line) {
            if !keys.contains(&parsed.key) {
                keys.push(parsed.key);
            }
        }
    }
    keys
}

/// Active (non-commented) assignments, last value wins per key
/// (TS: `collectActiveEnvAssignments`).
pub fn collect_active_env_assignments(content: &str) -> Vec<(String, String)> {
    let mut assignments: Vec<(String, String)> = Vec::new();
    for line in normalize_env_line_endings(content) {
        let Some(parsed) = parse_env_assignment_line(&line) else {
            continue;
        };
        if parsed.is_commented {
            continue;
        }
        if let Some(slot) = assignments.iter_mut().find(|(key, _)| *key == parsed.key) {
            slot.1 = parsed.raw_value;
        } else {
            assignments.push((parsed.key, parsed.raw_value));
        }
    }
    assignments
}

/// Custom lines (non-template keys, last occurrence each) back in file
/// order (TS: `collectCustomEnvAssignments`).
pub fn collect_custom_env_assignments(
    existing_content: &str,
    template_keys: &[String],
) -> Vec<String> {
    let lines = normalize_env_line_endings(existing_content);
    let mut picked: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for line in lines.iter().rev() {
        let Some(parsed) = parse_env_assignment_line(line) else {
            continue;
        };
        if parsed.is_commented || template_keys.contains(&parsed.key) || seen.contains(&parsed.key) {
            continue;
        }
        seen.push(parsed.key);
        picked.push(line.clone());
    }
    picked.reverse();
    picked
}

/// `` `${key}=${rawValue}` `` (TS: `renderEnvAssignment`).
pub fn render_env_assignment(key: &str, raw_value: &str) -> String {
    format!("{key}={raw_value}")
}

/// Lines joined with `\n` plus one trailing newline
/// (TS: `finalizeEnvContent`).
pub fn finalize_env_content(lines: &[String]) -> String {
    format!("{}\n", lines.join("\n"))
}

fn wizard_ordered_values(values: &WizardEnvValues) -> Vec<(String, String)> {
    vec![
        ("BOT_LOCALE".to_string(), values.locale.clone()),
        ("TELEGRAM_BOT_TOKEN".to_string(), values.token.clone()),
        (
            "TELEGRAM_ALLOWED_USER_ID".to_string(),
            values.allowed_user_id.clone(),
        ),
        (
            "OPENCODE_SERVER_VERSION".to_string(),
            values.server_version.as_str().to_string(),
        ),
        (
            "OPENCODE_API_URL".to_string(),
            values.api_url.clone().unwrap_or_default(),
        ),
        (
            "OPENCODE_SERVER_USERNAME".to_string(),
            values.server_username.clone(),
        ),
        (
            "OPENCODE_SERVER_PASSWORD".to_string(),
            values.server_password.clone().unwrap_or_default(),
        ),
        (
            "OPENCODE_MODEL_PROVIDER".to_string(),
            values.model_provider.clone(),
        ),
        ("OPENCODE_MODEL_ID".to_string(), values.model_id.clone()),
    ]
}

/// Flat rewrite: remove-then-append each wizard key in order, skipping
/// blank values (TS: `buildFlatEnvFileContent`).
pub fn build_flat_env_file_content(
    existing_content: &str,
    values: &WizardEnvValues,
) -> String {
    let mut lines = normalize_env_line_endings(existing_content);
    for (key, value) in wizard_ordered_values(values) {
        lines = remove_env_key(&lines, &key);
        if !value.trim().is_empty() {
            lines.push(render_env_assignment(&key, &value));
        }
    }
    finalize_env_content(&lines)
}

/// Template-shaped rewrite with custom-line carry-over, flat fallback
/// when the template is missing/empty (TS: `buildEnvFileContent`).
pub fn build_env_file_content(
    existing_content: &str,
    values: &WizardEnvValues,
    env_example_content: Option<&str>,
) -> String {
    let template = match env_example_content {
        None => return build_flat_env_file_content(existing_content, values),
        Some(content) if content.is_empty() => {
            return build_flat_env_file_content(existing_content, values)
        }
        Some(content) => content,
    };
    let template_lines = normalize_env_line_endings(template);
    if template_lines.is_empty() {
        return build_flat_env_file_content(existing_content, values);
    }

    let template_keys = build_template_key_set(template);
    let existing_assignments = collect_active_env_assignments(existing_content);
    let overrides = wizard_ordered_values(values);

    let mut rendered: Vec<String> = Vec::with_capacity(template_lines.len());
    for line in &template_lines {
        let Some(parsed) = parse_env_assignment_line(line) else {
            rendered.push(line.clone());
            continue;
        };
        if let Some((_, override_value)) =
            overrides.iter().find(|(key, _)| *key == parsed.key)
        {
            if !override_value.trim().is_empty() {
                rendered.push(render_env_assignment(&parsed.key, override_value));
            } else {
                rendered.push(line.clone());
            }
            continue;
        }
        if let Some((_, raw)) = existing_assignments
            .iter()
            .find(|(key, _)| *key == parsed.key)
        {
            rendered.push(render_env_assignment(&parsed.key, raw));
        } else {
            rendered.push(line.clone());
        }
    }

    let custom = collect_custom_env_assignments(existing_content, &template_keys);
    if !custom.is_empty() {
        if rendered.last().is_some_and(|line| !line.is_empty()) {
            rendered.push(String::new());
        }
        rendered.extend(custom);
    }
    finalize_env_content(&rendered)
}

fn lookup_env_value(content: &str, key: &str) -> Option<String> {
    let mut found: Option<String> = None;
    for line in normalize_env_line_endings(content) {
        if let Some(parsed) = parse_env_assignment_line(&line) {
            if !parsed.is_commented && parsed.key == key {
                found = Some(parsed.raw_value);
            }
        }
    }
    found
}

/// `v2` on first setup, else the saved choice (`v2` only when the saved
/// value trims + lowercases to `"v2"`) (TS:
/// `getWizardServerVersionDefault`).
pub fn wizard_server_version_default(existing_env_content: Option<&str>) -> ServerVersion {
    let Some(content) = existing_env_content else {
        return ServerVersion::V2;
    };
    match lookup_env_value(content, "OPENCODE_SERVER_VERSION") {
        Some(saved) if saved.trim().to_ascii_lowercase() == "v2" => ServerVersion::V2,
        _ => ServerVersion::V1,
    }
}

/// Typed answer wins, else the saved password, else none
/// (TS: `pickServerPassword`: `answer || saved || undefined`).
pub fn pick_server_password(answer: &str, saved_password: Option<&str>) -> Option<String> {
    if !answer.is_empty() {
        return Some(answer.to_string());
    }
    match saved_password {
        Some(saved) if !saved.is_empty() => Some(saved.to_string()),
        _ => None,
    }
}
