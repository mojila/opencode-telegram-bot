//! Question + permission + inline-menu + context-control pure gates (slice 58, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/menus/question-menu.ts` (`MAX_BUTTON_LENGTH`, the `❓` / `✏️`
//!   markers, the `question:*` callback builders, the `parseInt` index
//!   parses, the `n/total` progress text, `formatButtonText` 60-char `...`
//!   cut, the custom-answer `\s*\n\s*` collapse, the segment-length /
//!   plain-text helpers, the surrogate-safe slice, the tail-truncation
//!   window, the fallback-text `\n\n` join)
//! - `src/bot/callbacks/question-callback-handler.ts` (the `question:`
//!   prefix guard, the action dispatch names, the `parseInt` index gates,
//!   the last-question gate)
//! - `src/bot/menus/permission-menu.ts` (`permission:*` callbacks, the
//!   permission emoji map with the `🔐` fallback, the `once`/`always`/
//!   `reject` reply gate, the `PermissionNotFoundError` / `NotFoundError` /
//!   message match, the not-modified match, the outcome-line branches, the
//!   `4096` pattern budget with the `• …` cut)
//! - `src/bot/callbacks/permission-callback-handler.ts` (the `permission:`
//!   prefix guard, the reply classification)
//! - `src/bot/menus/inline-menu.ts` (`INLINE_MENU_CANCEL_PREFIX`, the legacy
//!   `compact:cancel`, the 11 menu kinds, the cancel callback-data build /
//!   parse)
//! - `src/bot/callbacks/inline-menu-cancel-callback-handler.ts` (legacy vs
//!   prefixed cancel parse)
//! - `src/bot/callbacks/callback-router.ts` (`parseCallbackPrefix`:
//!   `indexOf(":") <= 0` reads as no route)
//! - `src/bot/callbacks/feedback.ts` (`CALLBACK_ANSWER_MAX_LENGTH`, the
//!   200-char answer truncation)
//! - `src/bot/menus/context-control-menu.ts` (`compact:details` /
//!   `compact:confirm` callbacks, the `Math.round(used / limit * 100)`
//!   percent with the zero-limit `0`, the `$xx.xx` cost amount)
//! - `src/bot/callbacks/context-control-callback-handler.ts` (the exact
//!   `compact:details` / `compact:confirm` match guards)
//!
//! No grammy/Telegram I/O, no SDK calls, no store/`config`/env reads live
//! here — only callback prefixes, `parseInt` parses, label cuts, truncation
//! windows, status maps, and routing predicates. Callers resolve `t(...)`
//! locales, keyboards, and managers.
//!
//! DEVIATIONS from TS:
//! 1. `String.length` / `slice` / `substring` count UTF-16 code units in TS.
//!    The port works on `char`s; parity fixtures use ASCII-only inputs where
//!    the two coincide (emoji constants are compared as whole strings).
//! 2. `parseInt(x, 10)` tolerates leading whitespace, an optional sign, and
//!    trailing garbage (`"2x"` reads as `2`); only a digit-less payload is
//!    `NaN`. The port mirrors that with a leading-digit run; fixtures stick
//!    to decimal/empty/alpha/trailing-garbage inputs.
//! 3. `t(...)` lookups (headers, outcome lines, grouped text, status lines,
//!    button labels) are implicit in TS. The port takes translated text as
//!    explicit arguments or returns a classification enum so the gates stay
//!    pure; callers render the locale.
//! 4. `isOpencodeNotFoundError` / `isRecord` over `unknown` errors are
//!    flattened to explicit `(tag, is_not_found)` arguments.
//! 5. The two identical private `sliceOnSafeBoundary` helpers (question +
//!    permission menus) map to the single [`slice_on_safe_boundary`]; the
//!    permission variant lacks the `endIndex > 0` guard in TS, which only
//!    matters for empty inputs where both read as `""`.
//! 6. `Number.toFixed(2)` / `Math.round` float formatting follows the
//!    shortest-round-trip decimal form; fixtures use values exact in
//!    binary (`3.5`, `25`, `33.33…` avoided).

// ---------------------------------------------------------------------------
// Question menu (`bot/menus/question-menu.ts`)
// ---------------------------------------------------------------------------

/// Inline-button label budget for question option buttons.
pub const QUESTION_BUTTON_MAX_LENGTH: usize = 60;

