//! Slice 86 parity: settings callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/settings-callback-handler.ts` (the `settings:`
//! prefix dispatch guard; the ten exact-match toggle branches with the
//! unknown-payload `callback.processing_error` fallback; the TTS
//! `off` -> `all` -> `auto` -> `off` cycle; the TTS saved-toast key;
//! the version-dependent prompt-queue cycle; the streaming `edit` /
//! `draft` toggle; the boolean negations; the TTS-unconfigured block
//! gate; the success-toast key table).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `settings_callback_gates` is ported (only the constant-value tests
//! pass).

use otb_core::settings_callback_gates::*;

// ---- constants ----

#[test]
fn settings_prefix_value_parity() {
    assert_eq!(SETTINGS_CALLBACK_PREFIX, "settings:");
}

#[test]
fn settings_compact_output_callback_value_parity() {
    assert_eq!(SETTINGS_COMPACT_OUTPUT_CALLBACK, "settings:compact_output");
}

#[test]
fn settings_delete_progress_callback_value_parity() {
    assert_eq!(
        SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK,
        "settings:delete_progress_on_finish"
    );
}

#[test]
fn settings_thinking_content_callback_value_parity() {
    assert_eq!(SETTINGS_THINKING_CONTENT_CALLBACK, "settings:thinking_content");
}

#[test]
fn settings_response_streaming_callback_value_parity() {
    assert_eq!(
        SETTINGS_RESPONSE_STREAMING_CALLBACK,
        "settings:response_streaming"
    );
}

#[test]
fn settings_diff_files_callback_value_parity() {
    assert_eq!(SETTINGS_DIFF_FILES_CALLBACK, "settings:diff_files");
}

#[test]
fn settings_assistant_footer_callback_value_parity() {
    assert_eq!(SETTINGS_ASSISTANT_FOOTER_CALLBACK, "settings:assistant_footer");
}

#[test]
fn settings_pin_dashboard_callback_value_parity() {
    assert_eq!(
        SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK,
        "settings:pin_session_dashboard"
    );
}

#[test]
fn settings_background_notifications_callback_value_parity() {
    assert_eq!(
        SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK,
        "settings:background_notifications"
    );
}

#[test]
fn settings_tts_callback_value_parity() {
    assert_eq!(SETTINGS_TTS_CALLBACK, "settings:tts");
}

#[test]
fn settings_prompt_queue_callback_value_parity() {
    assert_eq!(SETTINGS_PROMPT_QUEUE_CALLBACK, "settings:prompt_queue");
}

// ---- prefix dispatch guard ----

#[test]
fn handles_settings_callback_none_parity() {
    assert!(!handles_settings_callback(None));
}

#[test]
fn handles_settings_callback_empty_parity() {
    assert!(!handles_settings_callback(Some("")));
}

#[test]
fn handles_settings_callback_exact_tts_parity() {
    assert!(handles_settings_callback(Some("settings:tts")));
}

#[test]
fn handles_settings_callback_rejects_other_prefix_parity() {
    assert!(!handles_settings_callback(Some("model:tts")));
    assert!(!handles_settings_callback(Some("settings")));
}

#[test]
fn handles_settings_callback_rejects_case_variant_parity() {
    assert!(!handles_settings_callback(Some("Settings:tts")));
}

// ---- dispatch classifier ----

#[test]
fn classify_compact_output_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:compact_output"),
        SettingsCallbackAction::CompactOutput
    );
}

#[test]
fn classify_delete_progress_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:delete_progress_on_finish"),
        SettingsCallbackAction::DeleteProgressOnFinish
    );
}

#[test]
fn classify_thinking_content_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:thinking_content"),
        SettingsCallbackAction::ThinkingContent
    );
}

#[test]
fn classify_response_streaming_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:response_streaming"),
        SettingsCallbackAction::ResponseStreaming
    );
}

#[test]
fn classify_diff_files_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:diff_files"),
        SettingsCallbackAction::DiffFiles
    );
}

#[test]
fn classify_assistant_footer_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:assistant_footer"),
        SettingsCallbackAction::AssistantFooter
    );
}

#[test]
fn classify_pin_dashboard_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:pin_session_dashboard"),
        SettingsCallbackAction::PinSessionDashboard
    );
}

#[test]
fn classify_background_notifications_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:background_notifications"),
        SettingsCallbackAction::BackgroundNotifications
    );
}

#[test]
fn classify_tts_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:tts"),
        SettingsCallbackAction::Tts
    );
}

#[test]
fn classify_prompt_queue_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:prompt_queue"),
        SettingsCallbackAction::PromptQueue
    );
}

