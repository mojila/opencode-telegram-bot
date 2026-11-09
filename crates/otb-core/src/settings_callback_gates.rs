//! Settings callback-handler pure gates (slice 86, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/callbacks/settings-callback-handler.ts`
//!   (`SETTINGS_CALLBACK_PREFIX` dispatch guard; the ten exact-match
//!   toggle branches with the unknown-payload `callback.processing_error`
//!   fallback; `getTtsSavedMessageKey`: `all` -> `tts.all`,
//!   `auto` -> `tts.auto`, otherwise `tts.off`; `getNextTtsMode`:
//!   `off` -> `all`, `all` -> `auto`, otherwise `off`;
//!   `getNextPromptQueueMode`: `off` -> `queue`,
//!   `queue` + V2 -> `steer`, otherwise `off`;
//!   `getNextResponseStreamingMode`: `edit` -> `draft`, otherwise
//!   `edit`; the seven boolean negations behind the on/off toggles;
//!   the TTS-unconfigured block gate: `nextMode !== "off"` while TTS is
//!   not configured answers `tts.not_configured` and leaves the stored
//!   mode untouched).
//! - `src/bot/menus/settings-menu.ts` (the `SETTINGS_*` callback
//!   constants; already classified in `catalog_settings_menu_gates`,
//!   repeated here only as the dispatch table the handler matches).
//!
//! Already ported elsewhere and NOT repeated here:
//! - `catalog_settings_menu_gates` (the `SETTINGS_*` constants, the
//!   `SettingsToggle` classification, the on/off + TTS + prompt-queue +
//!   streaming value formatters, the settings row plan).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`answerCallbackQuery`, `editMessageText`,
//!   `deleteMessage`), `pinnedMessageManager` init/apply/refresh,
//!   `safeBackgroundTask` scheduling.
//! - settings-store reads/writes (`get*`/`set*`), `isTtsConfigured()`,
//!   `opencodeServerVersion`, `buildSettingsMenuView`,
//!   `appendInlineMenuCancelButton`, `ensureActiveInlineMenu`.
//! - `t(...)` lookups: reply/answer keys travel as literal key strings
//!   in the caller.
//!
//! Deviations: none intended; `TtsMode` / `PromptQueueMode` /
//! `ResponseStreamingMode` string unions are modelled as enums, and the
//! server version flag arrives as an enum instead of the imported
//! `opencodeServerVersion` global.
//!
//! Step 2 (GREEN): gates ported; parity tests pass.

/// `settings:` callback prefix (TS: `SETTINGS_CALLBACK_PREFIX`).
pub const SETTINGS_CALLBACK_PREFIX: &str = "settings:";
/// Compact-output toggle callback.
pub const SETTINGS_COMPACT_OUTPUT_CALLBACK: &str = "settings:compact_output";
/// Delete-compact-progress-on-finish toggle callback.
pub const SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK: &str = "settings:delete_progress_on_finish";
/// Thinking-content toggle callback.
pub const SETTINGS_THINKING_CONTENT_CALLBACK: &str = "settings:thinking_content";
/// Response-streaming toggle callback.
pub const SETTINGS_RESPONSE_STREAMING_CALLBACK: &str = "settings:response_streaming";
/// Diff-files toggle callback.
pub const SETTINGS_DIFF_FILES_CALLBACK: &str = "settings:diff_files";
/// Assistant-footer toggle callback.
pub const SETTINGS_ASSISTANT_FOOTER_CALLBACK: &str = "settings:assistant_footer";
/// Pin-session-dashboard toggle callback.
pub const SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK: &str = "settings:pin_session_dashboard";
/// Background-notifications toggle callback.
pub const SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK: &str = "settings:background_notifications";
/// TTS mode cycle callback.
pub const SETTINGS_TTS_CALLBACK: &str = "settings:tts";
/// Prompt-queue mode cycle callback.
pub const SETTINGS_PROMPT_QUEUE_CALLBACK: &str = "settings:prompt_queue";

/// TTS modes (TS: `TtsMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsMode {
    Off,
    All,
    Auto,
}

/// Prompt-queue modes (TS: `PromptQueueMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptQueueMode {
    Off,
    Queue,
    Steer,
}

/// Response-streaming modes (TS: `ResponseStreamingMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStreamingMode {
    Edit,
    Draft,
}

/// OpenCode server version behind the prompt-queue cycle (TS: the
/// `opencodeServerVersion === "v2"` check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerVersion {
    V1,
    V2,
}

/// Handler dispatch branch for a `settings:*` payload (TS: the ten
/// exact-match `if` branches plus the trailing `processing_error`
/// fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCallbackAction {
    CompactOutput,
    DeleteProgressOnFinish,
    ThinkingContent,
    ResponseStreaming,
    DiffFiles,
    AssistantFooter,
    PinSessionDashboard,
    BackgroundNotifications,
    Tts,
    PromptQueue,
    Unknown,
}

