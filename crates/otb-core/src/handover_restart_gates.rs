//! Handover-delivery + queue-dispatch-ticket + auto-restart gates (slice 100, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/handlers/prompt-handover.ts` (`STATUS_POLL_INTERVAL_MS` 1500,
//!   `TURN_START_GRACE_MS` 5000, `readSessionStatus` error/no-data `unknown`
//!   vs `busy`/`retry` -> `busy` else `idle`, `isLive` record-identity gate,
//!   `startDeliveryLoop` pending/running/record guards, `handOverPreparedPrompt`
//!   record/detached/deps gate, `steer` vs `queue` delivery choice,
//!   `waitForTurnEnd` idle-after-busy-or-grace gate, `notifyDeliveryFailure`
//!   attached-only gate)
//! - `src/bot/handlers/prompt-queue-dispatch.ts` ticket subset
//!   (`takeArrivalTicket` session/enabled/busy gate, `tryEnqueuePromptIfBusy`
//!   ticket/detached/queueable handover branch)
//! - `src/opencode/auto-restart.ts` (`SERVER_READY_TIMEOUT_MS` 10000,
//!   `SERVER_READY_POLL_INTERVAL_MS` 500, `HEALTH_CHECK_TIMEOUT_MS` 3000,
//!   `start` started/enabled/target gate, `checkAndRestart` reentrancy gate,
//!   healthy-probe `!error && data?.healthy` gate, unavailable-resolution
//!   `refuse` / `answers|connected` / spawn dispatch, container no-spawn gate)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O, OpenCode clients (`session.status`,
//!   `session.promptAsync`), `promptHandover` / `promptQueue` mutation,
//!   `getCurrentSession`, `getPromptQueueMode`, `isForegroundBusy`,
//!   `setTimeout`/`setInterval` waits, `Date.now()` clocks (arrive as explicit
//!   `elapsed_ms` args), `logger`, `t(...)` replies.
//!
//! Deviations: none intended. `Date.now() - sentAt >= TURN_START_GRACE_MS`
//! arrives as an explicit `elapsed_ms` so tests stay deterministic.
//!
//! Step 2 (GREEN): pure port landed; the parity tests in
//! `crates/otb-core/tests/slice100_handover_restart_parity.rs`
//! assert the TS contract.

/// Poll interval for the handover delivery loop (TS: `STATUS_POLL_INTERVAL_MS`).
pub const HANDOVER_STATUS_POLL_INTERVAL_MS: u64 = 1500;

/// Grace after a handed-over send before an unseen turn counts as over
/// (TS: `TURN_START_GRACE_MS`).
pub const HANDOVER_TURN_START_GRACE_MS: u64 = 5000;

/// How long a spawned server may take to become ready (TS: `SERVER_READY_TIMEOUT_MS`).
pub const AUTO_RESTART_READY_TIMEOUT_MS: u64 = 10_000;

/// Ready-poll interval while waiting (TS: `SERVER_READY_POLL_INTERVAL_MS`).
pub const AUTO_RESTART_READY_POLL_INTERVAL_MS: u64 = 500;

/// Health-check budget for restart probes (TS: `HEALTH_CHECK_TIMEOUT_MS`).
pub const AUTO_RESTART_HEALTH_CHECK_TIMEOUT_MS: u64 = 3000;

/// Session liveness behind the handover delivery loop (TS: `readSessionStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoverSessionStatus {
    Busy,
    Idle,
    Unknown,
}

/// What an unavailable-server resolution asks the restart service to do
/// (TS: `resolution.kind` in `checkAndRestart`: `refuse` logs and stops,
/// `answers`/`connected` adopts the live server, anything else spawns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableResolutionAction {
    Refuse,
    Adopt,
    Spawn,
}

/// `unknown` on transport failure, `busy` on `busy`/`retry`, else `idle`
/// (TS: `readSessionStatus`).
pub fn classify_handover_session_status(
    has_error: bool,
    has_data: bool,
    status_type: Option<&str>,
) -> HandoverSessionStatus {
    if has_error {
        return HandoverSessionStatus::Unknown;
    }
    if !has_data {
        return HandoverSessionStatus::Unknown;
    }
    match status_type {
        Some("busy") | Some("retry") => HandoverSessionStatus::Busy,
        _ => HandoverSessionStatus::Idle,
    }
}

