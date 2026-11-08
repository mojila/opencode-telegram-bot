//! Context-control + feedback + inline-menu active-state gates (slice 95, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/feedback.ts`
//!   (`CALLBACK_ANSWER_MAX_LENGTH` 200, `resolveAnswerText`
//!   over-budget head cut)
//! - `src/bot/callbacks/context-control-callback-handler.ts`
//!   (`handleCompactDetails` / `handleCompactConfirm` dispatch: the exact
//!   `compact:details` / `compact:confirm` data match, the
//!   `metadata.stage === "details"` / `"confirm"` gate, the missing-session
//!   `context.no_active_session` gate)
//! - `src/bot/callbacks/inline-menu-cancel-callback-handler.ts` remainder
//!   (the `ensureActiveInlineMenu` / `clearActiveInlineMenu` flow behind the
//!   parse; the parse itself lives in slice 54)
//! - `src/bot/menus/inline-menu.ts` remainder
//!   (`getCallbackMessageId` numeric-`message_id` gate,
//!   `getActiveInlineMenuMetadata` validation, `ensureActiveInlineMenu`
//!   match decision, `clearActiveInlineMenu` kind gate,
//!   `appendInlineMenuCancelButton` trailing-empty-row trim + row insert +
//!   settings-`close`-vs-others-`cancel` text + cancel data)
//! - `src/bot/menus/context-control-menu.ts`
//!   (`compact:details` / `compact:confirm` literals, `formatContextDetails`
//!   window percent (`limit > 0 ? round(used / limit * 100) : 0`),
//!   unknown-limit label, metrics-lines presence, `$X.XX` cost line)
//!
//! Already ported elsewhere and NOT repeated here:
//! - `worktree_history_inline_gates::INLINE_MENU_KINDS`,
//!   `is_inline_menu_kind`, `INLINE_MENU_CANCEL_PREFIX`,
//!   `LEGACY_CONTEXT_CANCEL_CALLBACK`, `parse_inline_cancel_callback`
//!   (the kind table, the cancel-data build/parse behind the dispatch
//!   below; this slice takes kind-known outcomes as explicit bools and
//!   owns only the trailing-row/button-text/active-state gates).
//! - `keyboards::{InlineButton-ish}` shapes are restated locally as
//!   [`InlineButton`] so the append gate stays self-contained; the
//!   `formatTokenCount` rendering behind the details window line lives
//!   with the pinned/keyboard owners and arrives here preformatted.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, `sendChatAction`, transitions, `clear`).
//! - `interactionManager` snapshot/start/transition reads and writes,
//!   `promptAttachment` state, `getCurrentSession` / `fetchSessionTitle` /
//!   `getStoredModel` / `loadLatestAssistantMetrics` /
//!   `pinnedMessageManager.getContextInfo` reads,
//!   `opencodeClient.session.summarize` calls.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `messageId` values are `i64` (`typeof ===
//! `"number"` in TS), a missing message or a non-numeric `message_id`
//! reads as `None`. `String.length` / `slice` count UTF-16 code units in
//! TS; the port works on `char`s and parity fixtures use ASCII-only inputs
//! where the two coincide. Cost lines use `format!("${cost:.2}")` (TS:
//! `` `$${cost.toFixed(2)}` ``).

/// Telegram callback-answer budget (TS: `CALLBACK_ANSWER_MAX_LENGTH`).
pub const CALLBACK_ANSWER_MAX_LENGTH: usize = 200;

/// Exact details callback data (TS: `"compact:details"`).
pub const COMPACT_DETAILS_CALLBACK: &str = "compact:details";
/// Exact confirm callback data (TS: `"compact:confirm"`).
pub const COMPACT_CONFIRM_CALLBACK: &str = "compact:confirm";
/// Stale-menu answer key (TS: `t("inline.inactive_callback")`).
pub const INLINE_MENU_INACTIVE_KEY: &str = "inline.inactive_callback";
/// Missing-session answer key (TS: `t("context.no_active_session")`).
pub const CONTEXT_NO_SESSION_KEY: &str = "context.no_active_session";

/// One inline-keyboard button (TS: grammy `InlineKeyboard` cell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineButton {
    pub text: String,
    pub callback_data: String,
}

/// Resolved active inline menu (TS: `getActiveInlineMenuMetadata`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveInlineMenuRef {
    pub menu_kind: String,
    pub message_id: i64,
}

/// Over-budget head cut (TS: `resolveAnswerText`).
pub fn truncate_callback_answer(text: &str) -> String {
    if text.chars().count() <= CALLBACK_ANSWER_MAX_LENGTH {
        return text.to_string();
    }
    text.chars()
        .take(CALLBACK_ANSWER_MAX_LENGTH)
        .collect()
}

/// `true` when the answer needs the head cut (TS: `length > 200`).
pub fn should_truncate_callback_answer(char_count: usize) -> bool {
    char_count > CALLBACK_ANSWER_MAX_LENGTH
}

