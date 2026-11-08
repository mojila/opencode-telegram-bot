//! Parity port of command / session-selection / ingest pure gates
//! (slice 32b, RED).
//!
//! Mirrors:
//! - `tests/bot/commands/projects.test.ts` (16 its: folder name unix /
//!   windows / trailing / separators-only, button labels unix+windows,
//!   page-callback valid/non-page/negative+alpha, pagination first-page /
//!   clamp-last / empty / pageSize boundaries, 64-char label truncation)
//! - `tests/bot/commands/sessions.test.ts` (search query capped at 100
//!   chars, current-marker label, background open keyboard, `session:page:`
//!   callbacks; the REST is I/O and stays in TS)
//! - `tests/bot/commands/rename.test.ts` (trimmed title, blank title keeps
//!   the flow, cancel callback payload)
//! - `tests/bot/commands/abort.test.ts` (status-poll idle/busy/not-found
//!   mapping behind `pollSessionStatus`)
//! - `tests/bot/handlers/voice.test.ts` (voice/audio `file_id` pick,
//!   `.oga` -> `.ogg` filename, blank transcript gate, `STT_NOTE_PROMPT`
//!   gating incl. `"false"`/`"0"`/blank, TTS auto -> `text_and_tts`)
//! - `tests/bot/commands/status.test.ts` (model line with/without variant,
//!   project `main: branch` display)
//! - `tests/bot/commands/help.test.ts` (help text lists every command)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::command_session_gates::*;

// --- project folder name (4) ---

#[test]
fn slice32b_01_folder_name_from_unix_path() {
    assert_eq!(
        get_project_folder_name("/Users/evan/work/opencode-telegram-bot"),
        "opencode-telegram-bot"
    );
}

#[test]
fn slice32b_02_folder_name_from_windows_path() {
    assert_eq!(get_project_folder_name("C:\\work\\my-project"), "my-project");
}

#[test]
fn slice32b_03_folder_name_ignores_trailing_separators() {
    assert_eq!(get_project_folder_name("/var/www/project/"), "project");
    assert_eq!(get_project_folder_name("C:\\repo\\project\\"), "project");
}

#[test]
fn slice32b_04_folder_name_returns_input_when_only_separators() {
    assert_eq!(get_project_folder_name("///"), "///");
}

// --- project button label (2) ---

#[test]
fn slice32b_05_button_label_unix() {
    assert_eq!(
        build_project_button_label(0, "/Users/evan/work/opencode-telegram-bot"),
        "1. opencode-telegram-bot [/Users/evan/work/opencode-telegram-bot]"
    );
}

#[test]
fn slice32b_06_button_label_windows() {
    assert_eq!(
        build_project_button_label(3, "D:\\repo\\awesome"),
        "4. awesome [D:\\repo\\awesome]"
    );
}

// --- project page callback (3) ---

#[test]
fn slice32b_07_parses_valid_project_page_callbacks() {
    assert_eq!(parse_project_page_callback("projects:page:0"), Some(0));
    assert_eq!(parse_project_page_callback("projects:page:12"), Some(12));
}

#[test]
fn slice32b_08_rejects_non_project_page_callbacks() {
    assert_eq!(parse_project_page_callback("project:abc"), None);
    assert_eq!(parse_project_page_callback("projects:search"), None);
}

#[test]
fn slice32b_09_rejects_negative_and_alpha_project_pages() {
    assert_eq!(parse_project_page_callback("projects:page:-1"), None);
    assert_eq!(parse_project_page_callback("projects:page:abc"), None);
    assert_eq!(parse_project_page_callback("projects:page:"), None);
}

// --- projects pagination range (4) ---

#[test]
fn slice32b_10_pagination_first_page() {
    assert_eq!(
        calculate_projects_pagination_range(25, 0, 10),
        ProjectsPaginationRange {
            page: 0,
            total_pages: 3,
            start_index: 0,
            end_index: 10,
        }
    );
}

#[test]
fn slice32b_11_pagination_clamps_to_last_page() {
    assert_eq!(
        calculate_projects_pagination_range(25, 99, 10),
        ProjectsPaginationRange {
            page: 2,
            total_pages: 3,
            start_index: 20,
            end_index: 25,
        }
    );
}

#[test]
fn slice32b_12_pagination_empty_list_is_one_empty_page() {
    assert_eq!(
        calculate_projects_pagination_range(0, 0, 10),
        ProjectsPaginationRange {
            page: 0,
            total_pages: 1,
            start_index: 0,
            end_index: 0,
        }
    );
}

#[test]
fn slice32b_13_pagination_honours_small_page_size_boundary() {
    assert_eq!(
        calculate_projects_pagination_range(8, 0, 7),
        ProjectsPaginationRange {
            page: 0,
            total_pages: 2,
            start_index: 0,
            end_index: 7,
        }
    );
    assert_eq!(
        calculate_projects_pagination_range(8, 1, 7),
        ProjectsPaginationRange {
            page: 1,
            total_pages: 2,
            start_index: 7,
            end_index: 8,
        }
    );
}

