//! Question / permission / prompt-queue / interaction-slot pure state (slice 27b, GREEN).
//!
//! Rust port of the pure state/gating/queue logic in:
//! - `src/app/managers/question-manager.ts` (poll lifecycle, single/multi
//!   select, custom answers, answer/reply items, in-flight endings, answer/
//!   dismissal settle flags)
//! - `src/app/managers/permission-manager.ts` (stale/resolved drop reasons,
//!   question-active gate, generation-scoped resolved ids, settle/send gates)
//! - `src/app/managers/prompt-queue-manager.ts` (cap, trim/blank gates, FIFO,
//!   media budget, reservations, inbox mirror, hand-over, delivered inbox ids)
//! - `src/app/managers/interaction-manager.ts` pure subset (allowed-command
//!   normalize/dedupe, start/transition/expiry/clear, waiting-queue order,
//!   same-session replace, at-head requeue, per-session drops)

// --- Shared caps (prompt-queue-manager.ts) ---

/// Max queued prompts (`MAX_QUEUED_PROMPTS`).
pub const MAX_QUEUED_PROMPTS: usize = 5;
/// Max raw Telegram media bytes retained by all queued prompts.
pub const MAX_QUEUED_MEDIA_BYTES: usize = 20 * 1024 * 1024;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// --- Interaction slot pure subset (interaction-manager.ts) ---

/// Default allowed commands (`DEFAULT_ALLOWED_INTERACTION_COMMANDS`).
pub fn default_allowed_interaction_commands() -> Vec<String> {
    vec![
        "/help".to_string(),
        "/status".to_string(),
        "/abort".to_string(),
        "/detach".to_string(),
        "/opencode_stop".to_string(),
    ]
}

fn normalize_command(command: &str) -> Option<String> {
    let trimmed = command.trim().to_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    let with_slash = if trimmed.starts_with('/') {
        trimmed
    } else {
        format!("/{trimmed}")
    };
    let without_mention = with_slash.split('@').next().unwrap_or("").to_string();
    if without_mention.len() <= 1 {
        return None;
    }
    Some(without_mention)
}

/// Normalize + dedupe allowed commands (TS: `normalizeAllowedCommands`).
pub fn normalize_allowed_commands(commands: Option<&[&str]>) -> Vec<String> {
    match commands {
        None => default_allowed_interaction_commands(),
        Some(list) => {
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for command in list {
                if let Some(value) = normalize_command(command) {
                    if seen.insert(value.clone()) {
                        out.push(value);
                    }
                }
            }
            out
        }
    }
}

/// Snapshot of the interaction slot (TS: `InteractionState`).
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionSnapshot {
    pub kind: String,
    pub expected_input: String,
    pub allowed_commands: Vec<String>,
    pub created_at_ms: i64,
    pub expires_at_ms: Option<i64>,
}

struct SlotState {
    kind: String,
    expected_input: String,
    allowed_commands: Vec<String>,
    created_at_ms: i64,
    expires_at_ms: Option<i64>,
}

/// Pure interaction slot: start / transition / expiry / clear / generation.
pub struct InteractionSlot {
    state: Option<SlotState>,
    generation: u64,
}

impl InteractionSlot {
    pub fn new() -> Self {
        Self {
            state: None,
            generation: 0,
        }
    }

    pub fn start(
        &mut self,
        kind: &str,
        expected_input: &str,
        allowed: Option<&[&str]>,
        expires_in_ms: Option<i64>,
    ) -> InteractionSnapshot {
        let now = now_ms();
        let state = SlotState {
            kind: kind.to_string(),
            expected_input: expected_input.to_string(),
            allowed_commands: normalize_allowed_commands(allowed),
            created_at_ms: now,
            expires_at_ms: expires_in_ms.map(|ms| now + ms),
        };
        let snapshot = InteractionSnapshot {
            kind: state.kind.clone(),
            expected_input: state.expected_input.clone(),
            allowed_commands: state.allowed_commands.clone(),
            created_at_ms: state.created_at_ms,
            expires_at_ms: state.expires_at_ms,
        };
        self.state = Some(state);
        snapshot
    }

