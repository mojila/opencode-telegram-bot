//! Markdown-fallback pure gates (slice 45, GREEN).
//!
//! Rust port of the pure classification/escape logic behind
//! `src/bot/messages/send-with-markdown-fallback.ts`:
//! - `escapeTelegramMarkdownV2` (reserved-char escape with odd-backslash guard)
//! - `unescapeTelegramMarkdownV2` (`\x` -> `x` for reserved `x`)
//! - `getErrorText` (message + description join, lowercased)
//! - `isTelegramMarkdownParseError` / `isTelegramEntityUrlError`
//! - `isTelegramBadRequestError` (error_code/status 400)
//! - `isTelegramMarkdownPresentationError` (bad-request AND parse/url error)
//! - fallback-text resolution (`rawFallbackText ?? unescape` for MarkdownV2)
//! - escape-retry and raw-fallback decision gates
//! - format-flag stripping decision (`stripMarkdownFormattingOptions`)
//!
//! Transport (grammy `sendMessage` / `editMessageText`) stays out; this
//! module only decides *what* to send next.
//!
//! Deviations from the TS source (documented):
//! - `unescape_telegram_markdown_v2` does NOT collapse `\\` (backslash +
//!   backslash) to `\`. The TS regex `/\\([…\\])/g` would collapse a
//!   backslash pair, but the parity contract
//!   (`slice45_38_unescape_leaves_plain_backslash_runs`) requires
//!   `"a\\b"` (two backslashes) to round-trip unchanged. The Rust port
//!   therefore unescapes `\x` only for reserved `x != '\\'` and leaves
//!   backslash runs otherwise intact.
//! - `get_error_text` takes the normalized [`ErrorTextParts`] struct instead
//!   of `unknown` + `Reflect.get`; field order (error_message, description,
//!   message_prop, string_form) mirrors the TS push order.
//! - `is_telegram_*` predicates take the already-lowered error text (the TS
//!   `getErrorText` output) instead of `unknown` errors.
//! - `is_telegram_bad_request_error` takes `error_code`/`status` as
//!   `Option<i64>` instead of reading them off `unknown` via `Reflect.get`.
//! - `is_telegram_markdown_presentation_error`, `should_try_escape_retry`,
//!   `should_fall_back_to_raw`, and `strip_markdown_formatting_options`
//!   take precomputed booleans/flags (pure decision shape of the TS
//!   `if` gates); the TS `stripMarkdownFormattingOptions` deletes
//!   `parse_mode`/`entities` keys off an options object, which the Rust
//!   [`StrippedFormatFlags`] return value reports.
//! - Case folding uses Rust `str::to_lowercase` (Unicode) as the equivalent
//!   of JS `String.prototype.toLowerCase`.

/// Parse mode selecting formatted sends (`None` mirrors `parseMode`
/// undefined: send raw with no fallback path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackParseMode {
    Markdown,
    MarkdownV2,
}

/// Normalized error surface (TS `getErrorText` inputs: `Error.message`,
/// `error.description`, `error.message`, or a bare string).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorTextParts {
    pub error_message: Option<String>,
    pub description: Option<String>,
    pub message_prop: Option<String>,
    pub string_form: Option<String>,
}

/// Join error fragments with `\n` and lowercase them (TS `getErrorText`).
pub fn get_error_text(parts: &ErrorTextParts) -> String {
    let mut fragments: Vec<&str> = Vec::new();
    if let Some(s) = parts.error_message.as_deref() {
        fragments.push(s);
    }
    if let Some(s) = parts.description.as_deref() {
        fragments.push(s);
    }
    if let Some(s) = parts.message_prop.as_deref() {
        fragments.push(s);
    }
    if let Some(s) = parts.string_form.as_deref() {
        fragments.push(s);
    }
    if fragments.is_empty() {
        return String::new();
    }
    fragments.join("\n").to_lowercase()
}

const MARKDOWN_PARSE_ERROR_MARKERS: &[&str] = &[
    "can't parse entities",
    "can't parse entity",
    "can't find end of the entity",
    "entity beginning",
    "bad request: can't parse",
];

const TELEGRAM_ENTITY_URL_ERROR_MARKERS: &[&str] =
    &["entity url", "wrong http url", "url host is empty"];

/// True when the lowered error text carries a Markdown parse marker
/// (TS `isTelegramMarkdownParseError`).
pub fn is_telegram_markdown_parse_error_text(lowered_error_text: &str) -> bool {
    if lowered_error_text.is_empty() {
        return false;
    }
    MARKDOWN_PARSE_ERROR_MARKERS
        .iter()
        .any(|marker| lowered_error_text.contains(marker))
}

