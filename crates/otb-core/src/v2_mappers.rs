//! V1/V2 OpenCode mapper pure gates (slice 42, RED).
//!
//! Rust port of the pure sync mapping logic in:
//! - `src/opencode/v2/mappers.ts` (`toV1ToolName`, `toV1ToolInput`,
//!   `toV1ToolMetadata`, `toolContentText`, `toV1ErrorPayload`,
//!   `toV1Session`, `toV1GlobalSession`, `toV1Project`, `toV1Message`
//!   incl. the user/assistant/tool-state branches, `toV1FileDiff`,
//!   `toV1Model`, `toV1Providers`, `toV1Agent`, `toV1Commands`,
//!   `toV1McpStatus`, `toV1Permission`, `visibleFormFields`,
//!   `toV1Question`, `toFormAnswer`, `toV2PromptInput`)
//! - `src/opencode/v2/goal-message.ts` (`isGoalInternalMetadata`)
//!
//! Dynamic TS `unknown` record shapes are modelled as [`JsonVal`]; the V2
//! SDK input shapes and the V1 SDK output shapes they map to are modelled
//! as the minimal `V2*` / `V1*` structs below (only the fields the mappers
//! read or write).
//!
//! Step 2 (GREEN): pure sync ports of the TS mappers below. Dynamic TS
//! `unknown` record shapes are modelled as [`JsonMap`]; only the fields the
//! mappers read or write are modelled.
//!
//! Pure sync logic only: no fetch, no timers, no I/O.

use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Dynamic JSON-ish value (TS: `unknown` record fields)
// ---------------------------------------------------------------------------

/// Minimal dynamic value for the record fields the mappers probe
/// (TS: `unknown` values inside tool input / metadata / settings).
#[derive(Debug, Clone, PartialEq)]
pub enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Text(String),
    Arr(Vec<JsonVal>),
    Obj(JsonMap),
}

/// String-keyed dynamic record (TS: `Record<string, unknown>`).
pub type JsonMap = BTreeMap<String, JsonVal>;

// ---------------------------------------------------------------------------
// Goal metadata (`goal-message.ts`)
// ---------------------------------------------------------------------------

/// V2 message metadata shape relevant to the goal filter
/// (TS: `unknown` probed with `isRecord` + `.source`).
#[derive(Debug, Clone, PartialEq)]
pub enum MetaVal {
    Record(BTreeMap<String, String>),
    Other,
}

/// A kickoff or continuation the server marked as its own, not a person's
/// message (TS: `isGoalInternalMetadata`).
pub fn is_goal_internal_metadata(metadata: &MetaVal) -> bool {
    matches!(metadata, MetaVal::Record(map) if map.get("source").is_some_and(|s| s == "goal"))
}

// ---------------------------------------------------------------------------
// Tool names / inputs / metadata
// ---------------------------------------------------------------------------

/// V2 tool id -> V1 tool name the bot's formatters know
/// (TS: `toV1ToolName`; `shell` -> `bash`, `subagent` -> `task`,
/// `patch` -> `apply_patch`, everything else passes through).
pub fn to_v1_tool_name(name: &str) -> &str {
    match name {
        "shell" => "bash",
        "subagent" => "task",
        "patch" => "apply_patch",
        _ => name,
    }
}

/// Adds the V1 input field names the formatters read, keeping the V2 ones
/// (TS: `toV1ToolInput`: `path` -> `filePath`; `task` `agent`/`subagent`/
/// `type` -> `subagent_type`; `apply_patch` `patch`/`text` -> `patchText`).
pub fn to_v1_tool_input(tool: &str, input: &JsonMap) -> JsonMap {
    let mut result = input.clone();
    let is_text = |map: &JsonMap, key: &str| matches!(map.get(key), Some(JsonVal::Text(_)));
    if !is_text(&result, "filePath") {
        if let Some(JsonVal::Text(_)) = result.get("path") {
            if let Some(path) = result.get("path").cloned() {
                result.insert("filePath".to_string(), path);
            }
        }
    }
    if tool == "task" && !is_text(&result, "subagent_type") {
        let agent = ["agent", "subagent", "type"]
            .iter()
            .find_map(|key| result.get(*key))
            .cloned();
        if let Some(JsonVal::Text(agent)) = agent {
            result.insert("subagent_type".to_string(), JsonVal::Text(agent));
        }
    }
    if tool == "apply_patch" && !is_text(&result, "patchText") {
        let patch = ["patch", "text"]
            .iter()
            .find_map(|key| result.get(*key))
            .cloned();
        if let Some(JsonVal::Text(patch)) = patch {
            result.insert("patchText".to_string(), JsonVal::Text(patch));
        }
    }
    result
}