    pub fn transition(
        &mut self,
        expected_input: Option<&str>,
        allowed: Option<&[&str]>,
        expires_in_ms: Option<Option<i64>>,
    ) -> Option<InteractionSnapshot> {
        let state = self.state.as_mut()?;
        if let Some(input) = expected_input {
            state.expected_input = input.to_string();
        }
        if let Some(commands) = allowed {
            state.allowed_commands = normalize_allowed_commands(Some(commands));
        }
        match expires_in_ms {
            None => {}
            Some(None) => state.expires_at_ms = None,
            Some(Some(ms)) => state.expires_at_ms = Some(now_ms() + ms),
        }
        Some(InteractionSnapshot {
            kind: state.kind.clone(),
            expected_input: state.expected_input.clone(),
            allowed_commands: state.allowed_commands.clone(),
            created_at_ms: state.created_at_ms,
            expires_at_ms: state.expires_at_ms,
        })
    }

    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    /// Kind held by the slot, if any (live single-slot read for the
    /// attach/prompt busy guard; TS: `interactionManager.getSnapshot()?.kind`).
    pub fn kind(&self) -> Option<&str> {
        self.state.as_ref().map(|state| state.kind.as_str())
    }

    pub fn is_expired(&self, now_ms: i64) -> bool {
        match &self.state {
            None => false,
            Some(state) => match state.expires_at_ms {
                None => false,
                Some(expires) => now_ms >= expires,
            },
        }
    }

    pub fn clear(&mut self) {
        self.state = None;
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn bump_generation(&mut self) {
        self.generation += 1;
    }
}

/// Whether a plain-text prompt must not dispatch while the single
/// interaction slot is occupied (TS: the `interactionManager.isActive()` /
/// `questionManager.isActive()` gate in `message-router.ts` + `prompt.ts`;
/// PRODUCT.md single-slot rule: one blocking interaction at a time).
pub fn is_prompt_blocked_by_slot(slot_kind: Option<&str>) -> bool {
    slot_kind.is_some()
}

/// Locale hook for the blocked-prompt hint; copy stays with telegram-ux /
/// i18n (TS keys: `question.blocked.expected_answer`,
/// `permission.blocked.expected_reply`, otherwise
/// `interaction.blocked.finish_current`).
pub fn blocked_prompt_hint_key(slot_kind: Option<&str>) -> &'static str {
    match slot_kind {
        Some("question") => "question.blocked.expected_answer",
        Some("permission") => "permission.blocked.expected_reply",
        _ => "interaction.blocked.finish_current",
    }
}

// --- Waiting queue pure subset (interaction-manager.ts) ---

/// Kind of the waiting-agent request at the head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitingKind {
    Question,
    Permission,
}

#[derive(Debug, Clone)]
enum WaitingEntry {
    Question {
        request_id: String,
        session_id: String,
    },
    PermissionGroup {
        requests: Vec<(String, String)>,
    },
}

/// Pure waiting queue: order, same-session replace, at-head requeue, drops.
pub struct WaitingQueue {
    waiting: Vec<WaitingEntry>,
}

impl WaitingQueue {
    pub fn new() -> Self {
        Self {
            waiting: Vec::new(),
        }
    }

    pub fn wait_question(&mut self, request_id: &str, session_id: &str) {
        self.push_question(request_id, session_id, false);
    }

    pub fn wait_question_at_head(&mut self, request_id: &str, session_id: &str) {
        self.push_question(request_id, session_id, true);
    }

    fn push_question(&mut self, request_id: &str, session_id: &str, at_head: bool) {
        if let Some(index) = self.waiting.iter().position(|entry| match entry {
            WaitingEntry::Question { session_id: s, .. } => s == session_id,
            _ => false,
        }) {
            self.waiting[index] = WaitingEntry::Question {
                request_id: request_id.to_string(),
                session_id: session_id.to_string(),
            };
        } else if at_head {
            self.waiting.insert(
                0,
                WaitingEntry::Question {
                    request_id: request_id.to_string(),
                    session_id: session_id.to_string(),
                },
            );
        } else {
            self.waiting.push(WaitingEntry::Question {
                request_id: request_id.to_string(),
                session_id: session_id.to_string(),
            });
        }
    }

    pub fn wait_permission(&mut self, request_id: &str, session_id: &str) {
        self.push_permission(request_id, session_id, false);
    }

    pub fn wait_permission_at_head(&mut self, request_id: &str, session_id: &str) {
        self.push_permission(request_id, session_id, true);
    }

