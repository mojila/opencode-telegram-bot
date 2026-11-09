//! Slice 106 parity: session-lifecycle + utility command dispatch gates (RED).
//!
//! Mirrors the pure TS contract in `src/bot/commands/worktree-command.ts`
//! (`!context` not-a-git-repo gate), `src/bot/commands/new-command.ts`
//! (missing-folder-notice short-circuit, `SessionInfo` build with the
//! OpenCode title kept verbatim), `src/bot/commands/start-command.ts`
//! (`contextInfo` stored-wins / zero-usage-fallback / none shape),
//! `src/bot/commands/tasklist-command.ts` (`sortTasks` next-run ordering,
//! missing-runs-last, `createdAt` tiebreak, unparseable-dates-keep-order),
//! and `src/bot/commands/detach-command.ts` (pinned-context reset keeps the
//! limit while zeroing usage).
//! Busy / container / no-project / no-session / empty-list dispatch itself
//! already lives in `attach_foreground_gates` / `stores_runtime` /
//! `browse_command_gates` and is NOT re-tested here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `lifecycle_command_gates` is ported.

use otb_core::lifecycle_command_gates::*;
use std::cmp::Ordering;

// ---- /worktree git-context gate (2) ----

#[test]
fn reports_not_git_repo_without_context_parity() {
    assert!(should_report_not_git_repo(false));
}

#[test]
fn continues_with_worktree_context_parity() {
    assert!(!should_report_not_git_repo(true));
}

// ---- /new folder-notice gate (2) ----

#[test]
fn blocks_new_when_folder_notice_present_parity() {
    assert!(should_block_new_on_folder_notice(true));
}

#[test]
fn creates_new_without_folder_notice_parity() {
    assert!(!should_block_new_on_folder_notice(false));
}

// ---- /new SessionInfo build (2) ----

#[test]
fn new_session_info_points_at_project_worktree_parity() {
    assert_eq!(
        build_new_session_info("session-2", "Session Two", "/repo"),
        NewSessionInfo {
            id: "session-2".to_string(),
            title: "Session Two".to_string(),
            directory: "/repo".to_string(),
        }
    );
}

#[test]
fn new_session_info_keeps_empty_title_untouched_parity() {
    // The display layer (formatSessionTitle) names that a new session;
    // the stored info keeps the title empty.
    let info = build_new_session_info("session-3", "", "/repo");
    assert_eq!(info.title, "");
    assert_eq!(info.directory, "/repo");
}

// ---- /start context fallback (3) ----

#[test]
fn start_prefers_stored_context_info_parity() {
    let stored = StartContextInfo {
        tokens_used: 120,
        tokens_limit: 200_000,
    };
    assert_eq!(
        resolve_start_context_info(Some(stored), 200_000),
        Some(stored)
    );
}

#[test]
fn start_falls_back_to_zero_usage_when_limit_known_parity() {
    assert_eq!(
        resolve_start_context_info(None, 200_000),
        Some(StartContextInfo {
            tokens_used: 0,
            tokens_limit: 200_000,
        })
    );
}

#[test]
fn start_reports_no_context_when_limit_unknown_parity() {
    assert_eq!(resolve_start_context_info(None, 0), None);
}

// ---- /tasklist sortTasks ordering (5) ----

#[test]
fn task_orders_earlier_next_run_first_parity() {
    let earlier = TaskSortKey {
        next_run: TaskNextRun::Ms(100),
        created_at: "2026-03-12T10:00:00.000Z",
    };
    let later = TaskSortKey {
        next_run: TaskNextRun::Ms(200),
        created_at: "2026-03-11T10:00:00.000Z",
    };
    assert_eq!(compare_task_order(&earlier, &later), Ordering::Less);
    assert_eq!(compare_task_order(&later, &earlier), Ordering::Greater);
}

#[test]
fn task_sorts_missing_next_run_last_parity() {
    let dated = TaskSortKey {
        next_run: TaskNextRun::Ms(100),
        created_at: "2026-03-12T10:00:00.000Z",
    };
    let missing = TaskSortKey {
        next_run: TaskNextRun::Missing,
        created_at: "2026-03-10T10:00:00.000Z",
    };
    assert_eq!(compare_task_order(&dated, &missing), Ordering::Less);
    assert_eq!(compare_task_order(&missing, &dated), Ordering::Greater);
}

#[test]
fn task_breaks_ties_by_created_at_parity() {
    let first_created = TaskSortKey {
        next_run: TaskNextRun::Ms(100),
        created_at: "2026-03-10T10:00:00.000Z",
    };
    let second_created = TaskSortKey {
        next_run: TaskNextRun::Ms(100),
        created_at: "2026-03-12T10:00:00.000Z",
    };
    assert_eq!(
        compare_task_order(&first_created, &second_created),
        Ordering::Less
    );
    let missing_a = TaskSortKey {
        next_run: TaskNextRun::Missing,
        created_at: "2026-03-10T10:00:00.000Z",
    };
    let missing_b = TaskSortKey {
        next_run: TaskNextRun::Missing,
        created_at: "2026-03-12T10:00:00.000Z",
    };
    assert_eq!(compare_task_order(&missing_a, &missing_b), Ordering::Less);
}

#[test]
fn task_treats_unparseable_next_run_as_input_order_parity() {
    // TS Date.parse failure yields NaN, which the comparator returns as-is
    // and Array.sort treats as 0: input order is preserved.
    let invalid = TaskSortKey {
        next_run: TaskNextRun::Invalid,
        created_at: "2026-03-10T10:00:00.000Z",
    };
    let dated = TaskSortKey {
        next_run: TaskNextRun::Ms(100),
        created_at: "2026-03-12T10:00:00.000Z",
    };
    assert_eq!(compare_task_order(&invalid, &dated), Ordering::Equal);
    assert_eq!(compare_task_order(&dated, &invalid), Ordering::Equal);
    assert_eq!(compare_task_order(&invalid, &invalid), Ordering::Equal);
}

#[test]
fn task_treats_unparseable_next_run_as_equal_to_missing_parity() {
    // NaN !== +Infinity, so the comparator still returns NaN (== 0).
    let invalid = TaskSortKey {
        next_run: TaskNextRun::Invalid,
        created_at: "2026-03-10T10:00:00.000Z",
    };
    let missing = TaskSortKey {
        next_run: TaskNextRun::Missing,
        created_at: "2026-03-12T10:00:00.000Z",
    };
    assert_eq!(compare_task_order(&invalid, &missing), Ordering::Equal);
    assert_eq!(compare_task_order(&missing, &invalid), Ordering::Equal);
}

// ---- /detach pinned-context reset (2) ----

#[test]
fn detached_context_resets_usage_keeps_limit_parity() {
    assert_eq!(detached_context_usage(200_000), (0, 200_000));
}

#[test]
fn detached_context_zero_limit_stays_zero_parity() {
    assert_eq!(detached_context_usage(0), (0, 0));
}
