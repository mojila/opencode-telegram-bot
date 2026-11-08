//! Slice 88 parity: worktree callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/worktree-callback-handler.ts` (the `worktree:`
//! prefix dispatch guard; the page vs index dispatch branch; the
//! digits-only page/index parses with the page-prefix exclusion; the
//! foreground-busy `bot.session_busy` gate; the missing-project
//! `worktree.project_not_selected` and missing-git-context
//! `worktree.not_git_repo_callback` guards; the page-branch
//! `worktrees.length === 0` empty gate; the `index === null`
//! `callback.processing_error` gate; the out-of-range
//! `worktree.selection_missing_callback` gate; the `isCurrent`
//! `worktree.already_selected_callback` gate; the `worktree.selected`
//! success reply key; the `worktree.select_error` catch-all).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `worktree_callback_gates` is ported (only the constant-value tests
//! pass).

use otb_core::worktree_callback_gates::*;

// ---- constants ----

#[test]
fn worktree_cb_prefix_value_parity() {
    assert_eq!(WORKTREE_CB_PREFIX, "worktree:");
}

#[test]
fn worktree_cb_page_prefix_value_parity() {
    assert_eq!(WORKTREE_CB_PAGE_PREFIX, "worktree:page:");
}

#[test]
fn worktree_project_missing_key_value_parity() {
    assert_eq!(WORKTREE_PROJECT_MISSING_KEY, "worktree.project_not_selected");
}

#[test]
fn worktree_not_git_key_value_parity() {
    assert_eq!(WORKTREE_NOT_GIT_KEY, "worktree.not_git_repo_callback");
}

#[test]
fn worktree_page_empty_key_value_parity() {
    assert_eq!(WORKTREE_PAGE_EMPTY_KEY, "worktree.page_empty_callback");
}

#[test]
fn worktree_selection_missing_key_value_parity() {
    assert_eq!(
        WORKTREE_SELECTION_MISSING_KEY,
        "worktree.selection_missing_callback"
    );
}

#[test]
fn worktree_already_selected_key_value_parity() {
    assert_eq!(
        WORKTREE_ALREADY_SELECTED_KEY,
        "worktree.already_selected_callback"
    );
}

#[test]
fn worktree_processing_error_key_value_parity() {
    assert_eq!(WORKTREE_PROCESSING_ERROR_KEY, "callback.processing_error");
}

#[test]
fn worktree_select_error_key_value_parity() {
    assert_eq!(WORKTREE_SELECT_ERROR_KEY, "worktree.select_error");
}

#[test]
fn worktree_busy_key_value_parity() {
    assert_eq!(WORKTREE_BUSY_KEY, "bot.session_busy");
}

#[test]
fn worktree_selected_key_value_parity() {
    assert_eq!(WORKTREE_SELECTED_KEY, "worktree.selected");
}

// ---- prefix dispatch guard ----

#[test]
fn handles_worktree_callback_none_parity() {
    assert!(!handles_worktree_callback(None));
}

#[test]
fn handles_worktree_callback_empty_parity() {
    assert!(!handles_worktree_callback(Some("")));
}

#[test]
fn handles_worktree_callback_index_parity() {
    assert!(handles_worktree_callback(Some("worktree:2")));
}

#[test]
fn handles_worktree_callback_page_parity() {
    assert!(handles_worktree_callback(Some("worktree:page:1")));
}

#[test]
fn handles_worktree_callback_rejects_other_parity() {
    assert!(!handles_worktree_callback(Some("session:2")));
    assert!(!handles_worktree_callback(Some("worktree")));
    assert!(!handles_worktree_callback(Some("Worktree:1")));
}

// ---- dispatch classifier ----

#[test]
fn classify_page_parity() {
    assert_eq!(
        classify_worktree_callback_action("worktree:page:0"),
        WorktreeCallbackAction::Page
    );
}

#[test]
fn classify_select_index_parity() {
    assert_eq!(
        classify_worktree_callback_action("worktree:0"),
        WorktreeCallbackAction::Select
    );
}

#[test]
fn classify_select_bad_index_still_select_parity() {
    assert_eq!(
        classify_worktree_callback_action("worktree:abc"),
        WorktreeCallbackAction::Select
    );
}

#[test]
fn classify_unknown_suffix_parity() {
    assert_eq!(
        classify_worktree_callback_action("worktree:page"),
        WorktreeCallbackAction::Unknown
    );
}

#[test]
fn classify_missing_prefix_parity() {
    assert_eq!(
        classify_worktree_callback_action("other:1"),
        WorktreeCallbackAction::Unknown
    );
}

// ---- page parse ----