/// Telegram message budget the question card truncation honors.
pub const QUESTION_MESSAGE_LIMIT: usize = 4096;

/// Suffix marking a truncated question segment.
pub const QUESTION_TRUNCATION_SUFFIX: &str = "…";

/// Emoji leading the question card header.
pub const QUESTION_EMOJI: &str = "❓";

/// Emoji leading the custom-answer option row.
pub const CUSTOM_ANSWER_EMOJI: &str = "✏️";

/// `question:` callback prefix.
pub const QUESTION_CALLBACK_PREFIX: &str = "question:";

/// One paragraph of the question card: an optional bold lead-in plus text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionSegment {
    pub label: Option<String>,
    pub rest: String,
}

/// Where a question sits in its poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionProgress {
    pub index: u64,
    pub total: u64,
}

/// Question callback actions (TS: the `switch (action)` in the handler).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionCallbackAction {
    Select,
    Submit,
    Custom,
    ToggleCustom,
    Cancel,
}

/// A parsed `question:*` callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionCallback {
    pub action: QuestionCallbackAction,
    pub question_index: i64,
    pub option_index: Option<i64>,
}

/// `parseInt(x, 10)` with radix 10: leading whitespace, optional sign,
/// leading decimal digit run; `None` when there is no digit.
fn parse_js_int_decimal(text: &str) -> Option<i64> {
    let trimmed = text.trim_start();
    let (sign, rest) = match trimmed.strip_prefix('+') {
        Some(r) => (1i64, r),
        None => match trimmed.strip_prefix('-') {
            Some(r) => (-1i64, r),
            None => (1i64, trimmed),
        },
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i64>().ok().map(|v| sign * v)
}

/// `question:select:<q>:<o>` callback.
pub fn build_question_select_callback(question_index: u64, option_index: u64) -> String {
    format!("question:select:{question_index}:{option_index}")
}

/// `question:submit:<q>` callback.
pub fn build_question_submit_callback(question_index: u64) -> String {
    format!("question:submit:{question_index}")
}

/// `question:custom:<q>` callback.
pub fn build_question_custom_callback(question_index: u64) -> String {
    format!("question:custom:{question_index}")
}

/// `question:toggle_custom:<q>` callback.
pub fn build_question_toggle_custom_callback(question_index: u64) -> String {
    format!("question:toggle_custom:{question_index}")
}

/// `question:cancel:<q>` callback.
pub fn build_question_cancel_callback(question_index: u64) -> String {
    format!("question:cancel:{question_index}")
}

/// Parse a `question:*` callback: action dispatch name plus `parseInt`
/// index gates (TS: `handleQuestionCallback`).
pub fn parse_question_callback(data: &str) -> Option<QuestionCallback> {
    if !is_question_callback(data) {
        return None;
    }
    let parts: Vec<&str> = data.split(':').collect();
    let action = parts.get(1).copied().unwrap_or("");
    let question_index = parse_js_int_decimal(parts.get(2).copied().unwrap_or(""))?;
    match action {
        "select" => {
            let option_index = parse_js_int_decimal(parts.get(3).copied().unwrap_or(""))?;
            Some(QuestionCallback {
                action: QuestionCallbackAction::Select,
                question_index,
                option_index: Some(option_index),
            })
        }
        "submit" => Some(QuestionCallback {
            action: QuestionCallbackAction::Submit,
            question_index,
            option_index: None,
        }),
        "custom" => Some(QuestionCallback {
            action: QuestionCallbackAction::Custom,
            question_index,
            option_index: None,
        }),
        "toggle_custom" => Some(QuestionCallback {
            action: QuestionCallbackAction::ToggleCustom,
            question_index,
            option_index: None,
        }),
        "cancel" => Some(QuestionCallback {
            action: QuestionCallbackAction::Cancel,
            question_index,
            option_index: None,
        }),
        _ => None,
    }
}

/// `true` for any `question:*` payload (TS: the `startsWith` guard).
pub fn is_question_callback(data: &str) -> bool {
    data.starts_with(QUESTION_CALLBACK_PREFIX)
}

/// The poll's last question: answering it sends the whole poll to OpenCode
/// (TS: `questionIndex === getTotalQuestions() - 1`).
pub fn is_last_question(question_index: i64, total_questions: i64) -> bool {
    question_index == total_questions - 1
}

/// A reply to a request OpenCode no longer has pending: not found on either
/// version, or a V2 form that was already settled (TS:
/// `isQuestionRequestGone`).
pub fn is_question_request_gone(tag: Option<&str>, is_not_found: bool) -> bool {
    if is_not_found {
        return true;
    }
    matches!(tag, Some("QuestionNotFoundError") | Some("FormAlreadySettledError"))
}

/// `n/total` progress text, empty when there is no total (TS:
/// `formatQuestionDetailsPart`).
pub fn question_progress_text(index: u64, total: u64) -> String {
    if total == 0 {
        return String::new();
    }
    format!("{}/{}", index + 1, total)
}

/// Option button text: `✅ ` prefix when selected, 60-char `...` cut
/// (TS: `formatButtonText`).
pub fn format_question_button_text(label: &str, selected: bool) -> String {
    let icon = if selected { "✅ " } else { "" };
    let text = format!("{icon}{label}");
    if text.chars().count() <= QUESTION_BUTTON_MAX_LENGTH {
        return text;
    }
    let kept: String = text
        .chars()
        .take(QUESTION_BUTTON_MAX_LENGTH - 3)
        .collect();
    format!("{kept}...")
}

/// Collapse each newline plus surrounding whitespace to a single space
/// (TS: `customAnswer.replace(/\s*\n\s*/g, " ")`).
pub fn normalize_custom_answer_label(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            let mut j = i;
            let mut has_newline = false;
            while j < chars.len() && chars[j].is_whitespace() {
                if chars[j] == '\n' {
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
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Length of a card segment: label plus rest (TS: `segmentLength`).
pub fn question_segment_length(segment: &QuestionSegment) -> usize {
    segment.label.as_ref().map(|s| s.chars().count()).unwrap_or(0) + segment.rest.chars().count()
}

/// Plain text of a card segment: label plus rest (TS:
/// `segmentToPlainText`).
pub fn question_segment_plain_text(segment: &QuestionSegment) -> String {
    format!("{}{}", segment.label.as_deref().unwrap_or(""), segment.rest)
}

/// Surrogate-safe prefix cut (TS: `sliceOnSafeBoundary`).
pub fn slice_on_safe_boundary(text: &str, max_length: usize) -> String {
    text.chars().take(max_length).collect()
}

/// Keep the card within the limit by dropping whole segments from the tail,
/// cutting only the last one that still partially fits (TS:
/// `truncateQuestionSegments` with the `\n\n` separator of length 2).
pub fn truncate_question_segments(
    segments: &[QuestionSegment],
    limit: usize,
) -> Vec<QuestionSegment> {
    const SEPARATOR_LENGTH: usize = 2;
    let suffix_len = QUESTION_TRUNCATION_SUFFIX.chars().count();
    let mut result: Vec<QuestionSegment> = Vec::new();
    let mut used: usize = 0;
    for segment in segments {
        let prefix = if result.is_empty() { 0 } else { SEPARATOR_LENGTH };
        let length = question_segment_length(segment);
        if used.saturating_add(prefix).saturating_add(length) <= limit {
            result.push(segment.clone());
            used += prefix + length;
            continue;
        }
        let available = limit
            .saturating_sub(used)
            .saturating_sub(prefix)
            .saturating_sub(suffix_len);
        let label_len = segment.label.as_ref().map(|s| s.chars().count()).unwrap_or(0);
        if available > label_len {
            let rest_cut =
                slice_on_safe_boundary(&segment.rest, available.saturating_sub(label_len));
            result.push(QuestionSegment {
                label: segment.label.clone(),
                rest: format!("{rest_cut}{QUESTION_TRUNCATION_SUFFIX}"),
            });
        } else if !result.is_empty() {
            let last = result.pop().expect("non-empty");
            result.push(QuestionSegment {
                label: last.label,
                rest: format!("{}{QUESTION_TRUNCATION_SUFFIX}", last.rest),
            });
        } else {
            let plain = question_segment_plain_text(segment);
            let cut = slice_on_safe_boundary(&plain, limit.saturating_sub(suffix_len));
            result.push(QuestionSegment {
                label: None,
                rest: format!("{cut}{QUESTION_TRUNCATION_SUFFIX}"),
            });
        }
        break;
    }
    result
}

/// Fallback text of the card: plain segments joined with `\n\n` (TS:
/// `formatQuestionDetailsPart`).
pub fn question_fallback_text(segments: &[QuestionSegment]) -> String {
    segments
        .iter()
        .map(question_segment_plain_text)
        .collect::<Vec<_>>()
        .join("\n\n")
}

// ---------------------------------------------------------------------------
// Permission menu (`bot/menus/permission-menu.ts`)
// ---------------------------------------------------------------------------

/// `permission:` callback prefix.
pub const PERMISSION_CALLBACK_PREFIX: &str = "permission:";
/// `permission:once` callback data.
pub const PERMISSION_CALLBACK_ONCE: &str = "permission:once";
/// `permission:always` callback data.
pub const PERMISSION_CALLBACK_ALWAYS: &str = "permission:always";
/// `permission:reject` callback data.
pub const PERMISSION_CALLBACK_REJECT: &str = "permission:reject";

/// Telegram message budget the pattern list truncation honors.
pub const PERMISSION_MESSAGE_LIMIT: usize = 4096;

/// Pattern line prefix.
pub const PERMISSION_PATTERN_PREFIX: &str = "• ";
/// Cut suffix marking a shortened pattern list.
pub const PERMISSION_CUT_SUFFIX: &str = "…\n";
/// Whole cut marker line when nothing more fits.
pub const PERMISSION_CUT_PATTERNS_LINE: &str = "• …\n";

/// Possible permission responses (TS: `PermissionReply`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionReply {
    Once,
    Always,
    Reject,
}

/// How a permission prompt ended (TS: `PermissionOutcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOutcomeKind {
    Replied { reply: PermissionReply, outside: bool },
    SettledOutside,
    NotAnswered,
}

/// Which outcome line a closed prompt shows (TS: `formatOutcomeLine`
/// branches; callers resolve the `t(...)` text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOutcomeLine {
    ReplyOnce,
    ReplyAlways,
    ReplyReject,
    ReplyOnceOutside,
    ReplyAlwaysOutside,
    ReplyRejectOutside,
    SettledOutside,
    NotAnswered,
}

/// Flattened error shape for the not-found check (TS inspects `unknown`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionErrorShape {
    pub tag: Option<String>,
    pub name: Option<String>,
    pub message: Option<String>,
    pub data_message: Option<String>,
}

