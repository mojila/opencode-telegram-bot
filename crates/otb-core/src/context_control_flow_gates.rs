//! Compact flow gates (slice 166, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/bot/callbacks/context-control-callback-handler.test.ts`
//! (the "context compaction flow" contract over
//! `src/bot/callbacks/context-control-callback-handler.ts` +
//! `src/bot/callbacks/inline-menu-cancel-callback-handler.ts` for the
//! `context` menu):
//! - exact dispatch data (`compact:details` / `compact:confirm` /
//!   `inline:cancel:context`; anything else, including missing data, is
//!   not this flow)
//! - the details/confirm stage gates (details only proceeds from
//!   `"details"` and moves to `"confirm"`; confirm only compacts from
//!   `"confirm"`, so an early `compact:confirm` is inactive and never
//!   summarizes)
//! - the callback-message freshness gate (a callback from a different
//!   message than the active inline menu is stale and edits nothing)
//! - the cancel gate (an `inline:cancel:context` callback on a fresh
//!   message clears the flow from either stage without compacting)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `tests/bot/callbacks/context-control-callback-handler.test.ts`
//!   lines 35-60 (details shows confirmation, confirm compacts once),
//!   62-72 (cancel clears from `details` and `confirm` without
//!   summarizing), 74-86 (stale message and repeated details are
//!   rejected without summarizing).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, `sendChatAction`).
//! - `interactionManager` snapshot/transition reads and writes,
//!   `ensureActiveInlineMenu` / `clearActiveInlineMenu` effects,
//!   `getCurrentSession` / `fetchSessionTitle` / `getStoredModel` reads,
//!   `opencodeClient.session.summarize` calls.
//! - `t(...)` lookups: confirm/progress/success/error keys travel as
//!   literal key strings in the caller.
//! - answer-before-edit await ordering (test lines 100-114) is timing,
//!   not a gate.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `context_inline_gates::COMPACT_DETAILS_CALLBACK`,
//!   `COMPACT_CONFIRM_CALLBACK`, the details/confirm stage checks, the
//!   missing-session `context.no_active_session` gate (slice 95 owner;
//!   the dispatch strings are repeated below only as the dispatch table
//!   this flow matches, per the slice 92 precedent).
//! - `worktree_history_inline_gates::INLINE_MENU_CANCEL_PREFIX`,
//!   `LEGACY_CONTEXT_CANCEL_CALLBACK`, `parse_inline_cancel_callback`
//!   (the kind table and legacy `compact:cancel` mapping behind the
//!   scoped `inline:cancel:context` data below).
//! - `formatters::extra_formatters::format_session_title` (the empty
//!   title -> default-session-title fallback behind the confirm text,
//!   test lines 88-98).
//!
//! Deviations: none intended. `messageId` values are `i64` (`typeof ===
//! `"number"` in TS); a missing callback message or no active menu reads
//! as `None` and is never fresh. Stage arrives as an explicit
//! `Option<&str>` so the decide gate stays self-contained.
//!
//! Step 2 (GREEN): pure sync logic ported from the TS contract asserted
//! by the parity tests in
//! `crates/otb-core/tests/slice166_context_control_flow_parity.rs`.

/// Exact details callback data (TS: `"compact:details"`).
pub const COMPACT_DETAILS_DATA: &str = "compact:details";
/// Exact confirm callback data (TS: `"compact:confirm"`).
pub const COMPACT_CONFIRM_DATA: &str = "compact:confirm";
/// Context-scoped inline-menu cancel data (TS: `"inline:cancel:context"`).
pub const CONTEXT_CANCEL_DATA: &str = "inline:cancel:context";
/// Inline menu kind this flow belongs to (TS: `"context"`).
pub const CONTEXT_MENU_KIND: &str = "context";
/// Stage that accepts the details action (TS: `"details"`).
pub const DETAILS_STAGE: &str = "details";
/// Stage that accepts the confirm action (TS: `"confirm"`).
pub const CONFIRM_STAGE: &str = "confirm";

/// Outcome of one compact-flow callback (TS: the `true`/`false` return
/// plus which effects the handler performs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactFlowAction {
    /// Not this flow (`handleCompactDetails` / `handleCompactConfirm` /
    /// `handleInlineMenuCancel` return `false`).
    Ignore,
    /// This flow but rejected: stale message or wrong stage (the handler
    /// answers `inline.inactive_callback` or drops the stale callback and
    /// performs no flow effect; never summarizes).
    Inactive,
    /// Details accepted: show the confirmation menu and move the flow to
    /// the confirm stage.
    ShowConfirm,
    /// Confirm accepted: summarize (compact) the session and clear the flow.
    Compact,
    /// Cancel accepted: clear the flow without summarizing.
    Cancel,
}

/// Details dispatch guard (TS: `ctx.callbackQuery?.data !==
/// "compact:details"` returns `false` in `handleCompactDetails`).
pub fn handles_compact_details(data: Option<&str>) -> bool {
    data == Some(COMPACT_DETAILS_DATA)
}

/// Confirm dispatch guard (TS: `callbackQuery.data !== "compact:confirm"`
/// returns `false` in `handleCompactConfirm`).
pub fn handles_compact_confirm(data: Option<&str>) -> bool {
    data == Some(COMPACT_CONFIRM_DATA)
}

/// Context-cancel dispatch guard (TS: `handleInlineMenuCancel` resolving
/// `"inline:cancel:context"` to the `context` menu).
pub fn handles_context_cancel(data: Option<&str>) -> bool {
    data == Some(CONTEXT_CANCEL_DATA)
}

/// Callback-message freshness gate (TS: the `ensureActiveInlineMenu`
/// message match — a callback from a different message than the active
/// inline menu is stale; a missing callback message or no active menu is
/// never fresh).
pub fn is_compact_message_fresh(
    callback_message_id: Option<i64>,
    active_message_id: Option<i64>,
) -> bool {
    match (callback_message_id, active_message_id) {
        (Some(callback), Some(active)) => callback == active,
        _ => false,
    }
}

/// Compact-flow decision (TS: the dispatch + `ensureActiveInlineMenu` +
/// `metadata.stage` composition across `handleCompactDetails`,
/// `handleCompactConfirm`, and `handleInlineMenuCancel` for the context
/// menu). `stage` is the active inline menu's stage (`None` = no active
/// context menu); `message_fresh` is [`is_compact_message_fresh`].
pub fn decide_compact_flow(
    data: Option<&str>,
    stage: Option<&str>,
    message_fresh: bool,
) -> CompactFlowAction {
    let data = match data {
        Some(data) => data,
        None => return CompactFlowAction::Ignore,
    };
    if data == COMPACT_DETAILS_DATA {
        if !message_fresh || stage != Some(DETAILS_STAGE) {
            return CompactFlowAction::Inactive;
        }
        return CompactFlowAction::ShowConfirm;
    }
    if data == COMPACT_CONFIRM_DATA {
        if !message_fresh || stage != Some(CONFIRM_STAGE) {
            return CompactFlowAction::Inactive;
        }
        return CompactFlowAction::Compact;
    }
    if data == CONTEXT_CANCEL_DATA {
        if !message_fresh {
            return CompactFlowAction::Inactive;
        }
        return CompactFlowAction::Cancel;
    }
    CompactFlowAction::Ignore
}
