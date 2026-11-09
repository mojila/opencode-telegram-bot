//! File-browser service pure gates (slice 90, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/file-browser-service.ts` (`MAX_ENTRIES_PER_PAGE`,
//!   `pathToDisplayPath` home/`~` fold, the `scanDirectory` hidden-dir
//!   filter + base-sensitivity sort + `totalPages`/`safePage` clamp,
//!   `buildEntryLabel` (`📁 `), `buildTreeHeader` (`📂 ` + optional
//!   `  (page/total)` suffix), `expandTilde`, the `initBrowserRoots`
//!   comma-split/trim/filter, `isWithinAllowedRoot` exact-or-separator
//!   match, `isAllowedRoot`, `usesWindowsPath` drive/UNC detection,
//!   `joinPath`/`getBaseName`/`getParentPath` posix/win32 dispatch,
//!   `isSamePath` empty-relative check, `isPathWithinDirectory`
//!   self-or-descendant check, the `scanDirectory` errno ->
//!   `ENOENT`/`EACCES`/`ENOTDIR`/`UNKNOWN` mapping, the `scanLsDirectory`
//!   dirs-first + base-sensitivity sort + page clamp, `hasParent`
//!   root comparison).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `fs.readdir` / `fs.stat` / `realpath` scans, `os.homedir()`,
//!   `process.env.OPEN_BROWSER_ROOTS`, `getCurrentProject()`.
//! - `t(...)` lookups: count lines (`open.no_subfolders`,
//!   `open.subfolder_count`, `open.subfolders_count`) and scan errors
//!   (`ls.access_denied`, `ls.scan_error`) compose outside these gates.
//!
//! Deviations from the TS original (fixtures stay inside the
//! coinciding subset):
//! - `String.length` / `slice` count UTF-16 code units in TS while Rust
//!   works on `char`s / bytes; parity fixtures use ASCII-only inputs
//!   where the two coincide. `expand_tilde` therefore strips the 2-byte
//!   ASCII `"~/"` / `"~\\"` prefix by byte index, matching `p.slice(2)`.
//! - `localeCompare(..., { sensitivity: "base" })` maps to a
//!   case-folded (`to_lowercase`) comparison; ties keep input order
//!   (stable sort), matching `Array.prototype.sort` stability for
//!   base-equal names. A deterministic `name` tie-break is added only
//!   inside `sort_ls_entries` after the folded comparison.
//! - `path.win32` / `path.posix` dispatch maps to the explicit
//!   `windows` flag / `uses_windows_path` sniffing; fixtures use
//!   absolute already-normalized paths where resolution is the identity,
//!   so no `resolve` / `realpath` happens here (callers resolve first).
//! - `normalizePath` (`resolve` + lowercase on win32) takes
//!   already-resolved inputs here; the `windows` flag selects the
//!   lowercasing branch.
//! - `pathToDisplayPath` reads `os.homedir()` + `path.sep` in TS; here
//!   `home` is an explicit parameter and both `/` and `\` separators are
//!   accepted after home so the gate is platform-independent.
//! - `hasParent` in TS is a strict `dirPath !== root` comparison; the
//!   gate keeps exact string inequality for the same reason.

/// Max entries per browser page (TS: `MAX_ENTRIES_PER_PAGE`).
pub const MAX_ENTRIES_PER_PAGE: usize = 8;
/// Folder prefix for directory entry labels (TS: `` `📁 ${entry.name}` ``).
pub const ENTRY_FOLDER_PREFIX: &str = "📁 ";
/// Tree header prefix (TS: `` `📂 ${displayPath}` ``).
pub const TREE_HEADER_PREFIX: &str = "📂 ";
/// Home shorthand used when folding absolute paths (TS: `"~"`).
pub const HOME_SHORTHAND: &str = "~";

/// One `scanLsDirectory` row (TS: `LsEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsEntry {
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
}

/// Which count line `buildTreeHeader` appends (TS: the
/// `totalCount === 0 / === 1 / else` branch; labels stay with the caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeCount {
    Empty,
    One,
    Many,
}

