//! Outage-notice + ready-lifecycle + quoted-notice pure gates (slice 51, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/telegram-outage-notice-service.ts`
//!   (`TelegramOutageNoticeService`: undelivered/skipped pending flags,
//!   burst-suppressed `markMessagesSkipped`, `closeBurst`, chat-interval
//!   `noteChatSendSucceeded` / `msUntilAllowedSend`, `takeDueNotices` /
//!   `reportNoticeResult` / `finishFlush` flush guard)
//! - `src/opencode/ready-lifecycle.ts` (`OpencodeReadyLifecycle`: `isReady`,
//!   once-only `notifyReady`, `notifyUnavailable` only when ready,
//!   `onReady` subscribe/unsubscribe)
//! - `src/opencode/ready-refresh.ts` (`isModelCatalogComplete`,
//!   `isModelCatalogReadyForRestore`, the wait/poll timeout constants, the
//!   late-catalog-change gate behind `noteLateModelCatalogChange`)
//! - `src/utils/telegram-markdown.ts` (`escapePlainTextForTelegramMarkdownV2`)
//! - `src/app/services/quoted-notification.ts` (`buildQuotedPlainText`,
//!   `buildQuotedMarkdownText`, `buildQuotedNotification`)
//!
//! Step 1: the parity tests in
//! `crates/otb-core/tests/slice51_outage_ready_parity.rs` assert the TS
//! contract and MUST FAIL with `unimplemented!()` panics until the Rust
//! port lands (step 2).
//!
//! No grammy/Telegram I/O, no network/fetch, no live timers live here —
//! only the pending-flag state machines, readiness transitions, catalog
//! readiness predicates, and quote/escape text builders.
//!
//! DEVIATIONS from TS:
//! 1. `Date.now()` is implicit in TS (`markMessagesSkipped`,
//!    `noteChatSendSucceeded`, `msUntilAllowedSend`). The port takes an
//!    explicit `now_ms: u64` so the burst gap and chat interval stay pure
//!    and testable; callers pass the current epoch millis.
//! 2. `onReady` returns an unsubscribe closure in TS. The port returns a
//!    numeric subscription id; `remove_ready_handler` drops it.
//! 3. Ready handlers are `async` with a per-handler `try/catch` in TS. The
//!    port calls synchronous `FnMut(&str)` handlers in subscription order;
//!    a panicking handler is a caller bug, not a tolerated outcome.
//! 4. `buildQuotedNotification` takes `options?: { blankLineAfterTitle? }`
//!    in TS (absent or `true` both mean a blank line). The port takes
//!    `Option<bool>` with the same rule: only `Some(false)` collapses the
//!    separator to a single newline.
//! 5. `String.length` / `slice` count UTF-16 code units in TS. The port
//!    works on `char`s; parity tests use ASCII-only inputs where the two
//!    coincide.
//! 6. `u64` millisecond arithmetic uses saturating ops; TS numbers would go
//!    negative before `Math.max(0, ...)` clamps. Observable results match.

// ---------------------------------------------------------------------------
// Telegram outage notices (`app/services/telegram-outage-notice-service.ts`)
// ---------------------------------------------------------------------------

/// Minimum gap between chat sends after an outage notice (`CHAT_INTERVAL_MS`).
pub const OUTAGE_CHAT_INTERVAL_MS: u64 = 1000;

/// Window inside which repeated stale-message bursts stay suppressed
/// (`BURST_GAP_MS`).
pub const OUTAGE_BURST_GAP_MS: u64 = 60_000;

/// Which pending outage notice a result belongs to (TS:
/// `TelegramOutageNoticeKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutageNoticeKind {
    Undelivered,
    Skipped,
}

/// The pending flags snapshot a flush delivers (TS:
/// `TelegramOutageNoticesDue`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutageNoticesDue {
    pub undelivered: bool,
    pub skipped: bool,
}

/// Pending-flag state machine behind outage notice delivery (TS:
/// `TelegramOutageNoticeService`).
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct TelegramOutageNoticeService {
    undelivered_pending: bool,
    skipped_pending: bool,
    burst_active: bool,
    last_stale_at: u64,
    last_chat_send_at: u64,
    flush_in_flight: bool,
}

