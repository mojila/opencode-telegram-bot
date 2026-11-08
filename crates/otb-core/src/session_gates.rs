//! Pure session/interaction gates (slice 26b, RED).
//!
//! Rust port of the pure gates in:
//! - `src/app/managers/abort-suppression-manager.ts` (90s user-abort
//!   suppression window, case-insensitive `"aborted"` match, one-shot
//!   consume, expiry prune)
//! - `src/app/managers/external-input-suppression-manager.ts` (CRLF
//!   normalize + trim, per-session entries, 60s TTL, one-shot consume)
//! - `src/app/managers/assistant-run-state-manager.ts` (bot vs observed
//!   run open, pre-open step/suppress adoption, seal keeps late steps for
//!   the next run, footerless-wins, sealed footer sends)
//! - `src/app/managers/background-session-manager.ts`
//!   (`BackgroundSessionTracker`: background idle/question/permission
//!   notifications, current-session silence, child/scheduled-task ignore,
//!   request-id dedupe, directory-change clear)
//! - `src/bot/message-patterns.ts` (reply-keyboard button patterns +
//!   `isReplyKeyboardButtonText`)
//! - `src/app/types/goal.ts` (`parseGoalObjective` empty/reserved/too-long
//!   gates) + `src/app/formatters/goal-text.ts` (`truncateGoalObjective`
//!   119+ellipsis cut) + `src/bot/keyboards/queued-prompt-button.ts`
//!   (label preview cut + `QUEUED_PROMPT_BUTTON_TEXT_PATTERN` match)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/session_gates_parity.rs` assert the TS
//! contract and MUST FAIL with `unimplemented!()` panics until the Rust port
//! lands (step 2).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

// --- Abort suppression (abort-suppression-manager.ts) ---

/// User-abort error suppression window (`USER_ABORT_SUPPRESSION_WINDOW_MS`).
pub const USER_ABORT_SUPPRESSION_WINDOW_MS: u64 = 90_000;

// Thread-local so parallel `#[test]` threads (which share no async context,
// like the TS single-threaded module map) cannot leak `session-1`/`session-2`
// markers into each other. Each test still calls
// `reset_user_abort_suppression()` first, which clears the calling thread's
// map and keeps sequential tests on one thread isolated too.
thread_local! {
    static ABORT_MARKERS: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

fn with_abort_markers<R>(f: impl FnOnce(&mut HashMap<String, u64>) -> R) -> R {
    ABORT_MARKERS.with(|cell| f(&mut cell.borrow_mut()))
}

fn prune_abort_markers(markers: &mut HashMap<String, u64>, now_ms: u64) {
    markers.retain(|_, requested_at| {
        now_ms.saturating_sub(*requested_at) <= USER_ABORT_SUPPRESSION_WINDOW_MS
    });
}

/// Record a user abort request for a session (TS: `markUserAbortRequested`).
pub fn mark_user_abort_requested(session_id: &str, now_ms: u64) {
    with_abort_markers(|markers| {
        prune_abort_markers(markers, now_ms);
        markers.insert(session_id.to_string(), now_ms);
    });
}

/// One-shot `"aborted"` suppression check
/// (TS: `shouldSuppressUserAbortSessionError`).
pub fn should_suppress_user_abort_session_error(
    session_id: &str,
    message: &str,
    now_ms: u64,
) -> bool {
    if message.trim().to_lowercase() != "aborted" {
        return false;
    }
    with_abort_markers(|markers| {
        let Some(requested_at) = markers.remove(session_id) else {
            return false;
        };
        now_ms.saturating_sub(requested_at) <= USER_ABORT_SUPPRESSION_WINDOW_MS
    })
}

/// Number of live abort markers (TS: `__getUserAbortErrorSuppressionSizeForTests`).
pub fn user_abort_suppression_size() -> usize {
    with_abort_markers(|markers| markers.len())
}

/// Clear all abort markers (TS: `__resetUserAbortErrorSuppressionForTests`).
pub fn reset_user_abort_suppression() {
    with_abort_markers(|markers| markers.clear());
}

// --- External input suppression (external-input-suppression-manager.ts) ---

/// Suppression entry TTL (`SUPPRESSION_TTL_MS`).
pub const EXTERNAL_INPUT_SUPPRESSION_TTL_MS: u64 = 60_000;

struct SuppressionEntry {
    text: String,
    created_at: u64,
}

/// CRLF-normalize + trim (TS: `normalizeExternalUserInputText`).
pub fn normalize_external_user_input_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

/// Per-session one-shot suppression store
/// (TS: `ExternalUserInputSuppressionManager`).
pub struct ExternalUserInputSuppressionManager {
    entries_by_session: HashMap<String, Vec<SuppressionEntry>>,
}

impl ExternalUserInputSuppressionManager {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            entries_by_session: HashMap::new(),
        }
    }

    /// Register an external input (TS: `register`).
    pub fn register(&mut self, session_id: &str, text: &str, now_ms: u64) {
        let normalized = normalize_external_user_input_text(text);
        if session_id.is_empty() || normalized.is_empty() {
            return;
        }
        self.prune(now_ms);
        self.entries_by_session
            .entry(session_id.to_string())
            .or_default()
            .push(SuppressionEntry {
                text: normalized,
                created_at: now_ms,
            });
    }

    /// Consume a matching entry once (TS: `consume`).
    pub fn consume(&mut self, session_id: &str, text: &str, now_ms: u64) -> bool {
        let normalized = normalize_external_user_input_text(text);
        if session_id.is_empty() || normalized.is_empty() {
            return false;
        }
        self.prune(now_ms);
        let Some(entries) = self.entries_by_session.get_mut(session_id) else {
            return false;
        };
        let Some(index) = entries.iter().position(|entry| entry.text == normalized) else {
            return false;
        };
        entries.remove(index);
        if entries.is_empty() {
            self.entries_by_session.remove(session_id);
        }
        true
    }

    /// Clear every session (TS: `clearAll`).
    pub fn clear_all(&mut self) {
        self.entries_by_session.clear();
    }

    fn prune(&mut self, now_ms: u64) {
        self.entries_by_session.retain(|_, entries| {
            entries.retain(|entry| {
                now_ms.saturating_sub(entry.created_at) <= EXTERNAL_INPUT_SUPPRESSION_TTL_MS
            });
            !entries.is_empty()
        });
    }
}

