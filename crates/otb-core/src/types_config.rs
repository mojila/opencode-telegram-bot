//! Pure type/config logic (slice 21b, GREEN).
//!
//! Rust port of the pure helpers behind:
//! - `src/app/types/agent.ts` (`AGENT_EMOJI`, `getAgentEmoji`,
//!   `getAgentDisplayName`, `getAgentButtonLabel`)
//! - `src/app/types/model.ts` (`isSameModel`, `formatModelForButton`,
//!   `formatModelForDisplay`)
//! - `src/config.ts` (boolean/int/locale/message-mode/path-list/TTS/
//!   server-mode/server-version/API-URL/telegram-proxy/health-port/
//!   initial-settings-preset env parsing)
//! - `src/bot/commands/definitions.ts` (V1/V2 command filtering)
//! - `src/app/stores/settings-store.ts` pure subset (display defaults,
//!   prompt-queue defaults per server version, legacy `ttsEnabled`
//!   migration, TTS provider defaults)
//!
//! TS -> Rust name mapping:
//! - `getAgentEmoji` -> [`get_agent_emoji`]
//! - `getAgentDisplayName` -> [`get_agent_display_name`]
//! - `getAgentButtonLabel` -> [`get_agent_button_label`]
//! - `isSameModel` -> [`is_same_model`]
//! - `formatModelForButton` -> [`format_model_for_button`]
//! - `formatModelForDisplay` -> [`format_model_for_display`]
//! - `resolveOpencodeServerMode` -> [`resolve_server_mode`]
//! - `resolveOpencodeApiUrl` -> [`resolve_api_url`]
//! - `parseInitialSettingsPreset` -> [`parse_initial_settings_preset`]
//! - `buildTelegramConfig` -> [`validate_telegram_proxy`] +
//!   [`normalize_api_root`]
//! - `BOT_COMMANDS` / `BUILT_IN_COMMAND_NAMES` filtering ->
//!   [`active_commands`]

// ---------------------------------------------------------------------------
// Agent types (`src/app/types/agent.ts`)
// ---------------------------------------------------------------------------

/// Agent emoji table (TS: `AGENT_EMOJI`).
pub fn agent_emoji_table() -> Vec<(&'static str, &'static str)> {
    vec![
        ("plan", "📋"),
        ("build", "🛠️"),
        ("general", "💬"),
        ("explore", "🔍"),
        ("title", "📝"),
        ("summary", "📄"),
        ("compaction", "📦"),
    ]
}

/// Emoji for an agent, fallback `🤖` (TS: `getAgentEmoji`).
pub fn get_agent_emoji(agent_name: &str) -> &'static str {
    agent_emoji_table()
        .into_iter()
        .find(|(name, _)| *name == agent_name)
        .map(|(_, emoji)| emoji)
        .unwrap_or("🤖")
}

/// Display name with emoji (TS: `getAgentDisplayName`).
pub fn get_agent_display_name(agent_name: &str) -> String {
    let emoji = get_agent_emoji(agent_name);
    let mut chars = agent_name.chars();
    let capitalized = match chars.next() {
        Some(first) => {
            let mut out: String = first.to_uppercase().collect();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    };
    format!("{emoji} {capitalized}")
}

/// Reply-keyboard label with ` Agent` suffix (TS: `getAgentButtonLabel`).
pub fn get_agent_button_label(agent_name: &str) -> String {
    format!("{} Agent", get_agent_display_name(agent_name))
}

// ---------------------------------------------------------------------------
// Model types (`src/app/types/model.ts`)
// ---------------------------------------------------------------------------

/// Model identity (TS: `ModelInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

/// Same provider, model and variant (TS: `isSameModel`).
pub fn is_same_model(left: &ModelInfo, right: &ModelInfo) -> bool {
    left.provider_id == right.provider_id
        && left.model_id == right.model_id
        && left.variant == right.variant
}

/// Truncate to `keep` chars + `...` when longer than `max` (TS: `substring`).
fn truncate_with_ellipsis(value: &str, max: usize, keep: usize) -> String {
    if value.chars().count() > max {
        format!("{}...", value.chars().take(keep).collect::<String>())
    } else {
        value.to_string()
    }
}

/// Compact button format (TS: `formatModelForButton`).
pub fn format_model_for_button(provider_id: &str, model_id: &str) -> String {
    let display_provider_id = truncate_with_ellipsis(provider_id, 15, 12);
    let display_model_id = truncate_with_ellipsis(model_id, 20, 17);
    format!("🧠 {display_provider_id}\n{display_model_id}")
}

/// Full display format `provider / model` (TS: `formatModelForDisplay`).
pub fn format_model_for_display(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id} / {model_id}")
}

