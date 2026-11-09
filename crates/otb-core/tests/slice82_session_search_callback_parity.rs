//! Slice 82 parity: session-search callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/session-callback-handler.ts`
//! (`sessionSearchState` kind/flow gate, `sameSessionSearch`
//! token/stage equality, `handleSessionSearchCallback` dispatch guard,
//! exact `session:search` / `...:again` / `...:cancel` matches,
//! `...:page:` / `...:result:` prefix matches, page digits-only +
//! string-query gate, result digits-plus-`ids`-array-plus-in-bounds
//! gate, `results`/`input` stage gates, `messageId` mismatch gate,
//! combined `inactive_callback` gate, blank-query `trim()` gate,
//! text-input handled gate, foreground-busy block;
//! `handleSessionSearchTextInput` handled gate).
//! `cap_session_search_query` (`slice(0, 100)`), `session_matches_search`,
//! `parseSessionPageCallback`, `parseSessionIdCallback`,
//! `parseBackgroundSessionCallback`, and `SESSION_CALLBACK_PREFIX`
//! already live in `command_session_gates` and are NOT re-tested here.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `session_search_callback_gates` is ported (only the constant-value
//! tests pass).

use otb_core::session_search_callback_gates::*;

fn snapshot(
    kind: &str,
    flow: Option<&str>,
    stage: Option<&str>,
    token: Option<u64>,
) -> SessionSearchSnapshot {
    SessionSearchSnapshot {
        kind: kind.to_string(),
        flow: flow.map(str::to_string),
        stage: stage.map(str::to_string),
        token,
        query: Some("standup notes".to_string()),
        message_id: Some(21),
        ids: Some(vec!["s1".to_string(), "s2".to_string()]),
    }
}

fn ids() -> Vec<String> {
    vec!["s1".to_string(), "s2".to_string()]
}

// ---- constants ----

#[test]
fn search_callback_value_parity() {
    assert_eq!(SESSION_SEARCH_CALLBACK, "session:search");
}

#[test]
fn search_dispatch_prefix_value_parity() {
    assert_eq!(SESSION_SEARCH_DISPATCH_PREFIX, "session:search:");
}

#[test]
fn search_page_prefix_value_parity() {
    assert_eq!(SESSION_SEARCH_PAGE_PREFIX, "session:search:page:");
}

#[test]
fn search_result_prefix_value_parity() {
    assert_eq!(SESSION_SEARCH_RESULT_PREFIX, "session:search:result:");
}

#[test]
fn search_again_cancel_values_parity() {
    assert_eq!(SESSION_SEARCH_AGAIN, "session:search:again");
    assert_eq!(SESSION_SEARCH_CANCEL, "session:search:cancel");
}

// ---- dispatch guard ----

#[test]
fn handles_search_exact_parity() {
    assert!(handles_session_search_callback(Some("session:search")));
}

#[test]
fn handles_search_page_parity() {
    assert!(handles_session_search_callback(Some(
        "session:search:page:0"
    )));
}

#[test]
fn handles_search_bare_prefix_parity() {
    assert!(handles_session_search_callback(Some("session:search:")));
}

#[test]
fn handles_search_rejects_none_and_empty_parity() {
    assert!(!handles_session_search_callback(None));
    assert!(!handles_session_search_callback(Some("")));
}

#[test]
fn handles_search_rejects_session_prefix_parity() {
    assert!(!handles_session_search_callback(Some("session:abc")));
    assert!(!handles_session_search_callback(Some("session:page:0")));
    assert!(!handles_session_search_callback(Some("session:")));
}

#[test]
fn handles_search_rejects_other_prefixes_parity() {
    assert!(!handles_session_search_callback(Some("session:searchx")));
    assert!(!handles_session_search_callback(Some("project:abc")));
    assert!(!handles_session_search_callback(Some(
        "background-session:a:s1"
    )));
}

// ---- exact + prefix matches ----

#[test]
fn exact_start_match_parity() {
    assert!(is_session_search_start("session:search"));
    assert!(!is_session_search_start("session:search:page:0"));
    assert!(!is_session_search_start("session:search:"));
}