/// True when the lowered error text carries an entity-URL marker
/// (TS `isTelegramEntityUrlError`).
pub fn is_telegram_entity_url_error_text(lowered_error_text: &str) -> bool {
    if lowered_error_text.is_empty() {
        return false;
    }
    TELEGRAM_ENTITY_URL_ERROR_MARKERS
        .iter()
        .any(|marker| lowered_error_text.contains(marker))
}

/// True for Telegram 400s via `error_code` or `status`
/// (TS `isTelegramBadRequestError`).
pub fn is_telegram_bad_request_error(error_code: Option<i64>, status: Option<i64>) -> bool {
    if error_code == Some(400) {
        return true;
    }
    status == Some(400)
}

/// True when a bad request is also a Markdown presentation failure
/// (TS `isTelegramMarkdownPresentationError`).
pub fn is_telegram_markdown_presentation_error(
    is_bad_request: bool,
    is_parse_error: bool,
    is_url_error: bool,
) -> bool {
    is_bad_request && (is_parse_error || is_url_error)
}

fn is_markdown_v2_reserved(c: char) -> bool {
    matches!(
        c,
        '_' | '*'
            | '['
            | ']'
            | '('
            | ')'
            | '~'
            | '`'
            | '>'
            | '#'
            | '+'
            | '-'
            | '='
            | '|'
            | '{'
            | '}'
            | '.'
            | '!'
            | '\\'
    )
}

/// Escape every unescaped MarkdownV2 reserved char with a backslash.
/// A char preceded by an odd run of backslashes is already escaped
/// (TS `escapeTelegramMarkdownV2`).
pub fn escape_telegram_markdown_v2(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut trailing_backslashes: usize = 0;

    for ch in text.chars() {
        if ch == '\\' {
            result.push(ch);
            trailing_backslashes += 1;
            continue;
        }

        let is_escaped = trailing_backslashes % 2 == 1;
        trailing_backslashes = 0;

        if is_markdown_v2_reserved(ch) && !is_escaped {
            result.push('\\');
        }
        result.push(ch);
    }

    result
}

/// Remove one backslash before MarkdownV2 reserved chars
/// (TS `unescapeTelegramMarkdownV2`).
///
/// See the module docs: unlike the TS regex, a backslash pair (`\\`) is
/// left intact so plain backslash runs round-trip unchanged.
pub fn unescape_telegram_markdown_v2(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            if next != '\\' && is_markdown_v2_reserved(next) {
                result.push(next);
                i += 2;
                continue;
            }
        }
        result.push(ch);
        i += 1;
    }
    result
}

/// Resolve the raw-mode retry text: explicit `rawFallbackText` wins,
/// otherwise MarkdownV2 payloads are unescaped (TS fallback-text lines).
pub fn resolve_fallback_text(
    text: &str,
    raw_fallback_text: Option<&str>,
    parse_mode: Option<FallbackParseMode>,
) -> String {
    if let Some(raw) = raw_fallback_text {
        return raw.to_string();
    }
    match parse_mode {
        Some(FallbackParseMode::MarkdownV2) => unescape_telegram_markdown_v2(text),
        _ => text.to_string(),
    }
}

/// True when a failed MarkdownV2 send may retry with the escaped text:
/// MarkdownV2 + 400 + parse error + escaping changed the text
/// (TS `if (parseMode === "MarkdownV2" && ...)` escape branch).
pub fn should_try_escape_retry(
    parse_mode: Option<FallbackParseMode>,
    is_bad_request: bool,
    is_parse_error: bool,
    escaped_differs: bool,
) -> bool {
    parse_mode == Some(FallbackParseMode::MarkdownV2)
        && is_bad_request
        && is_parse_error
        && escaped_differs
}

/// True when a failed send/edit degrades to the raw fallback text
/// (TS `if (!isTelegramMarkdownPresentationError(error)) throw` gate,
/// inverted: fall back only on presentation errors).
pub fn should_fall_back_to_raw(is_presentation_error: bool) -> bool {
    is_presentation_error
}

/// Which format flags a raw retry drops (TS
/// `stripMarkdownFormattingOptions`: removes `parse_mode` and `entities`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrippedFormatFlags {
    pub parse_mode_removed: bool,
    pub entities_removed: bool,
}

/// Decide the stripped flags for a raw retry: both flags are dropped when
/// present, otherwise left alone (pure shape of the TS strip helper).
pub fn strip_markdown_formatting_options(
    had_parse_mode: bool,
    had_entities: bool,
) -> StrippedFormatFlags {
    StrippedFormatFlags {
        parse_mode_removed: had_parse_mode,
        entities_removed: had_entities,
    }
}
