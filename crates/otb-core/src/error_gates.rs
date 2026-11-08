//! Error classification / formatting pure gates (slice 41, RED).
//!
//! Rust port of the pure sync helpers in:
//! - `src/utils/type-guards.ts` (`isRecord`)
//! - `src/utils/opencode-error.ts`
//!   (`isExpectedOpencodeUnavailableError`, `isOpencodeNotFoundError`,
//!   `isServerUnavailableError`, `extractErrorMessage`)
//! - `src/utils/error-format.ts` (`formatErrorDetails`, inner `clipText`)
//!
//! Dynamic TS `unknown` error shapes are modelled as [`ErrorValue`]; nested
//! `.cause` chains collapse into `cause: Option<Box<ErrorValue>>`. There is
//! no Rust equivalent of the TS `seen`-set cycle guard (ownership forbids
//! cycles) or the `JSON.stringify` circular-fallback branch.
//!
//! Step 2 (GREEN): pure sync logic port; no I/O, no timers.

/// Default detail budget (TS: `DEFAULT_MAX_ERROR_DETAILS_LENGTH`).
pub const DEFAULT_MAX_ERROR_DETAILS_LENGTH: usize = 1500;

/// Markers proving the OpenCode server is unreachable
/// (TS: `SERVER_UNAVAILABLE_ERROR_MARKERS`).
pub const SERVER_UNAVAILABLE_ERROR_MARKERS: [&str; 4] = [
    "fetch failed",
    "econnrefused",
    "connection refused",
    "connect refused",
];

/// Substrings marking an expected OpenCode-unavailable failure
/// (TS: inline list in `isExpectedOpencodeUnavailableError`).
pub const EXPECTED_UNAVAILABLE_MARKERS: [&str; 6] = [
    "fetch failed",
    "failed to fetch",
    "econnrefused",
    "econnreset",
    "enotfound",
    "connectex",
];

/// Dynamic TS `unknown` error value (TS: probed with `instanceof Error`,
/// `typeof`, and `isRecord` field reads).
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorValue {
    /// An `Error` instance (TS: `error.name` / `error.message` /
    /// `error.stack` / `error.cause`).
    Instance(ErrorInstance),
    /// A plain string error.
    Text(String),
    /// A plain-object error (TS: `isRecord` branch with `code` / `message`
    /// / `name` / `data.message` / `cause` reads).
    Record(ErrorRecord),
    Number(f64),
    Boolean(bool),
    Null,
}

/// `Error`-instance shape (TS: `instanceof Error` branch).
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorInstance {
    pub name: String,
    pub message: String,
    pub stack: Option<String>,
    pub cause: Option<Box<ErrorValue>>,
}

/// Plain-object error shape (TS: `isRecord` branch).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ErrorRecord {
    pub name: Option<String>,
    pub message: Option<String>,
    pub code: Option<String>,
    pub data_message: Option<String>,
    pub cause: Option<Box<ErrorValue>>,
}

/// TS: `isRecord` — object (incl. `Error` instances) and non-null.
pub fn is_record_value(value: &ErrorValue) -> bool {
    // TS: `typeof value === "object" && value !== null`. Strings, numbers,
    // booleans and null are not records; `Error` instances and plain objects
    // are. `undefined`/functions/symbols have no `ErrorValue` counterpart.
    matches!(
        value,
        ErrorValue::Instance(_) | ErrorValue::Record(_)
    )
}