#[test]
fn exact_cancel_match_parity() {
    assert!(is_session_search_cancel("session:search:cancel"));
    assert!(!is_session_search_cancel("session:search:again"));
    assert!(!is_session_search_cancel("session:search:cancel:x"));
}

#[test]
fn exact_again_match_parity() {
    assert!(is_session_search_again("session:search:again"));
    assert!(!is_session_search_again("session:search:cancel"));
}

#[test]
fn page_prefix_match_parity() {
    assert!(is_session_search_page("session:search:page:2"));
    assert!(!is_session_search_page("session:search:result:0"));
    assert!(!is_session_search_page("session:search"));
}

#[test]
fn result_prefix_match_parity() {
    assert!(is_session_search_result("session:search:result:3"));
    assert!(!is_session_search_result("session:search:page:1"));
    assert!(!is_session_search_result("session:search"));
}

// ---- page raw + request gate ----

#[test]
fn page_raw_valid_parity() {
    assert_eq!(parse_session_search_page_raw("0"), Some(0));
    assert_eq!(parse_session_search_page_raw("12"), Some(12));
    assert_eq!(parse_session_search_page_raw("007"), Some(7));
}

#[test]
fn page_raw_rejects_bad_shapes_parity() {
    assert_eq!(parse_session_search_page_raw(""), None);
    assert_eq!(parse_session_search_page_raw("1a"), None);
    assert_eq!(parse_session_search_page_raw("-1"), None);
    assert_eq!(parse_session_search_page_raw("1.5"), None);
}

#[test]
fn page_request_valid_parity() {
    assert!(is_session_search_page_request_valid("2", true));
}

#[test]
fn page_request_invalid_parity() {
    assert!(!is_session_search_page_request_valid("2", false));
    assert!(!is_session_search_page_request_valid("x", true));
    assert!(!is_session_search_page_request_valid("", true));
}

// ---- result resolve ----

#[test]
fn result_resolves_ids_parity() {
    let list = ids();
    assert_eq!(
        resolve_session_search_result_id(Some(&list), "0"),
        Some("s1".to_string())
    );
    assert_eq!(
        resolve_session_search_result_id(Some(&list), "1"),
        Some("s2".to_string())
    );
}

#[test]
fn result_none_when_ids_missing_parity() {
    assert_eq!(resolve_session_search_result_id(None, "0"), None);
    assert_eq!(resolve_session_search_result_id(Some(&[]), "0"), None);
}

#[test]
fn result_none_for_bad_raw_parity() {
    let list = ids();
    assert_eq!(resolve_session_search_result_id(Some(&list), "x"), None);
    assert_eq!(resolve_session_search_result_id(Some(&list), ""), None);
    assert_eq!(resolve_session_search_result_id(Some(&list), "-1"), None);
}

#[test]
fn result_none_when_out_of_bounds_parity() {
    let list = ids();
    assert_eq!(resolve_session_search_result_id(Some(&list), "2"), None);
    assert_eq!(resolve_session_search_result_id(Some(&list), "99"), None);
}

// ---- flow state ----

#[test]
fn flow_state_none_input_is_none_parity() {
    assert_eq!(session_search_flow_state(None), None);
}

#[test]
fn flow_state_non_custom_kind_is_none_parity() {
    let mut s = snapshot("inline", Some("session-search"), Some("results"), Some(9));
    assert_eq!(session_search_flow_state(Some(&s)), None);
    s.kind = "rename".to_string();
    assert_eq!(session_search_flow_state(Some(&s)), None);
}

#[test]
fn flow_state_wrong_flow_is_none_parity() {
    let s = snapshot("custom", Some("project-search"), Some("results"), Some(9));
    assert_eq!(session_search_flow_state(Some(&s)), None);
    let s = snapshot("custom", None, Some("results"), Some(9));
    assert_eq!(session_search_flow_state(Some(&s)), None);
}

#[test]
fn flow_state_missing_stage_or_token_is_none_parity() {
    let s = snapshot("custom", Some("session-search"), None, Some(9));
    assert_eq!(session_search_flow_state(Some(&s)), None);
    let s = snapshot("custom", Some("session-search"), Some("results"), None);
    assert_eq!(session_search_flow_state(Some(&s)), None);
}

