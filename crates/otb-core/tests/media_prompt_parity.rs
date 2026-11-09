//! Parity port (RED, slice25b) for batch media-prompt.
//!
//! TS sources mapped:
//! - tests/app/services/tts-service.test.ts (`stripMarkdownForSpeech`,
//!   `extractLanguageCode`, `isTtsConfigured`, provider URL/model
//!   fallbacks, TTS gating; network synthesize paths excluded)
//! - tests/app/services/file-download-service.test.ts (`toDataUri`,
//!   `isFileSizeAllowed`, `formatFileSize`, `isTextMimeType`,
//!   `isTextFileName`, file-URL base, token redaction, transient
//!   retry delays; live fetch/retry I/O excluded)
//! - tests/bot/handlers/prompt.test.ts (`createIncomingPrompt`
//!   defaults, response-mode map, empty-prompt gating; Telegram/OpenCode
//!   dispatch excluded)
//! - tests/bot/messages/summary-message-formatter.test.ts
//!   (`formatSummaryWithMode` split/trim/wrap behaviour,
//!   `formatRunningTaskLine`, `normalizePathForDisplay`, pipe escaping,
//!   code-file gating; MarkdownV2 converter pipeline excluded)
//!
//! Every test MUST FAIL with `unimplemented!()` until the Rust port lands.

use otb_core::media_prompt::*;

// ---- tts-service.test.ts: isTtsConfigured ----

#[test]
fn slice25b_tts_openai_missing_key_not_configured() {
    assert!(!is_tts_configured(
        "openai",
        "https://api.openai.com/v1",
        "",
        false
    ));
}

#[test]
fn slice25b_tts_openai_set_configured() {
    assert!(is_tts_configured(
        "openai",
        "https://api.openai.com/v1",
        "sk-test-key",
        false
    ));
}

#[test]
fn slice25b_tts_google_without_credentials_not_configured() {
    assert!(!is_tts_configured("google", "", "", false));
}

#[test]
fn slice25b_tts_google_with_credentials_configured() {
    assert!(is_tts_configured("google", "", "", true));
}

#[test]
fn slice25b_tts_elevenlabs_set_configured() {
    assert!(is_tts_configured(
        "elevenlabs",
        "https://api.elevenlabs.io/v1",
        "xi-test-key",
        false
    ));
}

#[test]
fn slice25b_tts_edge_always_configured() {
    assert!(is_tts_configured("edge", "", "", false));
}

#[test]
fn slice25b_tts_not_configured_message_per_provider() {
    assert!(tts_not_configured_message("openai").contains("TTS_API_URL and TTS_API_KEY"));
    assert!(tts_not_configured_message("elevenlabs").contains("ElevenLabs"));
    assert!(tts_not_configured_message("google").contains("GOOGLE_APPLICATION_CREDENTIALS"));
}

// ---- tts-service.test.ts: stripMarkdownForSpeech ----

#[test]
fn slice25b_strip_bold_markers() {
    assert_eq!(
        strip_markdown_for_speech("this is **bold** text"),
        "this is bold text"
    );
}

#[test]
fn slice25b_strip_italic_markers() {
    assert_eq!(
        strip_markdown_for_speech("this is *italic* text"),
        "this is italic text"
    );
}

#[test]
fn slice25b_strip_inline_code_and_link() {
    assert_eq!(
        strip_markdown_for_speech("run `npm install` now"),
        "run npm install now"
    );
    assert_eq!(
        strip_markdown_for_speech("click [here](https://example.com) now"),
        "click here now"
    );
}

#[test]
fn slice25b_strip_fenced_block_keeps_content() {
    assert_eq!(
        strip_markdown_for_speech("before\n```js\nconst x = 1\n```\nafter"),
        "before\nconst x = 1\nafter"
    );
}

#[test]
fn slice25b_strip_headings_quotes_lists() {
    assert_eq!(strip_markdown_for_speech("## Title"), "Title");
    assert_eq!(strip_markdown_for_speech("> quoted text"), "quoted text");
    assert_eq!(
        strip_markdown_for_speech("- item one\n* item two"),
        "item one\nitem two"
    );
    assert_eq!(
        strip_markdown_for_speech("1. first\n2. second"),
        "first\nsecond"
    );
}

