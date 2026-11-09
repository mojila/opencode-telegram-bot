//! Interaction-guard edge gates (slice 64, GREEN).
//!
//! Rust port of the pure remainder of
//! `src/bot/middleware/interaction-guard-decision.ts` that
//! `streaming::decide_guard` does not cover, plus the reply-channel decision
//! in `src/bot/messages/busy-blocked-renderer.ts`:
//! - `isBusyAllowedCommand` with the `LocalCommandRegistry::allowsWhenBusy`
//!   fallback (`local-command-registry.ts`)
//! - `isBusyAllowedOutsideInteraction` (`/reload`, `/goal` + `isKnownCommand`)
//! - the busy `localCommandRegistry.has(command)` block gate
//! - the goal-button label branches (`GOAL_BUTTON_TEXT_PATTERN`, the
//!   `◎ `/`⏸ ` busy bypass, `flow === "goal"` cancel/confirm shortcut)
//! - `isAllowedRenameCancelCallback` / `isAllowedTaskCallback` (`rename:cancel`,
//!   `task:cancel`, `task:retry-schedule` from `scheduled-task-menu.ts`)
//! - `isQueuedPromptButtonPress` (`QUEUED_PROMPT_BUTTON_TEXT_PATTERN`)
//! - `replyBusyBlocked` channel pick (callback query -> `answerCallbackQuery`,
//!   else chat -> `reply`, else silent)
//!
//! No grammy/Telegram I/O, no manager/store reads, no `t(...)` locale lookups
//! live here — only name sets, pattern predicates, and small enum decisions.
//! Callers resolve `ctx` snapshots and registries.
//!
//! DEVIATIONS from TS:
//! 1. `isBusyAllowedCommand` / `has` close over `LocalCommandRegistry` in TS.
//!    The port takes the pre-resolved `registry_allows_when_busy` /
//!    `registry_has` booleans so the gate stays pure; callers query the
//!    registry.
//! 2. `isBusyAllowedOutsideInteraction` calls `isKnownCommand(command.slice(1))`
//!    in TS. The port takes the pre-resolved `is_known` boolean; callers pass
//!    the `BOT_COMMANDS` membership check.
//! 3. `resolveInteractionGuardDecision` acts on grammy `ctx` + managers in TS.
//!    The port reduces each branch to scalar/enum predicates; the middleware
//!    keeps the `ctx` wiring (as `streaming::decide_guard` already does for
//!    the core).
//! 4. `String.split(/\s+/)` in TS splits on Unicode whitespace; the port uses
//!    `char::is_whitespace`, which coincides on parity fixtures.
//! 5. `replyBusyBlocked` awaits Telegram sends in TS. The port returns only
//!    the channel decision; the caller performs the send.
//! 6. No regex crate: `GOAL_BUTTON_TEXT_PATTERN`
//!    (`/^(?:◎|⏸|▶|×)\s[\s\S]+$/`) and `QUEUED_PROMPT_BUTTON_TEXT_PATTERN`
//!    (`/^❌\s\d+\.\s/`) are hand-ported as `char` scans. `\s` maps to
//!    `char::is_whitespace` (JS `\s` additionally matches U+FEFF, which never
//!    appears in parity fixtures or real button labels) and `\d` maps to
//!    `is_ascii_digit` (exact match for JS `\d`, which is ASCII-only).
//! 7. `Boolean(command && ...)` in TS treats `""` as absent; the port returns
//!    `false` for `Some("")` explicitly.

// ---------------------------------------------------------------------------
// Busy allow-lists (`interaction-guard-decision.ts`)
// ---------------------------------------------------------------------------

/// Commands allowed while busy outside any interaction, pending the known-
/// command check (TS: `BUSY_ALLOWED_OUTSIDE_INTERACTION_COMMANDS`).
pub const BUSY_ALLOWED_OUTSIDE_INTERACTION_COMMANDS: &[&str] = &["/reload", "/goal"];