#[test]
fn parse_page_accepts_digits_parity() {
    assert_eq!(parse_worktree_page_value("worktree:page:3"), Some(3));
}

#[test]
fn parse_page_rejects_non_digits_parity() {
    assert_eq!(parse_worktree_page_value("worktree:page:3x"), None);
    assert_eq!(parse_worktree_page_value("worktree:page:-1"), None);
}

#[test]
fn parse_page_rejects_empty_parity() {
    assert_eq!(parse_worktree_page_value("worktree:page:"), None);
}

#[test]
fn parse_page_rejects_non_page_payload_parity() {
    assert_eq!(parse_worktree_page_value("worktree:2"), None);
    assert_eq!(parse_worktree_page_value("session:page:1"), None);
}

// ---- index parse ----

#[test]
fn parse_index_accepts_digits_parity() {
    assert_eq!(parse_worktree_index_value("worktree:2"), Some(2));
}

#[test]
fn parse_index_rejects_page_prefix_parity() {
    assert_eq!(parse_worktree_index_value("worktree:page:1"), None);
}

#[test]
fn parse_index_rejects_non_digit_parity() {
    assert_eq!(parse_worktree_index_value("worktree:abc"), None);
    assert_eq!(parse_worktree_index_value("worktree:-1"), None);
}

#[test]
fn parse_index_rejects_empty_and_other_parity() {
    assert_eq!(parse_worktree_index_value("worktree:"), None);
    assert_eq!(parse_worktree_index_value("session:1"), None);
}

// ---- page-empty gate ----

#[test]
fn list_empty_when_zero_parity() {
    assert!(is_worktree_list_empty(0));
}

#[test]
fn list_non_empty_parity() {
    assert!(!is_worktree_list_empty(3));
}

// ---- null-index gate ----

#[test]
fn index_missing_none_parity() {
    assert!(is_worktree_index_missing(None));
}

#[test]
fn index_present_parity() {
    assert!(!is_worktree_index_missing(Some(1)));
}

// ---- selection bounds ----

#[test]
fn resolve_selection_hit_parity() {
    assert_eq!(resolve_worktree_selection(3, Some(0)), Some(0));
}

#[test]
fn resolve_selection_last_hit_parity() {
    assert_eq!(resolve_worktree_selection(3, Some(2)), Some(2));
}

#[test]
fn resolve_selection_out_of_bounds_parity() {
    assert_eq!(resolve_worktree_selection(3, Some(3)), None);
}

#[test]
fn resolve_selection_none_parity() {
    assert_eq!(resolve_worktree_selection(3, None), None);
}

// ---- already-selected gate ----

#[test]
fn already_selected_true_parity() {
    assert!(is_worktree_already_selected(true));
}

#[test]
fn already_selected_false_parity() {
    assert!(!is_worktree_already_selected(false));
}

// ---- busy gate ----

#[test]
fn blocks_when_busy_parity() {
    assert!(should_block_worktree_when_busy(true));
}

#[test]
fn passes_when_idle_parity() {
    assert!(!should_block_worktree_when_busy(false));
}

// ---- answer keys ----

#[test]
fn answer_key_project_missing_parity() {
    assert_eq!(
        worktree_answer_key(Some(WorktreeFailure::ProjectMissing)),
        Some("worktree.project_not_selected")
    );
}

#[test]
fn answer_key_not_git_parity() {
    assert_eq!(
        worktree_answer_key(Some(WorktreeFailure::NotGitRepo)),
        Some("worktree.not_git_repo_callback")
    );
}

#[test]
fn answer_key_page_empty_parity() {
    assert_eq!(
        worktree_answer_key(Some(WorktreeFailure::PageEmpty)),
        Some("worktree.page_empty_callback")
    );
}

#[test]
fn answer_key_processing_error_parity() {
    assert_eq!(
        worktree_answer_key(Some(WorktreeFailure::ProcessingError)),
        Some("callback.processing_error")
    );
}

#[test]
fn answer_key_selection_missing_parity() {
    assert_eq!(
        worktree_answer_key(Some(WorktreeFailure::SelectionMissing)),
        Some("worktree.selection_missing_callback")
    );
}

#[test]
fn answer_key_already_selected_parity() {
    assert_eq!(
        worktree_answer_key(Some(WorktreeFailure::AlreadySelected)),
        Some("worktree.already_selected_callback")
    );
}

#[test]
fn answer_key_select_error_parity() {
    assert_eq!(
        worktree_answer_key(Some(WorktreeFailure::SelectError)),
        Some("worktree.select_error")
    );
}

#[test]
fn answer_key_success_empty_parity() {
    assert_eq!(worktree_answer_key(None), None);
}
