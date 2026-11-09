//! Slice 87 parity: MCP catalog callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/mcp-catalog-callback-handler.ts` (the `mcps:`
//! prefix dispatch guard; the exact `mcps:cancel` / `mcps:back` /
//! `mcps:toggle` matches; the `mcps:select:` prefix branch with index
//! bounds; `parseMcpsMetadata`: `custom` kind + `mcps` flow + numeric
//! `messageId` + string `projectDirectory` + valid servers, `list` vs
//! `detail`-with-non-blank-`serverName`; the metadata/callback
//! message-id mismatch guard; the `back`/`toggle`-require-`detail` and
//! `select`-requires-`list` stage guards; the toggle server-by-name
//! lookup; the `enable = status !== "connected"` gate; the
//! updated-server-missing list fallback; the `callback.unknown_command`
//! fallback; the `mcps.toggle_error` catch-all).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `mcp_catalog_callback_gates` is ported (only the constant-value tests
//! pass).

use otb_core::mcp_catalog_callback_gates::*;

fn raw_state(
    kind_is_custom: bool,
    flow: &str,
    stage: &str,
    message_id: Option<i64>,
    project_directory: Option<&str>,
    servers_valid: bool,
    server_count: usize,
    server_name: Option<&str>,
) -> RawMcpsState {
    RawMcpsState {
        kind_is_custom,
        flow: flow.to_string(),
        stage: stage.to_string(),
        message_id,
        project_directory: project_directory.map(|s| s.to_string()),
        servers_valid,
        server_count,
        server_name: server_name.map(|s| s.to_string()),
    }
}

// ---- constants ----

#[test]
fn mcps_prefix_value_parity() {
    assert_eq!(MCPS_CALLBACK_PREFIX, "mcps:");
}

#[test]
fn mcps_select_prefix_value_parity() {
    assert_eq!(MCPS_CALLBACK_SELECT_PREFIX, "mcps:select:");
}

#[test]
fn mcps_toggle_value_parity() {
    assert_eq!(MCPS_CALLBACK_TOGGLE, "mcps:toggle");
}

#[test]
fn mcps_back_value_parity() {
    assert_eq!(MCPS_CALLBACK_BACK, "mcps:back");
}

#[test]
fn mcps_cancel_value_parity() {
    assert_eq!(MCPS_CALLBACK_CANCEL, "mcps:cancel");
}

#[test]
fn mcps_inactive_key_value_parity() {
    assert_eq!(MCPS_INACTIVE_KEY, "inline.inactive_callback");
}

#[test]
fn mcps_processing_error_key_value_parity() {
    assert_eq!(MCPS_PROCESSING_ERROR_KEY, "callback.processing_error");
}

#[test]
fn mcps_toggle_error_key_value_parity() {
    assert_eq!(MCPS_TOGGLE_ERROR_KEY, "mcps.toggle_error");
}

// ---- prefix dispatch guard ----

#[test]
fn handles_mcps_callback_none_parity() {
    assert!(!handles_mcps_callback(None));
}

#[test]
fn handles_mcps_callback_empty_parity() {
    assert!(!handles_mcps_callback(Some("")));
}

#[test]
fn handles_mcps_callback_exact_toggle_parity() {
    assert!(handles_mcps_callback(Some("mcps:toggle")));
}

#[test]
fn handles_mcps_callback_rejects_other_prefix_parity() {
    assert!(!handles_mcps_callback(Some("model:toggle")));
    assert!(!handles_mcps_callback(Some("mcps")));
}

#[test]
fn handles_mcps_callback_rejects_case_variant_parity() {
    assert!(!handles_mcps_callback(Some("Mcps:toggle")));
}

// ---- dispatch classifier ----

#[test]
fn classify_cancel_parity() {
    assert_eq!(
        classify_mcps_callback_action("mcps:cancel"),
        McpsCallbackAction::Cancel
    );
}