/// Callback message-id resolve (TS: `getCallbackMessageId` in
/// `inline-menu.ts`: no message or no numeric `message_id` reads as
/// `None`).
pub fn inline_callback_message_id(
    has_message_with_id: bool,
    message_id: Option<i64>,
) -> Option<i64> {
    if has_message_with_id {
        message_id
    } else {
        None
    }
}

/// Active-menu metadata resolve (TS: `getActiveInlineMenuMetadata`:
/// non-`inline` state, unknown/missing kind, or missing numeric
/// `messageId` reads as `None`; kind recognition arrives as
/// `kind_known` from slice 54).
pub fn resolve_active_inline_metadata(
    is_inline_state: bool,
    menu_kind: Option<&str>,
    kind_known: bool,
    message_id: Option<i64>,
) -> Option<ActiveInlineMenuRef> {
    if !is_inline_state || !kind_known {
        return None;
    }
    let kind = menu_kind?;
    let id = message_id?;
    Some(ActiveInlineMenuRef {
        menu_kind: kind.to_string(),
        message_id: id,
    })
}

/// Active-menu match decision (TS: `ensureActiveInlineMenu` `isActive`:
/// active metadata plus a numeric callback id plus kind plus message-id
/// equality).
pub fn is_inline_menu_active(
    has_active_metadata: bool,
    kind_matches: bool,
    message_matches: bool,
    has_callback_id: bool,
) -> bool {
    has_active_metadata && kind_matches && message_matches && has_callback_id
}

/// Clear gate (TS: `clearActiveInlineMenu` only clears an `inline` state).
pub fn should_clear_active_inline_menu(state_kind: Option<&str>) -> bool {
    state_kind == Some("inline")
}

/// `true` for the `settings` close text (TS:
/// `menuKind === "settings" ? t("inline.button.close") : ...`).
pub fn is_close_text_kind(menu_kind: &str) -> bool {
    menu_kind == "settings"
}

/// `inline:cancel:<kind>` (TS: `getInlineCancelCallbackData`).
pub fn inline_cancel_callback_for(menu_kind: &str) -> String {
    format!("inline:cancel:{menu_kind}")
}

/// Trailing empty-row count (TS: the `while` trim inside
/// `appendInlineMenuCancelButton`).
pub fn trailing_empty_row_count(row_is_empty: &[bool]) -> usize {
    row_is_empty
        .iter()
        .rev()
        .take_while(|is_empty| **is_empty)
        .count()
}

/// Append the cancel row (TS: `appendInlineMenuCancelButton`: drop
/// trailing empty rows, `row()` when rows remain, then push the cancel
/// button with `button_text` + `cancel_callback_data`).
pub fn append_inline_cancel_button(
    rows: Vec<Vec<InlineButton>>,
    button_text: &str,
    cancel_callback_data: &str,
) -> Vec<Vec<InlineButton>> {
    let mut out = rows;
    while out.last().is_some_and(|row| row.is_empty()) {
        out.pop();
    }
    out.push(vec![InlineButton {
        text: button_text.to_string(),
        callback_data: cancel_callback_data.to_string(),
    }]);
    out
}

/// Window percent (TS: `limit > 0 ? Math.round((used / limit) * 100) : 0`).
pub fn context_window_percent(tokens_used: u64, tokens_limit: u64) -> u64 {
    if tokens_limit == 0 {
        return 0;
    }
    ((tokens_used as f64 / tokens_limit as f64) * 100.0).round() as u64
}

/// `true` when the window limit is unknown (TS: `limit > 0 ? ... :
/// t("pinned.unknown")`).
pub fn is_unknown_window_limit(tokens_limit: u64) -> bool {
    tokens_limit == 0
}

/// `$X.XX` cost line (TS: `` `$${cost.toFixed(2)}` ``).
pub fn format_compact_cost(cost: f64) -> String {
    format!("${cost:.2}")
}

/// Exact-match dispatch for the details entry (TS:
/// `ctx.callbackQuery?.data !== "compact:details"` return-false guard).
pub fn is_compact_details_data(data: Option<&str>) -> bool {
    data == Some(COMPACT_DETAILS_CALLBACK)
}

/// Exact-match dispatch for the confirm entry (TS:
/// `!callbackQuery?.data || data !== "compact:confirm"` return-false
/// guard).
pub fn is_compact_confirm_data(data: Option<&str>) -> bool {
    data == Some(COMPACT_CONFIRM_CALLBACK)
}

/// Details-stage gate (TS: `metadata?.stage !== "details"` answers
/// `inline.inactive_callback`).
pub fn is_details_stage(stage: Option<&str>) -> bool {
    stage == Some("details")
}

/// Confirm-stage gate (TS: `metadata?.stage !== "confirm"` answers
/// `inline.inactive_callback`).
pub fn is_confirm_stage(stage: Option<&str>) -> bool {
    stage == Some("confirm")
}

/// Missing-session reject gate (TS: `!session` answers
/// `context.no_active_session`).
pub fn should_reject_compact_without_session(has_session: bool) -> bool {
    !has_session
}
