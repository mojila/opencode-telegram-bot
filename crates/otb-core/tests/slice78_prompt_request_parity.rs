//! Slice 78 parity: prompt-request pure gates (RED).
//!
//! Mirrors the pure TS contract in `src/bot/handlers/prompt.ts`
//! (`buildPromptOptions` part ordering / placeholder / agent-model-variant
//! inclusion, the `prepareTelegramPhotos` branching, the empty-prompt
//! guard, the session/project mismatch predicate, response-mode
//! resolution and registry, the handed-over-prompt parts check and the
//! inbox-admit early guard; photo sources per
//! `src/app/types/prompt.ts`).
//!
//! Every test MUST FAIL with `unimplemented!()` panics until
//! `prompt_request_gates` is ported.

use otb_core::prompt_request_gates::*;

fn selection(
    agent: Option<&str>,
    provider: &str,
    model: &str,
    variant: Option<&str>,
) -> PromptSelection {
    PromptSelection {
        agent: agent.map(str::to_string),
        provider_id: provider.to_string(),
        model_id: model.to_string(),
        variant: variant.map(str::to_string),
    }
}

// --- build_prompt_parts ---

#[test]
fn slice78_text_only_single_text_part_parity() {
    assert_eq!(
        build_prompt_parts("hello", 0, false),
        vec![PromptPart::Text("hello".to_string())]
    );
}

#[test]
fn slice78_blank_text_no_parts_parity() {
    assert!(build_prompt_parts("   ", 0, false).is_empty());
}

#[test]
fn slice78_files_appended_after_text_parity() {
    assert_eq!(
        build_prompt_parts("hi", 2, false),
        vec![
            PromptPart::Text("hi".to_string()),
            PromptPart::File,
            PromptPart::File,
        ]
    );
}

#[test]
fn slice78_attachment_appended_last_parity() {
    assert_eq!(
        build_prompt_parts("hi", 0, true),
        vec![PromptPart::Text("hi".to_string()), PromptPart::File]
    );
}

#[test]
fn slice78_placeholder_single_file_parity() {
    assert_eq!(
        build_prompt_parts("", 1, false),
        vec![
            PromptPart::Text("See attached file".to_string()),
            PromptPart::File,
        ]
    );
}

#[test]
fn slice78_placeholder_multiple_files_parity() {
    assert_eq!(
        build_prompt_parts("", 2, false),
        vec![
            PromptPart::Text("See attached files".to_string()),
            PromptPart::File,
            PromptPart::File,
        ]
    );
}

#[test]
fn slice78_attachment_alone_gets_no_placeholder_parity() {
    assert_eq!(build_prompt_parts("", 0, true), vec![PromptPart::File]);
}

#[test]
fn slice78_placeholder_counts_only_file_parts_parity() {
    assert_eq!(
        build_prompt_parts("  ", 1, true),
        vec![
            PromptPart::Text("See attached file".to_string()),
            PromptPart::File,
            PromptPart::File,
        ]
    );
}

// --- attachment_placeholder_text / count_file_parts ---

#[test]
fn slice78_placeholder_text_single_parity() {
    assert_eq!(attachment_placeholder_text(1), "See attached file");
}

#[test]
fn slice78_placeholder_text_plural_parity() {
    assert_eq!(attachment_placeholder_text(0), "See attached files");
    assert_eq!(attachment_placeholder_text(3), "See attached files");
}

#[test]
fn slice78_count_file_parts_parity() {
    let parts = vec![
        PromptPart::Text("hi".to_string()),
        PromptPart::File,
        PromptPart::File,
    ];
    assert_eq!(count_file_parts(&parts), 2);
    assert_eq!(count_file_parts(&[]), 0);
}

// --- build_prompt_options ---

#[test]
fn slice78_options_session_and_directory_passthrough_parity() {
    let options = build_prompt_options(
        "ses-1",
        "/proj",
        "hi",
        0,
        false,
        selection(None, "anthropic", "claude", None),
    );
    assert_eq!(options.session_id, "ses-1");
    assert_eq!(options.directory, "/proj");
}

