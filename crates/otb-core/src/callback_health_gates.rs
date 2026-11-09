//! Pure callback / health / retry gates (slice 31b, RED).
//!
//! Rust port of the pure gates in:
//! - `src/bot/callbacks/callback-router.ts` (`parseCallbackPrefix`, the
//!   prefix -> error-scope route table)
//! - `src/bot/callbacks/feedback.ts` (`CALLBACK_ANSWER_MAX_LENGTH`,
//!   answer truncation)
//! - `src/bot/callbacks/permission-callback-handler.ts`
//!   (`isPermissionReply`, `isPermissionRequestNotFound`, action split)
//! - `src/bot/callbacks/question-callback-handler.ts` (callback action
//!   parse, `isLastQuestion`)
//! - `src/bot/callbacks/agent-selection-callback-handler.ts` (and the
//!   model/variant handlers: `prefix:` name strip)
//! - `src/opencode/server-health.ts` (`reportOpencodeProblem` once/always
//!   dedupe, `classifyFailedHealthCheck` cause mapping)
//! - `src/utils/telegram-rate-limit-retry.ts` (429 detection, transient
//!   server/network code sets, exponential backoff with the 8000ms cap)
//! - `src/utils/opencode-error.ts` (`isExpectedOpencodeUnavailableError`,
//!   `isOpencodeNotFoundError`, `isServerUnavailableError` markers,
//!   `extractErrorMessage` fallback chain)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice31b_callback_health_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2).
//!
//! No grammy/Telegram I/O, no network/fetch, no timers live here — only
//! pure predicates, classifiers, and small value transforms.

// ---------------------------------------------------------------------------
// Callback router (`src/bot/callbacks/callback-router.ts`)
// ---------------------------------------------------------------------------

/// Text before the first `:` (`parseCallbackPrefix`); `None` when there is
/// no separator or it leads the string.
pub fn parse_callback_prefix(data: &str) -> Option<String> {
    let separator = data.find(':')?;
    if separator == 0 {
        return None;
    }
    Some(data[..separator].to_string())
}

/// Prefixes with a registered route (TS: the `routes` map keys).
pub fn is_known_callback_prefix(prefix: &str) -> bool {
    matches!(
        prefix,
        "agent"
            | "attach"
            | "commands"
            | "goal"
            | "compact"
            | "ls"
            | "mcps"
            | "messages"
            | "model"
            | "open"
            | "permission"
            | "project"
            | "projects"
            | "question"
            | "recent"
            | "rename"
            | "session"
            | "settings"
            | "skills"
            | "task"
            | "tasklist"
            | "variant"
            | "worktree"
    )
}

