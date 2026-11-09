//! Task-creation pure gates (slice 69, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/managers/scheduled-task-creation-manager.ts`
//!   (`TaskCreationManager`: staged `task` interaction payload, clone-on-read
//!   state copies, `awaiting_schedule` -> `parsing_schedule` ->
//!   `awaiting_prompt` transitions, message-id bookkeeping, `resetSchedule`
//!   back to schedule input)
//! - `src/app/types/scheduled-task.ts` (`createScheduledTaskModel` null
//!   variant default, `cloneParsedTaskSchedule` / `cloneScheduledTaskModel` /
//!   `cloneScheduledTask` copies with `enabled ?? true` and pre-agent
//!   `agent ?? "build"` backfills)
//!
//! Pure sync data-structure logic only: no Telegram I/O, no network/fetch,
//! no timers, no `logger` calls. `logger.info`/`logger.debug` lines in TS
//! carry no state and are dropped here.
//!
//! DEVIATIONS from TS (planned, behavior-preserving for the covered cases):
//! - The TS manager reads/writes its state through an `InteractionManager`
//!   (`getPayload("task")` / `start` / `clearKind`). The port holds the
//!   `Option<TaskCreationState>` inline instead (no DI container in this
//!   port), matching the `AttachManager` precedent in
//!   `attach_foreground_gates`.
//! - `Object.assign(state, changes)` + `cloneState` return freshly cloned
//!   copies in TS; the port returns owned `TaskCreationState` clones the
//!   same way (`Clone` + return by value).
//! - `ScheduledTask` is a TS union (`ScheduledCronTask |
//!   `ScheduledOnceTask`). The port models it as one [`ScheduledTask`]
//!   enum with `Cron`/`Once` variants sharing [`ScheduledTaskBase`].
//! - Telegram message ids are `number` in TS and `i64` here.
//! - `enabled?: boolean` / `agent?` backfills (`?? true` / `?? "build"`) map
//!   `None` to the default on clone, exactly like the TS spread.
//! - TS `createScheduledTaskModel(model: ModelInfo)` takes a single model
//!   object (`{ providerID, modelID, variant }`); the port takes three scalar
//!   args `(provider_id, model_id, variant)` because the `ModelInfo` carrier
//!   is out of scope for this slice. `None` still maps to TS `null`.
//! - TS field names are camelCase (`projectId`, `scheduleText`,
//!   `parsedSchedule`, `previewMessageId`, ...); the port uses snake_case
//!   (`project_id`, `schedule_text`, `parsed_schedule`, `preview_message_id`,
//!   ...) per Rust convention. `ScheduledTaskModel` likewise uses
//!   `provider_id`/`model_id` for TS `providerID`/`modelID`.
//! - TS `agent` on `ScheduledTaskBase` is required `string`, but tasks
//!   persisted before the field existed may miss it at runtime (hence the
//!   `?? DEFAULT_TASK_AGENT` backfill). The port types it as
//!   `Option<String>` so the backfill is observable; `Some` values pass
//!   through unchanged.
//! - TS stage strings (`"awaiting_schedule"` / `"parsing_schedule"` /
//!   `"awaiting_prompt"`) map to [`TaskCreationStage`] variants of the same
//!   meaning.

/// Agent assumed for tasks persisted before the agent field existed
/// (TS: `DEFAULT_TASK_AGENT`).
pub const DEFAULT_TASK_AGENT: &str = "build";

/// Model a scheduled task runs with (TS: `ScheduledTaskModel`).
/// `variant: None` models TS `variant: string | null` set to `null`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTaskModel {
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

/// Parsed schedule text (TS: `ParsedTaskSchedule`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedTaskSchedule {
    Cron {
        cron: String,
        timezone: String,
        summary: String,
        next_run_at: String,
    },
    Once {
        run_at: String,
        timezone: String,
        summary: String,
        next_run_at: String,
    },
}

/// Where the creation flow waits (TS: `TaskCreationState["stage"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskCreationStage {
    AwaitingSchedule,
    ParsingSchedule,
    AwaitingPrompt,
}