#[test]
fn classify_back_parity() {
    assert_eq!(
        classify_mcps_callback_action("mcps:back"),
        McpsCallbackAction::Back
    );
}

#[test]
fn classify_toggle_parity() {
    assert_eq!(
        classify_mcps_callback_action("mcps:toggle"),
        McpsCallbackAction::Toggle
    );
}

#[test]
fn classify_select_index_parity() {
    assert_eq!(
        classify_mcps_callback_action("mcps:select:0"),
        McpsCallbackAction::Select
    );
}

#[test]
fn classify_select_bad_index_still_select_parity() {
    assert_eq!(
        classify_mcps_callback_action("mcps:select:abc"),
        McpsCallbackAction::Select
    );
}

#[test]
fn classify_unknown_suffix_parity() {
    assert_eq!(
        classify_mcps_callback_action("mcps:nope"),
        McpsCallbackAction::Unknown
    );
}

#[test]
fn classify_missing_prefix_parity() {
    assert_eq!(
        classify_mcps_callback_action("other:cancel"),
        McpsCallbackAction::Unknown
    );
}

// ---- metadata parser ----

#[test]
fn metadata_rejects_non_custom_kind_parity() {
    let state = raw_state(false, "mcps", "list", Some(7), Some("/repo"), true, 2, None);
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_rejects_wrong_flow_parity() {
    let state = raw_state(
        true,
        "commands",
        "list",
        Some(7),
        Some("/repo"),
        true,
        2,
        None,
    );
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_rejects_missing_message_id_parity() {
    let state = raw_state(true, "mcps", "list", None, Some("/repo"), true, 2, None);
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_rejects_missing_project_directory_parity() {
    let state = raw_state(true, "mcps", "list", Some(7), None, true, 2, None);
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_rejects_invalid_servers_parity() {
    let state = raw_state(true, "mcps", "list", Some(7), Some("/repo"), false, 0, None);
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_accepts_list_stage_parity() {
    let state = raw_state(true, "mcps", "list", Some(7), Some("/repo"), true, 2, None);
    assert_eq!(
        parse_mcps_metadata(&state),
        Some(McpsMetadata {
            message_id: 7,
            project_directory: "/repo".to_string(),
            stage: McpsStage::List,
            server_name: None,
            server_count: 2,
        })
    );
}

#[test]
fn metadata_accepts_detail_stage_parity() {
    let state = raw_state(
        true,
        "mcps",
        "detail",
        Some(7),
        Some("/repo"),
        true,
        2,
        Some("fetch"),
    );
    assert_eq!(
        parse_mcps_metadata(&state),
        Some(McpsMetadata {
            message_id: 7,
            project_directory: "/repo".to_string(),
            stage: McpsStage::Detail,
            server_name: Some("fetch".to_string()),
            server_count: 2,
        })
    );
}

#[test]
fn metadata_rejects_detail_missing_server_name_parity() {
    let state = raw_state(
        true,
        "mcps",
        "detail",
        Some(7),
        Some("/repo"),
        true,
        2,
        None,
    );
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_rejects_detail_blank_server_name_parity() {
    let state = raw_state(
        true,
        "mcps",
        "detail",
        Some(7),
        Some("/repo"),
        true,
        2,
        Some("   "),
    );
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_rejects_unknown_stage_parity() {
    let state = raw_state(
        true,
        "mcps",
        "confirm",
        Some(7),
        Some("/repo"),
        true,
        2,
        None,
    );
    assert_eq!(parse_mcps_metadata(&state), None);
}

#[test]
fn metadata_accepts_empty_project_directory_parity() {
    // TS only checks `typeof projectDirectory === "string"`; empty passes.
    let state = raw_state(true, "mcps", "list", Some(7), Some(""), true, 0, None);
    assert!(parse_mcps_metadata(&state).is_some());
}

// ---- message-id mismatch guard ----

#[test]
fn mismatch_both_missing_counts_parity() {
    assert!(is_mcps_message_mismatch(None, None));
}

#[test]
fn mismatch_equal_ids_clear_parity() {
    assert!(!is_mcps_message_mismatch(Some(7), Some(7)));
}

#[test]
fn mismatch_different_ids_blocks_parity() {
    assert!(is_mcps_message_mismatch(Some(7), Some(8)));
    assert!(is_mcps_message_mismatch(Some(7), None));
}

// ---- stage guards ----

#[test]
fn back_requires_detail_stage_parity() {
    assert!(!mcps_stage_allows(
        McpsCallbackAction::Back,
        McpsStage::List
    ));
    assert!(mcps_stage_allows(
        McpsCallbackAction::Back,
        McpsStage::Detail
    ));
}

#[test]
fn toggle_requires_detail_stage_parity() {
    assert!(!mcps_stage_allows(
        McpsCallbackAction::Toggle,
        McpsStage::List
    ));
    assert!(mcps_stage_allows(
        McpsCallbackAction::Toggle,
        McpsStage::Detail
    ));
}

#[test]
fn select_requires_list_stage_parity() {
    assert!(mcps_stage_allows(
        McpsCallbackAction::Select,
        McpsStage::List
    ));
    assert!(!mcps_stage_allows(
        McpsCallbackAction::Select,
        McpsStage::Detail
    ));
}

#[test]
fn cancel_is_stage_free_parity() {
    assert!(mcps_stage_allows(
        McpsCallbackAction::Cancel,
        McpsStage::List
    ));
    assert!(mcps_stage_allows(
        McpsCallbackAction::Cancel,
        McpsStage::Detail
    ));
}

// ---- server lookup / enable / fallback ----

#[test]
fn resolve_server_by_name_hit_parity() {
    assert_eq!(
        resolve_mcps_server_by_name(&["fetch", "grep"], "grep"),
        Some(1)
    );
}

#[test]
fn resolve_server_by_name_miss_parity() {
    assert_eq!(
        resolve_mcps_server_by_name(&["fetch", "grep"], "missing"),
        None
    );
}

#[test]
fn resolve_select_index_bounds_parity() {
    assert_eq!(resolve_mcps_select_server(2, Some(0)), Some(0));
    assert_eq!(resolve_mcps_select_server(2, Some(2)), None);
    assert_eq!(resolve_mcps_select_server(2, None), None);
}

#[test]
fn should_enable_non_connected_is_true_parity() {
    assert!(should_enable_mcp_server("disabled"));
    assert!(should_enable_mcp_server("failed"));
}

#[test]
fn should_enable_connected_is_false_parity() {
    assert!(!should_enable_mcp_server("connected"));
}

#[test]
fn fallback_to_list_when_updated_missing_parity() {
    assert!(should_fall_back_to_mcp_list(false));
    assert!(!should_fall_back_to_mcp_list(true));
}

// ---- answer keys ----

#[test]
fn answer_key_cancel_parity() {
    assert_eq!(
        mcps_answer_key(McpsCallbackAction::Cancel, false),
        Some("common.cancelled")
    );
}

#[test]
fn answer_key_toggle_enabling_parity() {
    assert_eq!(
        mcps_answer_key(McpsCallbackAction::Toggle, true),
        Some("mcps.enabling")
    );
}

#[test]
fn answer_key_toggle_disabling_parity() {
    assert_eq!(
        mcps_answer_key(McpsCallbackAction::Toggle, false),
        Some("mcps.disabling")
    );
}

#[test]
fn answer_key_back_select_empty_parity() {
    assert_eq!(mcps_answer_key(McpsCallbackAction::Back, false), None);
    assert_eq!(mcps_answer_key(McpsCallbackAction::Select, false), None);
}

#[test]
fn answer_key_unknown_command_parity() {
    assert_eq!(
        mcps_answer_key(McpsCallbackAction::Unknown, false),
        Some("callback.unknown_command")
    );
}
