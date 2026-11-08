//! Media + prompt pure logic (RED stubs, batch media-prompt).
//!
//! Mirrors (TS source -> Rust stub):
//! - `src/app/services/tts-service.ts`
//!   -> strip_markdown_for_speech, extract_language_code,
//!      is_tts_configured, tts_not_configured_message,
//!      openai_speech_url, elevenlabs_speech_url, elevenlabs_model_id,
//!      tts_input_limit_chars, should_prepare_tts_response
//! - `src/app/services/file-download-service.ts`
//!   -> to_data_uri, is_file_size_allowed, format_file_size,
//!      is_text_file_name, is_text_mime_type, telegram_file_url,
//!      redact_bot_token, transient_retry_delay_ms
//! - `src/app/types/prompt.ts` + `src/bot/handlers/prompt.ts`
//!   -> IncomingPrompt, new_incoming_prompt, PromptResponseMode,
//!      PromptResponseModes, is_prompt_sendable
//! - `src/bot/messages/summary-message-formatter.ts`
//!   + `src/app/formatters/summary-formatter.ts`
//!   -> split_summary_text, wrap_raw_part, truncate_with_ellipsis,
//!      format_running_task_line, normalize_path_for_display,
//!      escape_pipes_outside_code, code_file_too_large, UTF8_BOM
//!
//! Pure ports: no network or Telegram I/O lives here.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// TTS service (`src/app/services/tts-service.ts`)
// ---------------------------------------------------------------------------

/// Upper bound for TTS input (`MAX_TTS_INPUT_CHARS = 4_000`).
pub const TTS_INPUT_LIMIT_CHARS: usize = 4_000;

/// ElevenLabs model fallback when the configured model is empty.
pub const ELEVENLABS_FALLBACK_MODEL: &str = "eleven_flash_v2_5";

/// Replaces `marker`-wrapped spans with their inner text, mirroring a
/// non-greedy `marker(.+?)marker` global replace (single-line only, like JS
/// `.` without the `s` flag). Empty or newline-spanning candidates are left
/// in place so scanning can continue past the opening marker.
fn replace_wrapped(text: &str, marker: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let Some(start) = rest.find(marker) else {
            out.push_str(rest);
            break;
        };
        let after_open = start + marker.len();
        let Some(rel) = rest[after_open..].find(marker) else {
            out.push_str(rest);
            break;
        };
        let inner = &rest[after_open..after_open + rel];
        if inner.is_empty() || inner.contains('\n') {
            out.push_str(&rest[..after_open]);
            rest = &rest[after_open..];
            continue;
        }
        out.push_str(&rest[..start]);
        out.push_str(inner);
        rest = rest.get(after_open + rel + marker.len()..).unwrap_or("");
    }
    out
}

/// Mirrors the single-`*` italic pass with its `(?<!\*)\*(?!\*)` guards:
/// only asterisks not adjacent to another asterisk open/close a span.
fn replace_single_asterisk_italic(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let is_open = bytes[i] == b'*'
            && (i == 0 || bytes[i - 1] != b'*')
            && (i + 1 >= bytes.len() || bytes[i + 1] != b'*');
        if !is_open {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let mut close = None;
        let mut j = i + 1;
        while j < bytes.len() {
            if bytes[j] == b'*' && bytes[j - 1] != b'*' && (j + 1 >= bytes.len() || bytes[j + 1] != b'*') {
                close = Some(j);
                break;
            }
            j += 1;
        }
        match close {
            Some(j) if j > i + 1 && !text[i + 1..j].contains('\n') => {
                out.push_str(&text[i + 1..j]);
                i = j + 1;
            }
            _ => {
                out.push('*');
                i += 1;
            }
        }
    }
    out
}

/// Mirrors `/\[([^\]]+)\]\([^)]+\)/g`: keeps link text, drops the URL.
fn replace_markdown_links(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let mut ok = false;
        if let Some(rel_close) = text[i + 1..].find(']') {
            let label = &text[i + 1..i + 1 + rel_close];
            let after = i + 1 + rel_close + 1;
            if !label.is_empty()
                && !label.contains(']')
                && after < bytes.len()
                && bytes[after] == b'('
            {
                if let Some(rel_paren) = text[after + 1..].find(')') {
                    let target = &text[after + 1..after + 1 + rel_paren];
                    if !target.is_empty() && !target.contains(')') {
                        out.push_str(label);
                        i = after + 1 + rel_paren + 1;
                        ok = true;
                    }
                }
            }
        }
        if !ok {
            out.push('[');
            i += 1;
        }
    }
    out
}

