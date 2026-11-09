//! Slice 81 parity: project-search callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/menus/project-selection-menu.ts`
//! (`PROJECT_SEARCH_CALLBACK`, `PROJECT_SEARCH_PAGE_PREFIX`,
//! `PROJECT_SEARCH_RESULT_PREFIX`, `PROJECT_SEARCH_AGAIN`,
//! `PROJECT_SEARCH_CANCEL`) and
//! `src/bot/callbacks/project-callback-handler.ts`
//! (`handleProjectSearchCallback` dispatch guard, exact
//! `projects:search` / `...:again` / `...:cancel` matches, `...:page:` /
//! `...:result:` prefix matches, page digits-only + string-query gate,
//! result digits-plus-`ids`-array-plus-in-bounds gate, `searchState`
//! kind/flow gate, `results`/`input` stage gates, `isSameSearch`
//! token/stage equality, `messageId` mismatch gate, combined
//! `inactive_callback` gate, blank-query `trim()` gate, text-input
//! handled gate, `handleProjectSelect` page-vs-`project:` dispatch,
//! `project:<id>` id extraction, foreground-busy block).
//! `parseProjectPageCallback` (`projects:page:<n>`) and the per-project
//! search-needle match already live in `command_session_gates` and are
//! NOT re-tested here.
//!
//! Every test MUST FAIL with `unimplemented!()` panics until
//! `project_search_callback_gates` is ported.

use otb_core::project_search_callback_gates::*;

fn snapshot(
    kind: &str,
    flow: Option<&str>,
    stage: Option<&str>,
    token: Option<u64>,
) -> ProjectSearchSnapshot {
    ProjectSearchSnapshot {
        kind: kind.to_string(),
        flow: flow.map(str::to_string),
        stage: stage.map(str::to_string),
        token,
        query: Some("bot".to_string()),
        message_id: Some(11),
        ids: Some(vec!["a".to_string(), "b".to_string()]),
    }
}

fn ids() -> Vec<String> {
    vec!["a".to_string(), "b".to_string()]
}

// ---- constants ----

#[test]
fn search_callback_value_parity() {
    assert_eq!(PROJECT_SEARCH_CALLBACK, "projects:search");
}

#[test]
fn search_dispatch_prefix_value_parity() {
    assert_eq!(PROJECT_SEARCH_DISPATCH_PREFIX, "projects:search:");
}

#[test]
fn search_page_prefix_value_parity() {
    assert_eq!(PROJECT_SEARCH_PAGE_PREFIX, "projects:search:page:");
}

#[test]
fn search_result_prefix_value_parity() {
    assert_eq!(PROJECT_SEARCH_RESULT_PREFIX, "projects:search:result:");
}

#[test]
fn search_again_cancel_values_parity() {
    assert_eq!(PROJECT_SEARCH_AGAIN, "projects:search:again");
    assert_eq!(PROJECT_SEARCH_CANCEL, "projects:search:cancel");
}

#[test]
fn project_select_prefix_value_parity() {
    assert_eq!(PROJECT_SELECT_PREFIX, "project:");
}

// ---- dispatch guard ----

#[test]
fn handles_search_exact_parity() {
    assert!(handles_project_search_callback(Some("projects:search")));
}

#[test]
fn handles_search_page_parity() {
    assert!(handles_project_search_callback(Some(
        "projects:search:page:0"
    )));
}

#[test]
fn handles_search_bare_prefix_parity() {
    assert!(handles_project_search_callback(Some("projects:search:")));
}

#[test]
fn handles_search_rejects_none_and_empty_parity() {
    assert!(!handles_project_search_callback(None));
    assert!(!handles_project_search_callback(Some("")));
}

#[test]
fn handles_search_rejects_project_prefix_parity() {
    assert!(!handles_project_search_callback(Some("project:abc")));
}

#[test]
fn handles_search_rejects_other_prefixes_parity() {
    assert!(!handles_project_search_callback(Some("projects:page:1")));
    assert!(!handles_project_search_callback(Some("projects:searchx")));
    assert!(!handles_project_search_callback(Some("session:search")));
}

// ---- exact + prefix matches ----

#[test]
fn exact_start_match_parity() {
    assert!(is_project_search_start("projects:search"));
    assert!(!is_project_search_start("projects:search:page:0"));
    assert!(!is_project_search_start("projects:search:"));
}

