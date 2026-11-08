//! Parity port of the message-history / file-browser / folder-presence
//! pure gates (slice 52, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/app/services/message-history-service.ts` (`extractTextParts`,
//!   the user-message map/sort/revert gates, the latest-assistant and
//!   metrics reductions, `LATEST_ASSISTANT_RESPONSE_MESSAGES_LIMIT`)
//! - `src/app/services/file-browser-service.ts` (`MAX_ENTRIES_PER_PAGE`,
//!   home/`~` display collapse, entry labels, tree titles, count-line
//!   kinds, pagination math, error-code mapping, `isScanError`, name and
//!   `ls` sorts, win32/posix dispatch, `joinPath`,
//!   `isPathWithinDirectory`)
//! - `src/app/services/folder-presence-service.ts`
//!   (`GLOBAL_PROJECT_FOLDER`, path-style rule, `namesMatch`, ancestor
//!   entry check, own/ancestor presence decision)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::history_browser_gates::*;

fn text_part(text: &str) -> MessagePart {
    MessagePart {
        kind: "text".to_string(),
        text: Some(text.to_string()),
    }
}

fn other_part(kind: &str) -> MessagePart {
    MessagePart {
        kind: kind.to_string(),
        text: Some("ignored".to_string()),
    }
}

fn user_item(id: &str, created: i64) -> UserMessageItem {
    UserMessageItem {
        id: id.to_string(),
        text: format!("text-{id}"),
        created,
    }
}

fn assistant_candidate(created: i64, text: &str, summary: bool) -> AssistantCandidate {
    AssistantCandidate {
        role: "assistant".to_string(),
        summary,
        created,
        parts: vec![text_part(text)],
    }
}

fn metrics_candidate(created: i64, input: i64, summary: bool) -> AssistantMetricsCandidate {
    AssistantMetricsCandidate {
        role: "assistant".to_string(),
        summary,
        created,
        tokens: MetricsTokens {
            input,
            output: 12,
            reasoning: 3,
            cache_read: 8,
            cache_write: 4,
            cost: 0.123,
        },
    }
}

// --- text-part extraction (01-04) ---

#[test]
fn slice52_01_extract_joins_text_parts_without_separator() {
    assert_eq!(
        extract_text_parts(&[text_part("Hello "), text_part("world")]),
        Some("Hello world".to_string())
    );
}

#[test]
fn slice52_02_extract_skips_non_text_parts() {
    assert_eq!(
        extract_text_parts(&[other_part("image"), text_part("hi")]),
        Some("hi".to_string())
    );
}

#[test]
fn slice52_03_extract_trims_surrounding_whitespace() {
    assert_eq!(
        extract_text_parts(&[text_part("  hi  ")]),
        Some("hi".to_string())
    );
}

#[test]
fn slice52_04_extract_empty_and_missing_text_is_none() {
    assert_eq!(extract_text_parts(&[text_part("   ")]), None);
    assert_eq!(
        extract_text_parts(&[MessagePart {
            kind: "text".to_string(),
            text: None,
        }]),
        None
    );
    assert_eq!(extract_text_parts(&[other_part("image")]), None);
}

// --- user-message mapping and ordering (05-11) ---

#[test]
fn slice52_05_map_user_message_keeps_id_text_created() {
    assert_eq!(
        map_user_message("user", Some("m1"), Some(7), &[text_part("hi")]),
        Some(UserMessageItem {
            id: "m1".to_string(),
            text: "hi".to_string(),
            created: 7,
        })
    );
}

#[test]
fn slice52_06_map_user_message_rejects_non_user_and_empty_text() {
    assert_eq!(
        map_user_message("assistant", Some("m1"), Some(7), &[text_part("hi")]),
        None
    );
    assert_eq!(
        map_user_message("user", Some("m1"), Some(7), &[text_part("  ")]),
        None
    );
}

