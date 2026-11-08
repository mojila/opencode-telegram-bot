//! Telegram retry / backoff pure gates (slice 40, RED).
//!
//! Rust port of the pure sync remainder in
//! `src/utils/telegram-rate-limit-retry.ts`:
//! - `isTelegramRateLimitError` (status/`error_code` 429 wins; a system
//!   error `code` suppresses message sniffing; otherwise `429` word or
//!   `too many requests` in the composed message)
//! - `isUnsentTelegramNetworkError` (connect-not-established codes on the
//!   nested `.error`/`.cause` or top level; pre-TLS `ECONNRESET` wording;
//!   `request-timeout` type)
//! - `isTransientTelegramNetworkError` (broader transient set + timeout)
//! - `isTransientTelegramServerError` (`500`/`502`/`503`/`504` status wins;
//!   system code suppresses message sniffing; otherwise code words in text)
//! - `getTelegramRetryAfterMs` (rate-limit `retry_after` / message /
//!   fallback; exponential backoff for unsent-network and transient-server
//!   errors; `None` when not retryable)
//! - `serverErrorBackoffMs` (`base * 2^attempt` capped at 8000 ms)
//!
//! Pure sync logic only: no fetch, no timers, no retry loop (TS:
//! `withTelegramRateLimitRetry` is async I/O and stays out of scope).
//! Dynamic TS error shapes (`Reflect.get` over `unknown`) are modelled as
//! [`TelegramCallError`]; nested `.error`/`.cause` fields collapse into the
//! `nested_*` fields.

/// 5xx codes retried with exponential backoff
/// (TS: `TRANSIENT_SERVER_ERROR_CODES`).
pub const TRANSIENT_SERVER_ERROR_CODES: [i64; 4] = [500, 502, 503, 504];

/// Backoff ceiling for transient errors
/// (TS: `MAX_SERVER_ERROR_BACKOFF_MS`).
pub const MAX_SERVER_ERROR_BACKOFF_MS: u64 = 8000;

/// Connection codes proving nothing was sent
/// (TS: `CONNECT_NOT_ESTABLISHED_CODES`).
pub const CONNECT_NOT_ESTABLISHED_CODES: [&str; 5] =
    ["ECONNREFUSED", "ENOTFOUND", "EAI_AGAIN", "ENETUNREACH", "EHOSTUNREACH"];

/// Broader retryable-when-enabled network codes
/// (TS: `TRANSIENT_NETWORK_ERROR_CODES`).
pub const TRANSIENT_NETWORK_ERROR_CODES: [&str; 10] = [
    "ECONNREFUSED",
    "ENOTFOUND",
    "EAI_AGAIN",
    "ENETUNREACH",
    "EHOSTUNREACH",
    "ECONNABORTED",
    "ECONNRESET",
    "EPIPE",
    "EPROTO",
    "ETIMEDOUT",
];

/// Dynamic Telegram/network error shape (TS: `unknown` error probed with
/// `Reflect.get`: `status` / `error_code`, `description` / `message`,
/// `parameters.retry_after`, `code`, nested `.error` / `.cause` `code` and
/// `type`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TelegramCallError {
    pub status: Option<i64>,
    pub error_code: Option<i64>,
    pub description: Option<String>,
    pub message: Option<String>,
    /// `parameters.retry_after` in seconds (TS: must be finite and `> 0`).
    pub retry_after_secs: Option<f64>,
    /// Top-level system error code (TS: `code`).
    pub code: Option<String>,
    /// Nested `.error.code` / `.cause.code` (TS: `getNestedNetworkError`).
    pub nested_code: Option<String>,
    /// Top-level timeout marker (TS: `type`).
    pub type_field: Option<String>,
    /// Nested `.error.type` / `.cause.type`.
    pub nested_type: Option<String>,
}

fn composed_message(error: &TelegramCallError) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(d) = error.description.as_deref() {
        parts.push(d);
    }
    if let Some(m) = error.message.as_deref() {
        parts.push(m);
    }
    parts.join("\n")
}

fn status_code(error: &TelegramCallError) -> Option<i64> {
    // TS: `status` wins when it is a finite number, else `error_code`.
    // All i64 values are finite, so this is `status.or(error_code)`.
    error.status.or(error.error_code)
}

fn system_error_code(error: &TelegramCallError) -> Option<&str> {
    // TS: `readStringField(nested, "code") ?? readStringField(top, "code")`.
    if let Some(c) = error.nested_code.as_deref() {
        return Some(c);
    }
    error.code.as_deref()
}

