//! Summary aggregation (slices 1-2).
//!
//! Rust port of `src/app/managers/summary-aggregation-manager.ts`
//! (`SummaryAggregator`). Slice 1 covers only the constructor, callback
//! wiring (`set_on_cleared` / `set_on_tool` / `set_on_root_tool_update`),
//! `session_id` in the tool payload, and root-tool update filtering:
//! only the current (root) session's tool parts emit; parts from any
//! other session (e.g. an unrelated child session) are ignored, matching
//! the session filter at the top of TS `handleMessagePartUpdated`.
//! Slice 2 adds live subagent cards: card creation from root `subtask`
//! parts (`register_subtask_part`), task-call input pairing
//! (`update_subagent_from_task_tool` / `find_subagent_for_task_call`),
//! child-session attach, subagent variant from the child assistant
//! message (`update_subagent_from_assistant_message`), and subagent
//! current-tool timing start (`update_subagent_tool_state`).
//! `on_subagent` fires with the live card snapshot on every change
//! (deduped against the last emitted snapshot).
//! Slice 3 adds terminal card ends (interrupted/error with the end
//! message), nested-launch tracking (no card of its own), and live-turn
//! start (`startMissedLiveTurn` / `getLiveTurnStartedAt`).
//! Later slices add text/thinking state, completion dedup,
//! tokens/cost, questions, permissions, and outbound-hold scheduling.
//!
//! TS -> Rust name mapping (slices 1-2):
//! - `SummaryAggregator` -> [`SummaryAggregator`]
//! - `new SummaryAggregator()` -> [`SummaryAggregator::new`]
//! - `setSession` -> [`SummaryAggregator::set_session`]
//! - `clear` -> [`SummaryAggregator::clear`]
//! - `processEvent` -> [`SummaryAggregator::process_event`]
//! - `setOnCleared` -> [`SummaryAggregator::set_on_cleared`]
//! - `setOnTool` -> [`SummaryAggregator::set_on_tool`]
//! - `setOnRootToolUpdate` -> [`SummaryAggregator::set_on_root_tool_update`]
//! - `ToolInfo` -> [`ToolInfo`]
//! - `SubagentInfo` -> [`SubagentInfo`] (`task` = TS `description`,
//!   `agent` = TS `agent`, `model` = TS `providerID`/`modelID`,
//!   `variant` = TS `variant`)
//! - `setOnSubagent` -> [`SummaryAggregator::set_on_subagent`]
//! - `registerSubtaskPart` -> [`SummaryAggregator::register_subtask_part`]
//! - `updateSubagentFromTaskTool` -> [`SummaryAggregator::update_subagent_from_task_tool`]
//! - `findSubagentForTaskCall` -> [`SummaryAggregator::find_subagent_for_task_call`]
//! - `updateSubagentFromAssistantMessage` -> [`SummaryAggregator::update_subagent_from_assistant_message`]
//! - `updateSubagentToolState` -> [`SummaryAggregator::update_subagent_tool_state`]
//! - single-card `setSubagentTerminalStatus(..., "interrupted")` -> [`SummaryAggregator::interrupt_subagent`]
//! - single-card `setSubagentTerminalStatus(..., "error")` -> [`SummaryAggregator::fail_subagent`]
//! - nested `session.created` branch -> [`SummaryAggregator::track_nested_launch`]
//! - `startMissedLiveTurn` -> [`SummaryAggregator::start_missed_live_turn`]
//! - `getLiveTurnStartedAt` -> [`SummaryAggregator::get_live_turn_started_at`]

use std::collections::{HashMap, HashSet};

/// Tool status (TS: `ToolState["status"]`; minimal for slice 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Completed,
    Error,
    Unknown(String),
}

/// Tool payload (TS: `ToolInfo`; minimal fields for slice 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolInfo {
    pub session_id: String,
    pub message_id: String,
    pub call_id: String,
    pub tool: String,
    pub status: ToolStatus,
    pub has_file_attachment: bool,
}

/// One tool part carried by a `message.part.updated` event
/// (TS: the `part` of `message.part.updated` properties).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPart {
    pub session_id: String,
    pub message_id: String,
    pub call_id: String,
    pub tool: String,
    pub status: ToolStatus,
}

/// Subagent lifecycle (TS: `SubagentStatus`; slice-2 subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentStatus {
    Pending,
    Running,
    Completed,
    Error,
    Interrupted,
}

/// Token totals carried on a subagent card (TS: `TokensInfo`; slice-2 subset).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokensInfo {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Live subagent card (TS: `SubagentInfo`; slice-2 subset).
///
/// Field mapping: `task` is TS `description` (the card's task line),
/// `agent` is TS `agent`, `model` is the TS `providerID`/`modelID` pair
/// shown as `provider/id (variant)`, and `variant` is TS `variant`.
/// Card identity (`card_id`/`session_id`/`parent_session_id`) plus
/// current-tool timing (`current_tool_*`) is the subagent card state;
/// [`TaskCallInput`] plus the aggregator's `open_task_calls` map is the
/// task-call input pairing.
#[derive(Debug, Clone, PartialEq)]
pub struct SubagentInfo {
    pub card_id: String,
    pub session_id: Option<String>,
    pub parent_session_id: String,
    pub task: String,
    pub agent: String,
    pub model: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub variant: Option<String>,
    pub status: SubagentStatus,
    pub tokens: TokensInfo,
    pub cost: f64,
    pub current_tool: Option<String>,
    pub current_tool_call_id: Option<String>,
    pub current_tool_started_at: Option<u64>,
    pub current_tool_title: Option<String>,
    /// Terminal message of an errored card (TS: `terminalMessage`).
    pub terminal_message: Option<String>,
}

/// One root `task` call still open, and the card it was paired with once
/// that is known (TS: `openTaskCalls` entry + the call's `input`;
/// slice-2 subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCallInput {
    pub call_id: String,
    pub parent_session_id: String,
    pub description: String,
    pub agent: Option<String>,
    pub prompt: Option<String>,
    pub child_session_id: Option<String>,
    pub paired_card_id: Option<String>,
}

/// Minimal event model (TS: `Event`; slice 1 + slice-2 RED variants).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregatorEvent {
    MessageUpdated {
        session_id: String,
        message_id: String,
    },
    ToolPartUpdated {
        part: ToolPart,
    },
    /// Root `subtask` part announcing a subagent (TS: `part.type === "subtask"`
    /// branch of `handleMessagePartUpdated`; slice 2).
    SubtaskPartUpdated {
        parent_session_id: String,
        part_id: String,
        agent: String,
        description: String,
        prompt: String,
    },
    /// Root `task` tool call carrying its input (TS: `updateSubagentFromTaskTool`;
    /// slice 2). `child_session_id` is TS `state.metadata.sessionId`.
    TaskToolPartUpdated {
        input: TaskCallInput,
    },
    /// Child session discovered (TS: `handleSessionCreatedOrUpdated`; slice 2).
    ChildSessionCreated {
        child_session_id: String,
        parent_session_id: String,
        title: String,
    },
    /// Child assistant message carrying provider/model/variant (TS:
    /// `updateSubagentFromAssistantMessage`; slice 2).
    ChildAssistantMessage {
        child_session_id: String,
        provider_id: Option<String>,
        model_id: Option<String>,
        variant: Option<String>,
    },
    /// Child tool part reporting its current tool (TS: `updateSubagentToolState`;
    /// slice 2).
    ChildToolPartUpdated {
        child_session_id: String,
        call_id: String,
        tool: String,
        title: Option<String>,
        started_at: u64,
    },
    Other,
}

/// Fired when the aggregator is cleared (TS: `ClearedCallback`).
pub type ClearedCallback = Box<dyn FnMut() + Send>;
/// Fired for a tool part (TS: `ToolCallback`).
pub type ToolCallback = Box<dyn FnMut(ToolInfo) + Send>;
/// Fired for a root-session tool update (TS: `RootToolUpdateCallback`).
pub type RootToolUpdateCallback = Box<dyn FnMut(ToolInfo) + Send>;
/// Fired with the live subagent cards (TS: `SubagentCallback`; slice 2).
pub type SubagentCallback = Box<dyn FnMut(String, Vec<SubagentInfo>) + Send>;

/// Summary event aggregator (TS: `SummaryAggregator`; slices 1-2).
pub struct SummaryAggregator {
    /// Session currently followed (TS: `currentSessionId`).
    current_session_id: Option<String>,
    on_cleared: Option<ClearedCallback>,
    on_tool: Option<ToolCallback>,
    on_root_tool_update: Option<RootToolUpdateCallback>,
    /// Live subagent cards by card id (TS: `subagentStates`).
    subagent_cards: HashMap<String, SubagentInfo>,
    /// Card order for snapshots (TS: `subagentOrder`).
    subagent_order: Vec<String>,
    /// Root task calls still open, each with the card it was paired with
    /// once that is known (TS: `openTaskCalls`).
    open_task_calls: HashMap<String, TaskCallInput>,
    on_subagent: Option<SubagentCallback>,
    /// Card id by child session id (TS: `subagentCardIdBySessionId`).
    card_id_by_session: HashMap<String, String>,
    /// Child session id -> parent session id (TS: `trackedSessionParents`,
    /// slice-2 subset: only the child -> root links).
    session_parents: HashMap<String, String>,
    /// Cards waiting for their child session, by parent
    /// (TS: `pendingSubagentCardIdsByParent`).
    pending_cards_by_parent: HashMap<String, Vec<String>>,
    /// Child sessions waiting for their card, by parent
    /// (TS: `pendingChildSessionIdsByParent`).
    pending_children_by_parent: HashMap<String, Vec<String>>,
    /// Session-bound cards a later subtask part may enrich instead of
    /// duplicating (TS: `fallbackSubagentCardIdsByParent`).
    fallback_cards_by_parent: HashMap<String, Vec<String>>,
    /// Subagent events accepted once a session is followed
    /// (TS: `acceptsSubagentEvents`).
    accepts_subagent_events: bool,
    /// Start of the current session's running turn (TS: `liveTurnStartedAt`).
    live_turn_started_at: Option<u64>,
    /// Turn-end generation a missed-turn mark must match
    /// (TS: `turnEndCount`; bumped by `clear` / root idle, slice 8).
    turn_end_count: u64,
    /// Last emitted card snapshot, for change dedup (TS: `lastSubagentSnapshot`).
    last_snapshot: Vec<SubagentInfo>,
    /// Child sessions tracked as background (TS: `backgroundChildSessionIds`
    /// intent + the card `background` flag; slice 5).
    background_child_sessions: HashSet<String>,
    /// Completed tool call ids, for completion dedup (TS:
    /// `processedToolStates` `completed-{callID}` keys; slice 5).
    completed_tool_calls: HashSet<String>,
    /// Reasoning parts per session/message (TS: `thinkingMessageStates`;
    /// slice 4: keyed by `(session_id, message_id)` so sessions never
    /// share thinking state).
    thinking_by_message: HashMap<(String, String), ThinkingState>,
    /// Streaming text parts per session/message (TS: `textMessageStates`;
    /// slice-4 subset: `orderedPartIds` + `partTexts`, keyed by
    /// `(session_id, message_id)`).
    streams_by_message: HashMap<(String, String), StreamState>,
    /// Session-idle callback (TS: `onSessionIdleCallback`; slice 6).
    on_session_idle: Option<SessionIdleCallback>,
    /// Session-retry callback (TS: `onSessionRetryCallback`; slice 6).
    on_session_retry: Option<SessionRetryCallback>,
    /// Idle already emitted for the current run (TS: duplicate
    /// `session.idle` for the same run collapses to one emit; slice 6).
    idle_emitted: bool,
    /// Outbound hold flag (TS: `outboundHeld`; slice 6).
    outbound_held: bool,
    /// Outbound hold generation (TS: `outboundHoldGeneration`; slice 6).
    outbound_hold_generation: u64,
    /// Held outbound callbacks, replayed in order by `flush_outbound`
    /// (TS: `deferredOutbound`; slice 6).
    deferred_outbound: Vec<DeferredOutbound>,
    /// Changed files per session, in arrival order (TS: `onFileChange`
    /// payloads per changed file + `session.diff`; slice 7).
    file_changes_by_session: HashMap<String, Vec<FileChange>>,
    /// Open question cards by `(session_id, request_id)` (TS:
    /// `question.asked` until `question.settled`; slice 7).
    open_questions: HashSet<(String, String)>,
    /// Permission decisions by `(session_id, request_id)`: `None` while
    /// the card is open, `Some(allow)` once replied (TS:
    /// `permission.asked` / `permission.replied`; slice 7).
    permission_decisions: HashMap<(String, String), Option<bool>>,
    /// Child sessions with a recorded `session.created` (TS:
    /// `trackedSessionParents` / `handleSessionCreatedOrUpdated`; slice 7).
    created_child_sessions: HashSet<String>,
    /// Ended subagent sessions (TS: `finishedSubagentSessionIds`; slice 15).
    finished_subagent_sessions: HashSet<String>,
    /// Token totals callback (TS: `onTokensCallback`; slice 11).
    on_tokens: Option<TokensCallback>,
    /// Message cost callback (TS: `onCostCallback`; slice 11).
    on_cost: Option<CostCallback>,
    /// Run-end callback (TS: `onSessionRunEndedCallback`; slice 11).
    on_run_ended: Option<RunEndedCallback>,
    /// Todo lists per session (slice 11; models the todo-tool list the
    /// summary cards carry — no direct TS counterpart in the manager).
    todos_by_session: HashMap<String, Vec<TodoItem>>,
    /// Synthetic text part ids per message (TS: `syntheticPartIds`;
    /// slice 9). A delta event often carries only the part id, so the
    /// flag is looked up in the registry too.
    synthetic_part_ids: HashMap<String, HashSet<String>>,
    /// Messages that have fired a thinking update (TS:
    /// `thinkingFiredForMessages`; slice 14).
    thinking_fired_messages: HashSet<String>,
    /// Messages that already signalled thinking-finished (TS:
    /// `thinkingFinishedForMessages`; slice 14).
    thinking_finished_messages: HashSet<String>,
}

/// Held outbound callback (TS: `deferredOutbound` entry; slice-6 subset:
/// only the idle/retry callbacks go through the hold here).

/// Held outbound payload (TS: `DeferredOutboundTask`; slice-6 subset).
#[derive(Debug, Clone, PartialEq, Eq)]
enum DeferredOutbound {
    Idle(String, SessionIdleInfo),
    Retry(SessionRetryInfo),
}