/// Applies the per-line strips from `stripMarkdownForSpeech`
/// (headings, quotes, list markers, horizontal rules).
fn strip_line_prefixes(line: &str) -> Option<String> {
    // Horizontal rule: /^[-*_]{3,}\s*$/ -> whole line
    if line.len() >= 3 && line.chars().all(|c| c == '-' || c == '_' || c == '*') {
        return Some(String::new());
    }
    // Headings: ^#{1,6}\s+
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) {
        if let Some(next) = line[hashes..].chars().next() {
            if next.is_whitespace() {
                let rest = line[hashes..].trim_start_matches([' ', '\t', '\r']);
                return Some(rest.to_string());
            }
        }
    }
    // Blockquote: ^>\s?
    if let Some(stripped) = line.strip_prefix('>') {
        let rest = stripped
            .strip_prefix([' ', '\t', '\r'])
            .unwrap_or(stripped);
        return Some(rest.to_string());
    }
    // Unordered list: ^[-*]\s+
    if let Some(first) = line.chars().next() {
        if (first == '-' || first == '*')
            && line[1..].chars().next().is_some_and(|c| c.is_whitespace())
        {
            let rest = line[1..].trim_start_matches([' ', '\t', '\r']);
            return Some(rest.to_string());
        }
    }
    // Ordered list: ^\d+\.\s+
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 {
        let rest = &line[digits..];
        if let Some(after_dot) = rest.strip_prefix('.') {
            if after_dot.chars().next().is_some_and(|c| c.is_whitespace()) {
                return Some(
                    after_dot
                        .trim_start_matches([' ', '\t', '\r'])
                        .to_string(),
                );
            }
        }
    }
    None
}

/// Removes HTML-like tags (`/<\/?[A-Za-z][^>]*>/g`), keeping `<` that is not
/// followed by a tag start (e.g. `2 < 3`).
fn strip_html_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let is_close = i + 1 < bytes.len() && bytes[i + 1] == b'/';
        let name_idx = if is_close { i + 2 } else { i + 1 };
        let valid_start = name_idx < bytes.len() && bytes[name_idx].is_ascii_alphabetic();
        if !valid_start {
            out.push('<');
            i += 1;
            continue;
        }
        match text[name_idx..].find('>') {
            Some(rel) => {
                i = name_idx + rel + 1;
            }
            None => {
                out.push('<');
                i += 1;
            }
        }
    }
    out
}

/// Removes markdown syntax that TTS engines would read aloud.
pub fn strip_markdown_for_speech(text: &str) -> String {
    // Fenced code blocks -> inline content (newlines become spaces).
    let mut clean = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let Some(start) = rest.find("```") else {
            clean.push_str(rest);
            break;
        };
        let Some(rel_end) = rest[start + 3..].find("```") else {
            clean.push_str(rest);
            break;
        };
        let end = start + 3 + rel_end;
        let mut inner = &rest[start + 3..end];
        inner = inner.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '_');
        inner = inner.strip_prefix('\n').unwrap_or(inner);
        inner = inner.strip_suffix('\n').unwrap_or(inner);
        clean.push_str(&rest[..start]);
        clean.push_str(&inner.replace('\n', " "));
        rest = &rest[end + 3..];
    }

    let mut clean = replace_wrapped(&clean, "`");
    for marker in ["***", "**"] {
        clean = replace_wrapped(&clean, marker);
    }
    clean = replace_single_asterisk_italic(&clean);
    clean = replace_wrapped(&clean, "~~");
    clean = replace_markdown_links(&clean);
    let lines: Vec<String> = clean
        .split('\n')
        .map(|line| strip_line_prefixes(line).unwrap_or_else(|| line.to_string()))
        .collect();
    let clean = lines.join("\n");
    let clean = strip_html_tags(&clean);
    // Collapse [ \t]+ runs, then 3+ newlines, then trim.
    let mut collapsed = String::with_capacity(clean.len());
    let mut in_blank = false;
    for ch in clean.chars() {
        if ch == ' ' || ch == '\t' {
            if !in_blank {
                collapsed.push(' ');
                in_blank = true;
            }
        } else {
            collapsed.push(ch);
            in_blank = false;
        }
    }
    let mut squeezed = String::with_capacity(collapsed.len());
    let mut newlines = 0;
    for ch in collapsed.chars() {
        if ch == '\n' {
            newlines += 1;
            if newlines <= 2 {
                squeezed.push(ch);
            }
        } else {
            newlines = 0;
            squeezed.push(ch);
        }
    }
    squeezed.trim().to_string()
}

