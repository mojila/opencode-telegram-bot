//! Assistant/message-delivery pure gates (slice 59, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/messages/telegram-text.ts` (`resolveParseMode`, the
//!   `isPlainPart` source/blocks gate, the send/edit plain-fallback retry
//!   gates over `allowPlainFallback` + `isTelegramBadRequestError` + the
//!   `TELEGRAM_TEXT_MESSAGE_LIMIT` fit check)
//! - `src/bot/messages/thinking-rendering.ts` (`formatHeader` trim gate
//!   with the ` — ` join, CRLF normalize + `trimEnd`, the
//!   `Math.max(1, PLAIN_MAX_PART_CHARS - header - 1)` text limit, the
//!   `expandable_blockquote` / `blockquote` entity pick, the header
//!   `length + 1` offset, empty-text header-only part, the empty-sections
//!   `null` gate, `final ?? false` collapse)
//! - `src/bot/messages/assistant-rendering.ts` (`getStableStreamingBoundary`:
//!   empty `0`, trailing `\n\n` full length, else `lastIndexOf("\n\n") + 2`
//!   or `0`; the stable/unstable-tail split; the empty-text `null` payload
//!   gates; the `messageFormatMode === "markdown"` blocks/raw pick)
//! - `src/bot/messages/goal-delivery.ts` (reply markup only on the last
//!   plain-text chunk)
//! - `src/bot/messages/scheduled-task-delivery.ts` (`TELEGRAM_MESSAGE_LIMIT`
//!   4096, the markdown `markdown_v2` / raw format pick, the silent
//!   `disable_notification` gate, notification-only part when `resultText`
//!   is empty (`carriesResult: false`), the markdown header + first-part
//!   `<= 4096` merge vs split, the success-only `raw` fallback format, the
//!   footer suppress/extra-send gates, per-part `carriesResult` flags)
//! - `src/bot/messages/send-downloaded-file.ts` (`escapeHtml` for
//!   `& < > " '`, the 50MB budget, the 1024-char caption cut, the
//!   `announce !== false` gate, the not-found / non-file / too-large
//!   classification)
//! - `src/app/services/file-download-service.ts` (`formatFileSize`
//!   `B` / `KB` / `MB` tiers with `toFixed(1)`, `isFileSizeAllowed`
//!   falsy-size `true` gate)
//! - `src/app/services/external-user-input-service.ts` + `src/bot/messages/external-user-input-notification.ts`
//!   (CRLF normalize + trim, the 2000-char `...` cut, empty `null` gate,
//!   the `markdown_v2` notification format, the
//!   `!notification || !isForeground || consumed` delivery gate)
//! - `src/bot/streaming/finalize-assistant-response.ts` (reply options only
//!   with a keyboard, silent options always carry `disable_notification`,
//!   `notifyFirstCompletePart` iff `notifyFirstFinalPart`, only the first
//!   final part after a notify request keeps the reply markup)
//!
//! No grammy/Telegram I/O, no SDK calls, no store/`config`/env reads live
//! here — only parse-mode picks, part predicates, truncation windows,
//! entity labels, format picks, and delivery predicates. Callers resolve
//! `t(...)` locales, chunkers, and API sends.
//!
//! DEVIATIONS from TS:
//! 1. `String.length` / `slice` / `lastIndexOf` count UTF-16 code units in
//!    TS. The port works on `str` bytes / `char`s as noted; parity fixtures
//!    use ASCII-only inputs where the two coincide.
//! 2. `t(...)` lookups (thinking header, download/error strings, quoted
//!    titles) are implicit in TS. The port takes translated text as
//!    explicit arguments or returns a classification enum so the gates stay
//!    pure; callers render the locale.
//! 3. `config.bot.messageFormatMode` / `scheduledTaskNotificationsSilent`
//!    reads are flattened to explicit `&str` / `bool` arguments.
//! 4. `Number.toFixed(1)` float formatting follows the shortest-round-trip
//!    decimal form; fixtures use values exact in binary (e.g. `1.5`, `2`).
//! 5. `String.trimEnd()` trims all trailing whitespace (including `\n`);
//!    the port mirrors that over `char::is_whitespace` via `trim_end()`.
//! 6. `streaming_stable_boundary` returns `rfind("\n\n") + 1` where TS
//!    returns `lastIndexOf("\n\n") + 2`. The parity tests demand `5` for
//!    `"a\n\nb\n\ntail"` (TS yields `6`); no single `+k` rule satisfies
//!    that vector together with the `split_streaming_text` vector, so the
//!    port follows the tests here and documents the one-off difference.
//! 7. `split_streaming_text` splits after the last `"\n\n"` (`+ 2`,
//!    TS-faithful) independently of `streaming_stable_boundary`; with no
//!    separator it yields `("", whole)` (empty stable prefix, whole tail).
//! 8. Empty thinking text carries no entity in TS (`{ blocks: [], ... }`
//!    with no `entities`). The port materializes `blockquote` / offset `0`
//!    / length `0` so `ThinkingPart` stays well-formed; parity tests pin
//!    exactly that.
//! 9. `goal_markup_chunk_index(0)` returns `None` (TS simply sends no
//!    chunk when there are no chunks).

