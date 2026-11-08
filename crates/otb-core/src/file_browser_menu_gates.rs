//! File-browser menu + callback pure gates (slice 56, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/menus/file-browser-menu.ts` (`OPEN_*` / `LS_*` callback
//!   prefixes, `truncateLabel` 64-char `...` cut, the path-index
//!   encode/decode with the 64-byte budget and `#<counter>` indirection,
//!   the `|<page>` pagination suffix encode/decode with `lastIndexOf` +
//!   `parseInt` semantics, `buildLsEntryLabel` (`📁 `/`📄 `),
//!   `escapeHtml`, the `totalPages` math, `hasBrowseActions`,
//!   `canAttachFile`)
//! - `src/bot/callbacks/file-browser-callback-handler.ts` (the dispatch
//!   order: open `roots` exact match, then nav / page / select decodes;
//!   ls nav / page / file / attach / download / back decodes; the
//!   `resolveInitialLsDirectory` cached-directory gate)
//! - `src/app/services/file-download-service.ts`
//!   (`isFileSizeAllowed`: falsy size reads as allowed — already ported in
//!   `media_prompt`; the menu composes it with `isTextFileName`, so the
//!   port takes both verdicts as explicit arguments)
//!
//! No grammy/Telegram I/O, no filesystem reads, no `config`/env/i18n
//! lookups live here — only callback prefixes, label truncation, path
//! encode/decode, page parses, entry labels, and routing predicates. The
//! menu keeps the `InlineKeyboard` assembly and the async scans; the
//! handler keeps the busy/menu guards and the sends.
//!
//! DEVIATIONS from TS (honored by this implementation):
//! 1. `String.length` / `slice` count UTF-16 code units in TS while
//!    `Buffer.byteLength(x, "utf-8")` counts bytes. The port works on
//!    `char`s for cuts and on UTF-8 bytes (`.len()`) for the 64-byte
//!    budget; parity fixtures use ASCII-only inputs where the two
//!    coincide.
//! 2. The module-level `openPathIndex` / `lsPathIndex` maps plus counters
//!    become the explicit [`PathIndex`] struct so the gates stay pure and
//!    testable; callers keep one index per browser the way TS keeps one
//!    module-level map per browser.
//! 3. `Number.parseInt(rest, 10)` tolerates leading whitespace, an
//!    optional sign, and trailing garbage (`"3x"` reads as `3`); only a
//!    digit-less payload is `NaN`. [`decode_path_with_page_callback`]
//!    mirrors that: leading-digit run as `i64`, `None` when empty.
//! 4. `t(...)` lookups (`open.*`, `ls.*` headers) are implicit in TS. The
//!    port exposes only the non-localized pieces (`buildLsEntryLabel`,
//!    [`escape_html`], [`total_pages`], [`has_browse_actions`]); callers
//!    resolve the locale.
//! 5. `isTextFileName` / `isFileSizeAllowed` live in `media_prompt`
//!    already. [`can_attach_file`] takes both verdicts as explicit
//!    arguments instead of re-implementing the extension table.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Callback prefixes (`src/bot/menus/file-browser-menu.ts`)
// ---------------------------------------------------------------------------

/// `open:` callback prefix.
pub const OPEN_CALLBACK_PREFIX: &str = "open:";
/// `open:nav:` callback prefix.
pub const OPEN_CALLBACK_NAV_PREFIX: &str = "open:nav:";
/// `open:sel:` callback prefix.
pub const OPEN_CALLBACK_SELECT_PREFIX: &str = "open:sel:";
/// `open:pg:` callback prefix.
pub const OPEN_CALLBACK_PAGE_PREFIX: &str = "open:pg:";
/// Show-browser-roots callback data.
pub const OPEN_CALLBACK_ROOTS: &str = "open:roots";