/// Static busy allow-list (TS: `BUSY_ALLOWED_COMMANDS`).
const BUSY_ALLOWED_COMMANDS: &[&str] = &["/abort", "/detach", "/status", "/help", "/opencode_stop"];

/// Static busy allow-list plus the local-registry fallback
/// (TS: `isBusyAllowedCommand`: `/abort`, `/detach`, `/status`, `/help`,
/// `/opencode_stop`, or `localCommandRegistry?.allowsWhenBusy(command)`).
pub fn is_busy_allowed_with_registry(
    command: Option<&str>,
    registry_allows_when_busy: bool,
) -> bool {
    match command {
        Some(c) if !c.is_empty() => BUSY_ALLOWED_COMMANDS.contains(&c) || registry_allows_when_busy,
        _ => false,
    }
}

/// Outside an interaction, `/reload` and `/goal` additionally pass the busy
/// gate when known (TS: `isBusyAllowedOutsideInteraction`: set membership plus
/// `isKnownCommand(command.slice(1))`).
pub fn is_busy_allowed_outside_interaction(command: Option<&str>, is_known: bool) -> bool {
    match command {
        Some(c) if !c.is_empty() => {
            BUSY_ALLOWED_OUTSIDE_INTERACTION_COMMANDS.contains(&c) && is_known
        }
        _ => false,
    }
}

/// A live interaction plus a registry-known local command busy-blocks
/// (TS: `if (state && localCommandRegistry?.has(command))` in the busy
/// command branch).
pub fn should_busy_block_local_command(has_state: bool, registry_has: bool) -> bool {
    has_state && registry_has
}

// ---------------------------------------------------------------------------
// Goal-label branches (`interaction-guard-decision.ts`, `goal-button.ts`)
// ---------------------------------------------------------------------------

/// Goal reply-button labels match `^(?:◎|⏸|▶|×)\s[\s\S]+$`
/// (TS: `GOAL_BUTTON_TEXT_PATTERN`).
pub fn is_goal_button_label(text: &str) -> bool {
    let mut chars = text.chars();
    if !matches!(chars.next(), Some('◎' | '⏸' | '▶' | '×')) {
        return false;
    }
    if !matches!(chars.next(), Some(c) if c.is_whitespace()) {
        return false;
    }
    // `[\s\S]+` (no `m` flag, `$`-anchored): at least one trailing char.
    chars.next().is_some()
}

/// While busy, goal labels starting with `◎ ` or `⏸ ` bypass the block
/// (TS: `ctx.message.text.startsWith("◎ ") || ...startsWith("⏸ ")`).
pub fn goal_label_has_busy_bypass_prefix(text: &str) -> bool {
    text.starts_with("◎ ") || text.starts_with("⏸ ")
}

/// Verdict for a goal-button label press (TS: the `goalLabel` head of
/// `resolveInteractionGuardDecision`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalLabelBusyDecision {
    /// Live interaction: block with `expected_callback` (not busy-flagged).
    BlockExpectedCallback,
    /// No interaction and (`!busy` or bypass prefix): allow.
    Allow,
    /// No interaction, busy, no bypass prefix: busy-block `expected_text`.
    BlockBusyExpectedText,
}

/// `has_state` is `interactionManager.getSnapshot() != null`; `is_busy` folds
/// `foregroundSessionState.isBusy() || attachManager.isBusy()`.
pub fn decide_goal_label_while_busy(
    has_state: bool,
    is_busy: bool,
    bypass_prefix: bool,
) -> GoalLabelBusyDecision {
    if has_state {
        GoalLabelBusyDecision::BlockExpectedCallback
    } else if !is_busy || bypass_prefix {
        GoalLabelBusyDecision::Allow
    } else {
        GoalLabelBusyDecision::BlockBusyExpectedText
    }
}

