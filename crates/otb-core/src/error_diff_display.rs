//! Pure error-text / session-list / diff-display / question-permission-footer /
//! tool-line gates (slice 35b, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/utils/opencode-error.ts` (`isExpectedOpencodeUnavailableError`
//!   6-marker set, `isOpencodeNotFoundError` name check,
//!   `isServerUnavailableError` cause-chain walk over string / message /
//!   `code` fields, `extractErrorMessage` Error -> string -> `data.message`
//!   -> `message` -> `name` chain with whitespace fallthrough)
//! - `src/utils/error-format.ts` (`formatErrorDetails` trim, `"{}"` /
//!   `"[object Object]"` empty gate, `clipText` `maxLength - 3 + "..."` cut)
//! - `src/bot/menus/session-selection-menu.ts`
//!   (`formatSessionsSelectText` first-page vs `select_page` + 1-based page,
//!   `"n. [marker ]title (date)"` row label, current-session marker,
//!   empty-list gate)
//! - `src/app/formatters/summary-formatter.ts` (`formatDiffLineInfo`
//!   `" (+a -d)"` shape, `countDiffChangesFromText` skipping `+++`/`---`
//!   headers, `extractFirstFileFromPatchText` `*** Add|Update|Delete File:`
//!   headers incl. CRLF, `extractFirstUpdatedFileFromTitle` `[AMDURC] `
//!   status lines, `getPatchFileChanges` `movePath > relativePath >
//!   filePath` pick, `formatPatchFileLine`, `normalizePathForDisplay`
//!   backslash + worktree-strip + `"."` gate, `formatDiff` hunk-header
//!   strip + `"+ "/"- "` spacing, `getToolIcon`, write ` (+N)` line info)
//! - `src/bot/menus/question-menu.ts` (`formatButtonText` icon + 60-char
//!   cut, `isQuestionRequestGone` `_tag` / `NotFoundError` check,
//!   `index + 1/total` progress text)
//! - `src/bot/menus/permission-menu.ts` (`"message is not modified"`
//!   gate, `"• {pattern}\n"` pattern lines, `"• …\n"` cut line,
//!   `sliceOnSafeBoundary` surrogate-safe cut)
//! - tool lines in `src/app/formatters/summary-formatter.ts`
//!   (`truncateWithEllipsis` incl. `maxLength <= 3` dots + `trimEnd`
//!   before `...`, whitespace collapse, `formatTodos` `✅/🔄/🔲` markers
//!   with `MAX_TODOS = 20`, `formatRunningTaskLine` bare vs
//!   `"label: description"` with `…` cut, compact single-line + 64-char
//!   cut)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice35b_error_diff_parity.rs` assert
//! the TS contract and MUST FAIL with `unimplemented!()` panics until the
//! Rust port lands (step 2).
//!
//! No grammy/Telegram I/O, no network/fetch, no timers live here — only
//! pure predicates, classifiers, and small value transforms.

// ---------------------------------------------------------------------------
// UTF-16 helpers (mirror JS `string.length` / `slice`, never split surrogate)
// ---------------------------------------------------------------------------

/// JS `string.length`: number of UTF-16 code units.
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// First `n` UTF-16 code units, backing off a trailing high surrogate so a
/// surrogate pair is never split (Rust cannot hold a lone surrogate).
fn utf16_take(s: &str, n: usize) -> String {
    let mut used = 0usize;
    let mut out = String::new();
    for ch in s.chars() {
        let units = ch.len_utf16();
        if used + units > n {
            break;
        }
        out.push(ch);
        used += units;
    }
    out
}

// ---------------------------------------------------------------------------
// Error text (`src/utils/opencode-error.ts`, `src/utils/error-format.ts`)
// ---------------------------------------------------------------------------

/// Expected "server is down" text (TS:
/// `isExpectedOpencodeUnavailableError`: `fetch failed`, `failed to fetch`,
/// `econnrefused`, `econnreset`, `enotfound`, `connectex`, case-insensitive).
pub fn is_expected_opencode_unavailable(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("fetch failed")
        || lower.contains("failed to fetch")
        || lower.contains("econnrefused")
        || lower.contains("econnreset")
        || lower.contains("enotfound")
        || lower.contains("connectex")
}

/// A "not found" answer from OpenCode (TS: `isOpencodeNotFoundError`
/// reads `name === "NotFoundError"`).
pub fn is_opencode_not_found_name(name: &str) -> bool {
    name == "NotFoundError"
}