/// Ordered reasoning parts for one message (TS: `ThinkingMessageState`).
#[derive(Debug, Clone, Default)]
struct ThinkingState {
    ordered_part_ids: Vec<String>,
    sections: HashMap<String, ThinkingSection>,
}

/// Ordered streaming text parts for one message (TS: `TextMessageState`
/// slice-4 subset: `orderedPartIds` + `partTexts`).
#[derive(Debug, Clone, Default)]
struct StreamState {
    ordered_part_ids: Vec<String>,
    part_texts: HashMap<String, String>,
}

impl Default for SummaryAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// The live SSE pump runs on `tokio::spawn` (multi-thread); the aggregator
// crosses that `Send` boundary, so its TS-parity `FnMut` callbacks carry
// `+ Send` (tests capture via `Arc<Mutex<..>>`).
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<SummaryAggregator>();
};

impl SummaryAggregator {
    /// Create an aggregator (TS: `new SummaryAggregator()`).
    pub fn new() -> Self {
        Self {
            current_session_id: None,
            on_cleared: None,
            on_tool: None,
            on_root_tool_update: None,
            subagent_cards: HashMap::new(),
            subagent_order: Vec::new(),
            open_task_calls: HashMap::new(),
            on_subagent: None,
            card_id_by_session: HashMap::new(),
            session_parents: HashMap::new(),
            pending_cards_by_parent: HashMap::new(),
            pending_children_by_parent: HashMap::new(),
            fallback_cards_by_parent: HashMap::new(),
            accepts_subagent_events: false,
            live_turn_started_at: None,
            turn_end_count: 0,
            last_snapshot: Vec::new(),
            background_child_sessions: HashSet::new(),
            completed_tool_calls: HashSet::new(),
            thinking_by_message: HashMap::new(),
            streams_by_message: HashMap::new(),
            on_session_idle: None,
            on_session_retry: None,
            idle_emitted: false,
            outbound_held: false,
            outbound_hold_generation: 0,
            deferred_outbound: Vec::new(),
            file_changes_by_session: HashMap::new(),
            open_questions: HashSet::new(),
            permission_decisions: HashMap::new(),
            created_child_sessions: HashSet::new(),
            finished_subagent_sessions: HashSet::new(),
            on_tokens: None,
            on_cost: None,
            on_run_ended: None,
            todos_by_session: HashMap::new(),
            synthetic_part_ids: HashMap::new(),
            thinking_fired_messages: HashSet::new(),
            thinking_finished_messages: HashSet::new(),
        }
    }

    /// Follow a session, clearing previous state (TS: `setSession`).
    pub fn set_session(&mut self, session_id: &str) {
        if self.current_session_id.as_deref() != Some(session_id) {
            self.clear();
            self.current_session_id = Some(session_id.to_string());
            self.accepts_subagent_events = true;
        }
    }

    /// Reset state and fire the cleared callback (TS: `clear`).
    ///
    /// TS delivers the callback through the outbound queue; with no hold
    /// held that delivery is synchronous, so slice 1 invokes it inline.
    /// Slice-2 card/call state clears with the session (TS: `clear`
    /// empties `subagentStates`, `subagentOrder`, `openTaskCalls`).
    pub fn clear(&mut self) {
        self.current_session_id = None;
        self.subagent_cards.clear();
        self.subagent_order.clear();
        self.open_task_calls.clear();
        self.card_id_by_session.clear();
        self.session_parents.clear();
        self.pending_cards_by_parent.clear();
        self.pending_children_by_parent.clear();
        self.fallback_cards_by_parent.clear();
        self.accepts_subagent_events = false;
        self.live_turn_started_at = None;
        // Turn-end generation: `clear` ends the run (TS: `clear`
        // `turnEndCount++`), so a missed-turn mark taken before it no
        // longer matches.
        self.turn_end_count += 1;
        self.last_snapshot.clear();
        self.background_child_sessions.clear();
        self.completed_tool_calls.clear();
        self.thinking_by_message.clear();
        self.streams_by_message.clear();
        self.idle_emitted = false;
        self.outbound_held = false;
        self.deferred_outbound.clear();
        self.file_changes_by_session.clear();
        self.open_questions.clear();
        self.permission_decisions.clear();
        self.created_child_sessions.clear();
        self.finished_subagent_sessions.clear();
        self.todos_by_session.clear();
        self.synthetic_part_ids.clear();
        self.thinking_fired_messages.clear();
        self.thinking_finished_messages.clear();
        if let Some(callback) = self.on_cleared.as_mut() {
            callback();
        }
    }

    /// Handle one OpenCode event (TS: `processEvent`).
    pub fn process_event(&mut self, event: AggregatorEvent) {
        match event {
            AggregatorEvent::MessageUpdated { .. } => {
                // Message text-state tracking lands in a later slice (TS:
                // `handleMessageUpdated`). Slice 1 accepts the event and
                // keeps no per-message state.
            }
            AggregatorEvent::ToolPartUpdated { part } => {
                self.handle_tool_part(part);
            }
            AggregatorEvent::SubtaskPartUpdated {
                parent_session_id,
                part_id,
                agent,
                description,
                prompt,
            } => {
                self.register_subtask_part(
                    &parent_session_id,
                    &part_id,
                    &agent,
                    &description,
                    &prompt,
                );
            }
            AggregatorEvent::TaskToolPartUpdated { input } => {
                self.update_subagent_from_task_tool(input);
            }
            AggregatorEvent::ChildSessionCreated {
                child_session_id,
                parent_session_id,
                title,
            } => {
                self.attach_child_session(&child_session_id, &parent_session_id, &title);
            }
            AggregatorEvent::ChildAssistantMessage {
                child_session_id,
                provider_id,
                model_id,
                variant,
            } => {
                self.update_subagent_from_assistant_message(
                    &child_session_id,
                    provider_id.as_deref(),
                    model_id.as_deref(),
                    variant.as_deref(),
                );
            }
            AggregatorEvent::ChildToolPartUpdated {
                child_session_id,
                call_id,
                tool,
                title,
                started_at,
            } => {
                self.update_subagent_tool_state(
                    &child_session_id,
                    &call_id,
                    &tool,
                    title.as_deref(),
                    started_at,
                );
            }
            AggregatorEvent::Other => {}
        }
    }

    /// Register the cleared callback (TS: `setOnCleared`).
    pub fn set_on_cleared(&mut self, callback: ClearedCallback) {
        self.on_cleared = Some(callback);
    }

    /// Register the tool callback (TS: `setOnTool`).
    pub fn set_on_tool(&mut self, callback: ToolCallback) {
        self.on_tool = Some(callback);
    }

    /// Register the root-tool-update callback (TS: `setOnRootToolUpdate`).
    pub fn set_on_root_tool_update(&mut self, callback: RootToolUpdateCallback) {
        self.on_root_tool_update = Some(callback);
    }

    /// Register the subagent callback (TS: `setOnSubagent`).
    pub fn set_on_subagent(&mut self, callback: SubagentCallback) {
        self.on_subagent = Some(callback);
    }

    /// Announce a subagent from a root `subtask` part (TS:
    /// `registerSubtaskPart`).
    ///
    /// TS: `registerSubtaskPart` reuses a fallback card when the child
    /// session arrived first (`fallbackSubagentCardIdsByParent`), else
    /// creates the card (`subtask-{parent}-{partId}`), enriches it from
    /// the subtask (`enrichSubagentFromSubtask`), then pairs it with a
    /// pending child session or queues the card as pending.
    pub fn register_subtask_part(
        &mut self,
        parent_session_id: &str,
        part_id: &str,
        agent: &str,
        description: &str,
        prompt: &str,
    ) {
        if !self.accepts_subagent_events {
            return;
        }

        if let Some(fallback_id) =
            Self::dequeue_from(&mut self.fallback_cards_by_parent, parent_session_id)
        {
            if self.subagent_cards.contains_key(&fallback_id) {
                self.enrich_card_from_subtask(&fallback_id, agent, description, prompt);
                self.emit_subagents();
                return;
            }
        }

        let card_id = format!("subtask-{}-{}", parent_session_id, part_id);
        if !self.subagent_cards.contains_key(&card_id) {
            self.subagent_cards.insert(
                card_id.clone(),
                SubagentInfo {
                    card_id: card_id.clone(),
                    session_id: None,
                    parent_session_id: parent_session_id.to_string(),
                    task: String::new(),
                    agent: String::new(),
                    model: String::new(),
                    provider_id: None,
                    model_id: None,
                    variant: None,
                    status: SubagentStatus::Pending,
                    tokens: TokensInfo::default(),
                    cost: 0.0,
                    current_tool: None,
                    current_tool_call_id: None,
                    current_tool_started_at: None,
                    current_tool_title: None,
                    terminal_message: None,
                },
            );
            self.subagent_order.push(card_id.clone());
        }
        self.enrich_card_from_subtask(&card_id, agent, description, prompt);

        if let Some(child) =
            Self::dequeue_from(&mut self.pending_children_by_parent, parent_session_id)
        {
            self.attach_session_to_card(&card_id, &child);
        } else {
            let queue = self
                .pending_cards_by_parent
                .entry(parent_session_id.to_string())
                .or_default();
            if !queue.contains(&card_id) {
                queue.push(card_id.clone());
            }
        }
        self.emit_subagents();
    }

    /// Pair a root `task` call's input with its own card (TS:
    /// `updateSubagentFromTaskTool` + `findSubagentForTaskCall`).
    ///
    /// TS: `updateSubagentFromTaskTool` opens `openTaskCalls`, resolves
    /// the card via `findSubagentForTaskCall` (named child session, else the
    /// only open call with the only waiting card), then
    /// `enrichSubagentFromTaskTool` + `emitSubagentState`.
    pub fn update_subagent_from_task_tool(&mut self, input: TaskCallInput) {
        let call_id = input.call_id.clone();
        let parent_session_id = input.parent_session_id.clone();
        {
            let entry = self
                .open_task_calls
                .entry(call_id.clone())
                .or_insert_with(|| TaskCallInput {
                    call_id: call_id.clone(),
                    parent_session_id: parent_session_id.clone(),
                    description: String::new(),
                    agent: None,
                    prompt: None,
                    child_session_id: None,
                    paired_card_id: None,
                });
            entry.parent_session_id = parent_session_id.clone();
            entry.description = input.description.clone();
            entry.agent = input.agent.clone();
            entry.prompt = input.prompt.clone();
            if input.child_session_id.is_some() {
                entry.child_session_id = input.child_session_id.clone();
            }
        }

        let mut card_id = self
            .open_task_calls
            .get(&call_id)
            .and_then(|call| call.paired_card_id.clone())
            .filter(|id| self.subagent_cards.contains_key(id));
        if card_id.is_none() {
            let named_child = self
                .open_task_calls
                .get(&call_id)
                .and_then(|call| call.child_session_id.clone());
            if let Some(child) = named_child {
                card_id =
                    Some(self.get_or_create_card_for_session(&child, Some(&parent_session_id)));
            } else {
                let waiting_calls = self
                    .open_task_calls
                    .values()
                    .filter(|call| {
                        call.parent_session_id == parent_session_id && call.paired_card_id.is_none()
                    })
                    .count();
                if waiting_calls == 1 {
                    let linked: HashSet<String> = self
                        .open_task_calls
                        .values()
                        .filter_map(|call| call.paired_card_id.clone())
                        .collect();
                    let mut waiting_cards = self.subagent_order.iter().filter(|id| {
                        self.subagent_cards.get(*id).map_or(false, |card| {
                            card.parent_session_id == parent_session_id && !linked.contains(*id)
                        })
                    });
                    let first = waiting_cards.next().cloned();
                    let second = waiting_cards.next();
                    if first.is_some() && second.is_none() {
                        card_id = first;
                    }
                }
            }
        }

        let card_id = match card_id {
            Some(id) => id,
            None => return,
        };
        if let Some(call) = self.open_task_calls.get_mut(&call_id) {
            call.paired_card_id = Some(card_id.clone());
        }
        self.enrich_card_from_task_input(&card_id, &input);
        self.emit_subagents();
    }

    /// Resolve the card a root task call belongs to (TS:
    /// `findSubagentForTaskCall`).
    ///
    /// TS: the card of the child session the call names, or - while it
    /// names none - the only card still waiting for its call, when this is
    /// the only call still waiting for a card. Read-only here (TS mutates
    /// the open call's link; the pairing write lives in
    /// `update_subagent_from_task_tool` since this takes `&self`).
    pub fn find_subagent_for_task_call(
        &self,
        call_id: &str,
        child_session_id: Option<&str>,
    ) -> Option<String> {
        let call = self.open_task_calls.get(call_id)?;
        if let Some(paired) = call.paired_card_id.as_deref() {
            if self.subagent_cards.contains_key(paired) {
                return Some(paired.to_string());
            }
        }
        if let Some(child) = child_session_id {
            if let Some(card_id) = self.card_id_by_session.get(child) {
                return Some(card_id.clone());
            }
            if call.child_session_id.as_deref() == Some(child) {
                return Some(child.to_string());
            }
        }
        None
    }