// ---------------------------------------------------------------------------
// Config env parsing (`src/config.ts`)
// ---------------------------------------------------------------------------

/// `Number.parseInt(value, 10)` semantics: leading whitespace skipped,
/// optional sign, then a run of ASCII digits; `None` when no digits.
fn js_parse_int(raw: &str) -> Option<i64> {
    let trimmed = raw.trim_start();
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits: Vec<u8> = rest.bytes().take_while(u8::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let mut value: i64 = 0;
    for digit in digits {
        value = value
            .saturating_mul(10)
            .saturating_add((digit - b'0') as i64);
    }
    Some(if negative {
        value.saturating_neg()
    } else {
        value
    })
}

/// Boolean env parsing: `1/true/yes/on` -> true, `0/false/no/off` ->
/// false, missing/invalid -> default.
pub fn parse_bool_env(raw: Option<&str>, default: bool) -> bool {
    let value = match raw {
        Some(value) if !value.is_empty() => value,
        _ => return default,
    };
    match value.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

/// Positive-int env parsing; missing/invalid/non-positive -> default.
pub fn parse_positive_int_env(raw: Option<&str>, default: i64) -> i64 {
    match raw.and_then(|value| {
        if value.is_empty() {
            None
        } else {
            js_parse_int(value)
        }
    }) {
        Some(parsed) if parsed > 0 => parsed,
        _ => default,
    }
}

/// Non-negative-int env parsing (accepts 0); missing/invalid/negative ->
/// default.
pub fn parse_non_negative_int_env(raw: Option<&str>, default: i64) -> i64 {
    match raw.and_then(|value| {
        if value.is_empty() {
            None
        } else {
            js_parse_int(value)
        }
    }) {
        Some(parsed) if parsed >= 0 => parsed,
        _ => default,
    }
}

/// Message format mode: `raw`/`markdown` (case-insensitive), else default.
pub fn parse_message_format_mode(raw: Option<&str>) -> &'static str {
    match raw {
        Some(value) if !value.is_empty() => match value.trim().to_lowercase().as_str() {
            "raw" => "raw",
            "markdown" => "markdown",
            _ => "markdown",
        },
        _ => "markdown",
    }
}

/// Comma-separated path list: trim entries, drop empties.
pub fn parse_path_list(raw: Option<&str>) -> Vec<String> {
    match raw {
        Some(value) if !value.trim().is_empty() => value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

// --- Minimal JSON validation for `parse_initial_settings_preset` ---
// No serde dependency: accept exactly what `JSON.parse` accepts for the
// shapes that matter here (object vs non-object vs invalid).

fn skip_json_ws(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() && matches!(bytes[*pos], b' ' | b'\t' | b'\n' | b'\r') {
        *pos += 1;
    }
}

fn parse_json_string(bytes: &[u8], pos: &mut usize) -> Result<(), ()> {
    // Caller guarantees bytes[*pos] == b'"'.
    *pos += 1;
    while let Some(&byte) = bytes.get(*pos) {
        *pos += 1;
        match byte {
            b'"' => return Ok(()),
            b'\\' => match bytes.get(*pos) {
                Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => *pos += 1,
                Some(b'u') => {
                    *pos += 1;
                    for _ in 0..4 {
                        match bytes.get(*pos) {
                            Some(c) if c.is_ascii_hexdigit() => *pos += 1,
                            _ => return Err(()),
                        }
                    }
                }
                _ => return Err(()),
            },
            0x00..=0x1F => return Err(()),
            _ => {}
        }
    }
    Err(())
}

fn parse_json_number(bytes: &[u8], pos: &mut usize) -> Result<(), ()> {
    let start = *pos;
    if bytes.get(*pos) == Some(&b'-') {
        *pos += 1;
    }
    match bytes.get(*pos) {
        Some(b'0') => *pos += 1,
        Some(c) if c.is_ascii_digit() => {
            while bytes.get(*pos).is_some_and(u8::is_ascii_digit) {
                *pos += 1;
            }
        }
        _ => return Err(()),
    }
    if bytes.get(*pos) == Some(&b'.') {
        *pos += 1;
        if !bytes.get(*pos).is_some_and(u8::is_ascii_digit) {
            return Err(());
        }
        while bytes.get(*pos).is_some_and(u8::is_ascii_digit) {
            *pos += 1;
        }
    }
    if matches!(bytes.get(*pos), Some(b'e' | b'E')) {
        *pos += 1;
        if matches!(bytes.get(*pos), Some(b'+' | b'-')) {
            *pos += 1;
        }
        if !bytes.get(*pos).is_some_and(u8::is_ascii_digit) {
            return Err(());
        }
        while bytes.get(*pos).is_some_and(u8::is_ascii_digit) {
            *pos += 1;
        }
    }
    if *pos > start {
        Ok(())
    } else {
        Err(())
    }
}

fn parse_json_literal(bytes: &[u8], pos: &mut usize, word: &[u8]) -> Result<(), ()> {
    if bytes[*pos..].starts_with(word) {
        *pos += word.len();
        Ok(())
    } else {
        Err(())
    }
}

/// Parsed JSON shape; only object-vs-other matters to the caller.
#[derive(PartialEq, Eq)]
enum JsonShape {
    Object,
    Other,
}

fn parse_json_value(bytes: &[u8], pos: &mut usize) -> Result<JsonShape, ()> {
    skip_json_ws(bytes, pos);
    match bytes.get(*pos) {
        Some(b'{') => {
            *pos += 1;
            skip_json_ws(bytes, pos);
            if bytes.get(*pos) == Some(&b'}') {
                *pos += 1;
                return Ok(JsonShape::Object);
            }
            loop {
                skip_json_ws(bytes, pos);
                if bytes.get(*pos) != Some(&b'"') {
                    return Err(());
                }
                parse_json_string(bytes, pos)?;
                skip_json_ws(bytes, pos);
                if bytes.get(*pos) != Some(&b':') {
                    return Err(());
                }
                *pos += 1;
                parse_json_value(bytes, pos)?;
                skip_json_ws(bytes, pos);
                match bytes.get(*pos) {
                    Some(b',') => *pos += 1,
                    Some(b'}') => {
                        *pos += 1;
                        return Ok(JsonShape::Object);
                    }
                    _ => return Err(()),
                }
            }
        }
        Some(b'[') => {
            *pos += 1;
            skip_json_ws(bytes, pos);
            if bytes.get(*pos) == Some(&b']') {
                *pos += 1;
                return Ok(JsonShape::Other);
            }
            loop {
                parse_json_value(bytes, pos)?;
                skip_json_ws(bytes, pos);
                match bytes.get(*pos) {
                    Some(b',') => *pos += 1,
                    Some(b']') => {
                        *pos += 1;
                        return Ok(JsonShape::Other);
                    }
                    _ => return Err(()),
                }
            }
        }
        Some(b'"') => parse_json_string(bytes, pos).map(|()| JsonShape::Other),
        Some(b't') => parse_json_literal(bytes, pos, b"true").map(|()| JsonShape::Other),
        Some(b'f') => parse_json_literal(bytes, pos, b"false").map(|()| JsonShape::Other),
        Some(b'n') => parse_json_literal(bytes, pos, b"null").map(|()| JsonShape::Other),
        Some(b'-') | Some(b'0'..=b'9') => {
            parse_json_number(bytes, pos).map(|()| JsonShape::Other)
        }
        _ => Err(()),
    }
}

/// `INITIAL_SETTINGS_PRESET`: empty -> `{}`, invalid JSON / non-object ->
/// `Err`. Returns the raw object JSON on success.
pub fn parse_initial_settings_preset(raw: Option<&str>) -> Result<String, String> {
    let trimmed = raw.map(str::trim).unwrap_or("");
    if trimmed.is_empty() {
        return Ok("{}".to_string());
    }
    let bytes = trimmed.as_bytes();
    let mut pos = 0;
    match parse_json_value(bytes, &mut pos) {
        Ok(JsonShape::Object) => {
            skip_json_ws(bytes, &mut pos);
            if pos == bytes.len() {
                Ok(trimmed.to_string())
            } else {
                Err(
                    "INITIAL_SETTINGS_PRESET contains invalid JSON. Fix or unset the variable."
                        .to_string(),
                )
            }
        }
        Ok(JsonShape::Other) => {
            Err("INITIAL_SETTINGS_PRESET must be a JSON object.".to_string())
        }
        Err(()) => Err(
            "INITIAL_SETTINGS_PRESET contains invalid JSON. Fix or unset the variable.".to_string(),
        ),
    }
}

/// Supported locale codes (TS: `SUPPORTED_LOCALES`).
fn supported_locales() -> &'static [&'static str] {
    &[
        "en", "ar", "de", "es", "fr", "id", "it", "ko", "pt", "ru", "tr", "zh",
    ]
}

/// Locale: normalize regional tags (`ru-RU` -> `ru`), unsupported ->
/// default.
pub fn normalize_locale_tag(raw: Option<&str>, default: &str) -> String {
    let normalized = raw.map(|value| value.trim().to_lowercase()).unwrap_or_default();
    if normalized.is_empty() {
        return default.to_string();
    }
    if supported_locales().contains(&normalized.as_str()) {
        return normalized;
    }
    let base = normalized.split('-').next().unwrap_or("");
    if supported_locales().contains(&base) {
        return base.to_string();
    }
    default.to_string()
}

/// Server version: `v2` (case/whitespace-insensitive) else `v1`.
pub fn resolve_server_version(raw: Option<&str>) -> &'static str {
    match raw {
        Some(value) if value.trim().to_lowercase() == "v2" => "v2",
        _ => "v1",
    }
}

/// Server mode: V1 always `service`; V2 empty -> `service`,
/// `standalone` accepted, unknown -> `Err` (no secret echo).
pub fn resolve_server_mode(version: &str, raw: Option<&str>) -> Result<&'static str, String> {
    if version != "v2" {
        return Ok("service");
    }
    let normalized = raw.map(|value| value.trim().to_lowercase()).unwrap_or_default();
    if normalized.is_empty() || normalized == "service" {
        return Ok("service");
    }
    if normalized == "standalone" {
        return Ok("standalone");
    }
    Err("Invalid OPENCODE_SERVER_MODE. Expected service or standalone. Refusing to start, because an unknown value must not enable the V2 background server.".to_string())
}