/// "Server unavailable" marker anywhere in the text (TS:
/// `hasServerUnavailableMarker`: `fetch failed`, `econnrefused`,
/// `connection refused`, `connect refused`, case-insensitive).
pub fn has_server_unavailable_marker(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("fetch failed")
        || lower.contains("econnrefused")
        || lower.contains("connection refused")
        || lower.contains("connect refused")
}

/// One link of the `isServerUnavailableError` cause chain (TS walks
/// strings, `Error` `name: message`, and record `code` / `message` fields,
/// following `cause`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChainError {
    pub text: String,
    pub code: Option<String>,
    pub cause: Option<Box<ChainError>>,
}

/// True when the text, the code, or any cause carries a server-unavailable
/// marker (TS: `isServerUnavailableError`).
pub fn is_server_unavailable_chain(err: &ChainError) -> bool {
    let mut current: Option<&ChainError> = Some(err);
    while let Some(node) = current {
        if has_server_unavailable_marker(&node.text) {
            return true;
        }
        if let Some(code) = &node.code {
            if has_server_unavailable_marker(code) {
                return true;
            }
        }
        current = node.cause.as_deref();
    }
    false
}

/// `None` (TS `null` / `undefined`) is never unavailable.
pub fn is_server_unavailable_opt(err: Option<&ChainError>) -> bool {
    match err {
        None => false,
        Some(e) => is_server_unavailable_chain(e),
    }
}

/// The unknown shapes `extractErrorMessage` accepts (TS: `Error`
/// instance, plain string, SDK-style record, anything else).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorValue {
    ErrorMessage(String),
    Text(String),
    Record {
        data_message: Option<String>,
        message: Option<String>,
        name: Option<String>,
    },
    Unusable,
}

fn non_blank(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// First usable trimmed text: `Error` message, string as-is,
/// `data.message` -> `message` -> `name` with whitespace fallthrough,
/// else `None` (TS: `extractErrorMessage`).
pub fn extract_error_message(value: &ErrorValue) -> Option<String> {
    match value {
        ErrorValue::ErrorMessage(m) => non_blank(m),
        ErrorValue::Text(s) => non_blank(s),
        ErrorValue::Unusable => None,
        ErrorValue::Record {
            data_message,
            message,
            name,
        } => {
            if let Some(dm) = data_message {
                if let Some(v) = non_blank(dm) {
                    return Some(v);
                }
            }
            if let Some(m) = message {
                if let Some(v) = non_blank(m) {
                    return Some(v);
                }
            }
            if let Some(n) = name {
                if let Some(v) = non_blank(n) {
                    return Some(v);
                }
            }
            None
        }
    }
}

/// `value.slice(0, maxLength - 3) + "..."`, floored at `0`
/// (TS: `clipText`).
pub fn clip_error_text(value: &str, max_length: usize) -> String {
    if utf16_len(value) <= max_length {
        return value.to_string();
    }
    let cut = max_length.saturating_sub(3);
    format!("{}...", utf16_take(value, cut))
}

/// True when the input is blank after trimming, `"{}"`, or
/// `"[object Object]"` (TS: the `formatErrorDetails` fallback gate, which
/// runs on already-trimmed text).
pub fn is_empty_error_details(normalized: &str) -> bool {
    let t = normalized.trim();
    t.is_empty() || t == "{}" || t == "[object Object]"
}

/// Trimmed + empty-gated + clipped details, or `"unknown error"`
/// (TS: `formatErrorDetails`).
pub fn format_error_details_text(details: &str, max_length: usize) -> String {
    let normalized = details.trim();
    if is_empty_error_details(normalized) {
        return "unknown error".to_string();
    }
    clip_error_text(normalized, max_length)
}

// ---------------------------------------------------------------------------
// Session list (`src/bot/menus/session-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// `"n. [marker ]title (date)"`, 1-based (TS: the row label built in
/// `buildSessionsKeyboard`; `marker` already carries its trailing space).
pub fn format_session_row_label(position: usize, marker: &str, title: &str, date: &str) -> String {
    format!("{}. {marker}{title} ({date})", position + 1)
}

/// `"{marker} "` for the current session, else `""` (TS: the
/// `t("sessions.current_marker") + " "` marker).
pub fn current_session_marker(is_current: bool, marker_text: &str) -> String {
    if is_current {
        format!("{marker_text} ")
    } else {
        String::new()
    }
}

/// The `sessions.empty` gate (TS: `firstPage.sessions.length === 0`).
pub fn is_empty_session_list(count: usize) -> bool {
    count == 0
}

/// First page shows the plain title key, later pages the paged one
/// (TS: `formatSessionsSelectText`: `page <= 0` -> `sessions.select`,
/// else `sessions.select_page`).
pub fn sessions_select_title_key(page: u64) -> &'static str {
    if page == 0 {
        "sessions.select"
    } else {
        "sessions.select_page"
    }
}

