//! Goal-service pure gates (slice 72, RED).
//!
//! Rust port of the pure remainder in
//! `src/app/services/goal-service.ts` (the network/client calls,
//! `GoalManager` wiring, listeners, and `t(...)`/`localizeGoalReason`
//! lookups stay with the caller):
//! - `parseGoalState` (record/shape validation; optional non-empty
//!   `reason`)
//! - `isUnsupported` (`GoalUnsupportedError` name or, for `"rpc"` reads,
//!   `rpc.method_not_found` / `rpc.unavailable` types)
//! - `messageOf` (`Error` message, string itself, record `data.message` /
//!   `message` / `name`, unavailable-marker fallback to `"fetch failed"`)
//! - `readFailure` (unsupported vs error split)
//! - `available` (`opencodeServerVersion === "v2"`)
//! - `refresh` guards (wrong-session and control-token races keep the last
//!   snapshot instead of applying)
//! - `applyChanged` guards (wrong session ignored; `null` clears;
//!   unparseable non-null ignored)
//! - `command` error mapping (unsupported vs rejected-with-message vs
//!   error-needs-fallback; localization stays with the caller)
//!
//! No `opencodeV2Client`, `logger`, `GoalManager`, or `t(...)` lookups live
//! here — only validation, classification, and decision gates.
//!
//! DEVIATIONS from TS (behavior-preserving for the covered cases):
//! 1. TS takes `unknown` (`isRecord` probes + `instanceof Error`). The port
//!    models it as [`JsonVal`] (for goal payloads) and [`FailVal`] (for
//!    errors); `undefined`/functions/symbols have no counterpart.
//! 2. TS `number` maps to `f64`; any finite or non-finite number passes the
//!    `typeof` check the same way. Values are stored as `f64`.
//! 3. `messageOf`'s `isExpectedOpencodeUnavailableError(error)` fallback
//!    (which stringifies the whole error) is approximated by matching the
//!    same marker substrings against the carried message text; callers pass
//!    the already-extracted text via [`FailVal`].
//! 4. `localizeGoalReason(...)` wrapping of rejected/error messages stays
//!    with the caller: [`map_command_error`] returns the raw message and
//!    [`CommandErrorDecision::ErrorNeedsFallback`] when TS would localize an
//!    empty message.
//! 5. `available()` reads the mutable `opencodeServerVersion` global in TS;
//!    the port takes the version as `&str` ([`is_available`]) so it stays
//!    pure and testable.
//! 6. `message_of` follows the parity-test contract (which matches
//!    `extractErrorMessage` in `src/utils/opencode-error.ts`: string itself,
//!    record `data.message` first, then `message`, then `name`): the local
//!    `messageOf` in `goal-service.ts` instead returns `""` for strings, reads
//!    `message` before `data.message`, and has no `name` arm. The tests are
//!    authoritative, so the port implements the test order.
//! 7. The local `messageOf`'s `isExpectedOpencodeUnavailableError` fallback
//!    (`"fetch failed"`) is unreachable on plain values in TS (`String(record)`
//!    is `"[object Object]"`, and `Error` instances return via the first
//!    branch), so the port needs no extra marker scan: `ErrorInstance`/`Text`
//!    identity and direct field passthrough already yield `"fetch failed"`
//!    when that is the carried text.
//! 8. The `GoalSelectionError` branch of `command` (an `error`-kind result
//!    with the raw `cause` message, no localization) has no test case and
//!    stays with the caller; [`map_command_error`] covers only the
//!    unsupported / rejected-with-message / error-needs-fallback split.

/// Goal lifecycle status (TS: `GoalStatus` in `src/app/types/goal.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatusView {
    Active,
    Paused,
    Complete,
}

/// Parsed server goal (TS: `GoalState` in `src/app/types/goal.ts`).
#[derive(Debug, Clone, PartialEq)]
pub struct GoalStateView {
    pub id: String,
    pub objective: String,
    pub status: GoalStatusView,
    pub reason: Option<String>,
    pub rounds: f64,
    pub max_rounds: f64,
    pub updated_at: f64,
}

/// Dynamic JSON-like value for goal payloads (TS: `unknown` probed with
/// `isRecord` and `typeof` reads in `parseGoalState`).
#[derive(Debug, Clone, PartialEq)]
pub enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Obj(Vec<(String, JsonVal)>),
    Arr(Vec<JsonVal>),
}

/// Dynamic error value for failure classification (TS: `unknown` probed
/// with `instanceof Error`, `typeof`, and `isRecord` field reads in
/// `isUnsupported` / `messageOf`).
#[derive(Debug, Clone, PartialEq)]
pub enum FailVal {
    Null,
    Bool(bool),
    Num(f64),
    Text(String),
    ErrorInstance { message: String },
    Record {
        name: Option<String>,
        error_type: Option<String>,
        message: Option<String>,
        data_message: Option<String>,
    },
}

/// Which call site asks about support (TS: `source` param of
/// `isUnsupported`, default `"rpc"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedSource {
    Rpc,
    Command,
}

/// `readFailure` outcome (TS: `{ kind: "unsupported" }` vs
/// `{ kind: "error", message }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadFailure {
    Unsupported,
    Error { message: String },
}

/// `refresh` application guard (TS: early returns in `GoalService.refresh`
/// that keep the previous snapshot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshGuard {
    /// Followed session and control token both match: apply the read.
    Proceed,
    /// A different session is followed now: keep the last snapshot.
    WrongSession,
    /// The control token moved during the read: keep the last snapshot.
    StaleToken,
}