/// Extracts `ll-CC` from Google voice names like `de-DE-Neural2-B`.
pub fn extract_language_code(voice_name: &str) -> String {
    for len in [3, 2] {
        if voice_name.len() > len + 3
            && voice_name[..len].chars().all(|c| c.is_ascii_lowercase())
            && voice_name.as_bytes()[len] == b'-'
            && voice_name[len + 1..len + 3].chars().all(|c| c.is_ascii_uppercase())
        {
            return voice_name[..len + 3].to_string();
        }
    }
    "en-US".to_string()
}

/// Pure gating behind `isTtsConfigured`: google needs credentials flag,
/// edge is always configured, others need a URL and a key.
pub fn is_tts_configured(
    provider: &str,
    api_url: &str,
    api_key: &str,
    google_credentials_set: bool,
) -> bool {
    match provider {
        "google" => google_credentials_set,
        "edge" => true,
        _ => !api_url.is_empty() && !api_key.is_empty(),
    }
}

/// Provider-specific "TTS is not configured" message.
pub fn tts_not_configured_message(provider: &str) -> String {
    match provider {
        "google" => {
            "TTS is not configured: set GOOGLE_APPLICATION_CREDENTIALS for Google Cloud TTS"
                .to_string()
        }
        "elevenlabs" => {
            "TTS is not configured: set TTS_API_URL and TTS_API_KEY for ElevenLabs".to_string()
        }
        _ => "TTS is not configured: set TTS_API_URL and TTS_API_KEY".to_string(),
    }
}

/// OpenAI-compatible speech endpoint (`{apiUrl}/audio/speech`).
pub fn openai_speech_url(api_url: &str) -> String {
    format!("{api_url}/audio/speech")
}

/// Minimal `encodeURIComponent` for the voice id segment.
fn encode_uri_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// ElevenLabs speech endpoint; trims trailing slashes from the API URL.
pub fn elevenlabs_speech_url(api_url: &str, voice_id: &str) -> String {
    let base = api_url.trim_end_matches('/');
    format!("{base}/text-to-speech/{}", encode_uri_component(voice_id))
}

/// ElevenLabs model id with fallback when the configured model is empty.
pub fn elevenlabs_model_id(model: &str) -> String {
    if model.is_empty() {
        ELEVENLABS_FALLBACK_MODEL.to_string()
    } else {
        model.to_string()
    }
}

/// Gate from `prepareTtsResponseForSession`: only `text_and_tts` with
/// non-empty text within the char limit and configured TTS sends audio.
pub fn should_prepare_tts_response(
    response_mode: Option<&str>,
    text: &str,
    tts_configured: bool,
) -> bool {
    if response_mode != Some("text_and_tts") {
        return false;
    }
    let normalized = text.trim();
    if normalized.is_empty() {
        return false;
    }
    if !tts_configured {
        return false;
    }
    normalized.encode_utf16().count() <= TTS_INPUT_LIMIT_CHARS
}

