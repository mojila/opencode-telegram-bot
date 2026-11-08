//! Task-callback-handler pure gates (slice 83, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/scheduled-task-callback-handler.ts`
//!   (`handleTaskCallback` exact `task:retry-schedule` / `task:cancel`
//!   dispatch, `isTaskInteraction` `kind === "task"` gate,
//!   `isTaskCallbackActive` schedule/preview/prompt message-id membership,
//!   the combined inactive gate (`!flowState || !isTaskInteraction(...) ||
//!   callbackMessageId === null || !isTaskCallbackActive(...)`), the stale
//!   clear gate (`!flowState && isTaskInteraction(...)`),
//!   `handleTaskListCallback` `tasklist:` dispatch guard, the combined
//!   tasklist inactive gate (`!metadata || callbackMessageId === null ||
//!   metadata.messageId !== callbackMessageId`), `parseTaskListMetadata`
//!   (`custom` kind + `tasklist` flow + numeric `messageId` + `list` /
//!   `detail`-with-non-empty-`taskId` stage gates), the `list`-stage open
//!   gate, the `detail`-stage + `taskId`-equality toggle/delete gates, the
//!   `deleteMessageIfPresent` chat-plus-numeric-id gate, the retry-schedule
//!   gate (`isWaitingForPrompt() && callbackMessageId ===
//!   previewMessageId`), `buildTaskInteractionMetadata` shape,
//!   `truncatePromptForDetails` byte-budget cut with
//!   `TASK_DETAIL_PROMPT_BYTE_BUDGET`).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `recent_scheduled_gates::TASK_RETRY_SCHEDULE_CALLBACK`,
//!   `TASK_CANCEL_CALLBACK`, `TASKLIST_*` prefixes, `build_tasklist_*_callback`,
//!   `parse_tasklist_open/delete/toggle_callback`, `is_tasklist_cancel_callback`
//!   (the menu constants and id parses behind the branches above).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`, `reply`, transitions, `clear`).
//! - `taskCreationManager` reads/writes, `getScheduledTask` /
//!   `toggleScheduledTask` / `removeScheduledTask` store calls,
//!   `scheduledTaskRuntime.refreshTask` / `removeTask` calls.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller; `formatTaskDetails` (localized composition) and
//!   `formatDateTime` (locale `Intl` rendering) stay out.
//!
//! Deviations: none intended; `messageId` values are `i64` (`typeof ===
//! "number"` in TS), flow ids are `Option<i64>` (`null` in TS).
//! `truncate_prompt_for_details` counts UTF-8 bytes like
//! `Buffer.byteLength(..., "utf-8")`; parity fixtures are ASCII-only where
//! byte length and `char` count coincide, and the suffix is always `"..."`.

/// Byte budget for the prompt line of the task-details view (TS:
/// `TASK_DETAIL_PROMPT_BYTE_BUDGET`).
pub const TASK_DETAIL_PROMPT_BYTE_BUDGET: usize = 3400;

/// Message ids the task-creation flow may own (TS: the
/// `scheduleRequestMessageId` / `previewMessageId` /
/// `promptRequestMessageId` fields of `TaskCreationState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFlowMessageIds {
    pub schedule_request_message_id: Option<i64>,
    pub preview_message_id: Option<i64>,
    pub prompt_request_message_id: Option<i64>,
}

/// Flattened interaction snapshot the tasklist-metadata reader inspects
/// (TS: `InteractionState | null` with `kind` plus a `tasklist` metadata
/// bag: `flow`, `stage`, `messageId`, and `detail`-stage `taskId`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListSnapshot {
    pub kind: String,
    pub flow: Option<String>,
    pub stage: Option<String>,
    pub message_id: Option<i64>,
    pub task_id: Option<String>,
}

/// Live tasklist flow state (TS: the `TaskListMetadata` union: `list` with
/// `messageId`, or `detail` with `messageId` + `taskId`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskListMetadata {
    List { message_id: i64 },
    Detail { message_id: i64, task_id: String },
}

/// Metadata the retry-schedule branch transitions into (TS:
/// `buildTaskInteractionMetadata`: `flow: "task"` plus stage, project
/// id/worktree, and optional preview message id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInteractionMetadata {
    pub flow: String,
    pub stage: String,
    pub project_id: String,
    pub project_worktree: String,
    pub preview_message_id: Option<i64>,
}

/// Task-creation dispatch guard: only the exact `task:retry-schedule` and
/// `task:cancel` callbacks handle (TS: `data !== TASK_RETRY_SCHEDULE_CALLBACK
/// && data !== TASK_CANCEL_CALLBACK` returns `false`).
pub fn handles_task_callback(data: Option<&str>) -> bool {
    matches!(
        data,
        Some(
            crate::recent_scheduled_gates::TASK_RETRY_SCHEDULE_CALLBACK
                | crate::recent_scheduled_gates::TASK_CANCEL_CALLBACK
        )
    )
}

/// Task-flow interaction gate (TS: `isTaskInteraction`: `state?.kind ===
/// "task"`).
pub fn is_task_interaction(kind: Option<&str>) -> bool {
    kind == Some("task")
}

/// Task-callback liveness gate: the callback message must be one of the
/// flow's three owned message ids (TS: `isTaskCallbackActive`).
pub fn is_task_callback_active(flow: &TaskFlowMessageIds, message_id: i64) -> bool {
    flow.schedule_request_message_id == Some(message_id)
        || flow.preview_message_id == Some(message_id)
        || flow.prompt_request_message_id == Some(message_id)
}