/// Normalized scan errno (TS: `DirectoryScanError["code"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanErrorCode {
    NotFound,
    AccessDenied,
    NotDirectory,
    Unknown,
}

/// Fold an absolute path against the home directory (TS:
/// `pathToDisplayPath`: exact home reads as `"~"`, paths under
/// `home + sep` read as `"~" + rest`, anything else unchanged).
pub fn path_to_display_path(absolute: &str, home: &str) -> String {
    if absolute == home {
        return HOME_SHORTHAND.to_string();
    }
    if let Some(rest) = absolute.strip_prefix(home) {
        if rest.starts_with('/') || rest.starts_with('\\') {
            return format!("~{rest}");
        }
    }
    absolute.to_string()
}

/// Page count for an entry total (TS: `Math.max(1, Math.ceil(n / 8))`).
pub fn total_pages(count: usize) -> usize {
    std::cmp::max(1, count.div_ceil(MAX_ENTRIES_PER_PAGE))
}

/// Clamp a requested page into range (TS:
/// `Math.max(0, Math.min(page, totalPages - 1))`).
pub fn clamp_page(page: i64, total_pages: usize) -> usize {
    if page <= 0 {
        return 0;
    }
    let last = total_pages.saturating_sub(1);
    std::cmp::min(page as usize, last)
}

/// Half-open `[start, end)` entry window for a clamped page (TS:
/// `slice(start, start + MAX_ENTRIES_PER_PAGE)`).
pub fn page_slice_range(page: usize) -> (usize, usize) {
    let start = page.saturating_mul(MAX_ENTRIES_PER_PAGE);
    (start, start + MAX_ENTRIES_PER_PAGE)
}

/// Base-sensitivity ascending sort of directory names (TS:
/// `localeCompare(b.name, undefined, { sensitivity: "base" })`).
pub fn sorted_dir_names(names: &[String]) -> Vec<String> {
    let mut out = names.to_vec();
    out.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    out
}

/// Scan visibility predicate (TS: `entry.isDirectory() &&
/// !entry.name.startsWith(".")`).
pub fn is_visible_dir(name: &str, is_dir: bool) -> bool {
    is_dir && !name.starts_with('.')
}

/// Directory entry label (TS: `` `📁 ${entry.name}` ``).
pub fn build_entry_label(name: &str) -> String {
    format!("{ENTRY_FOLDER_PREFIX}{name}")
}

/// Tree header without the localized count line (TS: `` `📂
/// ${displayPath}` `` plus `  (page + 1/totalPages)` when
/// `totalPages > 1`).
pub fn tree_header_base(display: &str, page: usize, total_pages: usize) -> String {
    let mut header = format!("{TREE_HEADER_PREFIX}{display}");
    if total_pages > 1 {
        header.push_str(&format!("  ({}/{})", page + 1, total_pages));
    }
    header
}

/// Which localized count line the header needs (TS:
/// `totalCount === 0 / === 1 / else`).
pub fn tree_count_kind(total: usize) -> TreeCount {
    match total {
        0 => TreeCount::Empty,
        1 => TreeCount::One,
        _ => TreeCount::Many,
    }
}

/// Tilde expansion against an explicit home (TS: `expandTilde`:
/// `"~"` reads as home, `"~/"` / `"~\\"` prefixes join under home,
/// anything else unchanged).
pub fn expand_tilde(p: &str, home: &str) -> String {
    if p == "~" {
        return home.to_string();
    }
    if let Some(rest) = p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")) {
        if rest.is_empty() {
            return home.to_string();
        }
        let home = home.strip_suffix('/').unwrap_or(home);
        let home = home.strip_suffix('\\').unwrap_or(home);
        return format!("{home}/{rest}");
    }
    p.to_string()
}

