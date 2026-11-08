//! Goal callback-handler pure gates (slice 93, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/commands/goal-command.ts`
//!   (`goalConfirmData` / `goalCancelData` exact data, `goalDeleteData`
//!   `goal:delete:<token>` build)
//! - `src/bot/callbacks/goal-callback-handler.ts`
//!   (`handleGoalCallback` dispatch: the `goal:delete:` prefix route,
//!   the confirm/cancel exact-match route, the goal-flow cancel-valid
//!   gate on `metadata.flow === "goal"` with matching `messageId`;
//!   `readConfirmation` (`custom` kind + `goal` flow + `confirm` stage +
//!   numeric `messageId` + string `sessionId` / `directory` /
//!   `objective` / `controlToken` + `start` / `resume` action,
//!   nullable string `goalId`); `callbackMessageId` (message with
//!   numeric `message_id`); the foreground-busy / session-mismatch
//!   invalidation gate; the stale-confirmation gate (`stillActive`
//!   + `messageId` + session + directory + control-token + goal-id
//!   equality + idle); `matchesShownGoal` (`start` needs `none`,
//!   `resume` needs `goal` + id equality + JSON equality + `paused`
//!   status); the `resume` control-claim gate; `handleGoalAction`
//!   status gates (`pause` needs `active`, `resume` needs `paused`,
//!   `clear` needs non-`active`)).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `router_middleware_gates::handles_goal_delete_prefix`,
//!   `is_goal_confirm_data`, `is_goal_cancel_data`,
//!   `is_goal_cancel_valid` (the prefix/exact routes behind the dispatch
//!   below; this slice owns the canonical `goal:confirm` / `goal:cancel` /
//!   `goal:delete:` values and the full confirmation reader).
//! - `goal_manager_gates::GoalStatus`, `GoalState` (the goal snapshot the
//!   staleness and `matchesShownGoal` gates read; this slice takes their
//!   outcomes as explicit bools/enums).
//! - `goal_reason_gates` (`formatGoalConfirmation`, `formatGoalStatusText`),
//!   `goal_service_gates` (`read` / `command` / `refresh` kinds).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, transitions, `clear`).
//! - `goalService.read` / `command` / `refresh`, `goalManager.snapshot` /
//!   `getControlToken` / `claimControl`, `resolveProjectAgent` /
//!   `getStoredModel`, `getCurrentSession`, `isForegroundBusy`,
//!   `formatGoalStatusText` / `replyGoalText` sends.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `messageId` values are `i64` (`typeof ===
//! "number"` in TS), `token` (a TS `Symbol`) is modelled as structural
//! equality of the carried strings/bools, and `goalId` is `Option<String>`
//! (`string` or missing/`null` in TS). `shownGoal` JSON equality arrives as
//! an explicit bool (the TS `JSON.stringify` comparison stays with the
//! caller). An empty `goal:delete:` rest reads as `Some("")` (TS slices
//! the prefix without an emptiness check).

/// Exact confirm callback data (TS: `goalConfirmData()`).
pub const GOAL_CONFIRM_CALLBACK: &str = "goal:confirm";
/// Exact cancel callback data (TS: `goalCancelData()`).
pub const GOAL_CANCEL_CALLBACK: &str = "goal:cancel";
/// Delete callback prefix (TS: the `"goal:delete:"` literal).
pub const GOAL_DELETE_PREFIX: &str = "goal:delete:";

/// Goal confirmation action (TS: `GoalConfirmation["action"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalAction {
    Start,
    Resume,
}

/// Flattened interaction snapshot the confirmation reader inspects
/// (TS: `InteractionManager.getSnapshot()?.metadata` with `flow: "goal"`,
/// `stage: "confirm"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalConfirmationInput {
    pub kind: String,
    pub flow: Option<String>,
    pub stage: Option<String>,
    pub message_id: Option<i64>,
    pub session_id: Option<String>,
    pub directory: Option<String>,
    pub goal_id: Option<String>,
    pub shown_goal_json: Option<String>,
    pub action: Option<String>,
    pub objective: Option<String>,
    pub control_token: Option<String>,
}

/// Validated goal confirmation (TS: `GoalConfirmation`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalConfirmation {
    pub message_id: i64,
    pub session_id: String,
    pub directory: String,
    pub goal_id: Option<String>,
    pub shown_goal_json: Option<String>,
    pub action: GoalAction,
    pub objective: String,
    pub control_token: String,
}

/// Goal read kind behind `matchesShownGoal` (TS: `current.kind` from
/// `goalService.read`: `"goal"` / `"none"` / `"unsupported"` / `"error"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalReadKind {
    None,
    Goal,
    Unsupported,
    Error,
}

/// Confirm data builder (TS: `goalConfirmData()`).
pub fn goal_confirm_data() -> String {
    GOAL_CONFIRM_CALLBACK.to_string()
}

/// Cancel data builder (TS: `goalCancelData()`).
pub fn goal_cancel_data() -> String {
    GOAL_CANCEL_CALLBACK.to_string()
}

/// Delete data builder (TS: `goalDeleteData(token)`).
pub fn goal_delete_data(token: &str) -> String {
    format!("{GOAL_DELETE_PREFIX}{token}")
}

/// Goal-callback dispatch guard: confirm, cancel, or anything under
/// `goal:delete:` handles (TS: the `startsWith("goal:delete:")` branch
/// plus the `data !== confirm && data !== cancel` early-`false`).
pub fn handles_goal_callback(data: Option<&str>) -> bool {
    match data {
        Some(d) => {
            d == GOAL_CONFIRM_CALLBACK
                || d == GOAL_CANCEL_CALLBACK
                || d.starts_with(GOAL_DELETE_PREFIX)
        }
        None => false,
    }
}

