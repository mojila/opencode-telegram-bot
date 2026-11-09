//! Slice 70 parity: worktree-service gates (RED).
//!
//! Mirrors the pure TS contract in:
//! - `src/app/services/worktree-service.ts` (`normalizeBranchName`,
//!   `parseGitWorktreeList`, the `readHeadBranch` ref parse,
//!   `deriveMainProjectPath`, `normalizePathKey`, the `gitdir:` pointer
//!   parse + linked-worktree marker, the `isCurrent`/`isMain` flagging,
//!   the missing-current fallback, the detached-branch backfill, the
//!   main-first sort, `listPresentWorktrees`, `findWorktreeOwner`, and the
//!   no-current fallback of `getCurrentFolderWorktreeContext`)
//!
//! Every test MUST FAIL until `worktree_service_gates` is ported (step 2).

use otb_core::worktree_service_gates::*;

fn parsed(path: &str, branch: Option<&str>) -> ParsedGitWorktreeEntry {
    ParsedGitWorktreeEntry {
        path: path.to_string(),
        branch: branch.map(|b| b.to_string()),
    }
}

fn flagged(path: &str, branch: Option<&str>, is_current: bool, is_main: bool) -> GitWorktreeEntry {
    GitWorktreeEntry {
        path: path.to_string(),
        branch: branch.map(|b| b.to_string()),
        is_current,
        is_main,
    }
}

// --- normalize_branch_name ---

#[test]
fn slice70_normalize_branch_strips_heads_prefix_parity() {
    assert_eq!(normalize_branch_name("refs/heads/main"), "main");
}

#[test]
fn slice70_normalize_branch_without_prefix_kept_parity() {
    assert_eq!(normalize_branch_name("main"), "main");
}

#[test]
fn slice70_normalize_branch_bare_prefix_empties_parity() {
    assert_eq!(normalize_branch_name("refs/heads/"), "");
}

#[test]
fn slice70_normalize_branch_nested_slashes_kept_parity() {
    assert_eq!(normalize_branch_name("refs/heads/feature/a"), "feature/a");
}

// --- parse_git_worktree_list ---

#[test]
fn slice70_parse_empty_stdout_parity() {
    assert_eq!(parse_git_worktree_list(""), Vec::new());
}

#[test]
fn slice70_parse_single_worktree_no_branch_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a"),
        vec![parsed("/a", None)]
    );
}

#[test]
fn slice70_parse_branch_strips_heads_prefix_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a\nbranch refs/heads/main\n"),
        vec![parsed("/a", Some("main"))]
    );
}

#[test]
fn slice70_parse_detached_branch_none_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a\ndetached\n"),
        vec![parsed("/a", None)]
    );
}

#[test]
fn slice70_parse_detached_clears_earlier_branch_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a\nbranch refs/heads/x\ndetached\n"),
        vec![parsed("/a", None)]
    );
}

#[test]
fn slice70_parse_branch_after_detached_wins_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a\ndetached\nbranch refs/heads/main\n"),
        vec![parsed("/a", Some("main"))]
    );
}

#[test]
fn slice70_parse_two_entries_blank_separated_parity() {
    assert_eq!(
        parse_git_worktree_list(
            "worktree /a\nbranch refs/heads/main\n\nworktree /b\nbranch refs/heads/dev\n"
        ),
        vec![parsed("/a", Some("main")), parsed("/b", Some("dev"))]
    );
}

#[test]
fn slice70_parse_worktree_line_pushes_previous_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a\nworktree /b\n"),
        vec![parsed("/a", None), parsed("/b", None)]
    );
}

#[test]
fn slice70_parse_stale_branch_lingers_onto_next_parity() {
    // No `worktree` line was open, so the blank-line push is a no-op that
    // keeps the branch; it attaches to the next record.
    assert_eq!(
        parse_git_worktree_list("branch refs/heads/x\n\nworktree /a\n"),
        vec![parsed("/a", Some("x"))]
    );
}

#[test]
fn slice70_parse_crlf_and_unknown_lines_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a\r\nfoo bar\r\nbranch refs/heads/main\r\n"),
        vec![parsed("/a", Some("main"))]
    );
}

#[test]
fn slice70_parse_path_with_spaces_kept_parity() {
    assert_eq!(
        parse_git_worktree_list("worktree /a/my dir\n"),
        vec![parsed("/a/my dir", None)]
    );
}

// --- parse_head_ref_content ---

#[test]
fn slice70_head_ref_branch_parsed_parity() {
    assert_eq!(
        parse_head_ref_content("ref: refs/heads/main\n"),
        Some("main".to_string())
    );
}

