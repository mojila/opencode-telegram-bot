//! Command-router / middleware / callback-dispatch pure gates (slice 63, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/routers/command-utils.ts` (`extractCommandName` trim/slash/
//!   mention/lowercase gates, `isKnownCommand` membership over the
//!   `start` + `BOT_COMMANDS` set)
//! - `src/bot/routers/command-router.ts` (slash-text `flushPendingPrompt`
//!   gate, `ensureCommandsInitialized` skip/initialize decision,
//!   `localCommandReply` success/empty/timeout/failed mapping with the
//!   `exitCode ?? "unknown"` fallback)
//! - `src/bot/middleware/auth.ts` (allowed-user allow, bot-self ignore,
//!   stranger deny with the hide-commands gate when the chat is known and
//!   is not the authorized user's chat)
//! - `src/bot/middleware/stale-update.ts` (`MAX_MESSAGE_AGE_SECONDS`,
//!   `floor(now/1000) - date` age, `age <= 60` freshness gate)
//! - `src/bot/middleware/unknown-command.ts` (no-text / no-command / known
//!   passthrough, otherwise reply with the first whitespace token)
//! - `src/bot/commands/goal-command.ts` (`goalConfirmData`,
//!   `goalCancelData`, `goalDeleteData` token build)
//! - `src/bot/callbacks/goal-callback-handler.ts` (`goal:delete:` prefix
//!   route, confirm/cancel exact-match route, the goal-flow cancel-valid
//!   gate on `metadata.flow === "goal"` with matching `messageId`)
//! - `src/bot/callbacks/prompt-attachment-callback-handler.ts`
//!   (`ATTACHMENT_CANCEL_CALLBACK` exact-match guard)
//!
//! No grammy/Telegram I/O, no `config`/store reads, no `t(...)` locale
//! lookups live here — only name extraction, routing predicates, and small
//! value transforms. Callers resolve `BOT_COMMANDS`, `ctx` snapshots, and
//! keyboards.
//!
//! DEVIATIONS from TS:
//! 1. `isKnownCommand` closes over the module-level `KNOWN_COMMANDS` set in
//!    TS. The port takes the known names as an explicit `&[&str]` argument;
//!    callers pass `["start", ...BOT_COMMANDS]`.
//! 2. `localCommandReply` renders `t(...)` strings in TS. The port returns
//!    the i18n key (or `None` for `success`, whose raw text is sent) plus
//!    the `exitCode ?? "unknown"` label, so the gate stays pure; callers
//!    render the locale.
//! 3. `ensureCommandsInitialized` and the middlewares act on grammy `ctx`
//!    in TS. The port reduces them to explicit scalar/enum decisions
//!    (`from_id`, `has_chat`, `chat_id`, ...); the middleware keeps the
//!    `ctx.api` calls.
//! 4. `String.split(/\s+/)` in TS splits on Unicode whitespace; the port
//!    uses `char::is_whitespace`, which coincides on parity fixtures.
//! 5. `ctx.chat.id` truthiness in `auth.ts` admits `0` as "no chat" in TS.
//!    Telegram chat ids are never `0`; the port treats any `Some(chat_id)`
//!    as a known chat and documents the (unreachable) difference.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice63_router_middleware_parity.rs`
//! assert the TS contract and MUST FAIL until the Rust port lands (step 2).

// ---------------------------------------------------------------------------
// Command names (`src/bot/routers/command-utils.ts`)
// ---------------------------------------------------------------------------

/// Lowercase `/command` token without `@mention` or args, else `None`
/// (TS: `extractCommandName`).
pub fn extract_command_name(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let token = trimmed.split_whitespace().next()?;
    let without_slash = token.strip_prefix('/')?;
    if without_slash.is_empty() {
        return None;
    }
    let without_mention = without_slash.split('@').next()?.to_lowercase();
    if without_mention.is_empty() {
        return None;
    }
    Some(without_mention)
}

/// Membership in the known-command set (TS: `isKnownCommand`; callers pass
/// `["start", ...BOT_COMMANDS]`).
pub fn is_known_command(name: &str, known: &[&str]) -> bool {
    known.contains(&name)
}

// ---------------------------------------------------------------------------
// Command router (`src/bot/routers/command-router.ts`)
// ---------------------------------------------------------------------------

