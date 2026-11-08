//! Project-service pure gates (slice 74, GREEN).
//!
//! Rust port of the pure remainder in `src/app/services/project-service.ts`:
//! - `isWindowsWorktreePath` (win32 platform, drive-letter or `\\` prefix)
//! - `worktreeKey` (posix/win32 lexical normalize, trailing-separator strip
//!   with the root preserved, win32 lowercase fold)
//! - the `project.name || project.worktree` name fallback at API mapping
//! - the `getResolvedProjects` merge core (API rows keep id/name/worktree,
//!   cached rows bump `lastUpdated` to the max or append, recency-descending
//!   stable sort; linked-worktree / missing-folder filtering stays with the
//!   caller)
//! - the excluded-path filter (key-set membership after the same key)
//! - `getProjectById` / `getProjectByWorktree` lookup plus the exact
//!   `Project with id/worktree ... not found` messages
//!
//! No OpenCode clients (`project.list`), no filesystem reads (the `.git`
//! pointer `stat`/`readFile`), no `config.bot.excludedProjectPaths` lookup,
//! and no `logger` calls live here — only the key, merge, filter, and lookup
//! cores the service drives. Git-pointer parsing already lives in
//! `worktree_service_gates` (`parse_gitdir_pointer`,
//! `git_dir_is_linked_worktree`); the session-cache fold-only key lives in
//! `session_cache_gates::worktree_key` (no normalize/strip there).
//!
//! DEVIATIONS from TS (behavior-preserving for the covered cases):
//! 1. `process.platform === "win32"` is implicit in TS (`worktreeKey`,
//!    `isWindowsWorktreePath`). The port takes an explicit `is_windows` /
//!    `is_win32_platform` flag (precedent:
//!    `session_cache_gates::worktree_key`). `project_worktree_key` treats its
//!    flag as the platform flag and additionally sniffs the drive-letter /
//!    `\\` prefix per path, exactly like TS `worktreeKey` calling
//!    `isWindowsWorktreePath(worktree)`.
//! 2. `path.posix.normalize` / `path.win32.normalize` have no `std`
//!    counterpart; [`normalize_posix_path`] / [`normalize_win32_path`] are
//!    lexical (separator collapse, `.`/`..` resolution, trailing-separator
//!    strip with the filesystem / drive root preserved). Drive-relative
//!    (`C:foo`) and other exotic inputs are out of scope.
//! 3. `Array.sort` stability maps to `Vec::sort_by` (stable in Rust); the
//!    recency-descending comparator keeps ties in insertion order like TS.
//! 4. [`resolve_project_name`] treats only `None` and `""` as missing — a
//!    whitespace-only name is truthy in TS (`name || worktree`) and passes
//!    through unchanged.
//!
//! Step 2 (GREEN): pure sync logic port; no I/O, no timers.

use std::collections::{HashMap, HashSet};

/// Resolved project row (TS: `InternalProject` in `project-service.ts`).
#[derive(Debug, Clone, PartialEq)]
pub struct InternalProject {
    pub id: String,
    pub worktree: String,
    pub name: String,
    pub last_updated: f64,
}

/// Cached session directory row (TS: the `getCachedSessionProjects` element
/// shape: `id` / `worktree` / `name`, optional `lastUpdated`).
#[derive(Debug, Clone, PartialEq)]
pub struct CachedProject {
    pub id: String,
    pub worktree: String,
    pub name: String,
    pub last_updated: Option<f64>,
}

/// Public project row (TS: `ProjectInfo` in `src/app/types/project.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    pub id: String,
    pub worktree: String,
    pub name: String,
}

/// Whether a worktree path counts as Windows-style: the platform is win32, or
/// the path carries a drive letter (`C:\`, `C:/`) or a `\\` prefix
/// (TS: `isWindowsWorktreePath`).
pub fn is_windows_worktree_path(worktree: &str, is_win32_platform: bool) -> bool {
    if is_win32_platform {
        return true;
    }
    let bytes = worktree.as_bytes();
    if bytes.len() >= 3
        && (bytes[0] as char).is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    worktree.starts_with("\\\\")
}

/// Lexical POSIX normalization: collapse repeats, resolve `.`/`..`, strip
/// one trailing `/` run unless the value is the `/` root
/// (TS: `path.posix.normalize` inside `worktreeKey`).
pub fn normalize_posix_path(value: &str) -> String {
    if value.is_empty() {
        return ".".to_string();
    }
    let is_absolute = value.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for comp in value.split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        } else if comp == ".." {
            match parts.last() {
                Some(last) if *last != ".." => {
                    parts.pop();
                }
                Some(_) => {
                    parts.push(comp);
                }
                None => {
                    if !is_absolute {
                        parts.push(comp);
                    }
                }
            }
        } else {
            parts.push(comp);
        }
    }
    if is_absolute {
        if parts.is_empty() {
            return "/".to_string();
        }
        let mut out = String::with_capacity(value.len());
        out.push('/');
        out.push_str(&parts.join("/"));
        out
    } else if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

