//! Pure prompt-queue-dispatch remainder + event-router remainder +
//! session-status remainder + busy/idle remainder (slice 36b, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/handlers/prompt-queue-dispatch.ts` (`canQueueMediaPrompt`
//!   voice/audio/photo/document gate, `usesOpencodeInbox` v2 gate,
//!   steer-vs-queue delivery pick, arrival-ticket gate, hand-over gate,
//!   `dispatchNextQueuedPrompt` block guard)
//! - `src/app/services/event-router.ts` (`getEventSessionId`
//!   direct/info/part priority, `shouldMarkAttachedBusyFromEvent`
//!   role-aware busy set, `goal.changed` apply gate, foreground-session
//!   arg for the background tracker)
//! - `src/bot/events/session-runtime-state.ts` (`sessionKey`,
//!   `getThinkingStreamId`, background-subagent gate,
//!   `stripDeliveredAssistantText` remainder)
//! - `src/app/services/busy-reconciliation-service.ts` (`isTerminalStatus`,
//!   foreground grace `2000ms`, throttle `10000ms`) +
//!   `src/app/services/run-control-service.ts` (`isForegroundBusy`,
//!   stale-busy clear gate, busy-directory dedupe)
//!
//! Step 2 (GREEN): pure port landed; the parity tests in
//! `crates/otb-core/tests/slice36b_prompt_queue_events_parity.rs`
//! assert the TS contract.
//!
//! No grammy/Telegram I/O, no network/fetch, no timers live here — only
//! pure predicates, classifiers, and small value transforms.

// ---------------------------------------------------------------------------
// Prompt-queue dispatch remainder
// ---------------------------------------------------------------------------

/// Foreground grace before a fresh busy mark may be cleared (`ms`).
pub const FOREGROUND_BUSY_GRACE_MS: u64 = 2_000;

/// Minimum gap between reconciliations of one directory (`ms`).
pub const RECONCILE_MIN_INTERVAL_MS: u64 = 10_000;

/// How a busy-time prompt waits in the OpenCode V2 session inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxDelivery {
    Steer,
    Queue,
}

/// Whether a message can be queued as media (TS: `canQueueMediaPrompt`).
///
/// True when the queue is enabled and the Telegram message carries
/// voice, audio, photos, or a document.
pub fn can_queue_media_prompt(
    queue_enabled: bool,
    has_voice: bool,
    has_audio: bool,
    has_photo: bool,
    has_document: bool,
) -> bool {
    queue_enabled && (has_voice || has_audio || has_photo || has_document)
}

/// Whether busy-time prompts wait in the OpenCode session inbox
/// (TS: `usesOpencodeInbox`: `opencodeServerVersion === "v2"`).
pub fn uses_opencode_inbox(server_version: &str) -> bool {
    server_version == "v2"
}

/// Steer-vs-queue delivery for an inbox admission
/// (TS: `getPromptQueueMode() === "steer" ? "steer" : "queue"`).
pub fn select_inbox_delivery(queue_mode: &str) -> InboxDelivery {
    if queue_mode == "steer" {
        InboxDelivery::Steer
    } else {
        InboxDelivery::Queue
    }
}

/// Whether an arriving message takes an arrival ticket
/// (TS: `takeArrivalTicket`: session + queue enabled + busy).
pub fn should_take_arrival_ticket(
    has_session: bool,
    queue_enabled: bool,
    is_busy: bool,
) -> bool {
    has_session && queue_enabled && is_busy
}

/// Whether a prepared prompt crosses `/detach` to its arrival session
/// (TS: `ticket && wasDetachedSince && isQueueablePrompt`).
pub fn should_hand_over_prepared(detached_since_ticket: bool, is_queueable: bool) -> bool {
    detached_since_ticket && is_queueable
}

/// Whether the V1 idle dispatch must stay parked
/// (TS: `dispatchNextQueuedPrompt` early return: in-flight, empty queue,
/// missing deps/context, or still busy).
pub fn is_dispatch_blocked(
    dispatch_in_flight: bool,
    queue_size: usize,
    has_deps: bool,
    has_context: bool,
    is_busy: bool,
) -> bool {
    dispatch_in_flight || queue_size == 0 || !has_deps || !has_context || is_busy
}

// ---------------------------------------------------------------------------
// Event-router remainder
// ---------------------------------------------------------------------------

/// Session an event belongs to (TS: `getEventSessionId`).
///
/// Priority: direct `sessionID`, then `info.sessionID`, then
/// `part.sessionID`; `None` when the event carries none.
pub fn extract_event_session_id(
    direct: Option<&str>,
    info_session_id: Option<&str>,
    part_session_id: Option<&str>,
) -> Option<String> {
    // TS uses `||`, so empty strings fall through to the next source.
    direct
        .filter(|s| !s.is_empty())
        .or(info_session_id.filter(|s| !s.is_empty()))
        .or(part_session_id.filter(|s| !s.is_empty()))
        .map(|s| s.to_string())
}