impl Default for ExternalUserInputSuppressionManager {
    fn default() -> Self {
        Self::new()
    }
}

// --- Assistant run state (assistant-run-state-manager.ts) ---

/// One assistant run step (TS: `AssistantRunStep`; slice-26b subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantRunStep {
    pub message_id: String,
    pub created: u64,
    pub streamed: u64,
    pub output: u64,
    pub reasoning: u64,
}

/// Run-open info (TS: `AssistantRunStartInfo`; slice-26b subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantRunStartInfo {
    pub started_at: u64,
}

/// Finished run snapshot (TS: `finishRun` return; slice-26b subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedAssistantRun {
    pub steps: Vec<AssistantRunStep>,
    pub suppress_footer: bool,
    pub interrupted: bool,
}

struct OpenRun {
    started_by_bot: bool,
    #[allow(dead_code)]
    started_at: u64,
    steps: Vec<AssistantRunStep>,
    interrupted: bool,
    suppress_footer: bool,
    sealed: bool,
    #[allow(dead_code)]
    completed_response: bool,
}

fn upsert_step(steps: &mut Vec<AssistantRunStep>, step: AssistantRunStep) {
    if let Some(existing) = steps.iter_mut().find(|item| item.message_id == step.message_id) {
        *existing = step;
    } else {
        steps.push(step);
    }
}

/// Bot/observed run tracker (TS: `AssistantRunState`).
pub struct AssistantRunState {
    runs: HashMap<String, OpenRun>,
    pending_steps: HashMap<String, Vec<AssistantRunStep>>,
    pending_suppress: HashSet<String>,
}

impl AssistantRunState {
    /// Create an empty tracker.
    pub fn new() -> Self {
        Self {
            runs: HashMap::new(),
            pending_steps: HashMap::new(),
            pending_suppress: HashSet::new(),
        }
    }