/// 1-based page number for the `select_page` title (TS: `{ page: page + 1 }`).
pub fn sessions_select_page_display(page: u64) -> u64 {
    page.saturating_add(1)
}

// ---------------------------------------------------------------------------
// Diff display (`src/app/formatters/summary-formatter.ts`)
// ---------------------------------------------------------------------------

/// `" (+a -d)"`, parts omitted when zero/missing, `""` when both are
/// (TS: `formatDiffLineInfo`).
pub fn format_diff_line_info(additions: Option<u32>, deletions: Option<u32>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(a) = additions {
        if a > 0 {
            parts.push(format!("+{a}"));
        }
    }
    if let Some(d) = deletions {
        if d > 0 {
            parts.push(format!("-{d}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(" "))
    }
}

/// `+` / `-` line counts, skipping `+++` / `---` headers
/// (TS: `countDiffChangesFromText`).
pub fn count_diff_changes(text: &str) -> (u32, u32) {
    let mut additions = 0u32;
    let mut deletions = 0u32;
    for line in text.split('\n') {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions = additions.saturating_add(1);
            continue;
        }
        if line.starts_with('-') && !line.starts_with("---") {
            deletions = deletions.saturating_add(1);
        }
    }
    (additions, deletions)
}

fn strip_patch_prefix<'a>(line: &'a str, kind: &str) -> Option<&'a str> {
    let prefix = format!("*** {kind} File:");
    line.strip_prefix(prefix.as_str())
}

/// First `*** Add|Update|Delete File: <path>` header, `""` when missing
/// (TS: `extractFirstFileFromPatchText`, multiline, CRLF-tolerant).
pub fn extract_first_patch_file(patch_text: &str) -> String {
    for raw in patch_text.split('\n') {
        // Tolerate CRLF: trailing `\r` is trailing whitespace in TS `\s*$`.
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        for kind in ["Add", "Update", "Delete"] {
            if let Some(rest) = strip_patch_prefix(line, kind) {
                // TS `[ \t]*` skips leading spaces/tabs; `(.+?)\s*$` keeps
                // at least one char and trims trailing whitespace.
                let no_lead = rest.trim_start_matches([' ', '\t']);
                let trimmed = no_lead.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
                break;
            }
        }
    }
    String::new()
}

/// First `"X <path>"` line with `X` in `AMDURC`, `""` when missing
/// (TS: `extractFirstUpdatedFileFromTitle`).
pub fn extract_first_updated_file_from_title(title: &str) -> String {
    for raw_line in title.split('\n') {
        let line = raw_line.trim();
        let chars: Vec<char> = line.chars().collect();
        if utf16_len(line) >= 3
            && chars.get(1) == Some(&' ')
            && matches!(chars.first(), Some('A' | 'M' | 'D' | 'U' | 'R' | 'C'))
        {
            let rest: String = chars[2..].iter().collect();
            return rest.trim().to_string();
        }
    }
    String::new()
}

/// First non-empty of `movePath`, `relativePath`, `filePath`
/// (TS: the `getPatchFileChanges` path pick).
pub fn pick_patch_entry_path(
    move_path: Option<&str>,
    relative_path: Option<&str>,
    file_path: Option<&str>,
) -> Option<String> {
    for candidate in [move_path, relative_path, file_path].into_iter().flatten() {
        if !candidate.is_empty() {
            return Some(candidate.to_string());
        }
    }
    None
}

/// `"🩹 apply_patch {path}{line info}"` (TS: `formatPatchFileLine`).
pub fn format_patch_file_line(path: &str, additions: u32, deletions: u32) -> String {
    format!(
        "🩹 apply_patch {path}{}",
        format_diff_line_info(Some(additions), Some(deletions))
    )
}

