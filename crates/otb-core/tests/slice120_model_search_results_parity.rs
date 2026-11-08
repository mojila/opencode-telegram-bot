//! Slice 120 parity: model search results view + metadata gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/model-selection-callback-handler.ts`:
//! - `MODEL_SEARCH_RESULT_CALLBACK_PREFIX` + result callback/label rows
//! - results text key (`no_results` when empty else `results_title`)
//! - search metadata eligibility (`custom` kind + `model-search` flow +
//!   string stage), query `""` fallback
//! - results-stage stale-message gate (`meta.messageId !== undefined &&
//!   callbackMessageId !== meta.messageId`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! search/input routing (`task_model_text_input_gates`), list/legacy
//! parsing and results dispatch routing (`agent_model_variant_gates`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `model_search_results_gates` is ported (only constant-value tests pass).

use otb_core::model_search_results_gates::*;

// ---- constants (4) ----

#[test]
fn search_prefix_value_parity() {
    assert_eq!(MODEL_SEARCH_RESULT_CALLBACK_PREFIX, "model:result:");
}

#[test]
fn search_flow_stage_value_parity() {
    assert_eq!(MODEL_SEARCH_FLOW, "model-search");
    assert_eq!(MODEL_SEARCH_RESULTS_STAGE, "results");
    assert_eq!(MODEL_SEARCH_INPUT_STAGE, "input");
}

#[test]
fn search_text_keys_value_parity() {
    assert_eq!(MODEL_SEARCH_RESULTS_TITLE_KEY, "model.search.results_title");
    assert_eq!(MODEL_SEARCH_NO_RESULTS_KEY, "model.search.no_results");
    assert_eq!(MODEL_SEARCH_PROMPT_KEY, "model.search.prompt");
    assert_eq!(MODEL_SEARCH_ERROR_KEY, "model.search.error");
}

#[test]
fn search_button_keys_value_parity() {
    assert_eq!(MODEL_SEARCH_AGAIN_KEY, "model.search.search_again");
    assert_eq!(INLINE_CANCEL_KEY, "inline.button.cancel");
}

// ---- view builders (3) ----

#[test]
fn result_callback_build_parity() {
    assert_eq!(build_search_result_callback(0), "model:result:0");
    assert_eq!(build_search_result_callback(3), "model:result:3");
}

#[test]
fn result_label_parity() {
    assert_eq!(search_result_label("openai", "gpt-4o"), "openai/gpt-4o");
}

#[test]
fn results_text_key_parity() {
    assert_eq!(search_results_text_key(true), "model.search.no_results");
    assert_eq!(search_results_text_key(false), "model.search.results_title");
}

// ---- result index (2) ----

#[test]
fn is_search_result_callback_parity() {
    assert!(is_search_result_callback("model:result:2"));
    assert!(!is_search_result_callback("model:list:favorites:0"));
    assert!(!is_search_result_callback("model:search:again"));
}

#[test]
fn parses_search_result_index_parity() {
    assert_eq!(parse_search_result_index("model:result:2"), Some(2));
    assert_eq!(parse_search_result_index("model:result:0"), Some(0));
    assert_eq!(parse_search_result_index("model:result:"), None);
    assert_eq!(parse_search_result_index("model:result:abc"), None);
    assert_eq!(parse_search_result_index("model:list:favorites:0"), None);
}

// ---- metadata eligibility (2) ----

#[test]
fn search_metadata_eligibility_parity() {
    assert!(is_search_metadata_eligible("custom", Some("model-search"), true));
    assert!(!is_search_metadata_eligible("inline", Some("model-search"), true));
    assert!(!is_search_metadata_eligible("custom", Some("other"), true));
    assert!(!is_search_metadata_eligible("custom", None, true));
    assert!(!is_search_metadata_eligible(
        "custom",
        Some("model-search"),
        false
    ));
}

#[test]
fn query_fallback_parity() {
    assert_eq!(normalize_search_query(Some("gpt")), "gpt");
    assert_eq!(normalize_search_query(None), "");
}

// ---- stale-message gate (2) ----

#[test]
fn message_id_match_parity() {
    assert!(search_message_id_matches(Some(10), Some(10)));
    assert!(!search_message_id_matches(Some(10), Some(11)));
    assert!(search_message_id_matches(None, Some(11)));
    assert!(search_message_id_matches(None, None));
}

#[test]
fn stale_callback_parity() {
    assert!(!is_stale_search_callback(Some(10), Some(10)));
    assert!(is_stale_search_callback(Some(10), Some(11)));
    assert!(is_stale_search_callback(Some(10), None));
    assert!(!is_stale_search_callback(None, Some(11)));
    assert!(!is_stale_search_callback(None, None));
}
