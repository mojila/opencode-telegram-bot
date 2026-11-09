//! Slice 174 parity: start/stop/folder-gone lifecycle gates (GREEN).
//!
//! Mirrors the pure TS contract in
//! - `src/opencode/process.ts` (`createOpencodeServeSpawnCommand` serve
//!   args, `STAND_SERVER_PID_ENV` record/clear match, `PROCESS_EXIT_POLL_MS`
//!   wait bound, SIGTERM-then-SIGKILL escalation),
//! - `src/opencode/local-start.ts` (registered-server lookup skips for
//!   non-V2/standalone and same-port servers, the executable-version block),
//! - `src/opencode/ready-lifecycle.ts` (`notifyReady` first-fire and
//!   `notifyUnavailable` ready-to-down transition guards),
//! - `src/app/services/folder-presence-service.ts` (global-folder present
//!   gate, V2-only probe gate, `listFolder` answer classification, path
//!   style/name/basename/parent math, the ancestor-walk verdict, the
//!   present-collector refill gate, the ask-once dedupe),
//! - `src/i18n/en.ts` + `src/i18n/ru.ts`
//!   (`bot.project_folder_missing_worktree` verbatim templates),
//! - `src/opencode/v2/client.ts:487-492` (`file.list` route + entry shaping,
//!   covered in `otb-bot/src/opencode.rs` unit tests).
//!
//! Probing, switching, spawning, killing, pid-file I/O, clocks, and the
//! walk loop itself stay with the caller; outcomes arrive as explicit
//! snapshots. Stop dispatch (`STOP_REASON`, kill budget, remote /
//! not-running / error gates) already lives in `server_command_gates`
//! (slice 104) and is NOT re-tested here.

use otb_core::i18n_full::error_project_folder_missing_worktree;
use otb_core::local_process_gates::*;
use otb_core::server_command_gates::{
    should_fire_ready_notification, should_fire_unavailable_notification,
};
use otb_core::server_target_gates::{
    is_blocked_by_executable_version, is_registered_on_target_port, should_skip_registered_lookup,
};
use otb_core::session_context_gates::*;

// ---- start: serve spawn command ----

#[test]
fn serve_args_use_service_flag_only_for_v2_service_mode() {
    assert_eq!(
        serve_command_args("v2", "service", 4096),
        vec!["serve", "--service", "--port", "4096"]
    );
    assert_eq!(
        serve_command_args("v2", "standalone", 4097),
        vec!["serve", "--port", "4097"]
    );
    assert_eq!(
        serve_command_args("v1", "service", 4096),
        vec!["serve", "--port", "4096"]
    );
}

// ---- start: stand pid record/clear ----

#[test]
fn stand_pid_file_trims_and_treats_blank_as_unset() {
    assert_eq!(
        resolve_stand_pid_file(Some("  /tmp/stand.pid\n")),
        Some("/tmp/stand.pid")
    );
    assert_eq!(resolve_stand_pid_file(Some("   ")), None);
    assert_eq!(resolve_stand_pid_file(None), None);
}

#[test]
fn stand_pid_records_only_with_file_and_pid() {
    assert!(should_record_stand_server_pid(Some("/tmp/stand.pid"), true));
    assert!(!should_record_stand_server_pid(None, true));
    assert!(!should_record_stand_server_pid(
        Some("/tmp/stand.pid"),
        false
    ));
}

#[test]
fn stand_pid_clears_only_when_record_names_stopped_pid() {
    assert!(should_clear_stand_server_pid(
        Some("/tmp/stand.pid"),
        Some("4242"),
        4242
    ));
    assert!(!should_clear_stand_server_pid(
        Some("/tmp/stand.pid"),
        Some("4243"),
        4242
    ));
    assert!(!should_clear_stand_server_pid(None, Some("4242"), 4242));
    assert!(!should_clear_stand_server_pid(
        Some("/tmp/stand.pid"),
        None,
        4242
    ));
}

// ---- start/stop: exit wait + kill escalation ----

#[test]
fn exit_wait_polls_while_elapsed_is_below_timeout() {
    assert!(should_keep_waiting_for_exit(99, 5000));
    assert!(!should_keep_waiting_for_exit(5000, 5000));
    assert!(!should_keep_waiting_for_exit(6000, 5000));
}