#[test]
fn slice78_options_agent_included_when_set_parity() {
    let options = build_prompt_options(
        "s",
        "/p",
        "hi",
        0,
        false,
        selection(Some("coder"), "anthropic", "claude", None),
    );
    assert_eq!(options.agent, Some("coder".to_string()));
}

#[test]
fn slice78_options_agent_omitted_when_unset_parity() {
    let options = build_prompt_options(
        "s",
        "/p",
        "hi",
        0,
        false,
        selection(None, "anthropic", "claude", None),
    );
    assert_eq!(options.agent, None);
    let options = build_prompt_options(
        "s",
        "/p",
        "hi",
        0,
        false,
        selection(Some(""), "anthropic", "claude", None),
    );
    assert_eq!(options.agent, None);
}

#[test]
fn slice78_options_model_and_variant_included_parity() {
    let options = build_prompt_options(
        "s",
        "/p",
        "hi",
        0,
        false,
        selection(None, "openai", "gpt", Some("codex")),
    );
    assert_eq!(
        options.model,
        Some(PromptModelRef {
            provider_id: "openai".to_string(),
            model_id: "gpt".to_string(),
        })
    );
    assert_eq!(options.variant, Some("codex".to_string()));
}

#[test]
fn slice78_options_model_omitted_without_provider_parity() {
    let options = build_prompt_options(
        "s",
        "/p",
        "hi",
        0,
        false,
        selection(None, "", "gpt", Some("codex")),
    );
    assert_eq!(options.model, None);
    assert_eq!(options.variant, None);
}

#[test]
fn slice78_options_model_omitted_without_model_id_parity() {
    let options = build_prompt_options(
        "s",
        "/p",
        "hi",
        0,
        false,
        selection(None, "openai", "", None),
    );
    assert_eq!(options.model, None);
}

// --- should_dispatch_prompt ---

#[test]
fn slice78_empty_prompt_blocked_parity() {
    assert!(!should_dispatch_prompt("   ", 0, 0, false));
}

#[test]
fn slice78_text_prompt_dispatched_parity() {
    assert!(should_dispatch_prompt("hello", 0, 0, false));
}

#[test]
fn slice78_files_prompt_dispatched_parity() {
    assert!(should_dispatch_prompt("  ", 2, 0, false));
}

#[test]
fn slice78_photos_prompt_dispatched_parity() {
    assert!(should_dispatch_prompt("", 0, 1, false));
}

#[test]
fn slice78_blank_text_with_attachment_dispatched_parity() {
    assert!(should_dispatch_prompt("  ", 0, 0, true));
}

// --- mismatch / handover / inbox ---

#[test]
fn slice78_mismatch_on_different_directory_parity() {
    assert!(is_session_project_mismatch(Some("/other"), "/proj"));
}

#[test]
fn slice78_no_mismatch_on_same_directory_parity() {
    assert!(!is_session_project_mismatch(Some("/proj"), "/proj"));
}

#[test]
fn slice78_no_mismatch_without_session_parity() {
    assert!(!is_session_project_mismatch(None, "/proj"));
}

#[test]
fn slice78_handover_needs_parts_parity() {
    assert!(!handover_has_parts(0));
    assert!(handover_has_parts(2));
}

#[test]
fn slice78_inbox_blocked_without_session_or_folder_parity() {
    assert!(inbox_admit_blocked(false, false));
    assert!(inbox_admit_blocked(true, true));
}

#[test]
fn slice78_inbox_allowed_when_ready_parity() {
    assert!(!inbox_admit_blocked(true, false));
}

// --- photo prep ---

#[test]
fn slice78_only_standalone_all_standalone_parity() {
    assert!(only_standalone_photos(&["standalone", "standalone"]));
}

#[test]
fn slice78_only_standalone_album_is_false_parity() {
    assert!(!only_standalone_photos(&["album"]));
    assert!(!only_standalone_photos(&["standalone", "album"]));
}

