//! Slice 68 parity: attach/foreground gates (RED).
//!
//! Mirrors the pure TS contract in:
//! - `src/app/managers/attach-manager.ts` (`AttachManager`)
//! - `src/app/managers/foreground-session-state-manager.ts`
//!   (`ForegroundSessionState`)
//! - `src/app/managers/prompt-attachment-manager.ts`
//!   (`PromptAttachmentManager`)
//! - `src/app/services/attach-service.ts` (`getAttachBusyStatus`)
//! - `src/app/services/run-control-service.ts` (`isForegroundBusy`,
//!   `getBusyDirectories`)
//! - `src/app/services/prompt-attachment-service.ts` (`toRelativePath`)
//! - `src/app/services/busy-reconciliation-service.ts`
//!   (`isTerminalStatus`, foreground grace window)
//!
//! Every test MUST FAIL until `attach_foreground_gates` is ported
//! (step 2).

use otb_core::attach_foreground_gates::*;
use std::collections::HashMap;

fn busy_statuses(session_id: &str, status_type: Option<&str>) -> HashMap<String, Option<String>> {
    let mut map = HashMap::new();
    map.insert(session_id.to_string(), status_type.map(str::to_string));
    map
}

fn foreground_session(id: &str, dir: &str, marked_at: u64) -> ForegroundBusySession {
    ForegroundBusySession {
        session_id: id.to_string(),
        directory: dir.to_string(),
        marked_at,
    }
}

// --- AttachManager ---

#[test]
fn slice68_attach_snapshot_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert_eq!(
        manager.snapshot(),
        Some(AttachedSessionState {
            session_id: "sess-1".to_string(),
            directory: "/work/a".to_string(),
            busy: false,
        })
    );
}

#[test]
fn slice68_attach_overwrites_previous_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    manager.attach("sess-2", "/work/b");
    let snapshot = manager.snapshot().expect("attached");
    assert_eq!(snapshot.session_id, "sess-2");
    assert_eq!(snapshot.directory, "/work/b");
}

#[test]
fn slice68_attach_snapshot_is_copy_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    let mut snapshot = manager.snapshot().expect("attached");
    snapshot.session_id = "mutated".to_string();
    assert_eq!(manager.snapshot().expect("again").session_id, "sess-1");
}

#[test]
fn slice68_attach_not_attached_initially_parity() {
    let manager = AttachManager::new();
    assert!(!manager.is_attached());
    assert_eq!(manager.snapshot(), None);
}

#[test]
fn slice68_attach_is_attached_after_attach_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(manager.is_attached());
}

#[test]
fn slice68_attach_clear_detaches_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    manager.clear("detach");
    assert!(!manager.is_attached());
    assert_eq!(manager.snapshot(), None);
}

#[test]
fn slice68_attach_clear_empty_noop_parity() {
    let mut manager = AttachManager::new();
    manager.clear("detach");
    assert!(!manager.is_attached());
}

#[test]
fn slice68_attach_is_attached_session_hit_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(manager.is_attached_session(Some("sess-1"), None));
    assert!(manager.is_attached_session(Some("sess-1"), Some("/work/a")));
}

#[test]
fn slice68_attach_is_attached_session_wrong_session_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(!manager.is_attached_session(Some("sess-2"), None));
}

#[test]
fn slice68_attach_is_attached_session_wrong_directory_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(!manager.is_attached_session(Some("sess-1"), Some("/work/b")));
}

#[test]
fn slice68_attach_is_attached_session_empty_id_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(!manager.is_attached_session(None, None));
    assert!(!manager.is_attached_session(Some(""), None));
}

#[test]
fn slice68_attach_is_attached_session_detached_parity() {
    let manager = AttachManager::new();
    assert!(!manager.is_attached_session(Some("sess-1"), None));
}

#[test]
fn slice68_attach_busy_cycle_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(!manager.is_busy());
    assert!(manager.mark_busy("sess-1"));
    assert!(manager.is_busy());
    assert!(manager.mark_idle("sess-1"));
    assert!(!manager.is_busy());
}

#[test]
fn slice68_attach_mark_busy_twice_false_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(manager.mark_busy("sess-1"));
    assert!(!manager.mark_busy("sess-1"));
}

#[test]
fn slice68_attach_mark_busy_wrong_session_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(!manager.mark_busy("sess-2"));
    assert!(!manager.is_busy());
}

#[test]
fn slice68_attach_mark_idle_when_idle_false_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    assert!(!manager.mark_idle("sess-1"));
}

