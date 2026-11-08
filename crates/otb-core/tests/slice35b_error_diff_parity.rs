//! Parity port of the error-text / session-list / diff-display /
//! file-question-permission-footer / tool-line remainder (slice 35b, RED).
//!
//! Mirrors:
//! - `tests/utils/opencode-error.test.ts` (`isExpectedOpencodeUnavailableError`
//!   markers, `isOpencodeNotFoundError` name gate, `isServerUnavailableError`
//!   cause-chain + code fields, `extractErrorMessage` Error/string/record
//!   chain, whitespace fallthrough, unusable -> null)
//! - `tests/utils/error-format.test.ts` (stack/message content, `"{}"`
//!   fallback, overlong clip with `...`)
//! - `tests/bot/menus/session-selection-menu.ts` view contract exercised via
//!   `tests/bot/commands/sessions.test.ts` (row labels, current marker,
//!   empty gate, first-page vs paged title)
//! - `tests/bot/messages/summary-message-formatter.test.ts` remainder
//!   (diff line info, diff change counts, patch-file headers incl. CRLF,
//!   title status lines, movePath > relativePath > filePath pick, patch
//!   file lines, tool icons, worktree-relative paths, `formatDiff`
//!   rendering, write line counters)
//! - `tests/bot/menus/permission-menu.test.ts` remainder
//!   (`message is not modified` gate, `• pattern` lines, `• …` cut line,
//!   surrogate-safe cut)
//! - question footer remainder (`formatButtonText` 60-char cut,
//!   `isQuestionRequestGone` tags, `index + 1/total` progress)
//! - tool-line remainder (`truncateWithEllipsis` incl. tiny max,
//!   whitespace collapse, todo markers + `MAX_TODOS`, running-task line
//!   bare/collapsed/cut, compact single-line + 64-char cut)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::error_diff_display::*;

// --- expected-unavailable markers (3) ---

#[test]
fn slice35b_01_expected_unavailable_core_markers() {
    assert!(is_expected_opencode_unavailable("fetch failed"));
    assert!(is_expected_opencode_unavailable("ECONNREFUSED"));
    assert!(is_expected_opencode_unavailable("connection reset: econnreset"));
}

#[test]
fn slice35b_02_expected_unavailable_extended_markers_case_insensitive() {
    assert!(is_expected_opencode_unavailable("Failed to fetch"));
    assert!(is_expected_opencode_unavailable("ENOTFOUND registry"));
    assert!(is_expected_opencode_unavailable("connectex: no route"));
    assert!(is_expected_opencode_unavailable("Fetch Failed"));
}

#[test]
fn slice35b_03_expected_unavailable_rejects_unrelated() {
    assert!(!is_expected_opencode_unavailable("timeout after 5000ms"));
    assert!(!is_expected_opencode_unavailable(""));
    assert!(!is_expected_opencode_unavailable("aborted by user"));
}

// --- not-found + server markers (3) ---

#[test]
fn slice35b_04_not_found_name_gate() {
    assert!(is_opencode_not_found_name("NotFoundError"));
    assert!(!is_opencode_not_found_name("UnknownError"));
    assert!(!is_opencode_not_found_name(""));
}

#[test]
fn slice35b_05_server_marker_set() {
    assert!(has_server_unavailable_marker("fetch failed"));
    assert!(has_server_unavailable_marker("ECONNREFUSED"));
    assert!(has_server_unavailable_marker("connection refused"));
    assert!(has_server_unavailable_marker("Connect Refused by peer"));
}

#[test]
fn slice35b_06_server_marker_rejects_unrelated() {
    assert!(!has_server_unavailable_marker("timeout"));
    assert!(!has_server_unavailable_marker(""));
}

// --- cause chain (2) ---

#[test]
fn slice35b_07_chain_walks_causes() {
    let root = ChainError {
        text: "connection refused".to_string(),
        ..ChainError::default()
    };
    let wrapper = ChainError {
        text: "wrapped".to_string(),
        cause: Some(Box::new(root)),
        ..ChainError::default()
    };
    assert!(is_server_unavailable_chain(&wrapper));
    assert!(!is_server_unavailable_chain(&ChainError {
        text: "timeout".to_string(),
        ..ChainError::default()
    }));
}

#[test]
fn slice35b_08_chain_reads_code_field_and_none_is_false() {
    let coded = ChainError {
        text: "boom".to_string(),
        code: Some("ECONNREFUSED".to_string()),
        ..ChainError::default()
    };
    assert!(is_server_unavailable_chain(&coded));
    assert!(!is_server_unavailable_opt(None));
    assert!(is_server_unavailable_opt(Some(&coded)));
}

// --- extractErrorMessage (3) ---

