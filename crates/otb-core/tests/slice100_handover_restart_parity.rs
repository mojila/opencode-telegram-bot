//! Slice 100 parity: handover-delivery + queue-dispatch-ticket + auto-restart gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/handlers/prompt-handover.ts` (`STATUS_POLL_INTERVAL_MS`,
//! `TURN_START_GRACE_MS`, `readSessionStatus` unknown/busy/idle mapping,
//! `isLive` identity gate, `startDeliveryLoop` pending/running/record guards,
//! `handOverPreparedPrompt` record/detached/deps gate, steer-vs-queue delivery,
//! `waitForTurnEnd` idle-after-busy-or-grace gate, attached-only failure notice),
//! `src/bot/handlers/prompt-queue-dispatch.ts` (`takeArrivalTicket`
//! session/enabled/busy gate, `tryEnqueuePromptIfBusy` ticket/detached/queueable
//! handover branch) and `src/opencode/auto-restart.ts`
//! (`SERVER_READY_TIMEOUT_MS`, `SERVER_READY_POLL_INTERVAL_MS`,
//! `HEALTH_CHECK_TIMEOUT_MS`, start started/enabled/target gate, restart
//! reentrancy gate, `!error && healthy` probe gate, `refuse` / `answers` +
//! `connected` / spawn resolution dispatch, container no-spawn gate).
//!
//! Already covered elsewhere and NOT re-tested here:
//! `PromptHandover` record bookkeeping (`prompt_handover_gates`),
//! `PromptQueueManager` bounds (`prompt_queue_manager_gates`),
//! `resolveLocalOpencodeTarget` URL math (`local_process_gates`),
//! `isQueueablePrompt` content/command/button math (`message_ingest`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `handover_restart_gates` is ported (only the constant-value tests pass).

use otb_core::handover_restart_gates::*;

// ---- constants (5) ----

#[test]
fn handover_poll_interval_value_parity() {
    assert_eq!(HANDOVER_STATUS_POLL_INTERVAL_MS, 1500);
}

#[test]
fn handover_grace_value_parity() {
    assert_eq!(HANDOVER_TURN_START_GRACE_MS, 5000);
}

#[test]
fn restart_ready_timeout_value_parity() {
    assert_eq!(AUTO_RESTART_READY_TIMEOUT_MS, 10_000);
}

#[test]
fn restart_ready_poll_value_parity() {
    assert_eq!(AUTO_RESTART_READY_POLL_INTERVAL_MS, 500);
}

#[test]
fn restart_health_check_value_parity() {
    assert_eq!(AUTO_RESTART_HEALTH_CHECK_TIMEOUT_MS, 3000);
}

// ---- readSessionStatus (6) ----

#[test]
fn handover_status_error_is_unknown_parity() {
    assert_eq!(
        classify_handover_session_status(true, true, Some("busy")),
        HandoverSessionStatus::Unknown
    );
    assert_eq!(
        classify_handover_session_status(true, false, None),
        HandoverSessionStatus::Unknown
    );
}

#[test]
fn handover_status_no_data_is_unknown_parity() {
    assert_eq!(
        classify_handover_session_status(false, false, Some("idle")),
        HandoverSessionStatus::Unknown
    );
}

#[test]
fn handover_status_busy_parity() {
    assert_eq!(
        classify_handover_session_status(false, true, Some("busy")),
        HandoverSessionStatus::Busy
    );
}

#[test]
fn handover_status_retry_is_busy_parity() {
    assert_eq!(
        classify_handover_session_status(false, true, Some("retry")),
        HandoverSessionStatus::Busy
    );
}

#[test]
fn handover_status_idle_parity() {
    assert_eq!(
        classify_handover_session_status(false, true, Some("idle")),
        HandoverSessionStatus::Idle
    );
}

#[test]
fn handover_status_missing_type_is_idle_parity() {
    assert_eq!(
        classify_handover_session_status(false, true, None),
        HandoverSessionStatus::Idle
    );
    assert_eq!(
        classify_handover_session_status(false, true, Some("other")),
        HandoverSessionStatus::Idle
    );
}

// ---- isLive (2) ----

#[test]
fn handover_live_same_record_parity() {
    assert!(is_handover_live(true));
}

#[test]
fn handover_live_stale_record_parity() {
    assert!(!is_handover_live(false));
}

// ---- startDeliveryLoop (4) ----

#[test]
fn handover_delivery_starts_when_pending_parity() {
    assert!(should_start_handover_delivery(true, false, true));
}

#[test]
fn handover_delivery_skips_without_pending_parity() {
    assert!(!should_start_handover_delivery(false, false, true));
}

#[test]
fn handover_delivery_wakes_instead_when_running_parity() {
    assert!(!should_start_handover_delivery(true, true, true));
}

#[test]
fn handover_delivery_skips_without_record_parity() {
    assert!(!should_start_handover_delivery(true, false, false));
}

// ---- handOverPreparedPrompt gate (3) ----

#[test]
fn handover_prepared_ready_parity() {
    assert!(can_hand_over_prepared(true, true, true));
}

#[test]
fn handover_prepared_needs_record_parity() {
    assert!(!can_hand_over_prepared(false, true, true));
}

#[test]
fn handover_prepared_needs_detach_and_deps_parity() {
    assert!(!can_hand_over_prepared(true, false, true));
    assert!(!can_hand_over_prepared(true, true, false));
}

// ---- delivery (2) ----

#[test]
fn handover_delivery_steer_parity() {
    assert_eq!(resolve_handover_delivery(true), "steer");
}