fn resolve_win32_components<'a>(raw: &[&'a str], absolute: bool) -> Vec<&'a str> {
    let mut parts: Vec<&'a str> = Vec::new();
    for comp in raw {
        if comp.is_empty() || *comp == "." {
            continue;
        } else if *comp == ".." {
            match parts.last() {
                Some(last) if *last != ".." => {
                    parts.pop();
                }
                Some(_) => {
                    parts.push(comp);
                }
                None => {
                    if !absolute {
                        parts.push(comp);
                    }
                }
            }
        } else {
            parts.push(comp);
        }
    }
    parts
}

/// Lexical Win32 normalization: fold `/` to `\`, collapse repeats, resolve
/// `.`/`..`, strip one trailing separator run unless the value is a drive
/// (`C:\`) or UNC root (TS: `path.win32.normalize` inside `worktreeKey`).
pub fn normalize_win32_path(value: &str) -> String {
    if value.is_empty() {
        return ".".to_string();
    }
    let folded = value.replace('/', "\\");
    let bytes = folded.as_bytes();
    let is_drive =
        folded.len() >= 2 && (bytes[0] as char).is_ascii_alphabetic() && bytes[1] == b':';

    if folded.starts_with("\\\\") {
        let trimmed = folded.trim_start_matches('\\');
        let mut comps = trimmed.split('\\').filter(|s| !s.is_empty());
        let server = comps.next();
        let share = comps.next();
        match (server, share) {
            (Some(srv), Some(shr)) => {
                let remaining: Vec<&str> = comps.collect();
                let resolved = resolve_win32_components(&remaining, true);
                let prefix = format!("\\\\{srv}\\{shr}\\");
                if resolved.is_empty() {
                    return prefix;
                }
                return format!("{prefix}{}", resolved.join("\\"));
            }
            _ => return "\\".to_string(),
        }
    }

    if is_drive {
        if folded.len() >= 3 && bytes[2] == b'\\' {
            let prefix = folded[..3].to_string();
            let raw: Vec<&str> = folded[3..].split('\\').collect();
            let resolved = resolve_win32_components(&raw, true);
            if resolved.is_empty() {
                return prefix;
            }
            return format!("{prefix}{}", resolved.join("\\"));
        }
        let prefix = folded[..2].to_string();
        let rest = if folded.len() > 2 { &folded[2..] } else { "" };
        let raw: Vec<&str> = rest.split('\\').collect();
        let resolved = resolve_win32_components(&raw, false);
        if resolved.is_empty() {
            return prefix;
        }
        return format!("{prefix}{}", resolved.join("\\"));
    }

    if folded.starts_with('\\') {
        let rest = folded.trim_start_matches('\\');
        let raw: Vec<&str> = rest.split('\\').collect();
        let resolved = resolve_win32_components(&raw, true);
        if resolved.is_empty() {
            return "\\".to_string();
        }
        return format!("\\{}", resolved.join("\\"));
    }

    let raw: Vec<&str> = folded.split('\\').collect();
    let resolved = resolve_win32_components(&raw, false);
    if resolved.is_empty() {
        return ".".to_string();
    }
    resolved.join("\\")
}

fn win32_root(normalized: &str) -> String {
    if normalized.starts_with("\\\\") {
        let trimmed = normalized.trim_start_matches('\\');
        let mut it = trimmed.split('\\');
        match (it.next(), it.next()) {
            (Some(srv), Some(shr)) if !srv.is_empty() && !shr.is_empty() => {
                return format!("\\\\{srv}\\{shr}\\");
            }
            _ => return "\\".to_string(),
        }
    }
    let bytes = normalized.as_bytes();
    if normalized.len() >= 3
        && (bytes[0] as char).is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'\\'
    {
        return normalized[..3].to_string();
    }
    if normalized.len() >= 2 && (bytes[0] as char).is_ascii_alphabetic() && bytes[1] == b':' {
        return normalized[..2].to_string();
    }
    if normalized.starts_with('\\') {
        return "\\".to_string();
    }
    String::new()
}

/// Platform-folded project key: normalized, trailing separators stripped
/// (root preserved), lowercased on Windows (TS: `worktreeKey`).
///
/// The `is_windows` flag is the `process.platform === "win32"` stand-in; the
/// per-path drive-letter / `\\` sniff from [`is_windows_worktree_path`] still
/// applies, so Windows-looking paths fold even on a posix platform.
pub fn project_worktree_key(worktree: &str, is_windows: bool) -> String {
    let win = is_windows_worktree_path(worktree, is_windows);
    if win {
        let normalized = normalize_win32_path(worktree);
        let root = win32_root(&normalized);
        let trimmed = if normalized == root {
            normalized
        } else {
            normalized.trim_end_matches(['\\', '/']).to_string()
        };
        trimmed.to_lowercase()
    } else {
        let normalized = normalize_posix_path(worktree);
        let root = if normalized.starts_with('/') {
            "/".to_string()
        } else {
            String::new()
        };
        if normalized == root {
            normalized
        } else {
            normalized.trim_end_matches(['\\', '/']).to_string()
        }
    }
}