#[test]
fn slice52_07_map_user_message_id_falls_back_to_created() {
    assert_eq!(
        map_user_message("user", None, Some(42), &[text_part("hi")]),
        Some(UserMessageItem {
            id: "42".to_string(),
            text: "hi".to_string(),
            created: 42,
        })
    );
    assert_eq!(
        map_user_message("user", None, None, &[text_part("hi")]),
        Some(UserMessageItem {
            id: "0".to_string(),
            text: "hi".to_string(),
            created: 0,
        })
    );
}

#[test]
fn slice52_08_sort_user_messages_desc_by_created() {
    let mut messages = vec![user_item("a", 3), user_item("b", 1), user_item("c", 2)];
    sort_user_messages_desc(&mut messages);
    assert_eq!(
        messages.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "c", "b"]
    );
}

#[test]
fn slice52_09_revert_slice_drops_through_revert_message() {
    let messages = vec![user_item("a", 3), user_item("b", 2), user_item("c", 1)];
    assert_eq!(
        apply_revert_slice(messages, Some("b")),
        vec![user_item("c", 1)]
    );
}

#[test]
fn slice52_10_revert_slice_missing_id_keeps_all() {
    let messages = vec![user_item("a", 3), user_item("b", 2)];
    assert_eq!(
        apply_revert_slice(messages.clone(), Some("gone")),
        messages
    );
}

#[test]
fn slice52_11_revert_slice_without_id_keeps_all() {
    let messages = vec![user_item("a", 3), user_item("b", 2)];
    assert_eq!(apply_revert_slice(messages.clone(), None), messages);
}

// --- latest-assistant picks (12-15) ---

#[test]
fn slice52_12_pick_latest_skips_summary_and_picks_newest() {
    let messages = vec![
        assistant_candidate(9, "summary", true),
        assistant_candidate(5, "old", false),
        assistant_candidate(7, "new", false),
    ];
    assert_eq!(
        pick_latest_assistant_text(&messages),
        Some(AssistantPick {
            text: "new".to_string(),
            created: 7,
        })
    );
}

#[test]
fn slice52_13_pick_latest_tie_prefers_later_in_list() {
    let messages = vec![
        assistant_candidate(7, "first", false),
        assistant_candidate(7, "second", false),
    ];
    assert_eq!(
        pick_latest_assistant_text(&messages),
        Some(AssistantPick {
            text: "second".to_string(),
            created: 7,
        })
    );
}

#[test]
fn slice52_14_pick_latest_none_without_usable_assistant() {
    let user_only = vec![AssistantCandidate {
        role: "user".to_string(),
        summary: false,
        created: 7,
        parts: vec![text_part("hi")],
    }];
    assert_eq!(pick_latest_assistant_text(&user_only), None);
    let empty = vec![assistant_candidate(7, "   ", false)];
    assert_eq!(pick_latest_assistant_text(&empty), None);
}

#[test]
fn slice52_15_pick_metrics_selects_latest_non_summary() {
    let messages = vec![
        metrics_candidate(20, 200, true),
        metrics_candidate(10, 100, false),
        metrics_candidate(5, 50, false),
    ];
    assert_eq!(
        pick_latest_assistant_metrics(&messages),
        Some(AssistantMessageMetrics {
            input: 100,
            output: 12,
            reasoning: 3,
            cache_read: 8,
            cache_write: 4,
            cost: 0.123,
        })
    );
}

// --- browser display and headers (16-25) ---

#[test]
fn slice52_16_max_entries_per_page_is_eight() {
    assert_eq!(MAX_ENTRIES_PER_PAGE, 8);
}

#[test]
fn slice52_17_display_path_home_becomes_tilde() {
    assert_eq!(path_to_display_path("/home/u", "/home/u"), "~");
}

#[test]
fn slice52_18_display_path_under_home_uses_tilde_prefix() {
    assert_eq!(
        path_to_display_path("/home/u/docs", "/home/u"),
        "~/docs"
    );
    assert_eq!(path_to_display_path("/home/u2", "/home/u"), "/home/u2");
}