/// Combined inactive gate for task-creation callbacks (TS: `!flowState ||
/// !isTaskInteraction(interactionState) || callbackMessageId === null ||
/// !isTaskCallbackActive(...)` answers `task.inactive_callback`).
pub fn should_answer_task_inactive(
    has_flow_state: bool,
    is_task_interaction_state: bool,
    has_callback_message_id: bool,
    is_active_message: bool,
) -> bool {
    !has_flow_state
        || !is_task_interaction_state
        || !has_callback_message_id
        || !is_active_message
}

/// Stale-state clear gate: with no flow state but a live task interaction,
/// the interaction is cleared before answering inactive (TS: `!flowState &&
/// isTaskInteraction(interactionState)` calls `clearTaskInteraction`).
pub fn should_clear_stale_task_interaction(
    has_flow_state: bool,
    is_task_interaction_state: bool,
) -> bool {
    !has_flow_state && is_task_interaction_state
}

/// Tasklist dispatch guard: no data never handles; anything under
/// `tasklist:` does (TS: `!data || !data.startsWith(TASKLIST_CALLBACK_PREFIX)`
/// returns `false`).
pub fn handles_tasklist_callback(data: Option<&str>) -> bool {
    match data {
        Some(s) if !s.is_empty() => {
            s.starts_with(crate::recent_scheduled_gates::TASKLIST_CALLBACK_PREFIX)
        }
        _ => false,
    }
}

/// Live tasklist flow state: `custom` kind plus `flow === "tasklist"` plus
/// a numeric `messageId`, with `list` or `detail`-with-non-empty-`taskId`
/// stage (TS: `parseTaskListMetadata`).
pub fn parse_task_list_metadata(state: Option<&TaskListSnapshot>) -> Option<TaskListMetadata> {
    let state = state?;
    if state.kind != "custom" {
        return None;
    }
    if state.flow.as_deref() != Some("tasklist") {
        return None;
    }
    let message_id = state.message_id?;
    match state.stage.as_deref() {
        Some("list") => Some(TaskListMetadata::List { message_id }),
        Some("detail") => match state.task_id.as_deref() {
            Some(task_id) if !task_id.is_empty() => Some(TaskListMetadata::Detail {
                message_id,
                task_id: task_id.to_string(),
            }),
            _ => None,
        },
        _ => None,
    }
}

/// Tasklist open branch requires the `list` stage (TS:
/// `metadata.stage !== "list"` answers `tasklist.inactive_callback`).
pub fn is_tasklist_list_stage(stage: Option<&str>) -> bool {
    stage == Some("list")
}

/// Tasklist toggle/delete branches require the `detail` stage (TS:
/// `metadata.stage !== "detail"` answers `tasklist.inactive_callback`).
pub fn is_tasklist_detail_stage(stage: Option<&str>) -> bool {
    stage == Some("detail")
}

/// Combined inactive gate for tasklist callbacks (TS: `!metadata ||
/// callbackMessageId === null || metadata.messageId !== callbackMessageId`
/// answers `tasklist.inactive_callback`).
pub fn should_answer_tasklist_inactive(
    has_metadata: bool,
    has_callback_message_id: bool,
    messages_match: bool,
) -> bool {
    !has_metadata || !has_callback_message_id || !messages_match
}

/// Tasklist toggle gate: `detail` stage plus matching task id (TS:
/// `metadata.stage !== "detail" || metadata.taskId !== taskId` answers
/// `tasklist.inactive_callback`).
pub fn should_allow_tasklist_toggle(is_detail_stage: bool, task_ids_match: bool) -> bool {
    is_detail_stage && task_ids_match
}

/// Tasklist delete gate: `detail` stage plus matching task id (TS:
/// `metadata.stage !== "detail"` / `taskId !== metadata.taskId` answers
/// `tasklist.inactive_callback`).
pub fn should_allow_tasklist_delete(is_detail_stage: bool, task_ids_match: bool) -> bool {
    is_detail_stage && task_ids_match
}

/// Best-effort message-delete gate (TS: `deleteMessageIfPresent`: no chat
/// or a non-numeric id skips the delete).
pub fn should_delete_message_if_present(has_chat: bool, message_id_is_number: bool) -> bool {
    has_chat && message_id_is_number
}

/// Retry-schedule gate: the flow must wait for a prompt and the callback
/// message must be the preview message (TS:
/// `!deps.taskCreationManager.isWaitingForPrompt() || callbackMessageId !==
/// flowState.previewMessageId` answers `task.inactive_callback`).
pub fn should_retry_task_schedule(is_waiting_for_prompt: bool, is_preview_message: bool) -> bool {
    is_waiting_for_prompt && is_preview_message
}

/// `flow: "task"` metadata the retry branch transitions into (TS:
/// `buildTaskInteractionMetadata`).
pub fn build_task_interaction_metadata(
    stage: &str,
    project_id: &str,
    project_worktree: &str,
    preview_message_id: Option<i64>,
) -> TaskInteractionMetadata {
    TaskInteractionMetadata {
        flow: "task".to_string(),
        stage: stage.to_string(),
        project_id: project_id.to_string(),
        project_worktree: project_worktree.to_string(),
        preview_message_id,
    }
}

/// Prompt line cut to the details byte budget with a `"..."` suffix (TS:
/// `truncatePromptForDetails`: prompts within budget pass through,
/// otherwise the longest prefix within `budget - 3` bytes plus `"..."`).
pub fn truncate_prompt_for_details(prompt: &str) -> String {
    if prompt.as_bytes().len() <= TASK_DETAIL_PROMPT_BYTE_BUDGET {
        return prompt.to_string();
    }
    let budget = TASK_DETAIL_PROMPT_BYTE_BUDGET - "...".len();
    let mut end = 0usize;
    for (idx, ch) in prompt.char_indices() {
        let next = idx + ch.len_utf8();
        if next > budget {
            break;
        }
        end = next;
    }
    format!("{}...", &prompt[..end])
}
