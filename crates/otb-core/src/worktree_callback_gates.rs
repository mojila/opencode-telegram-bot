//! Worktree callback-handler pure gates (slice 88, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/worktree-callback-handler.ts`
//!   (`WORKTREE_CALLBACK_PREFIX` dispatch guard; the page vs index
//!   `parseWorktreePageCallback` / `parseWorktreeIndexCallback` branch;
//!   the foreground-busy `replyBusyBlocked` gate; the missing-project
//!   (`worktree.project_not_selected`) and missing-git-context
//!   (`worktree.not_git_repo_callback`) guards; the page-branch
//!   `worktrees.length === 0` empty gate
//!   (`worktree.page_empty_callback`); the `index === null`
//!   `callback.processing_error` gate; the out-of-range
//!   `worktree.selection_missing_callback` gate; the `isCurrent`
//!   `worktree.already_selected_callback` gate; the trailing
//!   `worktree.select_error` catch-all).
//! - `src/bot/menus/worktree-selection-menu.ts` (the `worktree:` and
//!   `worktree:page:` callback constants; already classified in
//!   `worktree_history_inline_gates`, repeated here only as the dispatch
//!   table the handler matches).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `worktree_history_inline_gates` (`WORKTREE_CALLBACK_PREFIX`,
//!   `WORKTREE_PAGE_CALLBACK_PREFIX`, `parse_worktree_page_callback`,
//!   `parse_worktree_index_callback`, label/pagination math).
//! - `worktree_service_gates` (git porcelain parsing, present-worktree
//!   filter, owner lookup, current-folder context).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, keyboard building).
//! - `listPresentWorktrees` / `getCurrentFolderWorktreeContext` /
//!   `getProjectByWorktree` / `upsertSessionDirectory` / `switchToProject`
//!   SDK and service calls, `ensureActiveInlineMenu` / `resetInteractions`.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: message routing (`ensureActiveInlineMenu("worktree")`) and
//! SDK reloads stay with the caller; the gates take the already-loaded
//! `worktree_count`, parsed `index`, `is_current`, and `is_busy` verdicts
//! as explicit arguments. Message ids are not compared here (the handler
//! relies on the active-inline-menu guard, not a message-id check).
//!
//! Step 1 (GREEN): pure port of the TS contract above; all parity
//! tests pass.

/// `worktree:<gitIndex>` callback prefix.
pub const WORKTREE_CB_PREFIX: &str = "worktree:";
/// `worktree:page:<n>` callback prefix.
pub const WORKTREE_CB_PAGE_PREFIX: &str = "worktree:page:";
/// Missing-project answer key (TS: `t("worktree.project_not_selected")`).
pub const WORKTREE_PROJECT_MISSING_KEY: &str = "worktree.project_not_selected";
/// Missing-git-context answer key (TS: `t("worktree.not_git_repo_callback")`).
pub const WORKTREE_NOT_GIT_KEY: &str = "worktree.not_git_repo_callback";
/// Empty-page answer key (TS: `t("worktree.page_empty_callback")`).
pub const WORKTREE_PAGE_EMPTY_KEY: &str = "worktree.page_empty_callback";
/// Out-of-range answer key (TS: `t("worktree.selection_missing_callback")`).
pub const WORKTREE_SELECTION_MISSING_KEY: &str = "worktree.selection_missing_callback";
/// Already-on-worktree answer key (TS: `t("worktree.already_selected_callback")`).
pub const WORKTREE_ALREADY_SELECTED_KEY: &str = "worktree.already_selected_callback";
/// Null-index answer key (TS: `t("callback.processing_error")`).
pub const WORKTREE_PROCESSING_ERROR_KEY: &str = "callback.processing_error";
/// Catch-all error key (TS: `failure(ctx, "worktree.select_error")`).
pub const WORKTREE_SELECT_ERROR_KEY: &str = "worktree.select_error";
/// Foreground-busy answer key (TS: `replyBusyBlocked` -> `t("bot.session_busy")`).
pub const WORKTREE_BUSY_KEY: &str = "bot.session_busy";
/// Success reply key (TS: `t("worktree.selected", ...)`).
pub const WORKTREE_SELECTED_KEY: &str = "worktree.selected";

/// Handler dispatch branch for a `worktree:*` payload (TS: the page
/// branch, the index branch, and the trailing `processing_error`
/// fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeCallbackAction {
    Page,
    Select,
    Unknown,
}