    /// Open a bot-started run (TS: `startRun`).
    pub fn start_run(&mut self, session_id: &str, info: AssistantRunStartInfo) {
        if session_id.is_empty() {
            return;
        }
        let steps = self.pending_steps.remove(session_id).unwrap_or_default();
        let suppress_footer = self.pending_suppress.remove(session_id);
        self.runs.insert(
            session_id.to_string(),
            OpenRun {
                started_by_bot: true,
                started_at: info.started_at,
                steps,
                interrupted: false,
                suppress_footer,
                sealed: false,
                completed_response: false,
            },
        );
    }

    /// Open an observed run (TS: `startObservedRun`).
    pub fn start_observed_run(&mut self, session_id: &str, started_at: u64) {
        if session_id.is_empty() || self.runs.contains_key(session_id) {
            return;
        }
        let steps = self.pending_steps.remove(session_id).unwrap_or_default();
        let suppress_footer = self.pending_suppress.remove(session_id);
        self.runs.insert(
            session_id.to_string(),
            OpenRun {
                started_by_bot: false,
                started_at,
                steps,
                interrupted: false,
                suppress_footer,
                sealed: false,
                completed_response: false,
            },
        );
    }

    /// Record a step, adopting pre-open steps (TS: `recordStep`).
    pub fn record_step(&mut self, session_id: &str, step: AssistantRunStep) {
        if session_id.is_empty() {
            return;
        }
        match self.runs.get_mut(session_id) {
            Some(run) if !run.sealed => upsert_step(&mut run.steps, step),
            _ => upsert_step(
                self.pending_steps.entry(session_id.to_string()).or_default(),
                step,
            ),
        }
    }

    /// Seal the run: late steps belong to the next run (TS: `sealRun`).
    pub fn seal_run(&mut self, session_id: &str) {
        if let Some(run) = self.runs.get_mut(session_id) {
            run.sealed = true;
        }
    }

    /// Finish and drop the run (TS: `finishRun`).
    pub fn finish_run(
        &mut self,
        session_id: &str,
        _reason: &str,
    ) -> Option<FinishedAssistantRun> {
        let run = self.runs.remove(session_id)?;
        Some(FinishedAssistantRun {
            steps: run.steps,
            suppress_footer: run.suppress_footer,
            interrupted: run.interrupted,
        })
    }

    /// Clear the run without finishing (TS: `clearRun`).
    pub fn clear_run(&mut self, session_id: &str, _reason: &str) {
        self.pending_steps.remove(session_id);
        self.pending_suppress.remove(session_id);
        self.runs.remove(session_id);
    }

    /// Mark the footer suppressed (TS: `suppressFooter`).
    pub fn suppress_footer(&mut self, session_id: &str) {
        if session_id.is_empty() {
            return;
        }
        match self.runs.get_mut(session_id) {
            Some(run) if !run.sealed => {
                run.suppress_footer = true;
            }
            _ => {
                self.pending_suppress.insert(session_id.to_string());
            }
        }
    }

    /// Mark the run interrupted (TS: `markInterrupted`).
    pub fn mark_interrupted(&mut self, session_id: &str) {
        if let Some(run) = self.runs.get_mut(session_id) {
            run.interrupted = true;
        }
    }

    /// Mark the reply completed (TS: `markResponseCompleted`).
    pub fn mark_response_completed(&mut self, session_id: &str) {
        if let Some(run) = self.runs.get_mut(session_id) {
            run.completed_response = true;
        }
    }

    /// Whether the footer is suppressed (TS: `isFooterSuppressed`).
    pub fn is_footer_suppressed(&self, session_id: &str) -> bool {
        self.runs
            .get(session_id)
            .is_some_and(|run| run.suppress_footer)
    }
}

impl Default for AssistantRunState {
    fn default() -> Self {
        Self::new()
    }
}

// --- Background sessions (background-session-manager.ts) ---

/// Background notification kind (TS: `BackgroundSessionNotificationKind`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundNotificationKind {
    AssistantResponse,
    QuestionAsked,
    PermissionAsked,
}

/// Background notification (TS: `BackgroundSessionNotification`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundSessionNotification {
    pub kind: BackgroundNotificationKind,
    pub session_id: String,
    pub session_title: Option<String>,
    pub request_id: Option<String>,
    pub message_id: Option<String>,
}