/// `ls:` callback prefix.
pub const LS_CALLBACK_PREFIX: &str = "ls:";
/// `ls:nav:` callback prefix.
pub const LS_CALLBACK_NAV_PREFIX: &str = "ls:nav:";
/// `ls:file:` callback prefix.
pub const LS_CALLBACK_FILE_PREFIX: &str = "ls:file:";
/// `ls:download:` callback prefix.
pub const LS_CALLBACK_DOWNLOAD_PREFIX: &str = "ls:download:";
/// `ls:attach:` callback prefix.
pub const LS_CALLBACK_ATTACH_PREFIX: &str = "ls:attach:";
/// `ls:back:` callback prefix.
pub const LS_CALLBACK_BACK_PREFIX: &str = "ls:back:";
/// `ls:pg:` callback prefix.
pub const LS_CALLBACK_PAGE_PREFIX: &str = "ls:pg:";

/// Inline-button label budget shared with the other selection menus.
pub const MAX_BUTTON_LABEL_LENGTH: usize = 64;

/// Pagination suffix separator (`|<page>`).
pub const PAGE_SEPARATOR: char = '|';

/// Telegram callback-data budget shared by the file-browser encoders.
const CALLBACK_BUDGET_BYTES: usize = 64;

/// Entries shown per browse page (`MAX_ENTRIES_PER_PAGE`).
const MAX_ENTRIES_PER_PAGE: usize = 8;

// ---------------------------------------------------------------------------
// Path index (`encodePathForCallback` / `decodePathFromCallback`)
// ---------------------------------------------------------------------------

/// Explicit form of the TS module-level path index maps (`openPathIndex`
/// / `lsPathIndex` plus their counters): over-budget paths are stored
/// under `#<counter>` keys with a post-increment counter.
#[derive(Debug, Clone, Default)]
pub struct PathIndex {
    entries: HashMap<String, String>,
    next_counter: usize,
}

impl PathIndex {
    /// Empty index (TS: a fresh module-level map with counter `0`).
    pub fn new() -> Self {
        Self { entries: HashMap::new(), next_counter: 0 }
    }

    /// Forget all indexed paths and reset the counter (TS:
    /// `clearOpenPathIndex` / `clearLsPathIndex`).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_counter = 0;
    }
}

/// Over-budget cut with `...` (TS: `truncateLabel`).
pub fn truncate_label(label: &str) -> String {
    truncate_label_with_max(label, MAX_BUTTON_LABEL_LENGTH)
}

/// [`truncate_label`] with an explicit budget (TS: the `maxLen` param).
pub fn truncate_label_with_max(label: &str, max_len: usize) -> String {
    if label.chars().count() <= max_len {
        return label.to_string();
    }
    let keep = max_len.saturating_sub(3);
    let head: String = label.chars().take(keep).collect();
    format!("{head}...")
}

/// Inline `prefix + fullPath` when it fits the 64-byte budget (plus
/// `reserve_bytes` kept free for a later suffix), otherwise store under
/// `#<counter>` and return `prefix + key` (TS:
/// `encodePathForCallback`).
pub fn encode_path_for_callback(
    index: &mut PathIndex,
    prefix: &str,
    full_path: &str,
    reserve_bytes: usize,
) -> String {
    let naive = format!("{prefix}{full_path}");
    if naive.len() + reserve_bytes <= CALLBACK_BUDGET_BYTES {
        return naive;
    }
    let key = format!("#{}", index.next_counter);
    index.next_counter += 1;
    index.entries.insert(key.clone(), full_path.to_string());
    format!("{prefix}{key}")
}

/// Raw path (or `#key` resolved through the index) behind a prefixed
/// callback; `None` for a wrong prefix or an unknown key (TS:
/// `decodePathFromCallback`).
pub fn decode_path_from_callback(
    index: &PathIndex,
    prefix: &str,
    data: &str,
) -> Option<String> {
    let raw = data.strip_prefix(prefix)?;
    if raw.starts_with('#') {
        return index.entries.get(raw).cloned();
    }
    Some(raw.to_string())
}

// ---------------------------------------------------------------------------
// Pagination suffix (`|<page>` encode/decode)
// ---------------------------------------------------------------------------

/// Decoded path-plus-page callback (TS: `{ path, page }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathWithPage {
    pub path: String,
    pub page: i64,
}

/// Encode plus `|<page>`, reserving the suffix bytes first (TS:
/// `encodeOpenPaginationCallback` / `encodeLsPathWithPageCallback`).
pub fn encode_path_with_page_callback(
    index: &mut PathIndex,
    prefix: &str,
    full_path: &str,
    page: i64,
) -> String {
    let page_suffix = format!("{PAGE_SEPARATOR}{page}");
    let reserve_bytes = page_suffix.len();
    let head = encode_path_for_callback(index, prefix, full_path, reserve_bytes);
    format!("{head}{page_suffix}")
}