/// Pure half of `initBrowserRoots` (TS: `raw.split(",")`, trim,
/// drop empties; resolution stays with the caller).
pub fn split_browser_roots(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Windows-path sniff (TS: `/^[A-Za-z]:[\\/]/` or `\\\\` prefix).
pub fn uses_windows_path(p: &str) -> bool {
    let bytes = p.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
    {
        return true;
    }
    p.starts_with("\\\\")
}

fn strip_trailing_sep<'a>(p: &'a str, windows: bool) -> &'a str {
    if windows {
        // Preserve drive roots (`C:\`) and the bare `\` root.
        if p.len() == 3
            && p.as_bytes()[0].is_ascii_alphabetic()
            && p.as_bytes()[1] == b':'
            && (p.as_bytes()[2] == b'/' || p.as_bytes()[2] == b'\\')
        {
            return p;
        }
        if p == "\\" || p == "/" {
            return p;
        }
        p.trim_end_matches(['/', '\\'])
    } else {
        if p == "/" {
            return p;
        }
        p.trim_end_matches('/')
    }
}

/// Join a child onto a parent, dispatching on the parent flavor
/// (TS: `getPathApi(parentPath).join(parentPath, childName)`).
pub fn join_path(parent: &str, child: &str) -> String {
    if uses_windows_path(parent) {
        let base = strip_trailing_sep(parent, true);
        if base.is_empty() {
            return child.to_string();
        }
        // `path.win32.join` strips leading separators of the child; keep
        // the same visible result for the fixtures.
        let child = child.trim_start_matches(['/', '\\']);
        format!("{base}\\{child}")
    } else {
        let base = strip_trailing_sep(parent, false);
        if base.is_empty() {
            return format!("/{child}");
        }
        let child = child.trim_start_matches('/');
        format!("{base}/{child}")
    }
}

/// Final segment, dispatching on the path flavor (TS:
/// `getPathApi(filePath).basename(filePath)`).
pub fn base_name(p: &str) -> String {
    let windows = uses_windows_path(p);
    let trimmed = strip_trailing_sep(p, windows);
    if trimmed.is_empty() {
        return String::new();
    }
    // A bare root has no basename (`basename("/") === ""`,
    // `basename("C:\\") === ""`).
    if (!windows && trimmed == "/")
        || (windows
            && (trimmed == "\\"
                || trimmed == "/"
                || trimmed.len() == 3 && trimmed.as_bytes()[1] == b':'))
    {
        return String::new();
    }
    let sep = |c: char| {
        if windows {
            c == '/' || c == '\\'
        } else {
            c == '/'
        }
    };
    trimmed.rsplit(sep).next().unwrap_or("").to_string()
}

/// Parent directory, dispatching on the path flavor (TS:
/// `getPathApi(filePath).dirname(filePath)`).
pub fn parent_path(p: &str) -> String {
    let windows = uses_windows_path(p);
    let trimmed = strip_trailing_sep(p, windows);
    let is_root = (!windows && trimmed == "/")
        || (windows
            && (trimmed == "\\"
                || trimmed == "/"
                || (trimmed.len() == 3 && trimmed.as_bytes()[1] == b':')));
    if is_root || trimmed.is_empty() {
        return trimmed.to_string();
    }
    let is_sep = |c: char| {
        if windows {
            c == '/' || c == '\\'
        } else {
            c == '/'
        }
    };
    match trimmed.rfind(is_sep) {
        None => ".".to_string(),
        Some(0) => {
            if windows {
                "\\".to_string()
            } else {
                "/".to_string()
            }
        }
        Some(idx) => {
            let head = &trimmed[..idx];
            // `dirname("C:\\a") === "C:\\"`: a bare drive needs its
            // separator back.
            if windows && head.len() == 2 && head.as_bytes()[1] == b':' {
                format!("{head}\\")
            } else if head.is_empty() {
                if windows {
                    "\\".to_string()
                } else {
                    "/".to_string()
                }
            } else {
                head.to_string()
            }
        }
    }
}

