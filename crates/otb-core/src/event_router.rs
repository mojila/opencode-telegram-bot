//! Event router gates (parity library only).
//!
//! Rust port of `src/app/services/event-router.ts` (`createEventRouter`):
//! - `getEventSessionId` (session identity always comes from the event)
//! - heartbeat is subscription-wide work for the subscribed directory
//! - attached-busy marking from the event's own session
//! - `session.created` / `session.updated` cache ingest when `info.directory`
//!   is present
//! - `goal.changed` short-circuit, background tracking with the foreground
//!   session resolved to `Some(id)` or `None`, then `summaryAggregator`
//!   delivery.
//!
//! No SDK, Telegram, or filesystem I/O lives here — only pure predicates and
//! small value transforms. The live router still lives in TS; wire this module
//! in when `otb-core` owns event dispatch.
//!
//! Wiring point: replace TS `createEventRouter` with a dispatcher built on
//! [`route_event`] once the Rust runtime owns subscriptions.

/// Session identity read from the event itself
/// (TS: `getEventSessionId`: `properties.sessionID || info.sessionID ||
/// `part.sessionID`, else `null`).
pub fn get_event_session_id(
    session_id: Option<&str>,
    info_session_id: Option<&str>,
    part_session_id: Option<&str>,
) -> Option<String> {
    for candidate in [session_id, info_session_id, part_session_id] {
        if let Some(id) = candidate {
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// True for the server heartbeat (TS: subscription-wide work for the
/// subscribed directory).
pub fn is_heartbeat_event(event_type: &str) -> bool {
    event_type == "server.heartbeat"
}

/// True when the event marks the attached session busy
/// (TS: `shouldMarkAttachedBusyFromEvent`: `session.status` busy,
/// assistant `message.updated` without `completed`, `message.part.updated`,
/// `message.part.delta`, `question.asked`, `permission.asked`).
pub fn should_mark_attached_busy(
    event_type: &str,
    assistant_uncompleted: bool,
    busy_status: bool,
) -> bool {
    match event_type {
        "session.status" => busy_status,
        "message.updated" => assistant_uncompleted,
        "message.part.updated" | "message.part.delta" | "question.asked" | "permission.asked" => {
            true
        }
        _ => false,
    }
}

/// True when the attached snapshot belongs to the event session and the event
/// marks busy (TS: `attached && sessionId === attached.sessionId &&
/// shouldMarkAttachedBusyFromEvent(event)`).
pub fn should_mark_attached_busy_for_snapshot(
    attached_session_id: Option<&str>,
    event_session_id: Option<&str>,
    event_type: &str,
    assistant_uncompleted: bool,
    busy_status: bool,
) -> bool {
    match (attached_session_id, event_session_id) {
        (Some(attached), Some(event)) if attached == event => {
            should_mark_attached_busy(event_type, assistant_uncompleted, busy_status)
        }
        _ => false,
    }
}

/// True when the event feeds the session cache
/// (TS: `session.created` / `session.updated` with `info.directory`).
pub fn should_ingest_session_info(event_type: &str, has_directory: bool) -> bool {
    matches!(event_type, "session.created" | "session.updated") && has_directory
}

/// True for the goal short-circuit (TS: `goal.changed` returns before
/// background/aggregator delivery).
pub fn is_goal_changed_event(event_type: &str) -> bool {
    event_type == "goal.changed"
}

/// Resolve the foreground session id passed to background tracking
/// (TS: `sessionId && isForegroundSession(sessionId) ? sessionId : null`).
pub fn resolve_foreground_session_id(
    event_session_id: Option<&str>,
    is_foreground: bool,
) -> Option<String> {
    match event_session_id {
        Some(id) if !id.is_empty() && is_foreground => Some(id.to_string()),
        _ => None,
    }
}

/// What one routed event should do (TS: the body of `createEventRouter`'s
/// returned closure, transport-free subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDecision {
    /// Run busy reconciliation for the subscribed directory (heartbeat).
    pub reconcile_busy: bool,
    /// Mark the attached session busy.
    pub mark_attached_busy: bool,
    /// Ingest `info` into the session cache.
    pub ingest_session_cache: bool,
    /// Short-circuit after `goal.changed` handling.
    pub goal_changed: bool,
    /// Foreground session id for background tracking (`None` = background).
    pub foreground_session_id: Option<String>,
    /// Forward to `summaryAggregator.processEvent`.
    pub forward_to_aggregator: bool,
}

/// Pure dispatch for one event (TS: `createEventRouter(...)` closure).
#[allow(clippy::too_many_arguments)]
pub fn route_event(
    event_type: &str,
    subscribed_directory: &str,
    attached_session_id: Option<&str>,
    event_session_id: Option<&str>,
    assistant_uncompleted: bool,
    busy_status: bool,
    info_has_directory: bool,
    is_foreground: bool,
) -> RouteDecision {
    let _ = subscribed_directory;
    let goal_changed = is_goal_changed_event(event_type);
    RouteDecision {
        reconcile_busy: is_heartbeat_event(event_type),
        mark_attached_busy: should_mark_attached_busy_for_snapshot(
            attached_session_id,
            event_session_id,
            event_type,
            assistant_uncompleted,
            busy_status,
        ),
        ingest_session_cache: should_ingest_session_info(event_type, info_has_directory),
        goal_changed,
        foreground_session_id: if goal_changed {
            None
        } else {
            resolve_foreground_session_id(event_session_id, is_foreground)
        },
        forward_to_aggregator: !goal_changed,
    }
}