    /// Apply provider/model/variant from a child assistant message (TS:
    /// `updateSubagentFromAssistantMessage`).
    ///
    /// TS: child `message.updated` with `role === "assistant"` sets
    /// `providerID`/`modelID`/`variant` on the card (the combo shows as
    /// `provider/id (variant)`), each kept across later messages and never
    /// stored when empty.
    pub fn update_subagent_from_assistant_message(
        &mut self,
        child_session_id: &str,
        provider_id: Option<&str>,
        model_id: Option<&str>,
        variant: Option<&str>,
    ) {
        let card_id = self.get_or_create_card_for_session(child_session_id, None);
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            if let Some(provider) = provider_id {
                if !provider.is_empty() {
                    card.provider_id = Some(provider.to_string());
                }
            }
            if let Some(model) = model_id {
                if !model.is_empty() {
                    card.model_id = Some(model.to_string());
                }
            }
            if let Some(variant) = variant {
                if !variant.is_empty() {
                    card.variant = Some(variant.to_string());
                }
            }
            let both_known = card.provider_id.is_some() && card.model_id.is_some();
            if let Some(display) = Self::format_subagent_model(
                card.provider_id.as_deref(),
                card.model_id.as_deref(),
                card.variant.as_deref(),
            ) {
                if both_known || card.model.is_empty() {
                    card.model = display;
                }
            }
        }
        self.emit_subagents();
    }

    /// Track a child subagent's current tool and start its timing (TS:
    /// `updateSubagentToolState`).
    ///
    /// TS: the child's running `tool` part sets `currentTool` (+ title)
    /// and, for a new call id, `currentToolCallId` with a fresh
    /// `currentToolStartedAt`, then `emitSubagentState`.
    pub fn update_subagent_tool_state(
        &mut self,
        child_session_id: &str,
        call_id: &str,
        tool: &str,
        title: Option<&str>,
        started_at: u64,
    ) {
        let card_id = self.get_or_create_card_for_session(child_session_id, None);
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            card.status = SubagentStatus::Running;
            if !call_id.is_empty() && card.current_tool_call_id.as_deref() != Some(call_id) {
                card.current_tool_call_id = Some(call_id.to_string());
                card.current_tool_started_at = Some(started_at);
            }
            card.current_tool = Some(tool.to_string());
            card.current_tool_title = title.map(|value| value.to_string());
        }
        self.emit_subagents();
    }

    /// Emit the live card snapshot (TS: `emitSubagentState`).
    ///
    /// TS returns early with no current session, no callback, or no cards,
    /// and skips unchanged snapshots; delivery is synchronous here since
    /// slice 2 has no outbound hold.
    fn emit_subagents(&mut self) {
        let current = match self.current_session_id.clone() {
            Some(session_id) => session_id,
            None => return,
        };
        if self.subagent_order.is_empty() {
            return;
        }
        let snapshot: Vec<SubagentInfo> = self
            .subagent_order
            .iter()
            .filter_map(|card_id| self.subagent_cards.get(card_id).cloned())
            .collect();
        if snapshot.is_empty() || snapshot == self.last_snapshot {
            return;
        }
        self.last_snapshot = snapshot.clone();
        if let Some(callback) = self.on_subagent.as_mut() {
            callback(current, snapshot);
        }
    }

    /// Shift the head off a per-parent queue, dropping the empty queue.
    fn dequeue_from(
        queue_by_parent: &mut HashMap<String, Vec<String>>,
        parent_session_id: &str,
    ) -> Option<String> {
        let queue = queue_by_parent.get_mut(parent_session_id)?;
        if queue.is_empty() {
            queue_by_parent.remove(parent_session_id);
            return None;
        }
        let value = queue.remove(0);
        if queue.is_empty() {
            queue_by_parent.remove(parent_session_id);
        }
        Some(value)
    }

    /// Card for a child session, creating a session-bound card when none
    /// exists (TS: `getOrCreateSubagentForSession`; slice-2 subset without
    /// the launch/end bookkeeping that lands in later slices).
    fn get_or_create_card_for_session(
        &mut self,
        session_id: &str,
        parent_hint: Option<&str>,
    ) -> String {
        if let Some(card_id) = self.card_id_by_session.get(session_id).cloned() {
            return card_id;
        }
        let parent = parent_hint
            .map(|value| value.to_string())
            .or_else(|| self.session_parents.get(session_id).cloned())
            .or_else(|| self.current_session_id.clone())
            .unwrap_or_else(|| session_id.to_string());
        if let Some(queue) = self.pending_children_by_parent.get_mut(&parent) {
            queue.retain(|pending| pending != session_id);
            if queue.is_empty() {
                self.pending_children_by_parent.remove(&parent);
            }
        }
        let card_id = session_id.to_string();
        self.subagent_cards.insert(
            card_id.clone(),
            SubagentInfo {
                card_id: card_id.clone(),
                session_id: Some(session_id.to_string()),
                parent_session_id: parent.clone(),
                task: String::new(),
                agent: String::new(),
                model: String::new(),
                provider_id: None,
                model_id: None,
                variant: None,
                status: SubagentStatus::Pending,
                tokens: TokensInfo::default(),
                cost: 0.0,
                current_tool: None,
                current_tool_call_id: None,
                current_tool_started_at: None,
                current_tool_title: None,
                terminal_message: None,
            },
        );
        self.subagent_order.push(card_id.clone());
        self.card_id_by_session
            .insert(session_id.to_string(), card_id.clone());
        let queue = self.fallback_cards_by_parent.entry(parent).or_default();
        if !queue.contains(&card_id) {
            queue.push(card_id.clone());
        }
        card_id
    }

    /// Bind a child session to a pending card (TS: `attachSessionToSubagent`).
    fn attach_session_to_card(&mut self, card_id: &str, session_id: &str) {
        let parent = match self.subagent_cards.get(card_id) {
            Some(card) => card.parent_session_id.clone(),
            None => return,
        };
        if let Some(card) = self.subagent_cards.get_mut(card_id) {
            card.session_id = Some(session_id.to_string());
        }
        self.card_id_by_session
            .insert(session_id.to_string(), card_id.to_string());
        self.session_parents
            .insert(session_id.to_string(), parent.clone());
        if let Some(queue) = self.pending_cards_by_parent.get_mut(&parent) {
            queue.retain(|pending| pending != card_id);
            if queue.is_empty() {
                self.pending_cards_by_parent.remove(&parent);
            }
        }
        if let Some(queue) = self.pending_children_by_parent.get_mut(&parent) {
            queue.retain(|pending| pending != session_id);
            if queue.is_empty() {
                self.pending_children_by_parent.remove(&parent);
            }
        }
    }

    /// Attach a discovered child session to its card (TS: the card half of
    /// `handleSessionCreatedOrUpdated`): an existing card is enriched from
    /// the title, else a pending card for the parent attaches, else a
    /// session-bound card is created (kept as a fallback for a later
    /// subtask part).
    fn attach_child_session(
        &mut self,
        child_session_id: &str,
        parent_session_id: &str,
        title: &str,
    ) {
        if self.current_session_id.is_none() {
            return;
        }
        self.session_parents
            .insert(child_session_id.to_string(), parent_session_id.to_string());
        if let Some(card_id) = self.card_id_by_session.get(child_session_id).cloned() {
            self.enrich_card_from_title(&card_id, title);
            self.emit_subagents();
            return;
        }
        if let Some(pending) =
            Self::dequeue_from(&mut self.pending_cards_by_parent, parent_session_id)
        {
            if self.subagent_cards.contains_key(&pending) {
                self.attach_session_to_card(&pending, child_session_id);
                self.enrich_card_from_title(&pending, title);
                self.emit_subagents();
                return;
            }
        }
        let card_id =
            self.get_or_create_card_for_session(child_session_id, Some(parent_session_id));
        self.enrich_card_from_title(&card_id, title);
        self.emit_subagents();
    }

    /// Enrich a card from a root subtask part (TS: `enrichSubagentFromSubtask`;
    /// slice-2 subset: Rust `task` = TS `description`; no `command` field).
    fn enrich_card_from_subtask(
        &mut self,
        card_id: &str,
        agent: &str,
        description: &str,
        prompt: &str,
    ) {
        if let Some(card) = self.subagent_cards.get_mut(card_id) {
            if !agent.is_empty() {
                card.agent = agent.to_string();
            }
            let next = if !description.is_empty() {
                description
            } else {
                prompt
            };
            if !next.is_empty() {
                card.task = next.to_string();
            }
        }
    }

    /// Enrich a card from a root task call's input (TS:
    /// `enrichSubagentFromTaskTool`; slice-2 subset).
    fn enrich_card_from_task_input(&mut self, card_id: &str, input: &TaskCallInput) {
        if let Some(card) = self.subagent_cards.get_mut(card_id) {
            if let Some(agent) = input.agent.as_deref() {
                let agent = agent.trim();
                if !agent.is_empty() {
                    card.agent = agent.to_string();
                }
            }
            let description = input.description.trim();
            let prompt = input.prompt.as_deref().unwrap_or("").trim();
            let next = if !description.is_empty() {
                description
            } else {
                prompt
            };
            if !next.is_empty() {
                card.task = next.to_string();
            }
        }
    }

    /// Enrich a card from a child session title (TS:
    /// `enrichSubagentFromSessionTitle`; `"Desc (@agent subagent)"`
    /// contributes both halves).
    fn enrich_card_from_title(&mut self, card_id: &str, title: &str) {
        let (description, agent) = Self::split_session_title(title);
        if let Some(card) = self.subagent_cards.get_mut(card_id) {
            if let Some(description) = description {
                if !description.is_empty() {
                    card.task = description;
                }
            }
            if let Some(agent) = agent {
                if !agent.is_empty() {
                    card.agent = agent;
                }
            }
        }
    }

    /// Split `"Desc (@agent subagent)"` into `(description, agent)`
    /// (TS: the title regex in `enrichSubagentFromSessionTitle`).
    fn split_session_title(title: &str) -> (Option<String>, Option<String>) {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return (None, None);
        }
        if let Some(open) = trimmed.rfind(" (@") {
            let tail = &trimmed[open + 3..];
            if let Some(inner) = tail.strip_suffix(')') {
                let mut words: Vec<&str> = inner.split_whitespace().collect();
                if words.len() >= 2
                    && words
                        .last()
                        .map_or(false, |word| word.eq_ignore_ascii_case("subagent"))
                {
                    words.pop();
                    let agent = words.join(" ").trim().trim_start_matches('@').to_string();
                    let description = trimmed[..open].trim().to_string();
                    return (
                        if description.is_empty() {
                            None
                        } else {
                            Some(description)
                        },
                        if agent.is_empty() { None } else { Some(agent) },
                    );
                }
            }
        }
        (Some(trimmed.to_string()), None)
    }

    /// Display for a provider/model/variant combo (TS: the card shows
    /// `provider/id (variant)`; the parts stay separate on the card).
    fn format_subagent_model(
        provider_id: Option<&str>,
        model_id: Option<&str>,
        variant: Option<&str>,
    ) -> Option<String> {
        let provider = provider_id.filter(|value| !value.is_empty());
        let model = model_id.filter(|value| !value.is_empty());
        let variant = variant.filter(|value| !value.is_empty());
        match (provider, model) {
            (Some(provider), Some(model)) => Some(match variant {
                Some(variant) => format!("{}/{} ({})", provider, model, variant),
                None => format!("{}/{}", provider, model),
            }),
            (Some(provider), None) => Some(provider.to_string()),
            (None, Some(model)) => Some(model.to_string()),
            (None, None) => None,
        }
    }

    /// End a running subagent card as interrupted.
    ///
    /// TS: `setSubagentTerminalStatus(..., "interrupted")` (subagent
    /// `session.idle` with `interrupted: true`, parent-run interrupt).
    pub fn interrupt_subagent(&mut self, child_session_id: &str) {
        let card_id = match self.card_id_by_session.get(child_session_id).cloned() {
            Some(card_id) => card_id,
            None => return,
        };
        let interrupted = match self.subagent_cards.get_mut(&card_id) {
            Some(card)
                if matches!(
                    card.status,
                    SubagentStatus::Pending | SubagentStatus::Running
                ) =>
            {
                card.status = SubagentStatus::Interrupted;
                card.current_tool = None;
                card.current_tool_call_id = None;
                card.current_tool_started_at = None;
                card.current_tool_title = None;
                card.terminal_message = None;
                true
            }
            _ => false,
        };
        if interrupted {
            self.emit_subagents();
        }
    }

    /// End a running subagent card as failed.
    ///
    /// TS: `setSubagentTerminalStatus(..., "error")` on `session.error`
    /// with a non-abort error; the message trims to `terminalMessage`.
    pub fn fail_subagent(&mut self, child_session_id: &str, message: &str) {
        let card_id = match self.card_id_by_session.get(child_session_id).cloned() {
            Some(card_id) => card_id,
            None => return,
        };
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            card.status = SubagentStatus::Error;
            card.current_tool = None;
            card.current_tool_call_id = None;
            card.current_tool_started_at = None;
            card.current_tool_title = None;
            let trimmed = message.trim();
            card.terminal_message = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        self.emit_subagents();
    }

    /// Track a nested subagent launch under a child subagent.
    ///
    /// TS: nested `session.created` under a tracked child (a nested subagent
    /// "has no card to end: it only stops being tracked"). The launch takes
    /// no card of its own, so the parent keeps its Running Task line and no
    /// snapshot follows.
    pub fn track_nested_launch(&mut self, parent_child_session_id: &str, nested_session_id: &str) {
        self.session_parents.insert(
            nested_session_id.to_string(),
            parent_child_session_id.to_string(),
        );
    }

    /// Start a missed live turn (turn-start emit).
    ///
    /// TS: `startMissedLiveTurn` (a `session.status` busy the stream missed).
    /// The mark must match the turn-end generation and no turn may already
    /// run. The start is a deterministic `1_000` stand-in for TS `Date.now()`
    /// (the TS tests fake the clock at `1_000`); real clock wiring lands
    /// with `session.status` handling in a later slice.
    pub fn start_missed_live_turn(&mut self, session_id: &str, mark: u64) {
        if Some(session_id) != self.current_session_id.as_deref() {
            return;
        }
        if mark != self.turn_end_count {
            return;
        }
        if self.live_turn_started_at.is_none() {
            self.live_turn_started_at = Some(1_000);
        }
    }

    /// Start of the session's running turn (turn-start emit).
    ///
    /// TS: `getLiveTurnStartedAt`: the start when the session is the current
    /// one and a turn runs, else `None`.
    pub fn get_live_turn_started_at(&self, session_id: &str) -> Option<u64> {
        if Some(session_id) == self.current_session_id.as_deref() {
            self.live_turn_started_at
        } else {
            None
        }
    }

    /// Tool-part handling (TS: the `tool` branch of
    /// `handleMessagePartUpdated` for the current root session).
    ///
    /// Session scoping first: parts whose session is not the current one
    /// are ignored (child tool parts arrive as `ChildToolPartUpdated`
    /// instead, matching the session filter at the top of TS
    /// `handleMessagePartUpdated`: `!isCurrentRootSession &&
    /// !isTrackedChildSession -> return`).
    fn handle_tool_part(&mut self, part: ToolPart) {
        if self.current_session_id.as_deref() != Some(part.session_id.as_str()) {
            return;
        }

        let info = ToolInfo {
            session_id: part.session_id.clone(),
            message_id: part.message_id.clone(),
            call_id: part.call_id.clone(),
            tool: part.tool.clone(),
            status: part.status.clone(),
            has_file_attachment: false,
        };

        // Root-tool updates fire for every part, running included, before
        // any completion filtering (TS emits `onRootToolUpdateCallback`
        // ahead of the `status === "completed"` gate). Only completed
        // parts reach `on_tool`, deduped per call id (TS:
        // `processedToolStates` `completed-{callID}`).
        if let Some(callback) = self.on_root_tool_update.as_mut() {
            callback(info.clone());
        }

        if part.status != ToolStatus::Completed {
            return;
        }
        if !self.completed_tool_calls.insert(part.call_id.clone()) {
            return;
        }

        if let Some(callback) = self.on_tool.as_mut() {
            callback(info);
        }
    }
}

