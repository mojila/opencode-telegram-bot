//! Message-history / file-browser / folder-presence pure gates (slice 52, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/message-history-service.ts` (`extractTextParts`:
//!   text-only join + trim + empty-to-`None`; the user-message map:
//!   `role === "user"` gate, `id ?? created` fallback, `created ?? 0`
//!   fallback, newest-first sort, `revertMessageID` slice-after gate;
//!   the latest-assistant reduce: skip non-assistant/summary/empty-text,
//!   newest wins with later-in-list ties (`>=`); the metrics pick uses the
//!   same reduction; `LATEST_ASSISTANT_RESPONSE_MESSAGES_LIMIT`)
//! - `src/app/services/file-browser-service.ts` (`MAX_ENTRIES_PER_PAGE`,
//!   `pathToDisplayPath` home/`~` collapse, `buildEntryLabel`,
//!   `buildTreeHeader` title + count lines, `isScanError`, the
//!   `totalPages`/`safePage` pagination math, the `ENOENT`/`EACCES`/
//!   `ENOTDIR`/`UNKNOWN` error-code mapping, base-sensitive directory sort,
//!   dirs-first `ls` entry sort, win32/posix path-style dispatch for
//!   `joinPath` / `isPathWithinDirectory`, `usesWindowsPath`)
//! - `src/app/services/folder-presence-service.ts`
//!   (`GLOBAL_PROJECT_FOLDER`, win32/posix `pathStyleOf`,
//!   case-folded `namesMatch`, trailing-separator-tolerant ancestor entry
//!   check, the own/ancestor listing presence decision)
//!
//! Step 2 (GREEN): every gate below is implemented as pure sync logic.
//! The parity tests in
//! `crates/otb-core/tests/slice52_history_browser_parity.rs` assert the TS
//! contract against these implementations.
//!
//! No OpenCode clients, no filesystem reads, no `config`/env/i18n lookups
//! live here — only the text math, pagination math, path-style dispatch,
//! and listing-decision predicates the services above drive.
//!
//! DEVIATIONS from TS:
//! 1. `os.homedir()`, `process.platform`, `path.resolve`, and `t(...)`
//!    are implicit in TS. The port takes them as explicit arguments
//!    (`home: &str`, [`PathStyle`], caller-resolved count labels via
//!    [`SubfolderCountKind`]) so the gates stay pure and testable.
//! 2. `String.length` / `slice` count UTF-16 code units in TS. The port
//!    works on `char`s/`&str` bytes; parity fixtures use ASCII-only inputs
//!    where the two coincide.
//! 3. `localeCompare(..., { sensitivity: "base" })` ordering is modelled
//!    as lowercase-compared byte order; parity fixtures use ASCII names
//!    where the two coincide.
//! 4. `message.info` nesting (`id`/`role`/`summary`/`time.created`) is
//!    flattened into explicit parameters / candidate structs; `id ??
//!    \`${created}\`` becomes `Option<&str>` + `Option<i64>`.
//! 5. The win32/posix branch is an explicit [`PathStyle`] (TS sniffs it
//!    from the input with the same drive-letter/UNC rule); TS `path`
//!    normalization inside `join`/`relative` is the GREEN step's job.
//! 6. `buildTreeHeader`'s `t("open.*")` count lines are split: the port
//!    returns the `"📂 …"` title from [`build_tree_title`] and the
//!    empty/one/many choice from [`subfolder_count_kind`]; callers render
//!    the localized line.

// ---------------------------------------------------------------------------
// Message history (`app/services/message-history-service.ts`)
// ---------------------------------------------------------------------------

/// How many recent messages `loadLatestAssistantResponse` reads.
pub const LATEST_ASSISTANT_RESPONSE_MESSAGES_LIMIT: usize = 20;

/// One message part as the history helpers inspect it (TS: `{ type,
// text? }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagePart {
    pub kind: String,
    pub text: Option<String>,
}

/// A user message as `loadUserMessages` returns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserMessageItem {
    pub id: String,
    pub text: String,
    pub created: i64,
}