/// The staged creation flow (TS: `TaskCreationState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCreationState {
    pub stage: TaskCreationStage,
    pub project_id: String,
    pub project_worktree: String,
    pub agent: String,
    pub model: ScheduledTaskModel,
    pub schedule_text: Option<String>,
    pub parsed_schedule: Option<ParsedTaskSchedule>,
    pub schedule_request_message_id: Option<i64>,
    pub preview_message_id: Option<i64>,
    pub prompt_request_message_id: Option<i64>,
}

/// Shared fields of a stored scheduled task (TS: `ScheduledTaskBase`).
/// `agent: None` models a task persisted before the agent field existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTaskBase {
    pub id: String,
    pub project_id: String,
    pub project_worktree: String,
    pub agent: Option<String>,
    pub model: ScheduledTaskModel,
    pub schedule_text: String,
    pub schedule_summary: String,
    pub timezone: String,
    pub prompt: String,
    pub created_at: String,
    pub next_run_at: Option<String>,
    pub last_run_at: Option<String>,
    pub run_count: u64,
    pub last_status: String,
    pub last_error: Option<String>,
}

/// A stored cron task (TS: `ScheduledCronTask`).
/// `enabled: None` models a missing field (defaults to `true` on clone).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledCronTask {
    pub base: ScheduledTaskBase,
    pub cron: String,
    pub enabled: Option<bool>,
    pub activation_pending: Option<bool>,
}

/// A stored one-shot task (TS: `ScheduledOnceTask`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledOnceTask {
    pub base: ScheduledTaskBase,
    pub run_at: String,
}

/// A stored scheduled task (TS: `ScheduledTask`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduledTask {
    Cron(ScheduledCronTask),
    Once(ScheduledOnceTask),
}

/// Staged creation flow holder (TS: `TaskCreationManager`).
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct TaskCreationManager {
    state: Option<TaskCreationState>,
}

impl TaskCreationManager {
    /// Creates an inactive flow holder.
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Starts the flow for a project (TS: `start`; returns a state copy).
    pub fn start(
        &mut self,
        project_id: &str,
        project_worktree: &str,
        model: ScheduledTaskModel,
        agent: &str,
    ) -> TaskCreationState {
        let state = TaskCreationState {
            stage: TaskCreationStage::AwaitingSchedule,
            project_id: project_id.to_string(),
            project_worktree: project_worktree.to_string(),
            agent: agent.to_string(),
            model: clone_scheduled_task_model(&model),
            schedule_text: None,
            parsed_schedule: None,
            schedule_request_message_id: None,
            preview_message_id: None,
            prompt_request_message_id: None,
        };
        self.state = Some(state.clone());
        state
    }

    /// Whether the flow holds staged state (TS: `isActive`).
    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    /// Whether the flow waits for schedule text (TS: `isWaitingForSchedule`).
    pub fn is_waiting_for_schedule(&self) -> bool {
        matches!(
            self.state.as_ref(),
            Some(state) if state.stage == TaskCreationStage::AwaitingSchedule
        )
    }

    /// Whether a schedule parse is in flight (TS: `isParsingSchedule`).
    pub fn is_parsing_schedule(&self) -> bool {
        matches!(
            self.state.as_ref(),
            Some(state) if state.stage == TaskCreationStage::ParsingSchedule
        )
    }

    /// Whether the flow waits for the task prompt (TS: `isWaitingForPrompt`).
    pub fn is_waiting_for_prompt(&self) -> bool {
        matches!(
            self.state.as_ref(),
            Some(state) if state.stage == TaskCreationStage::AwaitingPrompt
        )
    }

    /// Copy of the staged state (TS: `getState`).
    pub fn get_state(&self) -> Option<TaskCreationState> {
        self.state.clone()
    }