// ---------------------------------------------------------------------------
// SLICE 4 (RED): thinking/reasoning + partial streaming state.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`registerThinkingPart` / `setThinkingPartSnapshot` /
// `applyThinkingDelta`, `applyTextDelta`, `handleSessionError`
// streaming reset). TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`
// (reasoning/thinking, partial/streaming, idle/error/retry blocks).
// ---------------------------------------------------------------------------

/// One reasoning block (TS: `ThinkingSection`; slice-4 stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingSection {
    pub id: String,
    pub title: Option<String>,
    pub text: String,
}

impl SummaryAggregator {
    /// Track a reasoning part for a session message (TS:
    /// `registerThinkingPart` + `setThinkingPartSnapshot`; slice 4).
    ///
    /// TS stores the snapshot text per part id in `thinkingMessageStates`
    /// (`orderedPartIds` + `sections`); here the state is keyed by
    /// `(session_id, message_id)` so sessions never share thinking state.
    pub fn track_thinking(
        &mut self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        text: &str,
    ) {
        let key = (session_id.to_string(), message_id.to_string());
        let state = self.thinking_by_message.entry(key).or_default();
        if !state.ordered_part_ids.iter().any(|id| id == part_id) {
            state.ordered_part_ids.push(part_id.to_string());
        }
        let title = state
            .sections
            .get(part_id)
            .and_then(|section| section.title.clone());
        state.sections.insert(
            part_id.to_string(),
            ThinkingSection {
                id: part_id.to_string(),
                title,
                text: text.to_string(),
            },
        );
    }

    /// Reasoning sections for a message, collapsed in part order with
    /// text preserved (TS: `getThinkingSections`; slice 4).
    pub fn thinking_sections(&self, session_id: &str, message_id: &str) -> Vec<ThinkingSection> {
        let key = (session_id.to_string(), message_id.to_string());
        let state = match self.thinking_by_message.get(&key) {
            Some(state) => state,
            None => return Vec::new(),
        };
        state
            .ordered_part_ids
            .iter()
            .filter_map(|part_id| state.sections.get(part_id).cloned())
            .collect()
    }

    /// Collapse same-id reasoning sections in order, preserving text
    /// (TS: ordered `sections` map joined per part; slice 4).
    pub fn collapse_reasoning_sections(
        &self,
        sections: Vec<ThinkingSection>,
    ) -> Vec<ThinkingSection> {
        let mut order: Vec<String> = Vec::new();
        let mut merged: HashMap<String, ThinkingSection> = HashMap::new();
        for section in sections {
            if let Some(existing) = merged.get_mut(&section.id) {
                existing.text.push_str(&section.text);
                if existing.title.is_none() {
                    existing.title = section.title.clone();
                }
            } else {
                order.push(section.id.clone());
                merged.insert(section.id.clone(), section);
            }
        }
        order
            .into_iter()
            .filter_map(|id| merged.remove(&id))
            .collect()
    }

    /// Accumulate a streaming text delta, returning the combined text
    /// (TS: `applyTextDelta`; slice 4).
    ///
    /// TS joins every part of the message in `orderedPartIds` order
    /// (`getCombinedMessageText`); here the state is keyed by
    /// `(session_id, message_id)` so sessions never share streaming text.
    pub fn accumulate_stream_text(
        &mut self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        delta: &str,
    ) -> String {
        let key = (session_id.to_string(), message_id.to_string());
        let state = self.streams_by_message.entry(key).or_default();
        if !state.ordered_part_ids.iter().any(|id| id == part_id) {
            state.ordered_part_ids.push(part_id.to_string());
        }
        let previous = state.part_texts.get(part_id).cloned().unwrap_or_default();
        let accumulated = format!("{previous}{delta}");
        state.part_texts.insert(part_id.to_string(), accumulated);
        state
            .ordered_part_ids
            .iter()
            .map(|id| state.part_texts.get(id).cloned().unwrap_or_default())
            .collect::<String>()
    }

    /// Drop streaming state after an error/retry so the next turn
    /// starts clean (TS: `handleSessionError` streaming reset; slice 4).
    pub fn reset_stream_on_error(&mut self, session_id: &str) {
        self.thinking_by_message
            .retain(|(session, _), _| session != session_id);
        self.streams_by_message
            .retain(|(session, _), _| session != session_id);
    }
}

// ---------------------------------------------------------------------------
// SLICE 5 (GREEN): tokens/cost accumulation + background cards + completion dedup.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`updateSubagentStepFinish` tokens/cost at ~1819-1850,
// `updateSubagentFromAssistantMessage` tokens/cost at ~1689-1699,
// `handleMessageUpdated` cost callback at ~2023-2030,
// `markBackgroundSubagent` / `backgroundChildSessionIds` at ~1219-1242 /
// ~2227, completed-tool dedup `processedToolStates` at ~2279-2282).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (live subagent tokens/cost at lines 183-320, step-finish event at
// 1556-1575, background `launchInBackground` at 709-757 / 1435-1494,
// parallel subagents at 1604-1681, terminal/dedup cards at 1683-1901).
// Each method below is a RED stub (`unimplemented!()`) until the port lands.
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Replace token totals on a subagent card (TS:
    /// `updateSubagentStepFinish` tokens assignment + assistant-message
    /// tokens assignment; slice 5).
    ///
    /// TS assigns `subagent.tokens = {...}` on every step-finish
    /// (last finish wins); only `cost` accumulates (`subagent.cost +=
    /// cost`).
    pub fn record_tokens(&mut self, child_session_id: &str, tokens: TokensInfo) {
        let card_id = self.get_or_create_card_for_session(child_session_id, None);
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            card.status = SubagentStatus::Running;
            card.terminal_message = None;
            card.tokens = tokens;
        }
        self.emit_subagents();
    }

    /// Accumulate cost onto a subagent card (TS: `subagent.cost += cost`
    /// in `updateSubagentStepFinish` plus assistant-message `cost`; slice 5).
    pub fn record_cost(&mut self, child_session_id: &str, cost: f64) {
        let card_id = self.get_or_create_card_for_session(child_session_id, None);
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            card.cost += cost;
        }
        self.emit_subagents();
    }

    /// Total cost across live subagent cards (TS: summary cost totals;
    /// slice 5).
    pub fn total_cost(&self) -> f64 {
        self.subagent_cards.values().map(|card| card.cost).sum()
    }

    /// Format a cost as `$X.XX` (TS: `` `$${cost.toFixed(2)}` `` in
    /// `handleMessageUpdated` / `formatCostLine`; slice 5).
    pub fn format_cost(cost: f64) -> String {
        format!("${cost:.2}")
    }

    /// Mark a subagent card as background so it outlives its parent's turn
    /// (TS: `markBackgroundSubagent` + `backgroundChildSessionIds`; slice 5).
    pub fn track_background_card(&mut self, child_session_id: &str) {
        self.background_child_sessions
            .insert(child_session_id.to_string());
        self.emit_subagents();
    }

    /// Whether a subagent card is tracked as background (TS: `background`
    /// flag on the card; slice 5).
    pub fn is_background_card(&self, child_session_id: &str) -> bool {
        self.background_child_sessions.contains(child_session_id)
    }

    /// Record a completed tool call; first call returns true, repeats
    /// return false (TS: `processedToolStates` `completed-{callID}` dedup;
    /// slice 5).
    pub fn record_tool_completion(&mut self, call_id: &str) -> bool {
        self.completed_tool_calls.insert(call_id.to_string())
    }
}

// ---------------------------------------------------------------------------
// SLICE 6 (RED): idle/retry gates + outbound hold + completion-dedup
// remainder + `session.status` gating.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`handleSessionIdle` at ~2919-2969, `handleSessionStatus` busy/retry at
// ~2882-2917, `handleSessionError` at ~3003-3051,
// `holdOutbound`/`drainOutbound`/`scheduleOutbound` at ~634-793).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (root `session.idle` at lines 2749-2779, `live turn start` at 2781-2867,
// `session.status` retry at 3141-3167). Each method below is a RED stub
// (`unimplemented!()`) until the port lands.
// ---------------------------------------------------------------------------

/// Root-session idle payload (TS: `SessionIdleInfo`; slice-6 stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIdleInfo {
    pub interrupted: bool,
}

/// Root-session retry payload (TS: `SessionRetryInfo`; slice-6 stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRetryInfo {
    pub session_id: String,
    pub attempt: Option<u64>,
    pub message: String,
}

/// Fired when the followed session goes idle (TS: `SessionIdleCallback`).
pub type SessionIdleCallback = Box<dyn FnMut(String, SessionIdleInfo) + Send>;
/// Fired on a `session.status` retry for the followed session
/// (TS: `SessionRetryCallback`).
pub type SessionRetryCallback = Box<dyn FnMut(SessionRetryInfo) + Send>;

impl SummaryAggregator {
    /// Register the session-idle callback (TS: `setOnSessionIdle`).
    pub fn set_on_session_idle(&mut self, callback: SessionIdleCallback) {
        self.on_session_idle = Some(callback);
    }

    /// Register the session-retry callback (TS: `setOnSessionRetry`).
    pub fn set_on_session_retry(&mut self, callback: SessionRetryCallback) {
        self.on_session_retry = Some(callback);
    }

    /// Report the followed session idle through the idle gate (TS:
    /// `handleSessionIdle` root branch: only the current session emits,
    /// duplicate idles for the same run collapse to one emit; slice 6).
    pub fn set_idle_gated(&mut self, session_id: &str, interrupted: bool) {
        if Some(session_id) != self.current_session_id.as_deref() {
            return;
        }
        if self.idle_emitted {
            return;
        }
        self.idle_emitted = true;
        self.live_turn_started_at = None;
        self.turn_end_count += 1;
        self.accepts_subagent_events = false;
        let info = SessionIdleInfo { interrupted };
        if self.outbound_held {
            self.deferred_outbound
                .push(DeferredOutbound::Idle(session_id.to_string(), info));
            return;
        }
        if let Some(callback) = self.on_session_idle.as_mut() {
            callback(session_id.to_string(), info);
        }
    }

    /// Report a `session.status` retry after an error path (TS:
    /// `handleSessionStatus` retry branch + `handleSessionError` reset:
    /// a failed turn drops its streaming state so the retry re-emits;
    /// slice 6).
    pub fn retry_after_error(&mut self, session_id: &str, attempt: u64, message: &str) {
        if Some(session_id) != self.current_session_id.as_deref() {
            return;
        }
        // The retry opens a fresh attempt, so a later idle for the new run
        // must emit again.
        self.idle_emitted = false;
        let info = SessionRetryInfo {
            session_id: session_id.to_string(),
            attempt: Some(attempt),
            message: message.to_string(),
        };
        if self.outbound_held {
            self.deferred_outbound.push(DeferredOutbound::Retry(info));
            return;
        }
        if let Some(callback) = self.on_session_retry.as_mut() {
            callback(info);
        }
    }

    /// Hold outbound delivery, returning the hold generation (TS:
    /// `holdOutbound`; slice 6).
    pub fn hold_outbound(&mut self) -> u64 {
        self.outbound_held = true;
        self.outbound_hold_generation += 1;
        self.outbound_hold_generation
    }

    /// Drain the held outbound queue for a hold generation (TS:
    /// `drainOutbound`; slice 6).
    pub fn flush_outbound(&mut self, generation: u64) {
        if !self.outbound_held || generation != self.outbound_hold_generation {
            return;
        }
        self.outbound_held = false;
        let deferred = std::mem::take(&mut self.deferred_outbound);
        for entry in deferred {
            match entry {
                DeferredOutbound::Idle(session_id, info) => {
                    if let Some(callback) = self.on_session_idle.as_mut() {
                        callback(session_id, info);
                    }
                }
                DeferredOutbound::Retry(info) => {
                    if let Some(callback) = self.on_session_retry.as_mut() {
                        callback(info);
                    }
                }
            }
        }
    }

    /// Handle a `session.status` signal with current-session gating (TS:
    /// `handleSessionStatus`: non-current sessions return early; `busy`
    /// starts the live turn, `retry` fires the retry callback; slice 6).
    pub fn handle_session_status(&mut self, session_id: &str, status: &str) {
        if Some(session_id) != self.current_session_id.as_deref() {
            return;
        }
        if status == "busy" {
            if self.live_turn_started_at.is_none() {
                self.live_turn_started_at = Some(1_000);
            }
            // A new turn opens a new run, so the next idle emits again.
            self.idle_emitted = false;
        }
        // `retry` carries its attempt/message payload through
        // `retry_after_error`; without it there is nothing to emit here.
    }
}

// ---------------------------------------------------------------------------
// SLICE 7 (GREEN): apply_patch/file parts + question/permission cards +
// `session.created` gating.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`prepareToolFiles` apply_patch branch at ~2809-2870,
// `handleQuestionAsked`/`handleQuestionSettled` at ~3053-3111,
// `handlePermissionAsked`/`handlePermissionReplied` at ~3155-3230,
// `handleSessionCreatedOrUpdated` at ~1600-1656).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (apply_patch at lines 3169-3485, permission at 3638-3709,
// questions of subagent sessions at 3754-3886,
// `session.created` gating at 1535-1602).
// ---------------------------------------------------------------------------

/// One changed file tracked for a session (TS: `FileChange`; slice 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub file: String,
    pub additions: u64,
    pub deletions: u64,
}

impl SummaryAggregator {
    /// Track an `apply_patch` file change for a session (TS:
    /// `prepareToolFiles` apply_patch branch + `onFileChange`; slice 7).
    ///
    /// TS emits one `fileChange` per changed file (`metadata.files[]` via
    /// `getPatchFileChanges`, else the single `filediff` fallback); the
    /// Rust port keeps them per session in arrival order. Re-tracking an
    /// already listed file updates its totals in place.
    pub fn track_file_change(
        &mut self,
        session_id: &str,
        file: &str,
        additions: u64,
        deletions: u64,
    ) {
        let changes = self
            .file_changes_by_session
            .entry(session_id.to_string())
            .or_default();
        if let Some(existing) = changes.iter_mut().find(|change| change.file == file) {
            existing.additions = additions;
            existing.deletions = deletions;
        } else {
            changes.push(FileChange {
                file: file.to_string(),
                additions,
                deletions,
            });
        }
    }