/// Filesystem root, dispatching on the path flavor (TS:
/// `getPathApi(filePath).parse(filePath).root`).
pub fn root_path(p: &str) -> String {
    if uses_windows_path(p) {
        if p.starts_with("\\\\") {
            // UNC root: `\\server\share\`.
            let mut parts = p[2..].split(['/', '\\']).filter(|s| !s.is_empty());
            match (parts.next(), parts.next()) {
                (Some(server), Some(share)) => format!("\\\\{server}\\{share}\\"),
                _ => "\\\\".to_string(),
            }
        } else {
            // Drive root: `C:\` (normalized to backslash).
            let bytes = p.as_bytes();
            if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
                format!("{}:\\", &p[..1])
            } else {
                String::new()
            }
        }
    } else if p.starts_with('/') {
        "/".to_string()
    } else {
        String::new()
    }
}

fn normalize_for_compare(p: &str, windows: bool) -> String {
    if windows {
        // Matches `normalizePath`'s lowercasing branch; separators fold
        // to `\` so `/` vs `\` spellings compare equal.
        let lowered = p.to_lowercase().replace('/', "\\");
        strip_trailing_sep(&lowered, true).to_string()
    } else {
        strip_trailing_sep(p, false).to_string()
    }
}

fn split_drive_or_unc(rest: &str) -> (String, Vec<String>) {
    // Caller guarantees a windows flavored absolute path fragment.
    if rest.starts_with("\\\\") {
        let mut parts = rest[2..].split(['/', '\\']).filter(|s| !s.is_empty());
        match (parts.next(), parts.next()) {
            (Some(server), Some(share)) => (
                format!("\\\\{server}\\{share}").to_lowercase(),
                parts.map(|s| s.to_lowercase()).collect(),
            ),
            _ => ("\\\\".to_string(), Vec::new()),
        }
    } else if rest.len() >= 2
        && rest.as_bytes()[0].is_ascii_alphabetic()
        && rest.as_bytes()[1] == b':'
    {
        let drive = rest[..2].to_lowercase();
        let tail = rest[2..].replace('/', "\\");
        let parts = tail
            .split('\\')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect();
        (drive, parts)
    } else {
        (
            String::new(),
            rest.replace('/', "\\")
                .split('\\')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_lowercase())
                .collect(),
        )
    }
}