// --- project button label truncation (2) ---

#[test]
fn slice32b_14_active_label_gets_check_prefix() {
    assert_eq!(
        format_project_button_label("1. repo [/repo]", true),
        "✅ 1. repo [/repo]"
    );
    assert_eq!(
        format_project_button_label("1. repo [/repo]", false),
        "1. repo [/repo]"
    );
}

#[test]
fn slice32b_15_long_label_truncates_to_64_chars() {
    let long = format!("1. {} [/repo]", "r".repeat(100));
    let labelled = format_project_button_label(&long, false);
    assert_eq!(labelled.chars().count(), MAX_INLINE_BUTTON_LABEL_LENGTH);
    assert!(labelled.ends_with("..."));
}

// --- project search match (1) ---

#[test]
fn slice32b_16_project_matches_folder_substring_case_insensitively() {
    assert!(project_matches_search("opencode-telegram-bot", None, false, "TELEGRAM"));
    assert!(project_matches_search(
        "other",
        Some("My Cool Project"),
        true,
        "cool"
    ));
    assert!(!project_matches_search("other", None, false, "telegram"));
}

// --- session search query cap (2) ---

#[test]
fn slice32b_17_session_query_caps_at_100_chars() {
    assert_eq!(SESSION_SEARCH_QUERY_MAX_LENGTH, 100);
    assert_eq!(cap_session_search_query(&"x".repeat(140)), "x".repeat(100));
}

#[test]
fn slice32b_18_short_session_query_passes_through() {
    assert_eq!(cap_session_search_query(" SESSION "), " SESSION ");
}

// --- session page callback (3) ---

#[test]
fn slice32b_19_parses_valid_session_pages() {
    assert_eq!(parse_session_page_callback("session:page:1"), Some(1));
    assert_eq!(parse_session_page_callback("session:page:0"), Some(0));
}

#[test]
fn slice32b_20_rejects_wrong_prefix_session_page() {
    assert_eq!(parse_session_page_callback("model:openai/gpt"), None);
    assert_eq!(parse_session_page_callback("session:abc"), None);
}

#[test]
fn slice32b_21_rejects_negative_session_page() {
    assert_eq!(parse_session_page_callback("session:page:-1"), None);
}

// --- session id callback (2) ---

#[test]
fn slice32b_22_parses_session_id_callback() {
    assert_eq!(
        parse_session_id_callback("session:session-1"),
        Some("session-1".to_string())
    );
}

#[test]
fn slice32b_23_rejects_page_and_empty_session_callbacks() {
    assert_eq!(parse_session_id_callback("session:page:1"), None);
    assert_eq!(parse_session_id_callback("session:"), None);
}

// --- background session callback (4) ---

#[test]
fn slice32b_24_parses_plain_background_session_callback() {
    assert_eq!(
        parse_background_session_callback("background-session:session-1"),
        Some(BackgroundSessionCallback {
            session_id: "session-1".to_string(),
            kind: None,
        })
    );
}

#[test]
fn slice32b_25_parses_marked_background_callbacks() {
    assert_eq!(
        parse_background_session_callback("background-session:a:session-1"),
        Some(BackgroundSessionCallback {
            session_id: "session-1".to_string(),
            kind: Some(BackgroundSessionOpenKind::AssistantResponse),
        })
    );
    assert_eq!(
        parse_background_session_callback("background-session:q:session-1"),
        Some(BackgroundSessionCallback {
            session_id: "session-1".to_string(),
            kind: Some(BackgroundSessionOpenKind::QuestionAsked),
        })
    );
}

#[test]
fn slice32b_26_parses_permission_background_marker() {
    assert_eq!(
        parse_background_session_callback("background-session:p:session-9"),
        Some(BackgroundSessionCallback {
            session_id: "session-9".to_string(),
            kind: Some(BackgroundSessionOpenKind::PermissionAsked),
        })
    );
}

#[test]
fn slice32b_27_rejects_bad_background_callbacks() {
    assert_eq!(parse_background_session_callback("model:openai/gpt"), None);
    assert_eq!(parse_background_session_callback("background-session:"), None);
    assert_eq!(
        parse_background_session_callback("background-session:z:session-1"),
        None
    );
}

// --- session callbacks + labels (3) ---

#[test]
fn slice32b_28_builds_session_page_callback() {
    assert_eq!(build_session_page_callback(2), "session:page:2");
}

#[test]
fn slice32b_29_session_search_matches_title_or_id() {
    assert!(session_matches_search("Session 1", "session-1", "sion"));
    assert!(session_matches_search("Other", "session-2", "SESSION-2"));
    assert!(!session_matches_search("Other", "session-2", "zzz"));
}

#[test]
fn slice32b_30_session_button_label_with_marker() {
    assert_eq!(
        build_session_button_label(1, Some("★ "), "Session 2", "01/02/2024"),
        "2. ★ Session 2 (01/02/2024)"
    );
    assert_eq!(
        build_session_button_label(0, None, "Session 1", "01/02/2024"),
        "1. Session 1 (01/02/2024)"
    );
}

