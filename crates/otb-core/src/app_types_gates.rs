//! App domain type vocabularies + keyboard/pinned shape gates (slice 118).
//!
//! Rust port of the pure remainder in the ten `src/` modules with no
//! dedicated gates owner (full-path coverage audit):
//! - `src/app/types/interaction.ts` (`InteractionKind`, `ExpectedInput`,
//!   `IncomingInputType`, `BlockReason`, `StatefulInteractionKind` as the
//!   `InteractionPayloads` key set, `WaitingAgentRequest` kind tag)
//! - `src/app/types/permission.ts` (`PermissionReply`, `PermissionOutcome`
//!   kind tags; callback data `permission:once` / `:always` / `:reject` in
//!   `src/bot/menus/permission-menu.ts`)
//! - `src/app/types/question.ts` (`QuestionSettledOutcome`,
//!   `QuestionInFlightEnding`, the `custom?: boolean` "unstated means
//!   accepted" rule branched as `question.custom !== false` in
//!   `src/bot/menus/question-menu.ts`)
//! - `src/app/types/rename.ts` (`RenameState` shape)
//! - `src/app/types/settings.ts` (`ResponseStreamingMode`, `PromptQueueMode`,
//!   the `"off" | "all" | "auto"` TTS mode branched in
//!   `src/bot/menus/settings-menu.ts`)
//! - `src/app/types/summary.ts` (`FileChange` shape)
//! - `src/app/types/variant.ts` (re-export of `VariantInfo`; the
//!   `model.variant || "default"` blank gate cited in slice 99 over
//!   `src/bot/keyboards/keyboard-manager.ts`)
//! - `src/app/types/worktree.ts` (`GitWorktreeEntry`, `GitWorktreeContext`,
//!   `ListedWorktree` shapes)
//! - `src/bot/keyboards/keyboard-types.ts` (`ContextInfo`, `KeyboardState`
//!   shapes)
//! - `src/bot/pinned/pinned-message-types.ts` (`TokensInfo`,
//!   `PinnedMessageState` shapes; the empty-diff-removes-the-list rule from
//!   PRODUCT.md grounds the changed-files predicate)
//!
//! Step 2 (GREEN): pure port of the TS vocabularies; the parity tests in
//! `crates/otb-core/tests/slice118_app_types_parity.rs` assert the TS
//! contract.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `t(...)` locale lookups (keys travel as literal strings)
//! - grammy keyboards / Telegram I/O, interaction-manager state, settings
//!   persistence, session diff fetching.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` / `resolveSupportedLocale` / `normalizeLocale`
//!   (`i18n` owner; the `src/i18n/*.ts` locale dictionaries are data)
//! - interaction-guard decisions (`interaction_guard_edge_gates` owner)
//! - worktree button labels (`worktree_history_inline_gates` owner)
//! - keyboard debounce / fresh-context zero gate (`render_interaction`
//!   owner via `keyboard-manager.ts`)
//! - model display composition (`types_config` / `command_session_gates`
//!   owners).
//!
//! Deviations: none intended.
//! - TS string unions become enums; `parse_*` returns `None` for unknown
//!   text (same nullable convention as `notice_health_gates`).

// ---------------------------------------------------------------------------
// Interaction kinds (`src/app/types/interaction.ts`)
// ---------------------------------------------------------------------------

/// Slot kind (TS: `InteractionKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKind {
    Inline,
    Permission,
    Question,
    Rename,
    Task,
    Custom,
}

/// Parse an interaction kind (TS: the `InteractionKind` union).
pub fn parse_interaction_kind(raw: &str) -> Option<InteractionKind> {
    match raw {
        "inline" => Some(InteractionKind::Inline),
        "permission" => Some(InteractionKind::Permission),
        "question" => Some(InteractionKind::Question),
        "rename" => Some(InteractionKind::Rename),
        "task" => Some(InteractionKind::Task),
        "custom" => Some(InteractionKind::Custom),
        _ => None,
    }
}

/// Canonical text of an interaction kind (TS: the union member itself).
pub fn interaction_kind_as_str(kind: InteractionKind) -> &'static str {
    match kind {
        InteractionKind::Inline => "inline",
        InteractionKind::Permission => "permission",
        InteractionKind::Question => "question",
        InteractionKind::Rename => "rename",
        InteractionKind::Task => "task",
        InteractionKind::Custom => "custom",
    }
}

/// Whether the kind carries its own state payload (TS:
/// `StatefulInteractionKind` = keys of `InteractionPayloads`).
pub fn is_stateful_interaction_kind(kind: InteractionKind) -> bool {
    matches!(
        kind,
        InteractionKind::Question
            | InteractionKind::Permission
            | InteractionKind::Rename
            | InteractionKind::Task
    )
}