#[test]
fn exact_cancel_match_parity() {
    assert!(is_project_search_cancel("projects:search:cancel"));
    assert!(!is_project_search_cancel("projects:search:again"));
    assert!(!is_project_search_cancel("projects:search:cancel:x"));
}

#[test]
fn exact_again_match_parity() {
    assert!(is_project_search_again("projects:search:again"));
    assert!(!is_project_search_again("projects:search:cancel"));
}

#[test]
fn page_prefix_match_parity() {
    assert!(is_project_search_page("projects:search:page:2"));
    assert!(!is_project_search_page("projects:search:result:0"));
    assert!(!is_project_search_page("projects:search"));
}

#[test]
fn result_prefix_match_parity() {
    assert!(is_project_search_result("projects:search:result:3"));
    assert!(!is_project_search_result("projects:search:page:1"));
    assert!(!is_project_search_result("projects:search"));
}

// ---- page raw + request gate ----

#[test]
fn page_raw_valid_parity() {
    assert_eq!(parse_project_search_page_raw("0"), Some(0));
    assert_eq!(parse_project_search_page_raw("12"), Some(12));
    assert_eq!(parse_project_search_page_raw("007"), Some(7));
}

#[test]
fn page_raw_rejects_bad_shapes_parity() {
    assert_eq!(parse_project_search_page_raw(""), None);
    assert_eq!(parse_project_search_page_raw("1a"), None);
    assert_eq!(parse_project_search_page_raw("-1"), None);
    assert_eq!(parse_project_search_page_raw("1.5"), None);
}

#[test]
fn page_request_valid_parity() {
    assert!(is_project_search_page_request_valid("2", true));
}

#[test]
fn page_request_invalid_parity() {
    assert!(!is_project_search_page_request_valid("2", false));
    assert!(!is_project_search_page_request_valid("x", true));
    assert!(!is_project_search_page_request_valid("", true));
}

// ---- result resolve ----

#[test]
fn result_resolves_ids_parity() {
    let list = ids();
    assert_eq!(
        resolve_project_search_result_id(Some(&list), "0"),
        Some("a".to_string())
    );
    assert_eq!(
        resolve_project_search_result_id(Some(&list), "1"),
        Some("b".to_string())
    );
}

#[test]
fn result_none_when_ids_missing_parity() {
    assert_eq!(resolve_project_search_result_id(None, "0"), None);
}

#[test]
fn result_none_for_bad_raw_parity() {
    let list = ids();
    assert_eq!(resolve_project_search_result_id(Some(&list), "x"), None);
    assert_eq!(resolve_project_search_result_id(Some(&list), ""), None);
    assert_eq!(resolve_project_search_result_id(Some(&list), "-1"), None);
}

#[test]
fn result_none_when_out_of_bounds_parity() {
    let list = ids();
    assert_eq!(resolve_project_search_result_id(Some(&list), "2"), None);
    assert_eq!(resolve_project_search_result_id(Some(&list), "99"), None);
}

// ---- flow state ----

#[test]
fn flow_state_none_input_is_none_parity() {
    assert_eq!(project_search_flow_state(None), None);
}

#[test]
fn flow_state_non_custom_kind_is_none_parity() {
    let mut s = snapshot("inline", Some("project-search"), Some("results"), Some(7));
    assert_eq!(project_search_flow_state(Some(&s)), None);
    s.kind = "rename".to_string();
    assert_eq!(project_search_flow_state(Some(&s)), None);
}

#[test]
fn flow_state_wrong_flow_is_none_parity() {
    let s = snapshot("custom", Some("session-search"), Some("results"), Some(7));
    assert_eq!(project_search_flow_state(Some(&s)), None);
    let s = snapshot("custom", None, Some("results"), Some(7));
    assert_eq!(project_search_flow_state(Some(&s)), None);
}

#[test]
fn flow_state_valid_returns_stage_and_token_parity() {
    let s = snapshot("custom", Some("project-search"), Some("results"), Some(7));
    let out = project_search_flow_state(Some(&s)).unwrap();
    assert_eq!(
        out,
        ProjectSearchFlow {
            stage: "results".to_string(),
            token: 7,
        }
    );
}