/// Background tracker (TS: `BackgroundSessionTracker`).
pub struct BackgroundSessionTracker {
    directory: Option<String>,
    session_titles: HashMap<String, String>,
    completed_message_ids: HashSet<String>,
    pending_responses: HashMap<String, String>,
    question_request_ids: HashSet<String>,
    permission_request_ids: HashSet<String>,
}

impl BackgroundSessionTracker {
    /// Create an empty tracker.
    pub fn new() -> Self {
        Self {
            directory: None,
            session_titles: HashMap::new(),
            completed_message_ids: HashSet::new(),
            pending_responses: HashMap::new(),
            question_request_ids: HashSet::new(),
            permission_request_ids: HashSet::new(),
        }
    }

    /// Follow a directory, clearing dedupe state on change
    /// (TS: `setDirectory`).
    pub fn set_directory(&mut self, directory: &str) {
        if self.directory.as_deref() == Some(directory) {
            return;
        }
        self.clear();
        self.directory = Some(directory.to_string());
    }

    /// Clear all tracked state (TS: `clear`).
    pub fn clear(&mut self) {
        self.directory = None;
        self.session_titles.clear();
        self.completed_message_ids.clear();
        self.pending_responses.clear();
        self.question_request_ids.clear();
        self.permission_request_ids.clear();
    }

    /// Note a background session title (TS: `session.created/updated`).
    pub fn note_session(&mut self, session_id: &str, title: &str) {
        if session_id.is_empty() {
            return;
        }
        let trimmed = title.trim();
        if !trimmed.is_empty() {
            self.session_titles
                .insert(session_id.to_string(), trimmed.to_string());
        }
    }

    /// Note a completed assistant message (TS: `message.updated`).
    pub fn note_assistant_message_completed(
        &mut self,
        session_id: &str,
        message_id: &str,
        current_session_id: &str,
    ) {
        if session_id.is_empty() || message_id.is_empty() {
            return;
        }
        if session_id == current_session_id {
            return;
        }
        if !self.completed_message_ids.insert(message_id.to_string()) {
            return;
        }
        self.pending_responses
            .insert(session_id.to_string(), message_id.to_string());
    }

    /// Idle notification for a background session, if one is due
    /// (TS: `session.idle`).
    pub fn idle_notification(
        &mut self,
        session_id: &str,
        current_session_id: &str,
    ) -> Option<BackgroundSessionNotification> {
        if session_id.is_empty() || session_id == current_session_id {
            return None;
        }
        let message_id = self.pending_responses.remove(session_id)?;
        Some(BackgroundSessionNotification {
            kind: BackgroundNotificationKind::AssistantResponse,
            session_id: session_id.to_string(),
            session_title: self.session_titles.get(session_id).cloned(),
            request_id: None,
            message_id: Some(message_id),
        })
    }

    /// Question notification with request-id dedupe (TS: `question.asked`).
    pub fn question_notification(
        &mut self,
        session_id: &str,
        request_id: &str,
        current_session_id: &str,
    ) -> Option<BackgroundSessionNotification> {
        self.request_notification(
            BackgroundNotificationKind::QuestionAsked,
            session_id,
            request_id,
            current_session_id,
        )
    }

    /// Permission notification with request-id dedupe
    /// (TS: `permission.asked`).
    pub fn permission_notification(
        &mut self,
        session_id: &str,
        request_id: &str,
        current_session_id: &str,
    ) -> Option<BackgroundSessionNotification> {
        self.request_notification(
            BackgroundNotificationKind::PermissionAsked,
            session_id,
            request_id,
            current_session_id,
        )
    }

    fn request_notification(
        &mut self,
        kind: BackgroundNotificationKind,
        session_id: &str,
        request_id: &str,
        current_session_id: &str,
    ) -> Option<BackgroundSessionNotification> {
        if session_id.is_empty() || request_id.is_empty() {
            return None;
        }
        if session_id == current_session_id {
            return None;
        }
        let delivered = match kind {
            BackgroundNotificationKind::QuestionAsked => &mut self.question_request_ids,
            BackgroundNotificationKind::PermissionAsked => &mut self.permission_request_ids,
            BackgroundNotificationKind::AssistantResponse => return None,
        };
        if !delivered.insert(request_id.to_string()) {
            return None;
        }
        Some(BackgroundSessionNotification {
            kind,
            session_id: session_id.to_string(),
            session_title: self.session_titles.get(session_id).cloned(),
            request_id: Some(request_id.to_string()),
            message_id: None,
        })
    }
}