/// Expected input channel (TS: `ExpectedInput`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedInput {
    Callback,
    Text,
    Command,
    Mixed,
}

/// Parse an expected-input marker (TS: the `ExpectedInput` union).
pub fn parse_expected_input(raw: &str) -> Option<ExpectedInput> {
    match raw {
        "callback" => Some(ExpectedInput::Callback),
        "text" => Some(ExpectedInput::Text),
        "command" => Some(ExpectedInput::Command),
        "mixed" => Some(ExpectedInput::Mixed),
        _ => None,
    }
}

/// Canonical text of an expected-input marker.
pub fn expected_input_as_str(input: ExpectedInput) -> &'static str {
    match input {
        ExpectedInput::Callback => "callback",
        ExpectedInput::Text => "text",
        ExpectedInput::Command => "command",
        ExpectedInput::Mixed => "mixed",
    }
}

/// Incoming update channel (TS: `IncomingInputType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingInputType {
    Callback,
    Command,
    Text,
    Other,
}

/// Parse an incoming-input marker (TS: the `IncomingInputType` union).
pub fn parse_incoming_input_type(raw: &str) -> Option<IncomingInputType> {
    match raw {
        "callback" => Some(IncomingInputType::Callback),
        "command" => Some(IncomingInputType::Command),
        "text" => Some(IncomingInputType::Text),
        "other" => Some(IncomingInputType::Other),
        _ => None,
    }
}

/// Canonical text of an incoming-input marker.
pub fn incoming_input_type_as_str(input: IncomingInputType) -> &'static str {
    match input {
        IncomingInputType::Callback => "callback",
        IncomingInputType::Command => "command",
        IncomingInputType::Text => "text",
        IncomingInputType::Other => "other",
    }
}

/// Why an input was blocked (TS: `BlockReason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    Expired,
    ExpectedCallback,
    ExpectedText,
    ExpectedCommand,
    CommandNotAllowed,
}

/// Parse a block reason (TS: the `BlockReason` union).
pub fn parse_block_reason(raw: &str) -> Option<BlockReason> {
    match raw {
        "expired" => Some(BlockReason::Expired),
        "expected_callback" => Some(BlockReason::ExpectedCallback),
        "expected_text" => Some(BlockReason::ExpectedText),
        "expected_command" => Some(BlockReason::ExpectedCommand),
        "command_not_allowed" => Some(BlockReason::CommandNotAllowed),
        _ => None,
    }
}

/// Canonical text of a block reason.
pub fn block_reason_as_str(reason: BlockReason) -> &'static str {
    match reason {
        BlockReason::Expired => "expired",
        BlockReason::ExpectedCallback => "expected_callback",
        BlockReason::ExpectedText => "expected_text",
        BlockReason::ExpectedCommand => "expected_command",
        BlockReason::CommandNotAllowed => "command_not_allowed",
    }
}

/// Which queued agent request waits on the slot (TS: the
/// `WaitingAgentRequest` kind tag).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingAgentRequestKind {
    Question,
    Permission,
}

/// Parse a waiting-request kind tag.
pub fn parse_waiting_agent_request_kind(raw: &str) -> Option<WaitingAgentRequestKind> {
    match raw {
        "question" => Some(WaitingAgentRequestKind::Question),
        "permission" => Some(WaitingAgentRequestKind::Permission),
        _ => None,
    }
}

/// Canonical text of a waiting-request kind tag.
pub fn waiting_agent_request_kind_as_str(kind: WaitingAgentRequestKind) -> &'static str {
    match kind {
        WaitingAgentRequestKind::Question => "question",
        WaitingAgentRequestKind::Permission => "permission",
    }
}

// ---------------------------------------------------------------------------
// Permission replies (`src/app/types/permission.ts`)
// ---------------------------------------------------------------------------

/// Possible permission responses (TS: `PermissionReply`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionReply {
    Once,
    Always,
    Reject,
}

/// Parse a permission reply (TS: `"once" | "always" | "reject"`, also the
/// `permission:<reply>` callback suffix in `permission-menu.ts`).
pub fn parse_permission_reply(raw: &str) -> Option<PermissionReply> {
    match raw {
        "once" => Some(PermissionReply::Once),
        "always" => Some(PermissionReply::Always),
        "reject" => Some(PermissionReply::Reject),
        _ => None,
    }
}

/// Canonical text of a permission reply.
pub fn permission_reply_as_str(reply: PermissionReply) -> &'static str {
    match reply {
        PermissionReply::Once => "once",
        PermissionReply::Always => "always",
        PermissionReply::Reject => "reject",
    }
}