/// Adds the V1 file-change field names the formatters read, keeping the V2
/// ones (TS: `toV1ToolMetadata`: single-file `edit` gains `diff` +
/// `filediff`; `apply_patch` files gain `filePath` + `relativePath`).
pub fn to_v1_tool_metadata(tool: &str, metadata: &JsonMap) -> JsonMap {
    // A subagent call names its child session under the V1 key too, so its card is found.
    if tool == "task"
        && matches!(metadata.get("sessionID"), Some(JsonVal::Text(_)))
        && !metadata.contains_key("sessionId")
    {
        let mut result = metadata.clone();
        if let Some(id) = metadata.get("sessionID").cloned() {
            result.insert("sessionId".to_string(), id);
        }
        return result;
    }

    let files = match metadata.get("files") {
        Some(JsonVal::Arr(files)) => files.clone(),
        _ => return metadata.clone(),
    };
    // TS: `metadata.files.filter(isRecord)`.
    let files: Vec<JsonMap> = files
        .into_iter()
        .filter_map(|file| match file {
            JsonVal::Obj(map) => Some(map),
            _ => None,
        })
        .collect();

    if tool == "edit" && !metadata.contains_key("filediff") {
        if files.len() != 1 {
            return metadata.clone();
        }
        let file = &files[0];
        let file_name = match file.get("file") {
            Some(JsonVal::Text(name)) => name.clone(),
            _ => return metadata.clone(),
        };
        let patch = match file.get("patch") {
            Some(JsonVal::Text(patch)) => Some(patch.clone()),
            _ => None,
        };
        let mut result = metadata.clone();
        if patch.is_some() && !matches!(metadata.get("diff"), Some(JsonVal::Text(_))) {
            if let Some(patch) = patch.clone() {
                result.insert("diff".to_string(), JsonVal::Text(patch));
            }
        }
        let mut filediff = JsonMap::new();
        filediff.insert("file".to_string(), JsonVal::Text(file_name));
        if let Some(patch) = patch {
            filediff.insert("patch".to_string(), JsonVal::Text(patch));
        }
        if let Some(additions) = file.get("additions").cloned() {
            filediff.insert("additions".to_string(), additions);
        }
        if let Some(deletions) = file.get("deletions").cloned() {
            filediff.insert("deletions".to_string(), deletions);
        }
        result.insert("filediff".to_string(), JsonVal::Obj(filediff));
        return result;
    }

    if tool == "apply_patch" {
        let mapped: Vec<JsonVal> = files
            .into_iter()
            .map(|mut file| {
                // V2 reports the path relative to the project, and the file's diff under
                // `patch`, one of the two keys V1 entries use.
                if matches!(file.get("filePath"), Some(JsonVal::Text(_)))
                    || !matches!(file.get("file"), Some(JsonVal::Text(_)))
                {
                    return JsonVal::Obj(file);
                }
                if let Some(JsonVal::Text(name)) = file.get("file").cloned() {
                    file.insert("filePath".to_string(), JsonVal::Text(name.clone()));
                    file.insert("relativePath".to_string(), JsonVal::Text(name));
                }
                JsonVal::Obj(file)
            })
            .collect();
        let mut result = metadata.clone();
        result.insert("files".to_string(), JsonVal::Arr(mapped));
        return result;
    }

    metadata.clone()
}

