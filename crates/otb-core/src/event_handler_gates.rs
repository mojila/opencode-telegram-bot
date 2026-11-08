//! Event-handler dispatch gates (slice 101, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/events/handlers/handler-context.ts` (`SUBAGENT_STREAM_PREFIX`
//!   `"🧩"`, `SESSION_MESSAGE_MAX_LENGTH` 3500, `getReplyKeyboard`
//!   initialized gate, `formatSessionMessage` trim / `common.unknown_error`
//!   fallback / over-limit `slice(0, MAX - 3) + "..."` truncation)
//! - `src/bot/events/handlers/run-close.ts` (`closeForegroundRun` footer
//!   gate: `!getShowAssistantRunFooter() || !completedRun?.hasCompletedResponse
//!   || completedRun.suppressFooter` skip, then missing `agent` /
//!   `providerID` / `modelID` skip)
//! - `src/bot/events/handlers/dashboard-handler.ts` (tokens handler
//!   `!isInitialized` skip and `!isCompleted && contextSize === 0` zero
//!   intermediate skip, `contextLimit > 0` keyboard-update gate,
//!   cost/compacted/diff/file-change `!isInitialized` guards,
//!   `isCompactProgressMode()` diff/file-change compact routing)
//! - `src/bot/events/handlers/goal-handler.ts` (snapshot
//!   `!isInitialized` skip, notice `!destination ||
//!   !isForegroundSession` skip)
//! - `src/bot/events/handlers/interaction-handler.ts`
//!   (`getFollowedSessionId` subagent-to-root gate, `generation !== null &&
//!   generation !== getGeneration()` waiting-poll drop, in-flight `ending`
//!   drop, `!isForegroundSession` skip, `!startQuestions(...)` wait-branch,
//!   replacing-poll `isActive && sameSession` gate, question-error
//!   active/session/settling-from-Telegram dispatch, settled
//!   active/request/settling/destination dispatch with the cancelled +
//!   `hasLastCancelFailed` branch, permission `!destination` /
//!   non-foreground skip, `getDropReason` stale drop, question-kind wait,
//!   waiting-request `kind === "question"` dispatch, session-run-ended
//!   `changes.length === 0` skip)
//! - `src/bot/events/handlers/assistant-response-handler.ts` inbox-pickup
//!   subset (`delivery === "queue" && hasRun` run-close branch,
//!   `!hasBotRun && session?.id === sessionId` inbox-run start branch,
//!   `!buildExternalUserInputNotification(...)` echo skip)
//!
//! Already ported elsewhere and NOT re-tested here:
//! - assistant/thinking text rendering (`assistant-rendering`,
//!   `thinking-rendering`, `thinking-message` owners)
//! - streaming internals (`response-streamer`, `compact-progress-streamer`,
//!   `running-tool-tracker`, `stream-throttle`, `tool-call-streamer`)
//! - poll / permission prompt menus (`question-menu`, `permission-menu`)
//! - `summaryAggregator` subscription plumbing itself
//!   (`event_delivery_gates`); this slice owns only the dispatch predicates.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `Api`/`ctx` I/O, `AppContainer` services (`keyboardManager`,
//!   `pinnedMessageManager`, `assistantRunState`, `interactionManager`,
//!   `questionManager`, `permissionManager`, `summaryAggregator`,
//!   `goalService`), `SessionRuntimeState` queues/streamers,
//!   `policy.getDestination` / `isForegroundSession` /
//!   `isSubagentSession` / `getRootSessionId` snapshots (arrive as explicit
//!   bools/ids), `Date.now()` clocks, `settings-store` reads
//!   (`getShowAssistantRunFooter`, `getCompactOutputMode`,
//!   `getDeleteCompactProgressOnFinish` arrive as explicit bools),
//!   `t(...)` lookups (the unknown-error text arrives as an argument),
//!   `sendGoalText` / `sendText` / `formatAssistantRunFooter` sends.
//!
//! Deviations: none intended. `String.length`/`slice` parity fixtures use
//! ASCII-only inputs where JS UTF-16 code units and Rust `char`s agree.
//! `generation` arrives as `Option<u64>` (`None` = the live `null` path, no
//! drop); `contextSize` arrives precomputed as `input + cacheRead`.
//!
//! The unknown-error text arrives as an explicit argument because `t()`
//! stays with the caller (`formatSessionMessage` otherwise reads
//! `t("common.unknown_error")` itself).