/// How a permission prompt ended (TS: the `PermissionOutcome` kind tags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOutcomeKind {
    Replied,
    SettledOutside,
    NotAnswered,
}

/// Parse a permission-outcome kind tag.
pub fn parse_permission_outcome_kind(raw: &str) -> Option<PermissionOutcomeKind> {
    match raw {
        "replied" => Some(PermissionOutcomeKind::Replied),
        "settled_outside" => Some(PermissionOutcomeKind::SettledOutside),
        "not_answered" => Some(PermissionOutcomeKind::NotAnswered),
        _ => None,
    }
}

/// Canonical text of a permission-outcome kind tag.
pub fn permission_outcome_kind_as_str(kind: PermissionOutcomeKind) -> &'static str {
    match kind {
        PermissionOutcomeKind::Replied => "replied",
        PermissionOutcomeKind::SettledOutside => "settled_outside",
        PermissionOutcomeKind::NotAnswered => "not_answered",
    }
}

// ---------------------------------------------------------------------------
// Question settlement (`src/app/types/question.ts`)
// ---------------------------------------------------------------------------

/// How a question was settled outside Telegram (TS:
/// `QuestionSettledOutcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionSettledOutcome {
    Answered,
    Cancelled,
}

/// Parse a settled outcome (TS: `"answered" | "cancelled"`).
pub fn parse_question_settled_outcome(raw: &str) -> Option<QuestionSettledOutcome> {
    match raw {
        "answered" => Some(QuestionSettledOutcome::Answered),
        "cancelled" => Some(QuestionSettledOutcome::Cancelled),
        _ => None,
    }
}

/// Canonical text of a settled outcome.
pub fn question_settled_outcome_as_str(outcome: QuestionSettledOutcome) -> &'static str {
    match outcome {
        QuestionSettledOutcome::Answered => "answered",
        QuestionSettledOutcome::Cancelled => "cancelled",
    }
}

/// How a question ended before its poll reached the chat (TS:
/// `QuestionInFlightEnding`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionInFlightEnding {
    Answered,
    Cancelled,
    NotAnswered,
    Replaced,
}

/// Parse an in-flight ending tag.
pub fn parse_question_in_flight_ending(raw: &str) -> Option<QuestionInFlightEnding> {
    match raw {
        "answered" => Some(QuestionInFlightEnding::Answered),
        "cancelled" => Some(QuestionInFlightEnding::Cancelled),
        "not_answered" => Some(QuestionInFlightEnding::NotAnswered),
        "replaced" => Some(QuestionInFlightEnding::Replaced),
        _ => None,
    }
}

/// Canonical text of an in-flight ending tag.
pub fn question_in_flight_ending_as_str(ending: QuestionInFlightEnding) -> &'static str {
    match ending {
        QuestionInFlightEnding::Answered => "answered",
        QuestionInFlightEnding::Cancelled => "cancelled",
        QuestionInFlightEnding::NotAnswered => "not_answered",
        QuestionInFlightEnding::Replaced => "replaced",
    }
}

/// Whether a question accepts a custom answer (TS: `custom?: boolean`,
/// "unstated means accepted", branched as `question.custom !== false` in
/// `question-menu.ts`).
pub fn is_custom_answer_accepted(custom: Option<bool>) -> bool {
    custom != Some(false)
}

// ---------------------------------------------------------------------------
// Settings modes (`src/app/types/settings.ts`)
// ---------------------------------------------------------------------------

/// Assistant streaming mode (TS: `ResponseStreamingMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStreamingMode {
    Edit,
    Draft,
}

/// Parse a streaming mode (TS: `"edit" | "draft"`).
pub fn parse_response_streaming_mode(raw: &str) -> Option<ResponseStreamingMode> {
    match raw {
        "edit" => Some(ResponseStreamingMode::Edit),
        "draft" => Some(ResponseStreamingMode::Draft),
        _ => None,
    }
}

/// Canonical text of a streaming mode.
pub fn response_streaming_mode_as_str(mode: ResponseStreamingMode) -> &'static str {
    match mode {
        ResponseStreamingMode::Edit => "edit",
        ResponseStreamingMode::Draft => "draft",
    }
}

/// Mid-run message routing (TS: `PromptQueueMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptQueueMode {
    Off,
    Queue,
    Steer,
}

/// Parse a prompt-queue mode (TS: `"off" | "queue" | "steer"`).
pub fn parse_prompt_queue_mode(raw: &str) -> Option<PromptQueueMode> {
    match raw {
        "off" => Some(PromptQueueMode::Off),
        "queue" => Some(PromptQueueMode::Queue),
        "steer" => Some(PromptQueueMode::Steer),
        _ => None,
    }
}