/// Joined text of the `text` items, one per line
/// (TS: `toolContentText`).
pub fn tool_content_text(content: Option<&[V2ContentItem]>) -> String {
    match content {
        None => String::new(),
        Some(items) => items
            .iter()
            .filter_map(|item| {
                if item.kind == "text" {
                    item.text.as_ref().filter(|text| !text.is_empty()).cloned()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Tool-call content item probed for text (TS: `{ type, text? }`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2ContentItem {
    pub kind: String,
    pub text: Option<String>,
}

// ---------------------------------------------------------------------------
// Sessions / projects
// ---------------------------------------------------------------------------

/// Token usage shared by the V2 and V1 session shapes.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenUsage {
    pub input: i64,
    pub output: i64,
    pub reasoning: i64,
    pub cache_read: i64,
    pub cache_write: i64,
}

/// V2 session row (TS: `SessionInfo` fields the mappers read).
#[derive(Debug, Clone, PartialEq)]
pub struct V2SessionInfo {
    pub id: String,
    pub project_id: String,
    pub parent_id: Option<String>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub cost: f64,
    pub tokens: TokenUsage,
    pub title: Option<String>,
    pub directory: String,
    pub created: i64,
    pub updated: i64,
    pub archived: Option<i64>,
}

/// V1 session row (TS: `Session`; `version` is always `"v2"`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Session {
    pub id: String,
    pub slug: String,
    pub project_id: String,
    pub directory: String,
    pub parent_id: Option<String>,
    pub cost: f64,
    pub tokens: TokenUsage,
    pub title: String,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub version: String,
    pub created: i64,
    pub updated: i64,
    pub archived: Option<i64>,
}

/// V1 session row (TS: `Session`).
pub fn to_v1_session(session: &V2SessionInfo) -> V1Session {
    // TS spreads `...(session.agent ? ...)` so empty strings are omitted too.
    let present = |value: &Option<String>| value.as_ref().filter(|s| !s.is_empty()).cloned();
    V1Session {
        id: session.id.clone(),
        slug: session.id.clone(),
        project_id: session.project_id.clone(),
        directory: session.directory.clone(),
        parent_id: present(&session.parent_id),
        cost: session.cost,
        tokens: session.tokens.clone(),
        title: session.title.clone().unwrap_or_default(),
        agent: present(&session.agent),
        model: present(&session.model),
        version: "v2".to_string(),
        created: session.created,
        updated: session.updated,
        archived: session.archived,
    }
}

/// V1 global session: the V1 session plus a null project
/// (TS: `toV1GlobalSession`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1GlobalSession {
    pub inner: V1Session,
    /// Always `None` (TS: `project: null`).
    pub project: Option<String>,
}

/// The V1 session plus a null project (TS: `toV1GlobalSession`).
pub fn to_v1_global_session(session: &V2SessionInfo) -> V1GlobalSession {
    V1GlobalSession { inner: to_v1_session(session), project: None }
}

/// V2 project row (TS: `Project` from `@opencode/client`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2Project {
    pub id: String,
    pub canonical: String,
    pub name: Option<String>,
    pub created: i64,
    pub updated: i64,
    pub sandboxes: Vec<String>,
}

/// V1 project row (TS: `Project`; `worktree` is the V2 `canonical` path).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Project {
    pub id: String,
    pub worktree: String,
    pub name: Option<String>,
    pub created: i64,
    pub updated: i64,
    pub sandboxes: Vec<String>,
}

/// V2 project -> V1 project (TS: `toV1Project`).
pub fn to_v1_project(project: &V2Project) -> V1Project {
    V1Project {
        id: project.id.clone(),
        worktree: project.canonical.clone(),
        name: project.name.clone(),
        created: project.created,
        updated: project.updated,
        sandboxes: project.sandboxes.clone(),
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// V2 structured error (TS: `SessionStructuredError`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2StructuredError {
    pub message: Option<String>,
    /// TS field name `type` (a Rust keyword).
    pub err_type: Option<String>,
}

/// V1 assistant error payload: fixed `UnknownError` name plus the message
/// (TS: `toV1ErrorPayload`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1ErrorPayload {
    pub name: String,
    pub data_message: String,
}

/// Error payload with the `UnknownError` name (TS: `toV1ErrorPayload`).
pub fn to_v1_error_payload(error: Option<&V2StructuredError>) -> V1ErrorPayload {
    // TS: `error?.message || error?.type || "Unknown error"` (empty strings fall through).
    let data_message = error
        .and_then(|err| {
            let message = err.message.as_deref().filter(|s| !s.is_empty());
            let err_type = err.err_type.as_deref().filter(|s| !s.is_empty());
            message.or(err_type).map(str::to_string)
        })
        .unwrap_or_else(|| "Unknown error".to_string());
    V1ErrorPayload { name: "UnknownError".to_string(), data_message }
}

/// Model reference on a V2 assistant message.
#[derive(Debug, Clone, PartialEq)]
pub struct V2ModelRef {
    pub id: String,
    pub provider_id: String,
    pub variant: Option<String>,
}

/// Timestamps on a V2 message.
#[derive(Debug, Clone, PartialEq)]
pub struct V2Time {
    pub created: i64,
    pub completed: Option<i64>,
    pub streamed: Option<i64>,
}

/// V2 user attachment (TS: `source.type === "uri"` picks `source.uri`,
/// otherwise a `data:<mime>;base64,<data>` URL is built).
#[derive(Debug, Clone, PartialEq)]
pub struct V2UserFile {
    pub mime: String,
    pub name: Option<String>,
    pub source_uri: Option<String>,
    pub data: Option<String>,
}

/// V2 user message fields the mapper reads.
#[derive(Debug, Clone, PartialEq)]
pub struct V2UserMessage {
    pub id: String,
    pub created: i64,
    pub text: String,
    pub files: Vec<V2UserFile>,
    pub metadata: Option<MetaVal>,
}

/// V2 tool-call state (TS: `streaming` / `running` / `completed` / error).
#[derive(Debug, Clone, PartialEq)]
pub enum V2ToolState {
    Streaming { raw_input: JsonMap },
    Running { input: JsonMap, metadata: Option<JsonMap> },
    Completed { input: JsonMap, content: Vec<V2ContentItem>, metadata: Option<JsonMap> },
    Failed { input: JsonMap, message: String, metadata: Option<JsonMap> },
}

/// V2 assistant content item.
#[derive(Debug, Clone, PartialEq)]
pub enum V2Content {
    Text { text: String },
    Reasoning { text: String, created: Option<i64>, completed: Option<i64> },
    Tool { id: String, name: String, state: V2ToolState, created: i64, ran: Option<i64>, completed: Option<i64> },
}

/// V2 assistant message fields the mapper reads.
#[derive(Debug, Clone, PartialEq)]
pub struct V2AssistantMessage {
    pub id: String,
    pub agent: String,
    pub model: V2ModelRef,
    pub time: V2Time,
    pub tokens: TokenUsage,
    pub cost: Option<f64>,
    pub error: Option<V2StructuredError>,
    pub finish: Option<String>,
    pub content: Vec<V2Content>,
}

/// V2 message: user and assistant entries have a V1 form, every other
/// entry kind maps to `None` (TS: `toV1Message`).
#[derive(Debug, Clone, PartialEq)]
pub enum V2Message {
    User(V2UserMessage),
    Assistant(V2AssistantMessage),
    Other(String),
}

/// V1 message info half (TS: `UserMessage` / `AssistantMessage`).
#[derive(Debug, Clone, PartialEq)]
pub enum V1MessageInfo {
    User { id: String, session_id: String, created: i64 },
    Assistant {
        id: String,
        session_id: String,
        created: i64,
        completed: Option<i64>,
        streamed: Option<i64>,
        agent: String,
        model_id: String,
        provider_id: String,
        variant: Option<String>,
        finish: Option<String>,
        cost: f64,
        tokens: TokenUsage,
        error: Option<V1ErrorPayload>,
    },
}

/// V1 tool-call state (TS: `ToolState`; `streaming` becomes `pending`).
#[derive(Debug, Clone, PartialEq)]
pub enum V1ToolState {
    Pending { raw: JsonMap },
    Running { input: JsonMap, metadata: Option<JsonMap>, start: i64 },
    Completed { input: JsonMap, output: String, metadata: JsonMap, start: i64, end: i64 },
    Failed { input: JsonMap, error: String, metadata: Option<JsonMap>, start: i64, end: i64 },
}

/// V1 message part.
#[derive(Debug, Clone, PartialEq)]
pub enum V1Part {
    Text { id: String, session_id: String, message_id: String, text: String },
    File { id: String, session_id: String, message_id: String, mime: String, filename: Option<String>, url: String },
    Reasoning { id: String, session_id: String, message_id: String, text: String, start: i64, end: Option<i64> },
    Tool { id: String, session_id: String, message_id: String, call_id: String, tool: String, state: V1ToolState },
}

/// Mapped V1 message with its parts (TS: `V1MessageWithParts`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1MessageWithParts {
    pub info: V1MessageInfo,
    pub parts: Vec<V1Part>,
}

