//! Slice 74 parity: project-service pure gates (RED).
//!
//! Mirrors the pure TS contract in `src/app/services/project-service.ts`:
//! - `isWindowsWorktreePath` (win32 platform, drive-letter / `\\` prefix)
//! - `worktreeKey` (posix/win32 normalize, trailing strip with root kept,
//!   win32 lowercase fold)
//! - `project.name || project.worktree` name fallback, `time.updated ?? 0`
//! - `getResolvedProjects` merge (API spelling wins, cached bumps
//!   `lastUpdated` to the max or appends, recency-descending stable sort)
//! - excluded-path filter under the same key
//! - `getProjectById` / `getProjectByWorktree` lookup plus the exact
//!   `Project with id/worktree ... not found` messages
//!
//! Every test MUST FAIL until `project_service_gates` is ported (step 2).

use otb_core::project_service_gates::*;

// --- resolve_project_name ---

#[test]
fn slice74_resolve_name_kept_parity() {
    assert_eq!(
        resolve_project_name(Some("Repo A"), "/home/user/repo-a"),
        "Repo A"
    );
}

#[test]
fn slice74_resolve_name_none_falls_back_parity() {
    assert_eq!(
        resolve_project_name(None, "/home/user/repo-a"),
        "/home/user/repo-a"
    );
}

#[test]
fn slice74_resolve_name_empty_falls_back_parity() {
    assert_eq!(resolve_project_name(Some(""), "/home/user/repo-a"), "/home/user/repo-a");
}

#[test]
fn slice74_resolve_name_whitespace_kept_parity() {
    // `" " || worktree` keeps the whitespace: only `None`/`""` fall back.
    assert_eq!(resolve_project_name(Some("  "), "/home/user/repo-a"), "  ");
}

// --- is_windows_worktree_path ---

#[test]
fn slice74_is_windows_on_win32_platform_parity() {
    assert!(is_windows_worktree_path("/home/user/repo", true));
}

#[test]
fn slice74_is_windows_drive_backslash_parity() {
    assert!(is_windows_worktree_path("C:\\Users\\Dev\\Repo", false));
}

#[test]
fn slice74_is_windows_drive_forward_slash_parity() {
    assert!(is_windows_worktree_path("C:/Users/Dev/Repo", false));
}

#[test]
fn slice74_is_windows_unc_prefix_parity() {
    assert!(is_windows_worktree_path("\\\\server\\share", false));
}

#[test]
fn slice74_is_windows_posix_path_parity() {
    assert!(!is_windows_worktree_path("/home/user/repo", false));
}

#[test]
fn slice74_is_windows_drive_on_posix_platform_parity() {
    // The drive-letter pattern applies even when the platform is not win32.
    assert!(is_windows_worktree_path("D:/repo-a", false));
}

// --- normalize_posix_path ---

#[test]
fn slice74_posix_identity_parity() {
    assert_eq!(normalize_posix_path("/home/user/repo"), "/home/user/repo");
}

#[test]
fn slice74_posix_collapse_repeats_parity() {
    assert_eq!(normalize_posix_path("/a//b"), "/a/b");
}

#[test]
fn slice74_posix_resolve_dot_parity() {
    assert_eq!(normalize_posix_path("/a/./b"), "/a/b");
}

#[test]
fn slice74_posix_strip_trailing_parity() {
    assert_eq!(normalize_posix_path("/home/user/repo/"), "/home/user/repo");
}

#[test]
fn slice74_posix_root_preserved_parity() {
    assert_eq!(normalize_posix_path("/"), "/");
}

// --- normalize_win32_path ---

#[test]
fn slice74_win32_identity_parity() {
    assert_eq!(
        normalize_win32_path("C:\\Users\\Dev\\Repo"),
        "C:\\Users\\Dev\\Repo"
    );
}

#[test]
fn slice74_win32_fold_forward_slashes_parity() {
    assert_eq!(
        normalize_win32_path("C:/Users/Dev/Repo"),
        "C:\\Users\\Dev\\Repo"
    );
}

#[test]
fn slice74_win32_strip_trailing_parity() {
    assert_eq!(
        normalize_win32_path("C:\\Users\\Dev\\Repo\\"),
        "C:\\Users\\Dev\\Repo"
    );
}

#[test]
fn slice74_win32_drive_root_preserved_parity() {
    assert_eq!(normalize_win32_path("C:\\"), "C:\\");
}

// --- project_worktree_key ---

#[test]
fn slice74_key_posix_identity_parity() {
    assert_eq!(
        project_worktree_key("/home/user/repo", false),
        "/home/user/repo"
    );
}

#[test]
fn slice74_key_posix_trailing_strip_parity() {
    assert_eq!(
        project_worktree_key("/home/user/repo/", false),
        "/home/user/repo"
    );
}