/// API URL: standalone without explicit URL -> `Err`; else explicit or
/// version default (`v1` -> `http://localhost:4096`,
/// `v2` -> `http://127.0.0.1:49374`).
pub fn resolve_api_url(
    version: &str,
    mode: &str,
    from_env: Option<&str>,
) -> Result<String, String> {
    let explicit = from_env.map(str::trim).unwrap_or("");
    if version == "v2" && mode == "standalone" && explicit.is_empty() {
        return Err("OPENCODE_SERVER_MODE=standalone requires OPENCODE_API_URL. Refusing to default to the V2 service port.".to_string());
    }
    if !explicit.is_empty() {
        return Ok(explicit.to_string());
    }
    Ok(if version == "v2" {
        "http://127.0.0.1:49374"
    } else {
        "http://localhost:4096"
    }
    .to_string())
}

/// Strip trailing slashes from `TELEGRAM_API_ROOT`.
pub fn normalize_api_root(raw: &str) -> String {
    raw.trim_end_matches('/').to_string()
}

/// Telegram connectivity validation: proxy URL + API root conflict ->
/// `Err`; proxy secret without API root -> `Err`; else `Ok`.
pub fn validate_telegram_proxy(
    proxy_url: &str,
    api_root: &str,
    proxy_secret: &str,
) -> Result<(), String> {
    if !proxy_url.is_empty() && !api_root.is_empty() {
        return Err("TELEGRAM_PROXY_URL and TELEGRAM_API_ROOT are alternative connectivity modes and cannot be used together. Pick one.".to_string());
    }
    if !proxy_secret.is_empty() && api_root.is_empty() {
        return Err("TELEGRAM_PROXY_SECRET requires TELEGRAM_API_ROOT to be set.".to_string());
    }
    Ok(())
}