/// Lexical `path.relative(from, to)` for already-resolved absolute
/// paths (TS: `getPathApi(...).relative`). Returns `None` when the two
/// paths live under different roots (drive/UNC vs drive/UNC, or
/// absolute vs relative), where Node returns an absolute path.
fn lexical_relative(from: &str, to: &str, windows: bool) -> Option<String> {
    if windows {
        let from_n = from.to_lowercase().replace('/', "\\");
        let to_n = to.to_lowercase().replace('/', "\\");
        let from_t = strip_trailing_sep(&from_n, true);
        let to_t = strip_trailing_sep(&to_n, true);
        let (from_root, from_parts) = split_drive_or_unc(from_t);
        let (to_root, to_parts) = split_drive_or_unc(to_t);
        if from_root != to_root {
            return None;
        }
        let common = from_parts
            .iter()
            .zip(to_parts.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let mut rel: Vec<String> = vec!["..".to_string(); from_parts.len() - common];
        rel.extend(to_parts[common..].iter().cloned());
        Some(rel.join("\\"))
    } else {
        let from_abs = from.starts_with('/');
        let to_abs = to.starts_with('/');
        if from_abs != to_abs {
            return None;
        }
        let from_parts: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
        let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
        let common = from_parts
            .iter()
            .zip(to_parts.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let mut rel: Vec<String> = vec!["..".to_string(); from_parts.len() - common];
        rel.extend(to_parts[common..].iter().map(|s| s.to_string()));
        Some(rel.join("/"))
    }
}

/// Same-path check (TS: `path.relative(right, left) === ""`).
pub fn is_same_path(left: &str, right: &str) -> bool {
    // TS dispatches on the *right* (base) path's flavor.
    let windows = uses_windows_path(right);
    if windows {
        normalize_for_compare(left, true) == normalize_for_compare(right, true)
    } else {
        match lexical_relative(right, left, false) {
            Some(rel) => rel.is_empty(),
            None => false,
        }
    }
}

/// Self-or-descendant check (TS: `relative === "" ||
/// (!startsWith("..") && !isAbsolute(relative))`).
pub fn is_path_within_directory(target: &str, dir: &str) -> bool {
    // TS dispatches on the *directory* path's flavor.
    let windows = uses_windows_path(dir);
    if windows {
        if normalize_for_compare(target, true) == normalize_for_compare(dir, true) {
            return true;
        }
    }
    match lexical_relative(dir, target, windows) {
        Some(rel) => {
            if rel.is_empty() {
                return true;
            }
            let dots = rel == ".."
                || rel.starts_with("../")
                || rel.starts_with("..\\")
                || rel.starts_with("..//");
            if dots {
                return false;
            }
            // A `None` relative (different roots) is surfaced by Node as
            // an absolute path; absolute results never count as within.
            let absolute = if windows {
                uses_windows_path(&rel) || rel.starts_with('\\') || rel.starts_with('/')
            } else {
                rel.starts_with('/')
            };
            !absolute
        }
        // Different roots / absolute-vs-relative: Node returns the
        // absolute target, which fails the TS predicate.
        None => false,
    }
}

fn roots_equal(target: &str, root: &str, windows: bool) -> bool {
    normalize_for_compare(target, windows) == normalize_for_compare(root, windows)
}

/// Allowed-root membership with explicit flavor (TS:
/// `isWithinAllowedRoot`: exact match or `root + "/"` / `root + "\\"`
/// prefix, lowercased when `windows`).
pub fn is_within_allowed_root(target: &str, roots: &[String], windows: bool) -> bool {
    let target_n = normalize_for_compare(target, windows);
    for root in roots {
        let root_n = normalize_for_compare(root, windows);
        if target_n == root_n {
            return true;
        }
        // A filesystem root contains every path under the same root.
        if root_n == "/" || root_n == "\\" {
            if target_n.starts_with('/') || target_n.starts_with('\\') {
                return true;
            }
            continue;
        }
        if windows && root_n.len() == 2 && root_n.as_bytes()[1] == b':' {
            // Bare drive (`c:`) contains `c:\...`.
            if target_n.starts_with(&format!("{root_n}\\")) {
                return true;
            }
            continue;
        }
        if target_n.starts_with(&format!("{root_n}/"))
            || (windows && target_n.starts_with(&format!("{root_n}\\")))
        {
            return true;
        }
    }
    // Exact-match fallback without separator normalization (keeps
    // already-normalized inputs on the fast path above).
    roots.iter().any(|root| roots_equal(target, root, windows))
}

/// Exact-root membership with explicit flavor (TS: `isAllowedRoot`).
pub fn is_allowed_root(target: &str, roots: &[String], windows: bool) -> bool {
    roots.iter().any(|root| roots_equal(target, root, windows))
}

/// Errno normalization for failed scans (TS: `ENOENT`/`ELOOP` ->
/// not-found, `EACCES`/`EPERM` -> access-denied, `ENOTDIR` ->
/// not-a-directory, anything else -> unknown).
pub fn map_scan_error_code(code: &str) -> ScanErrorCode {
    match code {
        "ENOENT" | "ELOOP" => ScanErrorCode::NotFound,
        "EACCES" | "EPERM" => ScanErrorCode::AccessDenied,
        "ENOTDIR" => ScanErrorCode::NotDirectory,
        _ => ScanErrorCode::Unknown,
    }
}

/// `scanLsDirectory` ordering: directories first, then
/// base-sensitivity name order (TS: type branch, then
/// `localeCompare` with `sensitivity: "base"`).
pub fn sort_ls_entries(entries: Vec<LsEntry>) -> Vec<LsEntry> {
    let mut out = entries;
    out.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            return if a.is_dir {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// Parent-navigation availability (TS: `dirPath !== root`).
pub fn has_parent(dir: &str, root: &str) -> bool {
    dir != root
}
