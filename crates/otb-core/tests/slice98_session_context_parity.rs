//! Slice 98 parity: session-settings / model-context-limit / missing-folder-notice /
//! prompt-inbox / prompt-attachment gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/app/services/session-settings-service.ts` (agent adopt gate,
//! model-attempt gate, pulled-variant rule, no-settings early `false`,
//! changed inversion),
//! `src/app/services/model-context-limit-service.ts` (`DEFAULT_CONTEXT_LIMIT`,
//! `PROVIDER_CACHE_TTL_MS`, `getModelKey`, blank-id default gate,
//! every-empty no-cache guard, missing-providers no-cache guard, truthy
//! cache hit),
//! `src/app/services/missing-folder-notice-service.ts` (non-missing null
//! gate, worktree-owner key choice) with the `"missing"` equality from
//! `src/app/services/folder-presence-service.ts`,
//! `src/app/services/prompt-inbox-service.ts` (`withdrawInboxPrompt`
//! no-inbox / list-error / gone / cancel-error / removed precedence,
//! reconcile skip + stale-drop gates) and
//! `src/app/services/prompt-attachment-service.ts` (`OPENCODE_TEXT_MIME`,
//! project-change / stat-error / not-a-file / too-large / outside-root
//! drops).
//!
//! Already covered elsewhere and NOT re-tested here:
//! `toRelativePath` (`attach_foreground_gates`), `isFileSizeAllowed` /
//! `formatFileSize` / text-name/mime helpers (`media_prompt`),
//! `FolderPresence` style/`namesMatch`/listing walk (`history_browser_gates`),
//! `isSameModel` itself (`types_config` / `agent_model_variant_gates`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `session_context_gates` is ported (only the constant-value tests pass).

use otb_core::session_context_gates::*;

// ---- constants (5) ----

#[test]
fn default_context_limit_value_parity() {
    assert_eq!(DEFAULT_CONTEXT_LIMIT, 200_000);
}

#[test]
fn provider_cache_ttl_value_parity() {
    assert_eq!(PROVIDER_CACHE_TTL_MS, 600_000);
}

#[test]
fn opencode_text_mime_value_parity() {
    assert_eq!(OPENCODE_TEXT_MIME, "text/plain");
}

#[test]
fn missing_folder_notice_key_value_parity() {
    assert_eq!(MISSING_FOLDER_NOTICE_KEY, "bot.project_folder_missing");
}

#[test]
fn missing_folder_worktree_key_value_parity() {
    assert_eq!(
        MISSING_FOLDER_WORKTREE_NOTICE_KEY,
        "bot.project_folder_missing_worktree"
    );
}

// ---- model-context-limit (12) ----

#[test]
fn model_key_joins_with_slash_parity() {
    assert_eq!(model_key("openai", "gpt-4"), "openai/gpt-4".to_string());
}

#[test]
fn model_key_empty_parts_parity() {
    assert_eq!(model_key("", ""), "/".to_string());
}

#[test]
fn default_limit_when_provider_missing_parity() {
    assert!(should_use_default_limit(None, Some("gpt-4")));
}

#[test]
fn default_limit_when_model_empty_parity() {
    assert!(should_use_default_limit(Some("openai"), Some("")));
    assert!(should_use_default_limit(Some("openai"), None));
}

#[test]
fn default_limit_not_when_both_present_parity() {
    assert!(!should_use_default_limit(Some("openai"), Some("gpt-4")));
    assert!(!should_use_default_limit(Some("   "), Some("gpt-4")));
}

#[test]
fn providers_list_empty_when_all_zero_parity() {
    assert!(is_providers_list_empty(&[0, 0, 0]));
}

#[test]
fn providers_list_not_empty_when_one_lists_parity() {
    assert!(!is_providers_list_empty(&[0, 2, 0]));
}

#[test]
fn providers_list_empty_vacuous_for_no_providers_parity() {
    assert!(is_providers_list_empty(&[]));
}

#[test]
fn cache_providers_when_models_and_complete_parity() {
    assert!(should_cache_providers_list(true, 0));
}

#[test]
fn cache_providers_not_when_empty_parity() {
    assert!(!should_cache_providers_list(false, 0));
}

#[test]
fn cache_providers_not_when_missing_parity() {
    assert!(!should_cache_providers_list(true, 1));
    assert!(!should_cache_providers_list(false, 2));
}

#[test]
fn context_cache_hit_parity() {
    assert!(is_context_cache_hit(Some(200_000)));
    assert!(!is_context_cache_hit(None));
    assert!(!is_context_cache_hit(Some(0)));
}

// ---- session-settings (11) ----

#[test]
fn adopt_agent_when_present_parity() {
    assert!(should_adopt_agent(Some("build")));
    assert!(should_adopt_agent(Some("   ")));
}

#[test]
fn adopt_agent_not_when_missing_parity() {
    assert!(!should_adopt_agent(None));
    assert!(!should_adopt_agent(Some("")));
}

#[test]
fn attempt_model_adopt_when_both_ids_parity() {
    assert!(should_attempt_model_adopt(Some("openai"), Some("gpt-4")));
}

#[test]
fn attempt_model_adopt_not_when_half_missing_parity() {
    assert!(!should_attempt_model_adopt(None, Some("gpt-4")));
    assert!(!should_attempt_model_adopt(Some("openai"), None));
    assert!(!should_attempt_model_adopt(Some("openai"), Some("")));
    assert!(!should_attempt_model_adopt(Some(""), Some("gpt-4")));
}

#[test]
fn pulled_variant_keeps_session_variant_parity() {
    assert_eq!(resolve_pulled_variant(true, Some("pro")), "pro".to_string());
}