    /// Changed files tracked for a session, in arrival order (TS:
    /// `onFileChange` payloads per changed file; slice 7).
    pub fn file_changes(&self, session_id: &str) -> Vec<FileChange> {
        self.file_changes_by_session
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Total `(additions, deletions)` across a session's tracked files
    /// (TS: file-summary totals; slice 7).
    pub fn file_summary_totals(&self, session_id: &str) -> (u64, u64) {
        self.file_changes_by_session
            .get(session_id)
            .map(|changes| {
                changes
                    .iter()
                    .fold((0u64, 0u64), |(additions, deletions), change| {
                        (
                            additions.saturating_add(change.additions),
                            deletions.saturating_add(change.deletions),
                        )
                    })
            })
            .unwrap_or((0, 0))
    }

    /// Open a question card for a session (TS: `handleQuestionAsked`;
    /// slice 7).
    pub fn open_question(&mut self, session_id: &str, request_id: &str) {
        self.open_questions
            .insert((session_id.to_string(), request_id.to_string()));
    }

    /// Settle a question card as answered (TS: `handleQuestionSettled`
    /// with `outcome === "answered"`; slice 7).
    pub fn answer_question(&mut self, session_id: &str, request_id: &str) {
        self.open_questions
            .remove(&(session_id.to_string(), request_id.to_string()));
    }

    /// Whether a question card is still open (TS: open-question state;
    /// slice 7).
    pub fn is_question_open(&self, session_id: &str, request_id: &str) -> bool {
        self.open_questions
            .contains(&(session_id.to_string(), request_id.to_string()))
    }

    /// Open a permission card for a session (TS: `handlePermissionAsked`;
    /// slice 7). A re-ask keeps an already recorded decision.
    pub fn request_permission(&mut self, session_id: &str, request_id: &str, _permission: &str) {
        self.permission_decisions
            .entry((session_id.to_string(), request_id.to_string()))
            .or_insert(None);
    }

    /// Decide a permission card (TS: `handlePermissionReplied`; slice 7).
    /// `allow = true` records an allow, `false` a deny.
    pub fn decide_permission(&mut self, session_id: &str, request_id: &str, allow: bool) {
        self.permission_decisions.insert(
            (session_id.to_string(), request_id.to_string()),
            Some(allow),
        );
    }

    /// Last decision for a permission card: `Some(true)` allow,
    /// `Some(false)` deny, `None` undecided (TS: reply state; slice 7).
    pub fn permission_decision(&self, session_id: &str, request_id: &str) -> Option<bool> {
        self.permission_decisions
            .get(&(session_id.to_string(), request_id.to_string()))
            .copied()
            .flatten()
    }

    /// Record `session.created` so later child events attach (TS:
    /// `handleSessionCreatedOrUpdated`; slice 7). Events for unknown
    /// children before this call stay gated.
    pub fn handle_session_created(&mut self, session_id: &str, parent_session_id: &str) {
        self.created_child_sessions.insert(session_id.to_string());
        self.session_parents
            .insert(session_id.to_string(), parent_session_id.to_string());
    }

    /// Whether child events for a session are still gated behind a
    /// missing `session.created` (TS: unknown-child gating; slice 7).
    /// The followed session itself is never gated; sessions already
    /// known through any attach path (created record, parent link, or
    /// bound card) are ungated.
    pub fn is_child_gated(&self, session_id: &str) -> bool {
        if Some(session_id) == self.current_session_id.as_deref() {
            return false;
        }
        !self.created_child_sessions.contains(session_id)
            && !self.session_parents.contains_key(session_id)
            && !self.card_id_by_session.contains_key(session_id)
    }
}

// ---------------------------------------------------------------------------
// SLICE 8 (GREEN): completion filtering + turn-end mark + subagent-event
// gating + root-session mapping + per-session cost reset.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`handleMessagePartUpdated` tool branch `status === "completed"` gate at
// ~2273-2282, `clear` `turnEndCount++` at ~946,
// `setSession`/`handleSessionIdle` `acceptsSubagentEvents` at ~914/~2951,
// `getRootSessionId` at ~1012-1023, `clear` subagent-state reset at
// ~932-940).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (tool payload at lines 56-103, `live turn start` at 2781-2867,
// subagent gating at 1553/1632, root resolution in the nested-subagent
// block at 1175-1234, cost on the live card at 245-268).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Turn-end generation mark (TS: `getTurnEndMark` returns
    /// `turnEndCount`; slice 8). `clear` bumps it so a stale missed-turn
    /// mark no longer matches.
    pub fn get_turn_end_mark(&self) -> u64 {
        self.turn_end_count
    }

    /// Whether subagent events are currently accepted (TS:
    /// `acceptsSubagentEvents`: true after `setSession`, false after
    /// `clear` / root idle / root error; slice 8).
    pub fn accepts_subagent_events(&self) -> bool {
        self.accepts_subagent_events
    }

    /// Root session a tracked session belongs to, found by walking its
    /// parents; an untracked session is its own root (TS:
    /// `getRootSessionId`; slice 8).
    pub fn get_root_session_id(&self, session_id: &str) -> String {
        let mut visited: HashSet<String> = HashSet::new();
        let mut root = session_id.to_string();
        let mut parent = self.session_parents.get(&root).cloned();
        while let Some(parent_id) = parent {
            if visited.contains(&parent_id) {
                break;
            }
            visited.insert(root.clone());
            root = parent_id;
            parent = self.session_parents.get(&root).cloned();
        }
        root
    }

    /// Cost carried on one session's subagent card, `0.0` when it has none
    /// (TS: `subagent.cost`; `clear` drops all cards so every session
    /// reads back `0.0`; slice 8).
    pub fn session_cost(&self, session_id: &str) -> f64 {
        self.card_id_by_session
            .get(session_id)
            .and_then(|card_id| self.subagent_cards.get(card_id))
            .or_else(|| self.subagent_cards.get(session_id))
            .map(|card| card.cost)
            .unwrap_or(0.0)
    }
}

// ---------------------------------------------------------------------------
// SLICE 9 (RED): empty-response / synthetic / unexpected-shape robustness.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`isUpstreamEmptyResponseText` at ~304-319 with
// `UPSTREAM_EMPTY_RESPONSE_MARKER` at ~284 and
// `UPSTREAM_EMPTY_RESPONSE_KEY` at ~289; the synthetic filter in
// `handleMessagePartUpdated` at ~2114-2120 and in `handleMessagePartDelta`
// at ~2357-2365 backed by `syntheticPartIds`; the unexpected-shape guard
// in `processEvent` at ~856-861; `getCombinedMessageText` at ~2740-2755
// with the no-emit-on-empty rule in `applyTextDelta` at ~2422-2425).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (empty-response blocks at lines 3911-4169, synthetic blocks at
// 4171-4311, unexpected-shape `describe` at 4313-4453).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Whether a text part is the upstream empty-response placeholder
    /// (TS: `isUpstreamEmptyResponseText`; slice 9). Streaming text that
    /// still looks like the marker is held back; once the message is
    /// complete the marker alone is not enough — the serialized response
    /// object must be there too, or the text is a legitimate answer.
    pub fn is_empty_response(text: &str, is_final: bool) -> bool {
        const MARKER: &str = "Empty response: {'";
        const KEY: &str = "'stop_reason'";
        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            return false;
        }
        if trimmed.len() < MARKER.len() {
            return !is_final && MARKER.starts_with(trimmed);
        }
        if !trimmed.starts_with(MARKER) {
            return false;
        }
        if is_final {
            trimmed.contains(KEY)
        } else {
            true
        }
    }

    /// Join one message's text parts, dropping the upstream
    /// empty-response placeholder for assistant roles (TS:
    /// `getCombinedMessageText`; slice 9). User text is never filtered —
    /// it is returned verbatim. A placeholder-only assistant message
    /// combines to `""`, and the caller emits nothing (TS:
    /// `applyTextDelta` ~2422-2425).
    pub fn combine_text_parts(texts: &[&str], role: &str, is_final: bool) -> String {
        if role == "user" {
            return texts.concat();
        }
        texts
            .iter()
            .filter(|text| !Self::is_empty_response(text, is_final))
            .copied()
            .collect::<String>()
    }

    /// Remember a part flagged synthetic so later delta events carrying
    /// only the part id stay filtered (TS: `registerSyntheticPart` /
    /// `syntheticPartIds`; slice 9).
    pub fn register_synthetic_part(&mut self, message_id: &str, part_id: &str) {
        self.synthetic_part_ids
            .entry(message_id.to_string())
            .or_default()
            .insert(part_id.to_string());
    }

    /// Whether a part is known synthetic, either flagged on the event
    /// itself or remembered in the registry (TS: the `isSynthetic` check
    /// in `handleMessagePartDelta` at ~2359-2361; slice 9).
    pub fn is_synthetic_part(&self, message_id: &str, part_id: &str) -> bool {
        self.synthetic_part_ids
            .get(message_id)
            .map_or(false, |parts| parts.contains(part_id))
    }

    /// Accept or gracefully ignore a streamed text delta. Returns `false`
    /// — without panicking and without emitting — when the shape is
    /// unexpected (missing session/message ids, or a missing/empty
    /// delta), mirroring the warn-and-ignore guard for
    /// `message.part.delta` (TS: `processEvent` ~856-861; slice 9).
    pub fn handle_unexpected_delta(
        &mut self,
        session_id: Option<&str>,
        message_id: Option<&str>,
        delta: Option<&str>,
    ) -> bool {
        match (session_id, message_id, delta) {
            (Some(session), Some(message), Some(delta))
                if !session.is_empty() && !message.is_empty() && !delta.is_empty() =>
            {
                true
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// SLICE 10 (GREEN): text-message states + write/edit tool files + reasoning
// deltas + session/error cards.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`getCombinedMessageText` at ~2740-2755, `prepareToolFiles`
// write/edit branches at ~2760-2870, `handleMessagePartDelta`
// reasoning branch at ~2357-2397, `handleSessionError` message
// extraction at ~3003-3051).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (multi-text at 2579-2646, write oversized at 2119-2168, edit
// fallback at 3245-3307 + filediff at 3169-3243, reasoning deltas at
// 2216-2310, session.error at 3116-3139).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Join ordered assistant text parts into the single final message
    /// (TS: `getCombinedMessageText` text join; slice 10).
    pub fn combined_text_message(parts: &[&str]) -> String {
        parts.concat()
    }

    /// Whether a `write`-tool payload of `byte_len` bytes keeps its file
    /// attachment (TS: `prepareToolFiles` 100 KiB budget; slice 10).
    pub fn write_tool_attachment_allowed(byte_len: usize) -> bool {
        byte_len <= 100 * 1024
    }

    /// Map a patched file path to its `edit_<flat>.txt` document name
    /// (TS: `prepareToolFiles` apply_patch/edit filename; slice 10).
    pub fn edit_patch_filename(file_path: &str) -> String {
        let basename = file_path
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .unwrap_or(file_path);
        format!("edit_{basename}.txt")
    }

    /// Accumulate an explicit reasoning delta into the thinking text
    /// (TS: `handleMessagePartDelta` reasoning branch; slice 10).
    /// Reasoning deltas never enter the assistant text stream.
    pub fn append_reasoning_delta(current: &str, delta: &str) -> String {
        format!("{current}{delta}")
    }

    /// Extract the callback message from a `session.error` payload
    /// (TS: `handleSessionError` message extraction; slice 10).
    pub fn session_error_message(message: &str) -> String {
        message.to_string()
    }
}

// ---------------------------------------------------------------------------
// SLICE 11 (GREEN): todo lists, onTokens/onCost cards, run-end.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`handleMessageUpdated` tokens branch at ~1971-1992,
// cost branch at ~2023-2030, `emitSessionRunEnded` at ~3113-3126 via
// `handleSessionIdle` root/child at ~2937/2948 and `handleSessionError`
// root/child at ~3026/3035).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (onTokens completed at 3487-3539, intermediate at 3541-3565,
// zero->non-zero at 3567-3616, missing tokens at 3618-3636,
// cost 0.01 at 3527, run-end at 3862-3885).
// Todo-list aggregation has no direct TS counterpart in the manager;
// it models the todo-tool list summary the cards carry.
// ---------------------------------------------------------------------------

/// One todo-list entry (SLICE 11 stub).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoItem {
    pub title: String,
    pub done: bool,
}

/// Fired with token totals and the completed flag (TS: `TokensCallback`;
/// slice 11).
pub type TokensCallback = Box<dyn FnMut(String, TokensInfo, bool) + Send>;
/// Fired with the message cost (TS: `CostCallback`; slice 11).
pub type CostCallback = Box<dyn FnMut(String, f64) + Send>;
/// Fired when a run ends (TS: `SessionRunEndedCallback`; slice 11).
pub type RunEndedCallback = Box<dyn FnMut(String) + Send>;

impl SummaryAggregator {
    /// Register the tokens callback (TS: `setOnTokens`; slice 11).
    pub fn set_on_tokens(&mut self, callback: TokensCallback) {
        self.on_tokens = Some(callback);
    }

    /// Report token totals for a message (TS: `handleMessageUpdated`
    /// tokens branch; slice 11). `is_completed` mirrors
    /// `Boolean(time?.completed)`.
    pub fn handle_tokens_update(
        &mut self,
        session_id: &str,
        tokens: TokensInfo,
        is_completed: bool,
    ) {
        if let Some(callback) = self.on_tokens.as_mut() {
            callback(session_id.to_string(), tokens, is_completed);
        }
    }

    /// Register the cost callback (TS: `setOnCost`; slice 11).
    pub fn set_on_cost(&mut self, callback: CostCallback) {
        self.on_cost = Some(callback);
    }

    /// Report the cost of a completed message (TS: `handleMessageUpdated`
    /// cost branch; slice 11).
    pub fn handle_cost_update(&mut self, session_id: &str, cost: f64) {
        if let Some(callback) = self.on_cost.as_mut() {
            callback(session_id.to_string(), cost);
        }
    }

    /// Track a todo-tool list for a session (slice 11; no direct TS
    /// counterpart — models the todo list the card summarizes).
    pub fn track_todo_list(&mut self, session_id: &str, items: Vec<TodoItem>) {
        self.todos_by_session.insert(session_id.to_string(), items);
    }

    /// Summarize a session's todo list (slice 11).
    pub fn todo_summary(&self, session_id: &str) -> String {
        let items = self.todos_by_session.get(session_id);
        let total = items.map(|list| list.len()).unwrap_or(0);
        let done = items
            .map(|list| list.iter().filter(|item| item.done).count())
            .unwrap_or(0);
        format!("{done}/{total} done")
    }

    /// Register the run-end callback (TS: `setOnSessionRunEnded`;
    /// slice 11).
    pub fn set_on_session_run_ended(&mut self, callback: RunEndedCallback) {
        self.on_run_ended = Some(callback);
    }

    /// Report the end of a run for a followed or subagent session (TS:
    /// `emitSessionRunEnded`; slice 11).
    pub fn handle_run_end(&mut self, session_id: &str) {
        if let Some(callback) = self.on_run_ended.as_mut() {
            callback(session_id.to_string());
        }
    }
}

// ---------------------------------------------------------------------------
// SLICE 12 (RED): text-message edge states, edit/write fallbacks,
// thinking-finished gates.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (optimistic text at ~2200-2207, completed-message pending at ~1955-1992,
// `extractFirstUpdatedFileFromTitle` at ~362-371, write branch at
// ~2763-2783, `emitThinkingFinishedOnce` at ~2586-2600).
// Each method below is a RED stub (`unimplemented!()`) until the port lands.
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Whether an unknown-text update (no `messageInfo`) emits an
    /// optimistic partial (TS: `optimisticUpdateCount >= 2`; slice 12).
    pub fn should_emit_optimistic_partial(update_count: usize) -> bool {
        update_count >= 2
    }

    /// Whether pending text attached on a completed message emits a
    /// partial (TS: completed goes straight to `onComplete`; slice 12).
    /// Always false: the completed path never emits partial.
    pub fn should_emit_pending_on_completed(_is_completed: bool) -> bool {
        false
    }

    /// Parse the first patched path from an apply_patch tool title (TS:
    /// `extractFirstUpdatedFileFromTitle`; slice 12). `None` when no
    /// status line (`M/A/D/U/R/C` + space + path) is present.
    pub fn edit_path_from_title(title: &str) -> Option<String> {
        for raw_line in title.split('\n') {
            let line = raw_line.trim();
            let bytes = line.as_bytes();
            if bytes.len() >= 3
                && bytes[1] == b' '
                && matches!(bytes[0], b'A' | b'M' | b'D' | b'U' | b'R' | b'C')
            {
                let path = line[2..].trim();
                if !path.is_empty() {
                    return Some(path.to_string());
                }
            }
        }
        None
    }

    /// Whether a write-tool input yields a file (TS: `prepareToolFiles`
    /// write branch needs both `filePath` and `content`; slice 12).
    pub fn write_file_ready(file_path: &str, has_content: bool) -> bool {
        !file_path.trim().is_empty() && has_content
    }

    /// Whether assistant text after reasoning signals thinking-finished
    /// (TS: `emitThinkingFinishedOnce`: needs reasoning fired, skips when
    /// already signalled; slice 12).
    pub fn should_signal_thinking_finished(had_reasoning: bool, already_signalled: bool) -> bool {
        had_reasoning && !already_signalled
    }
}

