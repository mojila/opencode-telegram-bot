//! V2 event-translator pure gates (slice 108, GREEN).
//!
//! Rust port of the pure remainder in `src/opencode/v2/events.ts`
//! (`createV2EventTranslator` + `translateGoalChanged`):
//! - `translateGoalChanged` emit gates (non-`rpc.opencode.goal.changed`
//!   skip, non-string `sessionID` skip, `goal ?? null` fallback)
//! - `idleEvents` id pair (`${id}:status` / `${id}:idle`) and the
//!   interrupted + `reason` mark V1 has no other way to say
//! - `backgroundKey` (`shell:<shellID>` wins, else
//!   `session:<sessionID>`, else null) and the background-buffer gate
//!   (a `session.tool.success` whose merged metadata stays `running`
//!   is kept as a running tool until its shell/session operation ends)
//! - missing-tool-call drop (`session.tool.called` / `progress` /
//!   `success` / `failed` with no `tools.get(id)` entry emits nothing)
//! - inbox-delivery filter (missing item, non-`user` item, or
//!   `isGoalInternalMetadata` item emits nothing; otherwise a
//!   `message.updated` + `${inboxID}:text` text part)
//! - part/event id shapes (`${messageID}:updated`, `${part.id}:${time}`,
//!   `${id}:error`, `${assistantMessageID}:text:${ordinal}`,
//!   `:reasoning:`, `:step-start` / `:step-finish`, `message.part.delta`
//!   `partID`)
//! - translator directory memory (`rememberDirectory` empty skip,
//!   `location ?? remembered` attribution, remember only with a
//!   `sessionID`)
//! - `server.connected` tool-map clear + `restarted === undefined`
//!   flag gate
//! - `session.execution.*` end mapping (succeeded/interrupted ->
//!   idle (+ background end); failed -> error + idle (+ background
//!   end)) and `session.compaction.ended` -> `session.compacted`
//! - `form.created` / `form.replied` / `form.cancelled` ->
//!   `question.asked` / `question.replied` / `question.rejected`
//!   mapping.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `toV1ToolName` / `toV1ToolInput` / `toV1ToolMetadata` /
//!   `toolContentText` / `toV1ErrorPayload` / `toV1Permission` /
//!   `toV1Question` / `toFormAnswer` mapping bodies (see `v2_mappers`);
//!   this slice owns only the translator's emit/drop/id/directory/end
//!   decisions around those calls.
//! - `isGoalInternalMetadata` (see `v2_mappers`), `isSameDirectory` /
//!   `getReconnectInfo` (see `event_transport_gates`), subscription
//!   lifecycle / busy-idle routing (see `event_subscription`).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - the translator closure `Map`s (`sessions`, `messages`,
//!   `streamedAt`, `tools`, `backgroundCalls`, `inbox`), `Date.now()`
//!   clocks (arrive as explicit `created` args), `onForm` callback
//!   I/O, `V1GlobalEvent` network envelopes.
//!
//! Deviations: none intended. JS template ids (`${id}:status`,
//! `${id}:idle`, `${id}:error`, `${messageID}:updated`,
//! `${part.id}:${time}`) are plain concatenation; parity fixtures use
//! ASCII ids where JS UTF-16 and Rust `char`s agree. `||` / `??`
//! fallbacks treat only `None` and `""` as missing (whitespace-only
//! strings count as present, like TS).
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity tests in
//! `crates/otb-core/tests/slice108_v2_event_translator_parity.rs`
//! assert the TS behavior and now pass.

/// RPC type the translator turns into a V1 goal event
/// (TS: `"rpc.opencode.goal.changed"` in `translateGoalChanged`).
pub const V2_GOAL_CHANGED_RPC_TYPE: &str = "rpc.opencode.goal.changed";

/// V1 goal event the translator emits (TS: `"goal.changed"`).
pub const V1_GOAL_CHANGED_EVENT_TYPE: &str = "goal.changed";