// ---- tts-service.test.ts: extractLanguageCode + provider URL/model ----

#[test]
fn slice25b_extract_language_code_voices() {
    assert_eq!(extract_language_code("de-DE-Neural2-B"), "de-DE");
    assert_eq!(extract_language_code("en-US-Studio-O"), "en-US");
    assert_eq!(extract_language_code("cmn-CN-Wavenet-A"), "cmn-CN");
}

#[test]
fn slice25b_extract_language_code_fallback() {
    assert_eq!(extract_language_code("unknown"), "en-US");
}

#[test]
fn slice25b_openai_speech_url_shape() {
    assert_eq!(
        openai_speech_url("https://api.openai.com/v1"),
        "https://api.openai.com/v1/audio/speech"
    );
}

#[test]
fn slice25b_elevenlabs_url_trims_trailing_slashes() {
    assert_eq!(
        elevenlabs_speech_url("https://api.elevenlabs.io/v1/", "voice-id"),
        "https://api.elevenlabs.io/v1/text-to-speech/voice-id"
    );
}

#[test]
fn slice25b_elevenlabs_model_fallback_when_empty() {
    assert_eq!(elevenlabs_model_id(""), ELEVENLABS_FALLBACK_MODEL);
    assert_eq!(
        elevenlabs_model_id("eleven_flash_v2_5"),
        "eleven_flash_v2_5"
    );
}

#[test]
fn slice25b_tts_input_limit_is_4000() {
    assert_eq!(TTS_INPUT_LIMIT_CHARS, 4_000);
    assert!(!should_prepare_tts_response(
        Some("text_and_tts"),
        &"a".repeat(4_001),
        true
    ));
}

#[test]
fn slice25b_prepare_tts_gating() {
    assert!(!should_prepare_tts_response(
        Some("text_only"),
        "hello",
        true
    ));
    assert!(!should_prepare_tts_response(None, "hello", true));
    assert!(!should_prepare_tts_response(
        Some("text_and_tts"),
        "   ",
        true
    ));
    assert!(!should_prepare_tts_response(
        Some("text_and_tts"),
        "hello",
        false
    ));
    assert!(should_prepare_tts_response(
        Some("text_and_tts"),
        "hello",
        true
    ));
}

// ---- file-download-service.test.ts: pure helpers ----

#[test]
fn slice25b_to_data_uri_text() {
    assert_eq!(
        to_data_uri(b"Hello, World!", "text/plain"),
        "data:text/plain;base64,SGVsbG8sIFdvcmxkIQ=="
    );
}

#[test]
fn slice25b_to_data_uri_image_and_empty() {
    assert_eq!(
        to_data_uri(&[0x89, 0x50, 0x4E, 0x47], "image/png"),
        "data:image/png;base64,iVBORw=="
    );
    assert_eq!(
        to_data_uri(&[], "application/octet-stream"),
        "data:application/octet-stream;base64,"
    );
}

#[test]
fn slice25b_file_size_allowed_within_and_at_limit() {
    assert!(is_file_size_allowed(Some(100 * 1024), 200));
    assert!(is_file_size_allowed(Some(1024), 1));
}

#[test]
fn slice25b_file_size_allowed_over_limit() {
    assert!(!is_file_size_allowed(Some(300 * 1024), 200));
    assert!(!is_file_size_allowed(Some(1025), 1));
}

#[test]
fn slice25b_file_size_allowed_unknown() {
    assert!(is_file_size_allowed(None, 100));
}

#[test]
fn slice25b_format_file_size_units() {
    assert_eq!(format_file_size(0), "0B");
    assert_eq!(format_file_size(500), "500B");
    assert_eq!(format_file_size(1024), "1.0KB");
    assert_eq!(format_file_size(1536), "1.5KB");
    assert_eq!(format_file_size(1024 * 1024), "1.0MB");
}