#[test]
fn slice70_head_ref_nested_branch_parsed_parity() {
    assert_eq!(
        parse_head_ref_content("ref: refs/heads/a/b"),
        Some("a/b".to_string())
    );
}

#[test]
fn slice70_head_ref_detached_hash_none_parity() {
    assert_eq!(parse_head_ref_content("abc123def456"), None);
}

#[test]
fn slice70_head_ref_empty_none_parity() {
    assert_eq!(parse_head_ref_content(""), None);
}

// --- parse_gitdir_pointer ---

#[test]
fn slice70_gitdir_pointer_parsed_parity() {
    assert_eq!(
        parse_gitdir_pointer("gitdir: /r/.git/worktrees/w1\n"),
        Some("/r/.git/worktrees/w1".to_string())
    );
}

#[test]
fn slice70_gitdir_pointer_case_insensitive_parity() {
    assert_eq!(parse_gitdir_pointer("GITDIR: /x"), Some("/x".to_string()));
}

#[test]
fn slice70_gitdir_pointer_invalid_none_parity() {
    assert_eq!(parse_gitdir_pointer("not a pointer"), None);
}

// --- git_dir_is_linked_worktree ---

#[test]
fn slice70_git_dir_linked_marker_true_parity() {
    assert!(git_dir_is_linked_worktree("/r/.git/worktrees/w1"));
}

#[test]
fn slice70_git_dir_plain_git_false_parity() {
    assert!(!git_dir_is_linked_worktree("/r/.git"));
}

#[test]
fn slice70_git_dir_marker_without_trailing_slash_false_parity() {
    assert!(!git_dir_is_linked_worktree("/r/.git/worktrees"));
}

// --- derive_main_project_path ---

#[test]
fn slice70_derive_main_dot_git_is_active_parity() {
    assert_eq!(derive_main_project_path("/r", "/r/.git"), "/r".to_string());
}

#[test]
fn slice70_derive_main_dot_git_case_insensitive_parity() {
    assert_eq!(derive_main_project_path("/r", "/r/.GIT"), "/r".to_string());
}

#[test]
fn slice70_derive_main_linked_resolves_three_up_parity() {
    assert_eq!(
        derive_main_project_path("/w/w1", "/r/.git/worktrees/w1"),
        "/r".to_string()
    );
}

#[test]
fn slice70_derive_main_unrelated_git_dir_is_active_parity() {
    assert_eq!(
        derive_main_project_path("/r", "/other/place.git"),
        "/r".to_string()
    );
}

// --- normalize_path_key ---

#[test]
fn slice70_path_key_strips_trailing_separators_parity() {
    assert_eq!(normalize_path_key("/a/b/", false), "/a/b");
}

#[test]
fn slice70_path_key_posix_keeps_case_parity() {
    assert_eq!(normalize_path_key("/A/B", false), "/A/B");
}

#[test]
fn slice70_path_key_windows_lowercases_parity() {
    assert_eq!(normalize_path_key("C:\\A\\B\\", true), "c:\\a\\b");
}

// --- flag_worktree_entries ---

#[test]
fn slice70_flag_current_and_main_parity() {
    assert_eq!(
        flag_worktree_entries(
            &[parsed("/r", Some("main")), parsed("/w", None)],
            "/w",
            "/r"
        ),
        vec![
            flagged("/r", Some("main"), false, true),
            flagged("/w", None, true, false),
        ]
    );
}

#[test]
fn slice70_flag_active_is_main_both_true_parity() {
    assert_eq!(
        flag_worktree_entries(&[parsed("/r", Some("main"))], "/r", "/r"),
        vec![flagged("/r", Some("main"), true, true)]
    );
}

#[test]
fn slice70_flag_branch_passes_through_parity() {
    assert_eq!(
        flag_worktree_entries(&[parsed("/r", Some("feature/a"))], "/x", "/r"),
        vec![flagged("/r", Some("feature/a"), false, true)]
    );
}

// --- ensure_current_entry ---

#[test]
fn slice70_ensure_current_keeps_present_parity() {
    let rows = vec![flagged("/r", Some("main"), true, true)];
    assert_eq!(
        ensure_current_entry(rows.clone(), "/r", "/r", "/r", Some("dev")),
        rows
    );
}

#[test]
fn slice70_ensure_current_appends_missing_parity() {
    assert_eq!(
        ensure_current_entry(
            vec![flagged("/r", Some("main"), false, true)],
            "/w",
            "/w",
            "/r",
            Some("dev")
        ),
        vec![
            flagged("/r", Some("main"), false, true),
            flagged("/w", Some("dev"), true, false),
        ]
    );
}

