//! Slice 118 parity: app domain type vocabularies + keyboard/pinned shapes (RED).
//!
//! Mirrors the pure TS contract in the ten `src/` modules with no dedicated
//! gates owner (full-path coverage audit):
//! - `src/app/types/interaction.ts` (`InteractionKind` six members,
//!   `ExpectedInput`, `IncomingInputType`, `BlockReason` five members,
//!   `StatefulInteractionKind` = question/permission/rename/task,
//!   `WaitingAgentRequest` question/permission tags)
//! - `src/app/types/permission.ts` (`PermissionReply` once/always/reject,
//!   `PermissionOutcome` replied/settled_outside/not_answered tags)
//! - `src/app/types/question.ts` (`QuestionSettledOutcome`,
//!   `QuestionInFlightEnding`, `custom !== false` accept rule)
//! - `src/app/types/settings.ts` (`ResponseStreamingMode` edit/draft,
//!   `PromptQueueMode` off/queue/steer, TTS mode off/all/auto)
//! - `src/app/types/summary.ts` (`FileChange` counts), `rename.ts`
//!   (`RenameState`), `variant.ts` (`"default"` fallback),
//!   `src/app/types/worktree.ts` (entry/context/listed shapes)
//! - `src/bot/keyboards/keyboard-types.ts` (`ContextInfo`, `KeyboardState`)
//! - `src/bot/pinned/pinned-message-types.ts` (`TokensInfo`,
//!   `PinnedMessageState`, session binding, empty-diff rule)
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `app_types_gates` is ported (only constant-value tests pass).

use otb_core::app_types_gates::*;

// --- interaction kind (4) ---

#[test]
fn slice118_01_parses_all_interaction_kinds() {
    assert_eq!(
        parse_interaction_kind("inline"),
        Some(InteractionKind::Inline)
    );
    assert_eq!(
        parse_interaction_kind("permission"),
        Some(InteractionKind::Permission)
    );
    assert_eq!(
        parse_interaction_kind("question"),
        Some(InteractionKind::Question)
    );
    assert_eq!(
        parse_interaction_kind("rename"),
        Some(InteractionKind::Rename)
    );
    assert_eq!(parse_interaction_kind("task"), Some(InteractionKind::Task));
    assert_eq!(
        parse_interaction_kind("custom"),
        Some(InteractionKind::Custom)
    );
}

#[test]
fn slice118_02_rejects_unknown_interaction_kind() {
    assert_eq!(parse_interaction_kind("banana"), None);
    assert_eq!(parse_interaction_kind(""), None);
    assert_eq!(parse_interaction_kind("Inline"), None);
}

#[test]
fn slice118_03_interaction_kind_round_trips() {
    assert_eq!(interaction_kind_as_str(InteractionKind::Inline), "inline");
    assert_eq!(
        interaction_kind_as_str(InteractionKind::Permission),
        "permission"
    );
    assert_eq!(
        interaction_kind_as_str(InteractionKind::Question),
        "question"
    );
    assert_eq!(interaction_kind_as_str(InteractionKind::Rename), "rename");
    assert_eq!(interaction_kind_as_str(InteractionKind::Task), "task");
    assert_eq!(interaction_kind_as_str(InteractionKind::Custom), "custom");
}

#[test]
fn slice118_04_only_payload_kinds_are_stateful() {
    assert!(is_stateful_interaction_kind(InteractionKind::Question));
    assert!(is_stateful_interaction_kind(InteractionKind::Permission));
    assert!(is_stateful_interaction_kind(InteractionKind::Rename));
    assert!(is_stateful_interaction_kind(InteractionKind::Task));
    assert!(!is_stateful_interaction_kind(InteractionKind::Inline));
    assert!(!is_stateful_interaction_kind(InteractionKind::Custom));
}

// --- expected input (2) ---

#[test]
fn slice118_05_parses_expected_input() {
    assert_eq!(
        parse_expected_input("callback"),
        Some(ExpectedInput::Callback)
    );
    assert_eq!(parse_expected_input("text"), Some(ExpectedInput::Text));
    assert_eq!(
        parse_expected_input("command"),
        Some(ExpectedInput::Command)
    );
    assert_eq!(parse_expected_input("mixed"), Some(ExpectedInput::Mixed));
    assert_eq!(parse_expected_input("other"), None);
}