    fn push_permission(&mut self, request_id: &str, session_id: &str, at_head: bool) {
        if self
            .waiting_permission_ids()
            .iter()
            .any(|id| id == request_id)
        {
            return;
        }
        let neighbour_is_permission = if self.waiting.is_empty() {
            false
        } else if at_head {
            matches!(self.waiting[0], WaitingEntry::PermissionGroup { .. })
        } else {
            matches!(
                self.waiting[self.waiting.len() - 1],
                WaitingEntry::PermissionGroup { .. }
            )
        };
        // For append, neighbour is the last entry; for at-head, the first.
        if neighbour_is_permission {
            let target = if at_head { 0 } else { self.waiting.len() - 1 };
            if let WaitingEntry::PermissionGroup { requests } = &mut self.waiting[target] {
                requests.push((request_id.to_string(), session_id.to_string()));
            }
        } else if at_head {
            self.waiting.insert(
                0,
                WaitingEntry::PermissionGroup {
                    requests: vec![(request_id.to_string(), session_id.to_string())],
                },
            );
        } else {
            self.waiting.push(WaitingEntry::PermissionGroup {
                requests: vec![(request_id.to_string(), session_id.to_string())],
            });
        }
    }

    pub fn drop_waiting_permission(&mut self, request_id: &str) {
        self.waiting = self
            .waiting
            .drain(..)
            .filter_map(|entry| match entry {
                WaitingEntry::PermissionGroup { requests } => {
                    let kept: Vec<(String, String)> = requests
                        .into_iter()
                        .filter(|(id, _)| id != request_id)
                        .collect();
                    if kept.is_empty() {
                        None
                    } else {
                        Some(WaitingEntry::PermissionGroup { requests: kept })
                    }
                }
                other => Some(other),
            })
            .collect();
    }

    pub fn drop_waiting_question(&mut self, request_id: &str) -> bool {
        let before = self.waiting.len();
        self.waiting.retain(|entry| match entry {
            WaitingEntry::Question { request_id: id, .. } => id != request_id,
            _ => true,
        });
        self.waiting.len() != before
    }

    pub fn drop_waiting_questions_for_session(&mut self, session_id: &str) {
        self.waiting.retain(|entry| match entry {
            WaitingEntry::Question { session_id: s, .. } => s != session_id,
            _ => true,
        });
    }

    pub fn drop_waiting_permissions_for_session(&mut self, session_id: &str) {
        self.waiting = self
            .waiting
            .drain(..)
            .filter_map(|entry| match entry {
                WaitingEntry::PermissionGroup { requests } => {
                    let kept: Vec<(String, String)> = requests
                        .into_iter()
                        .filter(|(_, s)| s != session_id)
                        .collect();
                    if kept.is_empty() {
                        None
                    } else {
                        Some(WaitingEntry::PermissionGroup { requests: kept })
                    }
                }
                other => Some(other),
            })
            .collect();
    }

    pub fn waiting_kind(&self) -> Option<WaitingKind> {
        match self.waiting.first()? {
            WaitingEntry::Question { .. } => Some(WaitingKind::Question),
            WaitingEntry::PermissionGroup { .. } => Some(WaitingKind::Permission),
        }
    }

