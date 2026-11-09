//! OpenCode event-transport gates (slice 107, GREEN).
//!
//! Rust port of the pure remainder in `src/opencode/events.ts`:
//! - `RECONNECT_BASE_DELAY_MS` 1000, `RECONNECT_MAX_DELAY_MS` 15000,
//!   `FATAL_NO_STREAM_ERROR`, `SSE_IDLE_TIMEOUT_ERROR`
//! - `getReconnectDelayMs` exponential backoff
//!   (`1000 * 2^max(0, attempt - 1)`, capped at 15000)
//! - `normalizeDirectoryForComparison` (backslash-to-slash, trailing-slash
//!   strip, drive-letter lowercasing) and `isSameDirectory` equality
//! - `isEventStreamIdleTimeoutError` message gate
//!   (`message === SSE_IDLE_TIMEOUT_ERROR`)
//! - `isEventLike` shape gate (`isRecord` + `typeof type === "string"` +
//!   `isRecord(properties)`)
//! - `normalizeGlobalEvent` directory gates (mismatched-directory drop,
//!   `eventDirectory || directory` fallback) and `normalizeEvent`
//!   source dispatch (global vs legacy shape gate)
//! - `getReconnectInfo` restart flag
//!   (non-`server.connected` -> null, else `typeof restarted === "boolean"`
//!   ? restarted : null)
//!
//! Already ported elsewhere and NOT re-tested here:
//! - subscription lifecycle / busy-idle routing / footer gating / compact
//!   fallback (`event_subscription` owner for
//!   `src/bot/services/event-subscription-service.ts`,
//!   `src/bot/events/session-runtime-state.ts`,
//!   `src/bot/events/handlers/session-lifecycle-handler.ts`,
//!   `src/app/services/event-router.ts`).
//! - `stopEventListening` generation bookkeeping, `waitWithAbort` /
//!   `createAttemptAbortController` / `readStreamWithIdleTimeout` timer and
//!   `AbortSignal` wiring, `subscribeToGlobalEventStream` /
//!   `subscribeToLegacyEventStream` / `subscribeToEvents` network loops,
//!   `isExpectedOpencodeUnavailableError` classification
//!   (`error_gates` owner), `isRecord` object check (`type-guards`
//!   caller input), `logger` calls.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - SSE `AsyncGenerator` streams, `AbortController` signals, `setTimeout`
//!   / `setImmediate` scheduling, `opencodeClient` network reads,
//!   `Date`/`Math.random` clocks, `logger` I/O (inputs arrive as explicit
//!   scalars / bools).
//!
//! Deviations: none intended. JS `String.replace(/\\/g, "/")` and
//! `replace(/\/+$/, "")` are mirrored on `char`s; parity fixtures use
//! ASCII paths where JS UTF-16 and Rust `char`s agree. `Math.pow(2, n)`
//! on whole-number attempts is exact for the fixture range. `||`
//! fallback treats only `None` and `""` as missing (whitespace-only
//! directory strings count as present, like TS).
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity tests in
//! `crates/otb-core/tests/slice107_event_transport_parity.rs` assert the
//! TS behavior and now pass.

/// Base reconnect pause before backoff (TS: `RECONNECT_BASE_DELAY_MS`).
pub const RECONNECT_BASE_DELAY_MS: u64 = 1000;

/// Ceiling for the reconnect backoff (TS: `RECONNECT_MAX_DELAY_MS`).
pub const RECONNECT_MAX_DELAY_MS: u64 = 15000;

/// Error text when the SDK answers without a stream
/// (TS: `FATAL_NO_STREAM_ERROR`).
pub const FATAL_NO_STREAM_ERROR: &str = "No stream returned from event subscription";

/// Error text for an idle SSE stream (TS: `SSE_IDLE_TIMEOUT_ERROR`).
pub const SSE_IDLE_TIMEOUT_ERROR: &str = "SSE stream idle timeout";

/// Backoff before reconnect attempt `attempt` (TS: `getReconnectDelayMs`:
/// `1000 * 2^max(0, attempt - 1)`, capped at 15000).
pub fn get_reconnect_delay_ms(attempt: i64) -> u64 {
    let exponent: u32 = (attempt - 1).max(0).min(30) as u32;
    let exponential_delay = RECONNECT_BASE_DELAY_MS.saturating_mul(1u64 << exponent);
    exponential_delay.min(RECONNECT_MAX_DELAY_MS)
}

/// Comparable directory form (TS: `normalizeDirectoryForComparison`:
/// `\` -> `/`, trailing `/` stripped, lowercased when it starts with a
/// drive letter like `C:`).
pub fn normalize_directory_for_comparison(directory: &str) -> String {
    let slashed = directory.replace('\\', "/");
    let stripped = slashed.trim_end_matches('/').to_string();
    let bytes = stripped.as_bytes();
    let is_drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if is_drive {
        stripped.to_lowercase()
    } else {
        stripped
    }
}

/// `true` when both directories denote the same folder
/// (TS: `isSameDirectory`: normalized equality).
pub fn is_same_directory(left: &str, right: &str) -> bool {
    normalize_directory_for_comparison(left) == normalize_directory_for_comparison(right)
}

/// `true` when the failure message marks an idle SSE stream
/// (TS: `isEventStreamIdleTimeoutError` message comparison;
/// the caller checks the `Error` side).
pub fn is_idle_timeout_message(message: &str) -> bool {
    message == SSE_IDLE_TIMEOUT_ERROR
}

/// `true` for an event-shaped value (TS: `isEventLike`: a record whose
/// `type` is a string and whose `properties` is a record).
pub fn is_event_like(is_record: bool, type_is_string: bool, properties_is_record: bool) -> bool {
    is_record && type_is_string && properties_is_record
}

/// `true` when a global event from another folder is dropped
/// (TS: `normalizeGlobalEvent`: `eventDirectory && !isSameDirectory`
/// returns null).
pub fn should_drop_global_event_on_directory(
    event_directory: Option<&str>,
    same_directory: bool,
) -> bool {
    matches!(event_directory, Some(dir) if !dir.is_empty()) && !same_directory
}

/// Directory a kept global event is attributed to
/// (TS: `eventDirectory || directory`: the event folder wins unless it
/// is missing or empty).
pub fn resolve_global_event_directory(
    event_directory: Option<&str>,
    default_directory: &str,
) -> String {
    match event_directory {
        Some(dir) if !dir.is_empty() => dir.to_string(),
        _ => default_directory.to_string(),
    }
}

/// Restart flag a reconnect reports, or `None` when unknown
/// (TS: `getReconnectInfo`: non-`server.connected` -> null, else the
/// `restarted` property when it is a boolean).
pub fn resolve_reconnect_restarted(
    is_connected_event: bool,
    restarted: Option<bool>,
) -> Option<bool> {
    if !is_connected_event {
        return None;
    }
    restarted
}

/// True for the fatal no-stream failure (TS: `FATAL_NO_STREAM_ERROR`;
/// the listener throws instead of reconnecting).
pub fn is_fatal_no_stream_message(message: &str) -> bool {
    message == FATAL_NO_STREAM_ERROR
}

/// Normalize the directory a reconnected subscription filters on
/// (TS: the reconnect keeps `directory` of the original `subscribeToEvents`
/// call; cross-directory frames are dropped by `normalizeGlobalEvent`).
pub fn should_keep_subscription_on_reconnect(is_listening: bool, same_directory: bool) -> bool {
    is_listening && same_directory
}