/// V1 busy/idle status event (TS: `"session.status"` in `idleEvents`).
pub const V1_SESSION_STATUS_EVENT_TYPE: &str = "session.status";

/// V1 idle event (TS: `"session.idle"` in `idleEvents`).
pub const V1_SESSION_IDLE_EVENT_TYPE: &str = "session.idle";

/// V1 assistant update event (TS: `"message.updated"` in `assistantInfo`).
pub const V1_MESSAGE_UPDATED_EVENT_TYPE: &str = "message.updated";

/// V1 part-update event (TS: `"message.part.updated"` in `partUpdated`).
pub const V1_PART_UPDATED_EVENT_TYPE: &str = "message.part.updated";

/// V1 part-delta event (TS: `"message.part.delta"` for text/reasoning deltas).
pub const V1_PART_DELTA_EVENT_TYPE: &str = "message.part.delta";

/// V1 session-error event (TS: `"session.error"`).
pub const V1_SESSION_ERROR_EVENT_TYPE: &str = "session.error";

/// V1 compaction event (TS: `"session.compacted"`).
pub const V1_SESSION_COMPACTED_EVENT_TYPE: &str = "session.compacted";

/// V1 permission event (TS: `"permission.asked"`).
pub const V1_PERMISSION_ASKED_EVENT_TYPE: &str = "permission.asked";

/// V1 question-asked event (TS: `"question.asked"` for `form.created`).
pub const V1_QUESTION_ASKED_EVENT_TYPE: &str = "question.asked";

/// V1 question-replied event (TS: `"question.replied"` for `form.replied`).
pub const V1_QUESTION_REPLIED_EVENT_TYPE: &str = "question.replied";

/// V1 question-rejected event (TS: `"question.rejected"` for `form.cancelled`).
pub const V1_QUESTION_REJECTED_EVENT_TYPE: &str = "question.rejected";

/// `true` when a goal-changed RPC event translates at all
/// (TS: `translateGoalChanged`: `event.type === "rpc.opencode.goal.changed"`
/// and `typeof data.sessionID === "string"`, else `[]`).
pub fn should_emit_goal_changed(is_goal_changed_type: bool, session_id_is_string: bool) -> bool {
    is_goal_changed_type && session_id_is_string
}

/// `true` when the translated goal payload carries `null`
/// (TS: `goal: data.goal ?? null`: missing/`null` goal -> null).
pub fn goal_changed_goal_is_null(has_goal: bool) -> bool {
    !has_goal
}

/// Status half of an idle pair (TS: `` `${id}:status` `` in `idleEvents`).
pub fn idle_status_event_id(event_id: &str) -> String {
    format!("{event_id}:status")
}

/// Idle half of an idle pair (TS: `` `${id}:idle` `` in `idleEvents`).
pub fn idle_event_id(event_id: &str) -> String {
    format!("{event_id}:idle")
}

/// Error event for a failed execution (TS: `` `${event.id}:error` ``).
pub fn session_error_event_id(event_id: &str) -> String {
    format!("{event_id}:error")
}

/// Assistant-update event id (TS: `` `${messageID}:updated` `` in `assistantInfo`).
pub fn assistant_updated_event_id(message_id: &str) -> String {
    format!("{message_id}:updated")
}

/// Part-update event id (TS: `` `${part.id}:${time}` `` in `partUpdated`).
pub fn part_updated_event_id(part_id: &str, time: i64) -> String {
    format!("{part_id}:{time}")
}

/// Background-operation key a completed tool call is buffered under
/// (TS: `backgroundKey`: `shell:<shellID>`, else `session:<sessionID>`,
/// else null; empty strings count as missing, like the `typeof === "string"`
/// check on a missing value).
pub fn background_key(shell_id: Option<&str>, session_id: Option<&str>) -> Option<String> {
    match shell_id {
        Some(id) if !id.is_empty() => Some(format!("shell:{id}")),
        _ => match session_id {
            Some(id) if !id.is_empty() => Some(format!("session:{id}")),
            _ => None,
        },
    }
}