#[test]
fn exit_poll_cadence_is_100ms() {
    assert_eq!(PROCESS_EXIT_POLL_MS, 100);
}

#[test]
fn kill_escalates_only_while_process_is_alive() {
    assert!(should_escalate_kill(true));
    assert!(!should_escalate_kill(false));
}

// ---- start: registered-server lookup skips ----

#[test]
fn registered_lookup_skipped_for_non_v2_or_standalone() {
    assert!(should_skip_registered_lookup("v1", "service"));
    assert!(should_skip_registered_lookup("v2", "standalone"));
    assert!(!should_skip_registered_lookup("v2", "service"));
}

#[test]
fn registered_server_on_target_port_is_not_a_switch_candidate() {
    assert!(is_registered_on_target_port(Some(4096), 4096));
    assert!(!is_registered_on_target_port(Some(4097), 4096));
    assert!(!is_registered_on_target_port(None, 4096));
}

// ---- start: executable-version block ----

#[test]
fn executable_version_blocks_only_on_other_api_version() {
    assert!(!is_blocked_by_executable_version(None, "v2"));
    assert!(!is_blocked_by_executable_version(Some("v2"), "v2"));
    assert!(is_blocked_by_executable_version(Some("v1"), "v2"));
    assert!(is_blocked_by_executable_version(Some("v2"), "v1"));
}

// ---- start/stop: ready-lifecycle transitions ----

#[test]
fn ready_notification_fires_only_once() {
    assert!(should_fire_ready_notification(false));
    assert!(!should_fire_ready_notification(true));
}

#[test]
fn unavailable_notification_fires_only_on_ready_to_down() {
    assert!(should_fire_unavailable_notification(true));
    assert!(!should_fire_unavailable_notification(false));
}

// ---- folder-gone: scope gates ----

#[test]
fn global_project_folder_is_always_present() {
    assert_eq!(GLOBAL_PROJECT_FOLDER, "/");
    assert!(is_global_project_folder("/"));
    assert!(!is_global_project_folder("/work"));
}

#[test]
fn presence_probe_is_v2_only() {
    assert!(is_presence_checkable("v2"));
    assert!(!is_presence_checkable("v1"));
}

// ---- folder-gone: listing classification ----

#[test]
fn listing_result_maps_entries_ok_and_500() {
    assert_eq!(classify_listing_result(true, None), FolderListing::Listed);
    assert_eq!(
        classify_listing_result(false, Some(500)),
        FolderListing::ServerError
    );
    assert_eq!(
        classify_listing_result(false, Some(200)),
        FolderListing::OtherFailure
    );
    assert_eq!(
        classify_listing_result(false, None),
        FolderListing::OtherFailure
    );
}

// ---- folder-gone: walk verdict ----

#[test]
fn listed_folder_is_present_without_walking() {
    assert_eq!(
        classify_folder_presence(FolderListing::Listed, &[]),
        "present"
    );
    assert_eq!(
        classify_folder_presence(FolderListing::Listed, &[AncestorListing::WithoutChild]),
        "present"
    );
}

#[test]
fn failed_listing_without_500_is_unknown() {
    assert_eq!(
        classify_folder_presence(FolderListing::OtherFailure, &[]),
        "unknown"
    );
}

#[test]
fn ancestor_walk_reports_missing_only_on_listed_absence() {
    use AncestorListing::*;
    assert_eq!(
        classify_folder_presence(FolderListing::ServerError, &[]),
        "unknown"
    );
    assert_eq!(
        classify_folder_presence(FolderListing::ServerError, &[WithoutChild]),
        "missing"
    );
    assert_eq!(
        classify_folder_presence(FolderListing::ServerError, &[ContainsChild]),
        "unknown"
    );
    assert_eq!(
        classify_folder_presence(FolderListing::ServerError, &[OtherFailure]),
        "unknown"
    );
    assert_eq!(
        classify_folder_presence(FolderListing::ServerError, &[ServerError, WithoutChild]),
        "missing"
    );
    assert_eq!(
        classify_folder_presence(FolderListing::ServerError, &[ServerError, ContainsChild]),
        "unknown"
    );
    assert_eq!(
        classify_folder_presence(FolderListing::ServerError, &[WithoutChild, ContainsChild]),
        "missing"
    );
}