/// Per-tool glyph (TS: `getToolIcon`).
pub fn tool_icon(tool: &str) -> &'static str {
    match tool {
        "read" => "📖",
        "write" => "✍️",
        "edit" => "✏️",
        "apply_patch" => "🩹",
        "bash" => "💻",
        "glob" => "📁",
        "grep" => "🔍",
        "task" => "🤖",
        "question" => "❓",
        "todoread" => "📋",
        "todowrite" => "📝",
        "webfetch" => "🌐",
        "web-search_tavily_search" => "🔎",
        "web-search_tavily_extract" => "📄",
        "skill" => "🎓",
        _ => "🛠️",
    }
}

// ---------------------------------------------------------------------------
// File link / path / format (`summary-formatter.ts` path + `formatDiff`)
// ---------------------------------------------------------------------------

fn strip_trailing_slashes(s: &str) -> &str {
    let mut end = s.len();
    let bytes = s.as_bytes();
    while end > 0 && bytes[end - 1] == b'/' {
        end -= 1;
    }
    &s[..end]
}

/// Backslash-normalized path with the worktree prefix stripped, `"."`
/// when equal to the worktree (TS: `normalizePathForDisplay`; `worktree`
/// is `None` when no project is selected).
pub fn normalize_display_path(path: &str, worktree: Option<&str>) -> String {
    let normalized_path = path.replace('\\', "/");
    let Some(worktree) = worktree else {
        return normalized_path;
    };
    let normalized_worktree = strip_trailing_slashes(&worktree.replace('\\', "/")).to_string();
    if normalized_worktree.is_empty() {
        return normalized_path;
    }

    #[cfg(target_os = "windows")]
    let (path_cmp, worktree_cmp) = (
        normalized_path.to_lowercase(),
        normalized_worktree.to_lowercase(),
    );
    #[cfg(not(target_os = "windows"))]
    let (path_cmp, worktree_cmp) = (normalized_path.clone(), normalized_worktree.clone());

    if path_cmp == worktree_cmp {
        return ".".to_string();
    }
    let prefix = format!("{worktree_cmp}/");
    if path_cmp.starts_with(&prefix) {
        // TS `slice(normalizedWorktree.length + 1)` is in UTF-16 units;
        // take the same prefix length in UTF-16 units from the original.
        let cut_units = utf16_len(&normalized_worktree) + 1;
        let mut used = 0usize;
        let mut byte_idx = 0usize;
        for ch in normalized_path.chars() {
            if used >= cut_units {
                break;
            }
            used += ch.len_utf16();
            byte_idx += ch.len_utf8();
        }
        return normalized_path[byte_idx..].to_string();
    }
    normalized_path
}