fn to_v1_user_message(message: &V2UserMessage, session_id: &str) -> V1MessageWithParts {
    let info = V1MessageInfo::User {
        id: message.id.clone(),
        session_id: session_id.to_string(),
        created: message.created,
    };
    let mut parts = vec![V1Part::Text {
        id: format!("{}:text", message.id),
        session_id: session_id.to_string(),
        message_id: message.id.clone(),
        text: message.text.clone(),
    }];
    for (index, file) in message.files.iter().enumerate() {
        // TS: `file.source.type === "uri"` picks `source.uri`, otherwise a
        // `data:<mime>;base64,<data>` URL is built.
        let url = match &file.source_uri {
            Some(uri) => uri.clone(),
            None => format!("data:{};base64,{}", file.mime, file.data.as_deref().unwrap_or("")),
        };
        parts.push(V1Part::File {
            id: format!("{}:file:{index}", message.id),
            session_id: session_id.to_string(),
            message_id: message.id.clone(),
            mime: file.mime.clone(),
            filename: file.name.clone(),
            url,
        });
    }
    V1MessageWithParts { info, parts }
}

fn to_v1_tool_state(
    state: &V2ToolState,
    tool_name: &str,
    created: i64,
    ran: Option<i64>,
    completed: Option<i64>,
) -> V1ToolState {
    let start = ran.unwrap_or(created);
    let end = completed.unwrap_or(start);
    match state {
        V2ToolState::Streaming { raw_input } => V1ToolState::Pending { raw: raw_input.clone() },
        V2ToolState::Running { input, metadata } => V1ToolState::Running {
            input: to_v1_tool_input(tool_name, input),
            metadata: metadata.as_ref().map(|m| to_v1_tool_metadata(tool_name, m)),
            start,
        },
        V2ToolState::Completed { input, content, metadata } => V1ToolState::Completed {
            input: to_v1_tool_input(tool_name, input),
            output: tool_content_text(Some(content)),
            metadata: metadata
                .as_ref()
                .map(|m| to_v1_tool_metadata(tool_name, m))
                .unwrap_or_default(),
            start,
            end,
        },
        V2ToolState::Failed { input, message, metadata } => V1ToolState::Failed {
            input: to_v1_tool_input(tool_name, input),
            error: message.clone(),
            metadata: metadata.as_ref().map(|m| to_v1_tool_metadata(tool_name, m)),
            start,
            end,
        },
    }
}

fn to_v1_assistant_message(
    message: &V2AssistantMessage,
    session_id: &str,
    _directory: &str,
) -> V1MessageWithParts {
    // The V1 SDK type has no stream time (`streamed`) and no path; the poll
    // still needs the former, so it is carried on the Rust struct.
    let info = V1MessageInfo::Assistant {
        id: message.id.clone(),
        session_id: session_id.to_string(),
        created: message.time.created,
        completed: message.time.completed,
        streamed: message.time.streamed,
        agent: message.agent.clone(),
        model_id: message.model.id.clone(),
        provider_id: message.model.provider_id.clone(),
        variant: message.model.variant.clone(),
        finish: message.finish.clone(),
        cost: message.cost.unwrap_or(0.0),
        tokens: message.tokens.clone(),
        error: message.error.as_ref().map(|err| to_v1_error_payload(Some(err))),
    };
    let parts = message
        .content
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            V2Content::Text { text } => V1Part::Text {
                id: format!("{}:text:{index}", message.id),
                session_id: session_id.to_string(),
                message_id: message.id.clone(),
                text: text.clone(),
            },
            V2Content::Reasoning { text, created, completed } => V1Part::Reasoning {
                id: format!("{}:reasoning:{index}", message.id),
                session_id: session_id.to_string(),
                message_id: message.id.clone(),
                text: text.clone(),
                start: created.unwrap_or(message.time.created),
                end: *completed,
            },
            V2Content::Tool { id, name, state, created, ran, completed } => {
                let tool_name = to_v1_tool_name(name).to_string();
                V1Part::Tool {
                    id: id.clone(),
                    session_id: session_id.to_string(),
                    message_id: message.id.clone(),
                    call_id: id.clone(),
                    tool: tool_name.clone(),
                    state: to_v1_tool_state(state, &tool_name, *created, *ran, *completed),
                }
            }
        })
        .collect();
    V1MessageWithParts { info, parts }
}

