//! Slice 171 parity: reload-command gates.
//!
//! Mirrors the pure TS contract in
//! `tests/bot/commands/reload.test.ts` (the reload flow over
//! `src/bot/commands/reload-command.ts`):
//! - the reloading key (`reload.reloading`, the initial `ctx.reply`)
//! - the result key (`reload.success`, `reload.failed_with_error` when a
//!   failure carries an error, the bare `reload.failed` for a bare failure,
//!   a timeout, or a thrown reload)
//! - the immediate model-view refresh gate (only success with `modelChanged`)
//! - the late-catalog settle gate (every success, never on failure/throw)
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `config-reload-service` timeout/join/in-flight atoms (service owner)
//! - `refreshModelViews` / `refreshModelViewsAfterLateCatalogSettle`
//!   delivery, `ctx.reply` / `editBotText` / `safeBackgroundTask` wiring,
//!   per-message `message_id` plumbing (two reloads edit their own status
//!   message), `t()` lookups, `logger.error` emission (callers)
//! - locale strings for the `reload.*` keys (i18n owners)

use otb_core::reload_command_gates::*;

// ---- key values (4) ----

#[test]
fn reload_reloading_key_value_parity() {
    assert_eq!(RELOAD_RELOADING_KEY, "reload.reloading");
}

#[test]
fn reload_success_key_value_parity() {
    assert_eq!(RELOAD_SUCCESS_KEY, "reload.success");
}

#[test]
fn reload_failed_key_value_parity() {
    assert_eq!(RELOAD_FAILED_KEY, "reload.failed");
}

#[test]
fn reload_failed_with_error_key_value_parity() {
    assert_eq!(RELOAD_FAILED_WITH_ERROR_KEY, "reload.failed_with_error");
}

// ---- result key (5) ----

#[test]
fn reload_success_maps_to_success_key_parity() {
    // TS: `formatReloadResult({ kind: "success", ... })` is `reload.success`
    // whatever `modelChanged` is.
    assert_eq!(
        reload_result_key(ReloadCompletion::Success {
            model_changed: false
        }),
        "reload.success"
    );
    assert_eq!(
        reload_result_key(ReloadCompletion::Success {
            model_changed: true
        }),
        "reload.success"
    );
}

#[test]
fn reload_failed_with_error_maps_to_failed_with_error_key_parity() {
    // TS: `failed` with an error shows `failed_with_error` (test lines
    // 75-86).
    assert_eq!(
        reload_result_key(ReloadCompletion::Failed {
            error: Some("Invalid config")
        }),
        "reload.failed_with_error"
    );
}

#[test]
fn reload_bare_failure_maps_to_failed_key_parity() {
    // TS: `failed` with a null error shows the bare failure line (test
    // lines 88-93).
    assert_eq!(
        reload_result_key(ReloadCompletion::Failed { error: None }),
        "reload.failed"
    );
    assert_eq!(
        reload_result_key(ReloadCompletion::Failed { error: Some("") }),
        "reload.failed"
    );
}

#[test]
fn reload_timeout_maps_to_failed_key_parity() {
    // TS: `{ kind: "timeout" }` falls through to the bare failure line
    // (test lines 96-101).
    assert_eq!(
        reload_result_key(ReloadCompletion::Timeout),
        "reload.failed"
    );
}

#[test]
fn reload_thrown_maps_to_failed_key_parity() {
    // TS: a rejected `reloadOpencodeConfig` is caught into the bare failure
    // line (test lines 103-111).
    assert_eq!(reload_result_key(ReloadCompletion::Thrown), "reload.failed");
}

// ---- immediate refresh gate (3) ----

#[test]
fn reload_success_without_model_change_skips_refresh_parity() {
    // TS: success with `modelChanged: false` never calls `refreshModelViews`
    // (test lines 47-62).
    assert!(!should_refresh_model_views(ReloadCompletion::Success {
        model_changed: false
    }));
}

#[test]
fn reload_success_with_model_change_refreshes_parity() {
    // TS: success with `modelChanged: true` redraws the model views (test
    // lines 64-73).
    assert!(should_refresh_model_views(ReloadCompletion::Success {
        model_changed: true
    }));
}

#[test]
fn reload_failure_and_timeout_never_refresh_parity() {
    // TS: failures and timeouts never reach the refresh branch.
    assert!(!should_refresh_model_views(ReloadCompletion::Failed {
        error: Some("Invalid config")
    }));
    assert!(!should_refresh_model_views(ReloadCompletion::Failed {
        error: None
    }));
    assert!(!should_refresh_model_views(ReloadCompletion::Timeout));
    assert!(!should_refresh_model_views(ReloadCompletion::Thrown));
}

// ---- late-catalog settle gate (2) ----

#[test]
fn reload_success_schedules_late_catalog_settle_parity() {
    // TS: every success schedules `refreshModelViewsAfterLateCatalogSettle`,
    // with or without a model change (test line 61 covers the no-change
    // case).
    assert!(should_schedule_late_catalog_settle(
        ReloadCompletion::Success {
            model_changed: false
        }
    ));
    assert!(should_schedule_late_catalog_settle(
        ReloadCompletion::Success {
            model_changed: true
        }
    ));
}

#[test]
fn reload_failure_and_throw_never_schedule_late_settle_parity() {
    // TS: the settle call lives inside the success branch, so failures,
    // timeouts, and throws never schedule it (test line 85 covers failure;
    // the thrown path never reaches the branch).
    assert!(!should_schedule_late_catalog_settle(
        ReloadCompletion::Failed {
            error: Some("Invalid config")
        }
    ));
    assert!(!should_schedule_late_catalog_settle(
        ReloadCompletion::Failed { error: None }
    ));
    assert!(!should_schedule_late_catalog_settle(
        ReloadCompletion::Timeout
    ));
    assert!(!should_schedule_late_catalog_settle(
        ReloadCompletion::Thrown
    ));
}