// --- rename (4) ---

#[test]
fn slice32b_31_rename_cancel_callback_matches() {
    assert_eq!(RENAME_CANCEL_CALLBACK, "rename:cancel");
    assert!(is_rename_cancel_callback("rename:cancel"));
    assert!(!is_rename_cancel_callback("rename:other"));
}

#[test]
fn slice32b_32_rename_trims_title() {
    assert_eq!(
        normalize_rename_title("  New title  "),
        Some("New title".to_string())
    );
}

#[test]
fn slice32b_33_blank_rename_title_keeps_flow_active() {
    assert_eq!(normalize_rename_title("   "), None);
    assert_eq!(normalize_rename_title(""), None);
}

#[test]
fn slice32b_34_slash_input_is_not_a_rename_title() {
    assert!(rename_text_is_command_passthrough("/new"));
    assert!(!rename_text_is_command_passthrough("New title"));
}

// --- abort poll classifier (3) ---

#[test]
fn slice32b_35_poll_idle_and_error_map_to_idle() {
    assert_eq!(
        classify_poll_session_status(Some("idle")),
        PollSessionState::Idle
    );
    assert_eq!(
        classify_poll_session_status(Some("error")),
        PollSessionState::Idle
    );
}

#[test]
fn slice32b_36_poll_busy_stays_busy() {
    assert_eq!(
        classify_poll_session_status(Some("busy")),
        PollSessionState::Busy
    );
}

#[test]
fn slice32b_37_poll_missing_or_unknown_is_not_found() {
    assert_eq!(classify_poll_session_status(None), PollSessionState::NotFound);
    assert_eq!(
        classify_poll_session_status(Some("archived")),
        PollSessionState::NotFound
    );
}

// --- voice ingest (7) ---

#[test]
fn slice32b_38_voice_file_id_wins_over_audio() {
    assert_eq!(
        pick_voice_file_id(Some("v1"), Some("a1")),
        Some("v1".to_string())
    );
}

#[test]
fn slice32b_39_audio_file_id_is_fallback() {
    assert_eq!(pick_voice_file_id(None, Some("a1")), Some("a1".to_string()));
    assert_eq!(pick_voice_file_id(None, None), None);
}

#[test]
fn slice32b_40_oga_filename_becomes_ogg() {
    assert_eq!(
        voice_filename_from_file_path("voice/file_123.oga"),
        "file_123.ogg"
    );
}

#[test]
fn slice32b_41_filename_defaults_when_no_basename() {
    assert_eq!(voice_filename_from_file_path("voice/file_1.ogg"), "file_1.ogg");
    assert_eq!(normalize_recognized_text("   "), None);
}

#[test]
fn slice32b_42_recognized_text_trims() {
    assert_eq!(
        normalize_recognized_text("  run tests\n"),
        Some("run tests".to_string())
    );
}

#[test]
fn slice32b_43_stt_note_gating() {
    let note = "Handle with care";
    assert_eq!(stt_note_prefix(note), Some("[Note: Handle with care]".to_string()));
    assert_eq!(stt_note_prefix(""), None);
    assert_eq!(stt_note_prefix("false"), None);
    assert_eq!(stt_note_prefix("FALSE"), None);
    assert_eq!(stt_note_prefix("0"), None);
    assert_eq!(
        build_llm_text(note, "run tests"),
        "[Note: Handle with care]\nrun tests"
    );
    assert_eq!(build_llm_text("", "run tests"), "run tests");
}

#[test]
fn slice32b_44_voice_response_mode_follows_tts_mode() {
    assert_eq!(voice_response_mode("auto"), "text_and_tts");
    assert_eq!(voice_response_mode("all"), "text_and_tts");
    assert_eq!(voice_response_mode("off"), "text_only");
}

// --- status + help (4) ---

#[test]
fn slice32b_45_status_model_line_with_variant() {
    assert_eq!(
        format_status_model_line("openai", "gpt-5", Some("low")),
        "🧠 openai/gpt-5 (low)"
    );
    assert_eq!(
        format_status_model_line("openai", "gpt-5", Some("default")),
        "🧠 openai/gpt-5 (default)"
    );
}

#[test]
fn slice32b_46_status_model_line_without_variant() {
    assert_eq!(
        format_status_model_line("openai", "gpt-5", None),
        "🧠 openai/gpt-5"
    );
}

#[test]
fn slice32b_47_status_project_display_with_branch() {
    assert_eq!(
        format_status_project_display("/repo-main", Some("feature/mobile")),
        "/repo-main: feature/mobile"
    );
    assert_eq!(
        format_status_project_display("/repo", None),
        "/repo"
    );
}

#[test]
fn slice32b_48_help_text_lists_every_command() {
    let text = format_help_text(
        &[("status", "Show status"), ("new", "New session")],
        "Help",
        "Use the keyboard",
    );
    assert!(text.starts_with("📖 Help\n\n"));
    assert!(text.contains("/status - Show status"));
    assert!(text.contains("/new - New session"));
    assert!(text.ends_with("Use the keyboard"));
}