#[test]
fn handover_delivery_queue_parity() {
    assert_eq!(resolve_handover_delivery(false), "queue");
}

// ---- waitForTurnEnd (5) ----

#[test]
fn handover_turn_busy_keeps_waiting_parity() {
    assert!(!should_end_handover_turn(
        HandoverSessionStatus::Busy,
        true,
        10_000
    ));
}

#[test]
fn handover_turn_idle_after_busy_ends_parity() {
    assert!(should_end_handover_turn(
        HandoverSessionStatus::Idle,
        true,
        0
    ));
}

#[test]
fn handover_turn_idle_after_grace_ends_parity() {
    assert!(should_end_handover_turn(
        HandoverSessionStatus::Idle,
        false,
        5000
    ));
    assert!(should_end_handover_turn(
        HandoverSessionStatus::Idle,
        false,
        9000
    ));
}

#[test]
fn handover_turn_idle_early_keeps_waiting_parity() {
    assert!(!should_end_handover_turn(
        HandoverSessionStatus::Idle,
        false,
        0
    ));
    assert!(!should_end_handover_turn(
        HandoverSessionStatus::Idle,
        false,
        4999
    ));
}

#[test]
fn handover_turn_unknown_keeps_waiting_parity() {
    assert!(!should_end_handover_turn(
        HandoverSessionStatus::Unknown,
        true,
        10_000
    ));
    assert!(!should_end_handover_turn(
        HandoverSessionStatus::Unknown,
        false,
        10_000
    ));
}

// ---- notifyDeliveryFailure (2) ----

#[test]
fn handover_failure_notifies_when_attached_parity() {
    assert!(should_notify_handover_failure(true));
}

#[test]
fn handover_failure_silent_when_detached_parity() {
    assert!(!should_notify_handover_failure(false));
}

// ---- auto-restart start (4) ----

#[test]
fn restart_starts_fresh_parity() {
    assert!(should_start_auto_restart(false, true, true));
}

#[test]
fn restart_skips_when_started_parity() {
    assert!(!should_start_auto_restart(true, true, true));
}

#[test]
fn restart_skips_when_disabled_parity() {
    assert!(!should_start_auto_restart(false, false, true));
}

#[test]
fn restart_skips_without_target_parity() {
    assert!(!should_start_auto_restart(false, true, false));
}

// ---- checkAndRestart reentrancy (3) ----

#[test]
fn restart_check_skips_in_flight_parity() {
    assert!(should_skip_restart_check(true, true));
}

#[test]
fn restart_check_skips_without_target_parity() {
    assert!(should_skip_restart_check(false, false));
    assert!(should_skip_restart_check(true, false));
}

#[test]
fn restart_check_runs_when_idle_with_target_parity() {
    assert!(!should_skip_restart_check(false, true));
}

// ---- healthy probe (3) ----

#[test]
fn restart_probe_healthy_parity() {
    assert!(is_healthy_probe(false, true, true));
}

#[test]
fn restart_probe_error_is_unhealthy_parity() {
    assert!(!is_healthy_probe(true, true, true));
}

#[test]
fn restart_probe_missing_or_degraded_parity() {
    assert!(!is_healthy_probe(false, false, true));
    assert!(!is_healthy_probe(false, true, false));
}

// ---- resolution dispatch (4) ----

#[test]
fn restart_resolution_refuse_parity() {
    assert_eq!(
        classify_unavailable_resolution("refuse"),
        UnavailableResolutionAction::Refuse
    );
}

#[test]
fn restart_resolution_answers_adopts_parity() {
    assert_eq!(
        classify_unavailable_resolution("answers"),
        UnavailableResolutionAction::Adopt
    );
}

#[test]
fn restart_resolution_connected_adopts_parity() {
    assert_eq!(
        classify_unavailable_resolution("connected"),
        UnavailableResolutionAction::Adopt
    );
}

#[test]
fn restart_resolution_other_spawns_parity() {
    assert_eq!(
        classify_unavailable_resolution("starting"),
        UnavailableResolutionAction::Spawn
    );
    assert_eq!(
        classify_unavailable_resolution("missing"),
        UnavailableResolutionAction::Spawn
    );
    assert_eq!(
        classify_unavailable_resolution(""),
        UnavailableResolutionAction::Spawn
    );
}

// ---- container spawn (2) ----

#[test]
fn restart_spawns_on_host_parity() {
    assert!(should_spawn_local_server(false));
}

#[test]
fn restart_skips_spawn_in_container_parity() {
    assert!(!should_spawn_local_server(true));
}

// ---- arrival ticket (3) ----

#[test]
fn arrival_ticket_taken_when_busy_parity() {
    assert!(should_take_arrival_ticket(true, true, true));
}

#[test]
fn arrival_ticket_skips_without_session_parity() {
    assert!(!should_take_arrival_ticket(false, true, true));
}

#[test]
fn arrival_ticket_skips_when_idle_or_disabled_parity() {
    assert!(!should_take_arrival_ticket(true, false, true));
    assert!(!should_take_arrival_ticket(true, true, false));
}

// ---- tryEnqueuePromptIfBusy handover branch (2) ----

#[test]
fn handover_prompt_branch_taken_parity() {
    assert!(should_hand_over_prepared_prompt(true, true, true));
}

#[test]
fn handover_prompt_branch_skipped_parity() {
    assert!(!should_hand_over_prepared_prompt(false, true, true));
    assert!(!should_hand_over_prepared_prompt(true, false, true));
    assert!(!should_hand_over_prepared_prompt(true, true, false));
}