/// User and assistant entries map over; goal-internal user entries and
/// other entry kinds map to `None` (TS: `toV1Message`).
pub fn to_v1_message(
    message: &V2Message,
    session_id: &str,
    directory: &str,
) -> Option<V1MessageWithParts> {
    match message {
        V2Message::User(user) => {
            if let Some(metadata) = &user.metadata {
                if is_goal_internal_metadata(metadata) {
                    return None;
                }
            }
            Some(to_v1_user_message(user, session_id))
        }
        V2Message::Assistant(assistant) => {
            Some(to_v1_assistant_message(assistant, session_id, directory))
        }
        V2Message::Other(_) => None,
    }
}

// ---------------------------------------------------------------------------
// File diffs
// ---------------------------------------------------------------------------

/// V2 file-diff row (TS: `FileDiffInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2FileDiff {
    pub file: String,
    pub status: String,
    pub additions: i64,
    pub deletions: i64,
    pub patch: Option<String>,
}

/// V1 file-diff row (TS: `SnapshotFileDiff`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1FileDiff {
    pub file: String,
    pub status: String,
    pub additions: i64,
    pub deletions: i64,
    pub patch: Option<String>,
}

/// Field-for-field file-diff copy (TS: `toV1FileDiff`).
pub fn to_v1_file_diff(diff: &V2FileDiff) -> V1FileDiff {
    V1FileDiff {
        file: diff.file.clone(),
        status: diff.status.clone(),
        additions: diff.additions,
        deletions: diff.deletions,
        patch: diff.patch.clone(),
    }
}

// ---------------------------------------------------------------------------
// Models / providers / agents / commands / MCP / permissions
// ---------------------------------------------------------------------------

/// V2 model capabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct V2ModelCapabilities {
    pub tools: bool,
    pub input: Vec<String>,
    pub output: Vec<String>,
}

/// V2 model cost row (first row wins).
#[derive(Debug, Clone, PartialEq)]
pub struct V2ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// V2 model context limits.
#[derive(Debug, Clone, PartialEq)]
pub struct V2ModelLimit {
    pub context: i64,
    pub output: i64,
}

/// V2 model variant (`settings` defaults to an empty record).
#[derive(Debug, Clone, PartialEq)]
pub struct V2Variant {
    pub id: String,
    pub settings: Option<JsonMap>,
}

/// V2 model row (TS: `ModelInfo` fields the mapper reads).
#[derive(Debug, Clone, PartialEq)]
pub struct V2Model {
    pub id: String,
    pub model_id: String,
    pub provider_id: String,
    pub name: String,
    pub family: Option<String>,
    pub capabilities: V2ModelCapabilities,
    pub cost: Vec<V2ModelCost>,
    pub limit: V2ModelLimit,
    pub status: String,
    pub package: Option<String>,
    /// `time.released` epoch millis; rendered as `YYYY-MM-DD`.
    pub released_ms: i64,
    pub variants: Vec<V2Variant>,
    pub enabled: bool,
}

/// Text/audio/image/video/pdf modality flags.
#[derive(Debug, Clone, PartialEq)]
pub struct V1Modality {
    pub text: bool,
    pub audio: bool,
    pub image: bool,
    pub video: bool,
    pub pdf: bool,
}

/// V1 model capabilities (`temperature` is always true, `interleaved`
/// always false).
#[derive(Debug, Clone, PartialEq)]
pub struct V1ModelCapabilities {
    pub temperature: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub toolcall: bool,
    pub input: V1Modality,
    pub output: V1Modality,
    pub interleaved: bool,
}

/// V1 model cost (missing rows default to zero).
#[derive(Debug, Clone, PartialEq)]
pub struct V1ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// V1 model row (TS: `Model`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Model {
    pub id: String,
    pub provider_id: String,
    pub api_id: String,
    pub api_npm: String,
    pub name: String,
    pub family: Option<String>,
    pub capabilities: V1ModelCapabilities,
    pub cost: V1ModelCost,
    pub limit_context: i64,
    pub limit_output: i64,
    pub status: String,
    /// `time.released` rendered as `YYYY-MM-DD`.
    pub release_date: String,
    pub variants: BTreeMap<String, JsonMap>,
}

/// V2 model -> V1 model (TS: `toV1Model`).
pub fn to_v1_model(model: &V2Model) -> V1Model {
    fn has_modality(list: &[String], modality: &str) -> bool {
        list.iter().any(|entry| entry == modality)
    }
    let input = &model.capabilities.input;
    let output = &model.capabilities.output;
    let cost = model.cost.first();
    let mut variants = BTreeMap::new();
    for variant in &model.variants {
        variants.insert(variant.id.clone(), variant.settings.clone().unwrap_or_default());
    }
    V1Model {
        id: model.id.clone(),
        provider_id: model.provider_id.clone(),
        api_id: model.model_id.clone(),
        api_npm: model.package.clone().unwrap_or_default(),
        name: model.name.clone(),
        family: model.family.as_ref().filter(|s| !s.is_empty()).cloned(),
        capabilities: V1ModelCapabilities {
            temperature: true,
            reasoning: model.variants.iter().any(|variant| variant.id != "none"),
            attachment: input.iter().any(|modality| modality != "text"),
            toolcall: model.capabilities.tools,
            input: V1Modality {
                text: has_modality(input, "text"),
                audio: has_modality(input, "audio"),
                image: has_modality(input, "image"),
                video: has_modality(input, "video"),
                pdf: has_modality(input, "pdf"),
            },
            output: V1Modality {
                text: has_modality(output, "text"),
                audio: has_modality(output, "audio"),
                image: has_modality(output, "image"),
                video: has_modality(output, "video"),
                pdf: has_modality(output, "pdf"),
            },
            interleaved: false,
        },
        cost: V1ModelCost {
            input: cost.map(|row| row.input).unwrap_or(0.0),
            output: cost.map(|row| row.output).unwrap_or(0.0),
            cache_read: cost.map(|row| row.cache_read).unwrap_or(0.0),
            cache_write: cost.map(|row| row.cache_write).unwrap_or(0.0),
        },
        limit_context: model.limit.context,
        limit_output: model.limit.output,
        status: model.status.clone(),
        release_date: epoch_ms_to_date(model.released_ms),
        variants,
    }
}

