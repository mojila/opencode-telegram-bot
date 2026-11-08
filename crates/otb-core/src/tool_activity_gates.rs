//! Tool activity gates (slice 91, GREEN): pure routing/formatting decisions
//! ported from `src/bot/events/handlers/tool-activity-handler.ts`
//! (plus `SUBAGENT_STREAM_PREFIX` from `handler-context.ts`,
//! `TOOL_ELAPSED_MAX_TRACKING_HOURS` from `session-runtime-state.ts`, and
//! `RUNNING_ICON` from `duration-formatter.ts`).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `runtime.*` streamer/tracker reads+writes, `formatToolInfo` /
//!   `formatCompactToolActivity` / `countToolCall` composition, duration
//!   `formatDuration` rendering, settings-store reads, policy/destination
//!   checks, `t(...)` lookups, grammy sends.
//!
//! Deviations: none intended; `String.length` / `slice` count UTF-16 code
//! units in TS while the port works on `char`s; parity fixtures use
//! ASCII-only inputs where the two coincide.

/// Telegram document caption hard cap.
pub const TELEGRAM_DOCUMENT_CAPTION_MAX_LENGTH: usize = 1024;
/// Live running-tool line icon.
pub const RUNNING_ICON: &str = "⏳";
/// Elapsed ceiling (hours) shown once a tool timer goes final.
pub const TOOL_ELAPSED_MAX_TRACKING_HOURS: u32 = 24;
/// Stream prefix under which subagent cards are rendered.
pub const SUBAGENT_STREAM_PREFIX: &str = "🧩";
/// Stream key used for the `todowrite` tool.
pub const TOOL_STREAM_KEY_TODO: &str = "todo";
/// Stream key used for every other tool.
pub const TOOL_STREAM_KEY_DEFAULT: &str = "default";

/// Live line key for a running tool call (`⏳<callId>`).
pub fn live_tool_prefix(call_id: &str) -> String {
    format!("{RUNNING_ICON}{call_id}")
}

/// Stream routing key: `"todo"` for `todowrite`, `"default"` otherwise.
pub fn tool_stream_key(tool: &str) -> &'static str {
    if tool == "todowrite" {
        TOOL_STREAM_KEY_TODO
    } else {
        TOOL_STREAM_KEY_DEFAULT
    }
}

/// Stream routing key for a subagent card (`` `subagent:<cardId>` ``).
pub fn subagent_stream_key(card_id: &str) -> String {
    format!("subagent:{card_id}")
}

/// Trim + enforce the Telegram caption cap, appending `...` on overflow.
pub fn prepare_document_caption(caption: &str) -> String {
    let normalized = caption.trim();
    if normalized.is_empty() {
        return String::new();
    }
    if normalized.chars().count() <= TELEGRAM_DOCUMENT_CAPTION_MAX_LENGTH {
        return normalized.to_string();
    }
    let kept: String = normalized
        .chars()
        .take(TELEGRAM_DOCUMENT_CAPTION_MAX_LENGTH - 3)
        .collect();
    format!("{kept}...")
}

/// Launched in the background: input or metadata flagged `background`.
pub fn is_background_tool(input_background: bool, metadata_background: bool) -> bool {
    input_background || metadata_background
}

/// Still being composed: state status is `pending`, so nothing is drawn yet.
pub fn is_composing(status: Option<&str>) -> bool {
    matches!(status, Some("pending"))
}

/// Whether the call tracks elapsed time: always in compact mode, otherwise
/// every tool except `task` (whose card already reports progress).
pub fn tracks_elapsed(compact_mode: bool, tool: &str) -> bool {
    compact_mode || tool != "task"
}

/// A finished call: `completed` or `error`.
pub fn is_terminal_status(status: Option<&str>) -> bool {
    matches!(status, Some("completed") | Some("error"))
}

/// Diff file attachment gate: has attachment + setting on + write/edit/apply_patch.
pub fn should_send_tool_file_attachment(
    has_attachment: bool,
    setting_on: bool,
    tool: &str,
) -> bool {
    has_attachment && setting_on && matches!(tool, "write" | "edit" | "apply_patch")
}

/// Final tool delivery skip: file attachment handled elsewhere, or `task`
/// tool (its live line/card already reported it).
pub fn should_skip_tool_delivery(should_send_file_attachment: bool, tool: &str) -> bool {
    should_send_file_attachment || tool == "task"
}

/// Patch tools carry the duration on the first line only.
pub fn use_first_line_duration(tool: &str) -> bool {
    tool == "apply_patch"
}

/// Heartbeat stays active while some subagent is `pending`/`running`.
pub fn heartbeat_active(statuses: &[&str]) -> bool {
    statuses
        .iter()
        .any(|status| matches!(*status, "pending" | "running"))
}

/// A background subagent card stays pinned while it outlives the parent turn
/// and is still `pending`/`running`.
pub fn subagent_pin_active(outlives_parent_turn: bool, status: Option<&str>) -> bool {
    outlives_parent_turn && matches!(status, Some("pending") | Some("running"))
}

/// A failed-while-composing call was never shown, so it never counts.
pub fn failed_while_composing(
    status: Option<&str>,
    had_stored_info: bool,
    stored_was_composing: bool,
) -> bool {
    matches!(status, Some("error")) && had_stored_info && stored_was_composing
}

/// A failed call counts only if it was seen running before the failure.
pub fn seen_failing(
    status: Option<&str>,
    had_stored_info: bool,
    failed_while_composing: bool,
) -> bool {
    matches!(status, Some("error")) && had_stored_info && !failed_while_composing
}

/// Number of file entries that render as text lines (no file data, has line).
pub fn count_text_lines(has_file_data: &[bool], has_line: &[bool]) -> usize {
    has_file_data
        .iter()
        .zip(has_line.iter())
        .filter(|(file_data, line)| !**file_data && **line)
        .count()
}

/// Whether the first file entry renders inline as text.
pub fn first_file_is_text(first_has_file_data: bool) -> bool {
    !first_has_file_data
}