/// An assistant candidate for the latest-response pick (TS: `role`,
/// `summary`, `time.created`, `parts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantCandidate {
    pub role: String,
    pub summary: bool,
    pub created: i64,
    pub parts: Vec<MessagePart>,
}

/// The winning latest assistant response text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantPick {
    pub text: String,
    pub created: i64,
}

/// Token/cost breakdown of one assistant message (TS: `info.tokens` +
/// `info.cost`).
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsTokens {
    pub input: i64,
    pub output: i64,
    pub reasoning: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub cost: f64,
}

/// An assistant candidate carrying its token/cost breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct AssistantMetricsCandidate {
    pub role: String,
    pub summary: bool,
    pub created: i64,
    pub tokens: MetricsTokens,
}

/// The latest assistant token/cost breakdown (TS:
/// `AssistantMessageMetrics`).
#[derive(Debug, Clone, PartialEq)]
pub struct AssistantMessageMetrics {
    pub input: i64,
    pub output: i64,
    pub reasoning: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub cost: f64,
}

/// Join the `text` parts with `""`, trim, empty becomes `None` (TS:
/// `extractTextParts`).
pub fn extract_text_parts(parts: &[MessagePart]) -> Option<String> {
    let text: String = parts
        .iter()
        .filter(|part| part.kind == "text")
        .filter_map(|part| part.text.as_deref())
        .collect();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Map one raw message to a user item: `role === "user"` gate, `id ??
/// created` fallback, `created ?? 0` fallback, empty text dropped (TS:
/// the `loadUserMessages` mapper).
pub fn map_user_message(
    role: &str,
    id: Option<&str>,
    created: Option<i64>,
    parts: &[MessagePart],
) -> Option<UserMessageItem> {
    if role != "user" {
        return None;
    }
    let text = extract_text_parts(parts)?;
    let created = created.unwrap_or(0);
    let id = id.map(str::to_string).unwrap_or_else(|| created.to_string());
    Some(UserMessageItem { id, text, created })
}

/// Newest-first in-place sort (TS: `.sort((a, b) => b.created -
/// a.created)`).
pub fn sort_user_messages_desc(messages: &mut [UserMessageItem]) {
    messages.sort_by(|a, b| b.created.cmp(&a.created));
}

/// Drop everything up to and including the revert message when its id is
/// found; otherwise keep all (TS: the `revertMessageID` slice gate).
pub fn apply_revert_slice(
    messages: Vec<UserMessageItem>,
    revert_message_id: Option<&str>,
) -> Vec<UserMessageItem> {
    match revert_message_id {
        Some(want) => match messages.iter().position(|item| item.id == want) {
            Some(index) => messages.into_iter().skip(index + 1).collect(),
            None => messages,
        },
        None => messages,
    }
}

/// Newest non-summary assistant with text wins; later-in-list wins ties
/// (TS: the `loadLatestAssistantResponse` reduce with `>=`).
pub fn pick_latest_assistant_text(messages: &[AssistantCandidate]) -> Option<AssistantPick> {
    let mut latest: Option<AssistantPick> = None;
    for message in messages {
        if message.role != "assistant" || message.summary {
            continue;
        }
        let Some(text) = extract_text_parts(&message.parts) else {
            continue;
        };
        match &latest {
            None => latest = Some(AssistantPick { text, created: message.created }),
            Some(current) if message.created >= current.created => {
                latest = Some(AssistantPick { text, created: message.created });
            }
            Some(_) => {}
        }
    }
    latest
}

/// Same reduction as [`pick_latest_assistant_text`] but returns the
/// winner's token/cost breakdown (TS: `loadLatestAssistantMetrics`).
pub fn pick_latest_assistant_metrics(
    messages: &[AssistantMetricsCandidate],
) -> Option<AssistantMessageMetrics> {
    let mut latest: Option<&AssistantMetricsCandidate> = None;
    for message in messages {
        if message.role != "assistant" || message.summary {
            continue;
        }
        match latest {
            None => latest = Some(message),
            Some(current) if message.created >= current.created => latest = Some(message),
            Some(_) => {}
        }
    }
    latest.map(|winner| AssistantMessageMetrics {
        input: winner.tokens.input,
        output: winner.tokens.output,
        reasoning: winner.tokens.reasoning,
        cache_read: winner.tokens.cache_read,
        cache_write: winner.tokens.cache_write,
        cost: winner.tokens.cost,
    })
}

// ---------------------------------------------------------------------------
// File browser (`app/services/file-browser-service.ts`)
// ---------------------------------------------------------------------------

/// Entries per browser page (`MAX_ENTRIES_PER_PAGE`).
pub const MAX_ENTRIES_PER_PAGE: usize = 8;

/// Server-style path flavour (TS: `path.win32` vs `path.posix` picked by
/// `usesWindowsPath` / `pathStyleOf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStyle {
    Posix,
    Win32,
}

/// Which count line a tree header needs (TS: the `open.no_subfolders` /
/// `open.subfolder_count` / `open.subfolders_count` branches).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubfolderCountKind {
    Empty,
    One,
    Many,
}