/// Error scope of a route (`permission`, `question`, `taskCreation`,
/// `rename`, `none`, `interaction`); `None` for unknown prefixes.
pub fn route_error_scope(prefix: &str) -> Option<&'static str> {
    match prefix {
        "permission" => Some("permission"),
        "question" => Some("question"),
        "task" => Some("taskCreation"),
        "rename" => Some("rename"),
        "settings" => Some("none"),
        "agent" | "attach" | "commands" | "goal" | "compact" | "ls" | "mcps" | "messages"
        | "model" | "open" | "project" | "projects" | "recent" | "session" | "skills"
        | "tasklist" | "variant" | "worktree" => Some("interaction"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Callback feedback (`src/bot/callbacks/feedback.ts`)
// ---------------------------------------------------------------------------

/// Telegram rejects callback answers longer than this
/// (TS: `CALLBACK_ANSWER_MAX_LENGTH`).
pub const CALLBACK_ANSWER_MAX_LENGTH: usize = 200;

/// Cut an answer to the Telegram budget (TS: `resolveAnswerText`).
pub fn truncate_callback_answer(text: &str) -> String {
    if text.chars().count() <= CALLBACK_ANSWER_MAX_LENGTH {
        return text.to_string();
    }
    text.chars().take(CALLBACK_ANSWER_MAX_LENGTH).collect()
}

// ---------------------------------------------------------------------------
// Permission callback (`src/bot/callbacks/permission-callback-handler.ts`)
// ---------------------------------------------------------------------------

/// Valid permission replies (TS: `isPermissionReply`).
pub fn is_permission_reply(value: &str) -> bool {
    matches!(value, "once" | "always" | "reject")
}

/// Minimal error shape for the not-found check (TS reads `_tag`, `name`,
/// `message`, `data.message` off an unknown value).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionErrorShape {
    pub tag: Option<String>,
    pub name: Option<String>,
    pub message: Option<String>,
    pub data_message: Option<String>,
}

/// True when the error means the request is already gone
/// (TS: `isPermissionRequestNotFound`).
pub fn is_permission_request_not_found(err: &PermissionErrorShape) -> bool {
    if err.tag.as_deref() == Some("PermissionNotFoundError") {
        return true;
    }
    if err.name.as_deref() == Some("NotFoundError") {
        return true;
    }
    [&err.message, &err.data_message]
        .into_iter()
        .flatten()
        .any(|message| {
            message
                .to_lowercase()
                .contains("permission request not found")
        })
}

/// Action segment of a `permission:<action>:...` callback; `None` when the
/// prefix is wrong or the action is missing.
pub fn parse_permission_callback_action(data: &str) -> Option<String> {
    let mut parts = data.split(':');
    if parts.next() != Some("permission") {
        return None;
    }
    match parts.next() {
        Some(action) if !action.is_empty() => Some(action.to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Question callback (`src/bot/callbacks/question-callback-handler.ts`)
// ---------------------------------------------------------------------------

/// Question callback actions (TS: the `switch (action)` cases).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionCallbackAction {
    Select,
    Submit,
    Custom,
    ToggleCustom,
    Cancel,
}

/// Parsed `question:<action>:<index>[:<option>]` callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuestionCallback {
    pub action: QuestionCallbackAction,
    pub question_index: usize,
    pub option_index: Option<usize>,
}

/// Leading-integer parse mirroring TS `parseInt`: leading whitespace and a
/// leading `+` are skipped, trailing junk is ignored, negatives fail.
fn parse_question_index(raw: &str) -> Option<usize> {
    let text = raw.trim();
    let text = text.strip_prefix('+').unwrap_or(text);
    if text.starts_with('-') {
        return None;
    }
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<usize>().ok()
}

/// Parse a question callback; `None` for wrong prefix, unknown action, or
/// bad indexes (TS answers `processing_error_callback` / `inactive`).
pub fn parse_question_callback(data: &str) -> Option<ParsedQuestionCallback> {
    let mut parts = data.split(':');
    if parts.next() != Some("question") {
        return None;
    }
    let action = match parts.next()? {
        "select" => QuestionCallbackAction::Select,
        "submit" => QuestionCallbackAction::Submit,
        "custom" => QuestionCallbackAction::Custom,
        "toggle_custom" => QuestionCallbackAction::ToggleCustom,
        "cancel" => QuestionCallbackAction::Cancel,
        _ => return None,
    };
    let question_index = parse_question_index(parts.next()?)?;
    let option_index = if action == QuestionCallbackAction::Select {
        Some(parse_question_index(parts.next()?)?)
    } else {
        None
    };
    Some(ParsedQuestionCallback {
        action,
        question_index,
        option_index,
    })
}

/// The poll's last question: answering it sends the answers to OpenCode
/// (TS: `isLastQuestion`).
pub fn is_last_question(question_index: usize, total_questions: usize) -> bool {
    total_questions > 0 && question_index == total_questions - 1
}

// ---------------------------------------------------------------------------
// Agent / model / variant selection callbacks
// ---------------------------------------------------------------------------

/// Name after `prefix:` (TS: `data.replace("agent:", "")` and siblings);
/// `None` when the prefix does not match.
pub fn parse_prefixed_callback_name(prefix: &str, data: &str) -> Option<String> {
    let rest = data.strip_prefix(prefix)?;
    let name = rest.strip_prefix(':')?;
    Some(name.to_string())
}

// ---------------------------------------------------------------------------
// Server health (`src/opencode/server-health.ts`)
// ---------------------------------------------------------------------------

/// Report mode (TS: `ProblemReportMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemReportMode {
    Always,
    Once,
}

/// `once` dedupe behind `reportOpencodeProblem` (TS module-level
/// `lastReportedProblem`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProblemDedupe {
    pub last: Option<String>,
}

impl ProblemDedupe {
    /// Create an empty dedupe tracker.
    pub fn new() -> Self {
        Self { last: None }
    }

    /// True when the problem should be logged now; records the key.
    pub fn should_report(&mut self, key: &str, mode: ProblemReportMode) -> bool {
        match mode {
            ProblemReportMode::Always => {
                self.last = Some(key.to_string());
                true
            }
            ProblemReportMode::Once => {
                if self.last.as_deref() == Some(key) {
                    return false;
                }
                self.last = Some(key.to_string());
                true
            }
        }
    }
}

/// Probe outcome subset the classifier reads (TS: `probeOpencodeServer`
/// `kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind {
    Found,
    Unauthorized,
    Other,
}

/// Why a server did not answer as a healthy configured-version server
/// (TS: `FailedHealthCause`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailedHealthCause {
    Unauthorized,
    Mismatch {
        actual: String,
        other_version: String,
    },
    Unknown,
}