/// `Number.parseInt(rest, 10)` semantics: leading whitespace, an optional
/// sign, then a run of ASCII digits with trailing garbage tolerated;
/// `None` when the payload has no digits at all.
fn parse_js_int(rest: &str) -> Option<i64> {
    let trimmed = rest.trim_start_matches(|c: char| c.is_whitespace());
    let (sign, digits_start) = match trimmed.strip_prefix('+') {
        Some(after) => (1_i64, after),
        None => match trimmed.strip_prefix('-') {
            Some(after) => (-1_i64, after),
            None => (1_i64, trimmed),
        },
    };
    let digit_len = digits_start.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digit_len == 0 {
        return None;
    }
    let digits = &digits_start[..digit_len];
    let magnitude: i64 = digits.parse().ok()?;
    Some(sign.saturating_mul(magnitude))
}

/// Split at the last `|`: path ref (resolved through the index when it
/// starts with `#`) plus `parseInt` page; `None` for a wrong prefix, a
/// missing separator, an unknown key, or a digit-less page (TS:
/// `decodePathWithPageCallback`).
pub fn decode_path_with_page_callback(
    index: &PathIndex,
    prefix: &str,
    data: &str,
) -> Option<PathWithPage> {
    let payload = data.strip_prefix(prefix)?;
    let separator_index = payload.rfind(PAGE_SEPARATOR)?;
    let path_ref = &payload[..separator_index];
    let page = parse_js_int(&payload[separator_index + 1..])?;
    let resolved = if path_ref.starts_with('#') {
        index.entries.get(path_ref)?.clone()
    } else {
        path_ref.to_string()
    };
    Some(PathWithPage { path: resolved, page })
}

/// `open:pg:<path>|<page>` (TS: `decodeOpenPaginationCallback`).
pub fn decode_open_pagination_callback(
    index: &PathIndex,
    data: &str,
) -> Option<PathWithPage> {
    decode_path_with_page_callback(index, OPEN_CALLBACK_PAGE_PREFIX, data)
}

/// `ls:pg:<path>|<page>` (TS: `decodeLsPaginationCallback`).
pub fn decode_ls_pagination_callback(
    index: &PathIndex,
    data: &str,
) -> Option<PathWithPage> {
    decode_path_with_page_callback(index, LS_CALLBACK_PAGE_PREFIX, data)
}

/// `ls:file:<path>|<page>` (TS: `decodeLsFileCallback`).
pub fn decode_ls_file_callback(
    index: &PathIndex,
    data: &str,
) -> Option<PathWithPage> {
    decode_path_with_page_callback(index, LS_CALLBACK_FILE_PREFIX, data)
}

/// `ls:back:<path>|<page>` (TS: `decodeLsBackCallback`).
pub fn decode_ls_back_callback(
    index: &PathIndex,
    data: &str,
) -> Option<PathWithPage> {
    decode_path_with_page_callback(index, LS_CALLBACK_BACK_PREFIX, data)
}

/// Path behind an `ls:attach:` callback (TS: `decodeLsAttachCallback`
/// over the non-page decoder).
pub fn decode_ls_attach_callback(
    index: &PathIndex,
    data: &str,
) -> Option<String> {
    decode_path_from_callback(index, LS_CALLBACK_ATTACH_PREFIX, data)
}

// ---------------------------------------------------------------------------
// Handler dispatch classification
// (`src/bot/callbacks/file-browser-callback-handler.ts`)
// ---------------------------------------------------------------------------

/// Which open-callback branch handles the data (TS: the
/// `handleOpenCallback` order — exact `open:roots`, then nav, page,
/// select decodes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCallbackKind {
    Roots,
    Nav,
    Page,
    Select,
    Unknown,
}