#[test]
fn slice25b_text_mime_text_and_whitelist() {
    assert!(is_text_mime_type(Some("text/plain"), None));
    assert!(is_text_mime_type(Some("text/markdown"), None));
    assert!(is_text_mime_type(Some("application/json"), None));
    assert!(is_text_mime_type(Some("application/sql"), None));
}

#[test]
fn slice25b_text_mime_rejects_binary() {
    assert!(!is_text_mime_type(Some("application/pdf"), None));
    assert!(!is_text_mime_type(Some("application/zip"), None));
    assert!(!is_text_mime_type(Some("image/png"), None));
    assert!(!is_text_mime_type(None, None));
    assert!(!is_text_mime_type(Some(""), None));
}

#[test]
fn slice25b_text_mime_filename_fallback() {
    assert!(is_text_mime_type(
        Some("application/octet-stream"),
        Some("component.svelte")
    ));
    assert!(is_text_mime_type(
        Some("application/octet-stream"),
        Some("script.py")
    ));
    assert!(!is_text_mime_type(
        Some("application/octet-stream"),
        Some("file.xyz")
    ));
    assert!(!is_text_mime_type(Some("application/octet-stream"), None));
    assert!(!is_text_mime_type(None, Some("file.svelte")));
}

#[test]
fn slice25b_text_filename_sources_and_dotfiles() {
    assert!(is_text_file_name("index.ts"));
    assert!(is_text_file_name("package.json"));
    assert!(is_text_file_name("main.py"));
    assert!(is_text_file_name("Makefile"));
    assert!(is_text_file_name(".gitignore"));
}

#[test]
fn slice25b_text_filename_case_and_paths() {
    assert!(is_text_file_name("MAKEFILE"));
    assert!(is_text_file_name("dockerfile"));
    assert!(is_text_file_name("D:\\Repo\\src\\bot\\index.ts"));
    assert!(is_text_file_name("/repo/src/Makefile"));
}

#[test]
fn slice25b_text_filename_rejects_binary_and_unknown() {
    assert!(!is_text_file_name("logo.png"));
    assert!(!is_text_file_name("app.exe"));
    assert!(!is_text_file_name("archive.zip"));
    assert!(!is_text_file_name("mystery"));
    assert!(!is_text_file_name(""));
}

#[test]
fn slice25b_telegram_file_url_default_root() {
    assert_eq!(
        telegram_file_url(None, "bot-token-xyz", "voice/sample.ogg"),
        "https://api.telegram.org/file/botbot-token-xyz/voice/sample.ogg"
    );
}

#[test]
fn slice25b_telegram_file_url_custom_root_and_slash() {
    assert_eq!(
        telegram_file_url(
            Some("https://tg-proxy.example.com"),
            "bot-token-xyz",
            "voice/sample.ogg"
        ),
        "https://tg-proxy.example.com/file/botbot-token-xyz/voice/sample.ogg"
    );
    assert_eq!(
        telegram_file_url(
            Some("https://tg-proxy.example.com/"),
            "bot-token-xyz",
            "voice/sample.ogg"
        ),
        "https://tg-proxy.example.com/file/botbot-token-xyz/voice/sample.ogg"
    );
}

#[test]
fn slice25b_redact_bot_token_in_errors() {
    assert_eq!(
        redact_bot_token(
            "request to https://api.telegram.org/file/botbot-token-xyz/voice/sample.ogg failed",
            "bot-token-xyz"
        ),
        "request to https://api.telegram.org/file/bot***/voice/sample.ogg failed"
    );
}

#[test]
fn slice25b_transient_retry_delays() {
    assert_eq!(transient_retry_delay_ms(429, Some(2)), Some(2_000));
    assert_eq!(transient_retry_delay_ms(502, None), Some(1_000));
    assert_eq!(transient_retry_delay_ms(200, None), None);
    assert_eq!(transient_retry_delay_ms(404, None), None);
}

// ---- prompt.test.ts: pure prompt construction/gating ----