/// API name mapping: a missing or empty name falls back to the worktree
/// (TS: `project.name || project.worktree`).
pub fn resolve_project_name(name: Option<&str>, worktree: &str) -> String {
    match name {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => worktree.to_string(),
    }
}

/// Maps one OpenCode API project to a resolved row: name fallback plus
/// `time.updated ?? 0` (TS: the `apiProjects` mapping).
pub fn map_api_project(
    id: &str,
    worktree: &str,
    name: Option<&str>,
    updated: Option<f64>,
) -> InternalProject {
    InternalProject {
        id: id.to_string(),
        worktree: worktree.to_string(),
        name: resolve_project_name(name, worktree),
        last_updated: updated.unwrap_or(0.0),
    }
}

fn recency_desc(left: &InternalProject, right: &InternalProject) -> std::cmp::Ordering {
    right
        .last_updated
        .partial_cmp(&left.last_updated)
        .unwrap_or(std::cmp::Ordering::Equal)
}

/// Merges API rows with cached session directories: an already-listed
/// worktree keeps the API id/name/worktree spelling with `lastUpdated`
/// bumped to the max, otherwise the cached row appends; the result sorts
/// recency-descending, ties stable (TS: `getResolvedProjects` merge+sort).
pub fn merge_projects(
    api: Vec<InternalProject>,
    cached: Vec<CachedProject>,
    is_win32_platform: bool,
) -> Vec<InternalProject> {
    let mut order: Vec<InternalProject> = Vec::with_capacity(api.len() + cached.len());
    let mut index_by_key: HashMap<String, usize> = HashMap::with_capacity(api.len() + cached.len());

    for api_project in api {
        let key = project_worktree_key(&api_project.worktree, is_win32_platform);
        if let Some(&idx) = index_by_key.get(&key) {
            order[idx] = api_project;
        } else {
            index_by_key.insert(key, order.len());
            order.push(api_project);
        }
    }

    for cached_project in cached {
        let key = project_worktree_key(&cached_project.worktree, is_win32_platform);
        if let Some(&idx) = index_by_key.get(&key) {
            let cached_updated = cached_project.last_updated.unwrap_or(0.0);
            if cached_updated > order[idx].last_updated {
                order[idx].last_updated = cached_updated;
            }
            continue;
        }
        index_by_key.insert(key, order.len());
        order.push(InternalProject {
            id: cached_project.id.clone(),
            worktree: cached_project.worktree.clone(),
            name: cached_project.name.clone(),
            last_updated: cached_project.last_updated.unwrap_or(0.0),
        });
    }

    order.sort_by(recency_desc);
    order
}

/// Recency-descending stable sort, extracted for parity (TS: the inline
/// `.sort((left, right) => right.lastUpdated - left.lastUpdated)`).
pub fn sort_projects_by_recency(projects: &mut Vec<InternalProject>) {
    projects.sort_by(recency_desc);
}

/// Drops rows whose key matches an excluded path (TS: the `excludedKeys`
/// filter in `getResolvedProjects`).
pub fn filter_excluded_projects(
    projects: Vec<InternalProject>,
    excluded: &[&str],
    is_win32_platform: bool,
) -> Vec<InternalProject> {
    if excluded.is_empty() {
        return projects;
    }
    let excluded_keys: HashSet<String> = excluded
        .iter()
        .map(|path| project_worktree_key(path, is_win32_platform))
        .collect();
    projects
        .into_iter()
        .filter(|project| {
            !excluded_keys.contains(&project_worktree_key(&project.worktree, is_win32_platform))
        })
        .collect()
}

/// Finds a resolved row by id (TS: `getProjectById` lookup; the throw stays
/// with the caller via [`unknown_project_id_message`]).
pub fn find_project_by_id<'a>(
    projects: &'a [InternalProject],
    id: &str,
) -> Option<&'a InternalProject> {
    projects.iter().find(|project| project.id == id)
}

/// Exact `getProjectById` throw text: `Project with id <id> not found`.
pub fn unknown_project_id_message(id: &str) -> String {
    format!("Project with id {id} not found")
}

/// Finds a resolved row by worktree under project-key equality (TS:
/// `getProjectByWorktree` lookup, which runs over the linked-inclusive
/// resolution; the throw stays with the caller).
pub fn find_project_by_worktree<'a>(
    projects: &'a [InternalProject],
    worktree: &str,
    is_win32_platform: bool,
) -> Option<&'a InternalProject> {
    let key = project_worktree_key(worktree, is_win32_platform);
    projects.iter().find(|project| {
        project_worktree_key(&project.worktree, is_win32_platform) == key
    })
}

/// Exact `getProjectByWorktree` throw text:
/// `Project with worktree <worktree> not found`.
pub fn unknown_project_worktree_message(worktree: &str) -> String {
    format!("Project with worktree {worktree} not found")
}

/// Strips a resolved row to its public shape (TS: the
/// `({ id, worktree, name })` mapping in `getProjects`).
pub fn to_project_info(project: &InternalProject) -> ProjectInfo {
    ProjectInfo {
        id: project.id.clone(),
        worktree: project.worktree.clone(),
        name: project.name.clone(),
    }
}