/// `true` when the loop still owns its record (TS: `promptHandover.get(id) === record`).
pub fn is_handover_live(same_record: bool) -> bool {
    same_record
}

/// `true` when a new delivery loop may start (TS: `startDeliveryLoop`:
/// pending prompts, no running loop, record present).
pub fn should_start_handover_delivery(
    has_pending: bool,
    already_running: bool,
    has_record: bool,
) -> bool {
    has_pending && !already_running && has_record
}

/// `true` when a prepared prompt may follow its detached session (TS:
/// `handOverPreparedPrompt`: record, `wasDetachedSince`, deps).
pub fn can_hand_over_prepared(has_record: bool, was_detached: bool, has_deps: bool) -> bool {
    has_record && was_detached && has_deps
}

/// V2 inbox delivery for a handed-over prompt (TS:
/// `getPromptQueueMode() === "steer" ? "steer" : "queue"`).
pub fn resolve_handover_delivery(is_steer_mode: bool) -> &'static str {
    if is_steer_mode {
        "steer"
    } else {
        "queue"
    }
}

/// `true` when the handed-over turn has ended (TS: `waitForTurnEnd`:
/// `idle && (seenBusy || elapsed >= TURN_START_GRACE_MS)`).
pub fn should_end_handover_turn(
    status: HandoverSessionStatus,
    seen_busy: bool,
    elapsed_ms: u64,
) -> bool {
    if status != HandoverSessionStatus::Idle {
        return false;
    }
    seen_busy || elapsed_ms >= HANDOVER_TURN_START_GRACE_MS
}

/// `true` when a handover failure is posted (TS: `notifyDeliveryFailure`
/// only for the attached session).
pub fn should_notify_handover_failure(is_attached: bool) -> bool {
    is_attached
}

/// `true` when the auto-restart service may start (TS: `start`:
/// not started, `autoRestartEnabled`, local target resolved).
pub fn should_start_auto_restart(already_started: bool, enabled: bool, has_target: bool) -> bool {
    !already_started && enabled && has_target
}

/// `true` when a restart check is skipped (TS: `checkAndRestart`:
/// `checkInProgress || !localTarget`).
pub fn should_skip_restart_check(in_progress: bool, has_target: bool) -> bool {
    in_progress || !has_target
}

/// `true` when the probe counts as healthy (TS: `!error && data?.healthy`).
pub fn is_healthy_probe(has_error: bool, has_data: bool, healthy: bool) -> bool {
    !has_error && has_data && healthy
}

/// Dispatch for an unavailable-server resolution (TS: `refuse` stops,
/// `answers`/`connected` adopts, anything else spawns a local server).
pub fn classify_unavailable_resolution(kind: &str) -> UnavailableResolutionAction {
    match kind {
        "refuse" => UnavailableResolutionAction::Refuse,
        "answers" | "connected" => UnavailableResolutionAction::Adopt,
        _ => UnavailableResolutionAction::Spawn,
    }
}

/// `true` when the restart may spawn a local server (TS: never inside a
/// container runtime).
pub fn should_spawn_local_server(is_container: bool) -> bool {
    !is_container
}

/// `true` when an arrival ticket is taken (TS: `takeArrivalTicket`:
/// session, queue enabled, busy).
pub fn should_take_arrival_ticket(has_session: bool, queue_enabled: bool, is_busy: bool) -> bool {
    has_session && queue_enabled && is_busy
}

/// `true` when a prepared prompt follows its detached session (TS:
/// `tryEnqueuePromptIfBusy` handover branch: ticket, detached, queueable).
pub fn should_hand_over_prepared_prompt(
    has_ticket: bool,
    was_detached: bool,
    queueable: bool,
) -> bool {
    has_ticket && was_detached && queueable
}