/// A slash text flushes a pending merged prompt before routing
/// (TS: `ctx.message?.text?.startsWith("/")` in `registerCommandRouter`).
pub fn should_flush_pending_prompt(text: Option<&str>) -> bool {
    text.is_some_and(|t| t.starts_with('/'))
}

/// Whether the chat-scoped command list must be (re)installed
/// (TS: `ensureCommandsInitialized`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureCommandsDecision {
    SkipAlreadyInitialized,
    SkipUnauthorized,
    SkipNoChat,
    Initialize,
}

/// `initialized` is the module flag; `from_id` is `ctx.from?.id`;
/// `has_chat` is `ctx.chat.is_some()`.
pub fn decide_ensure_commands_init(
    initialized: bool,
    from_id: Option<u64>,
    allowed_user_id: u64,
    has_chat: bool,
) -> EnsureCommandsDecision {
    if initialized {
        return EnsureCommandsDecision::SkipAlreadyInitialized;
    }
    if from_id != Some(allowed_user_id) {
        return EnsureCommandsDecision::SkipUnauthorized;
    }
    if !has_chat {
        return EnsureCommandsDecision::SkipNoChat;
    }
    EnsureCommandsDecision::Initialize
}

/// Local-command outcome kinds (TS: `LocalCommandResult["kind"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCommandKind {
    Success,
    Empty,
    Timeout,
    Failed,
}

/// i18n key for a local-command reply, or `None` for `success` whose raw
/// text is sent (TS: `localCommandReply`).
pub fn local_command_reply_key(kind: LocalCommandKind) -> Option<&'static str> {
    match kind {
        LocalCommandKind::Success => None,
        LocalCommandKind::Empty => Some("local_command.empty_output"),
        LocalCommandKind::Timeout => Some("local_command.timeout"),
        LocalCommandKind::Failed => Some("local_command.failed"),
    }
}

/// `exitCode ?? "unknown"` label for the failed-command reply
/// (TS: `result.exitCode ?? "unknown"` in `localCommandReply`).
pub fn failed_exit_code_label(exit_code: Option<i32>) -> String {
    match exit_code {
        Some(code) => code.to_string(),
        None => "unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Auth middleware (`src/bot/middleware/auth.ts`)
// ---------------------------------------------------------------------------

/// Allow / ignore-self / deny verdict for one update
/// (TS: `authMiddleware`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthVerdict {
    Allow,
    IgnoreSelf,
    DenyHideCommands,
    Deny,
}

/// `user_id` is `ctx.from?.id`, `bot_id` is `ctx.me.id`, `chat_id` is
/// `ctx.chat?.id`. Strangers whose chat is known and differs from the
/// authorized user's chat get `DenyHideCommands`.
pub fn decide_auth(
    user_id: Option<u64>,
    allowed_user_id: u64,
    bot_id: u64,
    chat_id: Option<u64>,
) -> AuthVerdict {
    if user_id == Some(allowed_user_id) {
        return AuthVerdict::Allow;
    }
    if user_id == Some(bot_id) {
        return AuthVerdict::IgnoreSelf;
    }
    // Strangers (including a missing user) are denied. A present stranger
    // whose known chat differs from the authorized user's chat additionally
    // gets the hide-commands treatment (TS: `ctx.chat?.id` truthy and
    // `!== allowedUserId` — see module deviations for the `0` note).
    if let (Some(_), Some(chat)) = (user_id, chat_id) {
        if chat != allowed_user_id {
            return AuthVerdict::DenyHideCommands;
        }
    }
    AuthVerdict::Deny
}

// ---------------------------------------------------------------------------
// Stale-update middleware (`src/bot/middleware/stale-update.ts`)
// ---------------------------------------------------------------------------

/// Updates older than this are dropped (TS: `MAX_MESSAGE_AGE_SECONDS`).
pub const MAX_MESSAGE_AGE_SECONDS: i64 = 60;

/// `floor(now_ms / 1000) - message.date` (TS: the `ageSeconds` computation).
pub fn message_age_seconds(now_ms: i64, message_date_secs: i64) -> i64 {
    now_ms.div_euclid(1000) - message_date_secs
}

/// Stale exactly when the age exceeds the budget
/// (TS: `ageSeconds <= MAX_MESSAGE_AGE_SECONDS` stays fresh).
pub fn is_stale_message(age_seconds: i64) -> bool {
    age_seconds > MAX_MESSAGE_AGE_SECONDS
}

// ---------------------------------------------------------------------------
// Unknown-command middleware (`src/bot/middleware/unknown-command.ts`)
// ---------------------------------------------------------------------------

/// Pass the update through, or reply that the command is unknown with the
/// first whitespace token as written (TS: `unknownCommandMiddleware`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownCommandDecision {
    PassThrough,
    ReplyUnknown { token: String },
}

