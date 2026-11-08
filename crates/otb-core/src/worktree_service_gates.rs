//! Worktree-service pure gates (slice 70, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/worktree-service.ts`
//!   (`normalizeBranchName` `refs/heads/` strip, `parseGitWorktreeList`
//!   porcelain parser with blank-line records and `detached` reset,
//!   `readHeadBranch` `ref:` parse, `deriveMainProjectPath` `.git` /
//!   `.git/worktrees` branching, `normalizePathKey` trailing-separator
//!   strip + win32 lowercase, `resolveGitDir`/`isLinkedGitWorktree`
//!   `gitdir:` pointer parse + linked marker, the `isCurrent`/`isMain`
//!   flagging, the missing-current fallback append, the detached-branch
//!   backfill, the main-first worktree sort, `listPresentWorktrees`
//!   index-preserving filter, `findWorktreeOwner` owner match, and the
//!   `getCurrentFolderWorktreeContext` no-current fallback)
//!
//! No child processes (`git worktree list`), no fs reads (`stat`/`readFile`),
//! no server queries (`findMissingFolders`, `getProjects`), no container
//! checks, no `logger` calls live here — only the pure parsing, key, flag,
//! filter, and ordering cores. Callers resolve paths, read HEAD, and query
//! the OpenCode server.
//!
//! DEVIATIONS from TS (behavior-preserving for the covered cases):
//! 1. `path.resolve` (cwd joins, `.`/`..` normalization) has no pure
//!    counterpart here. Fixtures use normalized absolute paths and the
//!    caller resolves before calling; [`normalize_path_key`] keeps the
//!    trailing-separator strip and the win32 lowercase fold.
//! 2. `process.platform === "win32"` is implicit in TS (`normalizePathKey`,
//!    `GIT_WORKTREES_MARKER` via `path.sep`). The port takes an explicit
//!    `is_windows` flag (precedent: `session_cache_gates::worktree_key`)
//!    and recognizes both `/` and `\` separators everywhere, so fixtures
//!    stay portable.
//! 3. `String.localeCompare` in the worktree sort maps to byte-wise `cmp`;
//!    parity fixtures are ASCII-only where the two coincide.
//! 4. A trimmed `"worktree "` line with an empty path cannot occur in TS
//!    (`trim()` eats the trailing space, so `startsWith("worktree ")`
//!    fails); the port ignores such lines the same way.
//! 5. A `branch` line seen before any `worktree` line lingers in TS
//!    (`pushCurrent` early-returns without clearing it) and attaches to
//!    the next worktree record; the port reproduces that retention.
//! 6. `findMissingFolders` / `getProjects` / `isContainerRuntime` stay with
//!    the caller: [`filter_present_worktrees`] takes an explicit missing
//!    list, and [`find_worktree_owner_key`] takes precomputed candidates
//!    (the `"/"` + self-worktree skips included).
//! 7. `derive_main_project_path` operates on already-resolved inputs and
//!    returns the active path unchanged for the `.git`/fallback cases; the
//!    linked-worktree case folds `\` to `/` for marker detection and parent
//!    traversal, so a backslash input yields a forward-slash result.
//! 8. `readHeadBranch`'s fs read stays with the caller; only the `ref:`
//!    parse is ported ([`parse_head_ref_content`]). The `^ref:\s+(.+)$`
//!    match is case-sensitive like TS; `.` never spans newlines, so a
//!    trimmed body containing an interior newline matches nothing.
//! 9. The `gitdir:` pointer match is case-insensitive with `\s*` like TS
//!    (`/^gitdir:\s*(.+)$/i`); a body with an interior newline matches
//!    nothing, mirroring `.` without the `s` flag.

/// Branch prefix stripped from porcelain `branch` lines
/// (TS: `GIT_HEADS_PREFIX`).
pub const GIT_HEADS_PREFIX: &str = "refs/heads/";

/// Porcelain `git worktree list` record (TS: `ParsedGitWorktreeEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGitWorktreeEntry {
    pub path: String,
    pub branch: Option<String>,
}

/// Flagged worktree row (TS: `GitWorktreeEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeEntry {
    pub path: String,
    pub branch: Option<String>,
    pub is_current: bool,
    pub is_main: bool,
}

/// Listed row keeping its position in the full git list
/// (TS: `ListedWorktree`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedWorktree {
    pub entry: GitWorktreeEntry,
    pub git_index: usize,
}

/// Owner-search candidate with precomputed keys (TS: one `getProjects`
/// entry plus its `getGitWorktreeContext` rows; the `"/"` + self-worktree
/// skips from `findWorktreeOwner` apply before calling).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerCandidate<'a> {
    pub worktree_key: &'a str,
    pub main_project_path: &'a str,
    pub entries: &'a [OwnerEntry<'a>],
}

/// Worktree row as seen by the owner search (TS: `GitWorktreeEntry`
/// reduced to its match key and main flag; only `!is_main` rows match).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerEntry<'a> {
    pub key: &'a str,
    pub is_main: bool,
}

