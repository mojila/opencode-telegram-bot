//! Pure command / session-selection / ingest gates (slice 32b, GREEN).
//!
//! Rust port of the pure gates in:
//! - `src/bot/menus/project-selection-menu.ts` (`getProjectFolderName`,
//!   `buildProjectButtonLabel`, `parseProjectPageCallback`,
//!   `calculateProjectsPaginationRange`, `formatProjectButtonLabel`,
//!   project search-needle match)
//! - `src/bot/menus/session-selection-menu.ts`
//!   (`SESSION_SEARCH_QUERY_MAX_LENGTH`, query cap, `parseSessionPageCallback`,
//!   `parseSessionIdCallback`, `parseBackgroundSessionCallback`,
//!   `buildSessionPageCallback`, session search match, session button label)
//! - `src/bot/callbacks/rename-callback-handler.ts` (title trim/empty gate,
//!   `/`-command passthrough) and `src/bot/menus/rename-menu.ts`
//!   (`RENAME_CANCEL_CALLBACK`)
//! - `src/bot/commands/abort-command.ts` (`pollSessionStatus` type mapping)
//! - `src/bot/handlers/voice-handler.ts` (voice/audio `file_id` pick,
//!   `file_path` filename + `.oga` -> `.ogg`, recognized-text trim/empty,
//!   `STT_NOTE_PROMPT` gating, TTS response mode)
//! - `src/bot/commands/status-command.ts` (model display line, project
//!   display with worktree branch)
//! - `src/bot/commands/help-command.ts` (`formatHelpText` shape)
//!
//! No grammy/Telegram I/O, no network/fetch, no timers live here — only
//! pure classifiers, parsers, and small value transforms.

// ---------------------------------------------------------------------------
// Projects menu (`src/bot/menus/project-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// Inline-button label budget shared by the selection menus.
pub const MAX_INLINE_BUTTON_LABEL_LENGTH: usize = 64;

/// `projects:page:` callback prefix (TS: `PROJECT_PAGE_CALLBACK_PREFIX`).
pub const PROJECT_PAGE_CALLBACK_PREFIX: &str = "projects:page:";

/// Last segment of a worktree path, tolerating `/` and `\` separators and
/// trailing separators (TS: `getProjectFolderName`). Returns the input
/// unchanged when nothing but separators remains.
pub fn get_project_folder_name(worktree: &str) -> String {
    let normalized = worktree.trim_end_matches(['/', '\\']);
    if normalized.is_empty() {
        return worktree.to_string();
    }
    normalized
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .last()
        .unwrap_or(normalized)
        .to_string()
}

/// `"{n}. {folder} [{worktree}]"` (TS: `buildProjectButtonLabel`).
pub fn build_project_button_label(index: usize, worktree: &str) -> String {
    format!(
        "{}. {} [{}]",
        index + 1,
        get_project_folder_name(worktree),
        worktree
    )
}

/// Page number of a `projects:page:<n>` callback; `None` unless the digits
/// are all ASCII digits (TS: `parseProjectPageCallback`).
pub fn parse_project_page_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(PROJECT_PAGE_CALLBACK_PREFIX)?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok()
}

/// Clamped pagination window (TS: `calculateProjectsPaginationRange`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectsPaginationRange {
    pub page: i64,
    pub total_pages: i64,
    pub start_index: i64,
    pub end_index: i64,
}

/// First-page bounds, last-page clamp, `pageSize` floor of 1, and the
/// always-at-least-one page for an empty list.
pub fn calculate_projects_pagination_range(
    total_projects: i64,
    page: i64,
    page_size: i64,
) -> ProjectsPaginationRange {
    let safe_page_size = page_size.max(1);
    let total_pages = if total_projects <= 0 {
        1
    } else {
        ((total_projects + safe_page_size - 1) / safe_page_size).max(1)
    };
    let normalized_page = page.clamp(0, total_pages - 1);
    let start_index = normalized_page * safe_page_size;
    let end_index = (start_index + safe_page_size).min(total_projects).max(0);
    // For an empty list start stays 0 and end clamps to 0.
    let start_index = if total_projects <= 0 { 0 } else { start_index };
    ProjectsPaginationRange {
        page: normalized_page,
        total_pages,
        start_index,
        end_index,
    }
}

