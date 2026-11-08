//! Slice 92 parity: recent callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/recent-callback-handler.ts` (the `recent:`
//! prefix dispatch guard; the digits-only index gate with the `-1`
//! inactive path; the `sessionIds`/`directories` array-plus-bounds
//! resolve gate; the fetched-session usability gate (`error` / missing
//! / directory-mismatch / `parentID` present answers
//! `inline.inactive_callback`); the combined inactive gate; the
//! foreground-busy `bot.session_busy` gate).
//! `RECENT_CALLBACK_PREFIX`, `parse_recent_index_callback`, and
//! `build_recent_callback` already live in `recent_scheduled_gates`
//! and are NOT re-tested here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `recent_callback_gates` is ported (only the constant-value tests
//! pass).

use otb_core::recent_callback_gates::*;

fn ids() -> Vec<String> {
    vec!["s1".to_string(), "s2".to_string()]
}

fn dirs() -> Vec<String> {
    vec!["/a".to_string(), "/b".to_string()]
}

// ---- constants ----

#[test]
fn recent_cb_prefix_value_parity() {
    assert_eq!(RECENT_CB_PREFIX, "recent:");
}

#[test]
fn recent_busy_key_value_parity() {
    assert_eq!(RECENT_BUSY_KEY, "bot.session_busy");
}

#[test]
fn recent_inactive_key_value_parity() {
    assert_eq!(RECENT_INACTIVE_KEY, "inline.inactive_callback");
}

#[test]
fn recent_select_error_key_value_parity() {
    assert_eq!(RECENT_SELECT_ERROR_KEY, "sessions.select_error");
}

// ---- dispatch guard ----

#[test]
fn handles_recent_index_parity() {
    assert!(handles_recent_callback(Some("recent:0")));
    assert!(handles_recent_callback(Some("recent:12")));
}

#[test]
fn handles_recent_bare_prefix_parity() {
    // `startsWith("recent:")` holds; the empty index reads as inactive downstream.
    assert!(handles_recent_callback(Some("recent:")));
}

#[test]
fn handles_recent_rejects_none_and_empty_parity() {
    assert!(!handles_recent_callback(None));
    assert!(!handles_recent_callback(Some("")));
}

#[test]
fn handles_recent_rejects_other_prefixes_parity() {
    assert!(!handles_recent_callback(Some("session:abc")));
    assert!(!handles_recent_callback(Some("project:abc")));
    assert!(!handles_recent_callback(Some("recent")));
    assert!(!handles_recent_callback(Some("recentx:0")));
    assert!(!handles_recent_callback(Some("task:retry-schedule")));
}

// ---- resolve selection ----

#[test]
fn resolve_recent_valid_pair_parity() {
    let list = ids();
    let directories = dirs();
    assert_eq!(
        resolve_recent_selection(Some(&list), Some(&directories), Some(0)),
        Some(RecentSelection {
            session_id: "s1".to_string(),
            directory: "/a".to_string(),
        })
    );
    assert_eq!(
        resolve_recent_selection(Some(&list), Some(&directories), Some(1)),
        Some(RecentSelection {
            session_id: "s2".to_string(),
            directory: "/b".to_string(),
        })
    );
}

#[test]
fn resolve_recent_none_when_index_missing_parity() {
    let list = ids();
    let directories = dirs();
    // TS `-1` path: non-digits index never reaches the arrays.
    assert_eq!(
        resolve_recent_selection(Some(&list), Some(&directories), None),
        None
    );
}

#[test]
fn resolve_recent_none_when_ids_missing_parity() {
    let directories = dirs();
    assert_eq!(
        resolve_recent_selection(None, Some(&directories), Some(0)),
        None
    );
    assert_eq!(
        resolve_recent_selection(Some(&[]), Some(&directories), Some(0)),
        None
    );
}

#[test]
fn resolve_recent_none_when_directories_missing_parity() {
    let list = ids();
    assert_eq!(resolve_recent_selection(Some(&list), None, Some(0)), None);
    assert_eq!(
        resolve_recent_selection(Some(&list), Some(&[]), Some(0)),
        None
    );
}

#[test]
fn resolve_recent_none_when_out_of_bounds_parity() {
    let list = ids();
    let directories = dirs();
    assert_eq!(
        resolve_recent_selection(Some(&list), Some(&directories), Some(2)),
        None
    );
    assert_eq!(
        resolve_recent_selection(Some(&list), Some(&directories), Some(99)),
        None
    );
}

#[test]
fn resolve_recent_none_when_lengths_differ_parity() {
    let list = ids();
    let one_dir = vec!["/a".to_string()];
    // Index valid for `ids` but missing in `directories` reads as inactive.
    assert_eq!(
        resolve_recent_selection(Some(&list), Some(&one_dir), Some(1)),
        None
    );
}

// ---- session usability ----

#[test]
fn recent_session_usable_when_leaf_matches_parity() {
    assert!(is_recent_session_usable(true, true, false));
}

#[test]
fn recent_session_unusable_when_missing_parity() {
    assert!(!is_recent_session_usable(false, true, false));
    assert!(!is_recent_session_usable(false, false, false));
}

#[test]
fn recent_session_unusable_on_directory_mismatch_parity() {
    assert!(!is_recent_session_usable(true, false, false));
}

#[test]
fn recent_session_unusable_when_nested_parity() {
    assert!(!is_recent_session_usable(true, true, true));
}

// ---- combined inactive gate ----

#[test]
fn inactive_when_no_selection_parity() {
    assert!(should_answer_recent_inactive(false, true));
    assert!(should_answer_recent_inactive(false, false));
}

#[test]
fn inactive_when_session_unusable_parity() {
    assert!(should_answer_recent_inactive(true, false));
}

#[test]
fn active_when_selected_and_usable_parity() {
    assert!(!should_answer_recent_inactive(true, true));
}

// ---- busy block ----

#[test]
fn busy_blocks_recent_callback_parity() {
    assert!(should_block_recent_when_busy(true));
    assert!(!should_block_recent_when_busy(false));
}