/// TS: `isExpectedOpencodeUnavailableError` — falsy values are `false`;
/// otherwise the lowercased `Error.message` / `String(error)` must contain
/// one of [`EXPECTED_UNAVAILABLE_MARKERS`].
pub fn is_expected_opencode_unavailable_error(value: &ErrorValue) -> bool {
    let message: &str;
    let owned: String;
    match value {
        ErrorValue::Null => return false,
        ErrorValue::Boolean(b) => {
            if !b {
                return false;
            }
            owned = b.to_string();
            message = &owned;
        }
        ErrorValue::Number(n) => {
            // TS `!error`: 0, -0 and NaN are falsy.
            if *n == 0.0 || n.is_nan() {
                return false;
            }
            owned = number_to_string(*n);
            message = &owned;
        }
        ErrorValue::Text(s) => {
            // TS `!error`: `""` is falsy.
            if s.is_empty() {
                return false;
            }
            message = s;
        }
        ErrorValue::Instance(inst) => {
            // TS: `error instanceof Error ? error.message : String(error)`.
            message = &inst.message;
        }
        ErrorValue::Record(_) => {
            // TS: `String(plainObject)` is `"[object Object]"`, which never
            // contains a marker. Short-circuit without allocating.
            return false;
        }
    }
    let lower = message.to_lowercase();
    EXPECTED_UNAVAILABLE_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// TS: `isOpencodeNotFoundError` — a record whose `name` is
/// `"NotFoundError"`.
pub fn is_opencode_not_found_error(value: &ErrorValue) -> bool {
    // TS gates on `isRecord(error)` first, which is also true for `Error`
    // instances (`typeof new Error() === "object"`), so both shapes check
    // `.name`.
    match value {
        ErrorValue::Instance(inst) => inst.name == "NotFoundError",
        ErrorValue::Record(rec) => rec.name.as_deref() == Some("NotFoundError"),
        _ => false,
    }
}

fn has_server_unavailable_marker(value: &str) -> bool {
    let lower = value.to_lowercase();
    SERVER_UNAVAILABLE_ERROR_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// TS: `isServerUnavailableError` — BFS over the `cause` chain; strings,
/// `Error` `name: message` pairs, and record `code` / `message` fields match
/// against [`SERVER_UNAVAILABLE_ERROR_MARKERS`] (case-insensitive).
pub fn is_server_unavailable_error(value: &ErrorValue) -> bool {
    // TS uses a queue with `pop()` (LIFO) plus a `seen` cycle guard.
    // Ownership forbids cycles here, so a plain stack suffices.
    let mut stack: Vec<&ErrorValue> = vec![value];
    while let Some(current) = stack.pop() {
        match current {
            ErrorValue::Null | ErrorValue::Number(_) | ErrorValue::Boolean(_) => continue,
            ErrorValue::Text(s) => {
                if has_server_unavailable_marker(s) {
                    return true;
                }
            }
            ErrorValue::Instance(inst) => {
                let combined = format!("{}: {}", inst.name, inst.message);
                if has_server_unavailable_marker(&combined) {
                    return true;
                }
                if let Some(cause) = inst.cause.as_deref() {
                    stack.push(cause);
                }
            }
            ErrorValue::Record(rec) => {
                if let Some(code) = rec.code.as_deref() {
                    if has_server_unavailable_marker(code) {
                        return true;
                    }
                }
                if let Some(msg) = rec.message.as_deref() {
                    if has_server_unavailable_marker(msg) {
                        return true;
                    }
                }
                if let Some(cause) = rec.cause.as_deref() {
                    stack.push(cause);
                }
            }
        }
    }
    false
}

fn non_blank_trimmed(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// TS: `extractErrorMessage` — `Error` message, then string, then record
/// `data.message` / `message` / `name` (whitespace-only values fall through);
/// `None` when nothing usable is found.
pub fn extract_error_message(value: &ErrorValue) -> Option<String> {
    match value {
        ErrorValue::Instance(inst) => {
            // TS checks `error.message` first; when it is blank the Error
            // object still passes the later `isRecord` probe, falling
            // through to `.message` then `.name`.
            if let Some(m) = non_blank_trimmed(&inst.message) {
                return Some(m);
            }
            non_blank_trimmed(&inst.name)
        }
        ErrorValue::Text(s) => non_blank_trimmed(s),
        ErrorValue::Record(rec) => {
            if let Some(data_message) = rec.data_message.as_deref() {
                if let Some(m) = non_blank_trimmed(data_message) {
                    return Some(m);
                }
            }
            if let Some(msg) = rec.message.as_deref() {
                if let Some(m) = non_blank_trimmed(msg) {
                    return Some(m);
                }
            }
            if let Some(name) = rec.name.as_deref() {
                if let Some(m) = non_blank_trimmed(name) {
                    return Some(m);
                }
            }
            None
        }
        ErrorValue::Number(_) | ErrorValue::Boolean(_) | ErrorValue::Null => None,
    }
}

fn clip_text(value: &str, max_length: usize) -> String {
    // TS: `value.length <= maxLength ? value : value.slice(0, max(0, max-3)) + "..."`.
    // `chars()` keeps this panic-free on unicode; for ASCII (all tested
    // inputs) char counts equal TS UTF-16 code-unit counts.
    if value.chars().count() <= max_length {
        return value.to_string();
    }
    let keep = max_length.saturating_sub(3);
    let truncated: String = value.chars().take(keep).collect();
    format!("{truncated}...")
}

fn json_escape_into(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
}

fn json_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    json_escape_into(&mut out, s);
    out.push('"');
    out
}

fn number_to_string(n: f64) -> String {
    if n.is_nan() || n.is_infinite() {
        // `String(NaN)` is `"NaN"` for the expected-error probe, but
        // `JSON.stringify(NaN)` is `"null"`. Callers needing JSON map via
        // `json_number`. This helper serves the `String(error)` probe.
        return n.to_string();
    }
    if n == 0.0 {
        // Normalises `-0` to `"0"` like JS `String(-0)`.
        return "0".to_string();
    }
    n.to_string()
}

fn json_number(n: f64) -> String {
    // TS `JSON.stringify(NaN/Infinity)` yields `"null"`.
    if n.is_nan() || n.is_infinite() {
        return "null".to_string();
    }
    if n == 0.0 {
        return "0".to_string();
    }
    n.to_string()
}

fn pad(level: usize) -> String {
    "  ".repeat(level)
}

/// `JSON.stringify(value, null, 2)` equivalent for [`ErrorValue`].
/// Plain records render as pretty objects; nested `Instance` causes render
/// their enumerable shape (`name`/`message`/`cause`) rather than TS's `{}`
/// so no message content is lost (see deviations).
fn stringify_value(value: &ErrorValue, level: usize) -> String {
    match value {
        ErrorValue::Text(s) => json_quoted(s),
        ErrorValue::Number(n) => json_number(*n),
        ErrorValue::Boolean(b) => b.to_string(),
        ErrorValue::Null => "null".to_string(),
        ErrorValue::Instance(inst) => {
            let mut entries: Vec<String> = Vec::new();
            if !inst.name.is_empty() {
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("name"),
                    json_quoted(&inst.name)
                ));
            }
            if !inst.message.is_empty() {
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("message"),
                    json_quoted(&inst.message)
                ));
            }
            if let Some(cause) = inst.cause.as_deref() {
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("cause"),
                    stringify_value(cause, level + 1)
                ));
            }
            if entries.is_empty() {
                return "{}".to_string();
            }
            format!("{{\n{}\n{}}}", entries.join(",\n"), pad(level))
        }
        ErrorValue::Record(rec) => {
            let mut entries: Vec<String> = Vec::new();
            if let Some(name) = rec.name.as_deref() {
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("name"),
                    json_quoted(name)
                ));
            }
            if let Some(msg) = rec.message.as_deref() {
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("message"),
                    json_quoted(msg)
                ));
            }
            if let Some(code) = rec.code.as_deref() {
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("code"),
                    json_quoted(code)
                ));
            }
            if let Some(data_message) = rec.data_message.as_deref() {
                // TS shape is `{ data: { message } }`; the Rust model flattens
                // it into `data_message`, so re-nest for JSON output.
                let nested = format!(
                    "{{\n{}{}: {}\n{}}}",
                    pad(level + 2),
                    json_quoted("message"),
                    json_quoted(data_message),
                    pad(level + 1)
                );
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("data"),
                    nested
                ));
            }
            if let Some(cause) = rec.cause.as_deref() {
                entries.push(format!(
                    "{}{}: {}",
                    pad(level + 1),
                    json_quoted("cause"),
                    stringify_value(cause, level + 1)
                ));
            }
            if entries.is_empty() {
                return "{}".to_string();
            }
            format!("{{\n{}\n{}}}", entries.join(",\n"), pad(level))
        }
    }
}

/// TS: `formatErrorDetails` — `Error` stack (falling back to
/// `name: message`), strings as-is, anything else pretty-printed as JSON;
/// blank / `{}` results become `"unknown error"`; output is clipped to
/// `max_length` (`None` selects [`DEFAULT_MAX_ERROR_DETAILS_LENGTH`]).
pub fn format_error_details(value: &ErrorValue, max_length: Option<usize>) -> String {
    let max = max_length.unwrap_or(DEFAULT_MAX_ERROR_DETAILS_LENGTH);
    let details = match value {
        ErrorValue::Instance(inst) => match inst.stack.as_deref() {
            Some(stack) => stack.to_string(),
            None => format!("{}: {}", inst.name, inst.message),
        },
        ErrorValue::Text(s) => s.clone(),
        other => stringify_value(other, 0),
    };
    let normalized = details.trim().to_string();
    if normalized.is_empty() || normalized == "{}" || normalized == "[object Object]" {
        return "unknown error".to_string();
    }
    clip_text(&normalized, max)
}
