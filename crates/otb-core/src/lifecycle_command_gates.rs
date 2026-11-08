//! Session-lifecycle + utility command dispatch gates (slice 106, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/commands/worktree-command.ts` (busy gate, container gate,
//!   `!currentProject` gate, `!context` not-a-git-repo gate,
//!   `worktrees.length === 0` empty gate)
//! - `src/bot/commands/new-command.ts` (busy gate, `!currentProject` gate,
//!   `getMissingFolderNotice` folder-gone short-circuit,
//!   `SessionInfo` build `{ id, title, directory: worktree }` with the
//!   OpenCode title kept verbatim, even empty)
//! - `src/bot/commands/start-command.ts` (pin/keyboard re-init is impure;
//!   the pure remainder is the `contextInfo` fallback:
//!   `getContextInfo() ?? (limit > 0 ? { tokensUsed: 0, tokensLimit: limit }
//!   : null)`)
//! - `src/bot/commands/tasklist-command.ts` (`sortTasks`: `nextRunAt`
//!   parsed with `Date.parse`, missing dates sorting as `+Infinity`,
//!   ties broken by `createdAt.localeCompare`)
//! - `src/bot/commands/detach-command.ts` (`!currentProject` /
//!   `!currentSession` gates, pinned-context reset `updateContext(0, limit)`
//!   keeping the limit while zeroing usage)
//! - `src/bot/commands/rename-command.ts` (`!currentSession` gate)
//! - `src/bot/commands/settings-command.ts` (no pure remainder: it builds
//!   the settings view and replies with an inline menu)
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `isForegroundBusy` / `replyBusyBlocked` dispatch, `isContainerRuntime`
//!   dispatch, `!currentProject` / `!currentSession` / empty-list gates
//!   (`attach_foreground_gates` / `stores_runtime` / `browse_command_gates`
//!   owners; this slice takes their outcomes as explicit bools and only
//!   owns the gates below that are new per command).
//! - `formatSessionTitle` empty-title display mapping
//!   (`session-title-formatter`; the caller maps the verbatim title kept by
//!   `build_new_session_info` to `pinned.default_session_title`).
//! - `getMissingFolderNotice` / `getCurrentFolderWorktreeContext` /
//!   `listPresentWorktrees` scans, `buildWorktreeMenuView` /
//!   `buildSettingsMenuView` / `buildTaskListKeyboard` menu building,
//!   `t(...)` reply keys (the caller maps the bool gates below to the
//!   `worktree.*` / `new.*` / `detach.*` / `tasklist.*` texts).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`reply`, `replyWithInlineMenu`,
//!   `interactionManager.start`), `getCurrentProject` / `getCurrentSession`
//!   reads, `opencodeClient.session.create`, `attachToSession` /
//!   `detachAttachedSession`, `handOverPromptQueue`, store/manager writes,
//!   `Date.now()` clocks, `logger`.
//!
//! Deviations: none intended. `Date.parse` invalid-date handling is folded
//! into [`TaskNextRun::Invalid`]: TS compares `NaN` against anything with
//! `!==` (always true) and returns `NaN` from the comparator, which
//! `Array.sort` treats as `0`, so an unparseable `nextRunAt` keeps input
//! order against every other row. Callers map a missing `nextRunAt` to
//! [`TaskNextRun::Missing`] and a present-but-unparseable one to
//! [`TaskNextRun::Invalid`].
//!
//! Step 2 (GREEN): pure port landed. The parity tests in
//! `crates/otb-core/tests/slice106_lifecycle_command_parity.rs` assert this
//! contract.

use std::cmp::Ordering;

/// `true` when `/worktree` stops early because the project folder has no
/// git context (TS: `if (!context) { await ctx.reply(...); return; }`).
pub fn should_report_not_git_repo(has_context: bool) -> bool {
    !has_context
}