impl TelegramOutageNoticeService {
    /// Fresh service with nothing pending (TS: field initializers).
    pub fn new() -> Self {
        Self::default()
    }

    /// Flag that an assistant reply went undelivered (TS:
    /// `markAssistantReplyUndelivered`).
    pub fn mark_assistant_reply_undelivered(&mut self) {
        self.undelivered_pending = true;
    }

    /// Flag skipped messages unless inside an active burst window (TS:
    /// `markMessagesSkipped`).
    pub fn mark_messages_skipped(&mut self, now_ms: u64) {
        if self.burst_active && now_ms.saturating_sub(self.last_stale_at) <= OUTAGE_BURST_GAP_MS {
            self.last_stale_at = now_ms;
            return;
        }
        self.burst_active = true;
        self.skipped_pending = true;
        self.last_stale_at = now_ms;
    }

    /// End burst suppression (TS: `closeBurst`).
    pub fn close_burst(&mut self) {
        self.burst_active = false;
    }

    /// Stamp the last successful chat send (TS: `noteChatSendSucceeded`).
    pub fn note_chat_send_succeeded(&mut self, now_ms: u64) {
        self.last_chat_send_at = now_ms;
    }

    /// Millis until another chat send is allowed (TS:
    /// `msUntilAllowedSend`).
    pub fn ms_until_allowed_send(&self, now_ms: u64) -> u64 {
        if self.last_chat_send_at == 0 {
            return 0;
        }
        self.last_chat_send_at
            .saturating_add(OUTAGE_CHAT_INTERVAL_MS)
            .saturating_sub(now_ms)
    }

    /// Take the currently due notices, guarding concurrent flushes (TS:
    /// `takeDueNotices`).
    pub fn take_due_notices(&mut self) -> Option<OutageNoticesDue> {
        if self.flush_in_flight {
            return None;
        }
        if !self.undelivered_pending && !self.skipped_pending {
            return None;
        }
        self.flush_in_flight = true;
        Some(OutageNoticesDue {
            undelivered: self.undelivered_pending,
            skipped: self.skipped_pending,
        })
    }

    /// Clear the pending flag of a delivered notice (TS:
    /// `reportNoticeResult`).
    pub fn report_notice_result(&mut self, kind: OutageNoticeKind, delivered: bool) {
        if !delivered {
            return;
        }
        match kind {
            OutageNoticeKind::Undelivered => self.undelivered_pending = false,
            OutageNoticeKind::Skipped => self.skipped_pending = false,
        }
    }

    /// Release the flush guard (TS: `finishFlush`).
    pub fn finish_flush(&mut self) {
        self.flush_in_flight = false;
    }
}

// ---------------------------------------------------------------------------
// OpenCode ready lifecycle (`opencode/ready-lifecycle.ts`)
// ---------------------------------------------------------------------------

/// Tracks whether the OpenCode server is ready and fans notifications out
/// to subscribers (TS: `OpencodeReadyLifecycle`).
#[allow(dead_code)]
pub struct OpencodeReadyLifecycle {
    ready: bool,
    next_id: u64,
    handlers: Vec<(u64, Box<dyn FnMut(&str)>)>,
}

impl OpencodeReadyLifecycle {
    /// Fresh lifecycle, not ready with no subscribers (TS: field
    /// initializers).
    pub fn new() -> Self {
        Self {
            ready: false,
            next_id: 0,
            handlers: Vec::new(),
        }
    }