/// Hunk rendering: drops `@@`, `---`/`+++`, `Index:`, `===`, and
/// `\ No newline` lines; context lines keep one leading space, `+`/`-`
/// lines gain one (TS: `formatDiff`).
pub fn format_diff_text(diff: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in diff.split('\n') {
        if line.starts_with("@@") {
            continue;
        }
        if line.starts_with("---") || line.starts_with("+++") {
            continue;
        }
        if line.starts_with("Index:") {
            continue;
        }
        if line.starts_with("===") && line.contains('=') {
            continue;
        }
        if line.starts_with("\\ No newline") {
            continue;
        }
        if line.starts_with(' ') {
            out.push(format!(" {}", &line[1..]));
        } else if line.starts_with('+') {
            out.push(format!("+ {}", &line[1..]));
        } else if line.starts_with('-') {
            out.push(format!("- {}", &line[1..]));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

/// `" (+N)"` line count for a `write` payload (TS: the `write` branch of
/// `formatToolInfo`).
pub fn format_write_line_info(content: &str) -> String {
    format!(" (+{})", content.split('\n').count())
}

// ---------------------------------------------------------------------------
// File-question / permission footer (`question-menu.ts`, `permission-menu.ts`)
// ---------------------------------------------------------------------------

const MAX_BUTTON_LENGTH: usize = 60;

/// `"{icon}{label}"`, cut to 60 chars with `"..."` (TS: `formatButtonText`).
pub fn format_question_button_text(label: &str, icon: &str) -> String {
    let text = format!("{icon}{label}");
    if utf16_len(&text) <= MAX_BUTTON_LENGTH {
        return text;
    }
    format!("{}...", utf16_take(&text, MAX_BUTTON_LENGTH - 3))
}

/// A reply to a request OpenCode no longer has pending: V2
/// `QuestionNotFoundError` / `FormAlreadySettledError` tags or a
/// `NotFoundError` name (TS: `isQuestionRequestGone`).
pub fn is_question_request_gone(tag: Option<&str>, name: Option<&str>) -> bool {
    if name == Some("NotFoundError") {
        return true;
    }
    matches!(
        tag,
        Some("QuestionNotFoundError") | Some("FormAlreadySettledError")
    )
}

/// `"message is not modified"` anywhere, case-insensitive
/// (TS: `isMessageNotModifiedError`).
pub fn is_message_not_modified(text: &str) -> bool {
    text.to_lowercase().contains("message is not modified")
}

/// One listed pattern line (TS: `` `${PATTERN_PREFIX}${pattern}\n` ``).
pub fn permission_pattern_line(pattern: &str) -> String {
    format!("• {pattern}\n")
}

/// The cut marker replacing dropped patterns (TS: `CUT_PATTERNS_LINE`,
/// `"• …\n"`).
pub fn cut_patterns_line() -> &'static str {
    "• …\n"
}

/// `"i + 1/total"`, `""` when there is no total (TS: the progress text in
/// `formatQuestionDetailsPart`).
pub fn question_progress_text(index: usize, total: usize) -> String {
    if total == 0 {
        String::new()
    } else {
        format!("{}/{}", index.saturating_add(1), total)
    }
}

// ---------------------------------------------------------------------------
// Tool line (`src/app/formatters/summary-formatter.ts`)
// ---------------------------------------------------------------------------

/// Cut at `maxLength` with `trimEnd() + "..."`; `maxLength <= 3` yields
/// dots only (TS: `truncateWithEllipsis`).
pub fn truncate_tool_text(text: &str, max_length: usize) -> String {
    if utf16_len(text) <= max_length {
        return text.to_string();
    }
    if max_length <= 3 {
        return ".".repeat(max_length);
    }
    format!("{}...", utf16_take(text, max_length - 3).trim_end())
}

/// Whitespace-collapsed single line (TS: `replace(/\s+/g, " ")` + trim in
/// `formatRunningTaskLine`, and `/\s*\n+\s*/g -> " "` + trim in
/// `formatCompactToolInfo`).
pub fn collapse_tool_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One todo row with its status marker (TS: `formatTodos`:
/// `completed -> ✅`, `in_progress -> 🔄`, anything else -> `🔲`).
pub fn format_todo_line(status: &str, content: &str) -> String {
    let marker = match status {
        "completed" => "✅",
        "in_progress" => "🔄",
        _ => "🔲",
    };
    format!("{marker} {content}")
}

/// Todo rows rendered per card (TS: `MAX_TODOS = 20`).
pub const MAX_TODOS: usize = 20;

/// `"label: description"` collapsed to one line and cut at `max_len` with
/// `…`, or the bare label when the description is blank (TS:
/// `formatRunningTaskLine`).
pub fn format_running_task_line(description: Option<&str>, max_len: usize, label: &str) -> String {
    let description = description
        .map(collapse_tool_whitespace)
        .filter(|d| !d.is_empty())
        .unwrap_or_default();
    if description.is_empty() {
        return label.to_string();
    }
    let line = format!("{label}: {description}");
    if utf16_len(&line) <= max_len {
        return line;
    }
    let cut = max_len.saturating_sub(1);
    format!("{}…", utf16_take(&line, cut).trim_end())
}

/// Collapse whitespace runs that contain a newline to a single space
/// (TS: `replace(/\s*\n+\s*/g, " ")`), preserving other spacing.
fn collapse_newline_runs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            let mut j = i;
            let mut has_newline = false;
            while j < chars.len() && chars[j].is_whitespace() {
                if chars[j] == '\n' || chars[j] == '\r' {
                    has_newline = true;
                }
                j += 1;
            }
            if has_newline {
                out.push(' ');
            } else {
                for k in i..j {
                    out.push(chars[k]);
                }
            }
            i = j;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Newlines collapsed, trimmed, then cut at `max_length` with `trimEnd()`
/// + `"..."` (TS: `formatCompactToolInfo`).
pub fn format_compact_tool_line(line: &str, max_length: usize) -> String {
    let normalized = collapse_newline_runs(line).trim().to_string();
    if utf16_len(&normalized) <= max_length {
        return normalized;
    }
    format!(
        "{}...",
        utf16_take(&normalized, max_length.saturating_sub(3)).trim_end()
    )
}
