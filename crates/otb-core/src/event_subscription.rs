//! Pure subscription lifecycle / routing gates.
//!
//! Rust port of the pure gating, state and filter logic in:
//! - `src/bot/services/event-subscription-service.ts` (subscription guard,
//!   chat destination, background ops counting, lost-run ending, reconnect
//!   classification)
//! - `src/bot/events/session-runtime-state.ts` (busy/idle routing)
//! - `src/bot/events/handlers/session-lifecycle-handler.ts` (footer gating,
//!   busy/idle event classification, session error truncation)
//! - `src/app/services/event-router.ts` (heartbeat / busy / idle routing,
//!   cross-session filtering, background announcements)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/event_subscription_parity.rs` assert the
//! TS contract and MUST FAIL with `unimplemented!()`
//! panics until the Rust port lands (step 2).
//!
//! No SSE subscription, Telegram I/O, timers, or filesystem access lives
//! here — only pure predicates, classifiers, and small value transforms.

// --- Subscription guard (ensureEventSubscription: empty directory returns early) ---

/// True when the service should subscribe for `directory`.
///
/// TS: `if (!directory) { return; }` — empty/whitespace roots never subscribe.
pub fn should_subscribe(directory: &str) -> bool {
    !directory.trim().is_empty()
}

/// True when a Telegram destination exists (both bot and chat id are set).
///
/// TS: `getChatDestination()` returns null unless bot and chat id are set.
pub fn has_chat_destination(bot_set: bool, chat_id_set: bool) -> bool {
    bot_set && chat_id_set
}

// --- Reconnect classification (restoreAfterReconnect) ---

/// What the stream reconnect must do about the run torn down mid-flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectAction {
    /// Restarted server: end the lost run as after `/abort`, then restore.
    EndRunLost,
    /// Same server: stop background ops and forget the live turn.
    StopBackgroundAndForgetTurn,
}

/// Classify a reconnect from the `serverRestarted` flag.
///
/// TS: `serverRestarted === true` ends the lost run; `false`/`null` stops
/// background operations and forgets the live turn.
pub fn classify_reconnect(server_restarted: Option<bool>) -> ReconnectAction {
    match server_restarted {
        Some(true) => ReconnectAction::EndRunLost,
        _ => ReconnectAction::StopBackgroundAndForgetTurn,
    }
}

/// True when the lost-run ending applies (V2 servers only).
///
/// TS: `endRunLostWithServer` returns early unless `opencodeServerVersion === "v2"`.
pub fn should_end_lost_run(server_version: &str) -> bool {
    server_version == "v2"
}

// --- Footer gating (session-lifecycle-handler: when the idle footer is sent) ---

/// Pure inputs for the run-footer decision on `session.idle`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterGate {
    /// The turn start (busy) was observed before the reply.
    pub turn_start_seen: bool,
    /// The `showAssistantRunFooter` setting is on.
    pub footer_enabled: bool,
    /// A `session.error` ended the turn.
    pub had_session_error: bool,
    /// A permission reject ended the turn.
    pub permission_rejected: bool,
    /// Shutdown idle (`interrupted` + `reason: "shutdown"`).
    pub shutdown_idle: bool,
    /// `/abort` (or similar) already marked the turn footerless.
    pub footer_suppressed: bool,
    /// The session is still the foreground one.
    pub is_foreground: bool,
    /// The reply completed after its turn already went idle.
    pub completed_after_idle: bool,
}

/// True when the idle handler should send the run footer.
///
/// TS: no footer when the start was never seen, the setting is off, an
/// error/reject/shutdown ended the turn, the turn was suppressed, the
/// session lost focus, or the reply arrived after the idle.
pub fn should_send_footer(gate: &FooterGate) -> bool {
    gate.turn_start_seen
        && gate.footer_enabled
        && !gate.had_session_error
        && !gate.permission_rejected
        && !gate.shutdown_idle
        && !gate.footer_suppressed
        && gate.is_foreground
        && !gate.completed_after_idle
}

/// Footer duration label inputs: `busy_at_ms` to `idle_at_ms`.
pub fn footer_duration_ms(busy_at_ms: i64, idle_at_ms: i64) -> i64 {
    idle_at_ms.saturating_sub(busy_at_ms).max(0)
}

// --- Busy / idle routing (event-router.ts) ---

/// True for the server heartbeat event.
pub fn is_heartbeat_event(event_type: &str) -> bool {
    event_type == "server.heartbeat"
}