/// Status glyph for a permission type, `🔐` fallback (TS:
/// `PERMISSION_EMOJIS[request.permission] || "🔐"`).
pub fn permission_emoji_for(permission: &str) -> &'static str {
    match permission {
        "bash" => "⚡",
        "edit" => "✏️",
        "write" => "📝",
        "read" => "📖",
        "webfetch" => "🌐",
        "websearch" => "🔍",
        "glob" => "📁",
        "grep" => "🔎",
        "list" => "📂",
        "task" => "⚙️",
        "lsp" => "🔧",
        "external_directory" => "📁",
        _ => "🔐",
    }
}

/// `true` for `once` / `always` / `reject` (TS: `isPermissionReply`).
pub fn is_permission_reply_value(value: &str) -> bool {
    matches!(value, "once" | "always" | "reject")
}

/// Which reply a `permission:*` payload carries; `None` otherwise (TS: the
/// action gate in `handlePermissionCallback`).
pub fn classify_permission_callback(data: &str) -> Option<PermissionReply> {
    if !is_permission_callback(data) {
        return None;
    }
    let action = data.split(':').nth(1).unwrap_or("");
    match action {
        "once" => Some(PermissionReply::Once),
        "always" => Some(PermissionReply::Always),
        "reject" => Some(PermissionReply::Reject),
        _ => None,
    }
}