#[test]
fn slice25b_incoming_prompt_defaults() {
    let prompt = new_incoming_prompt("hello", vec![], vec![]);
    assert_eq!(prompt.text, "hello");
    assert!(prompt.file_parts.is_empty());
    assert!(prompt.photos.is_empty());
}

#[test]
fn slice25b_response_mode_set_get_clear() {
    let mut modes = PromptResponseModes::new();
    assert_eq!(modes.get("session-1"), None);
    modes.set("session-1", PromptResponseMode::TextAndTts);
    assert_eq!(modes.get("session-1"), Some(PromptResponseMode::TextAndTts));
    modes.clear("session-1");
    assert_eq!(modes.get("session-1"), None);
}

#[test]
fn slice25b_empty_prompt_without_attachments_not_sendable() {
    assert!(!is_prompt_sendable("", 0, 0));
    assert!(!is_prompt_sendable("   ", 0, 0));
}

#[test]
fn slice25b_text_or_attachment_prompt_sendable() {
    assert!(is_prompt_sendable("hello", 0, 0));
    assert!(is_prompt_sendable("", 1, 0));
    assert!(is_prompt_sendable("", 0, 2));
}

// ---- summary-message-formatter.test.ts: pure split/format ----

#[test]
fn slice25b_split_empty_and_blank_is_no_parts() {
    assert!(split_summary_text("", 4096).is_empty());
    assert!(split_summary_text("   ", 4096).is_empty());
}

#[test]
fn slice25b_split_prefers_newline_boundary() {
    let parts = split_summary_text("one\ntwo\nthree", 8);
    assert!(parts.len() > 1);
    assert!(parts.iter().all(|p| p.len() <= 8));
    assert_eq!(parts.join(""), "one\ntwo\nthree");
}

#[test]
fn slice25b_wrap_raw_part_code_fence() {
    assert_eq!(wrap_raw_part("hello"), "```\nhello\n```");
}

#[test]
fn slice25b_truncate_with_ellipsis() {
    assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
    assert_eq!(truncate_with_ellipsis("hi", 64), "hi");
    assert_eq!(truncate_with_ellipsis("hello", 3), "...");
}

#[test]
fn slice25b_running_task_line_from_description() {
    let line = format_running_task_line(Some("Explore project"), "Task", 128);
    assert!(line.contains("Explore project"));
    assert!(!line.contains("Inspect architecture"));
}

#[test]
fn slice25b_running_task_line_single_line_and_cut() {
    let line = format_running_task_line(Some("one\ntwo\nthree"), "Task", 128);
    assert!(!line.contains('\n'));
    let long_text = "a".repeat(200);
    let long = format_running_task_line(Some(long_text.as_str()), "Task", 32);
    assert!(long.len() <= 32);
    assert!(long.ends_with('…'));
}

#[test]
fn slice25b_running_task_line_empty_is_label() {
    assert_eq!(format_running_task_line(None, "Task", 128), "Task");
    assert_eq!(format_running_task_line(Some("  "), "Task", 128), "Task");
}

#[test]
fn slice25b_normalize_path_separators_and_worktree() {
    assert_eq!(
        normalize_path_for_display("D:\\Repo\\src\\index.ts", None),
        "D:/Repo/src/index.ts"
    );
    assert_eq!(
        normalize_path_for_display("/repo/src/index.ts", Some("/repo")),
        "src/index.ts"
    );
}

#[test]
fn slice25b_escape_pipes_outside_code_only() {
    assert_eq!(escape_pipes_outside_code("| A | B |"), "\\| A \\| B \\|");
    let fenced = escape_pipes_outside_code("```\n| raw |\n```");
    assert!(fenced.contains("| raw |"));
    assert!(!fenced.contains("\\|"));
}

#[test]
fn slice25b_code_file_size_gating_and_bom() {
    assert!(!code_file_too_large(100, 100));
    assert!(code_file_too_large(200 * 1024, 100));
    assert_eq!(UTF8_BOM, '\u{FEFF}');
}