#[test]
fn slice52_19_display_path_outside_home_unchanged() {
    assert_eq!(path_to_display_path("/other/x", "/home/u"), "/other/x");
}

#[test]
fn slice52_20_entry_label_prefixes_folder_emoji() {
    assert_eq!(build_entry_label("docs"), "📁 docs");
}

#[test]
fn slice52_21_tree_title_without_pages() {
    assert_eq!(build_tree_title("~/proj", 0, 1), "📂 ~/proj");
}

#[test]
fn slice52_22_tree_title_with_pages_is_one_based() {
    assert_eq!(build_tree_title("~/proj", 1, 3), "📂 ~/proj  (2/3)");
}

#[test]
fn slice52_23_count_kind_empty() {
    assert_eq!(subfolder_count_kind(0), SubfolderCountKind::Empty);
}

#[test]
fn slice52_24_count_kind_one() {
    assert_eq!(subfolder_count_kind(1), SubfolderCountKind::One);
}

#[test]
fn slice52_25_count_kind_many() {
    assert_eq!(subfolder_count_kind(4), SubfolderCountKind::Many);
}

// --- browser pagination and errors (26-34) ---

#[test]
fn slice52_26_total_pages_at_least_one() {
    assert_eq!(total_pages(0), 1);
    assert_eq!(total_pages(8), 1);
}

#[test]
fn slice52_27_total_pages_rounds_up() {
    assert_eq!(total_pages(9), 2);
    assert_eq!(total_pages(16), 2);
    assert_eq!(total_pages(17), 3);
}

#[test]
fn slice52_28_clamp_page_negative_is_zero() {
    assert_eq!(clamp_page(-3, 20), 0);
}

#[test]
fn slice52_29_clamp_page_over_is_last() {
    assert_eq!(clamp_page(9, 20), 2);
    assert_eq!(clamp_page(0, 0), 0);
}

#[test]
fn slice52_30_loop_link_error_reads_as_missing() {
    assert_eq!(map_scan_error_code("ELOOP"), ScanErrorCode::Enoent);
    assert_eq!(map_scan_error_code("ENOENT"), ScanErrorCode::Enoent);
}

#[test]
fn slice52_31_permission_error_reads_as_denied() {
    assert_eq!(map_scan_error_code("EPERM"), ScanErrorCode::Eacces);
    assert_eq!(map_scan_error_code("EACCES"), ScanErrorCode::Eacces);
}

#[test]
fn slice52_32_not_directory_maps() {
    assert_eq!(map_scan_error_code("ENOTDIR"), ScanErrorCode::Enotdir);
}

#[test]
fn slice52_33_unknown_code_maps_to_unknown() {
    assert_eq!(map_scan_error_code("EISDIR"), ScanErrorCode::Unknown);
}

#[test]
fn slice52_34_is_err_matches_variant() {
    let ok = DirectoryScan::Ok(DirectoryScanOk {
        total_count: 0,
        page: 0,
        current_path: "/a".to_string(),
        display_path: "/a".to_string(),
        has_parent: true,
        parent_path: Some("/".to_string()),
    });
    assert!(!ok.is_err());
    let err = DirectoryScan::Err(DirectoryScanError {
        error: "nope".to_string(),
        code: ScanErrorCode::Enoent,
    });
    assert!(err.is_err());
}

// --- browser paths (35-37) ---

#[test]
fn slice52_35_join_path_follows_parent_style() {
    assert_eq!(join_path("a/b", "c"), "a/b/c");
    assert_eq!(join_path("C:\\a", "b"), "C:\\a\\b");
}

#[test]
fn slice52_36_sorts_are_case_insensitive_with_dirs_first() {
    let mut names = vec!["b".to_string(), "A".to_string(), "c".to_string()];
    sort_dir_names(&mut names);
    assert_eq!(names, vec!["A".to_string(), "b".to_string(), "c".to_string()]);

    let mut entries = vec![
        LsEntry {
            name: "b.txt".to_string(),
            entry_type: LsEntryType::File,
        },
        LsEntry {
            name: "zeta".to_string(),
            entry_type: LsEntryType::Directory,
        },
        LsEntry {
            name: "Alpha".to_string(),
            entry_type: LsEntryType::Directory,
        },
    ];
    sort_ls_entries(&mut entries);
    assert_eq!(
        entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(),
        vec!["Alpha", "zeta", "b.txt"]
    );
}