#[test]
fn slice118_06_expected_input_round_trips() {
    assert_eq!(expected_input_as_str(ExpectedInput::Callback), "callback");
    assert_eq!(expected_input_as_str(ExpectedInput::Text), "text");
    assert_eq!(expected_input_as_str(ExpectedInput::Command), "command");
    assert_eq!(expected_input_as_str(ExpectedInput::Mixed), "mixed");
}

// --- incoming input type (2) ---

#[test]
fn slice118_07_parses_incoming_input_type() {
    assert_eq!(
        parse_incoming_input_type("callback"),
        Some(IncomingInputType::Callback)
    );
    assert_eq!(
        parse_incoming_input_type("command"),
        Some(IncomingInputType::Command)
    );
    assert_eq!(
        parse_incoming_input_type("text"),
        Some(IncomingInputType::Text)
    );
    assert_eq!(
        parse_incoming_input_type("other"),
        Some(IncomingInputType::Other)
    );
    assert_eq!(parse_incoming_input_type("mixed"), None);
}

#[test]
fn slice118_08_incoming_input_type_round_trips() {
    assert_eq!(
        incoming_input_type_as_str(IncomingInputType::Callback),
        "callback"
    );
    assert_eq!(
        incoming_input_type_as_str(IncomingInputType::Command),
        "command"
    );
    assert_eq!(incoming_input_type_as_str(IncomingInputType::Text), "text");
    assert_eq!(
        incoming_input_type_as_str(IncomingInputType::Other),
        "other"
    );
}

// --- block reason (2) ---

#[test]
fn slice118_09_parses_block_reason() {
    assert_eq!(parse_block_reason("expired"), Some(BlockReason::Expired));
    assert_eq!(
        parse_block_reason("expected_callback"),
        Some(BlockReason::ExpectedCallback)
    );
    assert_eq!(
        parse_block_reason("expected_text"),
        Some(BlockReason::ExpectedText)
    );
    assert_eq!(
        parse_block_reason("expected_command"),
        Some(BlockReason::ExpectedCommand)
    );
    assert_eq!(
        parse_block_reason("command_not_allowed"),
        Some(BlockReason::CommandNotAllowed)
    );
    assert_eq!(parse_block_reason("busy"), None);
}

#[test]
fn slice118_10_block_reason_round_trips() {
    assert_eq!(block_reason_as_str(BlockReason::Expired), "expired");
    assert_eq!(
        block_reason_as_str(BlockReason::ExpectedCallback),
        "expected_callback"
    );
    assert_eq!(
        block_reason_as_str(BlockReason::ExpectedText),
        "expected_text"
    );
    assert_eq!(
        block_reason_as_str(BlockReason::ExpectedCommand),
        "expected_command"
    );
    assert_eq!(
        block_reason_as_str(BlockReason::CommandNotAllowed),
        "command_not_allowed"
    );
}

// --- waiting request kind (1) ---

#[test]
fn slice118_11_parses_waiting_request_kind() {
    assert_eq!(
        parse_waiting_agent_request_kind("question"),
        Some(WaitingAgentRequestKind::Question)
    );
    assert_eq!(
        parse_waiting_agent_request_kind("permission"),
        Some(WaitingAgentRequestKind::Permission)
    );
    assert_eq!(parse_waiting_agent_request_kind("rename"), None);
    assert_eq!(
        waiting_agent_request_kind_as_str(WaitingAgentRequestKind::Question),
        "question"
    );
    assert_eq!(
        waiting_agent_request_kind_as_str(WaitingAgentRequestKind::Permission),
        "permission"
    );
}

// --- permission reply (2) ---

#[test]
fn slice118_12_parses_permission_reply() {
    assert_eq!(parse_permission_reply("once"), Some(PermissionReply::Once));
    assert_eq!(
        parse_permission_reply("always"),
        Some(PermissionReply::Always)
    );
    assert_eq!(
        parse_permission_reply("reject"),
        Some(PermissionReply::Reject)
    );
    assert_eq!(parse_permission_reply("allow"), None);
    assert_eq!(parse_permission_reply(""), None);
}

