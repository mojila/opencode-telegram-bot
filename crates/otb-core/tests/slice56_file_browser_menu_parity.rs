//! Parity port of the file-browser menu + callback pure gates
//! (slice 56, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/bot/menus/file-browser-menu.ts` (`OPEN_*` / `LS_*` prefixes,
//!   `truncate_label`, the path-index encode/decode, the `|<page>`
//!   pagination encode/decode, `build_ls_entry_label`, `escape_html`,
//!   `total_pages`, `has_browse_actions`, `can_attach_file`)
//! - `src/bot/callbacks/file-browser-callback-handler.ts`
//!   (`classify_open_callback`, `classify_ls_callback`,
//!   `resolve_initial_ls_directory`)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::file_browser_menu_gates::*;

// --- prefixes ---

#[test]
fn open_callback_prefix_values() {
    assert_eq!(OPEN_CALLBACK_PREFIX, "open:");
    assert_eq!(OPEN_CALLBACK_NAV_PREFIX, "open:nav:");
    assert_eq!(OPEN_CALLBACK_SELECT_PREFIX, "open:sel:");
    assert_eq!(OPEN_CALLBACK_PAGE_PREFIX, "open:pg:");
    assert_eq!(OPEN_CALLBACK_ROOTS, "open:roots");
}

#[test]
fn ls_callback_prefix_values() {
    assert_eq!(LS_CALLBACK_PREFIX, "ls:");
    assert_eq!(LS_CALLBACK_NAV_PREFIX, "ls:nav:");
    assert_eq!(LS_CALLBACK_FILE_PREFIX, "ls:file:");
    assert_eq!(LS_CALLBACK_DOWNLOAD_PREFIX, "ls:download:");
    assert_eq!(LS_CALLBACK_ATTACH_PREFIX, "ls:attach:");
    assert_eq!(LS_CALLBACK_BACK_PREFIX, "ls:back:");
    assert_eq!(LS_CALLBACK_PAGE_PREFIX, "ls:pg:");
}

#[test]
fn max_button_label_length_is_64() {
    assert_eq!(MAX_BUTTON_LABEL_LENGTH, 64);
}

// --- truncate_label ---

#[test]
fn short_label_passes_through() {
    assert_eq!(truncate_label("hello"), "hello");
}

#[test]
fn exact_64_label_passes_through() {
    let label = "a".repeat(64);
    assert_eq!(truncate_label(&label), label);
}

#[test]
fn overlong_label_cuts_with_ellipsis() {
    assert_eq!(truncate_label(&"a".repeat(65)), format!("{}...", "a".repeat(61)));
}

#[test]
fn custom_max_cut() {
    assert_eq!(truncate_label_with_max("abcdef", 5), "ab...");
}

// --- path-index encode ---

#[test]
fn short_path_encodes_inline() {
    let mut index = PathIndex::new();
    assert_eq!(
        encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, "/home/u/docs", 0),
        "open:nav:/home/u/docs"
    );
}

#[test]
fn exact_budget_path_encodes_inline() {
    let mut index = PathIndex::new();
    // "open:nav:" is 9 bytes; 9 + 55 = 64 fits exactly.
    let path = "p".repeat(55);
    assert_eq!(
        encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &path, 0),
        format!("open:nav:{}", path)
    );
}

#[test]
fn one_byte_over_uses_index_key() {
    let mut index = PathIndex::new();
    let path = "p".repeat(56);
    assert_eq!(
        encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &path, 0),
        "open:nav:#0"
    );
}

#[test]
fn second_overflow_increments_key() {
    let mut index = PathIndex::new();
    let first = "a".repeat(60);
    let second = "b".repeat(60);
    assert_eq!(
        encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &first, 0),
        "open:nav:#0"
    );
    assert_eq!(
        encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &second, 0),
        "open:nav:#1"
    );
}

#[test]
fn reserve_bytes_force_index() {
    let mut index = PathIndex::new();
    // 9 + 50 + 5 = 64 fits; 9 + 51 + 5 = 65 does not.
    assert_eq!(
        encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &"p".repeat(50), 5),
        format!("open:nav:{}", "p".repeat(50))
    );
    assert_eq!(
        encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &"p".repeat(51), 5),
        "open:nav:#0"
    );
}

#[test]
fn same_long_path_twice_gets_two_keys() {
    let mut index = PathIndex::new();
    let path = "p".repeat(60);
    let first = encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &path, 0);
    let second = encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &path, 0);
    assert_eq!(first, "open:nav:#0");
    assert_eq!(second, "open:nav:#1");
}

// --- path-index decode ---

#[test]
fn decode_inline_path_passthrough() {
    let index = PathIndex::new();
    assert_eq!(
        decode_path_from_callback(&index, OPEN_CALLBACK_NAV_PREFIX, "open:nav:/home/u/docs"),
        Some("/home/u/docs".to_string())
    );
}