/// Render epoch millis as `YYYY-MM-DD` (TS: `new Date(ms).toISOString().slice(0, 10)`).
/// No date dependency in this crate, so days are converted with
/// Howard Hinnant's `civil_from_days` algorithm.
fn epoch_ms_to_date(ms: i64) -> String {
    let days = ms.div_euclid(1000).div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    if month <= 2 {
        year += 1;
    }
    format!("{year:04}-{month:02}-{day:02}")
}

/// V2 provider name lookup row.
#[derive(Debug, Clone, PartialEq)]
pub struct V2ProviderRef {
    pub id: String,
    pub name: String,
}

/// V1 provider with its enabled models (TS: `Provider`; `source` is
/// always `"api"`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Provider {
    pub id: String,
    pub name: String,
    pub models: BTreeMap<String, V1Model>,
}

/// Enabled V2 models folded into V1 providers, grouped by provider
/// (TS: `toV1Providers`; unknown provider names fall back to the id).
pub fn to_v1_providers(providers: &[V2ProviderRef], models: &[V2Model]) -> Vec<V1Provider> {
    let mut out: Vec<V1Provider> = Vec::new();
    for model in models.iter().filter(|model| model.enabled) {
        let index = match out.iter().position(|provider| provider.id == model.provider_id) {
            Some(index) => index,
            None => {
                let name = providers
                    .iter()
                    .find(|provider| provider.id == model.provider_id)
                    .map(|provider| provider.name.clone())
                    .unwrap_or_else(|| model.provider_id.clone());
                out.push(V1Provider {
                    id: model.provider_id.clone(),
                    name,
                    models: BTreeMap::new(),
                });
                out.len() - 1
            }
        };
        out[index].models.insert(model.id.clone(), to_v1_model(model));
    }
    out
}

/// V2 agent row (TS: `AgentInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2Agent {
    pub id: String,
    pub description: Option<String>,
    pub mode: String,
    pub hidden: bool,
    pub color: Option<String>,
    pub model: Option<V2ModelRef>,
    pub steps: Option<u32>,
}

/// V1 agent row (TS: `Agent`; `name` is the V2 id, `permission` is
/// always empty).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Agent {
    pub name: String,
    pub description: Option<String>,
    pub mode: String,
    pub hidden: bool,
    pub color: Option<String>,
    pub model_id: Option<String>,
    pub model_provider_id: Option<String>,
    pub steps: Option<u32>,
}

/// V2 agent -> V1 agent (TS: `toV1Agent`).
pub fn to_v1_agent(agent: &V2Agent) -> V1Agent {
    V1Agent {
        name: agent.id.clone(),
        description: agent.description.clone(),
        mode: agent.mode.clone(),
        hidden: agent.hidden,
        color: agent.color.clone(),
        model_id: agent.model.as_ref().map(|model| model.id.clone()),
        model_provider_id: agent.model.as_ref().map(|model| model.provider_id.clone()),
        steps: agent.steps,
    }
}

/// V2 command / skill rows.
#[derive(Debug, Clone, PartialEq)]
pub struct V2CommandDef {
    pub name: String,
    pub description: Option<String>,
}

/// V2 skill row (TS: `SkillInfo` fields the mapper reads).
#[derive(Debug, Clone, PartialEq)]
pub struct V2SkillDef {
    pub id: String,
    pub description: Option<String>,
}

/// V1 command row (TS: `Command`; `template` is always `""`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Command {
    pub name: String,
    pub description: Option<String>,
    pub source: String,
    pub template: String,
}

/// Commands then skills, each with its source tag
/// (TS: `toV1Commands`).
pub fn to_v1_commands(commands: &[V2CommandDef], skills: &[V2SkillDef]) -> Vec<V1Command> {
    commands
        .iter()
        .map(|command| V1Command {
            name: command.name.clone(),
            description: command.description.clone(),
            source: "command".to_string(),
            template: String::new(),
        })
        .chain(skills.iter().map(|skill| V1Command {
            name: skill.id.clone(),
            description: skill.description.clone(),
            source: "skill".to_string(),
            template: String::new(),
        }))
        .collect()
}

/// V2 MCP server row (TS: `McpServer` fields the mapper reads).
#[derive(Debug, Clone, PartialEq)]
pub struct V2McpServer {
    pub name: String,
    pub status: String,
    pub error: Option<String>,
}

/// V1 MCP status row (TS: `McpStatus`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1McpStatus {
    pub status: String,
    pub error: Option<String>,
}