#[test]
fn slice118_13_permission_reply_round_trips() {
    assert_eq!(permission_reply_as_str(PermissionReply::Once), "once");
    assert_eq!(permission_reply_as_str(PermissionReply::Always), "always");
    assert_eq!(permission_reply_as_str(PermissionReply::Reject), "reject");
}

// --- permission outcome (1) ---

#[test]
fn slice118_14_parses_permission_outcome_kind() {
    assert_eq!(
        parse_permission_outcome_kind("replied"),
        Some(PermissionOutcomeKind::Replied)
    );
    assert_eq!(
        parse_permission_outcome_kind("settled_outside"),
        Some(PermissionOutcomeKind::SettledOutside)
    );
    assert_eq!(
        parse_permission_outcome_kind("not_answered"),
        Some(PermissionOutcomeKind::NotAnswered)
    );
    assert_eq!(parse_permission_outcome_kind("answered"), None);
    assert_eq!(
        permission_outcome_kind_as_str(PermissionOutcomeKind::Replied),
        "replied"
    );
    assert_eq!(
        permission_outcome_kind_as_str(PermissionOutcomeKind::SettledOutside),
        "settled_outside"
    );
    assert_eq!(
        permission_outcome_kind_as_str(PermissionOutcomeKind::NotAnswered),
        "not_answered"
    );
}

// --- question settlement (3) ---

#[test]
fn slice118_15_parses_settled_outcome() {
    assert_eq!(
        parse_question_settled_outcome("answered"),
        Some(QuestionSettledOutcome::Answered)
    );
    assert_eq!(
        parse_question_settled_outcome("cancelled"),
        Some(QuestionSettledOutcome::Cancelled)
    );
    assert_eq!(parse_question_settled_outcome("replaced"), None);
    assert_eq!(
        question_settled_outcome_as_str(QuestionSettledOutcome::Answered),
        "answered"
    );
    assert_eq!(
        question_settled_outcome_as_str(QuestionSettledOutcome::Cancelled),
        "cancelled"
    );
}

#[test]
fn slice118_16_parses_in_flight_ending() {
    assert_eq!(
        parse_question_in_flight_ending("answered"),
        Some(QuestionInFlightEnding::Answered)
    );
    assert_eq!(
        parse_question_in_flight_ending("cancelled"),
        Some(QuestionInFlightEnding::Cancelled)
    );
    assert_eq!(
        parse_question_in_flight_ending("not_answered"),
        Some(QuestionInFlightEnding::NotAnswered)
    );
    assert_eq!(
        parse_question_in_flight_ending("replaced"),
        Some(QuestionInFlightEnding::Replaced)
    );
    assert_eq!(parse_question_in_flight_ending("expired"), None);
    assert_eq!(
        question_in_flight_ending_as_str(QuestionInFlightEnding::NotAnswered),
        "not_answered"
    );
    assert_eq!(
        question_in_flight_ending_as_str(QuestionInFlightEnding::Replaced),
        "replaced"
    );
}

#[test]
fn slice118_17_unstated_custom_means_accepted() {
    assert!(is_custom_answer_accepted(None));
    assert!(is_custom_answer_accepted(Some(true)));
    assert!(!is_custom_answer_accepted(Some(false)));
}

// --- settings modes (3) ---

#[test]
fn slice118_18_parses_streaming_mode() {
    assert_eq!(
        parse_response_streaming_mode("edit"),
        Some(ResponseStreamingMode::Edit)
    );
    assert_eq!(
        parse_response_streaming_mode("draft"),
        Some(ResponseStreamingMode::Draft)
    );
    assert_eq!(parse_response_streaming_mode("live"), None);
    assert_eq!(
        response_streaming_mode_as_str(ResponseStreamingMode::Edit),
        "edit"
    );
    assert_eq!(
        response_streaming_mode_as_str(ResponseStreamingMode::Draft),
        "draft"
    );
}