    pub fn waiting_question_ids(&self) -> Vec<String> {
        self.waiting
            .iter()
            .filter_map(|entry| match entry {
                WaitingEntry::Question { request_id, .. } => Some(request_id.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn waiting_permission_ids(&self) -> Vec<String> {
        self.waiting
            .iter()
            .flat_map(|entry| match entry {
                WaitingEntry::PermissionGroup { requests } => requests
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect()
    }

    pub fn reset(&mut self) {
        self.waiting.clear();
    }
}

// --- Question poll pure subset (question-manager.ts) ---

/// One selectable question option (TS: `QuestionOption`).
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
    pub value: Option<String>,
}

/// One poll question (TS: `Question`).
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub multiple: bool,
    pub options: Vec<QuestionOption>,
}

/// Display line for one option (TS: `formatOptionLine`).
pub fn format_option_line(option: &QuestionOption) -> String {
    format!("* {}: {}", option.label, option.description)
}

/// How a poll settled while answers/a dismissal were on their way
/// (TS: `QuestionSettledOutcome`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettledOutcome {
    Answered,
    Cancelled,
}

/// How a question whose poll message is still in flight ended
/// (TS: `QuestionInFlightEnding`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InFlightEnding {
    Answered,
    Cancelled,
    NotAnswered,
}

struct PollState {
    questions: Vec<Question>,
    current_index: usize,
    selected_options: std::collections::HashMap<usize, Vec<usize>>,
    custom_answers: std::collections::HashMap<usize, String>,
    selected_custom_answers: std::collections::HashSet<usize>,
    custom_input_question_index: Option<usize>,
    active_message_id: Option<i64>,
    message_ids: Vec<i64>,
    request_id: String,
    session_id: String,
    answered_from_telegram: bool,
    dismissing: bool,
    settled_while_sending: Option<SettledOutcome>,
    last_cancel_failed: bool,
}

struct InFlightEntry {
    session_id: String,
    ending: Option<InFlightEnding>,
}

/// Pure question-poll state machine (TS: `QuestionManager` minus Telegram).
pub struct QuestionPoll {
    state: Option<PollState>,
    in_flight: std::collections::HashMap<String, InFlightEntry>,
}

impl QuestionPoll {
    pub fn new() -> Self {
        Self {
            state: None,
            in_flight: std::collections::HashMap::new(),
        }
    }

    pub fn start_questions(
        &mut self,
        questions: Vec<Question>,
        request_id: &str,
        session_id: &str,
    ) -> bool {
        if let Some(state) = &self.state {
            if state.session_id != session_id {
                return false;
            }
        }
        self.state = Some(PollState {
            questions,
            current_index: 0,
            selected_options: std::collections::HashMap::new(),
            custom_answers: std::collections::HashMap::new(),
            selected_custom_answers: std::collections::HashSet::new(),
            custom_input_question_index: None,
            active_message_id: None,
            message_ids: Vec::new(),
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            answered_from_telegram: false,
            dismissing: false,
            settled_while_sending: None,
            last_cancel_failed: false,
        });
        true
    }

    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    pub fn request_id(&self) -> Option<String> {
        self.state.as_ref().map(|s| s.request_id.clone())
    }

    pub fn current_index(&self) -> usize {
        self.state.as_ref().map(|s| s.current_index).unwrap_or(0)
    }

    pub fn total_questions(&self) -> usize {
        self.state.as_ref().map(|s| s.questions.len()).unwrap_or(0)
    }

    pub fn current_question(&self) -> Option<Question> {
        let state = self.state.as_ref()?;
        state.questions.get(state.current_index).cloned()
    }

    pub fn next_question(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.current_index += 1;
            state.custom_input_question_index = None;
            state.active_message_id = None;
        }
    }

    pub fn has_next_question(&self) -> bool {
        match &self.state {
            None => false,
            Some(state) => state.current_index < state.questions.len(),
        }
    }

    pub fn select_option(&mut self, question_index: usize, option_index: usize) {
        let state = match self.state.as_mut() {
            None => return,
            Some(state) => state,
        };
        let question = match state.questions.get(question_index) {
            None => return,
            Some(question) => question,
        };
        let selected = state.selected_options.entry(question_index).or_default();
        if question.multiple {
            if let Some(pos) = selected.iter().position(|&i| i == option_index) {
                selected.remove(pos);
            } else {
                selected.push(option_index);
            }
        } else {
            selected.clear();
            selected.push(option_index);
        }
    }

    pub fn selected_options(&self, question_index: usize) -> Vec<usize> {
        self.state
            .as_ref()
            .and_then(|s| s.selected_options.get(&question_index).cloned())
            .unwrap_or_default()
    }

    pub fn selected_answer(&self, question_index: usize) -> String {
        let state = match self.state.as_ref() {
            None => return String::new(),
            Some(state) => state,
        };
        let question = match state.questions.get(question_index) {
            None => return String::new(),
            Some(question) => question,
        };
        let selected: &[usize] = state
            .selected_options
            .get(&question_index)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        selected
            .iter()
            .filter_map(|idx| question.options.get(*idx).map(format_option_line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn set_custom_answer(&mut self, question_index: usize, answer: &str) {
        let state = match self.state.as_mut() {
            None => return,
            Some(state) => state,
        };
        state
            .custom_answers
            .insert(question_index, answer.to_string());
        if state
            .questions
            .get(question_index)
            .map(|q| q.multiple)
            .unwrap_or(false)
        {
            state.selected_custom_answers.insert(question_index);
        }
    }

    pub fn custom_answer(&self, question_index: usize) -> Option<String> {
        self.state
            .as_ref()
            .and_then(|s| s.custom_answers.get(&question_index).cloned())
    }

    pub fn has_custom_answer(&self, question_index: usize) -> bool {
        self.state
            .as_ref()
            .map(|s| s.custom_answers.contains_key(&question_index))
            .unwrap_or(false)
    }

    pub fn toggle_custom_answer(&mut self, question_index: usize) {
        let state = match self.state.as_mut() {
            None => return,
            Some(state) => state,
        };
        if !state.custom_answers.contains_key(&question_index) {
            return;
        }
        if !state.selected_custom_answers.remove(&question_index) {
            state.selected_custom_answers.insert(question_index);
        }
    }

    pub fn is_custom_answer_selected(&self, question_index: usize) -> bool {
        self.state
            .as_ref()
            .map(|s| s.selected_custom_answers.contains(&question_index))
            .unwrap_or(false)
    }

    pub fn has_answer(&self, question_index: usize) -> bool {
        !self.answer_items(question_index).is_empty()
    }

    pub fn answer_items(&self, question_index: usize) -> Vec<String> {
        let state = match self.state.as_ref() {
            None => return Vec::new(),
            Some(state) => state,
        };
        let question = match state.questions.get(question_index) {
            None => return Vec::new(),
            Some(question) => question,
        };
        let selected_answer = self.selected_answer(question_index);
        let custom_answer = state.custom_answers.get(&question_index).cloned();
        if !question.multiple {
            let answer = match custom_answer {
                Some(custom) if !custom.is_empty() => custom,
                _ => selected_answer,
            };
            return answer
                .split('\n')
                .filter(|part| !part.trim().is_empty())
                .map(|part| part.to_string())
                .collect();
        }
        let mut items: Vec<String> = selected_answer
            .split('\n')
            .filter(|part| !part.trim().is_empty())
            .map(|part| part.to_string())
            .collect();
        if let Some(custom) = custom_answer {
            if !custom.is_empty() && state.selected_custom_answers.contains(&question_index) {
                items.push(custom);
            }
        }
        items
    }

    pub fn reply_items(&self, question_index: usize) -> Vec<String> {
        let state = match self.state.as_ref() {
            None => return Vec::new(),
            Some(state) => state,
        };
        let question = match state.questions.get(question_index) {
            None => return Vec::new(),
            Some(question) => question,
        };
        let custom_answer = state.custom_answers.get(&question_index).cloned();
        if !question.multiple {
            if let Some(custom) = custom_answer.as_ref() {
                if !custom.is_empty() {
                    return self.answer_items(question_index);
                }
            }
        }
        let selected: &[usize] = state
            .selected_options
            .get(&question_index)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let mut items: Vec<String> = selected
            .iter()
            .flat_map(|idx| match question.options.get(*idx) {
                None => Vec::new(),
                Some(opt) => match &opt.value {
                    Some(value) => vec![value.clone()],
                    None => format_option_line(opt)
                        .split('\n')
                        .filter(|part| !part.trim().is_empty())
                        .map(|part| part.to_string())
                        .collect(),
                },
            })
            .collect();
        if question.multiple {
            if let Some(custom) = custom_answer.as_ref() {
                if !custom.is_empty() && state.selected_custom_answers.contains(&question_index) {
                    items.push(custom.clone());
                }
            }
        }
        items
    }

    pub fn all_answers(&self) -> Vec<(String, String)> {
        let state = match self.state.as_ref() {
            None => return Vec::new(),
            Some(state) => state,
        };
        let mut answers = Vec::new();
        for (i, question) in state.questions.iter().enumerate() {
            let final_answer = if question.multiple {
                self.answer_items(i).join("\n")
            } else {
                match state.custom_answers.get(&i) {
                    Some(custom) if !custom.is_empty() => custom.clone(),
                    _ => self.selected_answer(i),
                }
            };
            if !final_answer.is_empty() {
                answers.push((question.question.clone(), final_answer));
            }
        }
        answers
    }

    pub fn start_answer(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.answered_from_telegram = true;
            state.settled_while_sending = None;
            state.last_cancel_failed = false;
            state.custom_input_question_index = None;
        }
    }

    pub fn is_answering(&self) -> bool {
        self.state
            .as_ref()
            .map(|s| s.answered_from_telegram)
            .unwrap_or(false)
    }

    pub fn fail_answer(&mut self) {
        let state = match self.state.as_mut() {
            None => return,
            Some(state) => state,
        };
        state.answered_from_telegram = false;
        state.settled_while_sending = None;
        let is_multiple = state
            .questions
            .get(state.current_index)
            .map(|q| q.multiple)
            .unwrap_or(false);
        if !is_multiple {
            state.custom_answers.remove(&state.current_index);
        }
    }

    pub fn start_dismissal(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.dismissing = true;
            state.settled_while_sending = None;
            state.last_cancel_failed = false;
            state.custom_input_question_index = None;
        }
    }

    pub fn is_dismissing(&self) -> bool {
        self.state.as_ref().map(|s| s.dismissing).unwrap_or(false)
    }

    pub fn fail_dismissal(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.dismissing = false;
            state.settled_while_sending = None;
            state.last_cancel_failed = true;
        }
    }

    pub fn note_settled_while_sending(&mut self, outcome: SettledOutcome) {
        let state = match self.state.as_mut() {
            None => return,
            Some(state) => state,
        };
        if (state.dismissing || state.answered_from_telegram)
            && state.settled_while_sending != Some(SettledOutcome::Answered)
        {
            state.settled_while_sending = Some(outcome);
        }
    }

    pub fn settled_while_sending(&self) -> Option<SettledOutcome> {
        self.state
            .as_ref()
            .and_then(|s| s.settled_while_sending.clone())
    }

    pub fn is_settling_from_telegram(&self) -> bool {
        match &self.state {
            None => false,
            Some(state) => state.answered_from_telegram || state.dismissing,
        }
    }

    pub fn has_last_cancel_failed(&self) -> bool {
        self.state
            .as_ref()
            .map(|s| s.last_cancel_failed)
            .unwrap_or(false)
    }

    pub fn clear_last_cancel_failed(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.last_cancel_failed = false;
        }
    }