/// Mapped filesystem error kinds (TS: `"ENOENT" | "EACCES" | "ENOTDIR" |
/// "UNKNOWN"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanErrorCode {
    Enoent,
    Eacces,
    Enotdir,
    Unknown,
}

/// A failed directory scan (TS: `DirectoryScanError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryScanError {
    pub error: String,
    pub code: ScanErrorCode,
}

/// A successful directory scan header (TS: `DirectoryScanResult` without
/// the `entries` payload, which needs filesystem reads).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryScanOk {
    pub total_count: usize,
    pub page: usize,
    pub current_path: String,
    pub display_path: String,
    pub has_parent: bool,
    pub parent_path: Option<String>,
}

/// A directory scan outcome (TS: `DirectoryScanResult |
/// DirectoryScanError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryScan {
    Ok(DirectoryScanOk),
    Err(DirectoryScanError),
}

impl DirectoryScan {
    /// Whether the scan failed (TS: `isScanError`, the `"error" in
    /// result` check).
    pub fn is_err(&self) -> bool {
        matches!(self, DirectoryScan::Err(_))
    }
}

/// One `ls` entry (TS: `LsEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LsEntry {
    pub name: String,
    pub entry_type: LsEntryType,
}

/// `ls` entry flavour (TS: `"file" | "directory"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsEntryType {
    Directory,
    File,
}

/// Collapse a home-prefixed absolute path to `~` form (TS:
/// `pathToDisplayPath`).
pub fn path_to_display_path(absolute_path: &str, home: &str) -> String {
    if absolute_path == home {
        return "~".to_string();
    }
    if absolute_path.starts_with(&format!("{home}/"))
        || absolute_path.starts_with(&format!("{home}\\"))
    {
        return format!("~{}", &absolute_path[home.len()..]);
    }
    absolute_path.to_string()
}

/// `📁 <name>` entry label (TS: `buildEntryLabel`).
pub fn build_entry_label(name: &str) -> String {
    format!("📁 {name}")
}

/// `📂 <path>` plus `  (page/total)` when more than one page (TS: the
/// `buildTreeHeader` title line; `page` is 0-based).
pub fn build_tree_title(display_path: &str, page: usize, total_pages: usize) -> String {
    if total_pages > 1 {
        format!("📂 {display_path}  ({}/{total_pages})", page + 1)
    } else {
        format!("📂 {display_path}")
    }
}

/// Which localized count line the header needs for this total (TS: the
/// `totalCount === 0 / === 1 / else` branches).
pub fn subfolder_count_kind(total_count: usize) -> SubfolderCountKind {
    match total_count {
        0 => SubfolderCountKind::Empty,
        1 => SubfolderCountKind::One,
        _ => SubfolderCountKind::Many,
    }
}

/// At least one page; 8 entries per page (TS: `Math.max(1,
/// Math.ceil(n / MAX_ENTRIES_PER_PAGE))`).
pub fn total_pages(total_count: usize) -> usize {
    std::cmp::max(1, total_count.div_ceil(MAX_ENTRIES_PER_PAGE))
}