    /// Whether readiness was announced and not revoked (TS: `isReady`).
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Subscribe to ready notifications; returns a subscription id (TS:
    /// `onReady`, whose unsubscribe closure the id replaces).
    pub fn on_ready(&mut self, handler: Box<dyn FnMut(&str)>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.handlers.push((id, handler));
        id
    }

    /// Drop a subscription by id, reporting whether one existed.
    pub fn remove_ready_handler(&mut self, id: u64) -> bool {
        match self.handlers.iter().position(|(hid, _)| *hid == id) {
            Some(index) => {
                self.handlers.remove(index);
                true
            }
            None => false,
        }
    }

    /// Announce readiness once; later calls are ignored (TS:
    /// `notifyReady`).
    pub fn notify_ready(&mut self, reason: &str) -> bool {
        if self.ready {
            return false;
        }
        self.ready = true;
        for (_, handler) in self.handlers.iter_mut() {
            handler(reason);
        }
        true
    }

    /// Revoke readiness, only when currently ready (TS:
    /// `notifyUnavailable`).
    pub fn notify_unavailable(&mut self, _reason: &str) -> bool {
        if !self.ready {
            return false;
        }
        self.ready = false;
        true
    }
}

// ---------------------------------------------------------------------------
// Ready refresh catalog gates (`opencode/ready-refresh.ts`)
// ---------------------------------------------------------------------------

/// Bounded wait for a catalog naming the selected model (TS:
/// `MODEL_CATALOG_WAIT_TIMEOUT_MS`).
pub const MODEL_CATALOG_WAIT_TIMEOUT_MS: u64 = 3000;

/// Retry interval while waiting for that catalog (TS:
/// `MODEL_CATALOG_POLL_INTERVAL_MS`).
pub const MODEL_CATALOG_POLL_INTERVAL_MS: u64 = 500;

/// Background re-read interval through the warm-up window (TS:
/// `MODEL_CATALOG_WARMUP_POLL_INTERVAL_MS`).
pub const MODEL_CATALOG_WARMUP_POLL_INTERVAL_MS: u64 = 5000;

/// What one catalog reconciliation observed (TS:
/// `StoredModelReconcileResult`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StoredModelReconcile {
    pub catalog_available: bool,
    pub catalog_complete: bool,
    pub selected_model_listed: bool,
    pub stored_model_replaced: bool,
}

/// Whether the catalog settled completely (TS:
/// `isModelCatalogComplete`).
pub fn is_model_catalog_complete(result: StoredModelReconcile) -> bool {
    result.catalog_available && result.catalog_complete
}

/// Whether the catalog names the selected model or settled completely, so
/// the restore sequence may read it (TS:
/// `isModelCatalogReadyForRestore`).
pub fn is_model_catalog_ready_for_restore(result: StoredModelReconcile) -> bool {
    result.catalog_available && (result.selected_model_listed || result.catalog_complete)
}

/// Whether the background warm-up read observed a change watchers have not
/// seen yet (TS: the `storedModelReplaced || (!listed && nowListed)` gate
/// before `noteLateModelCatalogChange`).
pub fn should_note_late_catalog_change(
    stored_model_replaced: bool,
    listed_before: bool,
    listed_now: bool,
) -> bool {
    stored_model_replaced || (!listed_before && listed_now)
}

// ---------------------------------------------------------------------------
// Quoted notifications (`utils/telegram-markdown.ts`,
// `app/services/quoted-notification.ts`)
// ---------------------------------------------------------------------------

/// Escape MarkdownV2 reserved characters with a backslash (TS:
/// `escapePlainTextForTelegramMarkdownV2`).
pub fn escape_plain_text_for_telegram_markdown_v2(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(
            ch,
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{'
                | '}' | '.' | '!' | '\\'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Quote each line with `>` without escaping (TS:
/// `buildQuotedPlainText`).
pub fn build_quoted_plain_text(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Quote each line with `>`, escaping MarkdownV2 in every line (TS:
/// `buildQuotedMarkdownText`).
pub fn build_quoted_markdown_text(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {}", escape_plain_text_for_telegram_markdown_v2(line))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Quoted notice with its escaped and raw forms (TS:
/// `QuotedNotification`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotedNotification {
    pub text: String,
    pub raw_fallback_text: String,
}

/// Build the escaped notice and its raw fallback (TS:
/// `buildQuotedNotification`).
pub fn build_quoted_notification(
    title: &str,
    body: &str,
    blank_line_after_title: Option<bool>,
) -> QuotedNotification {
    let separator = if blank_line_after_title == Some(false) {
        "\n"
    } else {
        "\n\n"
    };
    QuotedNotification {
        text: format!(
            "{}{}{}",
            escape_plain_text_for_telegram_markdown_v2(title),
            separator,
            build_quoted_markdown_text(body)
        ),
        raw_fallback_text: format!(
            "{title}{separator}{}",
            build_quoted_plain_text(body)
        ),
    }
}