#[test]
fn pulled_variant_defaults_when_session_variant_missing_parity() {
    assert_eq!(resolve_pulled_variant(true, None), "default".to_string());
    assert_eq!(
        resolve_pulled_variant(true, Some("")),
        "default".to_string()
    );
}

#[test]
fn pulled_variant_defaults_when_not_session_model_parity() {
    assert_eq!(
        resolve_pulled_variant(false, Some("pro")),
        "default".to_string()
    );
    assert_eq!(resolve_pulled_variant(false, None), "default".to_string());
}

#[test]
fn report_no_settings_when_neither_parity() {
    assert!(should_report_no_settings(None, false));
    assert!(should_report_no_settings(Some(""), false));
}

#[test]
fn report_no_settings_not_when_agent_parity() {
    assert!(!should_report_no_settings(Some("build"), false));
}

#[test]
fn report_no_settings_not_when_model_parity() {
    assert!(!should_report_no_settings(None, true));
    assert!(!should_report_no_settings(Some("build"), true));
}

#[test]
fn session_model_changed_inversion_parity() {
    assert!(has_session_model_changed(false));
    assert!(!has_session_model_changed(true));
}

// ---- missing-folder-notice (5) ----

#[test]
fn missing_presence_equality_parity() {
    assert!(is_missing_presence("missing"));
    assert!(!is_missing_presence("present"));
    assert!(!is_missing_presence("unknown"));
}

#[test]
fn missing_notice_null_when_present_parity() {
    assert_eq!(select_missing_folder_notice(false, false), None);
    assert_eq!(select_missing_folder_notice(false, true), None);
}

#[test]
fn missing_notice_worktree_key_parity() {
    assert_eq!(
        select_missing_folder_notice(true, true),
        Some(MISSING_FOLDER_WORKTREE_NOTICE_KEY)
    );
}

#[test]
fn missing_notice_plain_key_parity() {
    assert_eq!(
        select_missing_folder_notice(true, false),
        Some(MISSING_FOLDER_NOTICE_KEY)
    );
}

#[test]
fn missing_notice_keys_differ_parity() {
    assert_ne!(
        MISSING_FOLDER_NOTICE_KEY,
        MISSING_FOLDER_WORKTREE_NOTICE_KEY
    );
}

// ---- prompt-inbox (7) ----

#[test]
fn withdraw_failed_without_inbox_parity() {
    assert_eq!(
        resolve_withdraw_outcome(false, true, true, true),
        InboxWithdraw::Failed
    );
    assert_eq!(
        resolve_withdraw_outcome(false, false, false, false),
        InboxWithdraw::Failed
    );
}

#[test]
fn withdraw_failed_on_list_error_parity() {
    assert_eq!(
        resolve_withdraw_outcome(true, false, true, true),
        InboxWithdraw::Failed
    );
}

#[test]
fn withdraw_gone_when_not_included_parity() {
    assert_eq!(
        resolve_withdraw_outcome(true, true, false, true),
        InboxWithdraw::Gone
    );
}

#[test]
fn withdraw_failed_on_cancel_error_parity() {
    assert_eq!(
        resolve_withdraw_outcome(true, true, true, false),
        InboxWithdraw::Failed
    );
}

#[test]
fn withdraw_removed_on_success_parity() {
    assert_eq!(
        resolve_withdraw_outcome(true, true, true, true),
        InboxWithdraw::Removed
    );
}

#[test]
fn inbox_reconcile_skip_gate_parity() {
    assert!(should_skip_inbox_reconcile(0));
    assert!(!should_skip_inbox_reconcile(1));
    assert!(!should_skip_inbox_reconcile(3));
}

#[test]
fn stale_mirror_drop_gate_parity() {
    assert!(should_drop_stale_mirror(true, false));
    assert!(!should_drop_stale_mirror(true, true));
    assert!(!should_drop_stale_mirror(false, false));
}

// ---- prompt-attachment (8) ----

#[test]
fn attachment_drop_on_project_change_parity() {
    assert!(should_drop_attachment_on_project_change("/a", "/b"));
}

#[test]
fn attachment_keep_on_same_project_parity() {
    assert!(!should_drop_attachment_on_project_change("/a", "/a"));
    assert!(!should_drop_attachment_on_project_change("", ""));
}

#[test]
fn attachment_drop_on_stat_error_parity() {
    assert!(should_drop_attachment_on_stat_error(false));
    assert!(!should_drop_attachment_on_stat_error(true));
}

#[test]
fn attachment_drop_when_not_file_parity() {
    assert!(should_drop_attachment_when_not_file(false));
    assert!(!should_drop_attachment_when_not_file(true));
}

#[test]
fn attachment_drop_when_too_large_parity() {
    assert!(should_drop_attachment_when_too_large(false));
    assert!(!should_drop_attachment_when_too_large(true));
}

#[test]
fn attachment_drop_when_outside_root_parity() {
    assert!(should_drop_attachment_when_outside_root(false));
    assert!(!should_drop_attachment_when_outside_root(true));
}

#[test]
fn attachment_mime_constant_used_parity() {
    assert_eq!(OPENCODE_TEXT_MIME, "text/plain");
}

#[test]
fn attachment_all_drops_clear_parity() {
    assert!(should_drop_attachment_on_project_change("/a", "/b"));
    assert!(should_drop_attachment_on_stat_error(false));
    assert!(should_drop_attachment_when_not_file(false));
    assert!(should_drop_attachment_when_too_large(false));
    assert!(should_drop_attachment_when_outside_root(false));
}