// ---------------------------------------------------------------------------
// File download service (`src/app/services/file-download-service.ts`)
// ---------------------------------------------------------------------------

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(BASE64_ALPHABET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64_ALPHABET[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_ALPHABET[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Encodes bytes as a `data:{mime};base64,{payload}` URI.
pub fn to_data_uri(bytes: &[u8], mime_type: &str) -> String {
    format!("data:{mime_type};base64,{}", base64_encode(bytes))
}

/// Whether `file_size` fits into `max_size_kb`; unknown size is allowed.
pub fn is_file_size_allowed(file_size: Option<u64>, max_size_kb: u64) -> bool {
    match file_size {
        None | Some(0) => true,
        Some(size) => size <= max_size_kb * 1024,
    }
}

/// Human-readable size (`500B`, `1.5KB`, `2.5MB`).
pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

const TEXT_FILE_EXTENSIONS: &[&str] = &[
    "svelte", "vue", "ts", "tsx", "js", "jsx", "mjs", "cjs", "json", "txt", "xml", "csv",
    "tsv", "sql", "env", "lock", "conf", "properties", "tf", "go", "rs", "rb", "py",
    "java", "c", "cpp", "h", "hpp", "cs", "swift", "kt", "kts", "sh", "bash", "yaml",
    "yml", "toml", "ini", "cfg", "md", "mdx", "css", "scss", "less", "html", "htm",
    "graphql", "gql", "proto", "gradle",
];

const APPLICATION_TEXT_MIME_TYPES: &[&str] = &[
    "application/json",
    "application/xml",
    "application/javascript",
    "application/x-yaml",
    "application/sql",
];

/// Text files that carry no extension, plus dotfiles whose whole name is the marker.
const TEXT_FILE_NAMES: &[&str] = &[
    "makefile",
    "dockerfile",
    "license",
    "readme",
    "changelog",
    ".gitignore",
    ".dockerignore",
    ".editorconfig",
    ".env",
    ".env.example",
    ".npmrc",
    ".prettierrc",
    ".eslintrc",
];

/// Whether a local file looks like text, judging by its name alone.
pub fn is_text_file_name(filename: &str) -> bool {
    let base = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    if base.is_empty() {
        return false;
    }
    if TEXT_FILE_NAMES.contains(&base.as_str()) {
        return true;
    }
    // Mirrors `baseName.includes(".") ? baseName.split(".").pop() : undefined`.
    if !base.contains('.') {
        return false;
    }
    match base.rsplit('.').next() {
        Some(ext) if !ext.is_empty() => TEXT_FILE_EXTENSIONS.contains(&ext),
        _ => false,
    }
}

/// Whether a MIME type (plus optional filename fallback) looks like text.
pub fn is_text_mime_type(mime_type: Option<&str>, filename: Option<&str>) -> bool {
    let Some(mime) = mime_type else {
        return false;
    };
    if mime.is_empty() {
        return false;
    }
    if mime.starts_with("text/") {
        return true;
    }
    if APPLICATION_TEXT_MIME_TYPES.contains(&mime) {
        return true;
    }
    if let Some(name) = filename {
        if let Some(ext) = name.rsplit('.').next() {
            let ext = ext.to_lowercase();
            if ext != name.to_lowercase() && TEXT_FILE_EXTENSIONS.contains(&ext.as_str()) {
                return true;
            }
        }
    }
    false
}

/// Telegram file download URL; defaults to `api.telegram.org` and trims a
/// trailing slash from a custom API root.
pub fn telegram_file_url(
    api_root: Option<&str>,
    bot_token: &str,
    file_path: &str,
) -> String {
    let base = match api_root {
        Some(root) if !root.is_empty() => root.trim_end_matches('/').to_string(),
        _ => "https://api.telegram.org".to_string(),
    };
    format!("{base}/file/bot{bot_token}/{file_path}")
}

/// Redacts the bot token from download errors (`bot<token>` -> `bot***`).
pub fn redact_bot_token(message: &str, bot_token: &str) -> String {
    if bot_token.is_empty() {
        return message.to_string();
    }
    message.replace(bot_token, "***")
}

/// Retry delay for transient HTTP statuses: 429 honors `retry-after`
/// seconds, 502 (and friends) wait 1s, anything else is not retried.
pub fn transient_retry_delay_ms(status: u16, retry_after_secs: Option<u64>) -> Option<u64> {
    match status {
        429 => Some(retry_after_secs.filter(|&s| s > 0).map_or(1_000, |s| s * 1_000)),
        500 | 502 | 503 | 504 => Some(1_000),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Prompt types (`src/app/types/prompt.ts`, `src/bot/handlers/prompt.ts`)
// ---------------------------------------------------------------------------

/// Minimal incoming prompt: text plus attachment placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingPrompt {
    pub text: String,
    pub file_parts: Vec<String>,
    pub photos: Vec<String>,
}

/// Builds an incoming prompt with empty attachment defaults.
pub fn new_incoming_prompt(
    text: &str,
    file_parts: Vec<String>,
    photos: Vec<String>,
) -> IncomingPrompt {
    IncomingPrompt {
        text: text.to_string(),
        file_parts,
        photos,
    }
}

/// Per-session voice-reply mode (`PromptResponseMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptResponseMode {
    TextOnly,
    TextAndTts,
}

/// In-memory per-session response-mode map.
#[derive(Debug, Default)]
pub struct PromptResponseModes {
    modes: HashMap<String, PromptResponseMode>,
}

impl PromptResponseModes {
    pub fn new() -> Self {
        Self {
            modes: HashMap::new(),
        }
    }

    pub fn set(&mut self, session_id: &str, mode: PromptResponseMode) {
        self.modes.insert(session_id.to_string(), mode);
    }

    pub fn clear(&mut self, session_id: &str) {
        self.modes.remove(session_id);
    }

    pub fn get(&self, session_id: &str) -> Option<PromptResponseMode> {
        self.modes.get(session_id).copied()
    }
}

/// Whether a prompt is worth sending: text or at least one attachment.
pub fn is_prompt_sendable(text: &str, file_count: usize, photo_count: usize) -> bool {
    !text.trim().is_empty() || file_count > 0 || photo_count > 0
}

// ---------------------------------------------------------------------------
// Summary message formatting (`summary-message-formatter.ts`,
// `summary-formatter.ts`)
// ---------------------------------------------------------------------------

/// Splits text into `max_length` chunks, preferring newline boundaries.
pub fn split_summary_text(text: &str, max_length: usize) -> Vec<String> {
    let max_length = max_length.max(1);
    if text.trim().is_empty() {
        return vec![];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut parts = Vec::new();
    let mut current = 0;
    while current < chars.len() {
        let hard = (current + max_length).min(chars.len());
        if hard >= chars.len() {
            parts.push(chars[current..].iter().collect());
            break;
        }
        let mut end = hard;
        if let Some(breakpoint) = chars[..=hard].iter().rposition(|&c| c == '\n') {
            if breakpoint > current {
                end = breakpoint + 1;
            }
        }
        if end <= current {
            let fallback = (current + 1).min(chars.len());
            parts.push(chars[current..fallback].iter().collect());
            current = fallback;
            continue;
        }
        parts.push(chars[current..end].iter().collect());
        current = end;
    }
    parts
}

/// Wraps a raw part in a triple-backtick code block.
pub fn wrap_raw_part(part: &str) -> String {
    format!("```\n{part}\n```")
}

/// Cuts text with `...` (or dots when `max_length <= 3`).
pub fn truncate_with_ellipsis(text: &str, max_length: usize) -> String {
    let len = text.chars().count();
    if len <= max_length {
        return text.to_string();
    }
    if max_length <= 3 {
        return ".".repeat(max_length);
    }
    let head: String = text.chars().take(max_length - 3).collect();
    format!("{}...", head.trim_end())
}

/// A subagent launch's line from its description, never from its prompt;
/// overlong lines end with `…`.
pub fn format_running_task_line(
    description: Option<&str>,
    label: &str,
    max_length: usize,
) -> String {
    let normalized = description
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return truncate_to_max(label, max_length);
    }
    let line = format!("{label}: {normalized}");
    if line.chars().count() <= max_length && line.len() <= max_length {
        return line;
    }
    truncate_to_max(&line, max_length)
}

/// Collapses to a single line already done by the caller; cuts to at most
/// `max_length` bytes/chars (UTF-8 safe) ending with `…` when truncated.
fn truncate_to_max(text: &str, max_length: usize) -> String {
    if text.chars().count() <= max_length && text.len() <= max_length {
        return text.to_string();
    }
    if max_length == 0 {
        return String::new();
    }
    let mut head: String = text.chars().take(max_length.saturating_sub(1)).collect();
    head = head.trim_end().to_string();
    let mut out = format!("{head}…");
    while out.len() > max_length && !head.is_empty() {
        head.pop();
        head = head.trim_end().to_string();
        out = format!("{head}…");
    }
    out
}

/// Normalizes separators to `/` and strips the project worktree prefix.
pub fn normalize_path_for_display(file_path: &str, worktree: Option<&str>) -> String {
    let normalized = file_path.replace('\\', "/");
    let Some(worktree) = worktree else {
        return normalized;
    };
    let trimmed = worktree.replace('\\', "/").trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        return normalized;
    }
    if normalized == trimmed {
        return ".".to_string();
    }
    match normalized.strip_prefix(&format!("{trimmed}/")) {
        Some(rest) => rest.to_string(),
        None => normalized,
    }
}

/// Escapes `|` for MarkdownV2 outside inline code and fences.
pub fn escape_pipes_outside_code(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_inline = false;
    let mut in_fence = false;
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`' {
            result.push_str("```");
            i += 3;
            in_fence = !in_fence;
            continue;
        }
        let c = chars[i];
        if !in_fence && c == '`' {
            in_inline = !in_inline;
            result.push(c);
            i += 1;
            continue;
        }
        if !in_fence && !in_inline && c == '|' && (i == 0 || chars[i - 1] != '\\') {
            result.push_str("\\|");
            i += 1;
            continue;
        }
        result.push(c);
        i += 1;
    }
    result
}

/// Whether processed code-file content exceeds the max file size.
pub fn code_file_too_large(content_bytes: usize, max_size_kb: u64) -> bool {
    content_bytes as f64 / 1024.0 > max_size_kb as f64
}

/// BOM prepended to code-file payloads so readers do not guess the code page.
pub const UTF8_BOM: char = '\u{FEFF}';