/// Max chars for an error/retry text in one Telegram message
/// (TS: `SESSION_MESSAGE_MAX_LENGTH`).
pub const SESSION_MESSAGE_MAX_LENGTH: usize = 3500;

/// Prefix marking a subagent stream (TS: `SUBAGENT_STREAM_PREFIX`).
pub const SUBAGENT_STREAM_PREFIX: &str = "🧩";

/// i18n key used when the session message is blank
/// (TS: `t("common.unknown_error")`).
pub const SESSION_MESSAGE_UNKNOWN_ERROR_KEY: &str = "common.unknown_error";

/// Suffix marking a truncated session message (TS: `+ "..."`).
pub const SESSION_MESSAGE_TRUNCATION_SUFFIX: &str = "...";

/// What a question-error event asks the chat to do (TS: the three exits in
/// `setOnQuestionError`: drop the waiting queue, leave the settling poll
/// alone, or clear the active poll and delete its messages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionErrorAction {
    DropWaiting,
    IgnoreSettling,
    ClearAndDelete,
}

/// Trimmed message, the unknown-error fallback when blank, truncated past
/// the budget (TS: `formatSessionMessage`).
pub fn format_session_message(message: &str, unknown_error_text: &str) -> String {
    let trimmed = message.trim();
    let normalized = if trimmed.is_empty() {
        unknown_error_text
    } else {
        trimmed
    };
    if normalized.chars().count() > SESSION_MESSAGE_MAX_LENGTH {
        let kept: String = normalized
            .chars()
            .take(SESSION_MESSAGE_MAX_LENGTH - SESSION_MESSAGE_TRUNCATION_SUFFIX.len())
            .collect();
        format!("{kept}{SESSION_MESSAGE_TRUNCATION_SUFFIX}")
    } else {
        normalized.to_string()
    }
}

/// `true` when the reply keyboard is available (TS: `getReplyKeyboard`:
/// `!isInitialized()` yields `undefined`).
pub fn should_return_reply_keyboard(is_initialized: bool) -> bool {
    is_initialized
}

/// Session the chat follows for a request: a subagent's requests belong to
/// its root (TS: `getFollowedSessionId`).
pub fn resolve_followed_session<'a>(
    is_subagent: bool,
    session_id: &'a str,
    root_id: &'a str,
) -> &'a str {
    if is_subagent {
        root_id
    } else {
        session_id
    }
}

/// `true` when the run close sends its footer (TS: `closeForegroundRun`:
/// footer on, completed response present, not suppressed, and the agent,
/// provider and model identities all known).
pub fn should_send_run_footer(
    show_footer: bool,
    has_completed_response: bool,
    suppress_footer: bool,
    has_agent: bool,
    has_provider: bool,
    has_model: bool,
) -> bool {
    show_footer
        && has_completed_response
        && !suppress_footer
        && has_agent
        && has_provider
        && has_model
}

/// `true` when a dashboard token update is handled at all (TS: the
/// `!pinnedMessageManager.isInitialized()` early return).
pub fn should_handle_dashboard_update(is_initialized: bool) -> bool {
    is_initialized
}

/// `true` when a token update is skipped as a zero intermediate (TS:
/// `if (!isCompleted && contextSize === 0) return;`).
pub fn should_skip_zero_token_update(is_completed: bool, context_size: u64) -> bool {
    !is_completed && context_size == 0
}

/// `true` when the keyboard context is refreshed (TS:
/// `if (contextLimit > 0) updateContext(...)`).
pub fn should_update_keyboard_context(context_limit: u64) -> bool {
    context_limit > 0
}

/// `true` when a session diff / file change also feeds the compact card
/// (TS: `if (isCompactProgressMode()) ...` in the diff and file-change
/// handlers).
pub fn should_route_session_diff_to_compact(is_compact: bool) -> bool {
    is_compact
}

/// `true` when a goal snapshot refreshes the dashboard (TS:
/// `if (!pinnedMessageManager.isInitialized()) return;`).
pub fn should_refresh_goal_snapshot(is_initialized: bool) -> bool {
    is_initialized
}

/// `true` when a goal notice is sent (TS: `if (!destination ||
/// !isForegroundSession(sessionId)) return;`).
pub fn should_send_goal_notice(has_destination: bool, is_foreground: bool) -> bool {
    has_destination && is_foreground
}