    /// Records the parsed schedule and moves to prompt input
    /// (TS: `setParsedSchedule`).
    pub fn set_parsed_schedule(
        &mut self,
        schedule_text: &str,
        parsed_schedule: ParsedTaskSchedule,
        preview_message_id: i64,
    ) -> Option<TaskCreationState> {
        let state = self.state.as_mut()?;
        state.stage = TaskCreationStage::AwaitingPrompt;
        state.schedule_text = Some(schedule_text.to_string());
        state.parsed_schedule = Some(clone_parsed_task_schedule(&parsed_schedule));
        state.schedule_request_message_id = None;
        state.preview_message_id = Some(preview_message_id);
        state.prompt_request_message_id = None;
        self.state.clone()
    }

    /// Marks schedule parsing as started (TS: `markScheduleParsing`).
    pub fn mark_schedule_parsing(&mut self) -> Option<TaskCreationState> {
        let state = self.state.as_mut()?;
        state.stage = TaskCreationStage::ParsingSchedule;
        self.state.clone()
    }

    /// Remembers the prompt-request message (TS:
    /// `setPromptRequestMessageId`).
    pub fn set_prompt_request_message_id(&mut self, message_id: i64) -> Option<TaskCreationState> {
        let state = self.state.as_mut()?;
        state.prompt_request_message_id = Some(message_id);
        self.state.clone()
    }

    /// Remembers the schedule-request message (TS:
    /// `setScheduleRequestMessageId`).
    pub fn set_schedule_request_message_id(
        &mut self,
        message_id: i64,
    ) -> Option<TaskCreationState> {
        let state = self.state.as_mut()?;
        state.schedule_request_message_id = Some(message_id);
        self.state.clone()
    }

    /// Drops the parsed schedule and returns to schedule input
    /// (TS: `resetSchedule`).
    pub fn reset_schedule(&mut self) -> Option<TaskCreationState> {
        let state = self.state.as_mut()?;
        state.stage = TaskCreationStage::AwaitingSchedule;
        state.schedule_text = None;
        state.parsed_schedule = None;
        state.schedule_request_message_id = None;
        state.preview_message_id = None;
        state.prompt_request_message_id = None;
        self.state.clone()
    }

    /// Drops the staged state (TS: `clear`).
    pub fn clear(&mut self) {
        self.state = None;
    }
}

/// Builds the model record a task runs with; a missing variant stays
/// missing (`null` in TS) (TS: `createScheduledTaskModel`).
pub fn create_scheduled_task_model(
    provider_id: &str,
    model_id: &str,
    variant: Option<&str>,
) -> ScheduledTaskModel {
    ScheduledTaskModel {
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
        variant: variant.map(|v| v.to_string()),
    }
}

/// Copies a task model record (TS: `cloneScheduledTaskModel`).
pub fn clone_scheduled_task_model(model: &ScheduledTaskModel) -> ScheduledTaskModel {
    model.clone()
}

/// Copies a parsed schedule (TS: `cloneParsedTaskSchedule`).
pub fn clone_parsed_task_schedule(schedule: &ParsedTaskSchedule) -> ParsedTaskSchedule {
    schedule.clone()
}

/// Copies a stored task, backfilling `enabled ?? true` (cron) and
/// `agent ?? "build"` (TS: `cloneScheduledTask`).
pub fn clone_scheduled_task(task: &ScheduledTask) -> ScheduledTask {
    match task {
        ScheduledTask::Cron(inner) => {
            let mut cloned = inner.clone();
            if cloned.enabled.is_none() {
                cloned.enabled = Some(true);
            }
            if cloned.base.agent.is_none() {
                cloned.base.agent = Some(DEFAULT_TASK_AGENT.to_string());
            }
            cloned.base.model = clone_scheduled_task_model(&inner.base.model);
            ScheduledTask::Cron(cloned)
        }
        ScheduledTask::Once(inner) => {
            let mut cloned = inner.clone();
            if cloned.base.agent.is_none() {
                cloned.base.agent = Some(DEFAULT_TASK_AGENT.to_string());
            }
            cloned.base.model = clone_scheduled_task_model(&inner.base.model);
            ScheduledTask::Once(cloned)
        }
    }
}