#[test]
fn slice78_only_standalone_empty_is_true_parity() {
    assert!(only_standalone_photos(&[]));
}

#[test]
fn slice78_has_album_photo_parity() {
    assert!(has_album_photo(&["standalone", "album"]));
}

#[test]
fn slice78_has_album_photo_false_parity() {
    assert!(!has_album_photo(&["standalone", "rich"]));
    assert!(!has_album_photo(&[]));
}

#[test]
fn slice78_unsupported_album_reply_key_parity() {
    assert_eq!(
        unsupported_photo_reply_key(true),
        "bot.media_group_not_processed"
    );
}

#[test]
fn slice78_unsupported_standalone_reply_key_parity() {
    assert_eq!(
        unsupported_photo_reply_key(false),
        "bot.photo_model_no_image"
    );
}

#[test]
fn slice78_download_notice_album_parity() {
    assert_eq!(photo_download_notice_key(true, 1), "bot.files_downloading");
}

#[test]
fn slice78_download_notice_single_standalone_parity() {
    assert_eq!(photo_download_notice_key(false, 1), "bot.photo_downloading");
    assert_eq!(photo_download_notice_key(false, 3), "bot.files_downloading");
}

#[test]
fn slice78_download_error_keys_parity() {
    assert_eq!(
        photo_download_error_key(true),
        "bot.media_group_download_error"
    );
    assert_eq!(photo_download_error_key(false), "bot.photo_download_error");
}

#[test]
fn slice78_strip_standalone_photos_with_text_parity() {
    assert!(should_strip_unsupported_photos(true, "look here"));
}

#[test]
fn slice78_no_strip_without_text_or_album_parity() {
    assert!(!should_strip_unsupported_photos(true, "   "));
    assert!(!should_strip_unsupported_photos(false, "look here"));
}

// --- response mode ---

#[test]
fn slice78_explicit_mode_wins_parity() {
    assert_eq!(
        resolve_response_mode(Some(PromptResponseMode::TextOnly), true),
        PromptResponseMode::TextOnly
    );
    assert_eq!(
        resolve_response_mode(Some(PromptResponseMode::TextAndTts), false),
        PromptResponseMode::TextAndTts
    );
}

#[test]
fn slice78_tts_all_defaults_to_text_and_tts_parity() {
    assert_eq!(
        resolve_response_mode(None, true),
        PromptResponseMode::TextAndTts
    );
}

#[test]
fn slice78_default_is_text_only_parity() {
    assert_eq!(
        resolve_response_mode(None, false),
        PromptResponseMode::TextOnly
    );
}

#[test]
fn slice78_response_mode_wire_values_parity() {
    assert_eq!(
        response_mode_as_str(PromptResponseMode::TextOnly),
        "text_only"
    );
    assert_eq!(
        response_mode_as_str(PromptResponseMode::TextAndTts),
        "text_and_tts"
    );
}

// --- registry ---

#[test]
fn slice78_registry_set_get_parity() {
    let mut registry = ResponseModeRegistry::new();
    registry.set("ses-1", PromptResponseMode::TextAndTts);
    assert_eq!(registry.get("ses-1"), Some(PromptResponseMode::TextAndTts));
    assert_eq!(registry.get("missing"), None);
}

#[test]
fn slice78_registry_overwrite_and_independence_parity() {
    let mut registry = ResponseModeRegistry::new();
    registry.set("a", PromptResponseMode::TextOnly);
    registry.set("b", PromptResponseMode::TextAndTts);
    registry.set("a", PromptResponseMode::TextAndTts);
    assert_eq!(registry.get("a"), Some(PromptResponseMode::TextAndTts));
    assert_eq!(registry.get("b"), Some(PromptResponseMode::TextAndTts));
}

#[test]
fn slice78_registry_clear_parity() {
    let mut registry = ResponseModeRegistry::new();
    assert_eq!(registry.get("ses-1"), None);
    registry.set("ses-1", PromptResponseMode::TextOnly);
    registry.clear("ses-1");
    assert_eq!(registry.get("ses-1"), None);
    registry.clear("missing");
}