#[test]
fn slice35b_09_extract_error_and_text_trimmed() {
    assert_eq!(
        extract_error_message(&ErrorValue::ErrorMessage("boom".to_string())),
        Some("boom".to_string())
    );
    assert_eq!(
        extract_error_message(&ErrorValue::Text("  oops  ".to_string())),
        Some("oops".to_string())
    );
}

#[test]
fn slice35b_10_extract_record_chain_with_fallthrough() {
    let full = ErrorValue::Record {
        data_message: Some("Model not found.".to_string()),
        message: Some("fallback".to_string()),
        name: Some("UnknownError".to_string()),
    };
    assert_eq!(
        extract_error_message(&full),
        Some("Model not found.".to_string())
    );
    let blank_data = ErrorValue::Record {
        data_message: Some("   ".to_string()),
        message: Some("fallback".to_string()),
        name: Some("UnknownError".to_string()),
    };
    assert_eq!(
        extract_error_message(&blank_data),
        Some("fallback".to_string())
    );
    let name_only = ErrorValue::Record {
        data_message: None,
        message: None,
        name: Some("UnknownError".to_string()),
    };
    assert_eq!(
        extract_error_message(&name_only),
        Some("UnknownError".to_string())
    );
}

#[test]
fn slice35b_11_extract_unusable_is_none() {
    assert_eq!(extract_error_message(&ErrorValue::Unusable), None);
    assert_eq!(
        extract_error_message(&ErrorValue::Record {
            data_message: None,
            message: None,
            name: None,
        }),
        None
    );
    assert_eq!(
        extract_error_message(&ErrorValue::Text("   ".to_string())),
        None
    );
}

// --- error-format (3) ---

#[test]
fn slice35b_12_clip_keeps_short_and_cuts_long() {
    assert_eq!(clip_error_text("boom", 16), "boom");
    let clipped = clip_error_text(&"x".repeat(100), 16);
    assert_eq!(clipped.len(), 16);
    assert!(clipped.ends_with("..."));
}

#[test]
fn slice35b_13_empty_details_gate() {
    assert!(is_empty_error_details(""));
    assert!(is_empty_error_details("   "));
    assert!(is_empty_error_details("{}"));
    assert!(is_empty_error_details("[object Object]"));
    assert!(!is_empty_error_details("boom"));
}

#[test]
fn slice35b_14_details_trims_and_falls_back() {
    assert_eq!(format_error_details_text("  boom  ", 1500), "boom");
    assert_eq!(format_error_details_text("{}", 1500), "unknown error");
    let clipped = format_error_details_text(&"x".repeat(100), 16);
    assert_eq!(clipped.len(), 16);
    assert!(clipped.ends_with("..."));
}

// --- session list (5) ---

#[test]
fn slice35b_15_session_row_label_shape() {
    assert_eq!(
        format_session_row_label(0, "", "Session 1", "01/11/2026"),
        "1. Session 1 (01/11/2026)"
    );
    assert_eq!(
        format_session_row_label(2, "", "Session 3", "01/11/2026"),
        "3. Session 3 (01/11/2026)"
    );
}

#[test]
fn slice35b_16_session_row_current_marker() {
    assert_eq!(current_session_marker(true, "●"), "● ");
    assert_eq!(current_session_marker(false, "●"), "");
    assert_eq!(
        format_session_row_label(0, "● ", "Session 1", "01/11/2026"),
        "1. ● Session 1 (01/11/2026)"
    );
}

#[test]
fn slice35b_17_session_list_empty_gate() {
    assert!(is_empty_session_list(0));
    assert!(!is_empty_session_list(1));
    assert!(!is_empty_session_list(7));
}

#[test]
fn slice35b_18_sessions_select_title_first_page() {
    assert_eq!(sessions_select_title_key(0), "sessions.select");
}

#[test]
fn slice35b_19_sessions_select_title_paged() {
    assert_eq!(sessions_select_title_key(2), "sessions.select_page");
    assert_eq!(sessions_select_page_display(0), 1);
    assert_eq!(sessions_select_page_display(2), 3);
}

// --- diff line info + counts (4) ---

#[test]
fn slice35b_20_diff_line_info_full() {
    assert_eq!(
        format_diff_line_info(Some(2), Some(1)),
        " (+2 -1)".to_string()
    );
}

#[test]
fn slice35b_21_diff_line_info_partial_and_empty() {
    assert_eq!(format_diff_line_info(Some(3), None), " (+3)".to_string());
    assert_eq!(format_diff_line_info(None, Some(1)), " (-1)".to_string());
    assert_eq!(format_diff_line_info(Some(0), Some(0)), String::new());
    assert_eq!(format_diff_line_info(None, None), String::new());
}

#[test]
fn slice35b_22_count_diff_changes_skips_headers() {
    let diff = [
        "--- a/README.md",
        "+++ b/README.md",
        "@@ -1,1 +1,4 @@",
        " old line",
        "+new line 1",
        "+new line 2",
        "+new line 3",
    ]
    .join("\n");
    assert_eq!(count_diff_changes(&diff), (3, 0));
}