/// True when the event marks the attached session busy.
///
/// TS busy set: assistant `message.updated` without `completed`,
/// `message.part.updated`, `message.part.delta`, `question.asked`,
/// `permission.asked`, `session.status` busy.
pub fn marks_session_busy(event_type: &str, completed: bool, busy_status: bool) -> bool {
    match event_type {
        "message.part.updated" | "message.part.delta" | "question.asked" | "permission.asked" => {
            true
        }
        "message.updated" => !completed,
        "session.status" => busy_status,
        _ => false,
    }
}

/// True when the event reports no work (assistant completed, idle, idle status).
pub fn reports_no_work(event_type: &str, completed: bool, idle_status: bool) -> bool {
    match event_type {
        "message.updated" => completed,
        "session.idle" => true,
        "session.status" => idle_status,
        _ => false,
    }
}

/// True when a progress event for `event_session` applies to `attached_session`.
pub fn event_session_matches(event_session: &str, attached_session: &str) -> bool {
    event_session == attached_session
}

// --- Session errors (session-lifecycle-handler) ---

/// Max Telegram-visible length for a session error report.
pub const SESSION_ERROR_MAX_CHARS: usize = 3700;

/// True when the error after a user-requested abort stays silent.
pub fn should_suppress_session_error(abort_requested: bool) -> bool {
    abort_requested
}

/// Truncate an oversized session error with a `...` marker within budget.
pub fn truncate_session_error(message: &str) -> String {
    // Mirrors `formatSessionMessage` in `handler-context.ts`: the message body
    // is capped at 3500 chars so the rendered `bot.session_error` report stays
    // within the 3700-char Telegram-visible budget.
    const MESSAGE_MAX_CHARS: usize = 3500;
    if message.chars().count() <= MESSAGE_MAX_CHARS {
        return message.to_string();
    }
    let head: String = message.chars().take(MESSAGE_MAX_CHARS - 3).collect();
    format!("{head}...")
}

// --- Tool line visibility (pending calls compose silently) ---

/// True when a tool call with `status` gets a visible line.
///
/// TS: `pending` (still composed, no arguments yet) shows nothing; a bare
/// running call with no arguments still shows.
pub fn should_show_tool_line(status: &str) -> bool {
    status != "pending"
}

/// True when `status` is the running marker.
pub fn is_tool_running(status: &str) -> bool {
    status == "running"
}

// --- Background operations (stopBackgroundOperations / countBackgroundOperations) ---

/// Total background operations: background commands + followed subagents.
pub fn count_background_operations(background_commands: usize, followed_subagents: usize) -> usize {
    background_commands + followed_subagents
}

/// Short label for an untitled background session (first 8 chars, no full id).
pub fn short_session_id(session_id: &str) -> String {
    if session_id.chars().count() <= 8 {
        return session_id.to_string();
    }
    session_id.chars().take(8).collect()
}

// --- Compact fallback (most recently started still-running tool takes the line) ---

/// Minimal compact-mode candidate: call id, stable start order, still running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactCandidate {
    pub call_id: String,
    pub started_order: u64,
    pub running: bool,
}

/// Pick the compact line holder: most recently started still-running tool,
/// or `None` when nothing runs.
pub fn pick_compact_tool(candidates: &[CompactCandidate]) -> Option<String> {
    candidates
        .iter()
        .filter(|c| c.running)
        .max_by_key(|c| c.started_order)
        .map(|c| c.call_id.clone())
}

// --- Lost-run idle synthesis (buildInterruptedIdleEvents) ---

/// Descriptor for one synthesized lost-run event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LostRunEvent {
    pub id: String,
    pub event_type: String,
    pub interrupted: bool,
}

/// The two events the ending synthesizes: `session.status` idle then
/// `session.idle` with `interrupted: true`.
pub fn build_interrupted_idle_events(session_id: &str) -> Vec<LostRunEvent> {
    vec![
        LostRunEvent {
            id: format!("{session_id}:lost:status"),
            event_type: "session.status".to_string(),
            interrupted: false,
        },
        LostRunEvent {
            id: format!("{session_id}:lost:idle"),
            event_type: "session.idle".to_string(),
            interrupted: true,
        },
    ]
}

// --- External input + prompt pickup (inbox-reconnect slice) ---

/// True when input typed outside Telegram should notify the chat.
pub fn should_notify_external_input(suppressed: bool) -> bool {
    !suppressed
}

/// How a queued inbox prompt was picked up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptPickup {
    Steer,
    Queue,
}

/// True when the pickup closes the answered run (queue) vs staying in it (steer).
pub fn pickup_closes_run(pickup: PromptPickup) -> bool {
    pickup == PromptPickup::Queue
}
