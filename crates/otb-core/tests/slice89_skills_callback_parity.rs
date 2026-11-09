//! Slice 89 parity: skills catalog callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/skills-callback-handler.ts` (the `skills:`
//! prefix dispatch guard; the cancel vs execute vs page vs select
//! dispatch branch; the null/mismatched message-id stale gate
//! `skills.inactive_callback`; the page-branch `stage !== "list"`
//! `callback.processing_error` gate; the `page >= totalPages`
//! `skills.page_empty_callback` gate; the `skillIndex === null ||
//! stage !== "list"` `callback.processing_error` gate; the
//! missing-skill `skills.inactive_callback` gate; the execute-branch
//! `stage !== "confirm"` `skills.inactive_callback` gate; the
//! `/<skill> <args>` prompt-text build; the `skills.execute_callback`
//! toast key and trailing `callback.processing_error` catch-all) and
//! `src/bot/menus/skills-catalog-menu.ts` (the `skills:`,
//! `skills:select:`, `skills:page:` callback constants plus
//! `skills:cancel` / `skills:execute`; the page/select parses and
//! builders).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `skills_callback_gates` is ported (only the constant-value tests
//! pass).

use otb_core::skills_callback_gates::*;

// ---- constants ----

#[test]
fn skills_cb_prefix_value_parity() {
    assert_eq!(SKILLS_CB_PREFIX, "skills:");
}

#[test]
fn skills_cb_select_prefix_value_parity() {
    assert_eq!(SKILLS_CB_SELECT_PREFIX, "skills:select:");
}

#[test]
fn skills_cb_page_prefix_value_parity() {
    assert_eq!(SKILLS_CB_PAGE_PREFIX, "skills:page:");
}

#[test]
fn skills_cb_cancel_value_parity() {
    assert_eq!(SKILLS_CB_CANCEL, "skills:cancel");
}

#[test]
fn skills_cb_execute_value_parity() {
    assert_eq!(SKILLS_CB_EXECUTE, "skills:execute");
}

#[test]
fn skills_inactive_key_value_parity() {
    assert_eq!(SKILLS_INACTIVE_KEY, "skills.inactive_callback");
}

#[test]
fn skills_execute_key_value_parity() {
    assert_eq!(SKILLS_EXECUTE_KEY, "skills.execute_callback");
}

#[test]
fn skills_page_empty_key_value_parity() {
    assert_eq!(SKILLS_PAGE_EMPTY_KEY, "skills.page_empty_callback");
}

#[test]
fn skills_processing_error_key_value_parity() {
    assert_eq!(SKILLS_PROCESSING_ERROR_KEY, "callback.processing_error");
}

#[test]
fn skills_project_missing_key_value_parity() {
    assert_eq!(SKILLS_PROJECT_MISSING_KEY, "bot.project_not_selected");
}

#[test]
fn skills_stage_list_value_parity() {
    assert_eq!(SKILLS_STAGE_LIST, "list");
}

// ---- prefix dispatch guard ----

#[test]
fn handles_skills_callback_none_parity() {
    assert!(!handles_skills_callback(None));
}

#[test]
fn handles_skills_callback_empty_parity() {
    assert!(!handles_skills_callback(Some("")));
}

#[test]
fn handles_skills_callback_cancel_parity() {
    assert!(handles_skills_callback(Some("skills:cancel")));
}

#[test]
fn handles_skills_callback_select_parity() {
    assert!(handles_skills_callback(Some("skills:select:2")));
}

#[test]
fn handles_skills_callback_rejects_other_parity() {
    assert!(!handles_skills_callback(Some("session:2")));
    assert!(!handles_skills_callback(Some("skills")));
    assert!(!handles_skills_callback(Some("Skills:cancel")));
}

// ---- dispatch classifier ----

#[test]
fn classify_cancel_parity() {
    assert_eq!(
        classify_skills_callback_action("skills:cancel"),
        SkillsCallbackAction::Cancel
    );
}

#[test]
fn classify_execute_parity() {
    assert_eq!(
        classify_skills_callback_action("skills:execute"),
        SkillsCallbackAction::Execute
    );
}

#[test]
fn classify_page_parity() {
    assert_eq!(
        classify_skills_callback_action("skills:page:1"),
        SkillsCallbackAction::Page
    );
}

#[test]
fn classify_select_parity() {
    assert_eq!(
        classify_skills_callback_action("skills:select:0"),
        SkillsCallbackAction::Select
    );
}