#[test]
fn slice68_attach_mark_idle_wrong_session_parity() {
    let mut manager = AttachManager::new();
    manager.attach("sess-1", "/work/a");
    manager.mark_busy("sess-1");
    assert!(!manager.mark_idle("sess-2"));
    assert!(manager.is_busy());
}

#[test]
fn slice68_attach_mark_detached_false_parity() {
    let mut manager = AttachManager::new();
    assert!(!manager.mark_busy("sess-1"));
    assert!(!manager.mark_idle("sess-1"));
}

// --- ForegroundSessionState ---

#[test]
fn slice68_foreground_mark_busy_visible_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("sess-1", "/work/a", 1000);
    assert!(state.is_busy());
    assert_eq!(
        state.busy_sessions(),
        vec![foreground_session("sess-1", "/work/a", 1000)]
    );
}

#[test]
fn slice68_foreground_empty_initially_parity() {
    let state = ForegroundSessionState::new();
    assert!(!state.is_busy());
    assert_eq!(state.busy_sessions(), vec![]);
}

#[test]
fn slice68_foreground_mark_busy_ignores_empty_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("", "/work/a", 1000);
    state.mark_busy("sess-1", "", 1000);
    assert!(!state.is_busy());
}

#[test]
fn slice68_foreground_mark_busy_refreshes_entry_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("sess-1", "/work/a", 1000);
    state.mark_busy("sess-1", "/work/a", 2000);
    assert_eq!(state.busy_sessions().len(), 1);
    assert_eq!(state.busy_sessions()[0].marked_at, 2000);
}

#[test]
fn slice68_foreground_mark_idle_removes_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("sess-1", "/work/a", 1000);
    state.mark_idle("sess-1");
    assert!(!state.is_busy());
}

#[test]
fn slice68_foreground_mark_idle_empty_noop_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("sess-1", "/work/a", 1000);
    state.mark_idle("");
    assert!(state.is_busy());
}

#[test]
fn slice68_foreground_sessions_are_copies_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("sess-1", "/work/a", 1000);
    let mut listed = state.busy_sessions();
    listed.push(foreground_session("sess-2", "/work/a", 1000));
    assert_eq!(state.busy_sessions().len(), 1);
}

#[test]
fn slice68_foreground_clear_all_empties_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("sess-1", "/work/a", 1000);
    state.mark_busy("sess-2", "/work/b", 1000);
    state.clear_all("detach");
    assert!(!state.is_busy());
    assert_eq!(state.busy_sessions(), vec![]);
}

#[test]
fn slice68_foreground_clear_all_empty_noop_parity() {
    let mut state = ForegroundSessionState::new();
    state.clear_all("detach");
    assert!(!state.is_busy());
}

#[test]
fn slice68_foreground_set_marked_at_parity() {
    let mut state = ForegroundSessionState::new();
    state.mark_busy("sess-1", "/work/a", 1000);
    state.set_marked_at_for_tests("sess-1", 5000);
    assert_eq!(state.busy_sessions()[0].marked_at, 5000);
}

#[test]
fn slice68_foreground_set_marked_at_unknown_noop_parity() {
    let mut state = ForegroundSessionState::new();
    state.set_marked_at_for_tests("missing", 5000);
    assert!(!state.is_busy());
}

// --- PromptAttachmentManager ---

#[test]
fn slice68_prompt_attachment_set_get_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.set("/work/a/file.txt", "/work/a");
    assert_eq!(
        holder.get(),
        Some(PendingAttachment {
            absolute_path: "/work/a/file.txt".to_string(),
            worktree: "/work/a".to_string(),
            confirmation_message_id: None,
        })
    );
}

#[test]
fn slice68_prompt_attachment_empty_initially_parity() {
    let holder = PromptAttachmentManager::new();
    assert_eq!(holder.get(), None);
}

#[test]
fn slice68_prompt_attachment_set_overwrites_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.set("/work/a/old.txt", "/work/a");
    holder.set("/work/a/new.txt", "/work/a");
    assert_eq!(
        holder.get().expect("pending").absolute_path,
        "/work/a/new.txt"
    );
}

#[test]
fn slice68_prompt_attachment_get_is_copy_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.set("/work/a/file.txt", "/work/a");
    let mut pending = holder.get().expect("pending");
    pending.absolute_path = "mutated".to_string();
    assert_eq!(
        holder.get().expect("again").absolute_path,
        "/work/a/file.txt"
    );
}

#[test]
fn slice68_prompt_attachment_confirmation_id_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.set("/work/a/file.txt", "/work/a");
    holder.set_confirmation_message_id(42);
    assert_eq!(
        holder.get().expect("pending").confirmation_message_id,
        Some(42)
    );
}