/// `true` for any `permission:*` payload (TS: the `startsWith` guard).
pub fn is_permission_callback(data: &str) -> bool {
    data.starts_with(PERMISSION_CALLBACK_PREFIX)
}

/// The reply never reached OpenCode because the request is gone: the
/// `PermissionNotFoundError` tag, the `NotFoundError` name, or a message
/// containing "permission request not found" (TS:
/// `isPermissionRequestNotFound`).
pub fn is_permission_request_not_found(error: &PermissionErrorShape) -> bool {
    if error.tag.as_deref() == Some("PermissionNotFoundError") {
        return true;
    }
    if error.name.as_deref() == Some("NotFoundError") {
        return true;
    }
    for candidate in [&error.message, &error.data_message] {
        if let Some(text) = candidate {
            if text.to_lowercase().contains("permission request not found") {
                return true;
            }
        }
    }
    false
}

/// The edit failed only because the text is unchanged (TS:
/// `isMessageNotModifiedError`).
pub fn is_message_not_modified_message(message: &str) -> bool {
    message.to_lowercase().contains("message is not modified")
}

/// Which outcome line a closed prompt shows (TS: `formatOutcomeLine`).
pub fn classify_permission_outcome_line(outcome: PermissionOutcomeKind) -> PermissionOutcomeLine {
    match outcome {
        PermissionOutcomeKind::Replied { reply, outside } => match (reply, outside) {
            (PermissionReply::Once, false) => PermissionOutcomeLine::ReplyOnce,
            (PermissionReply::Once, true) => PermissionOutcomeLine::ReplyOnceOutside,
            (PermissionReply::Always, false) => PermissionOutcomeLine::ReplyAlways,
            (PermissionReply::Always, true) => PermissionOutcomeLine::ReplyAlwaysOutside,
            (PermissionReply::Reject, false) => PermissionOutcomeLine::ReplyReject,
            (PermissionReply::Reject, true) => PermissionOutcomeLine::ReplyRejectOutside,
        },
        PermissionOutcomeKind::SettledOutside => PermissionOutcomeLine::SettledOutside,
        PermissionOutcomeKind::NotAnswered => PermissionOutcomeLine::NotAnswered,
    }
}