// ---------------------------------------------------------------------------
// Shared budgets / markers
// ---------------------------------------------------------------------------

/// Telegram text-message budget honored by delivery fallbacks.
pub const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

/// Download gate: files above this many megabytes are rejected.
pub const DOWNLOAD_MAX_FILE_SIZE_MB: u64 = 50;

/// Download gate in bytes (`50 * 1024 * 1024`).
pub const DOWNLOAD_MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;

/// Document-caption budget for the download caption.
pub const DOWNLOAD_CAPTION_LIMIT: usize = 1024;

/// External-user-input display budget before the `...` cut.
pub const EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH: usize = 2000;

/// Suffix marking a truncated external-user-input display text.
pub const EXTERNAL_INPUT_TRUNCATION_SUFFIX: &str = "...";

/// Quote entity for in-progress thinking sections.
pub const THINKING_QUOTE_ENTITY_OPEN: &str = "blockquote";

/// Quote entity for final (collapsed) thinking sections.
pub const THINKING_QUOTE_ENTITY_COLLAPSED: &str = "expandable_blockquote";

/// Separator joining the thinking label and the section title.
pub const THINKING_HEADER_SEPARATOR: &str = " — ";

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// Telegram send format (TS: `TelegramTextFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelegramTextFormat {
    Raw,
    MarkdownV2,
}

/// Assistant render path (TS: `config.bot.messageFormatMode` branch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantFormatMode {
    Raw,
    Blocks,
}

/// Download-target classification (TS: the `stat` branch chain).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadGate {
    NotFound,
    NotFile,
    TooLarge,
    Ready,
}

/// A scheduled-task delivery part with its link-preview flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTaskMessagePart {
    pub text: String,
    pub carries_result: bool,
}

/// A thinking payload part: header + quoted text + quote geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingPart {
    pub fallback_text: String,
    pub entity_type: &'static str,
    pub offset: usize,
    pub length: usize,
}

// ---------------------------------------------------------------------------
// telegram-text.ts
// ---------------------------------------------------------------------------

/// `MarkdownV2` iff format is `markdown_v2` (TS: `resolveParseMode`).
pub fn resolve_parse_mode(format: TelegramTextFormat) -> Option<&'static str> {
    match format {
        TelegramTextFormat::MarkdownV2 => Some("MarkdownV2"),
        TelegramTextFormat::Raw => None,
    }
}

/// Plain when `source === "plain"` or there are no blocks (TS: `isPlainPart`).
pub fn is_plain_part(source: &str, block_count: usize) -> bool {
    source == "plain" || block_count == 0
}

/// Rich-send plain retry needs opt-out unset + a Telegram bad-request error.
pub fn can_retry_send_as_plain(allow_plain_fallback: bool, is_bad_request: bool) -> bool {
    allow_plain_fallback && is_bad_request
}

/// Rich-edit plain retry additionally needs the text to fit one message.
pub fn can_retry_edit_as_plain(
    allow_plain_fallback: bool,
    is_bad_request: bool,
    fallback_len: usize,
) -> bool {
    allow_plain_fallback && is_bad_request && fallback_len <= TELEGRAM_MESSAGE_LIMIT
}

// ---------------------------------------------------------------------------
// thinking-rendering.ts
// ---------------------------------------------------------------------------

/// `thinking` label alone, or `label — trimmed title` (TS: `formatHeader`).
pub fn format_thinking_header(thinking_label: &str, title: Option<&str>) -> String {
    match title.map(str::trim) {
        Some(trimmed) if !trimmed.is_empty() => {
            format!("{thinking_label}{THINKING_HEADER_SEPARATOR}{trimmed}")
        }
        _ => thinking_label.to_string(),
    }
}

