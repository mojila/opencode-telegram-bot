//! Slice 90 parity: file-browser service pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/app/services/file-browser-service.ts` (the `~` display fold,
//! hidden-dir filter, base-sensitivity sorts, `totalPages` / `safePage`
//! clamps, `📁 `/`📂 ` labels, tilde expansion, roots split, allowed-root
//! and project-root membership, posix/win32 path dispatch, errno mapping,
//! ls dirs-first ordering, `hasParent`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `file_browser_service_gates` is ported (only the constant-value tests
//! pass).

use otb_core::file_browser_service_gates::*;

// ---- constants ----

#[test]
fn max_entries_per_page_value_parity() {
    assert_eq!(MAX_ENTRIES_PER_PAGE, 8);
}

#[test]
fn entry_folder_prefix_value_parity() {
    assert_eq!(ENTRY_FOLDER_PREFIX, "📁 ");
}

#[test]
fn tree_header_prefix_value_parity() {
    assert_eq!(TREE_HEADER_PREFIX, "📂 ");
}

// ---- pathToDisplayPath ----

#[test]
fn display_exact_home_parity() {
    assert_eq!(path_to_display_path("/home/u", "/home/u"), "~");
}

#[test]
fn display_under_home_parity() {
    assert_eq!(path_to_display_path("/home/u/docs", "/home/u"), "~/docs");
}

#[test]
fn display_outside_home_parity() {
    assert_eq!(path_to_display_path("/etc/hosts", "/home/u"), "/etc/hosts");
}

// ---- pagination ----

#[test]
fn total_pages_zero_is_one_parity() {
    assert_eq!(total_pages(0), 1);
}

#[test]
fn total_pages_exact_page_is_one_parity() {
    assert_eq!(total_pages(8), 1);
}

#[test]
fn total_pages_overflow_is_two_parity() {
    assert_eq!(total_pages(9), 2);
}

#[test]
fn clamp_negative_is_zero_parity() {
    assert_eq!(clamp_page(-3, 3), 0);
}

#[test]
fn clamp_over_is_last_parity() {
    assert_eq!(clamp_page(9, 2), 1);
}

#[test]
fn slice_first_page_parity() {
    assert_eq!(page_slice_range(0), (0, 8));
}

#[test]
fn slice_third_page_parity() {
    assert_eq!(page_slice_range(2), (16, 24));
}

// ---- scan filter + sort + labels ----

#[test]
fn sorted_case_insensitive_parity() {
    let names = vec!["b".to_string(), "A".to_string(), "c".to_string()];
    assert_eq!(sorted_dir_names(&names), vec!["A", "b", "c"]);
}

#[test]
fn sorted_empty_parity() {
    let names: Vec<String> = vec![];
    assert_eq!(sorted_dir_names(&names), Vec::<String>::new());
}

#[test]
fn visible_dir_true_parity() {
    assert!(is_visible_dir("docs", true));
}

#[test]
fn visible_hidden_false_parity() {
    assert!(!is_visible_dir(".git", true));
}

#[test]
fn visible_file_false_parity() {
    assert!(!is_visible_dir("a.txt", false));
}

#[test]
fn entry_label_parity() {
    assert_eq!(build_entry_label("docs"), "📁 docs");
}

#[test]
fn header_single_page_parity() {
    assert_eq!(tree_header_base("~/x", 0, 1), "📂 ~/x");
}

#[test]
fn header_multi_page_parity() {
    assert_eq!(tree_header_base("~/x", 1, 3), "📂 ~/x  (2/3)");
}

#[test]
fn count_empty_parity() {
    assert_eq!(tree_count_kind(0), TreeCount::Empty);
}

#[test]
fn count_one_parity() {
    assert_eq!(tree_count_kind(1), TreeCount::One);
}

#[test]
fn count_many_parity() {
    assert_eq!(tree_count_kind(5), TreeCount::Many);
}

// ---- tilde + roots ----