/// Strips the `refs/heads/` prefix when present, otherwise keeps the value
/// (TS: `normalizeBranchName`).
pub fn normalize_branch_name(value: &str) -> String {
    if let Some(stripped) = value.strip_prefix(GIT_HEADS_PREFIX) {
        stripped.to_string()
    } else {
        value.to_string()
    }
}

/// Parses porcelain `git worktree list` output into path/branch records
/// (TS: `parseGitWorktreeList`). Blank lines close the current record, a
/// `worktree ` line closes the previous one, `branch ` sets the current
/// branch (prefix-stripped), `detached` clears it, and anything else is
/// ignored. A trailing record is flushed at the end.
pub fn parse_git_worktree_list(stdout: &str) -> Vec<ParsedGitWorktreeEntry> {
    let mut entries: Vec<ParsedGitWorktreeEntry> = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;

    // Mirrors TS `pushCurrent`: early-returns without clearing the branch
    // when no `worktree` line is open, so a stray `branch` line lingers.
    let mut push_current = |current_path: &mut Option<String>,
                            current_branch: &mut Option<String>| {
        let Some(path) = current_path.take() else {
            return;
        };
        entries.push(ParsedGitWorktreeEntry {
            path,
            branch: current_branch.take(),
        });
    };

    for raw_line in stdout.split('\n') {
        // TS splits on /\r?\n/ then trims: strip one trailing \r via trim.
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line).trim();

        if line.is_empty() {
            push_current(&mut current_path, &mut current_branch);
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            push_current(&mut current_path, &mut current_branch);
            current_path = Some(path.to_string());
            continue;
        }

        if let Some(branch) = line.strip_prefix("branch ") {
            current_branch = Some(normalize_branch_name(branch));
            continue;
        }

        if line == "detached" {
            current_branch = None;
        }
    }

    push_current(&mut current_path, &mut current_branch);
    entries
}

/// Parses a `.git/HEAD` file body into a branch name: `ref: <target>`
/// (prefix-stripped), `None` for a detached hash or anything else
/// (TS: `readHeadBranch` after the fs read).
pub fn parse_head_ref_content(head_content: &str) -> Option<String> {
    let trimmed = head_content.trim();
    let rest = trimmed.strip_prefix("ref:")?;
    // TS `/^ref:\s+(.+)$/`: at least one whitespace after `ref:`.
    if rest.is_empty() || !rest.as_bytes()[0].is_ascii_whitespace() {
        return None;
    }
    let target = rest.trim_start();
    // `.` never matches `\n` in TS, and the body is already trimmed, so an
    // interior newline means no match.
    if target.is_empty() || target.contains('\n') || target.contains('\r') {
        return None;
    }
    Some(normalize_branch_name(target))
}

/// Parses a `.git` pointer file body (`gitdir: <path>`, case-insensitive,
/// non-empty capture) into the pointed-to git dir
/// (TS: the pointer match shared by `resolveGitDir`/`isLinkedGitWorktree`).
pub fn parse_gitdir_pointer(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.len() < 7 || !trimmed[..7].eq_ignore_ascii_case("gitdir:") {
        return None;
    }
    let target = trimmed[7..].trim();
    // Mirrors `.` without the `s` flag: no match across newlines.
    if target.is_empty() || target.contains('\n') || target.contains('\r') {
        return None;
    }
    Some(target.to_string())
}

/// Whether a resolved, lowercased, slash-folded git dir belongs to a linked
/// worktree: it contains `/.git/worktrees/`
/// (TS: `isLinkedGitWorktree` after the fs read).
pub fn git_dir_is_linked_worktree(resolved_git_dir: &str) -> bool {
    resolved_git_dir
        .replace('\\', "/")
        .to_lowercase()
        .contains("/.git/worktrees/")
}

/// Resolves which project folder owns this git dir: a `.git`-named dir
/// (case-insensitive basename) means the active worktree itself; a dir
/// under a `.git/worktrees` marker resolves three levels up; anything else
/// falls back to the active worktree (TS: `deriveMainProjectPath`).
pub fn derive_main_project_path(active_worktree_path: &str, git_dir: &str) -> String {
    // Basename check is case-insensitive in TS.
    let basename_is_dot_git = git_dir
        .split(['/', '\\'])
        .next_back()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case(".git");
    // Trailing separators would leave an empty basename; retry trimmed.
    let basename_is_dot_git = basename_is_dot_git
        || git_dir
            .trim_end_matches(['/', '\\'])
            .split(['/', '\\'])
            .next_back()
            .unwrap_or("")
            .eq_ignore_ascii_case(".git");
    if basename_is_dot_git {
        return active_worktree_path.to_string();
    }

    let folded = git_dir.replace('\\', "/");
    if folded.contains("/.git/worktrees/") {
        return parent_n_levels(&folded, 3);
    }

    active_worktree_path.to_string()
}