#[test]
fn slice35b_23_count_diff_changes_mixed_and_empty() {
    assert_eq!(count_diff_changes("-old\n+new\n+more\n same"), (2, 1));
    assert_eq!(count_diff_changes(""), (0, 0));
}

// --- patch headers + titles (5) ---

#[test]
fn slice35b_24_first_patch_file_update_header() {
    let patch = [
        "*** Begin Patch",
        "*** Update File: src/first.ts",
        "@@",
        "-a",
        "+b",
        "*** End Patch",
    ]
    .join("\n");
    assert_eq!(extract_first_patch_file(&patch), "src/first.ts");
}

#[test]
fn slice35b_25_first_patch_file_add_header_and_crlf() {
    let patch = "*** Begin Patch\r\n*** Add File: src/second.ts\r\n@@\r\n+c";
    assert_eq!(extract_first_patch_file(patch), "src/second.ts");
    assert_eq!(extract_first_patch_file("*** Begin Patch\n@@"), "");
}

#[test]
fn slice35b_26_updated_file_from_title() {
    assert_eq!(
        extract_first_updated_file_from_title("Success. Updated the following files:\nM src/one.ts"),
        "src/one.ts"
    );
    assert_eq!(
        extract_first_updated_file_from_title("ok\nD src/gone.ts"),
        "src/gone.ts"
    );
}

#[test]
fn slice35b_27_updated_file_from_title_miss() {
    assert_eq!(extract_first_updated_file_from_title("Run code"), "");
    assert_eq!(extract_first_updated_file_from_title("M"), "");
}

#[test]
fn slice35b_28_patch_entry_path_priority() {
    assert_eq!(
        pick_patch_entry_path(Some("D:/repo/src/moved.ts"), Some("src/old.ts"), Some("src/x.ts")),
        Some("D:/repo/src/moved.ts".to_string())
    );
    assert_eq!(
        pick_patch_entry_path(None, Some("src/a.ts"), Some("src/x.ts")),
        Some("src/a.ts".to_string())
    );
    assert_eq!(
        pick_patch_entry_path(None, None, Some("src/x.ts")),
        Some("src/x.ts".to_string())
    );
    assert_eq!(pick_patch_entry_path(None, None, None), None);
    assert_eq!(pick_patch_entry_path(Some(""), Some(""), None), None);
}

// --- patch lines + icons (2) ---

#[test]
fn slice35b_29_patch_file_line_with_and_without_counts() {
    assert_eq!(
        format_patch_file_line("src/a.ts", 1, 1),
        "🩹 apply_patch src/a.ts (+1 -1)"
    );
    assert_eq!(
        format_patch_file_line("src/moved.ts", 0, 0),
        "🩹 apply_patch src/moved.ts"
    );
}

#[test]
fn slice35b_30_tool_icons() {
    assert_eq!(tool_icon("read"), "📖");
    assert_eq!(tool_icon("write"), "✍️");
    assert_eq!(tool_icon("edit"), "✏️");
    assert_eq!(tool_icon("apply_patch"), "🩹");
    assert_eq!(tool_icon("bash"), "💻");
    assert_eq!(tool_icon("glob"), "📁");
    assert_eq!(tool_icon("grep"), "🔍");
    assert_eq!(tool_icon("mystery_tool"), "🛠️");
}

// --- path + diff format (5) ---

#[test]
fn slice35b_31_normalize_strips_worktree() {
    assert_eq!(
        normalize_display_path("D:/repo/src/absolute-write.ts", Some("D:/repo")),
        "src/absolute-write.ts"
    );
}

#[test]
fn slice35b_32_normalize_backslashes_and_dot() {
    assert_eq!(
        normalize_display_path("src\\main\\ChatConfig.kt", None),
        "src/main/ChatConfig.kt"
    );
    assert_eq!(normalize_display_path("D:/repo", Some("D:/repo")), ".");
}

#[test]
fn slice35b_33_normalize_passthrough_and_trailing_slash() {
    assert_eq!(
        normalize_display_path("src/app.ts", None),
        "src/app.ts"
    );
    assert_eq!(
        normalize_display_path("D:/repo/src/a.ts", Some("D:/repo/")),
        "src/a.ts"
    );
    assert_eq!(
        normalize_display_path("/other/src/a.ts", Some("D:/repo")),
        "/other/src/a.ts"
    );
}

#[test]
fn slice35b_34_format_diff_text_renders_hunks() {
    let diff = [
        "@@ -1,2 +1,2 @@",
        "--- a/src/app.ts",
        "+++ b/src/app.ts",
        " line1",
        "-line2",
        "+line2-updated",
        "\\ No newline at end of file",
    ]
    .join("\n");
    assert_eq!(
        format_diff_text(&diff),
        " line1\n- line2\n+ line2-updated"
    );
}