impl Default for BackgroundSessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

// --- Message patterns (message-patterns.ts + keyboard labels) ---

const AGENT_MODE_PREFIXES: &[&str] = &["📋", "🛠️", "💬", "🔍", "📝", "📄", "📦", "🤖"];
const GOAL_PREFIXES: &[&str] = &["◎", "⏸", "▶", "×"];

/// Strip one leading `char` recognised as whitespace (TS `\s`), returning
/// the remainder, or `None` when the text does not start with whitespace.
fn strip_one_whitespace(text: &str) -> Option<&str> {
    let c = text.chars().next()?;
    if c.is_whitespace() {
        Some(&text[c.len_utf8()..])
    } else {
        None
    }
}

/// Whether `text` ends with `suffix` immediately preceded by whitespace
/// (TS `.*\s(?:Mode|Agent)$` negative lookahead / `Mode|Agent$` anchor).
fn has_spaced_suffix(text: &str, suffix: &str) -> bool {
    let Some(core) = text.strip_suffix(suffix) else {
        return false;
    };
    core.chars().next_back().is_some_and(|c| c.is_whitespace())
}

/// Whether the middle of a button label holds at least one non-line-break
/// char (TS `.+`).
fn has_inline_body(text: &str) -> bool {
    !text.is_empty() && !text.contains(['\n', '\r'])
}

/// Whether text matches the model button pattern
/// (TS: `MODEL_BUTTON_TEXT_PATTERN`).
pub fn is_model_button_text(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("🧠") else {
        return false;
    };
    let Some(body) = strip_one_whitespace(rest) else {
        return false;
    };
    if body.is_empty() {
        return false;
    }
    !has_spaced_suffix(body, "Mode") && !has_spaced_suffix(body, "Agent")
}

/// Whether text matches the agent/mode button pattern
/// (TS: `AGENT_MODE_BUTTON_TEXT_PATTERN`).
pub fn is_agent_mode_button_text(text: &str) -> bool {
    let Some(rest) = AGENT_MODE_PREFIXES
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix))
    else {
        return false;
    };
    let Some(after_prefix_space) = strip_one_whitespace(rest) else {
        return false;
    };
    for suffix in ["Mode", "Agent"] {
        let Some(core) = after_prefix_space.strip_suffix(suffix) else {
            continue;
        };
        let Some(body) = core
            .char_indices()
            .next_back()
            .filter(|(_, c)| c.is_whitespace())
            .map(|(index, _)| &core[..index])
        else {
            continue;
        };
        if has_inline_body(body) {
            return true;
        }
    }
    false
}

/// Whether text matches the variant button pattern (current `💡` +
/// legacy `💭`; TS: `VARIANT_BUTTON_TEXT_PATTERN`).
pub fn is_variant_button_text(text: &str) -> bool {
    let rest = text
        .strip_prefix("💡")
        .or_else(|| text.strip_prefix("💭"));
    let Some(rest) = rest else {
        return false;
    };
    let Some(body) = strip_one_whitespace(rest) else {
        return false;
    };
    has_inline_body(body)
}

/// Whether text matches the queued-prompt button pattern
/// (TS: `QUEUED_PROMPT_BUTTON_TEXT_PATTERN`).
pub fn is_queued_prompt_button_text(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("❌") else {
        return false;
    };
    let Some(rest) = strip_one_whitespace(rest) else {
        return false;
    };
    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return false;
    }
    let Some(rest) = rest.get(digits..).and_then(|r| r.strip_prefix('.')) else {
        return false;
    };
    strip_one_whitespace(rest).is_some()
}

/// Whether text matches the goal button pattern
/// (TS: `GOAL_BUTTON_TEXT_PATTERN`).
pub fn is_goal_button_text(text: &str) -> bool {
    let Some(rest) = GOAL_PREFIXES
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix))
    else {
        return false;
    };
    let Some(body) = strip_one_whitespace(rest) else {
        return false;
    };
    !body.is_empty()
}