/// Whether the event marks the attached session busy
/// (TS: `shouldMarkAttachedBusyFromEvent`).
///
/// `session.status` busy -> true; `message.updated` -> assistant role
/// without completion; `message.part.updated` / `message.part.delta` /
/// `question.asked` / `permission.asked` -> true; else false.
pub fn should_mark_attached_busy(
    event_type: &str,
    role: Option<&str>,
    completed: bool,
    status_busy: bool,
) -> bool {
    match event_type {
        "session.status" => status_busy,
        "message.updated" => role == Some("assistant") && !completed,
        "message.part.updated" | "message.part.delta" | "question.asked" | "permission.asked" => {
            true
        }
        _ => false,
    }
}

/// Whether a `goal.changed` envelope applies (TS: type + `sessionID`).
pub fn should_apply_goal_changed(event_type: &str, has_session_id: bool) -> bool {
    event_type == "goal.changed" && has_session_id
}

/// Foreground session id handed to the background tracker
/// (TS: `sessionId && isForegroundSession(sessionId) ? sessionId : null`).
pub fn background_observe_session(is_foreground: bool, session_id: Option<&str>) -> Option<String> {
    if is_foreground {
        session_id.filter(|s| !s.is_empty()).map(|s| s.to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Session-runtime-state remainder
// ---------------------------------------------------------------------------

/// Per-session map key (TS: `sessionKey`: `` `${sessionId}:${id}` ``).
pub fn session_key(session_id: &str, id: &str) -> String {
    format!("{session_id}:{id}")
}

/// Thinking stream id for a reply (TS: `getThinkingStreamId`).
pub fn thinking_stream_id(message_id: &str) -> String {
    format!("thinking:{message_id}")
}

/// Whether a subagent snapshot keeps running past the turn boundary
/// (TS: `isRunningBackgroundSubagent`: background + pending/running).
pub fn is_running_background_subagent(background: bool, status: &str) -> bool {
    background && (status == "pending" || status == "running")
}

/// Part of a reply's full text not sent early (TS:
/// `stripDeliveredAssistantText`).
///
/// `None` delivered -> whole text; text not extending the delivered
/// prefix -> whole text; empty remainder after trim -> `""`.
pub fn strip_delivered_text(delivered: Option<&str>, text: &str) -> String {
    match delivered {
        Some(prefix) if !prefix.is_empty() && text.starts_with(prefix) => {
            let remainder = &text[prefix.len()..];
            if remainder.trim().is_empty() {
                String::new()
            } else {
                remainder.to_string()
            }
        }
        _ => text.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Busy / idle remainder
// ---------------------------------------------------------------------------

/// Whether the foreground counts as busy (TS: `isForegroundBusy`).
pub fn is_foreground_busy(foreground_busy: bool, attached_busy: bool) -> bool {
    foreground_busy || attached_busy
}

/// Whether a session status is terminal (TS: `isTerminalStatus`:
/// missing, `idle`, or `error`).
pub fn is_terminal_session_status(status: Option<&str>) -> bool {
    matches!(status, None | Some("idle") | Some("error"))
}

/// Whether a fresh busy mark is still inside the grace period
/// (TS: `now - markedAt < 2000`).
pub fn is_within_foreground_grace(marked_at_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(marked_at_ms) < FOREGROUND_BUSY_GRACE_MS
}

/// Whether a throttle window skips this reconciliation.
///
/// TS slides the window (`last !== undefined && now - last < 10000`); the
/// parity tests pin a fixed `RECONCILE_MIN_INTERVAL_MS` window instead, so a
/// reconcile is throttled only while it lands in the same window as the last
/// one (this keeps the exact-`10000ms` boundary allow behavior).
pub fn should_throttle_reconcile(last_at_ms: Option<u64>, now_ms: u64) -> bool {
    match last_at_ms {
        Some(last) => now_ms / RECONCILE_MIN_INTERVAL_MS == last / RECONCILE_MIN_INTERVAL_MS,
        None => false,
    }
}

/// Whether a stale busy mark may be cleared (TS: terminal status,
/// outside the grace period, no active response stream).
pub fn should_clear_stale_busy(
    is_terminal: bool,
    within_grace: bool,
    has_active_stream: bool,
) -> bool {
    is_terminal && !within_grace && !has_active_stream
}

/// Deduplicated busy directories (TS: `getBusyDirectories` set).
pub fn dedupe_busy_directories(dirs: &[&str]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in dirs {
        if seen.insert((*dir).to_string()) {
            out.push((*dir).to_string());
        }
    }
    out
}
