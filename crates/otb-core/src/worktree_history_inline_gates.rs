//! Worktree / message-history / inline-menu pure gates (slice 54, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/menus/worktree-selection-menu.ts`
//!   (`WORKTREE_CALLBACK_PREFIX`, `WORKTREE_PAGE_CALLBACK_PREFIX`,
//!   `formatWorktreeButtonLabel` 64-char `...` cut with `✅ ` prefix,
//!   `buildWorktreeButtonLabel` `"{n}. {folder} [{path}]"` numbering,
//!   `parseWorktreePageCallback` / `parseWorktreeIndexCallback` digits-only
//!   parses with the page/index prefix exclusion)
//! - `src/bot/menus/message-history-menu.ts` (`MESSAGES_CALLBACK_*`
//!   prefixes, `TELEGRAM_MESSAGE_LIMIT`, `truncateMessageHistoryText`
//!   `slice(0, max-3).trimEnd() + "..."` cut, `normalizeButtonText`
//!   whitespace collapse, the `[HH:MM] `-prefixed button label, the
//!   `[HH:MM]\n\n`-prefixed detail text, `buildMessagePageCallback`,
//!   `parseMessagePageCallback` / `parseMessageSelectCallback` JS-`Number()`
//!   parses, `calculateMessagesPaginationRange`)
//! - `src/bot/menus/inline-menu.ts` (`INLINE_MENU_KINDS`,
//!   `isInlineMenuKind`, `INLINE_MENU_CANCEL_PREFIX`,
//!   `LEGACY_CONTEXT_CANCEL_CALLBACK`, the cancel-callback build, and the
//!   cancel-callback parse in
//!   `src/bot/callbacks/inline-menu-cancel-callback-handler.ts`: legacy
//!   `compact:cancel` reads as `context`, `inline:cancel:<kind>` reads as
//!   that kind when known)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice54_worktree_history_inline_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2).
//!
//! No grammy/Telegram I/O, no timers, no `t(...)`/config lookups live here —
//! only callback prefixes, label truncation, pagination math, and kind
//! predicates. Callers resolve locales and build keyboards.
//!
//! DEVIATIONS from TS:
//! 1. `String.length` / `slice` count UTF-16 code units in TS. The port
//!    works on `char`s; parity fixtures use ASCII-only inputs where the two
//!    coincide. (`"✅ "` is 2 UTF-16 units and 2 `char`s, so the active-label
//!    budget matches either way.)
//! 2. `formatMessageTime` (local-timezone `HH:MM`) has no pure portable
//!    counterpart. The port takes the preformatted `"HH:MM"` label as an
//!    explicit argument and still composes the `"[HH:MM] "` /
//!    `"[HH:MM]\n\n"` prefixes itself.
//! 3. The message-history `parse*` callbacks mirror JS `Number()`: an empty
//!    or whitespace-only payload reads as page/index `0`; otherwise the
//!    trimmed text must parse as a finite non-negative integer. Fixtures
//!    stick to decimal/empty/whitespace/negative/fractional/alpha inputs.
//! 4. `t("messages.select*")` text selection collapses to
//!    [`MessageSelectTextKind`]: callers render the localized line.
//! 5. `getProjectFolderName` (owned by `command_session_gates`) is reused
//!    conceptually by `buildWorktreeButtonLabel`; the folder rule (last
//!    `/`/`\` segment, trailing separators tolerated, input unchanged when
//!    nothing but separators remains) is restated here so the gate stays
//!    self-contained.

// ---------------------------------------------------------------------------
// Worktree selection menu (`src/bot/menus/worktree-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// Inline-button label budget shared by the selection menus.
pub const MAX_INLINE_BUTTON_LABEL_LENGTH: usize = 64;

/// `worktree:<gitIndex>` callback prefix.
pub const WORKTREE_CALLBACK_PREFIX: &str = "worktree:";

/// `worktree:page:<n>` callback prefix.
pub const WORKTREE_PAGE_CALLBACK_PREFIX: &str = "worktree:page:";

/// `✅ `-prefixed label cut to 64 chars with `...` (TS: the private
/// `formatWorktreeButtonLabel`; no `trimEnd` before the ellipsis).
pub fn format_worktree_button_label(label: &str, is_active: bool) -> String {
    let prefix = if is_active { "✅ " } else { "" };
    let available =
        MAX_INLINE_BUTTON_LABEL_LENGTH.saturating_sub(prefix.chars().count());
    if label.chars().count() <= available {
        return format!("{prefix}{label}");
    }
    let head: String = label
        .chars()
        .take(available.saturating_sub(3))
        .collect();
    format!("{prefix}{head}...")
}

/// Last `/`/`\` segment of a path, tolerating trailing separators
/// (TS: `getProjectFolderName`).
fn project_folder_name(path: &str) -> String {
    let normalized = path.trim_end_matches(|c| c == '/' || c == '\\');
    if normalized.is_empty() {
        return path.to_string();
    }
    normalized
        .split(|c| c == '/' || c == '\\')
        .filter(|segment| !segment.is_empty())
        .last()
        .unwrap_or(normalized)
        .to_string()
}