/// Classify from the configured-version probe and the other-version probe
/// (TS: `classifyFailedHealthCheck`).
pub fn classify_health_cause(
    configured_unauthorized: bool,
    other_probe: ProbeKind,
    other_server_version: &str,
    other_version: &str,
) -> FailedHealthCause {
    if configured_unauthorized {
        return FailedHealthCause::Unauthorized;
    }
    match other_probe {
        ProbeKind::Found => FailedHealthCause::Mismatch {
            actual: format!("OpenCode {other_server_version}"),
            other_version: other_version.to_string(),
        },
        ProbeKind::Unauthorized => FailedHealthCause::Mismatch {
            actual: "an OpenCode server".to_string(),
            other_version: other_version.to_string(),
        },
        ProbeKind::Other => FailedHealthCause::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Telegram retry (`src/utils/telegram-rate-limit-retry.ts`)
// ---------------------------------------------------------------------------

/// Backoff ceiling for transient server errors
/// (TS: `MAX_SERVER_ERROR_BACKOFF_MS`).
pub const MAX_SERVER_ERROR_BACKOFF_MS: u64 = 8_000;

/// 429 rate-limit status (TS: `isTelegramRateLimitError` status branch).
pub fn is_rate_limit_status(code: u32) -> bool {
    code == 429
}

/// Transient gateway codes 500/502/503/504
/// (TS: `TRANSIENT_SERVER_ERROR_CODES`).
pub fn is_transient_server_error_code(code: u32) -> bool {
    matches!(code, 500 | 502 | 503 | 504)
}

/// `base * 2^attempt` floored at 1 and capped at 8000
/// (TS: `getServerErrorBackoffMs`).
pub fn server_error_backoff_ms(attempt: u32, base_delay_ms: u64) -> u64 {
    let mut delay = base_delay_ms;
    for _ in 0..attempt {
        delay = delay.saturating_mul(2);
        if delay >= MAX_SERVER_ERROR_BACKOFF_MS {
            break;
        }
    }
    delay.clamp(1, MAX_SERVER_ERROR_BACKOFF_MS)
}

/// `retry_after` seconds -> ms, else the fallback delay
/// (TS: `getTelegramRetryAfterMs` rate-limit branch).
pub fn rate_limit_retry_after_ms(retry_after_secs: Option<u64>, fallback_delay_ms: u64) -> u64 {
    match retry_after_secs {
        Some(secs) => secs.saturating_mul(1000),
        None => fallback_delay_ms.max(1),
    }
}

/// Never-sent connection codes (TS: `CONNECT_NOT_ESTABLISHED_CODES`).
pub fn is_unsent_network_code(code: &str) -> bool {
    matches!(
        code,
        "ECONNREFUSED" | "ENOTFOUND" | "EAI_AGAIN" | "ENETUNREACH" | "EHOSTUNREACH"
    )
}

/// Retryable network codes incl. post-write resets
/// (TS: `TRANSIENT_NETWORK_ERROR_CODES`).
pub fn is_transient_network_code(code: &str) -> bool {
    is_unsent_network_code(code)
        || matches!(
            code,
            "ECONNABORTED" | "ECONNRESET" | "EPIPE" | "EPROTO" | "ETIMEDOUT"
        )
}

// ---------------------------------------------------------------------------
// OpenCode errors (`src/utils/opencode-error.ts`)
// ---------------------------------------------------------------------------

/// Expected "server is down" markers (TS:
/// `isExpectedOpencodeUnavailableError`).
pub fn is_expected_unavailable_message(message: &str) -> bool {
    let lower = message.to_lowercase();
    [
        "fetch failed",
        "failed to fetch",
        "econnrefused",
        "econnreset",
        "enotfound",
        "connectex",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// V1/V2 "not found" answer (TS: `isOpencodeNotFoundError`).
pub fn is_not_found_error_name(name: &str) -> bool {
    name == "NotFoundError"
}

/// "Server unavailable" marker anywhere in the text (TS:
/// `hasServerUnavailableMarker`).
pub fn has_server_unavailable_marker(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "fetch failed",
        "econnrefused",
        "connection refused",
        "connect refused",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

/// Candidate fields for the message fallback chain
/// (TS: `extractErrorMessage` reads `message`, `data.message`, `name`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorMessageShape {
    pub message: Option<String>,
    pub data_message: Option<String>,
    pub name: Option<String>,
}

fn first_non_blank(value: &Option<String>) -> Option<String> {
    match value {
        Some(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        None => None,
    }
}

/// First non-blank of `message` -> `data.message` -> `name`
/// (TS: `extractErrorMessage` object branch).
pub fn extract_error_message(shape: &ErrorMessageShape) -> Option<String> {
    if let Some(text) = first_non_blank(&shape.message) {
        return Some(text);
    }
    if let Some(text) = first_non_blank(&shape.data_message) {
        return Some(text);
    }
    if let Some(text) = first_non_blank(&shape.name) {
        return Some(text);
    }
    None
}