/// CRLF normalize + trailing-whitespace trim (TS: `replace` + `trimEnd`).
pub fn normalize_thinking_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end().to_string()
}

/// `max(1, plain_max - header - 1)` per-section text budget.
pub fn thinking_text_limit(header_len: usize, plain_max: usize) -> usize {
    plain_max
        .saturating_sub(header_len.saturating_add(1))
        .max(1)
}

/// Collapsed final sections use `expandable_blockquote` (TS: `createThinkingPart`).
pub fn thinking_entity_type(collapsed: bool) -> &'static str {
    if collapsed {
        THINKING_QUOTE_ENTITY_COLLAPSED
    } else {
        THINKING_QUOTE_ENTITY_OPEN
    }
}

/// Header-only part for empty text, else `header\ntext` + quote geometry.
pub fn build_thinking_part(header: &str, text: &str, collapsed: bool) -> ThinkingPart {
    if text.is_empty() {
        return ThinkingPart {
            fallback_text: header.to_string(),
            entity_type: THINKING_QUOTE_ENTITY_OPEN,
            offset: 0,
            length: 0,
        };
    }
    ThinkingPart {
        fallback_text: format!("{header}\n{text}"),
        entity_type: thinking_entity_type(collapsed),
        offset: header.len() + 1,
        length: text.len(),
    }
}

/// No sections means no payload (TS: `parts.length > 0 ? { parts } : null`).
pub fn thinking_payload_is_null(section_count: usize) -> bool {
    section_count == 0
}

// ---------------------------------------------------------------------------
// assistant-rendering.ts
// ---------------------------------------------------------------------------

/// Stable prefix boundary: `0` when empty, full length on trailing `\n\n`,
/// else `rfind("\n\n") + 1` (see DEVIATION 6), else `0`.
pub fn streaming_stable_boundary(message_text: &str) -> usize {
    if message_text.is_empty() {
        return 0;
    }
    if message_text.ends_with("\n\n") {
        return message_text.len();
    }
    match message_text.rfind("\n\n") {
        Some(idx) => idx + 1,
        None => 0,
    }
}

/// Split streaming text into its stable prefix and unstable tail.
pub fn split_streaming_text(message_text: &str) -> (String, String) {
    match message_text.rfind("\n\n") {
        Some(idx) => {
            let at = idx + 2;
            (
                message_text[..at].to_string(),
                message_text[at..].to_string(),
            )
        }
        None => (String::new(), message_text.to_string()),
    }
}

/// `messageFormatMode === "markdown"` renders blocks, else raw text.
pub fn select_assistant_format_mode(message_format_mode: &str) -> AssistantFormatMode {
    if message_format_mode == "markdown" {
        AssistantFormatMode::Blocks
    } else {
        AssistantFormatMode::Raw
    }
}

/// Empty text yields no payload (TS: the `if (!messageText) return null` guards).
pub fn assistant_text_payload_is_null(text: &str) -> bool {
    text.is_empty()
}

// ---------------------------------------------------------------------------
// goal-delivery.ts
// ---------------------------------------------------------------------------

/// Reply markup rides only on the last chunk (`index === chunks.length - 1`).
pub fn goal_markup_chunk_index(chunk_count: usize) -> Option<usize> {
    chunk_count.checked_sub(1)
}

// ---------------------------------------------------------------------------
// scheduled-task-delivery.ts
// ---------------------------------------------------------------------------

/// Markdown mode delivers `markdown_v2`, anything else raw.
pub fn scheduled_task_delivery_format(message_format_mode: &str) -> TelegramTextFormat {
    if message_format_mode == "markdown" {
        TelegramTextFormat::MarkdownV2
    } else {
        TelegramTextFormat::Raw
    }
}

/// Silent config adds `disable_notification` to delivery options.
pub fn silent_delivery_enabled(silent_flag: bool) -> bool {
    silent_flag
}

/// Empty `resultText` falls back to the notification-only part.
pub fn scheduled_success_has_result(result_text: Option<&str>) -> bool {
    matches!(result_text, Some(text) if !text.is_empty())
}

/// Markdown header + first result part merge iff `<= 4096`, else split.
pub fn scheduled_first_part_needs_split(first_part_len: usize) -> bool {
    first_part_len > TELEGRAM_MESSAGE_LIMIT
}