    pub fn track_in_flight(&mut self, request_id: &str, session_id: &str) {
        self.in_flight
            .entry(request_id.to_string())
            .or_insert(InFlightEntry {
                session_id: session_id.to_string(),
                ending: None,
            });
    }

    pub fn untrack_in_flight(&mut self, request_id: &str) {
        self.in_flight.remove(request_id);
    }

    pub fn end_in_flight(&mut self, request_id: &str, ending: InFlightEnding) {
        if let Some(entry) = self.in_flight.get_mut(request_id) {
            if entry.ending.is_none() {
                entry.ending = Some(ending);
            }
        }
    }

    pub fn end_in_flight_for_session(&mut self, session_id: &str, ending: InFlightEnding) {
        let ids: Vec<String> = self
            .in_flight
            .iter()
            .filter(|(_, entry)| entry.session_id == session_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.end_in_flight(&id, ending.clone());
        }
    }

    pub fn end_all_in_flight(&mut self, ending: InFlightEnding) {
        let ids: Vec<String> = self.in_flight.keys().cloned().collect();
        for id in ids {
            self.end_in_flight(&id, ending.clone());
        }
    }

    pub fn in_flight_ending(&self, request_id: &str) -> Option<InFlightEnding> {
        self.in_flight.get(request_id)?.ending.clone()
    }