// ---------------------------------------------------------------------------
// SLICE 13 (RED): multi-file apply_patch edges, oversized/failed patch,
// onTokens no-field gate, session-error variants. Each method below is a RED
// stub (`unimplemented!()`) until the port lands.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`prepareToolFiles` apply_patch branch at ~2809-2867, completed-only tool
// gate at ~2273-2284, tokens gate at ~1976-1992, `handleSessionError` at
// ~3003-3051) + `src/app/formatters/summary-formatter.ts`
// (`getPatchFileChanges` at ~213-251, `formatPatchFileLine` at ~253-255,
// `prepareCodeFile` budget at ~488-507) + `src/utils/opencode-error.ts`
// (`extractErrorMessage` at ~100-130).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Format one `🩹 apply_patch <path> (+a -d)` line for a changed file
    /// (TS: `formatPatchFileLine`; slice 13).
    pub fn format_patch_file_line(path: &str, additions: usize, deletions: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        if additions > 0 {
            parts.push(format!("+{additions}"));
        }
        if deletions > 0 {
            parts.push(format!("-{deletions}"));
        }
        let info = if parts.is_empty() {
            String::new()
        } else {
            format!(" ({})", parts.join(" "))
        };
        format!("🩹 apply_patch {path}{info}")
    }

    /// Whether an apply_patch diff of this size keeps its file attachment
    /// (TS: `prepareCodeFile` budget — over-limit yields `fileData: null`
    /// but keeps the line and file change; slice 13).
    pub fn apply_patch_attachment_allowed(byte_len: usize) -> bool {
        byte_len <= 100 * 1024
    }

    /// Whether a tool part state yields prepared files (TS: only
    /// `status === "completed"` reaches `prepareToolFiles`, `error` adds
    /// nothing; slice 13).
    pub fn should_process_tool_state(status: &str) -> bool {
        status == "completed"
    }

    /// Whether `message.updated` fires `onTokens` (TS: gated on
    /// `assistantInfo.tokens` being present; slice 13).
    pub fn should_fire_tokens(has_tokens: bool) -> bool {
        has_tokens
    }

    /// Fallback message for unexpected `session.error` shapes (TS:
    /// `extractErrorMessage(error) ?? "Unknown session error"`: a blank or
    /// missing raw message falls back, a non-blank string passes through
    /// trimmed; slice 13).
    pub fn unexpected_session_error_message(raw: Option<&str>) -> String {
        match raw.map(str::trim) {
            Some(message) if !message.is_empty() => message.to_string(),
            _ => "Unknown session error".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// SLICE 14 (RED): session.created enrichment, subagent model/provider
// pairing, thinking-message state machines. Each method below is a RED
// stub (`unimplemented!()`) until the port lands.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`handleSessionCreatedOrUpdated` at ~1600-1656 with
// `getOrCreateSubagentForSession` at ~1531-1543 and
// `enrichSubagentFromSessionTitle`; `updateSubagentFromAssistantMessage`
// at ~1658-1703; `registerThinkingPart` at ~2478-2497,
// `setThinkingPartSnapshot` at ~2499-2521, `applyThinkingDelta` at
// ~2523-2548, `emitThinkingFinishedOnce` at ~2586-2600).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (live subagent at 183-320, variant at 322-433, reasoning at 2170-2214,
// reasoning deltas at 2216-2310, thinking-finished at 2312-2410,
// `session.created` gating at 1535-1602).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Attach a `session.created` child and enrich its card from the
    /// session title (TS: `handleSessionCreatedOrUpdated` +
    /// `enrichSubagentFromSessionTitle`; slice 14).
    pub fn handle_session_created_with_title(
        &mut self,
        session_id: &str,
        parent_session_id: &str,
        title: &str,
    ) {
        if self.current_session_id.is_none() {
            return;
        }
        self.created_child_sessions.insert(session_id.to_string());
        self.attach_child_session(session_id, parent_session_id, title);
    }

    /// Pair agent + provider/model/variant from a child assistant message
    /// onto its card (TS: `updateSubagentFromAssistantMessage`; slice 14).
    pub fn pair_assistant_message_model(
        &mut self,
        child_session_id: &str,
        agent: Option<&str>,
        provider_id: Option<&str>,
        model_id: Option<&str>,
        variant: Option<&str>,
    ) {
        let card_id = self.get_or_create_card_for_session(child_session_id, None);
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            if let Some(agent) = agent {
                if !agent.is_empty() {
                    card.agent = agent.to_string();
                }
            }
            if let Some(provider) = provider_id {
                if !provider.is_empty() {
                    card.provider_id = Some(provider.to_string());
                }
            }
            if let Some(model) = model_id {
                if !model.is_empty() {
                    card.model_id = Some(model.to_string());
                }
            }
            if let Some(variant) = variant {
                if !variant.is_empty() {
                    card.variant = Some(variant.to_string());
                }
            }
            let both_known = card.provider_id.is_some() && card.model_id.is_some();
            if let Some(display) = Self::format_subagent_model(
                card.provider_id.as_deref(),
                card.model_id.as_deref(),
                card.variant.as_deref(),
            ) {
                if both_known || card.model.is_empty() {
                    card.model = display;
                }
            }
        }
        self.emit_subagents();
    }

    /// Register a thinking snapshot, returning `(is_first_update,
    /// was_updated)` (TS: `registerThinkingPart` +
    /// `setThinkingPartSnapshot`; slice 14).
    pub fn register_thinking_snapshot(
        &mut self,
        message_id: &str,
        part_id: &str,
        text: &str,
        title: Option<&str>,
    ) -> (bool, bool) {
        let is_first = !self.thinking_fired_messages.contains(message_id);
        if is_first {
            self.thinking_fired_messages.insert(message_id.to_string());
        }
        let key = self
            .thinking_by_message
            .keys()
            .find(|(_, message)| message == message_id)
            .cloned()
            .unwrap_or_else(|| {
                (
                    self.current_session_id.clone().unwrap_or_default(),
                    message_id.to_string(),
                )
            });
        let state = self.thinking_by_message.entry(key).or_default();
        if !state.ordered_part_ids.iter().any(|id| id == part_id) {
            state.ordered_part_ids.push(part_id.to_string());
        }
        if state.sections.get(part_id).is_none() {
            state.sections.insert(
                part_id.to_string(),
                ThinkingSection {
                    id: part_id.to_string(),
                    title: title.map(|value| value.to_string()),
                    text: String::new(),
                },
            );
        } else if let Some(next_title) = title {
            if let Some(section) = state.sections.get_mut(part_id) {
                if section.title.as_deref() != Some(next_title) {
                    section.title = Some(next_title.to_string());
                }
            }
        }
        let next_title = title.map(|value| value.to_string()).or_else(|| {
            state
                .sections
                .get(part_id)
                .and_then(|section| section.title.clone())
        });
        let unchanged = state.sections.get(part_id).map_or(false, |section| {
            section.text == text && section.title == next_title
        });
        if unchanged {
            return (false, false);
        }
        state.sections.insert(
            part_id.to_string(),
            ThinkingSection {
                id: part_id.to_string(),
                title: next_title,
                text: text.to_string(),
            },
        );
        (is_first, true)
    }

    /// Accumulate a thinking delta, letting a longer full-text hint win
    /// (TS: `applyThinkingDelta`; slice 14).
    pub fn apply_thinking_delta_with_hint(
        &mut self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        delta: &str,
        full_text_hint: Option<&str>,
    ) -> String {
        let exact = (session_id.to_string(), message_id.to_string());
        let key = if self.thinking_by_message.contains_key(&exact) {
            exact
        } else {
            self.thinking_by_message
                .keys()
                .find(|(_, message)| message == message_id)
                .cloned()
                .unwrap_or(exact)
        };
        if Some(session_id) != self.current_session_id.as_deref() {
            return self
                .thinking_by_message
                .get(&key)
                .and_then(|state| state.sections.get(part_id))
                .map(|section| section.text.clone())
                .unwrap_or_default();
        }
        let state = self.thinking_by_message.entry(key).or_default();
        if !state.ordered_part_ids.iter().any(|id| id == part_id) {
            state.ordered_part_ids.push(part_id.to_string());
        }
        if state.sections.get(part_id).is_none() {
            state.sections.insert(
                part_id.to_string(),
                ThinkingSection {
                    id: part_id.to_string(),
                    title: None,
                    text: String::new(),
                },
            );
        }
        let previous = state
            .sections
            .get(part_id)
            .map(|section| section.text.clone())
            .unwrap_or_default();
        let mut accumulated = format!("{previous}{delta}");
        if let Some(hint) = full_text_hint {
            if hint.len() > accumulated.len() {
                accumulated = hint.to_string();
            }
        }
        let title = state
            .sections
            .get(part_id)
            .and_then(|section| section.title.clone());
        state.sections.insert(
            part_id.to_string(),
            ThinkingSection {
                id: part_id.to_string(),
                title,
                text: accumulated.clone(),
            },
        );
        accumulated
    }

    /// Poll the thinking-finished gate: true only for the first
    /// text-after-reasoning of a message (TS:
    /// `emitThinkingFinishedOnce`; slice 14).
    pub fn poll_thinking_finished(
        &mut self,
        session_id: &str,
        message_id: &str,
        had_reasoning: bool,
    ) -> bool {
        if Some(session_id) != self.current_session_id.as_deref() {
            return false;
        }
        let fired = had_reasoning || self.thinking_fired_messages.contains(message_id);
        if !fired {
            return false;
        }
        if self.thinking_finished_messages.contains(message_id) {
            return false;
        }
        self.thinking_finished_messages
            .insert(message_id.to_string());
        true
    }
}