/// Clamp into `0..total_pages(total_count)` (TS: `Math.max(0,
/// Math.min(page, totalPages - 1))`).
pub fn clamp_page(page: i64, total_count: usize) -> usize {
    let total = total_pages(total_count);
    (page.clamp(0, total as i64 - 1)) as usize
}

/// Map a raw fs error code: `ELOOP` reads as missing, `EPERM` as denied
/// (TS: the `scanDirectory` catch mapping).
pub fn map_scan_error_code(code: &str) -> ScanErrorCode {
    match code {
        "ENOENT" | "ELOOP" => ScanErrorCode::Enoent,
        "EACCES" | "EPERM" => ScanErrorCode::Eacces,
        "ENOTDIR" => ScanErrorCode::Enotdir,
        _ => ScanErrorCode::Unknown,
    }
}

/// Case-insensitive ascending directory-name order (TS: `localeCompare`
/// with base sensitivity).
pub fn sort_dir_names(names: &mut [String]) {
    names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
}

/// Directories first, then case-insensitive names (TS: the `ls` entry
/// sort).
pub fn sort_ls_entries(entries: &mut [LsEntry]) {
    entries.sort_by(|left, right| {
        let left_dir = left.entry_type == LsEntryType::Directory;
        let right_dir = right.entry_type == LsEntryType::Directory;
        match (left_dir, right_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
        }
    });
}

/// Drive-letter or UNC paths read as Windows paths (TS:
/// `usesWindowsPath`).
pub fn uses_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\'))
        || path.starts_with("\\\\")
}

/// Pick the path flavour by the same drive-letter/UNC rule (TS:
/// `pathStyleOf` / `getPathApi`).
pub fn path_style_of(path: &str) -> PathStyle {
    if uses_windows_path(path) {
        PathStyle::Win32
    } else {
        PathStyle::Posix
    }
}

/// Join on the parent's path flavour (TS: `joinPath`).
pub fn join_path(parent_path: &str, child_name: &str) -> String {
    let sep = if uses_windows_path(parent_path) { '\\' } else { '/' };
    if parent_path.ends_with('/') || parent_path.ends_with('\\') {
        format!("{parent_path}{child_name}")
    } else {
        format!("{parent_path}{sep}{child_name}")
    }
}

/// Whether `target` is `directory` itself or below it (TS:
/// `isPathWithinDirectory`: empty relative, or neither `..`-escaping
/// nor absolute).
pub fn is_path_within_directory(target_path: &str, directory_path: &str) -> bool {
    fn split(style: PathStyle, path: &str) -> (String, Vec<String>, bool) {
        let lowered;
        let raw = match style {
            PathStyle::Win32 => {
                lowered = path.replace('/', "\\").to_lowercase();
                lowered.as_str()
            }
            PathStyle::Posix => path,
        };
        // Root: win32 drive (`c:\`), UNC (`\\server\share\`), or leading `\`;
        // posix leading `/`. Everything else is relative (root "").
        let (root, rest) = match style {
            PathStyle::Posix => {
                if let Some(stripped) = raw.strip_prefix('/') {
                    ("/".to_string(), stripped)
                } else {
                    (String::new(), raw)
                }
            }
            PathStyle::Win32 => {
                let bytes = raw.as_bytes();
                if bytes.len() >= 3
                    && bytes[0].is_ascii_alphabetic()
                    && bytes[1] == b':'
                    && bytes[2] == b'\\'
                {
                    (raw[..3].to_string(), &raw[3..])
                } else if let Some(stripped) = raw.strip_prefix("\\\\") {
                    // UNC root covers `\\server\share\`.
                    let mut parts = stripped.split('\\');
                    let server = parts.next().unwrap_or("");
                    let share = parts.next().unwrap_or("");
                    let root = format!("\\\\{server}\\{share}\\");
                    let rest_len = stripped.len().saturating_sub(
                        server.len() + 1 + share.len() + usize::from(!share.is_empty() || !server.is_empty()),
                    );
                    let rest = if rest_len < stripped.len() {
                        &stripped[stripped.len() - rest_len..]
                    } else {
                        ""
                    };
                    (root, rest)
                } else if let Some(stripped) = raw.strip_prefix('\\') {
                    ("\\".to_string(), stripped)
                } else {
                    (String::new(), raw)
                }
            }
        };
        let absolute = !root.is_empty();
        let mut out: Vec<String> = Vec::new();
        for segment in rest.split(match style {
            PathStyle::Posix => '/',
            PathStyle::Win32 => '\\',
        }) {
            match segment {
                "" | "." => {}
                ".." => {
                    if out.pop().is_none() && !absolute {
                        out.push("..".to_string());
                    }
                }
                other => out.push(other.to_string()),
            }
        }
        (root, out, absolute)
    }

    let style = path_style_of(directory_path);
    // TS dispatches on the directory's flavour (`getPathApi(directoryPath)`).
    let (dir_root, dir_parts, _) = split(style, directory_path);
    let (target_root, target_parts, _) = split(style, target_path);
    if dir_root != target_root {
        return false;
    }
    if target_parts.len() < dir_parts.len() {
        return false;
    }
    target_parts[..dir_parts.len()] == dir_parts[..]
}