/// MCP servers keyed by name; unknown statuses become `failed`
/// (TS: `toV1McpStatus`).
pub fn to_v1_mcp_status(servers: &[V2McpServer]) -> BTreeMap<String, V1McpStatus> {
    servers
        .iter()
        .map(|server| {
            let status = match server.status.as_str() {
                "connected" | "disabled" => {
                    V1McpStatus { status: server.status.clone(), error: None }
                }
                "needs_auth" => {
                    V1McpStatus { status: "needs_auth".to_string(), error: None }
                }
                "failed" => {
                    V1McpStatus { status: "failed".to_string(), error: server.error.clone() }
                }
                unknown => {
                    V1McpStatus { status: "failed".to_string(), error: Some(unknown.to_string()) }
                }
            };
            (server.name.clone(), status)
        })
        .collect()
}

/// V2 permission request (TS: `PermissionRequest` from `@opencode/client`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2PermissionRequest {
    pub id: String,
    pub session_id: String,
    pub action: String,
    pub resources: Vec<String>,
    pub metadata: Option<JsonMap>,
    pub save: Option<Vec<String>>,
}

/// V1 permission request (TS: `PermissionRequest`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Permission {
    pub id: String,
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
    pub metadata: JsonMap,
    pub always: Vec<String>,
}

/// V2 permission -> V1 permission (TS: `toV1Permission`).
pub fn to_v1_permission(request: &V2PermissionRequest) -> V1Permission {
    V1Permission {
        id: request.id.clone(),
        session_id: request.session_id.clone(),
        permission: request.action.clone(),
        patterns: request.resources.clone(),
        metadata: request.metadata.clone().unwrap_or_default(),
        always: request.save.clone().unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// Forms / questions / answers
// ---------------------------------------------------------------------------

/// Default answer a form field carries for visibility checks.
#[derive(Debug, Clone, PartialEq)]
pub enum FormDefault {
    Bool(bool),
    Num(f64),
    Text(String),
    Null,
}

/// Visibility condition on a form field (TS: `{ key, op, value }`;
/// `op` is `"eq"` or anything else meaning "not equal").
#[derive(Debug, Clone, PartialEq)]
pub struct V2Condition {
    pub key: String,
    pub op: String,
    pub value: FormDefault,
}

/// Choice on a string/multiselect field.
#[derive(Debug, Clone, PartialEq)]
pub struct V2FormOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

/// V2 form field (TS: `FormField` fields the mappers read).
#[derive(Debug, Clone, PartialEq)]
pub struct V2FormField {
    pub key: String,
    pub field_type: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub hidden: bool,
    pub custom: Option<bool>,
    pub options: Vec<V2FormOption>,
    pub when: Vec<V2Condition>,
    pub default: FormDefault,
}

/// V2 form (TS: `FormInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2Form {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub fields: Vec<V2FormField>,
}

/// Question option; `value` is how the server expects the choice back.
#[derive(Debug, Clone, PartialEq)]
pub struct V1QuestionOption {
    pub label: String,
    pub description: String,
    pub value: Option<String>,
}

/// Question built from one form field (`multiple` only for multiselect;
/// `custom` is `None` when the TS object omits the key).
#[derive(Debug, Clone, PartialEq)]
pub struct V1Question {
    pub question: String,
    pub header: String,
    pub options: Vec<V1QuestionOption>,
    pub multiple: bool,
    pub custom: Option<bool>,
}

/// Question request built from a form (TS: `QuestionRequest`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1QuestionRequest {
    pub id: String,
    pub session_id: String,
    pub questions: Vec<V1Question>,
}

/// The fields shown as questions: `hidden` fields are dropped and `when`
/// conditions are evaluated against the fields' defaults
/// (TS: `visibleFormFields`).
pub fn visible_form_fields(form: &V2Form) -> Vec<V2FormField> {
    form.fields
        .iter()
        .filter(|field| {
            if field.hidden {
                return false;
            }
            if field.when.is_empty() {
                return true;
            }
            field.when.iter().all(|condition| {
                // TS: `answers.get(condition.key) === condition.value`; a missing
                // key reads `undefined`, which never strictly equals a default.
                let value = form
                    .fields
                    .iter()
                    .find(|other| other.key == condition.key)
                    .map(|other| &other.default);
                let matches = value == Some(&condition.value);
                if condition.op == "eq" { matches } else { !matches }
            })
        })
        .cloned()
        .collect()
}

fn field_header(field: &V2FormField, form: &V2Form) -> String {
    field.title.clone().unwrap_or_else(|| form.title.clone())
}

fn field_question_text(field: &V2FormField, form: &V2Form) -> String {
    let mut parts = vec![field_header(field, form)];
    if let Some(description) = field.description.as_ref().filter(|s| !s.is_empty()) {
        parts.push(description.clone());
    }
    if field.field_type == "external" {
        if let Some(url) = field.url.as_ref().filter(|s| !s.is_empty()) {
            parts.push(url.clone());
        }
    }
    let joined = parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if joined.is_empty() { field.key.clone() } else { joined }
}

fn to_question_option(option: &V2FormOption) -> V1QuestionOption {
    V1QuestionOption {
        label: option.label.clone(),
        description: option.description.clone().unwrap_or_default(),
        value: Some(option.value.clone()),
    }
}