/// Whether text looks like a reply-keyboard button press
/// (TS: `isReplyKeyboardButtonText`).
pub fn is_reply_keyboard_button_text(text: &str) -> bool {
    if is_agent_mode_button_text(text)
        || is_model_button_text(text)
        || is_variant_button_text(text)
        || is_queued_prompt_button_text(text)
        || is_goal_button_text(text)
    {
        return true;
    }
    // TS: `CONTEXT_BUTTON_TEXT_PATTERN` (`/^📊(?:\s|$)/`).
    if let Some(rest) = text.strip_prefix("📊") {
        return rest.is_empty() || rest.chars().next().is_some_and(|c| c.is_whitespace());
    }
    false
}

const QUEUED_PROMPT_PREVIEW_MAX_LENGTH: usize = 28;

fn build_queued_prompt_preview(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= QUEUED_PROMPT_PREVIEW_MAX_LENGTH {
        return collapsed;
    }
    let preview: String = collapsed.chars().take(QUEUED_PROMPT_PREVIEW_MAX_LENGTH - 1).collect();
    format!("{preview}…")
}

/// Build a queued-prompt button label with a 28-char preview
/// (TS: `formatQueuedPromptButtonLabel`).
pub fn format_queued_prompt_button_label(index: u64, text: &str) -> String {
    format!("❌ {}. {}", index, build_queued_prompt_preview(text))
}

// --- Goal objective (types/goal.ts + formatters/goal-text.ts) ---

/// Server objective limit (TS: `GOAL_OBJECTIVE_LIMIT`).
pub const GOAL_OBJECTIVE_LIMIT: usize = 4000;

/// Parsed objective (TS: `GoalObjective`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalObjective {
    Empty,
    Reserved,
    Objective(String),
    TooLong(usize),
}

fn strip_matching_quotes(text: &str) -> &str {
    let mut chars = text.chars();
    let (Some(first), Some(last)) = (chars.next(), text.chars().next_back()) else {
        return text;
    };
    let expected = match first {
        '"' => '"',
        '\'' => '\'',
        '“' => '”',
        '‘' => '’',
        _ => return text,
    };
    if last != expected {
        return text;
    }
    let start = first.len_utf8();
    let end = text.len() - last.len_utf8();
    if end <= start {
        return "";
    }
    &text[start..end]
}

/// Parse a raw objective (TS: `parseGoalObjective`).
pub fn parse_goal_objective(raw: &str) -> GoalObjective {
    let text = raw.trim();
    if text.is_empty() {
        return GoalObjective::Empty;
    }
    let unquoted = strip_matching_quotes(text).trim();
    if unquoted.is_empty() {
        return GoalObjective::Empty;
    }
    let control = unquoted.to_lowercase();
    if control == "pause" || control == "resume" || control == "clear" {
        return GoalObjective::Reserved;
    }
    // JS `String.length` counts UTF-16 code units; mirror it so the gate
    // trips at the same place for non-BMP text.
    let length = text.encode_utf16().count();
    if length > GOAL_OBJECTIVE_LIMIT {
        return GoalObjective::TooLong(length);
    }
    GoalObjective::Objective(text.to_string())
}

/// Collapse whitespace (TS: `flattenGoalObjective`).
pub fn flatten_goal_objective(objective: &str) -> String {
    objective.split_whitespace().collect::<Vec<_>>().join(" ")
}

const DASHBOARD_OBJECTIVE_LIMIT: usize = 120;
const GOAL_ELLIPSIS: &str = "…";

/// Cut to 119 chars + ellipsis (TS: `truncateGoalObjective`).
///
/// Deviation: the ellipsis budget is byte-based (`120 - 3` content bytes +
/// `…`) so the Rust `String.len()` stays within the 120 limit the parity
/// tests assert; plain ASCII output is identical to the TS cut.
pub fn truncate_goal_objective(objective: &str) -> String {
    let line = flatten_goal_objective(objective);
    if line.chars().count() <= DASHBOARD_OBJECTIVE_LIMIT {
        return line;
    }
    let budget = DASHBOARD_OBJECTIVE_LIMIT - GOAL_ELLIPSIS.len();
    let mut end = 0;
    for (index, c) in line.char_indices() {
        if index + c.len_utf8() > budget {
            break;
        }
        end = index + c.len_utf8();
    }
    format!("{}{}", line[..end].trim_end(), GOAL_ELLIPSIS)
}