// ---------------------------------------------------------------------------
// Folder presence (`app/services/folder-presence-service.ts`)
// ---------------------------------------------------------------------------

/// The global project of non-git folders; never a folder that can vanish
/// (TS: `GLOBAL_PROJECT_FOLDER`).
pub const GLOBAL_PROJECT_FOLDER: &str = "/";

/// How the server answered a folder listing (TS: `string[] |
/// SERVER_ERROR | OTHER_FAILURE`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderListing {
    Entries(Vec<String>),
    ServerError,
    OtherFailure,
}

/// Presence verdict for one folder (TS: `FolderPresence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderPresence {
    Present,
    Missing,
    Unknown,
}

/// Whether this is the never-vanishing global project folder (TS: the
/// `folder === GLOBAL_PROJECT_FOLDER` early return).
pub fn is_global_project_folder(folder: &str) -> bool {
    folder == GLOBAL_PROJECT_FOLDER
}

/// Case-insensitive on win32, exact on posix (TS: `namesMatch`).
pub fn names_match(style: PathStyle, left: &str, right: &str) -> bool {
    match style {
        PathStyle::Win32 => left.to_lowercase() == right.to_lowercase(),
        PathStyle::Posix => left == right,
    }
}

/// Whether an ancestor listing still names the child: trailing
/// separators stripped, basenames compared with [`names_match`] (TS: the
/// `entries.some(...)` gate inside `checkFolderPresence`).
fn entry_base_name(entry: &str, style: PathStyle) -> &str {
    let stripped = match style {
        PathStyle::Win32 => entry.trim_end_matches(|c| c == '/' || c == '\\'),
        PathStyle::Posix => entry.trim_end_matches('/'),
    };
    match style {
        PathStyle::Win32 => stripped
            .rfind(|c| c == '/' || c == '\\')
            .map(|index| &stripped[index + 1..])
            .unwrap_or(stripped),
        PathStyle::Posix => stripped
            .rfind('/')
            .map(|index| &stripped[index + 1..])
            .unwrap_or(stripped),
    }
}

pub fn ancestor_lists_child(
    entries: &[String],
    child_name: &str,
    style: PathStyle,
) -> bool {
    entries
        .iter()
        .any(|entry| names_match(style, entry_base_name(entry, style), child_name))
}

/// Presence from the own listing plus the nearest listed ancestor: own
/// entries mean present, other failures mean unknown, and a 500-listing
/// only means missing when a listed ancestor no longer names the child
/// (TS: `checkFolderPresence` after the async reads).
pub fn decide_folder_presence(
    own: &FolderListing,
    ancestor_entries: Option<&[String]>,
    child_name: &str,
    style: PathStyle,
) -> FolderPresence {
    match own {
        FolderListing::Entries(_) => FolderPresence::Present,
        FolderListing::OtherFailure => FolderPresence::Unknown,
        FolderListing::ServerError => match ancestor_entries {
            Some(entries) if !ancestor_lists_child(entries, child_name, style) => {
                FolderPresence::Missing
            }
            _ => FolderPresence::Unknown,
        },
    }
}