/// Delete-route match (TS: `data?.startsWith("goal:delete:")`).
pub fn is_goal_delete_callback(data: &str) -> bool {
    data.starts_with(GOAL_DELETE_PREFIX)
}

/// Delete token behind `goal:delete:` (TS:
/// `data.slice("goal:delete:".length)`; empty rest reads as `Some("")`).
pub fn parse_goal_delete_token(data: &str) -> Option<String> {
    data.strip_prefix(GOAL_DELETE_PREFIX)
        .map(|rest| rest.to_string())
}

/// Exact confirm match (TS: `data === goalConfirmData()`).
pub fn is_goal_confirm(data: &str) -> bool {
    data == GOAL_CONFIRM_CALLBACK
}

/// Exact cancel match (TS: `data === goalCancelData()`).
pub fn is_goal_cancel(data: &str) -> bool {
    data == GOAL_CANCEL_CALLBACK
}

/// Goal-flow cancel-valid gate (TS: `data === cancelData &&
/// metadata?.flow === "goal" && metadata.messageId === messageId` clears
/// and cancels the menu).
pub fn is_goal_cancel_valid(
    flow: Option<&str>,
    snapshot_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (flow, snapshot_message_id, callback_message_id) {
        (Some("goal"), Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// Confirmation reader (TS: `readConfirmation`: `custom` kind, `goal`
/// flow, `confirm` stage, numeric `messageId`, string `sessionId` /
/// `directory` / `objective` / `controlToken`, `start` / `resume` action;
/// `goalId` is a string or missing/null).
pub fn parse_goal_confirmation(input: Option<&GoalConfirmationInput>) -> Option<GoalConfirmation> {
    let input = input?;
    if input.kind != "custom" {
        return None;
    }
    if input.flow.as_deref() != Some("goal") {
        return None;
    }
    if input.stage.as_deref() != Some("confirm") {
        return None;
    }
    let message_id = input.message_id?;
    let session_id = input.session_id.clone()?;
    let directory = input.directory.clone()?;
    let objective = input.objective.clone()?;
    let control_token = input.control_token.clone()?;
    let action = match input.action.as_deref() {
        Some("start") => GoalAction::Start,
        Some("resume") => GoalAction::Resume,
        _ => return None,
    };
    Some(GoalConfirmation {
        message_id,
        session_id,
        directory,
        goal_id: input.goal_id.clone(),
        shown_goal_json: input.shown_goal_json.clone(),
        action,
        objective,
        control_token,
    })
}

/// Stale-confirmation mismatch gate (TS: `!confirmation || messageId ===
/// null || confirmation.messageId !== messageId` answers
/// `inactive_callback`).
pub fn is_confirmation_message_mismatch(
    confirmation_message_id: Option<i64>,
    callback_message_id: Option<i64>,
) -> bool {
    match (confirmation_message_id, callback_message_id) {
        (Some(a), Some(b)) => a != b,
        _ => true,
    }
}

/// Callback message-id resolve (TS: `callbackMessageId`: no message or no
/// numeric `message_id` reads as `None`).
pub fn resolve_callback_message_id(
    has_message_with_id: bool,
    message_id: Option<i64>,
) -> Option<i64> {
    if has_message_with_id {
        message_id
    } else {
        None
    }
}

/// Busy/session invalidation gate after the confirmation read (TS:
/// `isForegroundBusy(deps) || getCurrentSession()?.id !==
/// confirmation.sessionId` answers `inactive_callback`).
pub fn should_invalidate_goal_on_session(
    is_busy: bool,
    current_session_id: Option<&str>,
    confirmation_session_id: &str,
) -> bool {
    is_busy || current_session_id != Some(confirmation_session_id)
}

/// Stale-confirmation gate after the `goalService.read` (TS: `!stillActive
/// || messageId mismatch || session/directory/token/goal mismatch ||
/// isForegroundBusy` answers `inactive_callback`).
pub fn is_goal_confirmation_stale(
    still_active: bool,
    message_matches: bool,
    session_matches: bool,
    directory_matches: bool,
    token_matches: bool,
    goal_matches: bool,
    is_busy: bool,
) -> bool {
    !(still_active
        && message_matches
        && session_matches
        && directory_matches
        && token_matches
        && goal_matches
        && !is_busy)
}

/// Shown-goal match gate (TS: `matchesShownGoal`: `start` needs `none`;
/// otherwise `goal` + id equality + `JSON.stringify` equality; `resume`
/// additionally needs `paused` status).
pub fn matches_shown_goal(
    action: GoalAction,
    read_kind: GoalReadKind,
    goal_id_matches: bool,
    json_matches: bool,
    status_is_paused: bool,
) -> bool {
    match action {
        GoalAction::Start => read_kind == GoalReadKind::None,
        GoalAction::Resume => {
            read_kind == GoalReadKind::Goal
                && goal_id_matches
                && json_matches
                && status_is_paused
        }
    }
}

/// Resume claim gate (TS: `action === "resume" &&
/// !claimControl(controlToken)` answers `inactive_callback`).
pub fn should_reject_goal_resume_claim(action: GoalAction, claim_ok: bool) -> bool {
    matches!(action, GoalAction::Resume) && !claim_ok
}

/// Button-action status gate (TS: `pause` needs `active`, `resume` needs
/// `paused`, `clear` rejects `active` in `handleGoalAction`).
pub fn is_goal_action_allowed(action: &str, status: &str) -> bool {
    match action {
        "pause" => status == "active",
        "resume" => status == "paused",
        "clear" => status != "active",
        _ => false,
    }
}