/// Supported TTS providers (TS: `VALID_TTS_PROVIDERS`).
fn valid_tts_providers() -> &'static [&'static str] {
    &["openai", "google", "elevenlabs", "edge"]
}

/// TTS provider parsing with default fallback.
pub fn parse_tts_provider(raw: Option<&str>, default: &str) -> String {
    match raw {
        Some(value) if !value.is_empty() => {
            let normalized = value.trim().to_lowercase();
            if valid_tts_providers().contains(&normalized.as_str()) {
                return normalized;
            }
            default.to_string()
        }
        _ => default.to_string(),
    }
}

/// Default TTS voice per provider.
pub fn tts_default_voice(provider: &str) -> &'static str {
    match provider {
        "google" => "en-US-Studio-O",
        "elevenlabs" => "21m00Tcm4TlvDq8ikWAM",
        "edge" => "en-US-EmmaMultilingualNeural",
        _ => "alloy",
    }
}

/// Default TTS model per provider.
pub fn tts_default_model(provider: &str) -> &'static str {
    if provider == "elevenlabs" {
        "eleven_flash_v2_5"
    } else {
        "gpt-4o-mini-tts"
    }
}

const MAX_PORT: u32 = 65535;
const CONTAINER_DEFAULT_HEALTH_PORT: u16 = 3100;