    pub fn start_custom_input(&mut self, question_index: usize) {
        let state = match self.state.as_mut() {
            None => return,
            Some(state) => state,
        };
        if state.questions.get(question_index).is_none() {
            return;
        }
        state.custom_input_question_index = Some(question_index);
    }

    pub fn is_waiting_for_custom_input(&self, question_index: usize) -> bool {
        self.state
            .as_ref()
            .and_then(|s| s.custom_input_question_index)
            .map(|i| i == question_index)
            .unwrap_or(false)
    }

    pub fn set_active_message_id(&mut self, message_id: i64) {
        if let Some(state) = self.state.as_mut() {
            state.active_message_id = Some(message_id);
        }
    }

    pub fn active_message_id(&self) -> Option<i64> {
        self.state.as_ref().and_then(|s| s.active_message_id)
    }

    pub fn is_active_message(&self, message_id: i64) -> bool {
        match self.active_message_id() {
            None => false,
            Some(active) => active == message_id,
        }
    }

    pub fn add_message_id(&mut self, message_id: i64) {
        if let Some(state) = self.state.as_mut() {
            state.message_ids.push(message_id);
        }
    }

    pub fn message_ids(&self) -> Vec<i64> {
        self.state
            .as_ref()
            .map(|s| s.message_ids.clone())
            .unwrap_or_default()
    }

    pub fn cancel(&mut self) {
        self.state = None;
    }

    pub fn clear(&mut self) {
        self.state = None;
    }
}