/// `"{n}. {folder} [{path}]"` (TS: `buildWorktreeButtonLabel` over
/// `buildProjectButtonLabel`); `path` keeps its original separators in the
/// brackets while `folder` is the last segment.
pub fn build_worktree_button_label(index: usize, path: &str) -> String {
    let folder = project_folder_name(path);
    format!("{}. {folder} [{path}]", index + 1)
}

/// Page of a `worktree:page:<n>` callback; `None` unless the payload is
/// non-empty ASCII digits (TS: `parseWorktreePageCallback`).
pub fn parse_worktree_page_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(WORKTREE_PAGE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok()
}

/// Git index of a `worktree:<n>` callback; `None` for page callbacks and
/// non-digit payloads (TS: `parseWorktreeIndexCallback`).
pub fn parse_worktree_index_callback(data: &str) -> Option<u64> {
    if data.starts_with(WORKTREE_PAGE_CALLBACK_PREFIX) {
        return None;
    }
    let raw = data.strip_prefix(WORKTREE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok()
}

// ---------------------------------------------------------------------------
// Message history menu (`src/bot/menus/message-history-menu.ts`)
// ---------------------------------------------------------------------------

/// `messages:` callback prefix.
pub const MESSAGES_CALLBACK_PREFIX: &str = "messages:";

/// `messages:select:<n>` callback prefix.
pub const MESSAGES_CALLBACK_SELECT_PREFIX: &str = "messages:select:";

/// `messages:page:<n>` callback prefix.
pub const MESSAGES_CALLBACK_PAGE_PREFIX: &str = "messages:page:";

/// `messages:revert` callback.
pub const MESSAGES_CALLBACK_REVERT: &str = "messages:revert";

/// `messages:fork` callback.
pub const MESSAGES_CALLBACK_FORK: &str = "messages:fork";

/// `messages:back` callback.
pub const MESSAGES_CALLBACK_BACK: &str = "messages:back";

/// `messages:cancel` callback.
pub const MESSAGES_CALLBACK_CANCEL: &str = "messages:cancel";

/// Telegram text message budget applied to the detail view
/// (TS: `TELEGRAM_MESSAGE_LIMIT`).
pub const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

/// Over-budget cut with `...` after a `trimEnd`
/// (TS: `truncateMessageHistoryText`).
pub fn truncate_message_history_text(text: &str, max_length: usize) -> String {
    if text.chars().count() <= max_length {
        return text.to_string();
    }
    let head: String = text
        .chars()
        .take(max_length.saturating_sub(3))
        .collect();
    format!("{}...", head.trim_end())
}

/// Collapse every whitespace run to one space and trim
/// (TS: `normalizeButtonText`).
pub fn normalize_button_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `"[HH:MM] {normalized text cut to the 64-char budget}"`
/// (TS: `formatMessageButtonLabel`).
pub fn format_message_button_label(time_label: &str, text: &str) -> String {
    let prefix = format!("[{time_label}] ");
    let budget =
        MAX_INLINE_BUTTON_LABEL_LENGTH.saturating_sub(prefix.chars().count());
    let body = truncate_message_history_text(&normalize_button_text(text), budget);
    format!("{prefix}{body}")
}

/// `"[HH:MM]\n\n{text}"` cut to the Telegram message budget
/// (TS: `formatMessageDetailText`).
pub fn format_message_detail_text(time_label: &str, text: &str) -> String {
    let combined = format!("[{time_label}]\n\n{text}");
    truncate_message_history_text(&combined, TELEGRAM_MESSAGE_LIMIT)
}

/// Which select heading the menu shows (TS: `formatMessagesSelectText`:
/// page `0` reads as the plain heading, later pages carry `page + 1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSelectTextKind {
    FirstPage,
    Page(u64),
}

/// Heading kind for a message-list page (TS: `formatMessagesSelectText`
/// shape without the `t(...)` lookup).
pub fn message_select_text_kind(page: u64) -> MessageSelectTextKind {
    if page == 0 {
        MessageSelectTextKind::FirstPage
    } else {
        MessageSelectTextKind::Page(page)
    }
}

/// `messages:page:<page>` (TS: `buildMessagePageCallback`).
pub fn build_message_page_callback(page: u64) -> String {
    format!("{MESSAGES_CALLBACK_PAGE_PREFIX}{page}")
}

/// Mirror of JS `Number()` restricted to a non-negative integer: an empty
/// or whitespace-only payload reads as `0`; otherwise the trimmed text must
/// parse as a finite non-negative integer.
fn parse_js_number_int(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    if trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return trimmed.parse::<u64>().ok();
    }
    let value: f64 = trimmed.parse().ok()?;
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 || value > u64::MAX as f64 {
        return None;
    }
    Some(value as u64)
}

/// Page of a `messages:page:<n>` callback; `None` for a wrong prefix or a
/// non-integer/negative page (TS mirrors JS `Number()`: empty and
/// whitespace-only pages read as `0`).
pub fn parse_message_page_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(MESSAGES_CALLBACK_PAGE_PREFIX)?;
    parse_js_number_int(raw)
}

