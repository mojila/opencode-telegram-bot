//! Notice-delivery + runtime-health pure gates (slice 117, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/messages/thinking-message.ts` (`deliverThinkingMessage`:
//!   `t("bot.thinking")` message key, `"thinking_started"` batcher tag,
//!   immediate `sendTextNow` path — never `enqueue`)
//! - `src/bot/telegram-outage-notices.ts` (`NOTICE_TEXT` record:
//!   `undelivered -> "bot.assistant_reply_undelivered"`,
//!   `skipped -> "bot.stale_messages_skipped"`)
//! - `src/runtime/bot-version.ts` (`getBotVersion` fallback:
//!   `packageJson.version ?? "unknown"`, on read failure too)
//! - `src/runtime/healthcheck.ts` (`PROBE_TIMEOUT_MS` 5000,
//!   `"/health/live"` probe path, `port === 0` skip gate,
//!   `status === 200 ? 0 : 1` exit mapping, fetch failure -> 1)
//! - `src/runtime/service/types.ts` (`BotServiceStatus["status"]`
//!   `"running" | "stopped"` union, `ServiceCleanupReason`
//!   `"stale" | "invalid" | null` union)
//!
//! Step 1 (RED): constants carry their TS values so constant-value tests
//! pass; every function is an `unimplemented!()` stub. The parity tests in
//! `crates/otb-core/tests/slice117_notice_health_parity.rs` assert the TS
//! contract and MUST FAIL with `unimplemented!()` panics until the Rust
//! port lands (step 2).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `Api` sends, `AsyncLocalStorage` tracking, outage-service
//!   timers/state (`telegram-outage-notice-service.ts` owner:
//!   `outage_ready_gates`), `ToolMessageBatcher` I/O
//! - `package.json` file read + version cache (`getBotVersion` body)
//! - `fetch` liveness probe, `.env` file read, container detection
//! - `t(...)` locale lookups (keys travel as literal strings).
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `telegram-outage-notice-service.ts` state machine
//!   (`outage_ready_gates` owner; due-flags arrive here as the
//!   `OutageNoticeKind` itself)
//! - `/health/live` route dispatch (`session_health_gates` owner; this
//!   slice owns only the probe path constant and the exit-code mapping)
//! - `BOT_HEALTH_PORT` env parsing (`types_config::resolve_health_port`
//!   owner; the port arrives here as an explicit `u16`).
//!
//! Deviations: none intended.
//! - TS `?? "unknown"` is nullish-only: an empty-string version stays
//!   empty (it is not null/undefined). `resolve_bot_version_text` keeps
//!   that rule: only `None` maps to the fallback.
//! - `ServiceCleanupReason` is nullable in TS (`| null`). The port uses
//!   `Option<ServiceCleanupReason>`: `None` covers both TS `null` and an
//!   unknown string, which the parity tests pin down.

// ---------------------------------------------------------------------------
// Thinking indicator (`src/bot/messages/thinking-message.ts`)
// ---------------------------------------------------------------------------

/// i18n key sent as the thinking indicator (TS: `t("bot.thinking")`).
pub const THINKING_MESSAGE_KEY: &str = "bot.thinking";

/// Batcher tag for the thinking indicator (TS: `"thinking_started"`).
pub const THINKING_STARTED_TAG: &str = "thinking_started";

/// Whether the thinking indicator takes the immediate-send path (TS:
/// `sendTextNow`, never `enqueue`).
pub fn should_send_thinking_now() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Telegram outage notices (`src/bot/telegram-outage-notices.ts`)
// ---------------------------------------------------------------------------

/// Which pending outage notice a flush delivers (TS:
/// `TelegramOutageNoticeKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutageNoticeKind {
    Undelivered,
    Skipped,
}

/// i18n key for one outage notice (TS: the `NOTICE_TEXT` record).
pub fn outage_notice_text_key(kind: OutageNoticeKind) -> &'static str {
    match kind {
        OutageNoticeKind::Undelivered => "bot.assistant_reply_undelivered",
        OutageNoticeKind::Skipped => "bot.stale_messages_skipped",
    }
}

// ---------------------------------------------------------------------------
// Bot version (`src/runtime/bot-version.ts`)
// ---------------------------------------------------------------------------

/// Fallback when `package.json` has no version or cannot be read
/// (TS: `"unknown"` in `getBotVersion`).
pub const UNKNOWN_BOT_VERSION: &str = "unknown";

/// Resolve the bot version text (TS: `packageJson.version ?? "unknown"`).
pub fn resolve_bot_version_text(version: Option<&str>) -> &str {
    match version {
        Some(text) => text,
        None => UNKNOWN_BOT_VERSION,
    }
}

// ---------------------------------------------------------------------------
// Healthcheck probe (`src/runtime/healthcheck.ts`)
// ---------------------------------------------------------------------------

/// Default probe timeout (TS: `PROBE_TIMEOUT_MS`).
pub const HEALTH_PROBE_TIMEOUT_MS: u64 = 5000;

/// Liveness probe path (TS: `"/health/live"` in `runHealthcheck`).
pub const HEALTH_LIVE_PATH: &str = "/health/live";

/// Whether the healthcheck passes without probing (TS: `port === 0`
/// early return).
pub fn should_skip_health_probe(port: u16) -> bool {
    port == 0
}

/// Exit code for a completed probe (TS: `response.status === 200 ? 0 : 1`;
/// a failed fetch is `None` -> 1).
pub fn health_probe_exit_code(status: Option<u16>) -> i32 {
    match status {
        Some(200) => 0,
        _ => 1,
    }
}

// ---------------------------------------------------------------------------
// Service types (`src/runtime/service/types.ts`)
// ---------------------------------------------------------------------------

/// Service liveness (TS: `BotServiceStatus["status"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotServiceStatusValue {
    Running,
    Stopped,
}

/// Parse service status text (TS: the `"running" | "stopped"` union).
pub fn parse_bot_service_status(raw: &str) -> Option<BotServiceStatusValue> {
    match raw {
        "running" => Some(BotServiceStatusValue::Running),
        "stopped" => Some(BotServiceStatusValue::Stopped),
        _ => None,
    }
}

/// Whether the service counts as running (TS: `status === "running"`).
pub fn is_bot_service_running(status: BotServiceStatusValue) -> bool {
    status == BotServiceStatusValue::Running
}

/// Service cleanup reason (TS: `ServiceCleanupReason`, without the
/// nullable wrapper).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceCleanupReason {
    Stale,
    Invalid,
}

/// Parse cleanup reason text; `None`/unknown -> `None` (TS: the nullable
/// `"stale" | "invalid" | null` union).
pub fn parse_service_cleanup_reason(raw: Option<&str>) -> Option<ServiceCleanupReason> {
    match raw {
        Some("stale") => Some(ServiceCleanupReason::Stale),
        Some("invalid") => Some(ServiceCleanupReason::Invalid),
        _ => None,
    }
}