/// Walks `levels` parents up from a slash-folded absolute path, clamping at
/// the filesystem root (TS: `path.resolve(dir, "..", ...)`).
fn parent_n_levels(folded: &str, levels: usize) -> String {
    let mut current = folded.trim_end_matches('/').to_string();
    // Preserve the POSIX root the same way `path.resolve` would.
    if current.is_empty() {
        return "/".to_string();
    }
    for _ in 0..levels {
        match current.rfind('/') {
            Some(0) => {
                current = "/".to_string();
                break;
            }
            Some(idx) => current.truncate(idx),
            None => break,
        }
    }
    if current.is_empty() {
        "/".to_string()
    } else {
        current
    }
}

/// Normalizes a path for key comparison: strips trailing separators and,
/// on Windows, lowercases (TS: `normalizePathKey`).
pub fn normalize_path_key(value: &str, is_windows: bool) -> String {
    let stripped = value.trim_end_matches(['/', '\\']);
    if is_windows {
        stripped.to_lowercase()
    } else {
        stripped.to_string()
    }
}

/// Flags parsed rows with `isCurrent`/`isMain` by key equality against the
/// active and main keys (TS: the `worktrees` map in
/// `getGitWorktreeContext`). Branch values pass through untouched.
pub fn flag_worktree_entries(
    parsed: &[ParsedGitWorktreeEntry],
    active_key: &str,
    main_key: &str,
) -> Vec<GitWorktreeEntry> {
    parsed
        .iter()
        .map(|entry| GitWorktreeEntry {
            path: entry.path.clone(),
            branch: entry.branch.clone(),
            is_current: entry.path == active_key,
            is_main: entry.path == main_key,
        })
        .collect()
}

/// Appends the active worktree when no row is marked current, carrying the
/// HEAD branch and `isMain` when the active key equals the main key;
/// otherwise returns the rows unchanged (TS: the `currentEntry` fallback in
/// `getGitWorktreeContext`).
pub fn ensure_current_entry(
    entries: Vec<GitWorktreeEntry>,
    active_path: &str,
    active_key: &str,
    main_key: &str,
    head_branch: Option<&str>,
) -> Vec<GitWorktreeEntry> {
    if entries.iter().any(|entry| entry.is_current) {
        return entries;
    }
    let mut next = entries;
    next.push(GitWorktreeEntry {
        path: active_path.to_string(),
        branch: head_branch.map(|branch| branch.to_string()),
        is_current: true,
        is_main: active_key == main_key,
    });
    next
}

/// Fills the current row's missing branch from HEAD; rows that already name
/// a branch and non-current rows stay untouched (TS: the `else if
/// (!currentEntry.branch)` backfill in `getGitWorktreeContext`).
pub fn backfill_detached_branch(
    entries: &mut Vec<GitWorktreeEntry>,
    head_branch: Option<&str>,
) {
    let Some(head_branch) = head_branch else {
        return;
    };
    for entry in entries.iter_mut() {
        if entry.is_current && entry.branch.is_none() {
            entry.branch = Some(head_branch.to_string());
        }
    }
}

/// Orders rows main-first, then by path ascending (TS: the `worktrees.sort`
/// in `getGitWorktreeContext`).
pub fn sort_worktree_entries(entries: &mut Vec<GitWorktreeEntry>) {
    // `sort_by` is stable in Rust, preserving input order for equal paths
    // the way TS returns `0` does. Byte-wise `cmp` coincides with
    // `localeCompare` on the ASCII-only parity fixtures.
    entries.sort_by(|left, right| {
        if left.is_main != right.is_main {
            return if left.is_main {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        left.path.cmp(&right.path)
    });
}

/// Keeps the rows whose folder is not reported missing, preserving each
/// row's index in the full git list (TS: `listPresentWorktrees`).
pub fn filter_present_worktrees(
    entries: &[GitWorktreeEntry],
    missing_paths: &[&str],
) -> Vec<ListedWorktree> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| !missing_paths.contains(&entry.path.as_str()))
        .map(|(git_index, entry)| ListedWorktree {
            entry: entry.clone(),
            git_index,
        })
        .collect()
}

/// Finds the main project path of the first candidate listing the folder as
/// a non-main row, or `None` when no candidate does (TS: the owner match in
/// `findWorktreeOwner`; the `"/"` + self-worktree skips and the empty-owner
/// `null` apply here too).
pub fn find_worktree_owner_key<'a>(
    folder_key: &str,
    candidates: &'a [OwnerCandidate<'a>],
) -> Option<&'a str> {
    candidates
        .iter()
        .find(|candidate| {
            candidate
                .entries
                .iter()
                .any(|entry| !entry.is_main && entry.key == folder_key)
        })
        .map(|candidate| candidate.main_project_path)
}

/// Clears every row's current and main marks while keeping path, branch,
/// and order (TS: the no-current fallback map in
/// `getCurrentFolderWorktreeContext`).
pub fn strip_current_flags(entries: &[GitWorktreeEntry]) -> Vec<GitWorktreeEntry> {
    entries
        .iter()
        .map(|entry| GitWorktreeEntry {
            path: entry.path.clone(),
            branch: entry.branch.clone(),
            is_current: false,
            is_main: false,
        })
        .collect()
}