#[test]
fn slice74_key_posix_root_parity() {
    assert_eq!(project_worktree_key("/", false), "/");
}

#[test]
fn slice74_key_posix_normalize_keeps_case_parity() {
    assert_eq!(project_worktree_key("/A//B/./C/", false), "/A/B/C");
}

#[test]
fn slice74_key_win32_lowercase_parity() {
    assert_eq!(
        project_worktree_key("C:\\Users\\Dev\\Repo", true),
        "c:\\users\\dev\\repo"
    );
}

#[test]
fn slice74_key_win32_mixed_separators_equal_parity() {
    assert_eq!(
        project_worktree_key("C:/Users/Dev/Repo", true),
        project_worktree_key("C:\\Users\\Dev\\Repo", true)
    );
}

#[test]
fn slice74_key_win32_trailing_and_case_parity() {
    assert_eq!(
        project_worktree_key("c:/users/dev/repo/", true),
        "c:\\users\\dev\\repo"
    );
}

// --- map_api_project ---

#[test]
fn slice74_map_api_name_and_updated_parity() {
    let row = map_api_project("p1", "/r", Some("Repo"), Some(5.0));
    assert_eq!(
        row,
        InternalProject {
            id: "p1".to_string(),
            worktree: "/r".to_string(),
            name: "Repo".to_string(),
            last_updated: 5.0,
        }
    );
}

#[test]
fn slice74_map_api_empty_name_falls_back_parity() {
    let row = map_api_project("p2", "D:/repo-b", Some(""), None);
    assert_eq!(row.name, "D:/repo-b");
    assert_eq!(row.last_updated, 0.0);
}

#[test]
fn slice74_map_api_missing_name_and_updated_parity() {
    let row = map_api_project("p3", "/r", None, None);
    assert_eq!(row.name, "/r");
    assert_eq!(row.last_updated, 0.0);
}

// --- merge_projects ---

fn api_row(id: &str, worktree: &str, name: &str, updated: f64) -> InternalProject {
    InternalProject {
        id: id.to_string(),
        worktree: worktree.to_string(),
        name: name.to_string(),
        last_updated: updated,
    }
}

fn cached_row(id: &str, worktree: &str, name: &str, updated: Option<f64>) -> CachedProject {
    CachedProject {
        id: id.to_string(),
        worktree: worktree.to_string(),
        name: name.to_string(),
        last_updated: updated,
    }
}

#[test]
fn slice74_merge_cached_appends_parity() {
    let merged = merge_projects(
        vec![api_row("p1", "/a", "A", 0.0)],
        vec![cached_row("dir_1", "/c", "/c", None)],
        false,
    );
    assert_eq!(
        merged.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        vec!["p1", "dir_1"]
    );
}

#[test]
fn slice74_merge_api_and_cached_mirror_parity() {
    // Mirrors the `merges API projects with cached session directories` case:
    // `""` names fall back to the worktree, the cached duplicate of `p2`
    // folds in, insertion order survives the all-zero sort.
    let merged = merge_projects(
        vec![
            map_api_project("p1", "D:/repo-a", Some("Repo A"), None),
            map_api_project("p2", "D:/repo-b", Some(""), None),
        ],
        vec![
            cached_row("dir_1", "D:/repo-c", "D:/repo-c", None),
            cached_row("dir_2", "D:/repo-b", "D:/repo-b", None),
        ],
        false,
    );
    assert_eq!(
        merged,
        vec![
            api_row("p1", "D:/repo-a", "Repo A", 0.0),
            api_row("p2", "D:/repo-b", "D:/repo-b", 0.0),
            api_row("dir_1", "D:/repo-c", "D:/repo-c", 0.0),
        ]
    );
}

#[test]
fn slice74_merge_cached_newer_bumps_parity() {
    let merged = merge_projects(
        vec![api_row("p2", "/b", "B", 0.0)],
        vec![cached_row("dir_2", "/b", "Cached", Some(9.0))],
        false,
    );
    assert_eq!(merged, vec![api_row("p2", "/b", "B", 9.0)]);
}

#[test]
fn slice74_merge_cached_older_keeps_max_parity() {
    let merged = merge_projects(
        vec![api_row("p2", "/b", "B", 9.0)],
        vec![cached_row("dir_2", "/b", "Cached", Some(3.0))],
        false,
    );
    assert_eq!(merged, vec![api_row("p2", "/b", "B", 9.0)]);
}

#[test]
fn slice74_merge_keeps_api_identity_parity() {
    // A dup cached row bumps the timestamp but never the API id/name/worktree.
    let merged = merge_projects(
        vec![api_row("p2", "/b", "API Name", 1.0)],
        vec![cached_row("dir_9", "/b/", "Cached Name", Some(7.0))],
        false,
    );
    assert_eq!(merged, vec![api_row("p2", "/b", "API Name", 7.0)]);
}