/// `true` when `/new` answers with the missing-folder notice and creates
/// nothing (TS: `if (folderNotice) { await ctx.reply(folderNotice); return; }`).
pub fn should_block_new_on_folder_notice(has_notice: bool) -> bool {
    has_notice
}

/// Session identity `/new` stores and follows (TS: `const sessionInfo:
/// SessionInfo = { id: session.id, title: session.title, directory:
/// currentProject.worktree }`). The OpenCode title travels verbatim,
/// including empty (the display layer names that a new session).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSessionInfo {
    pub id: String,
    pub title: String,
    pub directory: String,
}

/// Build the [`NewSessionInfo`] for a freshly created OpenCode session.
pub fn build_new_session_info(id: &str, title: &str, project_worktree: &str) -> NewSessionInfo {
    NewSessionInfo {
        id: id.to_string(),
        title: title.to_string(),
        directory: project_worktree.to_string(),
    }
}

/// Token context `/start` shows on its fresh keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartContextInfo {
    pub tokens_used: u64,
    pub tokens_limit: u64,
}

/// Resolve the `/start` keyboard context (TS: `getContextInfo() ??
/// (getContextLimit() > 0 ? { tokensUsed: 0, tokensLimit } : null)`).
/// A stored snapshot always wins, even when its limit is zero.
pub fn resolve_start_context_info(
    stored: Option<StartContextInfo>,
    context_limit: u64,
) -> Option<StartContextInfo> {
    if let Some(info) = stored {
        return Some(info);
    }
    if context_limit > 0 {
        return Some(StartContextInfo {
            tokens_used: 0,
            tokens_limit: context_limit,
        });
    }
    None
}

/// Sortable form of a scheduled task's next run (TS: `left.nextRunAt ?
/// Date.parse(left.nextRunAt) : Number.POSITIVE_INFINITY`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskNextRun {
    /// A parseable `nextRunAt` as epoch millis.
    Ms(i64),
    /// A missing `nextRunAt`, sorting as `+Infinity` (last).
    Missing,
    /// A present-but-unparseable `nextRunAt` (`Date.parse` yields `NaN`,
    /// which compares as input order against every other row).
    Invalid,
}

/// Sortable snapshot of one scheduled task (TS: `sortTasks` comparator
/// inputs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSortKey<'a> {
    pub next_run: TaskNextRun,
    pub created_at: &'a str,
}

/// Compare two scheduled tasks for `/tasklist` display order (TS:
/// `sortTasks`: earlier next run first, missing runs last, `createdAt`
/// string tiebreak). An [`TaskNextRun::Invalid`] row compares [`Ordering::Equal`]
/// against every other row, preserving input order under a stable sort.
pub fn compare_task_order(a: &TaskSortKey<'_>, b: &TaskSortKey<'_>) -> Ordering {
    if matches!(a.next_run, TaskNextRun::Invalid) || matches!(b.next_run, TaskNextRun::Invalid) {
        return Ordering::Equal;
    }
    match (a.next_run, b.next_run) {
        (TaskNextRun::Ms(left), TaskNextRun::Ms(right)) => {
            if left != right {
                return left.cmp(&right);
            }
        }
        (TaskNextRun::Ms(_), TaskNextRun::Missing) => return Ordering::Less,
        (TaskNextRun::Missing, TaskNextRun::Ms(_)) => return Ordering::Greater,
        (TaskNextRun::Missing, TaskNextRun::Missing) => {}
        (TaskNextRun::Invalid, _) | (_, TaskNextRun::Invalid) => {
            return Ordering::Equal;
        }
    }
    a.created_at.cmp(b.created_at)
}

/// Pinned-context usage `/detach` resets to while keeping the limit
/// (TS: `deps.keyboardManager.updateContext(0, contextLimit)`).
/// Returns `(tokens_used, tokens_limit)`.
pub fn detached_context_usage(context_limit: u64) -> (u64, u64) {
    (0, context_limit)
}