#[test]
fn slice68_prompt_attachment_confirmation_id_empty_noop_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.set_confirmation_message_id(42);
    assert_eq!(holder.get(), None);
}

#[test]
fn slice68_prompt_attachment_clear_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.set("/work/a/file.txt", "/work/a");
    holder.clear("consumed");
    assert_eq!(holder.get(), None);
}

#[test]
fn slice68_prompt_attachment_clear_empty_noop_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.clear("consumed");
    assert_eq!(holder.get(), None);
}

#[test]
fn slice68_prompt_attachment_reset_parity() {
    let mut holder = PromptAttachmentManager::new();
    holder.set("/work/a/file.txt", "/work/a");
    holder.set_confirmation_message_id(7);
    holder.reset_for_tests();
    assert_eq!(holder.get(), None);
}

// --- getAttachBusyStatus ---

#[test]
fn slice68_get_attach_busy_status_busy_parity() {
    assert!(get_attach_busy_status(
        Some(&busy_statuses("sess-1", Some("busy"))),
        "sess-1"
    ));
}

#[test]
fn slice68_get_attach_busy_status_idle_parity() {
    assert!(!get_attach_busy_status(
        Some(&busy_statuses("sess-1", Some("idle"))),
        "sess-1"
    ));
}

#[test]
fn slice68_get_attach_busy_status_missing_parity() {
    assert!(!get_attach_busy_status(
        Some(&busy_statuses("other", Some("busy"))),
        "sess-1"
    ));
    assert!(!get_attach_busy_status(None, "sess-1"));
}

// --- isForegroundBusy / getBusyDirectories ---

#[test]
fn slice68_is_foreground_busy_truth_table_parity() {
    assert!(!is_foreground_busy(false, false));
    assert!(is_foreground_busy(true, false));
    assert!(is_foreground_busy(false, true));
    assert!(is_foreground_busy(true, true));
}

#[test]
fn slice68_get_busy_directories_combined_parity() {
    let foreground = vec![
        foreground_session("sess-1", "/work/a", 1000),
        foreground_session("sess-2", "/work/b", 1000),
    ];
    let attached = AttachedSessionState {
        session_id: "sess-3".to_string(),
        directory: "/work/c".to_string(),
        busy: true,
    };
    assert_eq!(
        get_busy_directories(&foreground, Some(&attached)),
        vec![
            "/work/a".to_string(),
            "/work/b".to_string(),
            "/work/c".to_string()
        ]
    );
}

#[test]
fn slice68_get_busy_directories_dedupes_parity() {
    let foreground = vec![foreground_session("sess-1", "/work/a", 1000)];
    let attached = AttachedSessionState {
        session_id: "sess-1".to_string(),
        directory: "/work/a".to_string(),
        busy: true,
    };
    assert_eq!(
        get_busy_directories(&foreground, Some(&attached)),
        vec!["/work/a".to_string()]
    );
}

#[test]
fn slice68_get_busy_directories_idle_attached_skipped_parity() {
    let foreground = vec![foreground_session("sess-1", "/work/a", 1000)];
    let attached = AttachedSessionState {
        session_id: "sess-9".to_string(),
        directory: "/work/z".to_string(),
        busy: false,
    };
    assert_eq!(
        get_busy_directories(&foreground, Some(&attached)),
        vec!["/work/a".to_string()]
    );
}

#[test]
fn slice68_get_busy_directories_empty_parity() {
    assert_eq!(get_busy_directories(&[], None), Vec::<String>::new());
}

// --- toRelativePath / isTerminalStatus / grace ---

#[test]
fn slice68_to_relative_path_inside_parity() {
    assert_eq!(
        to_relative_path("/work/a/sub/file.txt", "/work/a"),
        "sub/file.txt"
    );
}

#[test]
fn slice68_to_relative_path_escaping_parity() {
    assert_eq!(
        to_relative_path("/work/other/file.txt", "/work/a"),
        "/work/other/file.txt"
    );
}

#[test]
fn slice68_is_terminal_status_parity() {
    assert!(is_terminal_status(None));
    assert!(is_terminal_status(Some("idle")));
    assert!(is_terminal_status(Some("error")));
    assert!(!is_terminal_status(Some("busy")));
}

#[test]
fn slice68_foreground_grace_window_parity() {
    assert_eq!(FOREGROUND_BUSY_RECONCILE_GRACE_MS, 2_000);
    assert_eq!(RECONCILE_MIN_INTERVAL_MS, 10_000);
    assert!(is_within_foreground_busy_grace_period(1000, 1000 + 1999));
    assert!(!is_within_foreground_busy_grace_period(1000, 1000 + 2000));
}