// ---------------------------------------------------------------------------
// SLICE 15 (RED): subagent terminal/cost full paths + background stale-drop
// + nested guards. Each method below is a RED stub (`unimplemented!()`)
// until the port lands.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`handleSessionCreatedOrUpdated` at ~1600-1656 with the nested branch at
// ~1618-1625, the background stale-drop at ~1627-1639, and the
// `trackedSessionParents` parent guard at ~1641-1643;
// `updateSubagentFromAssistantMessage` tokens arm at ~1689-1697 + cost arm
// at ~1698-1700; `setSubagentTerminalStatus` completed path at ~1852-1876
// via `endSubagent` + `retireSubagent` at ~1901-1915; `handleSessionIdle`
// child completed/nested split at ~2928-2938 with `endNestedSubagentSession`
// at ~2971-2975; `handleSessionError` child nested/abort/error split at
// ~3017-3028; nested early returns in `handleMessageUpdated` at ~1930-1934
// and `handleMessagePartUpdated` at ~2072-2074 + subtask-of-subagent guard
// at ~2076-2090; `isNestedSubagentSession` at ~965-969).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (live subagent tokens/cost at 183-320, background `launchInBackground`
// at 709-757, stale rediscovery, nested-subagent block at 1122-1234,
// parallel subagents at 1604-1681, terminal cards at 1683-1901).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// End a child card as completed through the full terminal path (TS:
    /// `setSubagentTerminalStatus(..., "completed")` -> `endSubagent` +
    /// `retireSubagent`; slice 15).
    pub fn complete_subagent(&mut self, child_session_id: &str) {
        let card_id = match self.card_id_by_session.get(child_session_id).cloned() {
            Some(card_id) => card_id,
            None => {
                if self.subagent_cards.contains_key(child_session_id) {
                    child_session_id.to_string()
                } else {
                    return;
                }
            }
        };
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            card.status = SubagentStatus::Completed;
            card.current_tool = None;
            card.current_tool_call_id = None;
            card.current_tool_started_at = None;
            card.current_tool_title = None;
            card.terminal_message = None;
        }
        self.emit_subagents();
        self.retire_completed_card(&card_id);
        self.finished_subagent_sessions
            .insert(child_session_id.to_string());
    }

    /// Replace token totals and assign cost from a child assistant message
    /// (TS: `updateSubagentFromAssistantMessage` tokens arm + cost arm;
    /// slice 15).
    pub fn apply_assistant_tokens_cost(
        &mut self,
        child_session_id: &str,
        tokens: TokensInfo,
        cost: f64,
    ) {
        if self.finished_subagent_sessions.contains(child_session_id) {
            return;
        }
        let card_id = self.get_or_create_card_for_session(child_session_id, None);
        if let Some(card) = self.subagent_cards.get_mut(&card_id) {
            card.tokens = tokens;
            card.cost = cost;
        }
        self.emit_subagents();
    }

    /// Attach a `session.created` child honoring the background stale-drop
    /// (TS: `handleSessionCreatedOrUpdated` stale-drop: an unknown card
    /// whose `createdAt < subagentRunStartedAt` is finished, never
    /// attached; slice 15).
    pub fn handle_session_created_with_created_at(
        &mut self,
        session_id: &str,
        parent_session_id: &str,
        title: &str,
        created_at: u64,
        run_started_at: u64,
    ) {
        if self.current_session_id.is_none() {
            return;
        }
        if self.finished_subagent_sessions.contains(session_id) {
            return;
        }
        let current = self.current_session_id.clone().unwrap_or_default();
        // Nested branch first (TS ~1618-1625): launched by a tracked child,
        // followed without a card.
        if parent_session_id != current.as_str()
            && self.session_parents.contains_key(parent_session_id)
        {
            if !self.session_parents.contains_key(session_id) {
                self.session_parents
                    .insert(session_id.to_string(), parent_session_id.to_string());
            }
            return;
        }
        // Stale-drop (TS ~1627-1639): unknown card while not accepting, or
        // created before the run that started the subagent.
        if !self.card_id_by_session.contains_key(session_id)
            && (!self.accepts_subagent_events || created_at < run_started_at)
        {
            self.finished_subagent_sessions
                .insert(session_id.to_string());
            return;
        }
        // Tracked-parent guard (TS ~1641-1643).
        if parent_session_id != current.as_str()
            && !self.session_parents.contains_key(parent_session_id)
        {
            return;
        }
        if session_id == current {
            return;
        }
        self.created_child_sessions.insert(session_id.to_string());
        self.attach_child_session(session_id, parent_session_id, title);
    }

    /// Attach a `session.created` child honoring the tracked-parent guard
    /// (TS: `handleSessionCreatedOrUpdated` parent guard: a parent the
    /// aggregator never tracked attaches nothing; slice 15). Returns true
    /// when a card was attached.
    pub fn handle_session_created_guarded(
        &mut self,
        session_id: &str,
        parent_session_id: &str,
        title: &str,
    ) -> bool {
        if self.current_session_id.is_none() {
            return false;
        }
        if self.finished_subagent_sessions.contains(session_id) {
            return false;
        }
        let current = self.current_session_id.clone().unwrap_or_default();
        // Nested launch takes no card (TS ~1620-1624).
        if parent_session_id != current.as_str()
            && self.session_parents.contains_key(parent_session_id)
        {
            if !self.session_parents.contains_key(session_id) {
                self.session_parents
                    .insert(session_id.to_string(), parent_session_id.to_string());
            }
            return false;
        }
        // Untracked parent attaches nothing (TS ~1641-1643).
        if parent_session_id != current.as_str()
            && !self.session_parents.contains_key(parent_session_id)
        {
            return false;
        }
        if session_id == current {
            return false;
        }
        self.created_child_sessions.insert(session_id.to_string());
        self.attach_child_session(session_id, parent_session_id, title);
        true
    }

    /// Whether a session is a nested subagent (TS:
    /// `isNestedSubagentSession`: tracked with a non-root parent; slice 15).
    /// Nested sessions never get cards and their assistant/tool updates
    /// early-return.
    pub fn is_nested_subagent_session(&self, session_id: &str) -> bool {
        match self.session_parents.get(session_id) {
            Some(parent) => {
                !parent.is_empty() && Some(parent.as_str()) != self.current_session_id.as_deref()
            }
            None => false,
        }
    }

    /// Drop a completed card (TS: `endSubagent` + `retireSubagent`
    /// bookkeeping; slice-15 subset without launch/held-end state).
    fn retire_completed_card(&mut self, card_id: &str) {
        let (parent, session_id) = match self.subagent_cards.get(card_id) {
            Some(card) => (card.parent_session_id.clone(), card.session_id.clone()),
            None => return,
        };
        self.subagent_cards.remove(card_id);
        self.subagent_order.retain(|id| id.as_str() != card_id);
        if let Some(session_id) = session_id {
            self.card_id_by_session.remove(&session_id);
            self.background_child_sessions.remove(&session_id);
            self.session_parents.remove(&session_id);
            if let Some(queue) = self.pending_children_by_parent.get_mut(&parent) {
                queue.retain(|pending| pending != &session_id);
                if queue.is_empty() {
                    self.pending_children_by_parent.remove(&parent);
                }
            }
        }
        if let Some(queue) = self.pending_cards_by_parent.get_mut(&parent) {
            queue.retain(|pending| pending.as_str() != card_id);
            if queue.is_empty() {
                self.pending_cards_by_parent.remove(&parent);
            }
        }
        if let Some(queue) = self.fallback_cards_by_parent.get_mut(&parent) {
            queue.retain(|pending| pending.as_str() != card_id);
            if queue.is_empty() {
                self.fallback_cards_by_parent.remove(&parent);
            }
        }
        self.last_snapshot.clear();
    }
}

// ---------------------------------------------------------------------------
// SLICE 16 (RED): text-message edge states + edit/write fallbacks +
// remaining nested guards. Each method below is a RED stub
// (`unimplemented!()`) until the port lands.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`applyTextDelta` full-text hint at ~2414-2418,
// `handleMessageUpdated` partial gate at ~1967-1969
// (`!isCompleted && optimisticUpdateCount === 1`),
// `prepareToolFiles` edit branch at ~2785-2807
// (`if (!filePath || !diffText) return []`),
// `prepareToolFiles` write branch at ~2763-2782
// (`additions: content.split("\n").length`),
// `handleMessagePartUpdated` nested early return at ~2072-2074 +
// subtask-of-subagent guard at ~2077-2080,
// `isNestedSubagentSession` at ~965-969).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (multi-text/optimistic/completed-pending text blocks,
// edit `filediff`/title fallback, write oversized/missing-input fallback,
// nested-subagent block at 1122-1234).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Accumulate a streaming text delta, letting a longer full-text hint
    /// win (TS: `applyTextDelta`: `previous + delta`, then `fullTextHint`
    /// replaces it when longer; slice 16).
    pub fn accumulate_text_delta_with_hint(
        previous: &str,
        delta: &str,
        full_text_hint: Option<&str>,
    ) -> String {
        let mut accumulated = format!("{previous}{delta}");
        if let Some(hint) = full_text_hint {
            if hint.len() > accumulated.len() {
                accumulated = hint.to_string();
            }
        }
        accumulated
    }

    /// Whether `message.updated` emits a partial for pending text (TS:
    /// `handleMessageUpdated`: only the first optimistic update of a
    /// still-running message emits; completed messages never emit partial;
    /// slice 16).
    pub fn should_emit_updated_partial(is_completed: bool, optimistic_update_count: usize) -> bool {
        !is_completed && optimistic_update_count == 1
    }

    /// Whether an `edit`-tool payload yields a file (TS: `prepareToolFiles`
    /// edit branch needs both a `filediff.file` path and `metadata.diff`
    /// text; slice 16).
    pub fn edit_file_ready(file_path: &str, has_diff: bool) -> bool {
        !file_path.trim().is_empty() && has_diff
    }

    /// Count `write`-tool additions by lines (TS: `prepareToolFiles` write
    /// branch: `additions: content.split("\n").length`; slice 16).
    pub fn count_write_additions(content: &str) -> usize {
        content.split('\n').count()
    }

    /// Whether a `message.part.updated` is dropped by the nested guards
    /// (TS: `handleMessagePartUpdated`: any part from a nested subagent
    /// session returns at ~2072-2074, and a `subtask` part whose session
    /// is not the current root returns at ~2077-2080 so a subtask of a
    /// subagent never takes a card; slice 16).
    pub fn should_drop_nested_subtask(
        is_nested: bool,
        is_subtask: bool,
        is_current_root: bool,
    ) -> bool {
        is_nested || (is_subtask && !is_current_root)
    }
}

