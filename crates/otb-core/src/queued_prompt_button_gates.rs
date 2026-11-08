//! Queued-prompt button-press dispatch gates (slice 116, GREEN).
//!
//! Rust port of the pure remainder in
//! `src/bot/routers/message-router.ts` (`bot.hears(
//! QUEUED_PROMPT_BUTTON_TEXT_PATTERN, ...)`):
//! - interaction block wins before any label lookup
//!   (`blockMenuWhileInteractionActive` early return)
//! - label lookup miss (`label` missing or `findQueuedPromptByButtonLabel`
//!   returning `null`: the queue was drained or cleared after Telegram
//!   rendered the keyboard)
//! - inbox-backed press withdraws via `withdrawInboxPrompt` and replies
//!   `removed -> "queue.removed"`, `gone -> "queue.not_found"`,
//!   otherwise `"bot.prompt_send_error"`
//! - locally queued press removes by id and replies `"queue.removed"`
//! - unknown-label press replies `"queue.not_found"`
//! - no branch ever falls through to the prompt handler: even the miss
//!   replies instead of sending the button label to OpenCode as a prompt.
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity tests in
//! `crates/otb-core/tests/slice116_queued_prompt_button_parity.rs`
//! assert the TS contract.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx` / `Api` I/O, `container.setTelegramContext`,
//!   `container.keyboardManager.getKeyboard` reads
//! - `findQueuedPromptByButtonLabel` queue scan (arrives here as
//!   precomputed `found` / `has_inbox` bools), `promptQueue.removeById`
//!   removal, `withdrawInboxPrompt` execution (arrives as a precomputed
//!   `withdraw_result` string)
//! - `t(...)` locale lookups (reply keys travel as literal key strings).
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `blockMenuWhileInteractionActive` active gate
//!   (`block_interaction_gates::should_block_menu_while_active` owner;
//!   blocked-ness arrives here as a precomputed `bool`)
//! - `removed` / `gone` / error sub-mapping
//!   (`block_interaction_gates::withdraw_reply_key` owner; the
//!   `WithdrawInbox` arm of `queued_prompt_button_reply_key` defers to
//!   that mapping, exactly like the TS ternary)
//! - `withdrawInboxPrompt` list/cancel outcome
//!   (`session_context_gates::resolve_withdraw_outcome` owner)
//! - `QUEUED_PROMPT_BUTTON_TEXT_PATTERN` matching and label formatting
//!   (`keyboards` / `session_gates` owners; pattern match arrives as the
//!   hears dispatch itself, label presence as `has_label`).
//!
//! Deviations: none intended. `label ? ... : null` (missing label) and a
//! label with no queue entry both land on `MissNotFound`, exactly like the
//! TS `queuedPrompt === null` fallthrough to the `queue.not_found` reply.

/// Reply key after a locally queued press is removed by id
/// (TS: `t("queue.removed")`).
pub const QUEUED_PROMPT_REMOVED_KEY: &str = "queue.removed";

/// Reply key when the pressed entry is no longer queued
/// (TS: `t("queue.not_found")` for a drained/cleared queue).
pub const QUEUED_PROMPT_NOT_FOUND_KEY: &str = "queue.not_found";

/// Reply key when an inbox withdraw fails unexpectedly
/// (TS: `t("bot.prompt_send_error")` fallthrough arm).
pub const QUEUED_PROMPT_SEND_ERROR_KEY: &str = "bot.prompt_send_error";

/// Reply key when the press lands while an interaction is active
/// (TS: `t("interaction.blocked.finish_current")` inside
/// `blockMenuWhileInteractionActive`).
pub const QUEUED_PROMPT_BLOCKED_KEY: &str = "interaction.blocked.finish_current";

/// Where one queued-prompt button press ends up (TS: the early returns
/// inside the `QUEUED_PROMPT_BUTTON_TEXT_PATTERN` hears handler).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuedPromptButtonRoute {
    /// Blocked by an active interaction before the label is read.
    Blocked,
    /// Inbox-backed entry: withdraw via `withdrawInboxPrompt`.
    WithdrawInbox,
    /// Locally queued entry: `promptQueue.removeById`.
    RemoveQueued,
    /// No entry for the label (missing label or drained queue).
    MissNotFound,
}

/// `true` when the handler reads the label and scans the queue
/// (TS: `const label = ctx.message?.text; const queuedPrompt = label ?
/// findQueuedPromptByButtonLabel(label) : null`).
pub fn should_lookup_queued_prompt(has_label: bool) -> bool {
    has_label
}

/// Full precedence for one button press (TS: the ordered early returns in
/// the hears handler: interaction block, then `queuedPrompt?.inbox`,
/// then `queuedPrompt`, else the drained-queue miss).
pub fn select_queued_prompt_button_route(
    interaction_blocked: bool,
    found: bool,
    has_inbox: bool,
) -> QueuedPromptButtonRoute {
    if interaction_blocked {
        return QueuedPromptButtonRoute::Blocked;
    }
    if found && has_inbox {
        return QueuedPromptButtonRoute::WithdrawInbox;
    }
    if found {
        return QueuedPromptButtonRoute::RemoveQueued;
    }
    QueuedPromptButtonRoute::MissNotFound
}

/// Reply key for one button press (TS: `t(replyKey)` per branch; the
/// `WithdrawInbox` arm is the `removed` / `gone` / error ternary).
pub fn queued_prompt_button_reply_key(
    route: QueuedPromptButtonRoute,
    withdraw_result: &str,
) -> &'static str {
    match route {
        QueuedPromptButtonRoute::WithdrawInbox => {
            if withdraw_result == "removed" {
                QUEUED_PROMPT_REMOVED_KEY
            } else if withdraw_result == "gone" {
                QUEUED_PROMPT_NOT_FOUND_KEY
            } else {
                QUEUED_PROMPT_SEND_ERROR_KEY
            }
        }
        QueuedPromptButtonRoute::RemoveQueued => QUEUED_PROMPT_REMOVED_KEY,
        QueuedPromptButtonRoute::MissNotFound => QUEUED_PROMPT_NOT_FOUND_KEY,
        QueuedPromptButtonRoute::Blocked => QUEUED_PROMPT_BLOCKED_KEY,
    }
}

/// `true` when the button text continues into the prompt handler
/// (TS: never — every branch replies and returns so the button label is
/// never sent to OpenCode as a prompt).
pub fn should_route_button_text_to_prompt(_route: QueuedPromptButtonRoute) -> bool {
    false
}