/// Active-project check prefix + 64-char cut with `...` (TS: the private
/// `formatProjectButtonLabel`).
pub fn format_project_button_label(label: &str, is_active: bool) -> String {
    let prefix = if is_active { "✅ " } else { "" };
    let prefix_len = prefix.chars().count();
    let available = MAX_INLINE_BUTTON_LABEL_LENGTH - prefix_len;
    if label.chars().count() <= available {
        return format!("{prefix}{label}");
    }
    let kept: String = label.chars().take(available.saturating_sub(3)).collect();
    format!("{prefix}{kept}...")
}

/// Folder-name (or current-project display-name) substring match behind
/// `filterProjectsForSearch` (TS: the `needle` filter; `query` is trimmed
/// and lowercased inside).
pub fn project_matches_search(
    folder_name: &str,
    current_name: Option<&str>,
    is_active: bool,
    query: &str,
) -> bool {
    let needle = query.trim().to_lowercase();
    if folder_name.to_lowercase().contains(&needle) {
        return true;
    }
    if is_active {
        if let Some(name) = current_name {
            if !name.is_empty() && name.to_lowercase().contains(&needle) {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Sessions menu (`src/bot/menus/session-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// Typed-query cap (TS: `SESSION_SEARCH_QUERY_MAX_LENGTH`).
pub const SESSION_SEARCH_QUERY_MAX_LENGTH: usize = 100;

/// `session:page:` callback prefix.
pub const SESSION_PAGE_CALLBACK_PREFIX: &str = "session:page:";

/// `session:` callback prefix.
pub const SESSION_CALLBACK_PREFIX: &str = "session:";

/// `background-session:` callback prefix.
pub const BACKGROUND_SESSION_CALLBACK_PREFIX: &str = "background-session:";

/// Cap a typed search query (TS: `text.slice(0, 100)` in the search input
/// handler).
pub fn cap_session_search_query(query: &str) -> String {
    query.chars().take(SESSION_SEARCH_QUERY_MAX_LENGTH).collect()
}

/// Parse a JS `Number()`-produced page: empty/whitespace-only reads as `0`;
/// otherwise the trimmed text must parse as a finite non-negative integer.
fn parse_js_number_page(raw: &str) -> Option<u64> {
    if raw.trim().is_empty() {
        return Some(0);
    }
    let trimmed = raw.trim();
    // Minimal hex/binary/octal support mirroring JS `Number()`.
    let (negative, digits) = match trimmed.strip_prefix(['+', '-']) {
        Some(rest) => (trimmed.starts_with('-'), rest),
        None => (false, trimmed),
    };
    if negative {
        // `-0` reads as `0` in the `page < 0` gate (`-0 < 0` is false).
        if let Ok(value) = trimmed.parse::<f64>() {
            if value == 0.0 {
                return Some(0);
            }
        }
        return None;
    }
    let lower = digits.to_lowercase();
    if let Some(hex) = lower.strip_prefix("0x") {
        if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return u64::from_str_radix(hex, 16).ok();
        }
        return None;
    }
    if let Some(bin) = lower.strip_prefix("0b") {
        if !bin.is_empty() && bin.bytes().all(|b| b == b'0' || b == b'1') {
            return u64::from_str_radix(bin, 2).ok();
        }
        return None;
    }
    if let Some(oct) = lower.strip_prefix("0o") {
        if !oct.is_empty() && oct.bytes().all(|b| b >= b'0' && b <= b'7') {
            return u64::from_str_radix(oct, 8).ok();
        }
        return None;
    }
    match trimmed.parse::<f64>() {
        Ok(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= 0.0
                && value <= u64::MAX as f64 =>
        {
            Some(value as u64)
        }
        _ => None,
    }
}

/// Page of a `session:page:<n>` callback; `None` for a wrong prefix or a
/// non-integer/negative page (TS mirrors JS `Number()`: empty and
/// whitespace-only pages read as `0`).
pub fn parse_session_page_callback(data: &str) -> Option<u64> {
    let raw = data.strip_prefix(SESSION_PAGE_CALLBACK_PREFIX)?;
    parse_js_number_page(raw)
}

/// Session id of a `session:<id>` callback; `None` for page callbacks and
/// empty ids (TS: `parseSessionIdCallback`).
pub fn parse_session_id_callback(data: &str) -> Option<String> {
    if !data.starts_with(SESSION_CALLBACK_PREFIX) {
        return None;
    }
    if data.starts_with(SESSION_PAGE_CALLBACK_PREFIX) {
        return None;
    }
    let id = &data[SESSION_CALLBACK_PREFIX.len()..];
    if id.is_empty() {
        return None;
    }
    Some(id.to_string())
}

/// Background-open kinds (TS: `BackgroundSessionOpenKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundSessionOpenKind {
    AssistantResponse,
    QuestionAsked,
    PermissionAsked,
}

/// Parsed `background-session:[<marker>:]<id>` callback (TS:
/// `parseBackgroundSessionCallback`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundSessionCallback {
    pub session_id: String,
    pub kind: Option<BackgroundSessionOpenKind>,
}

pub fn parse_background_session_callback(data: &str) -> Option<BackgroundSessionCallback> {
    let payload = data.strip_prefix(BACKGROUND_SESSION_CALLBACK_PREFIX)?;
    match payload.find(':') {
        None => {
            if payload.is_empty() {
                None
            } else {
                Some(BackgroundSessionCallback {
                    session_id: payload.to_string(),
                    kind: None,
                })
            }
        }
        Some(separator) => {
            let marker = &payload[..separator];
            let session_id = &payload[separator + 1..];
            let kind = match marker {
                "a" => BackgroundSessionOpenKind::AssistantResponse,
                "q" => BackgroundSessionOpenKind::QuestionAsked,
                "p" => BackgroundSessionOpenKind::PermissionAsked,
                _ => return None,
            };
            if session_id.is_empty() {
                return None;
            }
            Some(BackgroundSessionCallback {
                session_id: session_id.to_string(),
                kind: Some(kind),
            })
        }
    }
}

/// `session:page:<page>` callback (TS: the private
/// `buildSessionPageCallback`).
pub fn build_session_page_callback(page: u64) -> String {
    format!("{SESSION_PAGE_CALLBACK_PREFIX}{page}")
}

/// Title/id substring match behind `searchSessions` (TS: the `needle`
/// filter over the display title and the session id; `query` is trimmed
/// and lowercased inside).
pub fn session_matches_search(display_title: &str, session_id: &str, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    display_title.to_lowercase().contains(&needle) || session_id.to_lowercase().contains(&needle)
}

/// `"{n}. {[marker ]}{title} ({date})"` row label (TS: the label built in
/// `buildSessionsKeyboard`; `current_marker` is passed in, date formatting
/// stays with the caller).
pub fn build_session_button_label(
    position: usize,
    current_marker: Option<&str>,
    title: &str,
    date: &str,
) -> String {
    format!(
        "{}. {}{} ({})",
        position + 1,
        current_marker.unwrap_or(""),
        title,
        date
    )
}

// ---------------------------------------------------------------------------
// Rename (`rename-callback-handler.ts`, `rename-menu.ts`)
// ---------------------------------------------------------------------------

/// Cancel callback data (TS: `RENAME_CANCEL_CALLBACK`).
pub const RENAME_CANCEL_CALLBACK: &str = "rename:cancel";

/// True for the rename cancel button payload.
pub fn is_rename_cancel_callback(data: &str) -> bool {
    data == RENAME_CANCEL_CALLBACK
}

/// Trimmed title, or `None` when blank (TS: the `newTitle` empty gate that
/// answers `rename.empty_title` and keeps the flow active).
pub fn normalize_rename_title(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `/`-leading input is not a title: it falls through to the command
/// pipeline (TS: `text.startsWith("/")` early-`false` in
/// `handleRenameTextAnswer`).
pub fn rename_text_is_command_passthrough(text: &str) -> bool {
    text.starts_with('/')
}

// ---------------------------------------------------------------------------
// Abort poll (`src/bot/commands/abort-command.ts`)
// ---------------------------------------------------------------------------

/// Abort request timeout (TS: the `setTimeout(..., 5000)` around
/// `session.abort`).
pub const ABORT_REQUEST_TIMEOUT_MS: u64 = 5_000;

/// Status-poll budget (TS: `pollSessionStatus` `maxWaitMs` default).
pub const ABORT_POLL_MAX_WAIT_MS: u64 = 5_000;

/// Status-poll interval (TS: `pollIntervalMs`).
pub const ABORT_POLL_INTERVAL_MS: u64 = 500;

/// Poll outcome (TS: `SessionState` in `abort-command.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollSessionState {
    Idle,
    Busy,
    NotFound,
}

/// Map one status-probe entry to its outcome (TS: the loop body of
/// `pollSessionStatus`): missing entry or unknown type -> `NotFound`,
/// `idle`/`error` -> `Idle`, `busy` -> keep polling (`Busy`).
pub fn classify_poll_session_status(entry_type: Option<&str>) -> PollSessionState {
    match entry_type {
        Some("idle") | Some("error") => PollSessionState::Idle,
        Some("busy") => PollSessionState::Busy,
        _ => PollSessionState::NotFound,
    }
}

// ---------------------------------------------------------------------------
// Voice ingest (`src/bot/handlers/voice-handler.ts`)
// ---------------------------------------------------------------------------

/// `voice.file_id ?? audio.file_id` (TS: the `fileId` pick in
/// `handleVoiceMessage`).
pub fn pick_voice_file_id(
    voice_file_id: Option<&str>,
    audio_file_id: Option<&str>,
) -> Option<String> {
    voice_file_id
        .or(audio_file_id)
        .map(|value| value.to_string())
}

/// Basename of a Telegram `file_path`, defaulting to `audio.ogg`, with a
/// trailing `.oga` renamed to `.ogg` (TS: `downloadTelegramFile`).
pub fn voice_filename_from_file_path(file_path: &str) -> String {
    let base = file_path.rsplit('/').next().unwrap_or("");
    let base = if base.is_empty() { "audio.ogg" } else { base };
    if let Some(stem) = base.strip_suffix(".oga") {
        format!("{stem}.ogg")
    } else {
        base.to_string()
    }
}

/// Trimmed transcript, or `None` when empty (TS: the `recognizedText`
/// empty gate answering `stt.empty_result`).
pub fn normalize_recognized_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `[Note: ...]` prefix for the LLM prompt, or `None` when the configured
/// note is blank, `"false"` (any case), or `"0"` (TS: the `notePrompt`
/// gate in `handleVoiceMessage`).
pub fn stt_note_prefix(note_prompt: &str) -> Option<String> {
    let trimmed = note_prompt.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("false") || trimmed == "0" {
        return None;
    }
    Some(format!("[Note: {trimmed}]"))
}

/// LLM prompt text: note prefix plus transcript (TS: `textForLLM`).
pub fn build_llm_text(note_prompt: &str, recognized_text: &str) -> String {
    match stt_note_prefix(note_prompt) {
        Some(prefix) => format!("{prefix}\n{recognized_text}"),
        None => recognized_text.to_string(),
    }
}

/// `all`/`auto` TTS modes hear the reply back, anything else is text-only
/// (TS: `responseMode` in `handleVoiceMessage`).
pub fn voice_response_mode(tts_mode: &str) -> &'static str {
    if tts_mode == "all" || tts_mode == "auto" {
        "text_and_tts"
    } else {
        "text_only"
    }
}

// ---------------------------------------------------------------------------
// Status + help display (`status-command.ts`, `help-command.ts`)
// ---------------------------------------------------------------------------

/// `🧠 provider/model[ (variant)]` (TS: the Model line in `statusCommand`;
/// a stored variant — including `"default"` — always parenthesizes).
pub fn format_status_model_line(
    provider_id: &str,
    model_id: &str,
    variant: Option<&str>,
) -> String {
    match variant {
        Some(variant) if !variant.is_empty() => {
            format!("🧠 {provider_id}/{model_id} ({variant})")
        }
        _ => format!("🧠 {provider_id}/{model_id}"),
    }
}

/// `main` or `main: branch` project display (TS: the worktree-context
/// branch in `statusCommand`).
pub fn format_status_project_display(main_path: &str, branch: Option<&str>) -> String {
    match branch {
        Some(branch) if !branch.is_empty() => format!("{main_path}: {branch}"),
        _ => main_path.to_string(),
    }
}

/// `📖 {title}\n\n{lines}\n\n{hint}` (TS: `formatHelpText`).
pub fn format_help_text(commands: &[(&str, &str)], title: &str, hint: &str) -> String {
    let lines: Vec<String> = commands
        .iter()
        .map(|(command, description)| format!("/{command} - {description}"))
        .collect();
    format!("📖 {title}\n\n{}\n\n{hint}", lines.join("\n"))
}