#[test]
fn slice74_merge_sorts_recency_desc_parity() {
    let merged = merge_projects(
        vec![api_row("old", "/o", "O", 1.0), api_row("new", "/n", "N", 9.0)],
        vec![],
        false,
    );
    assert_eq!(
        merged.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        vec!["new", "old"]
    );
}

#[test]
fn slice74_merge_win32_fold_dedupes_parity() {
    // `D:/repo-b` and `d:\repo-b` share a key even on a posix platform.
    let merged = merge_projects(
        vec![api_row("p1", "D:/repo-b", "B", 0.0)],
        vec![cached_row("c1", "d:\\repo-b", "C", Some(4.0))],
        false,
    );
    assert_eq!(merged, vec![api_row("p1", "D:/repo-b", "B", 4.0)]);
}

// --- sort_projects_by_recency ---

#[test]
fn slice74_sort_recency_desc_stable_parity() {
    let mut rows = vec![
        api_row("a", "/a", "A", 1.0),
        api_row("b", "/b", "B", 3.0),
        api_row("c", "/c", "C", 2.0),
        api_row("d", "/d", "D", 3.0),
    ];
    sort_projects_by_recency(&mut rows);
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        vec!["b", "d", "c", "a"]
    );
}

// --- filter_excluded_projects ---

#[test]
fn slice74_filter_no_exclusions_parity() {
    let rows = vec![api_row("p1", "/a", "A", 0.0), api_row("p2", "/b", "B", 0.0)];
    assert_eq!(filter_excluded_projects(rows.clone(), &[], false), rows);
}

#[test]
fn slice74_filter_exact_excluded_parity() {
    let rows = vec![api_row("p1", "/a", "A", 0.0), api_row("p2", "/b", "B", 0.0)];
    assert_eq!(
        filter_excluded_projects(rows, &["/home/user/repo-b"], false)
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["p1", "p2"]
    );
    let rows = vec![api_row("p1", "/a", "A", 0.0), api_row("p2", "/b", "B", 0.0)];
    assert_eq!(
        filter_excluded_projects(rows, &["/b"], false)
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["p1"]
    );
}

#[test]
fn slice74_filter_excluded_trailing_separator_parity() {
    let rows = vec![api_row("p1", "/a", "A", 0.0), api_row("p2", "/b", "B", 0.0)];
    assert_eq!(
        filter_excluded_projects(rows, &["/b/"], false)
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["p1"]
    );
}

#[test]
fn slice74_filter_worktree_trailing_separator_parity() {
    let rows = vec![api_row("p1", "/a/", "A", 0.0), api_row("p2", "/b/", "B", 0.0)];
    assert_eq!(
        filter_excluded_projects(rows, &["/b"], false)
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["p1"]
    );
}

#[test]
fn slice74_filter_win32_case_insensitive_parity() {
    let rows = vec![
        api_row("p1", "C:\\Users\\Dev\\Repo", "Repo A", 0.0),
        api_row("p2", "C:\\Users\\Dev\\Other", "Other", 0.0),
    ];
    assert_eq!(
        filter_excluded_projects(rows, &["c:\\users\\dev\\repo"], false)
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["p2"]
    );
}

// --- find_project_by_id ---

#[test]
fn slice74_find_by_id_parity() {
    let rows = vec![api_row("p1", "/a", "A", 0.0)];
    assert_eq!(find_project_by_id(&rows, "p1"), Some(&rows[0]));
    assert_eq!(find_project_by_id(&rows, "nope"), None);
}

#[test]
fn slice74_unknown_id_message_parity() {
    assert_eq!(unknown_project_id_message("nope"), "Project with id nope not found");
}

// --- find_project_by_worktree ---

#[test]
fn slice74_find_by_worktree_parity() {
    let rows = vec![api_row("p1", "/home/user/repo", "Repo", 0.0)];
    assert_eq!(
        find_project_by_worktree(&rows, "/home/user/repo", false),
        Some(&rows[0])
    );
    assert_eq!(find_project_by_worktree(&rows, "/home/user/other", false), None);
}

#[test]
fn slice74_find_by_worktree_trailing_parity() {
    let rows = vec![api_row("p1", "/home/user/repo", "Repo", 0.0)];
    assert_eq!(
        find_project_by_worktree(&rows, "/home/user/repo/", false),
        Some(&rows[0])
    );
}

#[test]
fn slice74_unknown_worktree_message_parity() {
    assert_eq!(
        unknown_project_worktree_message("/home/user/other"),
        "Project with worktree /home/user/other not found"
    );
}

// --- to_project_info ---

#[test]
fn slice74_to_project_info_strips_parity() {
    let row = api_row("p1", "/a", "A", 9.0);
    assert_eq!(
        to_project_info(&row),
        ProjectInfo {
            id: "p1".to_string(),
            worktree: "/a".to_string(),
            name: "A".to_string(),
        }
    );
}