/// The goal-flow shortcut allows the press without consulting the slot
/// (TS: `state?.metadata.flow === "goal" && (data === "goal:cancel" ||
/// data === "goal:confirm")`).
pub fn is_goal_flow_shortcut(flow: Option<&str>, callback_data: Option<&str>) -> bool {
    flow == Some("goal") && matches!(callback_data, Some("goal:cancel" | "goal:confirm"))
}

// ---------------------------------------------------------------------------
// Callback allowances (`interaction-guard-decision.ts`)
// ---------------------------------------------------------------------------

/// `rename:cancel` arrives while a rename interaction expects text
/// (TS: `isAllowedRenameCancelCallback`).
pub fn is_allowed_rename_cancel(
    kind: Option<&str>,
    expected_input: Option<&str>,
    callback_data: Option<&str>,
) -> bool {
    kind == Some("rename")
        && expected_input == Some("text")
        && callback_data == Some("rename:cancel")
}

/// `task:cancel` / `task:retry-schedule` arrive during a task interaction
/// (TS: `isAllowedTaskCallback`; values from `scheduled-task-menu.ts`).
pub fn is_allowed_task_callback(kind: Option<&str>, callback_data: Option<&str>) -> bool {
    kind == Some("task") && matches!(callback_data, Some("task:cancel" | "task:retry-schedule"))
}

// ---------------------------------------------------------------------------
// Queued-prompt button (`interaction-guard-decision.ts`)
// ---------------------------------------------------------------------------

/// `❌ <n>. <text>` presses pass the busy gate like `/abort`
/// (TS: `isQueuedPromptButtonPress` over
/// `QUEUED_PROMPT_BUTTON_TEXT_PATTERN` = `/^❌\s\d+\.\s/`).
pub fn is_queued_prompt_button_press(text: &str) -> bool {
    let mut rest = match text.strip_prefix('❌') {
        Some(rest) => rest,
        None => return false,
    };
    // Single `\s` after the marker.
    let ws_len = match rest.chars().next() {
        Some(c) if c.is_whitespace() => c.len_utf8(),
        _ => return false,
    };
    rest = &rest[ws_len..];
    // `\d+`: one or more ASCII digits (JS `\d` is ASCII-only).
    let digits_len: usize = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .map(|c| c.len_utf8())
        .sum();
    if digits_len == 0 {
        return false;
    }
    rest = &rest[digits_len..];
    // Literal `.` followed by a single `\s` (prefix pattern, no `$` anchor).
    let mut chars = rest.chars();
    if chars.next() != Some('.') {
        return false;
    }
    matches!(chars.next(), Some(c) if c.is_whitespace())
}

// ---------------------------------------------------------------------------
// Mixed input (`interaction-guard-decision.ts`)
// ---------------------------------------------------------------------------

/// `mixed` expected input admits callbacks and text, nothing else
/// (TS: the `state.expectedInput === "mixed"` branches).
pub fn is_mixed_allowed_input(is_callback: bool, is_text: bool) -> bool {
    is_callback || is_text
}

// ---------------------------------------------------------------------------
// Busy-blocked reply channel (`busy-blocked-renderer.ts`)
// ---------------------------------------------------------------------------

/// Where the session-busy notice goes (TS: `replyBusyBlocked`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusyBlockedChannel {
    /// A callback query is present: `answerCallbackQuery`.
    AnswerCallback,
    /// No callback query but a chat is known: `reply`.
    Reply,
    /// Neither: stay silent.
    Silent,
}

/// `has_callback_query` is `ctx.callbackQuery != null`; `has_chat` is
/// `ctx.chat != null`.
pub fn decide_busy_blocked_channel(has_callback_query: bool, has_chat: bool) -> BusyBlockedChannel {
    if has_callback_query {
        BusyBlockedChannel::AnswerCallback
    } else if has_chat {
        BusyBlockedChannel::Reply
    } else {
        BusyBlockedChannel::Silent
    }
}