#[test]
fn tilde_bare_parity() {
    assert_eq!(expand_tilde("~", "/home/u"), "/home/u");
}

#[test]
fn tilde_slash_parity() {
    assert_eq!(expand_tilde("~/docs", "/home/u"), "/home/u/docs");
}

#[test]
fn roots_split_mixed_parity() {
    assert_eq!(split_browser_roots("a, b,, c "), vec!["a", "b", "c"]);
}

// ---- path flavor dispatch ----

#[test]
fn win_drive_backslash_parity() {
    assert!(uses_windows_path("C:\\a"));
}

#[test]
fn win_unc_parity() {
    assert!(uses_windows_path("\\\\srv\\s"));
}

#[test]
fn win_posix_false_parity() {
    assert!(!uses_windows_path("/home/u"));
}

#[test]
fn join_posix_parity() {
    assert_eq!(join_path("/a/b", "c"), "/a/b/c");
}

#[test]
fn join_win_parity() {
    assert_eq!(join_path("C:\\a", "b"), "C:\\a\\b");
}

#[test]
fn base_posix_parity() {
    assert_eq!(base_name("/a/b/c.txt"), "c.txt");
}

#[test]
fn base_win_parity() {
    assert_eq!(base_name("C:\\a\\b"), "b");
}

#[test]
fn parent_posix_parity() {
    assert_eq!(parent_path("/a/b/c"), "/a/b");
}

#[test]
fn parent_win_parity() {
    assert_eq!(parent_path("C:\\a\\b"), "C:\\a");
}

#[test]
fn root_posix_parity() {
    assert_eq!(root_path("/a/b"), "/");
}

// ---- membership ----

#[test]
fn same_equal_parity() {
    assert!(is_same_path("/a/b", "/a/b"));
}

#[test]
fn same_different_parity() {
    assert!(!is_same_path("/a/b", "/a/c"));
}

#[test]
fn within_child_parity() {
    assert!(is_path_within_directory("/a/b/c", "/a/b"));
}

#[test]
fn within_self_parity() {
    assert!(is_path_within_directory("/a/b", "/a/b"));
}

#[test]
fn within_sibling_false_parity() {
    assert!(!is_path_within_directory("/a/b2", "/a/b"));
}

#[test]
fn allowed_exact_parity() {
    assert!(is_within_allowed_root("/a", &["/a".to_string()], false));
}

#[test]
fn allowed_child_parity() {
    assert!(is_within_allowed_root("/a/b", &["/a".to_string()], false));
}

#[test]
fn allowed_outside_false_parity() {
    assert!(!is_within_allowed_root("/b", &["/a".to_string()], false));
}

// ---- errno + ls ordering + parent ----

#[test]
fn scan_code_enoent_parity() {
    assert_eq!(map_scan_error_code("ENOENT"), ScanErrorCode::NotFound);
}

#[test]
fn scan_code_unknown_parity() {
    assert_eq!(map_scan_error_code("BOGUS"), ScanErrorCode::Unknown);
}

#[test]
fn sort_ls_dirs_first_parity() {
    let entries = vec![
        LsEntry {
            name: "b.txt".to_string(),
            full_path: "/w/b.txt".to_string(),
            is_dir: false,
        },
        LsEntry {
            name: "zeta".to_string(),
            full_path: "/w/zeta".to_string(),
            is_dir: true,
        },
        LsEntry {
            name: "A.txt".to_string(),
            full_path: "/w/A.txt".to_string(),
            is_dir: false,
        },
        LsEntry {
            name: "alpha".to_string(),
            full_path: "/w/alpha".to_string(),
            is_dir: true,
        },
    ];
    let names: Vec<String> = sort_ls_entries(entries)
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(names, vec!["alpha", "zeta", "A.txt", "b.txt"]);
}

#[test]
fn has_parent_child_true_parity() {
    assert!(has_parent("/a/b", "/"));
}

#[test]
fn has_parent_root_false_parity() {
    assert!(!has_parent("/", "/"));
}