fn timeout_type(error: &TelegramCallError) -> Option<&str> {
    // TS: `readStringField(nested, "type") ?? readStringField(top, "type")`.
    if let Some(t) = error.nested_type.as_deref() {
        return Some(t);
    }
    error.type_field.as_deref()
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// JS `\b<digits>\b` over an ASCII haystack: every occurrence of `needle`
/// must sit between non-word chars (or string edges).
fn has_code_word(haystack: &str, needle: &str) -> bool {
    let hay = haystack.as_bytes();
    let nd = needle.as_bytes();
    if nd.is_empty() || hay.len() < nd.len() {
        return false;
    }
    let mut i = 0;
    while i + nd.len() <= hay.len() {
        if &hay[i..i + nd.len()] == nd {
            let before_ok = i == 0 || !is_word_char(hay[i - 1]);
            let after_ok = i + nd.len() == hay.len() || !is_word_char(hay[i + nd.len()]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// TS: `getRetryAfterSecondsFromError` — params first, then
/// `/retry after\s+(\d+)/i` over the composed message.
fn retry_after_seconds(error: &TelegramCallError) -> Option<f64> {
    if let Some(secs) = error.retry_after_secs {
        if secs.is_finite() && secs > 0.0 {
            return Some(secs);
        }
    }
    parse_retry_after_from_message(&composed_message(error))
}

/// First case-insensitive `retry after <digits>` match; non-positive parses
/// yield `None` just like the TS `<= 0` guard.
fn parse_retry_after_from_message(message: &str) -> Option<f64> {
    let lower = message.to_lowercase();
    let bytes = lower.as_bytes();
    let needle = b"retry after";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let mut j = i + needle.len();
            // TS `\s+`: at least one whitespace char.
            let ws_start = j;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j == ws_start {
                i += 1;
                continue;
            }
            let digits_start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if digits_start == j {
                i += 1;
                continue;
            }
            let parsed: i64 = lower[digits_start..j].parse().ok()?;
            if parsed <= 0 {
                return None;
            }
            return Some(parsed as f64);
        }
        i += 1;
    }
    None
}

/// 429 status/code, else `429` word / `too many requests` message sniffing
/// unless a system code is present (TS: `isTelegramRateLimitError`).
pub fn is_telegram_rate_limit_error(error: &TelegramCallError) -> bool {
    if let Some(status) = status_code(error) {
        return status == 429;
    }
    if system_error_code(error).is_some() {
        return false;
    }
    let message = composed_message(error).to_lowercase();
    has_code_word(&message, "429") || message.contains("too many requests")
}

/// Nothing was sent: connect-not-established code, pre-TLS `ECONNRESET`
/// wording, or `request-timeout` type
/// (TS: `isUnsentTelegramNetworkError`).
pub fn is_unsent_telegram_network_error(error: &TelegramCallError) -> bool {
    let code = system_error_code(error);
    if let Some(c) = code {
        if CONNECT_NOT_ESTABLISHED_CODES.contains(&c) {
            return true;
        }
    }
    // Generic resets may happen after Telegram accepted a request; only the
    // pre-TLS wording proves nothing was sent. Case-sensitive like TS.
    if code == Some("ECONNRESET")
        && composed_message(error).contains("before secure TLS connection was established")
    {
        return true;
    }
    timeout_type(error) == Some("request-timeout")
}

/// Retryable-when-enabled network failure: transient code set or
/// `request-timeout` type (TS: `isTransientTelegramNetworkError`).
pub fn is_transient_telegram_network_error(error: &TelegramCallError) -> bool {
    if let Some(c) = system_error_code(error) {
        if TRANSIENT_NETWORK_ERROR_CODES.contains(&c) {
            return true;
        }
    }
    timeout_type(error) == Some("request-timeout")
}

/// Retryable 5xx: status/code in `{500, 502, 503, 504}`, else code words in
/// the composed message unless a system code is present
/// (TS: `isTransientTelegramServerError`).
pub fn is_transient_telegram_server_error(error: &TelegramCallError) -> bool {
    if let Some(status) = status_code(error) {
        return TRANSIENT_SERVER_ERROR_CODES.contains(&status);
    }
    if system_error_code(error).is_some() {
        return false;
    }
    let message = composed_message(error);
    TRANSIENT_SERVER_ERROR_CODES
        .iter()
        .any(|code| has_code_word(&message, &code.to_string()))
}

/// `base * 2^attempt` floored to whole ms, min 1, capped at 8000
/// (TS: `getServerErrorBackoffMs`).
pub fn server_error_backoff_ms(attempt: u32, base_delay_ms: u64) -> u64 {
    // Saturating doubling avoids overflow for large attempts; values are
    // integers so flooring is a no-op.
    let mut delay = base_delay_ms;
    for _ in 0..attempt {
        delay = delay.saturating_mul(2);
        if delay >= MAX_SERVER_ERROR_BACKOFF_MS {
            break;
        }
    }
    delay.max(1).min(MAX_SERVER_ERROR_BACKOFF_MS)
}

/// Ms to wait before retrying, or `None` when not retryable: rate-limit
/// `retry_after` (params, then message, then fallback) wins; unsent-network
/// and transient-server errors use exponential backoff
/// (TS: `getTelegramRetryAfterMs`).
pub fn get_telegram_retry_after_ms(
    error: &TelegramCallError,
    fallback_delay_ms: u64,
    attempt: u32,
) -> Option<u64> {
    if is_telegram_rate_limit_error(error) {
        if let Some(secs) = retry_after_seconds(error) {
            let ms = (secs * 1000.0).floor() as u64;
            return Some(ms.max(1));
        }
        return Some(fallback_delay_ms.max(1));
    }

    if is_unsent_telegram_network_error(error) {
        return Some(server_error_backoff_ms(attempt, fallback_delay_ms));
    }

    if is_transient_telegram_server_error(error) {
        return Some(server_error_backoff_ms(attempt, fallback_delay_ms));
    }

    None
}