/// Answer-key failure behind a handled `worktree:*` callback (TS: the
/// `alert` / `answerCallbackQuery({ text })` / `failure` keys; success
/// answers empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeFailure {
    ProjectMissing,
    NotGitRepo,
    PageEmpty,
    ProcessingError,
    SelectionMissing,
    AlreadySelected,
    SelectError,
}

/// Worktree-callback dispatch guard (TS: `if (!callbackQuery?.data ||
/// !callbackQuery.data.startsWith(WORKTREE_CALLBACK_PREFIX)) return false`).
pub fn handles_worktree_callback(data: Option<&str>) -> bool {
    match data {
        Some(value) => value.starts_with(WORKTREE_CB_PREFIX),
        None => false,
    }
}

/// Handler dispatch classifier (TS: `parseWorktreePageCallback(data) !==
/// null` takes the page branch; otherwise `parseWorktreeIndexCallback`
/// takes the select branch, anything else falls through to
/// `callback.processing_error`).
pub fn classify_worktree_callback_action(data: &str) -> WorktreeCallbackAction {
    if data.starts_with(WORKTREE_CB_PAGE_PREFIX) {
        return WorktreeCallbackAction::Page;
    }
    let Some(rest) = data.strip_prefix(WORKTREE_CB_PREFIX) else {
        return WorktreeCallbackAction::Unknown;
    };
    // Bare/malformed `worktree:page*` payloads are neither branch (TS: both
    // parses return null); everything else under `worktree:` heads for the
    // select branch (which answers `callback.processing_error` when the
    // index itself does not parse).
    if rest.starts_with("page") {
        WorktreeCallbackAction::Unknown
    } else {
        WorktreeCallbackAction::Select
    }
}

/// Page of a `worktree:page:<n>` callback; `None` unless the payload is
/// non-empty ASCII digits (TS: `parseWorktreePageCallback`).
pub fn parse_worktree_page_value(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(WORKTREE_CB_PAGE_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok()
}

/// Git index of a `worktree:<n>` callback; `None` for page callbacks and
/// non-digit payloads (TS: `parseWorktreeIndexCallback`).
pub fn parse_worktree_index_value(data: &str) -> Option<u64> {
    if data.starts_with(WORKTREE_CB_PAGE_PREFIX) {
        return None;
    }
    let raw = data.strip_prefix(WORKTREE_CB_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok()
}

/// Page-branch empty gate (TS: `worktrees.length === 0` answers
/// `worktree.page_empty_callback`).
pub fn is_worktree_list_empty(worktree_count: usize) -> bool {
    worktree_count == 0
}

/// Null-index gate (TS: `index === null` answers
/// `callback.processing_error`).
pub fn is_worktree_index_missing(index: Option<u64>) -> bool {
    index.is_none()
}

/// Index bounds against the full git list (TS: `context.worktrees[index]`;
/// missing reads as `worktree.selection_missing_callback`).
pub fn resolve_worktree_selection(count: usize, index: Option<u64>) -> Option<usize> {
    let value = usize::try_from(index?).ok()?;
    if value < count {
        Some(value)
    } else {
        None
    }
}

/// Already-on-worktree gate (TS: `selectedWorktree.isCurrent` answers
/// `worktree.already_selected_callback`).
pub fn is_worktree_already_selected(is_current: bool) -> bool {
    is_current
}

/// Foreground-busy gate (TS: `isForegroundBusy(deps)` answers
/// `replyBusyBlocked` before the menu guard).
pub fn should_block_worktree_when_busy(is_busy: bool) -> bool {
    is_busy
}

/// Answer key for a handled outcome (`None` = empty answer, TS: bare
/// `answerCallbackQuery()` on page render / worktree switch).
pub fn worktree_answer_key(failure: Option<WorktreeFailure>) -> Option<&'static str> {
    match failure {
        None => None,
        Some(WorktreeFailure::ProjectMissing) => Some(WORKTREE_PROJECT_MISSING_KEY),
        Some(WorktreeFailure::NotGitRepo) => Some(WORKTREE_NOT_GIT_KEY),
        Some(WorktreeFailure::PageEmpty) => Some(WORKTREE_PAGE_EMPTY_KEY),
        Some(WorktreeFailure::ProcessingError) => Some(WORKTREE_PROCESSING_ERROR_KEY),
        Some(WorktreeFailure::SelectionMissing) => Some(WORKTREE_SELECTION_MISSING_KEY),
        Some(WorktreeFailure::AlreadySelected) => Some(WORKTREE_ALREADY_SELECTED_KEY),
        Some(WorktreeFailure::SelectError) => Some(WORKTREE_SELECT_ERROR_KEY),
    }
}