/// Health port: valid 1-65535 kept, 0/out-of-range/empty -> 0 outside a
/// container, 3100 inside a container.
pub fn resolve_health_port(raw: Option<&str>, in_container: bool) -> u16 {
    let default_port = if in_container {
        CONTAINER_DEFAULT_HEALTH_PORT
    } else {
        0
    };
    let value = raw.map(str::trim).unwrap_or("");
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return default_port;
    }
    let mut port: u32 = 0;
    for byte in value.bytes() {
        port = port.saturating_mul(10).saturating_add((byte - b'0') as u32);
    }
    if port <= MAX_PORT {
        port as u16
    } else {
        default_port
    }
}

// ---------------------------------------------------------------------------
// Bot commands (`src/bot/commands/definitions.ts`)
// ---------------------------------------------------------------------------

/// All command definitions with their required server version, if any.
fn command_definitions() -> Vec<(&'static str, Option<&'static str>)> {
    vec![
        ("status", None),
        ("new", None),
        ("abort", None),
        ("detach", None),
        ("sessions", None),
        ("recent", None),
        ("messages", None),
        ("settings", None),
        ("projects", None),
        ("worktree", None),
        ("task", None),
        ("tasklist", None),
        ("rename", None),
        ("commands", None),
        ("skills", None),
        ("mcps", None),
        ("opencode_start", None),
        ("opencode_stop", None),
        ("reload", Some("v2")),
        ("goal", Some("v2")),
        ("open", None),
        ("ls", None),
        ("help", None),
    ]
}

/// Command names active for a server version (`reload`/`goal` V2-only).
pub fn active_commands(server_version: &str) -> Vec<String> {
    command_definitions()
        .into_iter()
        .filter(|(_, required)| required.map_or(true, |version| version == server_version))
        .map(|(command, _)| command.to_string())
        .collect()
}

/// Built-in command names (`start` + active) for a server version.
pub fn builtin_command_names(server_version: &str) -> Vec<String> {
    let mut names = vec!["start".to_string()];
    names.extend(active_commands(server_version));
    names
}

// ---------------------------------------------------------------------------
// Settings-store pure subset (`src/app/stores/settings-store.ts`)
// ---------------------------------------------------------------------------

/// Default compact output mode: `false`.
pub fn default_compact_output_mode() -> bool {
    false
}

/// Default show-thinking: `true`.
pub fn default_show_thinking() -> bool {
    true
}

/// Default assistant-run footer: `true`.
pub fn default_show_assistant_run_footer() -> bool {
    true
}

/// Default pinned dashboard: `true`.
pub fn default_pinned_dashboard_enabled() -> bool {
    true
}

/// Default background-session notifications: `true`.
pub fn default_show_background_session_notifications() -> bool {
    true
}

/// Default track-background-sessions: `true`.
pub fn default_track_background_sessions() -> bool {
    true
}

/// Default prompt-queue mode: `off` on V1, `steer` on V2.
pub fn default_prompt_queue_mode(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "steer"
    } else {
        "off"
    }
}

/// Legacy `ttsEnabled` boolean migration: `true` -> `all`, `false` -> `off`.
pub fn migrate_tts_enabled(old_value: bool) -> &'static str {
    if old_value { "all" } else { "off" }
}