// ---- folder-gone: path math ----

#[test]
fn windows_style_detection_matches_ts_path_style_of() {
    assert!(is_windows_style_path("C:\\work\\proj"));
    assert!(is_windows_style_path("C:/work/proj"));
    assert!(is_windows_style_path("\\\\server\\share\\proj"));
    assert!(!is_windows_style_path("/work/proj"));
    assert!(!is_windows_style_path("relative/proj"));
}

#[test]
fn trailing_separators_strip_both_styles() {
    assert_eq!(strip_trailing_separators("/work/proj///"), "/work/proj");
    assert_eq!(strip_trailing_separators("C:\\work\\\\"), "C:\\work");
    assert_eq!(strip_trailing_separators("proj"), "proj");
}

#[test]
fn entry_names_match_case_insensitively_on_windows() {
    assert!(folder_names_equal(false, "src", "src"));
    assert!(!folder_names_equal(false, "SRC", "src"));
    assert!(folder_names_equal(true, "SRC", "src"));
    assert!(!folder_names_equal(true, "src", "other"));
}

#[test]
fn basename_takes_last_segment_per_style() {
    assert_eq!(path_basename("/work/proj/src", false), "src");
    assert_eq!(path_basename("/work/proj/", false), "proj");
    assert_eq!(path_basename("C:\\work\\proj", true), "proj");
    assert_eq!(path_basename("C:/work/proj/", true), "proj");
    assert_eq!(path_basename("C:\\", true), "");
}

#[test]
fn parent_climbs_with_roots_mapping_to_themselves() {
    assert_eq!(path_parent("/", false), "/");
    assert_eq!(path_parent("/work/proj", false), "/work");
    assert_eq!(path_parent("/work", false), "/");
    assert_eq!(path_parent("proj", false), ".");
    assert_eq!(path_parent("C:\\work\\proj", true), "C:\\work");
    assert_eq!(path_parent("C:\\work", true), "C:\\");
    assert_eq!(path_parent("C:\\", true), "C:\\");
    assert_eq!(path_parent("C:/", true), "C:\\");
    assert_eq!(path_parent("\\\\server\\share", true), "\\\\server\\share");
    assert_eq!(
        path_parent("\\\\server\\share\\proj", true),
        "\\\\server\\share"
    );
}

#[test]
fn ancestor_match_strips_separators_and_takes_basenames() {
    assert!(ancestor_lists_child(
        false,
        &["/work/other", "/work/proj/"],
        "proj"
    ));
    assert!(!ancestor_lists_child(false, &["/work/other"], "proj"));
    assert!(ancestor_lists_child(true, &["C:\\Work\\PROJ"], "proj"));
}

// ---- folder-gone: refill + dedupe ----

#[test]
fn present_collector_refetches_only_while_short_and_source_grows() {
    assert!(should_refetch_present_items(1, 3, 3, 3));
    assert!(!should_refetch_present_items(3, 3, 3, 3));
    assert!(!should_refetch_present_items(1, 3, 2, 3));
}

#[test]
fn missing_folders_are_asked_about_once_in_order() {
    assert_eq!(
        dedupe_folders(&["/a", "/b", "/a", "/c", "/b"]),
        vec!["/a", "/b", "/c"]
    );
    assert!(dedupe_folders(&[]).is_empty());
}

// ---- folder-gone: worktree notice i18n ----

#[test]
fn worktree_notice_points_to_worktree_in_en_and_ru() {
    assert_eq!(
        error_project_folder_missing_worktree("en", "/work/gone"),
        "🚫 The project folder no longer exists: /work/gone. Choose another worktree in /worktree."
    );
    assert_eq!(
        error_project_folder_missing_worktree("ru", "/work/gone"),
        "🚫 Папка проекта больше не существует: /work/gone. Выберите другой worktree в /worktree."
    );
    assert_eq!(
        error_project_folder_missing_worktree("xx", "/work/gone"),
        error_project_folder_missing_worktree("en", "/work/gone")
    );
}