/// `text` is `ctx.message?.text`; `known` is the known-command set.
pub fn decide_unknown_command(text: Option<&str>, known: &[&str]) -> UnknownCommandDecision {
    let text = match text {
        Some(t) => t,
        None => return UnknownCommandDecision::PassThrough,
    };
    let name = match extract_command_name(text) {
        Some(n) => n,
        None => return UnknownCommandDecision::PassThrough,
    };
    if is_known_command(&name, known) {
        return UnknownCommandDecision::PassThrough;
    }
    let token = text
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    UnknownCommandDecision::ReplyUnknown { token }
}

// ---------------------------------------------------------------------------
// Goal callback data (`goal-command.ts`, `goal-callback-handler.ts`)
// ---------------------------------------------------------------------------

/// Goal confirm callback data (TS: `goalConfirmData()`).
pub const GOAL_CONFIRM_DATA: &str = "goal:confirm";

/// Goal cancel callback data (TS: `goalCancelData()`).
pub const GOAL_CANCEL_DATA: &str = "goal:cancel";

/// Goal delete callback prefix (TS: `"goal:delete:"`).
pub const GOAL_DELETE_PREFIX: &str = "goal:delete:";

/// `goal:delete:<token>` callback build (TS: `goalDeleteData`).
pub fn build_goal_delete_data(token: &str) -> String {
    format!("{GOAL_DELETE_PREFIX}{token}")
}

/// Goal callback dispatch (TS: the `handleGoalCallback` head: `goal:delete:`
/// prefix, confirm/cancel exact matches, otherwise unhandled).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalCallbackRoute {
    Delete { token: String },
    Confirm,
    Cancel,
    Other,
}

/// `data` is `ctx.callbackQuery?.data`.
pub fn route_goal_callback(data: Option<&str>) -> GoalCallbackRoute {
    match data {
        Some(d) if d.starts_with(GOAL_DELETE_PREFIX) => GoalCallbackRoute::Delete {
            token: d[GOAL_DELETE_PREFIX.len()..].to_string(),
        },
        Some(GOAL_CONFIRM_DATA) => GoalCallbackRoute::Confirm,
        Some(GOAL_CANCEL_DATA) => GoalCallbackRoute::Cancel,
        _ => GoalCallbackRoute::Other,
    }
}

/// The cancel press belongs to a live goal-input flow with the same message
/// (TS: `data === goalCancelData() && metadata?.flow === "goal" &&
/// metadata.messageId === messageId`).
pub fn is_valid_goal_cancel(
    flow: Option<&str>,
    metadata_message_id: Option<u64>,
    message_id: Option<u64>,
) -> bool {
    // TS compares `metadata.messageId === messageId` where the sides are
    // `number | undefined` vs `number | null`: `None == None` across the two
    // representations is `false`, so both sides must be present and equal.
    flow == Some("goal") && metadata_message_id.is_some() && metadata_message_id == message_id
}

// ---------------------------------------------------------------------------
// Prompt-attachment cancel
// (`src/bot/callbacks/prompt-attachment-callback-handler.ts`)
// ---------------------------------------------------------------------------

/// Prompt-attachment cancel callback (TS: `ATTACHMENT_CANCEL_CALLBACK`).
pub const ATTACHMENT_CANCEL_CALLBACK: &str = "attach:cancel";

/// Exact-match guard (TS: `ctx.callbackQuery?.data !== ...` early return).
pub fn is_attachment_cancel_callback(data: Option<&str>) -> bool {
    data == Some(ATTACHMENT_CANCEL_CALLBACK)
}