#[test]
fn flow_state_valid_returns_stage_and_token_parity() {
    let s = snapshot("custom", Some("session-search"), Some("results"), Some(9));
    let out = session_search_flow_state(Some(&s)).unwrap();
    assert_eq!(
        out,
        SessionSearchFlow {
            stage: "results".to_string(),
            token: 9,
        }
    );
}

// ---- stage gates ----

#[test]
fn search_stages_parity() {
    assert!(is_session_search_results_stage(Some("results")));
    assert!(!is_session_search_results_stage(Some("input")));
    assert!(!is_session_search_results_stage(None));
}

#[test]
fn search_input_stage_parity() {
    assert!(is_session_search_input_stage(Some("input")));
    assert!(!is_session_search_input_stage(Some("results")));
    assert!(!is_session_search_input_stage(None));
}

// ---- same-search gate ----

#[test]
fn same_search_true_parity() {
    assert!(is_same_session_search(
        Some(9),
        Some("results"),
        Some(9),
        Some("results")
    ));
}

#[test]
fn same_search_false_on_token_parity() {
    assert!(!is_same_session_search(
        Some(9),
        Some("results"),
        Some(10),
        Some("results")
    ));
}

#[test]
fn same_search_false_when_stage_or_token_missing_parity() {
    assert!(!is_same_session_search(
        Some(9),
        Some("results"),
        Some(9),
        Some("input")
    ));
    assert!(!is_same_session_search(
        None,
        Some("results"),
        Some(9),
        Some("results")
    ));
    assert!(!is_same_session_search(
        Some(9),
        Some("results"),
        None,
        Some("results")
    ));
    assert!(!is_same_session_search(None, None, None, None));
}

// ---- message mismatch ----

#[test]
fn mismatch_when_state_message_missing_parity() {
    assert!(is_session_search_message_mismatch(None, Some(21)));
    assert!(is_session_search_message_mismatch(None, None));
}

#[test]
fn mismatch_on_differing_ids_parity() {
    assert!(is_session_search_message_mismatch(Some(21), Some(22)));
    assert!(is_session_search_message_mismatch(Some(21), None));
}

#[test]
fn no_mismatch_on_equal_ids_parity() {
    assert!(!is_session_search_message_mismatch(Some(21), Some(21)));
}

// ---- combined inactive gate ----

#[test]
fn inactive_when_no_state_parity() {
    assert!(should_answer_session_search_inactive(false, true, true));
    assert!(should_answer_session_search_inactive(false, false, false));
}

#[test]
fn inactive_when_stage_or_message_bad_parity() {
    assert!(should_answer_session_search_inactive(true, false, true));
    assert!(should_answer_session_search_inactive(true, true, false));
    assert!(should_answer_session_search_inactive(true, false, false));
}

#[test]
fn active_when_all_good_parity() {
    assert!(!should_answer_session_search_inactive(true, true, true));
}

// ---- blank query + text input ----

#[test]
fn blank_query_true_parity() {
    assert!(is_blank_session_search_query(""));
    assert!(is_blank_session_search_query("   "));
    assert!(is_blank_session_search_query("\t\n "));
}

#[test]
fn blank_query_false_parity() {
    assert!(!is_blank_session_search_query("standup"));
    assert!(!is_blank_session_search_query(" standup "));
}

#[test]
fn text_input_handled_when_ready_parity() {
    assert!(handles_session_search_text_input(true, true, true));
}

#[test]
fn text_input_empty_string_still_handled_parity() {
    // TS checks `ctx.message?.text === undefined`: `""` is defined, so the
    // gate passes here and the blank-query gate consumes it downstream.
    assert!(handles_session_search_text_input(true, true, true));
}

#[test]
fn text_input_ignored_parity() {
    assert!(!handles_session_search_text_input(false, true, true));
    assert!(!handles_session_search_text_input(true, false, true));
    assert!(!handles_session_search_text_input(true, true, false));
}

// ---- busy block ----

#[test]
fn busy_blocks_callback_parity() {
    assert!(should_block_session_callback_when_busy(true));
    assert!(!should_block_session_callback_when_busy(false));
}