#[test]
fn slice52_37_within_directory_self_child_and_escape() {
    assert!(is_path_within_directory("a/b", "a/b"));
    assert!(is_path_within_directory("a/b/c", "a/b"));
    assert!(!is_path_within_directory("a/c", "a/b"));
}

// --- folder presence (38-50) ---

#[test]
fn slice52_38_global_project_folder_is_root() {
    assert_eq!(GLOBAL_PROJECT_FOLDER, "/");
    assert!(is_global_project_folder("/"));
    assert!(!is_global_project_folder("/proj"));
}

#[test]
fn slice52_39_drive_letter_path_is_win32() {
    assert_eq!(path_style_of("C:\\proj"), PathStyle::Win32);
    assert_eq!(path_style_of("d:/proj"), PathStyle::Win32);
}

#[test]
fn slice52_40_unc_path_is_win32() {
    assert_eq!(path_style_of("\\\\server\\share"), PathStyle::Win32);
}

#[test]
fn slice52_41_posix_path_is_posix() {
    assert_eq!(path_style_of("/srv/app"), PathStyle::Posix);
    assert_eq!(path_style_of("relative/dir"), PathStyle::Posix);
}

#[test]
fn slice52_42_names_match_case_insensitive_on_win32() {
    assert!(names_match(PathStyle::Win32, "Dist", "dist"));
    assert!(!names_match(PathStyle::Win32, "dist", "other"));
}

#[test]
fn slice52_43_names_match_exact_on_posix() {
    assert!(names_match(PathStyle::Posix, "dist", "dist"));
    assert!(!names_match(PathStyle::Posix, "Dist", "dist"));
}

#[test]
fn slice52_44_uses_windows_path_gate() {
    assert!(uses_windows_path("C:\\proj"));
    assert!(!uses_windows_path("/srv/app"));
}

#[test]
fn slice52_45_ancestor_lists_child_ignoring_trailing_separators() {
    assert!(ancestor_lists_child(
        &["/srv/app/dist/".to_string()],
        "dist",
        PathStyle::Posix
    ));
}

#[test]
fn slice52_46_ancestor_missing_child_is_false() {
    assert!(!ancestor_lists_child(
        &["/srv/app/other".to_string()],
        "dist",
        PathStyle::Posix
    ));
}

#[test]
fn slice52_47_decide_present_when_own_listing_answers() {
    assert_eq!(
        decide_folder_presence(
            &FolderListing::Entries(vec!["a".to_string()]),
            None,
            "proj",
            PathStyle::Posix
        ),
        FolderPresence::Present
    );
}

#[test]
fn slice52_48_decide_unknown_when_listing_fails_softly() {
    assert_eq!(
        decide_folder_presence(
            &FolderListing::OtherFailure,
            None,
            "proj",
            PathStyle::Posix
        ),
        FolderPresence::Unknown
    );
}

#[test]
fn slice52_49_decide_missing_when_ancestor_drops_child() {
    assert_eq!(
        decide_folder_presence(
            &FolderListing::ServerError,
            Some(&["/srv/other".to_string()]),
            "dist",
            PathStyle::Posix
        ),
        FolderPresence::Missing
    );
}

#[test]
fn slice52_50_decide_unknown_when_child_still_listed_or_no_ancestor() {
    assert_eq!(
        decide_folder_presence(
            &FolderListing::ServerError,
            Some(&["/srv/dist".to_string()]),
            "dist",
            PathStyle::Posix
        ),
        FolderPresence::Unknown
    );
    assert_eq!(
        decide_folder_presence(&FolderListing::ServerError, None, "dist", PathStyle::Posix),
        FolderPresence::Unknown
    );
}