/// Canonical text of a prompt-queue mode.
pub fn prompt_queue_mode_as_str(mode: PromptQueueMode) -> &'static str {
    match mode {
        PromptQueueMode::Off => "off",
        PromptQueueMode::Queue => "queue",
        PromptQueueMode::Steer => "steer",
    }
}

/// Audio-reply mode (TS: `ttsMode`, `"off" | "all" | "auto"` in
/// `settings-menu.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsMode {
    Off,
    All,
    Auto,
}

/// Parse an audio-reply mode.
pub fn parse_tts_mode(raw: &str) -> Option<TtsMode> {
    match raw {
        "off" => Some(TtsMode::Off),
        "all" => Some(TtsMode::All),
        "auto" => Some(TtsMode::Auto),
        _ => None,
    }
}

/// Canonical text of an audio-reply mode.
pub fn tts_mode_as_str(mode: TtsMode) -> &'static str {
    match mode {
        TtsMode::Off => "off",
        TtsMode::All => "all",
        TtsMode::Auto => "auto",
    }
}

// ---------------------------------------------------------------------------
// Variant fallback (`src/app/types/variant.ts` via keyboard-manager gate)
// ---------------------------------------------------------------------------

/// Display name used when no variant is set (TS: the `"default"` in
/// `model.variant || "default"`).
pub const DEFAULT_VARIANT_NAME: &str = "default";

/// Resolve the display variant (TS: `model.variant || "default"`, blank
/// included).
pub fn resolve_variant_name(variant: Option<&str>) -> &str {
    match variant {
        Some(name) if !name.is_empty() => name,
        _ => DEFAULT_VARIANT_NAME,
    }
}

// ---------------------------------------------------------------------------
// Shapes (`summary.ts`, `rename.ts`, `worktree.ts`, keyboard/pinned types)
// ---------------------------------------------------------------------------

/// One changed file (TS: `FileChange` in `src/app/types/summary.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub file: String,
    pub additions: u32,
    pub deletions: u32,
}

/// Whether the change counts (non-zero additions or deletions).
pub fn has_file_changes(change: &FileChange) -> bool {
    change.additions != 0 || change.deletions != 0
}

/// Rename interaction payload (TS: `RenameState` in
/// `src/app/types/rename.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameState {
    pub session_id: String,
    pub session_directory: String,
    pub current_title: String,
    pub message_id: Option<i64>,
}

/// One git worktree row (TS: `GitWorktreeEntry` in
/// `src/app/types/worktree.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeEntry {
    pub path: String,
    pub branch: Option<String>,
    pub is_current: bool,
    pub is_main: bool,
}

/// Worktree context for the current project (TS: `GitWorktreeContext`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeContext {
    pub main_project_path: String,
    pub active_worktree_path: String,
    pub branch: Option<String>,
    pub is_linked_worktree: bool,
    pub worktrees: Vec<GitWorktreeEntry>,
}

/// A listed worktree row with its position (TS: `ListedWorktree`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedWorktree {
    pub entry: GitWorktreeEntry,
    pub git_index: usize,
}

/// Token counts for the reply keyboard (TS: `ContextInfo` in
/// `src/bot/keyboards/keyboard-types.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextInfo {
    pub tokens_used: u64,
    pub tokens_limit: u64,
}

/// Reply-keyboard state (TS: `KeyboardState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardState {
    pub current_agent: String,
    pub current_model_provider: String,
    pub current_model_id: String,
    pub context_info: Option<ContextInfo>,
    pub variant_name: Option<String>,
}

/// Whether the keyboard carries context info.
pub fn has_keyboard_context(state: &KeyboardState) -> bool {
    state.context_info.is_some()
}

/// Token breakdown (TS: `TokensInfo` in
/// `src/bot/pinned/pinned-message-types.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokensInfo {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Pinned dashboard state (TS: `PinnedMessageState`).
#[derive(Debug, Clone, PartialEq)]
pub struct PinnedMessageState {
    pub message_id: Option<i64>,
    pub chat_id: Option<i64>,
    pub session_id: Option<String>,
    pub session_title: String,
    pub attach_active: bool,
    pub attach_busy: bool,
    pub project_path: String,
    pub project_branch: Option<String>,
    pub project_worktree_path: Option<String>,
    pub tokens_used: u64,
    pub tokens_limit: u64,
    pub last_updated: i64,
    pub changed_files: Vec<FileChange>,
    pub cost: Option<f64>,
}

/// Whether the dashboard is bound to a session.
pub fn has_pinned_session(state: &PinnedMessageState) -> bool {
    state.session_id.is_some()
}

/// Whether the dashboard lists changed files (TS: an empty diff removes the
/// list).
pub fn has_pinned_changed_files(state: &PinnedMessageState) -> bool {
    !state.changed_files.is_empty()
}