/// Whether a new poll may open over the shared slot (TS: the `startQuestions`
/// permission/other-session guards in `QuestionManager`).
pub fn poll_start_allowed(
    slot_kind: Option<&str>,
    onscreen_session_id: Option<&str>,
    new_session_id: &str,
) -> bool {
    if slot_kind == Some("permission") {
        return false;
    }
    if slot_kind == Some("question") && onscreen_session_id != Some(new_session_id) {
        return false;
    }
    true
}

// --- Permission pure subset (permission-manager.ts) ---

/// Why a sent permission message was not registered
/// (TS: `StartPermissionResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartPermissionResult {
    Started,
    Stale,
    Resolved,
    QuestionActive,
}

/// Minimal permission request identity for gating (TS: `PermissionRequest`).
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionRequest {
    pub id: String,
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
}

/// Pure permission registry gating (TS: `PermissionManager` minus Telegram).
pub struct PermissionRegistry {
    generation: u64,
    resolved: std::collections::HashSet<String>,
    pending: std::collections::HashMap<i64, PermissionRequest>,
    sending: std::collections::HashSet<i64>,
}

impl PermissionRegistry {
    pub fn new() -> Self {
        Self {
            generation: 0,
            resolved: std::collections::HashSet::new(),
            pending: std::collections::HashMap::new(),
            sending: std::collections::HashSet::new(),
        }
    }

    pub fn start_permission(
        &mut self,
        request: PermissionRequest,
        message_id: i64,
        generation: u64,
    ) -> StartPermissionResult {
        if let Some(reason) = self.drop_reason(&request.id, generation) {
            return reason;
        }
        self.pending.insert(message_id, request);
        StartPermissionResult::Started
    }

    pub fn drop_reason(&self, request_id: &str, generation: u64) -> Option<StartPermissionResult> {
        if generation != self.generation {
            return Some(StartPermissionResult::Stale);
        }
        if self.resolved.contains(request_id) {
            return Some(StartPermissionResult::Resolved);
        }
        None
    }

    pub fn settle_request(&mut self, request_id: &str) {
        self.resolved.insert(request_id.to_string());
        let stale_messages: Vec<i64> = self
            .pending
            .iter()
            .filter(|(_, req)| req.id == request_id)
            .map(|(id, _)| *id)
            .collect();
        for id in stale_messages {
            self.pending.remove(&id);
            self.sending.remove(&id);
        }
    }