#[test]
fn decode_indexed_path_resolves() {
    let mut index = PathIndex::new();
    let path = "p".repeat(60);
    let encoded = encode_path_for_callback(&mut index, OPEN_CALLBACK_NAV_PREFIX, &path, 0);
    assert_eq!(decode_path_from_callback(&index, OPEN_CALLBACK_NAV_PREFIX, &encoded), Some(path));
}

#[test]
fn decode_unknown_key_returns_none() {
    let index = PathIndex::new();
    assert_eq!(decode_path_from_callback(&index, OPEN_CALLBACK_NAV_PREFIX, "open:nav:#7"), None);
}

#[test]
fn decode_wrong_prefix_returns_none() {
    let index = PathIndex::new();
    assert_eq!(decode_path_from_callback(&index, OPEN_CALLBACK_NAV_PREFIX, "open:sel:/x"), None);
    assert_eq!(decode_path_from_callback(&index, OPEN_CALLBACK_NAV_PREFIX, "ls:nav:/x"), None);
}

// --- pagination encode/decode ---

#[test]
fn page_encode_appends_pipe_page() {
    let mut index = PathIndex::new();
    assert_eq!(
        encode_path_with_page_callback(&mut index, LS_CALLBACK_PAGE_PREFIX, "/a", 2),
        "ls:pg:/a|2"
    );
}

#[test]
fn page_encode_long_path_uses_index() {
    let mut index = PathIndex::new();
    let path = "p".repeat(60);
    assert_eq!(
        encode_path_with_page_callback(&mut index, LS_CALLBACK_PAGE_PREFIX, &path, 3),
        "ls:pg:#0|3"
    );
}

#[test]
fn open_page_decode_simple() {
    let index = PathIndex::new();
    assert_eq!(
        decode_open_pagination_callback(&index, "open:pg:/a|2"),
        Some(PathWithPage { path: "/a".to_string(), page: 2 })
    );
}

#[test]
fn page_decode_keeps_pipe_in_path() {
    // lastIndexOf: the final segment is the page.
    let index = PathIndex::new();
    assert_eq!(
        decode_ls_pagination_callback(&index, "ls:pg:/a|b|2"),
        Some(PathWithPage { path: "/a|b".to_string(), page: 2 })
    );
}

#[test]
fn page_decode_indexed_ref() {
    let mut index = PathIndex::new();
    let path = "p".repeat(60);
    let encoded = encode_path_with_page_callback(&mut index, LS_CALLBACK_PAGE_PREFIX, &path, 3);
    assert_eq!(
        decode_ls_pagination_callback(&index, &encoded),
        Some(PathWithPage { path, page: 3 })
    );
}

#[test]
fn ls_file_decode_reads_page_suffix() {
    let index = PathIndex::new();
    assert_eq!(
        decode_ls_file_callback(&index, "ls:file:/notes.txt|1"),
        Some(PathWithPage { path: "/notes.txt".to_string(), page: 1 })
    );
}

#[test]
fn ls_back_decode_reads_page_suffix() {
    let index = PathIndex::new();
    assert_eq!(
        decode_ls_back_callback(&index, "ls:back:/proj|0"),
        Some(PathWithPage { path: "/proj".to_string(), page: 0 })
    );
}

#[test]
fn ls_attach_decode_reads_plain_path() {
    let index = PathIndex::new();
    assert_eq!(
        decode_ls_attach_callback(&index, "ls:attach:/notes.txt"),
        Some("/notes.txt".to_string())
    );
}

#[test]
fn page_decode_wrong_prefix_none() {
    let index = PathIndex::new();
    assert_eq!(decode_open_pagination_callback(&index, "open:nav:/a|2"), None);
    assert_eq!(decode_ls_pagination_callback(&index, "open:pg:/a|2"), None);
}

#[test]
fn page_decode_missing_separator_none() {
    let index = PathIndex::new();
    assert_eq!(decode_ls_pagination_callback(&index, "ls:pg:/noseparator"), None);
}

#[test]
fn page_decode_non_numeric_none() {
    let index = PathIndex::new();
    assert_eq!(decode_ls_pagination_callback(&index, "ls:pg:/a|x"), None);
}

#[test]
fn page_decode_empty_page_none() {
    let index = PathIndex::new();
    assert_eq!(decode_ls_pagination_callback(&index, "ls:pg:/a|"), None);
}

#[test]
fn page_decode_trailing_garbage_tolerated() {
    // JS parseInt("3x") reads 3.
    let index = PathIndex::new();
    assert_eq!(
        decode_ls_pagination_callback(&index, "ls:pg:/a|3x"),
        Some(PathWithPage { path: "/a".to_string(), page: 3 })
    );
}