#[test]
fn slice118_19_parses_prompt_queue_mode() {
    assert_eq!(parse_prompt_queue_mode("off"), Some(PromptQueueMode::Off));
    assert_eq!(
        parse_prompt_queue_mode("queue"),
        Some(PromptQueueMode::Queue)
    );
    assert_eq!(
        parse_prompt_queue_mode("steer"),
        Some(PromptQueueMode::Steer)
    );
    assert_eq!(parse_prompt_queue_mode("on"), None);
    assert_eq!(prompt_queue_mode_as_str(PromptQueueMode::Off), "off");
    assert_eq!(prompt_queue_mode_as_str(PromptQueueMode::Queue), "queue");
    assert_eq!(prompt_queue_mode_as_str(PromptQueueMode::Steer), "steer");
}

#[test]
fn slice118_20_parses_tts_mode() {
    assert_eq!(parse_tts_mode("off"), Some(TtsMode::Off));
    assert_eq!(parse_tts_mode("all"), Some(TtsMode::All));
    assert_eq!(parse_tts_mode("auto"), Some(TtsMode::Auto));
    assert_eq!(parse_tts_mode("on"), None);
    assert_eq!(tts_mode_as_str(TtsMode::Off), "off");
    assert_eq!(tts_mode_as_str(TtsMode::All), "all");
    assert_eq!(tts_mode_as_str(TtsMode::Auto), "auto");
}

// --- variant fallback (2: 1 const passes, 1 RED) ---

#[test]
fn slice118_21_default_variant_name_is_default() {
    assert_eq!(DEFAULT_VARIANT_NAME, "default");
}

#[test]
fn slice118_22_missing_or_blank_variant_falls_back_to_default() {
    assert_eq!(resolve_variant_name(None), "default");
    assert_eq!(resolve_variant_name(Some("")), "default");
    assert_eq!(resolve_variant_name(Some("Pro")), "Pro");
}

// --- file change (1) ---

fn sample_change(additions: u32, deletions: u32) -> FileChange {
    FileChange {
        file: "src/one.ts".to_string(),
        additions,
        deletions,
    }
}

#[test]
fn slice118_23_change_counts_when_either_side_is_nonzero() {
    assert!(has_file_changes(&sample_change(1, 1)));
    assert!(has_file_changes(&sample_change(2, 0)));
    assert!(has_file_changes(&sample_change(0, 3)));
    assert!(!has_file_changes(&sample_change(0, 0)));
}

// --- keyboard context (1) ---

fn keyboard_state(with_context: bool) -> KeyboardState {
    KeyboardState {
        current_agent: "plan".to_string(),
        current_model_provider: "openai".to_string(),
        current_model_id: "gpt-5".to_string(),
        context_info: with_context.then(|| ContextInfo {
            tokens_used: 10,
            tokens_limit: 200,
        }),
        variant_name: None,
    }
}

#[test]
fn slice118_24_keyboard_context_tracks_presence() {
    assert!(has_keyboard_context(&keyboard_state(true)));
    assert!(!has_keyboard_context(&keyboard_state(false)));
}

// --- pinned dashboard (2) ---

fn pinned_state(session: Option<&str>, files: usize) -> PinnedMessageState {
    PinnedMessageState {
        message_id: Some(7),
        chat_id: Some(42),
        session_id: session.map(str::to_string),
        session_title: "new session".to_string(),
        attach_active: false,
        attach_busy: false,
        project_path: "/repo".to_string(),
        project_branch: None,
        project_worktree_path: None,
        tokens_used: 0,
        tokens_limit: 0,
        last_updated: 0,
        changed_files: (0..files)
            .map(|i| FileChange {
                file: format!("src/f{i}.ts"),
                additions: 1,
                deletions: 0,
            })
            .collect(),
        cost: None,
    }
}

#[test]
fn slice118_25_pinned_session_tracks_binding() {
    assert!(has_pinned_session(&pinned_state(Some("ses_1"), 0)));
    assert!(!has_pinned_session(&pinned_state(None, 0)));
}

#[test]
fn slice118_26_empty_diff_means_no_changed_files() {
    assert!(has_pinned_changed_files(&pinned_state(Some("ses_1"), 2)));
    assert!(!has_pinned_changed_files(&pinned_state(Some("ses_1"), 0)));
}
