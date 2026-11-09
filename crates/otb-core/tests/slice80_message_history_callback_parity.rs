//! Slice 80 parity: message-history callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/message-history-callback-handler.ts`
//! (`parseMessages`, `parseMessagesMetadata`, `messages:` prefix,
//! exact revert/fork/back/cancel, message-id mismatch, `detail`
//! revert/fork/back gates, selected-message-exists gate, page-empty
//! gate, `list`-stage select gate, foreground-busy block, clear gate).
//! Menu-level `MESSAGES_CALLBACK_*` constants and
//! `parseMessagePageCallback` / `parseMessageSelectCallback` already
//! live in `worktree_history_inline_gates` and are NOT re-tested here.
//!
//! Every test MUST FAIL with `unimplemented!()` panics until
//! `message_history_callback_gates` is ported.

use otb_core::message_history_callback_gates::*;

fn raw(id: Option<&str>, text: Option<&str>, created: Option<f64>) -> RawMessageItem {
    RawMessageItem {
        id: id.map(str::to_string),
        text: text.map(str::to_string),
        created,
    }
}

fn snapshot(
    kind: &str,
    flow: Option<&str>,
    stage: Option<&str>,
    message_id: Option<i64>,
    project: Option<&str>,
    session_id: Option<&str>,
    messages: Option<Vec<RawMessageItem>>,
    page: Option<f64>,
    selected_index: Option<f64>,
) -> InteractionSnapshot {
    InteractionSnapshot {
        kind: kind.to_string(),
        flow: flow.map(str::to_string),
        stage: stage.map(str::to_string),
        message_id,
        project_directory: project.map(str::to_string),
        session_id: session_id.map(str::to_string),
        messages,
        page,
        selected_index,
    }
}

fn valid_messages() -> Vec<RawMessageItem> {
    vec![
        raw(Some("m1"), Some("hello"), Some(1700000000.0)),
        raw(Some("m2"), Some("world"), Some(1700000001.0)),
    ]
}

fn valid_snapshot(stage: &str) -> InteractionSnapshot {
    snapshot(
        "custom",
        Some("messages"),
        Some(stage),
        Some(11),
        Some("/repo"),
        Some("sess-1"),
        Some(valid_messages()),
        Some(0.0),
        if stage == "detail" { Some(1.0) } else { None },
    )
}

// ---- parse_messages ----

#[test]
fn parse_messages_valid_single_item_parity() {
    let out = parse_messages(&[raw(Some("m1"), Some("hi"), Some(7.0))]).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, "m1");
    assert_eq!(out[0].text, "hi");
    assert_eq!(out[0].created, 7.0);
}

#[test]
fn parse_messages_valid_multiple_items_parity() {
    let out = parse_messages(&valid_messages()).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[1].id, "m2");
}

#[test]
fn parse_messages_rejects_non_string_id_parity() {
    assert_eq!(parse_messages(&[raw(None, Some("hi"), Some(7.0))]), None);
}

#[test]
fn parse_messages_rejects_non_string_text_parity() {
    assert_eq!(parse_messages(&[raw(Some("m1"), None, Some(7.0))]), None);
}

#[test]
fn parse_messages_rejects_non_number_created_parity() {
    assert_eq!(parse_messages(&[raw(Some("m1"), Some("hi"), None)]), None);
}

#[test]
fn parse_messages_rejects_when_any_item_invalid_parity() {
    let items = vec![
        raw(Some("m1"), Some("ok"), Some(1.0)),
        raw(Some("m2"), None, Some(2.0)),
    ];
    assert_eq!(parse_messages(&items), None);
}

#[test]
fn parse_messages_empty_array_is_some_empty_parity() {
    assert_eq!(parse_messages(&[]), Some(vec![]));
}

#[test]
fn parse_messages_rejects_non_object_item_parity() {
    assert_eq!(parse_messages(&[raw(None, None, None)]), None);
}

// ---- parse_messages_metadata ----

#[test]
fn parse_metadata_none_state_is_none_parity() {
    assert_eq!(parse_messages_metadata(None), None);
}