#[test]
fn classify_unknown_settings_suffix_parity() {
    assert_eq!(
        classify_settings_callback_action("settings:nope"),
        SettingsCallbackAction::Unknown
    );
}

// ---- TTS cycle ----

#[test]
fn next_tts_off_goes_all_parity() {
    assert_eq!(next_tts_mode(TtsMode::Off), TtsMode::All);
}

#[test]
fn next_tts_all_goes_auto_parity() {
    assert_eq!(next_tts_mode(TtsMode::All), TtsMode::Auto);
}

#[test]
fn next_tts_auto_goes_off_parity() {
    assert_eq!(next_tts_mode(TtsMode::Auto), TtsMode::Off);
}

// ---- TTS saved key ----

#[test]
fn tts_saved_key_off_parity() {
    assert_eq!(tts_saved_message_key(TtsMode::Off), "tts.off");
}

#[test]
fn tts_saved_key_all_parity() {
    assert_eq!(tts_saved_message_key(TtsMode::All), "tts.all");
}

#[test]
fn tts_saved_key_auto_parity() {
    assert_eq!(tts_saved_message_key(TtsMode::Auto), "tts.auto");
}

// ---- prompt-queue cycle ----

#[test]
fn next_queue_off_goes_queue_v1_parity() {
    assert_eq!(
        next_prompt_queue_mode(PromptQueueMode::Off, ServerVersion::V1),
        PromptQueueMode::Queue
    );
}

#[test]
fn next_queue_off_goes_queue_v2_parity() {
    assert_eq!(
        next_prompt_queue_mode(PromptQueueMode::Off, ServerVersion::V2),
        PromptQueueMode::Queue
    );
}

#[test]
fn next_queue_queue_goes_steer_on_v2_parity() {
    assert_eq!(
        next_prompt_queue_mode(PromptQueueMode::Queue, ServerVersion::V2),
        PromptQueueMode::Steer
    );
}

#[test]
fn next_queue_queue_goes_off_on_v1_parity() {
    assert_eq!(
        next_prompt_queue_mode(PromptQueueMode::Queue, ServerVersion::V1),
        PromptQueueMode::Off
    );
}

#[test]
fn next_queue_steer_goes_off_v2_parity() {
    assert_eq!(
        next_prompt_queue_mode(PromptQueueMode::Steer, ServerVersion::V2),
        PromptQueueMode::Off
    );
}

// ---- streaming toggle ----

#[test]
fn next_streaming_edit_goes_draft_parity() {
    assert_eq!(
        next_response_streaming_mode(ResponseStreamingMode::Edit),
        ResponseStreamingMode::Draft
    );
}

#[test]
fn next_streaming_draft_goes_edit_parity() {
    assert_eq!(
        next_response_streaming_mode(ResponseStreamingMode::Draft),
        ResponseStreamingMode::Edit
    );
}

// ---- boolean toggles ----

#[test]
fn toggle_boolean_true_goes_false_parity() {
    assert!(!toggle_boolean_value(true));
}

#[test]
fn toggle_boolean_false_goes_true_parity() {
    assert!(toggle_boolean_value(false));
}

// ---- TTS block gate ----

#[test]
fn block_tts_unconfigured_next_all_parity() {
    assert!(should_block_tts_toggle(TtsMode::All, false));
}

#[test]
fn block_tts_configured_next_all_parity() {
    assert!(!should_block_tts_toggle(TtsMode::All, true));
}

#[test]
fn block_tts_next_off_never_blocks_parity() {
    assert!(!should_block_tts_toggle(TtsMode::Off, false));
    assert!(!should_block_tts_toggle(TtsMode::Off, true));
}

// ---- answer keys ----

#[test]
fn answer_key_tts_all_parity() {
    assert_eq!(
        settings_answer_key(SettingsCallbackAction::Tts, TtsMode::All),
        "tts.all"
    );
}

#[test]
fn answer_key_tts_auto_parity() {
    assert_eq!(
        settings_answer_key(SettingsCallbackAction::Tts, TtsMode::Auto),
        "tts.auto"
    );
}

#[test]
fn answer_key_tts_off_parity() {
    assert_eq!(
        settings_answer_key(SettingsCallbackAction::Tts, TtsMode::Off),
        "tts.off"
    );
}

#[test]
fn answer_key_compact_output_saved_parity() {
    assert_eq!(
        settings_answer_key(SettingsCallbackAction::CompactOutput, TtsMode::Off),
        "settings.saved"
    );
}

#[test]
fn answer_key_unknown_processing_error_parity() {
    assert_eq!(
        settings_answer_key(SettingsCallbackAction::Unknown, TtsMode::Off),
        "callback.processing_error"
    );
}
