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
    MessageUpdated { session_id: String, message_id: String },
    ToolPartUpdated { part: ToolPart },
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
    TaskToolPartUpdated { input: TaskCallInput },
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
pub type ClearedCallback = Box<dyn FnMut()>;
/// Fired for a tool part (TS: `ToolCallback`).
pub type ToolCallback = Box<dyn FnMut(ToolInfo)>;
/// Fired for a root-session tool update (TS: `RootToolUpdateCallback`).
pub type RootToolUpdateCallback = Box<dyn FnMut(ToolInfo)>;
/// Fired with the live subagent cards (TS: `SubagentCallback`; slice 2).
pub type SubagentCallback = Box<dyn FnMut(String, Vec<SubagentInfo>)>;

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
    /// Synthetic text part ids per message (TS: `syntheticPartIds`;
    /// slice 9). A delta event often carries only the part id, so the
    /// flag is looked up in the registry too.
    synthetic_part_ids: HashMap<String, HashSet<String>>,
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
            synthetic_part_ids: HashMap::new(),
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
        self.synthetic_part_ids.clear();
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
            let entry =
                self.open_task_calls
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
                card_id = Some(self.get_or_create_card_for_session(&child, Some(&parent_session_id)));
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
        let queue = self
            .fallback_cards_by_parent
            .entry(parent)
            .or_default();
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
        self.session_parents.insert(
            child_session_id.to_string(),
            parent_session_id.to_string(),
        );
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
                    && words.last().map_or(false, |word| word.eq_ignore_ascii_case("subagent"))
                {
                    words.pop();
                    let agent = words
                        .join(" ")
                        .trim()
                        .trim_start_matches('@')
                        .to_string();
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
                if matches!(card.status, SubagentStatus::Pending | SubagentStatus::Running) =>
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
    pub fn track_nested_launch(
        &mut self,
        parent_child_session_id: &str,
        nested_session_id: &str,
    ) {
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
pub type SessionIdleCallback = Box<dyn FnMut(String, SessionIdleInfo)>;
/// Fired on a `session.status` retry for the followed session
/// (TS: `SessionRetryCallback`).
pub type SessionRetryCallback = Box<dyn FnMut(SessionRetryInfo)>;

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
