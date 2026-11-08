//! Slice 104 parity: server command dispatch gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/commands/opencode-start-command.ts` (the healthy-probe
//! already-running gate, the `refuse` / `answers` / `connected` / spawn
//! resolution dispatch, the spawned-process pid gate, the ready-wait poll
//! bound and not-ready gate, the lifecycle reason strings),
//! `src/bot/commands/opencode-stop-command.ts` (`STOP_REASON`, the kill
//! budget, the remote-target / not-running / kill-failed gates), and
//! `src/bot/commands/reload-command.ts` (`formatReloadResult` key
//! selection, the model-views refresh gate, the late-catalog watch gate).
//! Resolution kind mapping, wait budgets, local-target resolution, and
//! health-cause classification already live in `handover_restart_gates` /
//! `local_process_gates` / `server_target_gates` / `callback_health_gates`
//! and are NOT re-tested here.

use otb_core::server_command_gates::*;

// ---- start: already-running probe ----

#[test]
fn start_healthy_probe_reports_already_running() {
    assert!(should_report_start_already_running(true));
}

#[test]
fn start_unhealthy_probe_continues_to_resolution() {
    assert!(!should_report_start_already_running(false));
}

// ---- start: resolution dispatch ----

#[test]
fn start_refuse_kind_refuses() {
    assert_eq!(
        classify_start_resolution("refuse"),
        StartResolutionAction::Refuse
    );
}

#[test]
fn start_answers_kind_reports_already_running() {
    assert_eq!(
        classify_start_resolution("answers"),
        StartResolutionAction::AlreadyRunning
    );
}

#[test]
fn start_connected_kind_connects() {
    assert_eq!(
        classify_start_resolution("connected"),
        StartResolutionAction::Connected
    );
}

#[test]
fn start_unknown_kind_spawns() {
    assert_eq!(
        classify_start_resolution("spawn"),
        StartResolutionAction::Spawn
    );
    assert_eq!(classify_start_resolution(""), StartResolutionAction::Spawn);
}

// ---- start: spawn + ready gates ----

#[test]
fn start_missing_pid_reports_spawn_error() {
    assert!(should_report_start_spawn_error(false));
    assert!(!should_report_start_spawn_error(true));
}

#[test]
fn start_ready_wait_bound_parity() {
    assert!(should_keep_waiting_for_ready(0, 10_000));
    assert!(should_keep_waiting_for_ready(9_999, 10_000));
    assert!(!should_keep_waiting_for_ready(10_000, 10_000));
    assert!(!should_keep_waiting_for_ready(10_001, 10_000));
}

#[test]
fn start_unready_server_reports_not_ready() {
    assert!(should_report_start_not_ready(false));
    assert!(!should_report_start_not_ready(true));
}

// ---- start: lifecycle reason strings ----

#[test]
fn start_notify_reason_values_parity() {
    assert_eq!(START_NOTIFY_ALREADY_RUNNING, "opencode_start_already_running");
    assert_eq!(START_NOTIFY_NOT_RUNNING, "opencode_start_not_running");
    assert_eq!(START_NOTIFY_CONNECTED, "opencode_start_connected");
    assert_eq!(START_NOTIFY_SUCCESS, "opencode_start_success");
}

// ---- stop ----

#[test]
fn stop_reason_and_kill_budget_parity() {
    assert_eq!(STOP_REASON, "opencode_stop");
    assert_eq!(STOP_KILL_TIMEOUT_MS, 5_000);
}

#[test]
fn stop_remote_target_reports_remote_configured() {
    assert!(should_report_stop_remote(false));
    assert!(!should_report_stop_remote(true));
}

#[test]
fn stop_missing_pid_reports_not_running() {
    assert!(should_report_stop_not_running(false));
    assert!(!should_report_stop_not_running(true));
}

#[test]
fn stop_failed_kill_reports_stop_error() {
    assert!(should_report_stop_error(false));
    assert!(!should_report_stop_error(true));
}

// ---- reload ----

#[test]
fn reload_success_key_parity() {
    assert_eq!(
        reload_result_message_key(ReloadResultKind::Success, false),
        "reload.success"
    );
    assert_eq!(
        reload_result_message_key(ReloadResultKind::Success, true),
        "reload.success"
    );
}

#[test]
fn reload_failed_with_error_key_parity() {
    assert_eq!(
        reload_result_message_key(ReloadResultKind::Failed, true),
        "reload.failed_with_error"
    );
}

#[test]
fn reload_failed_without_error_falls_back() {
    assert_eq!(
        reload_result_message_key(ReloadResultKind::Failed, false),
        "reload.failed"
    );
}

#[test]
fn reload_timeout_falls_back_even_with_error() {
    assert_eq!(
        reload_result_message_key(ReloadResultKind::Timeout, true),
        "reload.failed"
    );
    assert_eq!(
        reload_result_message_key(ReloadResultKind::Timeout, false),
        "reload.failed"
    );
}

#[test]
fn reload_model_views_refresh_needs_success_and_change() {
    assert!(should_refresh_model_views_after_reload(
        ReloadResultKind::Success,
        true
    ));
    assert!(!should_refresh_model_views_after_reload(
        ReloadResultKind::Success,
        false
    ));
    assert!(!should_refresh_model_views_after_reload(
        ReloadResultKind::Failed,
        true
    ));
    assert!(!should_refresh_model_views_after_reload(
        ReloadResultKind::Timeout,
        true
    ));
}

#[test]
fn reload_late_catalog_watch_needs_success() {
    assert!(should_watch_late_catalog_after_reload(
        ReloadResultKind::Success
    ));
    assert!(!should_watch_late_catalog_after_reload(
        ReloadResultKind::Failed
    ));
    assert!(!should_watch_late_catalog_after_reload(
        ReloadResultKind::Timeout
    ));
}