#[test]
fn slice70_ensure_current_append_marks_main_when_equal_parity() {
    assert_eq!(
        ensure_current_entry(vec![], "/r", "/r", "/r", None),
        vec![flagged("/r", None, true, true)]
    );
}

// --- sort_worktree_entries ---

#[test]
fn slice70_sort_main_first_parity() {
    let mut rows = vec![
        flagged("/w/b", None, false, false),
        flagged("/r", Some("main"), false, true),
        flagged("/w/a", None, false, false),
    ];
    sort_worktree_entries(&mut rows);
    assert_eq!(
        rows,
        vec![
            flagged("/r", Some("main"), false, true),
            flagged("/w/a", None, false, false),
            flagged("/w/b", None, false, false),
        ]
    );
}

#[test]
fn slice70_sort_equal_paths_stable_parity() {
    let mut rows = vec![
        flagged("/r", Some("a"), false, true),
        flagged("/r", Some("b"), false, true),
    ];
    sort_worktree_entries(&mut rows);
    assert_eq!(rows[0].branch, Some("a".to_string()));
    assert_eq!(rows[1].branch, Some("b".to_string()));
}

// --- backfill_detached_branch ---

#[test]
fn slice70_backfill_fills_current_missing_branch_parity() {
    let mut rows = vec![
        flagged("/r", Some("main"), false, true),
        flagged("/w", None, true, false),
    ];
    backfill_detached_branch(&mut rows, Some("dev"));
    assert_eq!(rows[1].branch, Some("dev".to_string()));
    assert_eq!(rows[0].branch, Some("main".to_string()));
}

#[test]
fn slice70_backfill_leaves_existing_branch_parity() {
    let mut rows = vec![flagged("/w", Some("kept"), true, false)];
    backfill_detached_branch(&mut rows, Some("dev"));
    assert_eq!(rows[0].branch, Some("kept".to_string()));
}

// --- filter_present_worktrees ---

#[test]
fn slice70_filter_present_keeps_git_index_parity() {
    let rows = vec![
        flagged("/r", Some("main"), false, true),
        flagged("/gone", None, false, false),
        flagged("/w", None, true, false),
    ];
    assert_eq!(
        filter_present_worktrees(&rows, &["/gone"]),
        vec![
            ListedWorktree {
                entry: flagged("/r", Some("main"), false, true),
                git_index: 0,
            },
            ListedWorktree {
                entry: flagged("/w", None, true, false),
                git_index: 2,
            },
        ]
    );
}

#[test]
fn slice70_filter_present_empty_when_all_missing_parity() {
    let rows = vec![flagged("/gone", None, false, false)];
    assert_eq!(filter_present_worktrees(&rows, &["/gone"]), Vec::new());
}

#[test]
fn slice70_filter_present_empty_input_parity() {
    assert_eq!(filter_present_worktrees(&[], &["/gone"]), Vec::new());
}

// --- find_worktree_owner_key ---

#[test]
fn slice70_owner_match_returns_main_path_parity() {
    let candidates = [OwnerCandidate {
        worktree_key: "/r",
        main_project_path: "/r",
        entries: &[OwnerEntry {
            key: "/gone",
            is_main: false,
        }],
    }];
    assert_eq!(find_worktree_owner_key("/gone", &candidates), Some("/r"));
}

#[test]
fn slice70_owner_main_row_does_not_match_parity() {
    let candidates = [OwnerCandidate {
        worktree_key: "/r",
        main_project_path: "/r",
        entries: &[OwnerEntry {
            key: "/gone",
            is_main: true,
        }],
    }];
    assert_eq!(find_worktree_owner_key("/gone", &candidates), None);
}

#[test]
fn slice70_owner_no_candidate_none_parity() {
    assert_eq!(find_worktree_owner_key("/gone", &[]), None);
}

// --- strip_current_flags ---

#[test]
fn slice70_strip_current_clears_marks_parity() {
    let rows = vec![
        flagged("/r", Some("main"), false, true),
        flagged("/w", Some("dev"), true, false),
    ];
    assert_eq!(
        strip_current_flags(&rows),
        vec![
            flagged("/r", Some("main"), false, false),
            flagged("/w", Some("dev"), false, false),
        ]
    );
}

#[test]
fn slice70_strip_current_keeps_order_parity() {
    let rows = vec![
        flagged("/w", None, true, false),
        flagged("/r", Some("main"), false, true),
    ];
    let stripped = strip_current_flags(&rows);
    assert_eq!(stripped.len(), 2);
    assert_eq!(stripped[0].path, "/w");
    assert_eq!(stripped[1].path, "/r");
    assert!(stripped.iter().all(|row| !row.is_current));
}