/// Permission request text over the already translated header / grouped /
/// status lines: patterns are cut when the whole text would not fit into one
/// Telegram message (TS: `formatPermissionText`).
pub fn format_permission_text(
    header: &str,
    patterns: &[String],
    grouped_text: &str,
    status_line: Option<&str>,
) -> String {
    let status = status_line.map(|s| format!("\n{s}")).unwrap_or_default();
    let header_len = header.chars().count();
    let grouped_len = grouped_text.chars().count();
    let status_len = status.chars().count();
    let mut budget = PERMISSION_MESSAGE_LIMIT
        .saturating_sub(header_len)
        .saturating_sub(grouped_len)
        .saturating_sub(status_len);
    let cut_line_len = PERMISSION_CUT_PATTERNS_LINE.chars().count();
    let mut rendered = String::new();
    let total = patterns.len();
    for (index, pattern) in patterns.iter().enumerate() {
        let line_len = PERMISSION_PATTERN_PREFIX.chars().count() + pattern.chars().count() + 1;
        let reserve = if index == total - 1 { 0 } else { cut_line_len };
        if line_len.saturating_add(reserve) <= budget {
            rendered.push_str(PERMISSION_PATTERN_PREFIX);
            rendered.push_str(pattern);
            rendered.push('\n');
            budget -= line_len;
            continue;
        }
        let room = budget.saturating_sub(cut_line_len);
        if room > 0 {
            rendered.push_str(PERMISSION_PATTERN_PREFIX);
            rendered.push_str(&slice_on_safe_boundary(pattern, room));
            rendered.push_str(PERMISSION_CUT_SUFFIX);
        } else {
            rendered.push_str(PERMISSION_CUT_PATTERNS_LINE);
        }
        break;
    }
    format!("{header}{rendered}{grouped_text}{status}")
}

// ---------------------------------------------------------------------------
// Inline menu + callback router + feedback
// (`bot/menus/inline-menu.ts`,
// `bot/callbacks/inline-menu-cancel-callback-handler.ts`,
// `bot/callbacks/callback-router.ts`, `bot/callbacks/feedback.ts`)
// ---------------------------------------------------------------------------

/// `inline:cancel:` callback prefix.
pub const INLINE_MENU_CANCEL_PREFIX: &str = "inline:cancel:";
/// Legacy cancel callback for the context menu.
pub const LEGACY_CONTEXT_CANCEL_CALLBACK: &str = "compact:cancel";