#[test]
fn classify_unknown_bare_page_parity() {
    assert_eq!(
        classify_skills_callback_action("skills:page"),
        SkillsCallbackAction::Unknown
    );
}

#[test]
fn classify_unknown_suffix_parity() {
    assert_eq!(
        classify_skills_callback_action("skills:bogus"),
        SkillsCallbackAction::Unknown
    );
}

// ---- page parse ----

#[test]
fn parse_page_accepts_digits_parity() {
    assert_eq!(parse_skills_page_value("skills:page:3"), Some(3));
}

#[test]
fn parse_page_rejects_non_digits_parity() {
    assert_eq!(parse_skills_page_value("skills:page:3x"), None);
    assert_eq!(parse_skills_page_value("skills:page:-1"), None);
}

#[test]
fn parse_page_rejects_empty_parity() {
    assert_eq!(parse_skills_page_value("skills:page:"), None);
}

#[test]
fn parse_page_rejects_non_page_payload_parity() {
    assert_eq!(parse_skills_page_value("skills:select:1"), None);
    assert_eq!(parse_skills_page_value("session:page:1"), None);
}

// ---- select parse ----

#[test]
fn parse_select_accepts_digits_parity() {
    assert_eq!(parse_skills_select_index("skills:select:2"), Some(2));
}

#[test]
fn parse_select_rejects_page_prefix_parity() {
    assert_eq!(parse_skills_select_index("skills:page:1"), None);
}

#[test]
fn parse_select_rejects_non_digit_parity() {
    assert_eq!(parse_skills_select_index("skills:select:abc"), None);
    assert_eq!(parse_skills_select_index("skills:select:-1"), None);
}

#[test]
fn parse_select_rejects_empty_and_other_parity() {
    assert_eq!(parse_skills_select_index("skills:select:"), None);
    assert_eq!(parse_skills_select_index("session:select:1"), None);
}

// ---- builders + prompt text ----

#[test]
fn build_page_callback_parity() {
    assert_eq!(build_skills_page_callback(2), "skills:page:2");
}

#[test]
fn build_select_callback_parity() {
    assert_eq!(build_skills_select_callback(4), "skills:select:4");
}

#[test]
fn build_prompt_text_with_args_parity() {
    assert_eq!(
        build_skills_prompt_text("deploy", "prod now"),
        "/deploy prod now"
    );
}

#[test]
fn build_prompt_text_without_args_parity() {
    assert_eq!(build_skills_prompt_text("deploy", ""), "/deploy");
}

// ---- stale-message gate ----

#[test]
fn stale_when_callback_id_missing_parity() {
    assert!(is_skills_message_stale(Some(7), None));
}

#[test]
fn stale_when_ids_mismatch_parity() {
    assert!(is_skills_message_stale(Some(7), Some(8)));
}

#[test]
fn fresh_when_ids_match_parity() {
    assert!(!is_skills_message_stale(Some(7), Some(7)));
}

// ---- stage gates ----

#[test]
fn rejects_page_for_confirm_stage_parity() {
    assert!(should_reject_page_for_stage("confirm"));
}

#[test]
fn passes_page_for_list_stage_parity() {
    assert!(!should_reject_page_for_stage("list"));
}

#[test]
fn rejects_execute_for_list_stage_parity() {
    assert!(should_reject_execute_for_stage("list"));
}

#[test]
fn rejects_select_for_confirm_stage_parity() {
    assert!(should_reject_select_for_stage("confirm"));
}

// ---- list / bounds ----

#[test]
fn list_empty_when_zero_parity() {
    assert!(is_skills_list_empty(0));
}

#[test]
fn list_non_empty_parity() {
    assert!(!is_skills_list_empty(3));
}

#[test]
fn page_out_of_range_parity() {
    assert!(is_skills_page_out_of_range(2, 2));
}

#[test]
fn page_in_range_parity() {
    assert!(!is_skills_page_out_of_range(1, 2));
}

#[test]
fn resolve_selection_hit_parity() {
    assert_eq!(resolve_skills_selection(3, Some(0)), Some(0));
}

#[test]
fn resolve_selection_out_of_bounds_parity() {
    assert_eq!(resolve_skills_selection(3, Some(3)), None);
}

#[test]
fn resolve_selection_none_parity() {
    assert_eq!(resolve_skills_selection(3, None), None);
}