/// Settings-callback dispatch guard (TS: `if
/// (!callbackData?.startsWith(SETTINGS_CALLBACK_PREFIX)) return false`).
pub fn handles_settings_callback(data: Option<&str>) -> bool {
    data.is_some_and(|d| d.starts_with(SETTINGS_CALLBACK_PREFIX))
}

/// Handler dispatch classifier (TS: the ten exact-match branches;
/// anything else falls through to `callback.processing_error`).
pub fn classify_settings_callback_action(data: &str) -> SettingsCallbackAction {
    if data == SETTINGS_COMPACT_OUTPUT_CALLBACK {
        SettingsCallbackAction::CompactOutput
    } else if data == SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK {
        SettingsCallbackAction::DeleteProgressOnFinish
    } else if data == SETTINGS_THINKING_CONTENT_CALLBACK {
        SettingsCallbackAction::ThinkingContent
    } else if data == SETTINGS_RESPONSE_STREAMING_CALLBACK {
        SettingsCallbackAction::ResponseStreaming
    } else if data == SETTINGS_DIFF_FILES_CALLBACK {
        SettingsCallbackAction::DiffFiles
    } else if data == SETTINGS_ASSISTANT_FOOTER_CALLBACK {
        SettingsCallbackAction::AssistantFooter
    } else if data == SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK {
        SettingsCallbackAction::PinSessionDashboard
    } else if data == SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK {
        SettingsCallbackAction::BackgroundNotifications
    } else if data == SETTINGS_TTS_CALLBACK {
        SettingsCallbackAction::Tts
    } else if data == SETTINGS_PROMPT_QUEUE_CALLBACK {
        SettingsCallbackAction::PromptQueue
    } else {
        SettingsCallbackAction::Unknown
    }
}

/// TTS mode cycle (TS: `getNextTtsMode`: `off` -> `all`, `all` ->
/// `auto`, otherwise `off`).
pub fn next_tts_mode(mode: TtsMode) -> TtsMode {
    match mode {
        TtsMode::Off => TtsMode::All,
        TtsMode::All => TtsMode::Auto,
        TtsMode::Auto => TtsMode::Off,
    }
}

/// TTS saved-toast key (TS: `getTtsSavedMessageKey`: `all` ->
/// `tts.all`, `auto` -> `tts.auto`, otherwise `tts.off`).
pub fn tts_saved_message_key(mode: TtsMode) -> &'static str {
    match mode {
        TtsMode::All => "tts.all",
        TtsMode::Auto => "tts.auto",
        TtsMode::Off => "tts.off",
    }
}

/// Prompt-queue mode cycle (TS: `getNextPromptQueueMode`: `off` ->
/// `queue`, `queue` + V2 -> `steer`, otherwise `off`).
pub fn next_prompt_queue_mode(
    mode: PromptQueueMode,
    server_version: ServerVersion,
) -> PromptQueueMode {
    if mode == PromptQueueMode::Off {
        return PromptQueueMode::Queue;
    }
    if mode == PromptQueueMode::Queue && server_version == ServerVersion::V2 {
        return PromptQueueMode::Steer;
    }
    PromptQueueMode::Off
}

/// Response-streaming toggle (TS: `getNextResponseStreamingMode`:
/// `edit` -> `draft`, otherwise `edit`).
pub fn next_response_streaming_mode(mode: ResponseStreamingMode) -> ResponseStreamingMode {
    match mode {
        ResponseStreamingMode::Edit => ResponseStreamingMode::Draft,
        ResponseStreamingMode::Draft => ResponseStreamingMode::Edit,
    }
}

/// Boolean-toggle application (TS: the `setX(!getX())` negations behind
/// the compact-output, delete-progress, thinking-content, diff-files,
/// assistant-footer, pin-dashboard, and background-notifications
/// branches).
pub fn toggle_boolean_value(current: bool) -> bool {
    !current
}

/// TTS-unconfigured block gate (TS: `if (nextMode !== "off" &&
/// !isTtsConfigured())` answers `tts.not_configured` and returns early
/// without storing the mode).
pub fn should_block_tts_toggle(next_mode: TtsMode, tts_configured: bool) -> bool {
    next_mode != TtsMode::Off && !tts_configured
}

/// Success-toast key for a handled toggle (TS: the TTS branch answers
/// `t(getTtsSavedMessageKey(nextMode))`; every other handled branch
/// answers `t("settings.saved")`; the fallback answers
/// `t("callback.processing_error")`).
pub fn settings_answer_key(action: SettingsCallbackAction, tts_mode: TtsMode) -> &'static str {
    match action {
        SettingsCallbackAction::Tts => tts_saved_message_key(tts_mode),
        SettingsCallbackAction::Unknown => "callback.processing_error",
        _ => "settings.saved",
    }
}