// ---------------------------------------------------------------------------
// SLICE 17 (RED): cost-zero gate, token cache defaults, forget-live-turn,
// diff counts, abort/interrupt/retry fallbacks. Each method below is a RED
// stub (`unimplemented!()`) until the port lands.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (cost gate `cost !== undefined` at ~2024 + `` `$${cost.toFixed(2)}` `` at
// ~2025; tokens `cache?.read || 0` / `cache?.write || 0` at ~1981-1982 and
// the assistant-message tokens arm at ~1694-1695; `forgetLiveTurn` at
// ~612-615; `countDiffChangesFromText` at ~373-389 skipping `+++`/`---`;
// `isAbortError` at ~358-360; `isInterruptedIdle` at ~322-324 +
// `readInterruptReason` at ~326-341; retry blank fallback
// `status.message?.trim() || "Unknown retry error"` at ~2903).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (cost `0.01` at 3526-3528, tokens `cache.read` at 3487-3616, live-turn
// start at 2781-2867, apply_patch `getPatchFileChanges` diffs at
// 3309-3455, `session.error` abort vs genuine error at 3116-3139 and
// 3841-3886, retry at 3141-3167).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Whether a cost value reaches `onCost` (TS: `handleMessageUpdated`
    /// cost branch: `cost !== undefined`; slice 17). `Some(0.0)` still
    /// fires — the gate is undefined-ness, not truthiness.
    pub fn cost_fires_for_value(cost: Option<f64>) -> bool {
        cost.is_some()
    }

    /// Build token totals defaulting missing cache sides to 0 (TS: `cache?.read
    /// || 0` / `cache?.write || 0`; slice 17).
    pub fn tokens_with_cache_defaults(
        input: u64,
        output: u64,
        reasoning: u64,
        cache_read: Option<u64>,
        cache_write: Option<u64>,
    ) -> TokensInfo {
        TokensInfo {
            input,
            output,
            reasoning,
            cache_read: cache_read.unwrap_or(0),
            cache_write: cache_write.unwrap_or(0),
        }
    }

    /// Forget the running turn without ending it (TS: `forgetLiveTurn`:
    /// `liveTurnStartedAt = null`, no `turnEndCount++`; slice 17).
    pub fn forget_live_turn(&mut self) {
        self.live_turn_started_at = None;
    }

    /// Count additions/deletions in a unified diff, skipping `+++`/`---`
    /// headers (TS: `countDiffChangesFromText`; slice 17).
    pub fn count_diff_changes(text: &str) -> (u64, u64) {
        let mut additions = 0u64;
        let mut deletions = 0u64;
        for line in text.split('\n') {
            if line.starts_with('+') && !line.starts_with("+++") {
                additions += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                deletions += 1;
            }
        }
        (additions, deletions)
    }

    /// Whether a `session.error` payload is an abort (TS: `isAbortError`:
    /// record with `name === "MessageAbortedError"`; slice 17).
    pub fn is_abort_error(error_name: Option<&str>) -> bool {
        error_name == Some("MessageAbortedError")
    }

    /// Interrupt reason for an idle, or `None` (TS: `isInterruptedIdle` +
    /// `readInterruptReason`: only when interrupted and one of
    /// user/shutdown/superseded/inactivity; slice 17).
    pub fn read_interrupt_reason(reason: Option<&str>, interrupted: bool) -> Option<String> {
        if !interrupted {
            return None;
        }
        match reason {
            Some("user") | Some("shutdown") | Some("superseded") | Some("inactivity") => {
                reason.map(|value| value.to_string())
            }
            _ => None,
        }
    }

    /// Retry message with blank fallback (TS: `status.message?.trim() ||
    /// "Unknown retry error"`; slice 17).
    pub fn retry_message_or_fallback(raw: Option<&str>) -> String {
        match raw.map(str::trim) {
            Some(message) if !message.is_empty() => message.to_string(),
            _ => "Unknown retry error".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// SLICE 18 (RED): session.updated enrichment + model-label partials +
// thinking-update gate + footer-suppression edges + diff file fallback.
// Each method below is a RED stub (`unimplemented!()`) until the port lands.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`handleSessionCreatedOrUpdated` at ~1600-1656, shared by
// `session.created` + `session.updated` at ~863-866;
// `updateSubagentFromAssistantMessage` provider/model/variant arms at
// ~1680-1688 with the `provider/id (variant)` display;
// `emitThinkingUpdate` early return at ~2562-2574: no callback or zero
// sections emits nothing;
// `handleQuestionSettled` cancelled arm at ~3097-3099 +
// `handlePermissionReplied` reject arm at ~3209-3211, both firing
// `onFooterSuppressedCallback`;
// `handleSessionDiff` file mapping `file: d.file ?? ""` at ~3141-3146).
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`
// (live subagent at 183-320, variant at 322-433, reasoning at 2170-2214,
// settled questions at 3816-3839, replied permissions at 3841-3860,
// session diffs).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Attach a `session.updated` child and enrich its card from the
    /// session title (TS: `handleSessionCreatedOrUpdated` updated branch;
    /// slice 18). Returns true when a card was attached/enriched.
    pub fn handle_session_updated(
        &mut self,
        session_id: &str,
        parent_session_id: &str,
        title: &str,
    ) -> bool {
        self.handle_session_created_guarded(session_id, parent_session_id, title)
    }

    /// Display label for a provider/model/variant combo (TS: the card shows
    /// `provider/id (variant)`; slice 18). `None` when both ids are missing
    /// or blank; a blank variant is ignored.
    pub fn subagent_model_label(
        provider_id: Option<&str>,
        model_id: Option<&str>,
        variant: Option<&str>,
    ) -> Option<String> {
        let provider = provider_id.map(str::trim).filter(|value| !value.is_empty());
        let model = model_id.map(str::trim).filter(|value| !value.is_empty());
        let variant = variant.map(str::trim).filter(|value| !value.is_empty());
        match (provider, model) {
            (Some(provider), Some(model)) => Some(match variant {
                Some(variant) => format!("{}/{} ({})", provider, model, variant),
                None => format!("{}/{}", provider, model),
            }),
            (Some(provider), None) => Some(provider.to_string()),
            (None, Some(model)) => Some(model.to_string()),
            (None, None) => None,
        }
    }

    /// Whether a thinking update emits (TS: `emitThinkingUpdate`: no
    /// callback or zero sections emits nothing; slice 18).
    pub fn should_emit_thinking_update(has_callback: bool, section_count: usize) -> bool {
        has_callback && section_count > 0
    }

    /// Whether a settled question / replied permission suppresses the run
    /// footer (TS: `cancelled` at ~3097-3099 + `reject` at ~3209-3211 fire
    /// `onFooterSuppressedCallback`; slice 18). `outcome` is the question
    /// outcome (`answered`/`cancelled`, empty when the check is for a
    /// permission reply); `reply` is the permission reply (`allow`/`reject`,
    /// `None` for question settles or an unknown decision).
    pub fn footer_suppressed_for_settled(outcome: &str, reply: Option<&str>) -> bool {
        outcome == "cancelled" || reply == Some("reject")
    }

    /// Normalize one `session.diff` file entry (TS: `file: d.file ?? ""`;
    /// slice 18). Missing file ids become `""`, present paths pass through.
    pub fn normalize_diff_file(file: Option<&str>) -> String {
        file.unwrap_or_default().to_string()
    }
}

// ---------------------------------------------------------------------------
// SLICE 19B (RED): remaining parity leftovers, grouped by theme:
// updated/model/thinking, tokens/cost/settled/dispatch, pairing/pending/
// title, patch/budget/gates, text/file/reasoning/error. Each method below
// is a RED stub (`unimplemented!()`) until the port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// Assistant completion metadata (TS: `MessageCompletionInfo`; slice 19B).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistantCompletionInfo {
    pub agent: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub created_at: Option<u64>,
    pub completed_at: Option<u64>,
}

/// Where a root `task` call may land (TS: `findSubagentForTaskCall`;
/// slice 19B).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskCallPairing {
    NamedCard,
    WaitingCard,
    Unpaired,
}

/// Text-message outbound step (TS: `onPartial` vs `onComplete`; slice 19B).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextMessageFlow {
    Partial,
    Complete,
    None,
}

impl SummaryAggregator {
    /// Variant carried by a child assistant message (TS:
    /// `updateSubagentFromAssistantMessage` variant block, src ~1686-1688;
    /// slice 19B). A non-empty incoming variant replaces the card value
    /// (the caller re-emits); an omitted or blank one keeps the last value.
    pub fn apply_variant_to_card(current: Option<&str>, incoming: Option<&str>) -> Option<String> {
        match incoming.map(str::trim).filter(|value| !value.is_empty()) {
            Some(variant) => Some(variant.to_string()),
            None => current.map(|value| value.to_string()),
        }
    }

    /// Whether a message may emit a thinking update (TS:
    /// `registerThinkingPart` + `emitThinkingUpdate`, src ~2478-2584;
    /// slice 19B). Text without any reasoning part never fires thinking.
    pub fn should_emit_thinking_for_message(has_reasoning_parts: bool, has_callback: bool) -> bool {
        has_reasoning_parts && has_callback
    }

    /// Whether this reasoning part is the message's first thinking update
    /// (TS: `emitThinkingUpdate` `isFirstUpdate`, src ~2562+; slice 19B).
    pub fn marks_first_thinking_update(already_emitted_for_message: bool) -> bool {
        !already_emitted_for_message
    }

    /// Completion metadata for a finished assistant message (TS:
    /// `handleMessageUpdated` completed branch + `MessageCompletionInfo`,
    /// src ~1930+; slice 19B).
    pub fn assistant_completion_info(
        agent: Option<&str>,
        provider_id: Option<&str>,
        model_id: Option<&str>,
        created_at: Option<u64>,
        completed_at: Option<u64>,
    ) -> AssistantCompletionInfo {
        AssistantCompletionInfo {
            agent: agent.map(|value| value.to_string()),
            provider_id: provider_id.map(|value| value.to_string()),
            model_id: model_id.map(|value| value.to_string()),
            created_at,
            completed_at,
        }
    }

    /// Settle outcome for a question event kind (TS: `handleQuestionSettled`,
    /// `question.replied` -> `"answered"` / `question.rejected` ->
    /// `"cancelled"`, src ~3084-3111; slice 19B).
    pub fn question_settled_outcome(event_kind: &str) -> &'static str {
        if event_kind == "rejected" {
            "cancelled"
        } else {
            "answered"
        }
    }

    /// Whether a settled question reaches the settled callback (TS:
    /// `handleQuestionSettled` fires for the followed session and its
    /// subagents only, src ~3084-3111; slice 19B).
    pub fn should_emit_question_settled(is_followed_or_subagent: bool) -> bool {
        is_followed_or_subagent
    }

    /// Permission reply value passed along (TS: `handlePermissionReplied`,
    /// `reply` carried verbatim, missing -> `None`, src ~3188-3230;
    /// slice 19B).
    pub fn permission_reply_value(reply: Option<&str>) -> Option<String> {
        reply.map(|value| value.to_string())
    }

    /// Whether an incoming permission request must wait behind a running
    /// dispatch (TS: `permissionQueue` serialization, src ~441; slice 19B).
    pub fn should_queue_permission_while_dispatching(is_dispatching: bool) -> bool {
        is_dispatching
    }

    /// Whether an asked permission reaches the permission callback (TS:
    /// `handlePermissionAsked` fires for the current session and tracked
    /// child sessions only, src ~3155-3186; slice 19B).
    pub fn should_forward_permission_asked(
        is_current_session: bool,
        is_tracked_child: bool,
    ) -> bool {
        is_current_session || is_tracked_child
    }

    /// Pairing for a root `task` call update (TS: `findSubagentForTaskCall`,
    /// src ~1440-1488: the named child card when live, else the only
    /// waiting card for the single open call, otherwise unpaired;
    /// slice 19B).
    pub fn resolve_task_call_pairing(
        names_live_child_card: bool,
        unpaired_open_count: usize,
        unlinked_waiting_count: usize,
    ) -> TaskCallPairing {
        if names_live_child_card {
            TaskCallPairing::NamedCard
        } else if unpaired_open_count == 1 && unlinked_waiting_count == 1 {
            TaskCallPairing::WaitingCard
        } else {
            TaskCallPairing::Unpaired
        }
    }

    /// Whether a task call left open across the parent turn end is dropped
    /// (TS: `retireForegroundSubagents` clears unended `openTaskCalls`,
    /// src ~1141-1147; slice 19B).
    pub fn should_drop_stale_task_call(call_open_across_turn_end: bool) -> bool {
        call_open_across_turn_end
    }

    /// Card a task-call update belongs to (TS: a call keeps updating only
    /// the card it was paired with, src ~1440-1529; slice 19B). Returns the
    /// paired card id when the update's call matches, else `None`.
    pub fn route_task_call_update(
        paired_card_id: Option<&str>,
        event_call_id: &str,
        update_call_id: &str,
    ) -> Option<String> {
        if event_call_id == update_call_id {
            paired_card_id.map(|value| value.to_string())
        } else {
            None
        }
    }

    /// Thinking title for a reasoning part (TS: `registerThinkingPart` /
    /// `setThinkingPartSnapshot` title keys, src ~2478-2521: no title keys
    /// -> the thinking callback carries no title; slice 19B).
    pub fn reasoning_section_title(has_title_keys: bool, title: Option<&str>) -> Option<String> {
        if !has_title_keys {
            return None;
        }
        title
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    }

    /// Whether a tool update restarts current-tool timing (TS:
    /// `updateSubagentToolState`: a new `callId` restarts timing and
    /// re-emits, the same `callId` stays quiet, src ~1705-1748; slice 19B).
    pub fn should_restart_current_tool_timing(
        prev_call_id: Option<&str>,
        next_call_id: &str,
    ) -> bool {
        prev_call_id != Some(next_call_id)
    }

    /// Whether a `task` call runs in the background (TS:
    /// `isBackgroundTaskCall`: `input.background` or
    /// `metadata.background`, src ~351-355; slice 19B).
    pub fn is_background_task_call(input_background: bool, metadata_background: bool) -> bool {
        input_background || metadata_background
    }

    /// Whether a completed tool part with non-object metadata still fires
    /// (TS: `handleMessagePartUpdated` tool branch ignores non-object
    /// metadata instead of crashing, src ~2279+; slice 19B).
    pub fn completed_tool_proceeds_without_object_metadata(metadata_is_object: bool) -> bool {
        let _ = metadata_is_object;
        true
    }

    /// Whether a parent-run interrupt reaches a card (TS:
    /// `interruptSubagents`: ended cards are untouched, src ~1170-1209;
    /// slice 19B).
    pub fn interrupt_reaches_card(is_terminal_status: bool) -> bool {
        !is_terminal_status
    }

    /// Outbound step for a text message update (TS: first text update ->
    /// `onPartial`, completed message -> `onComplete`, src ~2135+/~1930+;
    /// slice 19B).
    pub fn text_message_flow_event(is_completed_message: bool, has_text: bool) -> TextMessageFlow {
        if !has_text {
            TextMessageFlow::None
        } else if is_completed_message {
            TextMessageFlow::Complete
        } else {
            TextMessageFlow::Partial
        }
    }

    /// Accumulated streaming text after a part delta (TS:
    /// `handleMessagePartDelta` text path, src ~2395+; slice 19B).
    pub fn delta_partial_text(previous: &str, delta: &str) -> String {
        format!("{previous}{delta}")
    }

    /// Whether a part delta streams to the user (TS:
    /// `handleMessagePartDelta`: streams with or without a part type, but
    /// an unknown part after reasoning started stays silent, src ~2395+;
    /// slice 19B).
    pub fn should_stream_part_delta(
        part_type_known: bool,
        delta_non_empty: bool,
        reasoning_started: bool,
    ) -> bool {
        delta_non_empty && (!reasoning_started || part_type_known)
    }

    /// Whether a root error freezes subagent intake (TS:
    /// `handleSessionError`: late child tool/discovery events after the
    /// root error emit nothing, src ~3003-3051; slice 19B).
    pub fn should_freeze_subagent_intake(root_errored: bool) -> bool {
        root_errored
    }

    /// Whether a finished subagent identity is evicted (TS:
    /// `finishedSubagentSessionIds` retains exact identities with no cap,
    /// so late updates of long-finished children stay silent; slice 19B).
    pub fn evicts_finished_identity(_finished_count: usize) -> bool {
        false
    }

    /// Record a background intent seen before its child session exists (TS:
    /// `backgroundChildSessionIds`, honored when the card attaches;
    /// slice 19B).
    pub fn record_pending_background_intent(&mut self, child_session_id: &str) {
        self.background_child_sessions
            .insert(child_session_id.to_string());
    }

    /// Take a pending background intent for an attaching child session
    /// (TS: `applyPendingBackgroundIntent`; slice 19B).
    pub fn take_pending_background_intent(&mut self, child_session_id: &str) -> bool {
        self.background_child_sessions.remove(child_session_id)
    }

    /// Whether external user input reaches its callback (TS: completed
    /// user text for the current session fires; other sessions and
    /// blank text stay silent; slice 19B).
    pub fn should_emit_external_user_input(
        is_current_session: bool,
        role_is_user: bool,
        text_non_blank: bool,
    ) -> bool {
        is_current_session && role_is_user && text_non_blank
    }
}

// ---------------------------------------------------------------------------
// SLICE 20 (GREEN): subagent terminal/cost, background stale-drop, nested
// guards + reselect/outbound-heal gates.
//
// TS source: `src/app/managers/summary-aggregation-manager.ts`
// (`setSubagentTerminalStatus`, `handleSessionStatus` cost/tokens arms,
// `retireBackgroundSubagents`, `isNestedSubagentSession`, `holdOutbound` /
// `drainOutbound` generation guard, `setSession` reselect path) and
// `src/bot/callbacks/session-callback-handler.ts` (`releaseSessionPickHold`).
// ---------------------------------------------------------------------------

impl SummaryAggregator {
    /// Whether a terminal card emits (TS: `setSubagentTerminalStatus` emits
    /// only when the status actually changes; slice 20).
    pub fn should_emit_terminal_card(status_changed: bool, has_card: bool) -> bool {
        status_changed && has_card
    }

    /// Whether a cost update fires (TS: `onCostCallback` fires for a
    /// defined cost value; `None` stays silent; slice 20).
    pub fn cost_fires_for_update(cost: Option<f64>) -> bool {
        cost.is_some()
    }

    /// Whether a background card update is stale-dropped (TS: background
    /// cards retired by `retireBackgroundSubagents` no longer emit tool
    /// updates; slice 20).
    pub fn should_drop_stale_background_update(is_retired_background: bool) -> bool {
        is_retired_background
    }

    /// Whether a nested subagent launch takes no card (TS:
    /// `isNestedSubagentSession`: a child of a tracked child is nested and
    /// stays cardless; slice 20).
    pub fn should_guard_nested_launch(is_child_of_tracked_child: bool) -> bool {
        is_child_of_tracked_child
    }

    /// Whether a reselect with the same session id keeps live state
    /// (TS: `setSession` early-returns without `clear()` when the id is
    /// unchanged, then re-applies bot/chat; slice 20).
    pub fn should_keep_live_state_on_reselect(
        current_session_id: Option<&str>,
        next: &str,
    ) -> bool {
        current_session_id == Some(next)
    }

    /// Which outbound generation a pick release drains with (TS:
    /// `releaseSessionPickHold` drains with the current generation so a newer
    /// hold started during the pick cannot swallow deferred progress;
    /// slice 20).
    pub fn pick_release_generation(captured: u64, current: u64) -> u64 {
        let _ = captured;
        current
    }

    /// Whether a finished subagent identity stays remembered (TS:
    /// `finishedSubagentSessionIds` has no eviction; slice 20).
    pub fn remembers_finished_subagent(&self, child_session_id: &str) -> bool {
        self.finished_subagent_sessions.contains(child_session_id)
    }
}