/// `true` when a released poll is dropped after a reset (TS:
/// `generation !== null && generation !== getGeneration()`).
pub fn should_drop_waiting_poll(generation: Option<u64>, current_generation: u64) -> bool {
    matches!(generation, Some(g) if g != current_generation)
}

/// `true` when a poll is dropped because its question ended while its
/// messages were still on their way (TS: `getInFlightEnding(requestID)`
/// truthy branch).
pub fn should_drop_poll_with_ending(has_ending: bool) -> bool {
    has_ending
}

/// `true` when a question waits again instead of opening (TS:
/// `if (!startQuestions(...)) waitQuestion(...)`).
pub fn should_wait_question_after_start(started: bool) -> bool {
    !started
}

/// `true` when the new poll replaces the active poll of the same session
/// (TS: `isActive() && getSessionId() === sessionId`).
pub fn is_replacing_poll(is_active: bool, same_session: bool) -> bool {
    is_active && same_session
}

/// Dispatch for a question-tool failure (TS: `setOnQuestionError`:
/// inactive-or-other-session drops the waiting queue, settling from
/// Telegram is left alone, otherwise the active poll is cleared).
pub fn resolve_question_error_action(
    is_active: bool,
    session_matches: bool,
    settling_from_telegram: bool,
) -> QuestionErrorAction {
    if !is_active || !session_matches {
        QuestionErrorAction::DropWaiting
    } else if settling_from_telegram {
        QuestionErrorAction::IgnoreSettling
    } else {
        QuestionErrorAction::ClearAndDelete
    }
}

/// `true` when a settled question drops its waiting entry instead of
/// closing the screen poll (TS: `!isActive() || getRequestID() !==
/// requestID` in `setOnQuestionSettled`).
pub fn should_drop_waiting_settled(is_active: bool, request_matches: bool) -> bool {
    !is_active || !request_matches
}

/// `true` when a settled-while-sending poll is left to its in-flight
/// answers (TS: `isSettlingFromTelegram()` branch with
/// `noteSettledWhileSending`).
pub fn should_note_settled_while_sending(settling_from_telegram: bool) -> bool {
    settling_from_telegram
}

/// `true` when a cancelled outcome closes as a Telegram cancel (TS:
/// `outcome === "cancelled" && hasLastCancelFailed()` branch).
pub fn should_close_cancelled_as_cancelled(
    outcome_is_cancelled: bool,
    last_cancel_failed: bool,
) -> bool {
    outcome_is_cancelled && last_cancel_failed
}

/// `true` when a permission request is dropped as stale or resolved (TS:
/// `getDropReason(request, generation)` truthy branch).
pub fn should_drop_stale_permission(has_drop_reason: bool) -> bool {
    has_drop_reason
}

/// `true` when a permission waits behind the question on screen (TS:
/// `getSnapshot()?.kind === "question"` branch).
pub fn should_wait_permission_for_question(snapshot_is_question: bool) -> bool {
    snapshot_is_question
}

/// `true` when a released waiting request is a question (TS:
/// `request.kind === "question"` in `setOnWaitingRequestReady`).
pub fn is_waiting_question_request(kind_is_question: bool) -> bool {
    kind_is_question
}

/// `true` when a session-run end closes prompts in the chat (TS:
/// `if (changes.length === 0) return;` in `setOnSessionRunEnded`).
pub fn should_apply_session_run_ended_changes(change_count: usize) -> bool {
    change_count > 0
}

/// `true` when an inbox pickup first closes the queue-delivered run (TS:
/// `item.inbox?.delivery === "queue" && hasRun(sessionId)` in
/// `pickUpInboxPrompt`).
pub fn should_close_run_on_inbox_pickup(delivery_is_queue: bool, has_run: bool) -> bool {
    delivery_is_queue && has_run
}

/// `true` when an inbox pickup starts its own run (TS:
/// `!hasBotRun(sessionId)` plus `session?.id === sessionId`).
pub fn should_start_inbox_run(has_bot_run: bool, is_current_session: bool) -> bool {
    !has_bot_run && is_current_session
}

/// `true` when an inbox pickup echoes its quote (TS:
/// `if (!buildExternalUserInputNotification(...)) return;`).
pub fn should_echo_inbox_prompt(has_notification: bool) -> bool {
    has_notification
}