fn to_question_info(field: &V2FormField, form: &V2Form) -> V1Question {
    let question = field_question_text(field, form);
    let header = field_header(field, form);
    if field.field_type == "multiselect" {
        return V1Question {
            question,
            header,
            options: field.options.iter().map(to_question_option).collect(),
            multiple: true,
            custom: Some(field.custom == Some(true)),
        };
    }
    if field.field_type == "string" && !field.options.is_empty() {
        return V1Question {
            question,
            header,
            options: field.options.iter().map(to_question_option).collect(),
            multiple: false,
            custom: Some(field.custom == Some(true)),
        };
    }
    if field.field_type == "boolean" {
        return V1Question {
            question,
            header,
            options: vec![
                V1QuestionOption {
                    label: "true".to_string(),
                    description: String::new(),
                    value: Some("true".to_string()),
                },
                V1QuestionOption {
                    label: "false".to_string(),
                    description: String::new(),
                    value: Some("false".to_string()),
                },
            ],
            multiple: false,
            custom: Some(false),
        };
    }
    V1Question { question, header, options: vec![], multiple: false, custom: None }
}

/// Visible form fields mapped into the question UI; a form with no
/// visible fields yields one question carrying the form title
/// (TS: `toV1Question`).
pub fn to_v1_question(form: &V2Form) -> V1QuestionRequest {
    let fields = visible_form_fields(form);
    let questions = if fields.is_empty() {
        vec![V1Question {
            question: form.title.clone(),
            header: form.title.clone(),
            options: vec![],
            multiple: false,
            custom: None,
        }]
    } else {
        fields.iter().map(|field| to_question_info(field, form)).collect()
    };
    V1QuestionRequest { id: form.id.clone(), session_id: form.session_id.clone(), questions }
}

fn option_value(options: &[V2FormOption], item: &str) -> String {
    // A chosen value that is another choice's label resolves to its own value first.
    if options.iter().any(|option| option.value == item) {
        return item.to_string();
    }
    options
        .iter()
        .find(|option| option.label == item)
        .map(|option| option.value.clone())
        .unwrap_or_else(|| item.to_string())
}

fn to_field_answer(
    field: &V2FormField,
    answer: &[String],
) -> Result<Option<FormAnswerValue>, String> {
    let invalid = || format!("Answer for \"{}\" is not a valid {}", field.key, field.field_type);
    match field.field_type.as_str() {
        "multiselect" => Ok(Some(FormAnswerValue::List(
            answer.iter().map(|item| option_value(&field.options, item)).collect(),
        ))),
        "string" => Ok(answer
            .first()
            .map(|first| FormAnswerValue::Text(option_value(&field.options, first)))),
        "boolean" => Ok(answer
            .first()
            .map(|first| FormAnswerValue::Bool(first.trim().to_lowercase() == "true"))),
        "number" | "integer" => {
            let first = match answer.first() {
                None => return Ok(None),
                Some(first) => first,
            };
            let value: f64 = first.trim().parse().map_err(|_| invalid())?;
            if !value.is_finite() || (field.field_type == "integer" && value.fract() != 0.0) {
                return Err(invalid());
            }
            Ok(Some(FormAnswerValue::Num(value)))
        }
        _ => Ok(None),
    }
}

/// Per-question Telegram answers back into a V2 form answer; invalid
/// numbers are an error (TS: `toFormAnswer`, which throws
/// `Answer for "<key>" is not a valid <type>`).
pub fn to_form_answer(
    form: &V2Form,
    answers: &[Vec<String>],
) -> Result<BTreeMap<String, FormAnswerValue>, String> {
    let mut result = BTreeMap::new();
    for (index, field) in visible_form_fields(form).iter().enumerate() {
        let answer: &[String] = answers.get(index).map(Vec::as_slice).unwrap_or(&[]);
        if let Some(value) = to_field_answer(field, answer)? {
            result.insert(field.key.clone(), value);
        }
    }
    Ok(result)
}

/// Converted per-question answer value.
#[derive(Debug, Clone, PartialEq)]
pub enum FormAnswerValue {
    Text(String),
    Num(f64),
    Bool(bool),
    List(Vec<String>),
}

// ---------------------------------------------------------------------------
// Prompt parts
// ---------------------------------------------------------------------------

/// V1 prompt part (TS: `TextPartInput | FilePartInput`).
#[derive(Debug, Clone, PartialEq)]
pub struct V1PromptPart {
    pub part_type: String,
    pub text: Option<String>,
    pub mime: Option<String>,
    pub filename: Option<String>,
    pub url: Option<String>,
}

/// V2 file attachment.
#[derive(Debug, Clone, PartialEq)]
pub struct V2PromptFile {
    pub uri: String,
    pub name: Option<String>,
}

/// Split V1 prompt input (TS: `toV2PromptInput`).
#[derive(Debug, Clone, PartialEq)]
pub struct V2PromptInput {
    pub text: String,
    pub files: Vec<V2PromptFile>,
}

/// V1 prompt parts split into V2 prompt text and file attachments
/// (TS: `toV2PromptInput`).
pub fn to_v2_prompt_input(parts: &[V1PromptPart]) -> V2PromptInput {
    let text = parts
        .iter()
        .filter(|part| part.part_type == "text")
        .filter_map(|part| part.text.clone())
        .collect::<Vec<_>>()
        .join("\n\n");
    let files = parts
        .iter()
        .filter(|part| part.part_type == "file")
        .map(|part| V2PromptFile {
            uri: part.url.clone().unwrap_or_default(),
            name: part.filename.clone(),
        })
        .collect();
    V2PromptInput { text, files }
}