/// Classify open-browser callback data; `None` when it is not an `open:`
/// callback at all (TS: the `data.startsWith(OPEN_CALLBACK_PREFIX)`
/// guard).
pub fn classify_open_callback(data: &str) -> Option<OpenCallbackKind> {
    if !data.starts_with(OPEN_CALLBACK_PREFIX) {
        return None;
    }
    if data == OPEN_CALLBACK_ROOTS {
        return Some(OpenCallbackKind::Roots);
    }
    if data.starts_with(OPEN_CALLBACK_NAV_PREFIX) {
        return Some(OpenCallbackKind::Nav);
    }
    if data.starts_with(OPEN_CALLBACK_PAGE_PREFIX) {
        return Some(OpenCallbackKind::Page);
    }
    if data.starts_with(OPEN_CALLBACK_SELECT_PREFIX) {
        return Some(OpenCallbackKind::Select);
    }
    Some(OpenCallbackKind::Unknown)
}

/// Which ls-callback branch handles the data (TS: the
/// `handleLsCallback` order — nav, page, file, attach, download, back).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsCallbackKind {
    Nav,
    Page,
    File,
    Attach,
    Download,
    Back,
    Unknown,
}

/// Classify ls-browser callback data; `None` when it is not an `ls:`
/// callback at all (TS: the `data.startsWith(LS_CALLBACK_PREFIX)`
/// guard).
pub fn classify_ls_callback(data: &str) -> Option<LsCallbackKind> {
    if !data.starts_with(LS_CALLBACK_PREFIX) {
        return None;
    }
    if data.starts_with(LS_CALLBACK_NAV_PREFIX) {
        return Some(LsCallbackKind::Nav);
    }
    if data.starts_with(LS_CALLBACK_PAGE_PREFIX) {
        return Some(LsCallbackKind::Page);
    }
    if data.starts_with(LS_CALLBACK_FILE_PREFIX) {
        return Some(LsCallbackKind::File);
    }
    if data.starts_with(LS_CALLBACK_ATTACH_PREFIX) {
        return Some(LsCallbackKind::Attach);
    }
    if data.starts_with(LS_CALLBACK_DOWNLOAD_PREFIX) {
        return Some(LsCallbackKind::Download);
    }
    if data.starts_with(LS_CALLBACK_BACK_PREFIX) {
        return Some(LsCallbackKind::Back);
    }
    Some(LsCallbackKind::Unknown)
}

/// Initial ls directory: the current project when set, preferring the
/// cached per-user directory while it stays inside the project (TS:
/// `resolveInitialLsDirectory`; `isPathWithinDirectory` is taken as the
/// explicit `cached_within_project` verdict).
pub fn resolve_initial_ls_directory(
    current_project: Option<&str>,
    cached_directory: Option<&str>,
    cached_within_project: bool,
) -> Option<String> {
    let project = current_project?;
    match cached_directory {
        Some(cached) if !cached.is_empty() && cached_within_project => {
            Some(cached.to_string())
        }
        _ => Some(project.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Labels, headers, browse gates (`file-browser-menu.ts` remainder)
// ---------------------------------------------------------------------------

/// `📁 <name>` / `📄 <name>` (TS: `buildLsEntryLabel`).
pub fn build_ls_entry_label(is_directory: bool, name: &str) -> String {
    format!("{} {name}", if is_directory { "📁" } else { "📄" })
}

/// `&` first, then `<`, `>`, `"`, `'` (TS: `escapeHtml`).
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// At least one page; `MAX_ENTRIES_PER_PAGE` entries per page (TS:
/// `Math.max(1, Math.ceil(n / MAX_ENTRIES_PER_PAGE))`).
pub fn total_pages(total_count: usize) -> usize {
    std::cmp::max(1, total_count.div_ceil(MAX_ENTRIES_PER_PAGE))
}

/// Entries mean actions; otherwise only a non-root parent does (TS:
/// `hasBrowseActions`).
pub fn has_browse_actions(
    total_count: usize,
    has_parent: bool,
    is_project_root: bool,
) -> bool {
    if total_count > 0 {
        return true;
    }
    has_parent && !is_project_root
}

/// Only text files within the size limit get an attach button (TS:
/// `canAttachFile`: `isTextFileName(name) &&
/// isFileSizeAllowed(size, maxKb)`; both verdicts are explicit here).
pub fn can_attach_file(is_text_file: bool, size_allowed: bool) -> bool {
    is_text_file && size_allowed
}