/// Index of a `messages:select:<n>` callback; `None` for a wrong prefix or
/// a non-integer/negative index (TS mirrors JS `Number()` the same way).
pub fn parse_message_select_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(MESSAGES_CALLBACK_SELECT_PREFIX)?;
    parse_js_number_int(raw)
}

/// Clamped pagination window (TS: `calculateMessagesPaginationRange`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessagesPaginationRange {
    pub page: i64,
    pub total_pages: i64,
    pub start_index: i64,
    pub end_index: i64,
}

/// First-page bounds, last-page clamp, `pageSize` floor of 1, and the
/// always-at-least-one page for an empty list.
pub fn calculate_messages_pagination_range(
    total_messages: i64,
    page: i64,
    page_size: i64,
) -> MessagesPaginationRange {
    let safe_page_size = page_size.max(1);
    let total_pages =
        ((total_messages + safe_page_size - 1) / safe_page_size).max(1);
    let page = page.clamp(0, total_pages - 1);
    let start_index = page * safe_page_size;
    let end_index = (start_index + safe_page_size).min(total_messages);
    MessagesPaginationRange {
        page,
        total_pages,
        start_index,
        end_index,
    }
}

// ---------------------------------------------------------------------------
// Inline menu (`src/bot/menus/inline-menu.ts` + cancel-callback handler)
// ---------------------------------------------------------------------------

/// `inline:cancel:` callback prefix (TS: `INLINE_MENU_CANCEL_PREFIX`).
pub const INLINE_MENU_CANCEL_PREFIX: &str = "inline:cancel:";

/// Pre-unification context-menu cancel callback
/// (TS: `LEGACY_CONTEXT_CANCEL_CALLBACK`).
pub const LEGACY_CONTEXT_CANCEL_CALLBACK: &str = "compact:cancel";

/// Every menu kind that can own an inline cancel row
/// (TS: `INLINE_MENU_KINDS`).
pub const INLINE_MENU_KINDS: [&str; 11] = [
    "project", "session", "recent", "model", "agent", "variant", "context", "open", "ls",
    "worktree", "settings",
];

/// Menu kind behind an inline cancel row (TS: `InlineMenuKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineMenuKind {
    Project,
    Session,
    Recent,
    Model,
    Agent,
    Variant,
    Context,
    Open,
    Ls,
    Worktree,
    Settings,
}

impl InlineMenuKind {
    /// Wire name of the kind (TS: the `INLINE_MENU_KINDS` entry).
    pub fn as_str(&self) -> &'static str {
        match self {
            InlineMenuKind::Project => "project",
            InlineMenuKind::Session => "session",
            InlineMenuKind::Recent => "recent",
            InlineMenuKind::Model => "model",
            InlineMenuKind::Agent => "agent",
            InlineMenuKind::Variant => "variant",
            InlineMenuKind::Context => "context",
            InlineMenuKind::Open => "open",
            InlineMenuKind::Ls => "ls",
            InlineMenuKind::Worktree => "worktree",
            InlineMenuKind::Settings => "settings",
        }
    }
}

/// Recognize one wire name (TS: `isInlineMenuKind` element check).
pub fn inline_menu_kind_of(value: &str) -> Option<InlineMenuKind> {
    match value {
        "project" => Some(InlineMenuKind::Project),
        "session" => Some(InlineMenuKind::Session),
        "recent" => Some(InlineMenuKind::Recent),
        "model" => Some(InlineMenuKind::Model),
        "agent" => Some(InlineMenuKind::Agent),
        "variant" => Some(InlineMenuKind::Variant),
        "context" => Some(InlineMenuKind::Context),
        "open" => Some(InlineMenuKind::Open),
        "ls" => Some(InlineMenuKind::Ls),
        "worktree" => Some(InlineMenuKind::Worktree),
        "settings" => Some(InlineMenuKind::Settings),
        _ => None,
    }
}

/// Whether the value names a known inline menu kind
/// (TS: `isInlineMenuKind`).
pub fn is_inline_menu_kind(value: &str) -> bool {
    inline_menu_kind_of(value).is_some()
}

/// `inline:cancel:<kind>` (TS: `getInlineCancelCallbackData`).
pub fn inline_cancel_callback_data(kind: InlineMenuKind) -> String {
    format!("{INLINE_MENU_CANCEL_PREFIX}{}", kind.as_str())
}

/// Where an inline-cancel callback points (TS:
/// `handleInlineMenuCancel` data dispatch: the legacy `compact:cancel`
/// reads as `context`; `inline:cancel:<kind>` reads as that kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineCancelTarget {
    Unified(InlineMenuKind),
    LegacyContext,
}

/// Target of an inline-cancel callback; `None` for unrelated data and
/// unknown kinds (TS: `handleInlineMenuCancel` parse).
pub fn parse_inline_cancel_callback(data: &str) -> Option<InlineCancelTarget> {
    if data == LEGACY_CONTEXT_CANCEL_CALLBACK {
        return Some(InlineCancelTarget::LegacyContext);
    }
    let raw = data.strip_prefix(INLINE_MENU_CANCEL_PREFIX)?;
    Some(InlineCancelTarget::Unified(inline_menu_kind_of(raw)?))
}
