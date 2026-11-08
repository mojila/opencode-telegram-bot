//! Slice 105 parity: browse + catalog command dispatch gates (RED).
//!
//! Mirrors the pure TS contract in `src/bot/commands/ls-command.ts`
//! (busy/container/project gates, `ctx.match` trim + `args || remembered`
//! target resolution, no-target and access-denied gates, browse-error and
//! no-actions plain-send gates), `src/bot/commands/open-command.ts`
//! (busy/container gates, `roots.length === 1` single-root gate,
//! browse-error gate),
//! `src/bot/commands/command-catalog-command.ts` /
//! `skills-catalog-command.ts` / `mcp-catalog-command.ts`
//! (no-project and empty-list gates),
//! `src/bot/commands/sessions-command.ts` / `projects-command.ts` /
//! `recent-command.ts` (no-project where present, empty-list gates),
//! `src/bot/commands/messages-command.ts` (no-project, no-session,
//! session/project mismatch, empty-list gates), and
//! `src/bot/commands/models.ts` (fetch-error, no-providers, and
//! per-provider no-models gates).
//! Busy/container/project-root math themselves already live in
//! `attach_foreground_gates` / `stores_runtime` /
//! `file_browser_service_gates` and are NOT re-tested here (this slice
//! takes their outcomes as bools and counts).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `browse_command_gates` is ported.

use otb_core::browse_command_gates::*;

// ---- busy / container / project gates (3) ----

#[test]
fn blocks_command_when_busy_parity() {
    assert!(should_block_command_when_busy(true));
}

#[test]
fn continues_command_when_idle_parity() {
    assert!(!should_block_command_when_busy(false));
}

#[test]
fn reports_container_unavailable_parity() {
    assert!(should_report_container_unavailable(true));
    assert!(!should_report_container_unavailable(false));
}

#[test]
fn reports_no_project_parity() {
    assert!(should_report_no_project(false));
    assert!(!should_report_no_project(true));
}

// ---- empty-list gate (2) ----

#[test]
fn reports_empty_list_parity() {
    assert!(should_report_empty_list(0));
}

#[test]
fn keeps_non_empty_list_parity() {
    assert!(!should_report_empty_list(1));
    assert!(!should_report_empty_list(8));
}

// ---- messages session gates (4) ----

#[test]
fn reports_no_session_parity() {
    assert!(should_report_no_session(false));
    assert!(!should_report_no_session(true));
}

#[test]
fn reports_session_project_mismatch_parity() {
    assert!(should_report_session_project_mismatch("/other", "/project"));
}

#[test]
fn keeps_matching_session_project_parity() {
    assert!(!should_report_session_project_mismatch(
        "/project",
        "/project"
    ));
}

// ---- /ls target resolution (5) ----

#[test]
fn prefers_explicit_ls_arg_parity() {
    assert_eq!(
        resolve_ls_target(Some("/project/sub"), Some("/project")),
        Some("/project/sub")
    );
}

#[test]
fn falls_back_to_remembered_ls_dir_parity() {
    assert_eq!(resolve_ls_target(None, Some("/project")), Some("/project"));
}

#[test]
fn empty_ls_arg_falls_back_parity() {
    assert_eq!(resolve_ls_target(Some(""), Some("/project")), Some("/project"));
    assert_eq!(
        resolve_ls_target(Some("   "), Some("/project")),
        Some("/project")
    );
    assert_eq!(resolve_ls_target(Some("   "), None), None);
}

#[test]
fn reports_no_ls_target_parity() {
    assert!(should_report_no_ls_target(false));
    assert!(!should_report_no_ls_target(true));
}

#[test]
fn reports_ls_access_denied_outside_root_parity() {
    assert!(should_report_ls_access_denied(false));
    assert!(!should_report_ls_access_denied(true));
}

// ---- /ls + /open view gates (4) ----

#[test]
fn sends_ls_without_keyboard_when_no_actions_parity() {
    assert!(should_send_ls_without_keyboard(false));
    assert!(!should_send_ls_without_keyboard(true));
}

#[test]
fn shows_single_open_root_parity() {
    assert!(should_show_single_open_root(1));
    assert!(!should_show_single_open_root(0));
    assert!(!should_show_single_open_root(2));
}

#[test]
fn reports_browse_scan_error_parity() {
    assert!(should_report_browse_scan_error(true));
    assert!(!should_report_browse_scan_error(false));
}

// ---- legacy /models gates (4) ----

#[test]
fn reports_models_fetch_error_parity() {
    assert!(should_report_models_fetch_error(true, true));
    assert!(should_report_models_fetch_error(false, false));
    assert!(should_report_models_fetch_error(true, false));
}

#[test]
fn keeps_models_on_data_parity() {
    assert!(!should_report_models_fetch_error(false, true));
}

#[test]
fn reports_no_providers_parity() {
    assert!(should_report_no_providers(0));
    assert!(!should_report_no_providers(1));
}

#[test]
fn shows_no_models_line_for_empty_provider_parity() {
    assert!(should_show_no_models_for_provider(0));
    assert!(!should_show_no_models_for_provider(2));
}