#[test]
fn page_decode_negative_page() {
    let index = PathIndex::new();
    assert_eq!(
        decode_ls_pagination_callback(&index, "ls:pg:/a|-1"),
        Some(PathWithPage { path: "/a".to_string(), page: -1 })
    );
}

// --- dispatch classification ---

#[test]
fn classify_open_roots() {
    assert_eq!(classify_open_callback("open:roots"), Some(OpenCallbackKind::Roots));
}

#[test]
fn classify_open_nav_page_select() {
    assert_eq!(classify_open_callback("open:nav:/x"), Some(OpenCallbackKind::Nav));
    assert_eq!(classify_open_callback("open:pg:/x|0"), Some(OpenCallbackKind::Page));
    assert_eq!(classify_open_callback("open:sel:/x"), Some(OpenCallbackKind::Select));
}

#[test]
fn classify_open_unknown() {
    assert_eq!(classify_open_callback("open:bogus"), Some(OpenCallbackKind::Unknown));
}

#[test]
fn classify_open_foreign_none() {
    assert_eq!(classify_open_callback("ls:nav:/x"), None);
    assert_eq!(classify_open_callback(""), None);
}

#[test]
fn classify_ls_nav_page_file() {
    assert_eq!(classify_ls_callback("ls:nav:/x"), Some(LsCallbackKind::Nav));
    assert_eq!(classify_ls_callback("ls:pg:/x|0"), Some(LsCallbackKind::Page));
    assert_eq!(classify_ls_callback("ls:file:/f|0"), Some(LsCallbackKind::File));
}

#[test]
fn classify_ls_attach_download_back() {
    assert_eq!(classify_ls_callback("ls:attach:/f"), Some(LsCallbackKind::Attach));
    assert_eq!(classify_ls_callback("ls:download:/f"), Some(LsCallbackKind::Download));
    assert_eq!(classify_ls_callback("ls:back:/d|0"), Some(LsCallbackKind::Back));
}

#[test]
fn classify_ls_unknown_and_foreign() {
    assert_eq!(classify_ls_callback("ls:bogus"), Some(LsCallbackKind::Unknown));
    assert_eq!(classify_ls_callback("open:roots"), None);
}

// --- labels ---

#[test]
fn ls_directory_entry_label() {
    assert_eq!(build_ls_entry_label(true, "src"), "📁 src");
}

#[test]
fn ls_file_entry_label() {
    assert_eq!(build_ls_entry_label(false, "notes.txt"), "📄 notes.txt");
}

#[test]
fn escape_html_basic() {
    assert_eq!(escape_html("<b>hi</b>"), "&lt;b&gt;hi&lt;/b&gt;");
}

#[test]
fn escape_html_ampersand_first() {
    // "&lt;" must become "&amp;lt;", not double-processed back.
    assert_eq!(escape_html("&lt;"), "&amp;lt;");
    assert_eq!(escape_html("a&b"), "a&amp;b");
}

#[test]
fn escape_html_quotes() {
    assert_eq!(escape_html("\"it's\""), "&quot;it&#39;s&quot;");
}

// --- pagination / browse gates ---

#[test]
fn total_pages_boundaries() {
    assert_eq!(total_pages(0), 1);
    assert_eq!(total_pages(8), 1);
    assert_eq!(total_pages(9), 2);
    assert_eq!(total_pages(17), 3);
}

#[test]
fn browse_actions_with_entries_true() {
    assert!(has_browse_actions(5, false, false));
}

#[test]
fn browse_actions_parent_non_root_true() {
    assert!(has_browse_actions(0, true, false));
}

#[test]
fn browse_actions_empty_root_or_no_parent_false() {
    assert!(!has_browse_actions(0, true, true));
    assert!(!has_browse_actions(0, false, false));
}

#[test]
fn can_attach_true_only_when_both() {
    assert!(can_attach_file(true, true));
    assert!(!can_attach_file(false, true));
    assert!(!can_attach_file(true, false));
    assert!(!can_attach_file(false, false));
}

// --- initial ls directory ---

#[test]
fn resolve_initial_no_project_none() {
    assert_eq!(resolve_initial_ls_directory(None, Some("/cached"), true), None);
    assert_eq!(resolve_initial_ls_directory(None, None, false), None);
}

#[test]
fn resolve_initial_prefers_cached_within_project() {
    assert_eq!(
        resolve_initial_ls_directory(Some("/proj"), Some("/proj/sub"), true),
        Some("/proj/sub".to_string())
    );
    assert_eq!(
        resolve_initial_ls_directory(Some("/proj"), Some("/elsewhere"), false),
        Some("/proj".to_string())
    );
    assert_eq!(
        resolve_initial_ls_directory(Some("/proj"), None, false),
        Some("/proj".to_string())
    );
}