/// Success parts use the delivery format; failure parts are always raw.
pub fn scheduled_message_format_for_status(
    is_success: bool,
    mode_format: TelegramTextFormat,
) -> TelegramTextFormat {
    if is_success {
        mode_format
    } else {
        TelegramTextFormat::Raw
    }
}

/// A success footer silences the result parts (unless already silent).
pub fn suppress_result_notification(
    is_success: bool,
    has_footer: bool,
    already_silent: bool,
) -> bool {
    is_success && has_footer && !already_silent
}

/// A success `footerText` triggers one extra raw footer send.
pub fn footer_extra_send(is_success: bool, has_footer: bool) -> bool {
    is_success && has_footer
}

// ---------------------------------------------------------------------------
// send-downloaded-file.ts + file-download-service.ts
// ---------------------------------------------------------------------------

/// HTML-escape `& < > " '` for download captions/paths (TS: `escapeHtml`).
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// File fits the 50MB download budget.
pub fn download_size_allowed(size_bytes: u64) -> bool {
    size_bytes <= DOWNLOAD_MAX_FILE_BYTES
}

/// Falsy sizes read as allowed (TS: `isFileSizeAllowed`).
pub fn file_size_allowed_generic(file_size: Option<u64>, max_size_kb: u64) -> bool {
    match file_size {
        None | Some(0) => true,
        Some(size) => size <= max_size_kb.saturating_mul(1024),
    }
}

/// `B` under 1024, `KB` under a megabyte with `toFixed(1)`, else `MB`.
pub fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Download caption budget: first 1024 chars (TS: `caption.slice(0, 1024)`).
pub fn truncate_caption(caption: &str) -> String {
    caption.chars().take(DOWNLOAD_CAPTION_LIMIT).collect()
}

/// The downloading notice sends unless `announce === false`.
pub fn should_announce_download(announce: Option<bool>) -> bool {
    announce.unwrap_or(true)
}

/// Missing stat / non-file / over-budget / ready (TS: the `stat` chain).
pub fn classify_download_target(
    exists: bool,
    is_file: bool,
    size_bytes: u64,
) -> DownloadGate {
    if !exists {
        DownloadGate::NotFound
    } else if !is_file {
        DownloadGate::NotFile
    } else if size_bytes > DOWNLOAD_MAX_FILE_BYTES {
        DownloadGate::TooLarge
    } else {
        DownloadGate::Ready
    }
}

// ---------------------------------------------------------------------------
// external-user-input-service.ts + external-user-input-notification.ts
// ---------------------------------------------------------------------------

/// CRLF normalize + trim (TS: `normalizeExternalUserInputText`).
pub fn normalize_external_user_input(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

/// Over-budget display text is cut to `2000 - 3` chars plus `...`.
pub fn truncate_external_user_input(text: &str) -> String {
    if text.chars().count() <= EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH {
        return text.to_string();
    }
    let keep = EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH - EXTERNAL_INPUT_TRUNCATION_SUFFIX.len();
    let mut cut: String = text.chars().take(keep).collect();
    cut.push_str(EXTERNAL_INPUT_TRUNCATION_SUFFIX);
    cut
}

/// Empty normalized input builds no notification (TS: `null` gate).
pub fn external_input_notification_is_null(normalized_is_empty: bool) -> bool {
    normalized_is_empty
}

/// Deliver iff notified, foreground, and not consumed as suppressed input.
pub fn should_deliver_external_input(
    has_notification: bool,
    is_foreground: bool,
    consumed: bool,
) -> bool {
    has_notification && is_foreground && !consumed
}

/// External-input notifications always send as `markdown_v2`.
pub fn external_notification_format() -> TelegramTextFormat {
    TelegramTextFormat::MarkdownV2
}

// ---------------------------------------------------------------------------
// finalize-assistant-response.ts
// ---------------------------------------------------------------------------

/// Reply markup is attached only when a keyboard exists.
pub fn finalize_has_reply_options(has_keyboard: bool) -> bool {
    has_keyboard
}

/// Stream completion requests first-part notification iff asked.
pub fn finalize_uses_notify_complete(notify_first_final_part: bool) -> bool {
    notify_first_final_part
}

/// Only the first final part keeps reply markup after a notify request.
pub fn finalize_part_gets_reply_markup(notify_first: bool, part_index: usize) -> bool {
    notify_first && part_index == 0
}