/// `applyChanged` routing (TS: guards at the top of
/// `GoalService.applyChanged`).
#[derive(Debug, Clone, PartialEq)]
pub enum ApplyChangedDecision {
    /// Followed session differs, or a non-null payload with an unexpected
    /// shape: ignore the change.
    Ignore,
    /// Payload is `null`: clear the goal.
    Clear,
    /// Parsed payload to announce once.
    Apply(GoalStateView),
}

/// `command` failure mapping (TS: branches after a failed
/// `goal.command`; the caller localizes messages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandErrorDecision {
    /// Server lacks goal support.
    Unsupported,
    /// Caller shows the localized message.
    Rejected { message: String },
    /// No message to show: caller falls back to the localized
    /// stopped-after-error line.
    ErrorNeedsFallback,
}

/// Validates a raw goal payload (TS: `parseGoalState`).
pub fn parse_goal_state(value: &JsonVal) -> Option<GoalStateView> {
    let JsonVal::Obj(fields) = value else {
        return None;
    };
    let get = |key: &str| fields.iter().find(|(k, _)| k == key).map(|(_, v)| v);
    let id = match get("id") {
        Some(JsonVal::Str(s)) => s.clone(),
        _ => return None,
    };
    let objective = match get("objective") {
        Some(JsonVal::Str(s)) => s.clone(),
        _ => return None,
    };
    let status = match get("status") {
        Some(JsonVal::Str(s)) => match s.as_str() {
            "active" => GoalStatusView::Active,
            "paused" => GoalStatusView::Paused,
            "complete" => GoalStatusView::Complete,
            _ => return None,
        },
        _ => return None,
    };
    let rounds = match get("rounds") {
        Some(JsonVal::Num(n)) => *n,
        _ => return None,
    };
    let max_rounds = match get("maxRounds") {
        Some(JsonVal::Num(n)) => *n,
        _ => return None,
    };
    let updated_at = match get("updatedAt") {
        Some(JsonVal::Num(n)) => *n,
        _ => return None,
    };
    let reason = match get("reason") {
        Some(JsonVal::Str(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    };
    Some(GoalStateView {
        id,
        objective,
        status,
        reason,
        rounds,
        max_rounds,
        updated_at,
    })
}

/// Whether the failure means the server lacks goal support
/// (TS: `isUnsupported`).
pub fn is_unsupported(error: &FailVal, source: UnsupportedSource) -> bool {
    let FailVal::Record {
        name, error_type, ..
    } = error
    else {
        return false;
    };
    if name.as_deref() == Some("GoalUnsupportedError") {
        return true;
    }
    source == UnsupportedSource::Rpc
        && matches!(
            error_type.as_deref(),
            Some("rpc.method_not_found" | "rpc.unavailable")
        )
}

/// Human-readable message from an unknown error value
/// (TS: `messageOf`, with the parity-test order: `data.message` first, then
/// `message`, then `name`; see deviation 6).
pub fn message_of(error: &FailVal) -> String {
    match error {
        FailVal::ErrorInstance { message } => message.clone(),
        FailVal::Text(s) => s.clone(),
        FailVal::Record {
            name,
            message,
            data_message,
            ..
        } => {
            if let Some(inner) = data_message {
                return inner.clone();
            }
            if let Some(outer) = message {
                return outer.clone();
            }
            if let Some(n) = name {
                return n.clone();
            }
            String::new()
        }
        FailVal::Null | FailVal::Bool(_) | FailVal::Num(_) => String::new(),
    }
}

/// Splits a read failure into unsupported vs error (TS: `readFailure`).
pub fn classify_read_failure(error: &FailVal) -> ReadFailure {
    if is_unsupported(error, UnsupportedSource::Rpc) {
        ReadFailure::Unsupported
    } else {
        ReadFailure::Error {
            message: message_of(error),
        }
    }
}

/// Whether the goal backend can be used (TS: `available`: the
/// `opencodeServerVersion === "v2"` check).
pub fn is_available(server_version: &str) -> bool {
    server_version == "v2"
}

/// Guards a `refresh` application after the read resolves
/// (TS: session/token races in `GoalService.refresh`).
pub fn refresh_guard(
    current_session: Option<&str>,
    session_id: &str,
    token_before: &str,
    token_now: &str,
) -> RefreshGuard {
    if current_session != Some(session_id) {
        return RefreshGuard::WrongSession;
    }
    if token_before != token_now {
        return RefreshGuard::StaleToken;
    }
    RefreshGuard::Proceed
}

/// Routes an incoming goal-changed payload (TS: `GoalService.applyChanged`
/// guards; parsing via [`parse_goal_state`]).
pub fn decide_apply_changed(
    current_session: Option<&str>,
    session_id: &str,
    goal: Option<&JsonVal>,
) -> ApplyChangedDecision {
    if current_session != Some(session_id) {
        return ApplyChangedDecision::Ignore;
    }
    match goal {
        None => ApplyChangedDecision::Clear,
        Some(value) => match parse_goal_state(value) {
            Some(parsed) => ApplyChangedDecision::Apply(parsed),
            None => ApplyChangedDecision::Ignore,
        },
    }
}

/// Maps a failed `goal.command` to its outcome (TS: `command` error
/// branches; `GoalSelectionError` logging and localization stay with the
/// caller, which passes the already-extracted message).
pub fn map_command_error(
    is_unsupported_error: bool,
    message: &str,
) -> CommandErrorDecision {
    if is_unsupported_error {
        return CommandErrorDecision::Unsupported;
    }
    if !message.is_empty() {
        return CommandErrorDecision::Rejected {
            message: message.to_string(),
        };
    }
    CommandErrorDecision::ErrorNeedsFallback
}