/// `true` when a `session.tool.success` stays a running tool instead of
/// completing (TS: merged metadata `status === "running"` with a
/// non-null `backgroundKey`; only success reports can launch a
/// background operation).
pub fn should_buffer_background_call(
    is_success_event: bool,
    merged_status_is_running: bool,
    has_background_key: bool,
) -> bool {
    is_success_event && merged_status_is_running && has_background_key
}

/// `true` when a tool event is dropped for lack of a start entry
/// (TS: `tools.get(data.id)` miss in `session.tool.called` /
/// `progress` / `success` / `failed` returns `[]`).
pub fn should_drop_missing_tool_call(has_call: bool) -> bool {
    !has_call
}

/// `true` when an inbox delivery translates into user message events
/// (TS: `session.inbox.delivered` emits only when the item exists, its
/// `type === "user"`, and it is not goal-internal metadata).
pub fn should_emit_inbox_delivery(
    has_item: bool,
    is_user_item: bool,
    is_goal_internal: bool,
) -> bool {
    has_item && is_user_item && !is_goal_internal
}

/// Text part id for a delivered inbox item
/// (TS: `` `${inboxID}:text` `` with `messageID: inboxID`).
pub fn inbox_text_part_id(inbox_id: &str) -> String {
    format!("{inbox_id}:text")
}

/// Text part id for a step (TS: `` `${assistantMessageID}:text:${ordinal}` ``).
pub fn step_text_part_id(assistant_message_id: &str, ordinal: u64) -> String {
    format!("{assistant_message_id}:text:{ordinal}")
}

/// Reasoning part id for a step
/// (TS: `` `${assistantMessageID}:reasoning:${ordinal}` ``).
pub fn step_reasoning_part_id(assistant_message_id: &str, ordinal: u64) -> String {
    format!("{assistant_message_id}:reasoning:{ordinal}")
}

/// Delta `partID` for a text/reasoning delta
/// (TS: `` `${assistantMessageID}:${kind}:${ordinal}` `` where
/// `kind` is `"text"` or `"reasoning"`).
pub fn delta_part_id(assistant_message_id: &str, kind: &str, ordinal: u64) -> String {
    format!("{assistant_message_id}:{kind}:{ordinal}")
}

/// Directory a translated event is attributed to
/// (TS: `location ?? remembered`: the stream location wins when
/// present and non-empty, else the remembered session directory).
pub fn resolve_translator_directory(
    location: Option<&str>,
    remembered: Option<&str>,
) -> Option<String> {
    match location {
        Some(dir) if !dir.is_empty() => Some(dir.to_string()),
        _ => match remembered {
            Some(dir) if !dir.is_empty() => Some(dir.to_string()),
            _ => None,
        },
    }
}

/// `true` when the translator remembers a directory for the session
/// (TS: `rememberDirectory` runs only with a `sessionID` and a
/// non-empty `directory`; `if (!directory) return`).
pub fn should_remember_directory(has_session_id: bool, directory: Option<&str>) -> bool {
    has_session_id && matches!(directory, Some(dir) if !dir.is_empty())
}

/// `true` when a connect event clears pending tool starts
/// (TS: `case "server.connected": tools.clear()`).
pub fn should_clear_tools_on_connect(is_connected_event: bool) -> bool {
    is_connected_event
}

/// `true` when the connect carries an explicit restart mark
/// (TS: `restarted === undefined ? {} : { restarted }`).
pub fn has_restarted_flag(restarted: Option<bool>) -> bool {
    restarted.is_some()
}

/// `true` when an execution end counts as interrupted rather than
/// finished (TS: `event.type === "session.execution.interrupted"`;
/// `succeeded` is the finished path).
pub fn execution_end_is_interrupted(is_interrupted_type: bool) -> bool {
    is_interrupted_type
}

/// `true` when an execution end reports through the failure path
/// (TS: `case "session.execution.failed"` emits `session.error` first,
/// unlike succeeded/interrupted which go straight to idle).
pub fn execution_end_is_failed(is_failed_type: bool) -> bool {
    is_failed_type
}