// ---- stage gates ----

#[test]
fn search_stages_parity() {
    assert!(is_project_search_results_stage(Some("results")));
    assert!(!is_project_search_results_stage(Some("input")));
    assert!(!is_project_search_results_stage(None));
}

#[test]
fn search_input_stage_parity() {
    assert!(is_project_search_input_stage(Some("input")));
    assert!(!is_project_search_input_stage(Some("results")));
    assert!(!is_project_search_input_stage(None));
}

// ---- same-search gate ----

#[test]
fn same_search_true_parity() {
    assert!(is_same_project_search(
        Some(7),
        Some("results"),
        Some(7),
        Some("results")
    ));
}

#[test]
fn same_search_false_on_token_parity() {
    assert!(!is_same_project_search(
        Some(7),
        Some("results"),
        Some(8),
        Some("results")
    ));
}

#[test]
fn same_search_false_when_stage_or_token_missing_parity() {
    assert!(!is_same_project_search(
        Some(7),
        Some("results"),
        Some(7),
        Some("input")
    ));
    assert!(!is_same_project_search(
        None,
        Some("results"),
        Some(7),
        Some("results")
    ));
    assert!(!is_same_project_search(
        Some(7),
        Some("results"),
        None,
        Some("results")
    ));
}

// ---- message mismatch ----

#[test]
fn mismatch_when_state_message_missing_parity() {
    assert!(is_project_search_message_mismatch(None, Some(11)));
    assert!(is_project_search_message_mismatch(None, None));
}

#[test]
fn mismatch_on_differing_ids_parity() {
    assert!(is_project_search_message_mismatch(Some(11), Some(12)));
}

#[test]
fn no_mismatch_on_equal_ids_parity() {
    assert!(!is_project_search_message_mismatch(Some(11), Some(11)));
}

// ---- combined inactive gate ----

#[test]
fn inactive_when_no_state_parity() {
    assert!(should_answer_project_search_inactive(false, true, true));
}

#[test]
fn inactive_when_stage_or_message_bad_parity() {
    assert!(should_answer_project_search_inactive(true, false, true));
    assert!(should_answer_project_search_inactive(true, true, false));
}

#[test]
fn active_when_all_good_parity() {
    assert!(!should_answer_project_search_inactive(true, true, true));
}

// ---- blank query + text input ----

#[test]
fn blank_query_true_parity() {
    assert!(is_blank_project_search_query(""));
    assert!(is_blank_project_search_query("   "));
    assert!(is_blank_project_search_query("\t\n "));
}

#[test]
fn blank_query_false_parity() {
    assert!(!is_blank_project_search_query("bot"));
    assert!(!is_blank_project_search_query(" bot "));
}

#[test]
fn text_input_handled_when_ready_parity() {
    assert!(handles_project_search_text_input(true, true, true));
}

#[test]
fn text_input_ignored_parity() {
    assert!(!handles_project_search_text_input(false, true, true));
    assert!(!handles_project_search_text_input(true, false, true));
    assert!(!handles_project_search_text_input(true, true, false));
}

// ---- project-select dispatch + id ----

#[test]
fn select_handles_page_callback_parity() {
    assert!(handles_project_select(Some(0), Some("projects:page:0")));
}

#[test]
fn select_handles_project_id_parity() {
    assert!(handles_project_select(None, Some("project:abc")));
}

#[test]
fn select_rejects_non_selection_parity() {
    assert!(!handles_project_select(None, None));
    assert!(!handles_project_select(None, Some("")));
    assert!(!handles_project_select(None, Some("projects:search")));
    assert!(!handles_project_select(None, Some("projects:page:0")));
}

#[test]
fn select_extracts_id_parity() {
    assert_eq!(
        parse_project_select_id("project:abc"),
        Some("abc".to_string())
    );
}

#[test]
fn select_extract_rejects_bad_shapes_parity() {
    assert_eq!(parse_project_select_id("project:"), None);
    assert_eq!(parse_project_select_id("projects:page:0"), None);
    assert_eq!(parse_project_select_id("other:abc"), None);
}

// ---- busy block ----

#[test]
fn busy_blocks_callback_parity() {
    assert!(should_block_project_callback_when_busy(true));
    assert!(!should_block_project_callback_when_busy(false));
}