/// All inline menu kinds (TS: `INLINE_MENU_KINDS`).
pub const INLINE_MENU_KINDS: [&str; 11] = [
    "project",
    "session",
    "recent",
    "model",
    "agent",
    "variant",
    "context",
    "open",
    "ls",
    "worktree",
    "settings",
];

/// Telegram limit for callback answer toasts (TS:
/// `CALLBACK_ANSWER_MAX_LENGTH`).
pub const CALLBACK_ANSWER_MAX_LENGTH: usize = 200;

/// Inline menu kinds (TS: `InlineMenuKind`).
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

/// Canonical name of an inline menu kind.
pub fn inline_menu_kind_name(kind: InlineMenuKind) -> &'static str {
    match kind {
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

/// `true` for one of the 11 menu-kind names (TS: `isInlineMenuKind`).
pub fn is_inline_menu_kind(value: &str) -> bool {
    parse_inline_menu_kind(value).is_some()
}

/// Parse a menu-kind name; `None` for anything else.
pub fn parse_inline_menu_kind(value: &str) -> Option<InlineMenuKind> {
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

/// `inline:cancel:<kind>` callback data (TS:
/// `getInlineCancelCallbackData`).
pub fn inline_cancel_callback_data(kind: InlineMenuKind) -> String {
    format!("{INLINE_MENU_CANCEL_PREFIX}{}", inline_menu_kind_name(kind))
}

/// Parse an inline-menu cancel callback: the legacy `compact:cancel` reads
/// as context, `inline:cancel:<kind>` reads as that kind, anything else is
/// not a cancel (TS: `handleInlineMenuCancel`).
pub fn parse_inline_cancel_callback(data: &str) -> Option<InlineMenuKind> {
    if data == LEGACY_CONTEXT_CANCEL_CALLBACK {
        return Some(InlineMenuKind::Context);
    }
    if let Some(raw) = data.strip_prefix(INLINE_MENU_CANCEL_PREFIX) {
        return parse_inline_menu_kind(raw);
    }
    None
}

/// Route prefix of a callback payload: text before the first `:`; `None`
/// when there is no usable separator (TS: `parseCallbackPrefix` with
/// `separatorIndex <= 0`).
pub fn parse_callback_prefix(data: &str) -> Option<&str> {
    let index = data.find(':')?;
    if index == 0 {
        return None;
    }
    Some(&data[..index])
}

/// Truncate a callback answer toast to the Telegram limit (TS:
/// `resolveAnswerText`).
pub fn truncate_callback_answer_text(text: &str) -> String {
    if text.chars().count() <= CALLBACK_ANSWER_MAX_LENGTH {
        return text.to_string();
    }
    text.chars().take(CALLBACK_ANSWER_MAX_LENGTH).collect()
}

// ---------------------------------------------------------------------------
// Context control (`bot/menus/context-control-menu.ts`,
// `bot/callbacks/context-control-callback-handler.ts`)
// ---------------------------------------------------------------------------

/// `compact:details` callback data.
pub const COMPACT_DETAILS_CALLBACK: &str = "compact:details";
/// `compact:confirm` callback data.
pub const COMPACT_CONFIRM_CALLBACK: &str = "compact:confirm";

/// Compact-menu callbacks (TS: the exact-match guards in the compact
/// handlers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactCallback {
    Details,
    Confirm,
}

/// Which compact callback a payload is; `None` for anything else.
pub fn classify_compact_callback(data: &str) -> Option<CompactCallback> {
    match data {
        COMPACT_DETAILS_CALLBACK => Some(CompactCallback::Details),
        COMPACT_CONFIRM_CALLBACK => Some(CompactCallback::Confirm),
        _ => None,
    }
}

/// `true` for either compact callback payload.
pub fn is_compact_callback(data: &str) -> bool {
    classify_compact_callback(data).is_some()
}

/// Context usage percent: `Math.round(used / limit * 100)`, `0` without a
/// limit (TS: `formatContextDetails`).
pub fn format_context_percent(tokens_used: u64, tokens_limit: u64) -> u64 {
    if tokens_limit == 0 {
        return 0;
    }
    ((tokens_used as f64 / tokens_limit as f64) * 100.0).round() as u64
}

/// Cost amount with a `$` prefix and two decimals (TS:
/// `` `$${metrics.cost.toFixed(2)}` ``).
pub fn format_cost_amount(cost: f64) -> String {
    format!("${cost:.2}")
}