    pub fn is_resolved(&self, request_id: &str) -> bool {
        self.resolved.contains(request_id)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn is_active(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn mark_sending(&mut self, message_id: i64) -> bool {
        if !self.pending.contains_key(&message_id) || self.sending.contains(&message_id) {
            return false;
        }
        self.sending.insert(message_id);
        true
    }

    pub fn is_sending(&self, message_id: i64) -> bool {
        self.sending.contains(&message_id)
    }

    pub fn request_signature(request: &PermissionRequest) -> String {
        let mut patterns = request.patterns.clone();
        patterns.sort();
        format!(
            "{}|{}|{}",
            request.session_id,
            request.permission,
            patterns.join(",")
        )
    }
}

/// Whether a permission may register while a poll holds the slot (TS: the
/// `question_active` guard in `PermissionManager.startPermission`).
pub fn permission_start_decision(
    slot_kind: Option<&str>,
    drop_reason: Option<&StartPermissionResult>,
) -> StartPermissionResult {
    if let Some(reason) = drop_reason {
        return reason.clone();
    }
    if slot_kind == Some("question") {
        return StartPermissionResult::QuestionActive;
    }
    StartPermissionResult::Started
}

// --- Prompt queue pure subset (prompt-queue-manager.ts) ---

/// One queued prompt (TS: `QueuedPrompt`).
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedPrompt {
    pub id: String,
    pub text: String,
    pub display_text: String,
    pub media_bytes: usize,
    pub inbox_id: Option<String>,
}

/// Pure prompt queue (TS: `PromptQueueManager` singleton behaviour).
pub struct PromptQueue {
    items: Vec<QueuedPrompt>,
    next_id: u64,
    queued_media_bytes: usize,
    reservations: std::collections::HashSet<String>,
    delivered_inbox_ids: Vec<String>,
}

impl PromptQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            queued_media_bytes: 0,
            reservations: std::collections::HashSet::new(),
            delivered_inbox_ids: Vec::new(),
        }
    }

    pub fn add(&mut self, text: &str, media_bytes: usize) -> Option<QueuedPrompt> {
        let normalized = text.trim().to_string();
        if normalized.is_empty() {
            return None;
        }
        if self.is_full() || !self.can_accept_media(media_bytes) {
            return None;
        }
        let item = QueuedPrompt {
            id: format!("queued-{}", self.next_id),
            text: normalized.clone(),
            display_text: normalized,
            media_bytes,
            inbox_id: None,
        };
        self.next_id += 1;
        self.items.push(item.clone());
        self.queued_media_bytes += media_bytes;
        Some(item)
    }

    pub fn add_attachment(&mut self, display_text: &str) -> Option<QueuedPrompt> {
        if self.is_full() {
            return None;
        }
        let trimmed = display_text.trim();
        let display = if trimmed.is_empty() {
            "[Attachment]".to_string()
        } else {
            trimmed.to_string()
        };
        let item = QueuedPrompt {
            id: format!("queued-{}", self.next_id),
            text: String::new(),
            display_text: display,
            media_bytes: 0,
            inbox_id: None,
        };
        self.next_id += 1;
        self.items.push(item.clone());
        Some(item)
    }

    pub fn size(&self) -> usize {
        self.items.len()
    }

    pub fn is_full(&self) -> bool {
        self.items.len() + self.reservations.len() >= MAX_QUEUED_PROMPTS
    }

    pub fn can_accept_media(&self, media_bytes: usize) -> bool {
        self.queued_media_bytes + media_bytes <= MAX_QUEUED_MEDIA_BYTES
    }

    pub fn media_size(&self) -> usize {
        self.queued_media_bytes
    }

    pub fn list_texts(&self) -> Vec<String> {
        self.items.iter().map(|item| item.text.clone()).collect()
    }

    pub fn remove_by_id(&mut self, id: &str) -> Option<QueuedPrompt> {
        let index = self.items.iter().position(|item| item.id == id)?;
        let removed = self.items.remove(index);
        self.queued_media_bytes = self.queued_media_bytes.saturating_sub(removed.media_bytes);
        Some(removed)
    }

    pub fn take_next(&mut self) -> Option<QueuedPrompt> {
        if self.items.is_empty() {
            return None;
        }
        let item = self.items.remove(0);
        self.queued_media_bytes = self.queued_media_bytes.saturating_sub(item.media_bytes);
        Some(item)
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.reservations.clear();
        self.queued_media_bytes = 0;
        self.delivered_inbox_ids.clear();
    }

    pub fn reserve(&mut self) -> Option<String> {
        if self.is_full() {
            return None;
        }
        let id = format!("reserved-{}", self.next_id);
        self.next_id += 1;
        self.reservations.insert(id.clone());
        Some(id)
    }

    pub fn confirm_reservation(
        &mut self,
        reservation_id: &str,
        display_text: &str,
        inbox_id: &str,
    ) -> Option<QueuedPrompt> {
        if !self.reservations.remove(reservation_id) {
            return None;
        }
        let trimmed = display_text.trim();
        let display = if trimmed.is_empty() {
            "[Attachment]".to_string()
        } else {
            trimmed.to_string()
        };
        let item = QueuedPrompt {
            id: format!("queued-{}", self.next_id),
            text: String::new(),
            display_text: display,
            media_bytes: 0,
            inbox_id: Some(inbox_id.to_string()),
        };
        self.next_id += 1;
        self.items.push(item.clone());
        Some(item)
    }

    pub fn release_reservation(&mut self, reservation_id: &str) -> bool {
        self.reservations.remove(reservation_id)
    }

    pub fn find_by_inbox_id(&self, inbox_id: &str) -> Option<QueuedPrompt> {
        self.items
            .iter()
            .find(|item| item.inbox_id.as_deref() == Some(inbox_id))
            .cloned()
    }

    pub fn remember_delivered_inbox_id(&mut self, inbox_id: &str) {
        self.delivered_inbox_ids.push(inbox_id.to_string());
        if self.delivered_inbox_ids.len() > 20 {
            self.delivered_inbox_ids.remove(0);
        }
    }

    pub fn was_inbox_id_delivered(&self, inbox_id: &str) -> bool {
        self.delivered_inbox_ids.iter().any(|id| id == inbox_id)
    }

    pub fn hand_over(&mut self, _session_id: &str) -> Vec<QueuedPrompt> {
        let removed = std::mem::take(&mut self.items);
        self.reservations.clear();
        self.queued_media_bytes = 0;
        removed
    }
}