#[test]
fn parse_metadata_non_custom_kind_is_none_parity() {
    let s = valid_snapshot("list");
    let mut bad = s.clone();
    bad.kind = "rename".to_string();
    assert_eq!(parse_messages_metadata(Some(&bad)), None);
}

#[test]
fn parse_metadata_wrong_flow_is_none_parity() {
    let mut s = valid_snapshot("list");
    s.flow = Some("recent".to_string());
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_missing_message_id_is_none_parity() {
    let mut s = valid_snapshot("list");
    s.message_id = None;
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_missing_project_is_none_parity() {
    let mut s = valid_snapshot("list");
    s.project_directory = None;
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_missing_session_id_is_none_parity() {
    let mut s = valid_snapshot("list");
    s.session_id = None;
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_invalid_messages_is_none_parity() {
    let mut s = valid_snapshot("list");
    s.messages = Some(vec![raw(None, Some("hi"), Some(1.0))]);
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_missing_messages_is_none_parity() {
    let mut s = valid_snapshot("list");
    s.messages = None;
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_list_stage_parses_with_page_parity() {
    let mut s = valid_snapshot("list");
    s.page = Some(2.0);
    let out = parse_messages_metadata(Some(&s)).unwrap();
    match out {
        MessagesMetadata::List(list) => {
            assert_eq!(list.message_id, 11);
            assert_eq!(list.project_directory, "/repo");
            assert_eq!(list.session_id, "sess-1");
            assert_eq!(list.messages.len(), 2);
            assert_eq!(list.page, 2);
        }
        other => panic!("expected list, got {other:?}"),
    }
}

#[test]
fn parse_metadata_negative_page_clamps_to_zero_parity() {
    let mut s = valid_snapshot("list");
    s.page = Some(-3.0);
    let out = parse_messages_metadata(Some(&s)).unwrap();
    match out {
        MessagesMetadata::List(list) => assert_eq!(list.page, 0),
        other => panic!("expected list, got {other:?}"),
    }
}

#[test]
fn parse_metadata_non_integer_page_defaults_zero_parity() {
    let mut s = valid_snapshot("list");
    s.page = Some(1.5);
    let out = parse_messages_metadata(Some(&s)).unwrap();
    match out {
        MessagesMetadata::List(list) => assert_eq!(list.page, 0),
        other => panic!("expected list, got {other:?}"),
    }
}

#[test]
fn parse_metadata_missing_page_defaults_zero_parity() {
    let mut s = valid_snapshot("list");
    s.page = None;
    let out = parse_messages_metadata(Some(&s)).unwrap();
    match out {
        MessagesMetadata::List(list) => assert_eq!(list.page, 0),
        other => panic!("expected list, got {other:?}"),
    }
}

#[test]
fn parse_metadata_detail_stage_parses_selected_index_parity() {
    let s = valid_snapshot("detail");
    let out = parse_messages_metadata(Some(&s)).unwrap();
    match out {
        MessagesMetadata::Detail(detail) => {
            assert_eq!(detail.selected_index, 1);
            assert_eq!(detail.page, 0);
            assert_eq!(detail.messages.len(), 2);
        }
        other => panic!("expected detail, got {other:?}"),
    }
}

#[test]
fn parse_metadata_detail_rejects_negative_selected_index_parity() {
    let mut s = valid_snapshot("detail");
    s.selected_index = Some(-1.0);
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_detail_rejects_non_integer_selected_index_parity() {
    let mut s = valid_snapshot("detail");
    s.selected_index = Some(1.5);
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_detail_rejects_missing_selected_index_parity() {
    let mut s = valid_snapshot("detail");
    s.selected_index = None;
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

#[test]
fn parse_metadata_unknown_stage_is_none_parity() {
    let s = valid_snapshot("confirm");
    assert_eq!(parse_messages_metadata(Some(&s)), None);
}

// ---- prefix + exact matches ----

#[test]
fn handles_messages_prefix_parity() {
    assert!(handles_messages_callback(Some("messages:revert")));
    assert!(handles_messages_callback(Some("messages:")));
}

#[test]
fn handles_messages_rejects_none_and_empty_parity() {
    assert!(!handles_messages_callback(None));
    assert!(!handles_messages_callback(Some("")));
}

#[test]
fn handles_messages_rejects_other_prefix_parity() {
    assert!(!handles_messages_callback(Some("recent:select:0")));
    assert!(!handles_messages_callback(Some("mcps:toggle")));
}

#[test]
fn exact_revert_match_parity() {
    assert!(is_messages_revert("messages:revert"));
    assert!(!is_messages_revert("messages:fork"));
    assert!(!is_messages_revert("messages:revert:x"));
}

#[test]
fn exact_fork_match_parity() {
    assert!(is_messages_fork("messages:fork"));
    assert!(!is_messages_fork("messages:revert"));
    assert!(!is_messages_fork("messages:fork:x"));
}

#[test]
fn exact_back_match_parity() {
    assert!(is_messages_back("messages:back"));
    assert!(!is_messages_back("messages:cancel"));
}

#[test]
fn exact_cancel_match_parity() {
    assert!(is_messages_cancel("messages:cancel"));
    assert!(!is_messages_cancel("messages:back"));
}

// ---- mismatch guard ----

#[test]
fn mismatch_when_metadata_missing_parity() {
    assert!(is_callback_message_mismatch(None, Some(11)));
}

#[test]
fn mismatch_when_callback_id_missing_parity() {
    assert!(is_callback_message_mismatch(Some(11), None));
    assert!(is_callback_message_mismatch(None, None));
}

#[test]
fn mismatch_on_differing_ids_parity() {
    assert!(is_callback_message_mismatch(Some(11), Some(12)));
}

#[test]
fn no_mismatch_on_equal_ids_parity() {
    assert!(!is_callback_message_mismatch(Some(11), Some(11)));
}

// ---- stage gates ----

#[test]
fn detail_stage_true_parity() {
    assert!(is_detail_stage(Some("detail")));
}

#[test]
fn detail_stage_false_for_list_and_none_parity() {
    assert!(!is_detail_stage(Some("list")));
    assert!(!is_detail_stage(None));
}

#[test]
fn list_stage_true_parity() {
    assert!(is_list_stage(Some("list")));
}

#[test]
fn list_stage_false_for_detail_and_none_parity() {
    assert!(!is_list_stage(Some("detail")));
    assert!(!is_list_stage(None));
}

// ---- page-empty gate ----

#[test]
fn page_empty_when_page_beyond_total_parity() {
    assert!(is_messages_page_empty(3, 3));
    assert!(is_messages_page_empty(9, 3));
}

#[test]
fn page_empty_when_negative_parity() {
    assert!(is_messages_page_empty(-1, 4));
}

#[test]
fn page_not_empty_inside_range_parity() {
    assert!(!is_messages_page_empty(0, 3));
    assert!(!is_messages_page_empty(2, 3));
}

// ---- select + selected-message gates ----

#[test]
fn select_valid_in_list_stage_parity() {
    assert!(is_message_select_valid(0, 3, true));
    assert!(is_message_select_valid(2, 3, true));
}

#[test]
fn select_invalid_when_not_list_stage_parity() {
    assert!(!is_message_select_valid(0, 3, false));
}

#[test]
fn select_invalid_when_negative_parity() {
    assert!(!is_message_select_valid(-1, 3, true));
}

#[test]
fn select_invalid_when_out_of_bounds_parity() {
    assert!(!is_message_select_valid(3, 3, true));
}

#[test]
fn has_selected_message_true_parity() {
    assert!(has_selected_message(2, 0));
    assert!(has_selected_message(2, 1));
}

#[test]
fn selected_message_missing_when_index_out_of_bounds_parity() {
    assert!(!has_selected_message(2, 2));
    assert!(!has_selected_message(2, -1));
    assert!(!has_selected_message(0, 0));
}

// ---- busy + clear gates ----

#[test]
fn busy_blocks_callback_parity() {
    assert!(should_block_when_busy(true));
    assert!(!should_block_when_busy(false));
}

#[test]
fn clear_runs_with_metadata_parity() {
    assert!(should_clear_messages_interaction(true));
    assert!(!should_clear_messages_interaction(false));
}