#[test]
fn slice35b_35_write_line_info_counts_lines() {
    assert_eq!(format_write_line_info("one line"), " (+1)");
    assert_eq!(format_write_line_info("line1\nline2"), " (+2)");
}

// --- question / permission footer (6) ---

#[test]
fn slice35b_36_question_button_text_plain() {
    assert_eq!(format_question_button_text("Yes", "✅ "), "✅ Yes");
    assert_eq!(format_question_button_text("No", ""), "No");
}

#[test]
fn slice35b_37_question_button_text_cuts_at_sixty() {
    let long = "a".repeat(80);
    let out = format_question_button_text(&long, "");
    assert_eq!(out.len(), 60);
    assert!(out.ends_with("..."));
}

#[test]
fn slice35b_38_question_gone_tags() {
    assert!(is_question_request_gone(
        Some("QuestionNotFoundError"),
        None
    ));
    assert!(is_question_request_gone(
        Some("FormAlreadySettledError"),
        None
    ));
    assert!(is_question_request_gone(None, Some("NotFoundError")));
    assert!(!is_question_request_gone(None, Some("UnknownError")));
    assert!(!is_question_request_gone(None, None));
}

#[test]
fn slice35b_39_message_not_modified_gate() {
    assert!(is_message_not_modified(
        "Message is not modified: specified new message content"
    ));
    assert!(is_message_not_modified("MESSAGE IS NOT MODIFIED"));
    assert!(!is_message_not_modified("message sent"));
}

#[test]
fn slice35b_40_permission_pattern_and_cut_lines() {
    assert_eq!(permission_pattern_line("npm test"), "• npm test\n");
    assert_eq!(cut_patterns_line(), "• …\n");
}

#[test]
fn slice35b_41_question_progress_text() {
    assert_eq!(question_progress_text(0, 3), "1/3");
    assert_eq!(question_progress_text(2, 3), "3/3");
    assert_eq!(question_progress_text(0, 0), "");
}

// --- tool line (7) ---

#[test]
fn slice35b_42_truncate_tool_text_long() {
    let long = "a".repeat(160);
    let out = truncate_tool_text(&long, 128);
    assert_eq!(out.len(), 128);
    assert!(out.ends_with("..."));
}

#[test]
fn slice35b_43_truncate_tool_text_short_and_tiny() {
    assert_eq!(truncate_tool_text("npm test", 128), "npm test");
    assert_eq!(truncate_tool_text("abcdef", 3), "...");
    assert_eq!(truncate_tool_text("hello world, this is long", 8), "hello...");
}

#[test]
fn slice35b_44_collapse_tool_whitespace() {
    assert_eq!(collapse_tool_whitespace("a  b\nc"), "a b c");
    assert_eq!(
        collapse_tool_whitespace("  Count\n  Kotlin   files\n"),
        "Count Kotlin files"
    );
}

#[test]
fn slice35b_45_todo_markers_and_budget() {
    assert_eq!(MAX_TODOS, 20);
    assert_eq!(format_todo_line("completed", "Done item"), "✅ Done item");
    assert_eq!(
        format_todo_line("in_progress", "Working item"),
        "🔄 Working item"
    );
    assert_eq!(format_todo_line("pending", "Later item"), "🔲 Later item");
    assert_eq!(format_todo_line("mystery", "Other item"), "🔲 Other item");
}

#[test]
fn slice35b_46_running_task_line_full() {
    assert_eq!(
        format_running_task_line(
            Some("Count Kotlin files"),
            128,
            "🤖 Running Task"
        ),
        "🤖 Running Task: Count Kotlin files"
    );
}

#[test]
fn slice35b_47_running_task_line_bare_and_collapsed() {
    assert_eq!(format_running_task_line(None, 128, "🤖 Running Task"), "🤖 Running Task");
    assert_eq!(
        format_running_task_line(Some(" \n\t "), 128, "🤖 Running Task"),
        "🤖 Running Task"
    );
    assert_eq!(
        format_running_task_line(Some("  Count\n  Kotlin   files\n"), 128, "🤖 Running Task"),
        "🤖 Running Task: Count Kotlin files"
    );
}

#[test]
fn slice35b_48_running_task_line_and_compact_cut() {
    let line = format_running_task_line(Some(&"word ".repeat(60)), 128, "🤖 Running Task");
    assert!(line.chars().count() <= 128);
    assert!(line.starts_with("🤖 Running Task: word word"));
    assert!(line.ends_with("…"));
    assert_eq!(format_compact_tool_line("a\nb", 64), "a b");
    let long = format_compact_tool_line(&"b ".repeat(100), 64);
    assert_eq!(long.len(), 64);
    assert!(long.ends_with("..."));
}
