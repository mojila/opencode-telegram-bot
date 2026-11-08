//! Parity port of the catalog + settings menu pure gates
//! (slice 57, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/bot/menus/command-catalog-menu.ts` (`COMMANDS_*` prefixes,
//!   `formatExecutingCommandMessage`, page/select `Number()` parses,
//!   select-text gate, `"/name - desc"` label cut, pagination range)
//! - `src/bot/menus/skills-catalog-menu.ts` (same shape over `skills:`)
//! - `src/bot/menus/mcp-catalog-menu.ts` (`MCPS_*` prefixes, status emoji,
//!   label cut, select parse, detail-keyboard toggle gate, error/auth gates,
//!   detail text join)
//! - `src/bot/menus/settings-menu.ts` (`SETTINGS_*` callbacks, boolean/TTS/
//!   queue/streaming value gates, row plan)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::catalog_settings_menu_gates::*;

// --- shared ---

#[test]
fn max_catalog_button_label_length_is_64() {
    assert_eq!(MAX_CATALOG_BUTTON_LABEL_LENGTH, 64);
}

#[test]
fn catalog_select_text_kind_branches() {
    assert_eq!(catalog_select_text_kind(0), CatalogSelectTextKind::FirstPage);
    assert_eq!(catalog_select_text_kind(2), CatalogSelectTextKind::Page(2));
}

// --- command prefixes ---

#[test]
fn commands_prefix_values() {
    assert_eq!(COMMANDS_CALLBACK_PREFIX, "commands:");
    assert_eq!(COMMANDS_CALLBACK_SELECT_PREFIX, "commands:select:");
    assert_eq!(COMMANDS_CALLBACK_PAGE_PREFIX, "commands:page:");
    assert_eq!(COMMANDS_CALLBACK_CANCEL, "commands:cancel");
    assert_eq!(COMMANDS_CALLBACK_EXECUTE, "commands:execute");
}

// --- command executing message ---

#[test]
fn executing_command_message_without_args() {
    let message = format_executing_command_message("Running", "build", "");
    assert_eq!(message.text, "Running\n/build");
    assert_eq!(message.code_offset, "Running".len() + 1);
    assert_eq!(message.code_length, "/build".len());
}

#[test]
fn executing_command_message_with_args() {
    let message = format_executing_command_message("Running", "build", "--force");
    assert_eq!(message.text, "Running\n/build --force");
    assert_eq!(message.code_offset, "Running".len() + 1);
    assert_eq!(message.code_length, "/build".len());
}

// --- command callbacks ---

#[test]
fn build_command_page_callback_shape() {
    assert_eq!(build_command_page_callback(3), "commands:page:3");
}

#[test]
fn parse_command_page_callback_valid() {
    assert_eq!(parse_command_page_callback("commands:page:2"), Some(2));
}

#[test]
fn parse_command_page_callback_empty_reads_as_zero() {
    assert_eq!(parse_command_page_callback("commands:page:"), Some(0));
}

#[test]
fn parse_command_page_callback_rejects_negative() {
    assert_eq!(parse_command_page_callback("commands:page:-1"), None);
}

#[test]
fn parse_command_page_callback_rejects_fraction() {
    assert_eq!(parse_command_page_callback("commands:page:1.5"), None);
}

#[test]
fn parse_command_select_callback_valid() {
    assert_eq!(parse_command_select_callback("commands:select:7"), Some(7));
}

#[test]
fn command_select_callback_for_index_shape() {
    assert_eq!(command_select_callback_for_index(7), "commands:select:7");
}

// --- command label + pagination ---

#[test]
fn format_command_button_label_short() {
    assert_eq!(
        format_command_button_label("build", Some("Build it"), "No description"),
        "/build - Build it"
    );
}

#[test]
fn format_command_button_label_cuts_at_64() {
    let label = format_command_button_label("build", Some(&"d".repeat(100)), "No description");
    assert_eq!(label.len(), 64);
    assert!(label.ends_with("..."));
}

#[test]
fn commands_pagination_empty_list() {
    assert_eq!(
        calculate_commands_pagination_range(0, 0, 10),
        CatalogPaginationRange { page: 0, total_pages: 1, start_index: 0, end_index: 0 }
    );
}

#[test]
fn commands_pagination_clamps_high_page() {
    let range = calculate_commands_pagination_range(25, 99, 10);
    assert_eq!((range.page, range.start_index, range.end_index), (2, 20, 25));
}

// --- skills prefixes ---

#[test]
fn skills_prefix_values() {
    assert_eq!(SKILLS_CALLBACK_PREFIX, "skills:");
    assert_eq!(SKILLS_CALLBACK_SELECT_PREFIX, "skills:select:");
    assert_eq!(SKILLS_CALLBACK_PAGE_PREFIX, "skills:page:");
    assert_eq!(SKILLS_CALLBACK_CANCEL, "skills:cancel");
    assert_eq!(SKILLS_CALLBACK_EXECUTE, "skills:execute");
}

// --- skills executing message ---

#[test]
fn executing_skill_message_without_args() {
    let message = format_executing_skill_message("Running", "review", "");
    assert_eq!(message.text, "Running\n/review");
    assert_eq!(message.code_offset, "Running".len() + 1);
    assert_eq!(message.code_length, "/review".len());
}

// --- skills callbacks ---

#[test]
fn build_skill_page_callback_shape() {
    assert_eq!(build_skill_page_callback(1), "skills:page:1");
}

#[test]
fn parse_skill_page_callback_valid() {
    assert_eq!(parse_skill_page_callback("skills:page:2"), Some(2));
}

#[test]
fn parse_skill_select_callback_valid() {
    assert_eq!(parse_skill_select_callback("skills:select:4"), Some(4));
}

#[test]
fn skill_select_callback_for_index_shape() {
    assert_eq!(skill_select_callback_for_index(4), "skills:select:4");
}

// --- skills label + pagination ---

#[test]
fn format_skill_button_label_missing_description_falls_back() {
    assert_eq!(
        format_skill_button_label("review", None, "No description"),
        "/review - No description"
    );
}

#[test]
fn format_skill_button_label_cuts_at_64() {
    let label = format_skill_button_label("review", Some(&"d".repeat(100)), "No description");
    assert_eq!(label.len(), 64);
    assert!(label.ends_with("..."));
}

#[test]
fn skills_pagination_clamps_negative_page() {
    let range = calculate_skills_pagination_range(25, -3, 10);
    assert_eq!((range.page, range.start_index), (0, 0));
}

// --- mcp prefixes ---

#[test]
fn mcps_prefix_values() {
    assert_eq!(MCPS_CALLBACK_PREFIX, "mcps:");
    assert_eq!(MCPS_CALLBACK_SELECT_PREFIX, "mcps:select:");
    assert_eq!(MCPS_CALLBACK_TOGGLE, "mcps:toggle");
    assert_eq!(MCPS_CALLBACK_BACK, "mcps:back");
    assert_eq!(MCPS_CALLBACK_CANCEL, "mcps:cancel");
}

// --- mcp status + labels ---

#[test]
fn mcp_status_emoji_map() {
    assert_eq!(mcp_status_emoji(McpStatusKind::Connected), "🟢");
    assert_eq!(mcp_status_emoji(McpStatusKind::Disabled), "🔴");
    assert_eq!(mcp_status_emoji(McpStatusKind::Failed), "⚠️");
    assert_eq!(mcp_status_emoji(McpStatusKind::NeedsAuth), "🔒");
    assert_eq!(mcp_status_emoji(McpStatusKind::NeedsClientRegistration), "🔒");
    assert_eq!(mcp_status_emoji(McpStatusKind::Unknown), "❓");
}

#[test]
fn format_mcp_button_label_short() {
    assert_eq!(
        format_mcp_button_label(McpStatusKind::Connected, "github"),
        "🟢 github"
    );
}

#[test]
fn format_mcp_button_label_cuts_at_64() {
    let label = format_mcp_button_label(McpStatusKind::Failed, &"n".repeat(100));
    assert_eq!(label.chars().count(), 64);
    assert!(label.ends_with("..."));
}

#[test]
fn parse_mcp_select_callback_valid() {
    assert_eq!(parse_mcp_select_callback("mcps:select:2"), Some(2));
}

// --- mcp detail keyboard + text ---

#[test]
fn mcp_detail_toggle_connected_offers_disable() {
    assert_eq!(
        mcp_detail_toggle_label(McpStatusKind::Connected),
        Some(McpToggleLabel::Disable)
    );
}

#[test]
fn mcp_detail_toggle_disabled_and_failed_offer_enable() {
    assert_eq!(
        mcp_detail_toggle_label(McpStatusKind::Disabled),
        Some(McpToggleLabel::Enable)
    );
    assert_eq!(
        mcp_detail_toggle_label(McpStatusKind::Failed),
        Some(McpToggleLabel::Enable)
    );
}

#[test]
fn mcp_detail_toggle_absent_without_switchable_status() {
    assert_eq!(mcp_detail_toggle_label(McpStatusKind::NeedsAuth), None);
    assert_eq!(mcp_detail_toggle_label(McpStatusKind::Unknown), None);
}

#[test]
fn mcp_detail_rows_with_toggle_have_two_rows() {
    assert_eq!(
        mcp_detail_callback_rows(McpStatusKind::Connected),
        vec![vec![MCPS_CALLBACK_TOGGLE], vec![MCPS_CALLBACK_BACK, MCPS_CALLBACK_CANCEL]]
    );
}

#[test]
fn mcp_detail_error_gate() {
    assert!(mcp_detail_shows_error(McpStatusKind::Failed));
    assert!(mcp_detail_shows_error(McpStatusKind::NeedsClientRegistration));
    assert!(!mcp_detail_shows_error(McpStatusKind::Connected));
}

#[test]
fn mcp_detail_auth_gate() {
    assert!(mcp_detail_shows_auth_notice(McpStatusKind::NeedsAuth));
    assert!(mcp_detail_shows_auth_notice(McpStatusKind::NeedsClientRegistration));
    assert!(!mcp_detail_shows_auth_notice(McpStatusKind::Disabled));
}

#[test]
fn build_mcp_detail_text_minimal() {
    assert_eq!(
        build_mcp_detail_text("Title", "Status: ok", None, None),
        "Title\n\nStatus: ok"
    );
}

#[test]
fn build_mcp_detail_text_with_error_and_auth() {
    assert_eq!(
        build_mcp_detail_text("Title", "Status: bad", Some("Error: boom"), Some("Auth needed")),
        "Title\n\nStatus: bad\nError: boom\n\nAuth needed"
    );
}

// --- settings callbacks ---

#[test]
fn settings_prefix_and_callback_values() {
    assert_eq!(SETTINGS_CALLBACK_PREFIX, "settings:");
    assert_eq!(SETTINGS_COMPACT_OUTPUT_CALLBACK, "settings:compact_output");
    assert_eq!(SETTINGS_TTS_CALLBACK, "settings:tts");
    assert_eq!(SETTINGS_PROMPT_QUEUE_CALLBACK, "settings:prompt_queue");
}

#[test]
fn classify_settings_callback_known() {
    assert_eq!(
        classify_settings_callback("settings:tts"),
        Some(SettingsToggle::Tts)
    );
    assert_eq!(
        classify_settings_callback("settings:compact_output"),
        Some(SettingsToggle::CompactOutput)
    );
}

#[test]
fn classify_settings_callback_unknown() {
    assert_eq!(classify_settings_callback("settings:nope"), None);
    assert_eq!(classify_settings_callback("commands:cancel"), None);
}

#[test]
fn is_settings_callback_guard() {
    assert!(is_settings_callback("settings:tts"));
    assert!(!is_settings_callback("mcps:toggle"));
}

// --- settings values ---

#[test]
fn format_boolean_setting_value_branches() {
    assert_eq!(format_boolean_setting_value(true, "On", "Off"), "On");
    assert_eq!(format_boolean_setting_value(false, "On", "Off"), "Off");
}

#[test]
fn format_tts_mode_value_three_way() {
    assert_eq!(format_tts_mode_value(TtsMode::All, "All", "Auto", "Off"), "All");
    assert_eq!(format_tts_mode_value(TtsMode::Auto, "All", "Auto", "Off"), "Auto");
    assert_eq!(format_tts_mode_value(TtsMode::Off, "All", "Auto", "Off"), "Off");
}

#[test]
fn format_prompt_queue_mode_value_v1_is_on_off() {
    assert_eq!(
        format_prompt_queue_mode_value(false, PromptQueueMode::Queue, "Queue", "Steer", "On", "Off"),
        "On"
    );
    assert_eq!(
        format_prompt_queue_mode_value(false, PromptQueueMode::Off, "Queue", "Steer", "On", "Off"),
        "Off"
    );
}

#[test]
fn format_prompt_queue_mode_value_v2_names_modes() {
    assert_eq!(
        format_prompt_queue_mode_value(true, PromptQueueMode::Queue, "Queue", "Steer", "On", "Off"),
        "Queue"
    );
    assert_eq!(
        format_prompt_queue_mode_value(true, PromptQueueMode::Steer, "Queue", "Steer", "On", "Off"),
        "Steer"
    );
    assert_eq!(
        format_prompt_queue_mode_value(true, PromptQueueMode::Off, "Queue", "Steer", "On", "Off"),
        "Off"
    );
}

#[test]
fn format_response_streaming_mode_value_branches() {
    assert_eq!(
        format_response_streaming_mode_value(ResponseStreamingMode::Draft, "Draft", "Edit"),
        "Draft"
    );
    assert_eq!(
        format_response_streaming_mode_value(ResponseStreamingMode::Edit, "Draft", "Edit"),
        "Edit"
    );
}

// --- settings rows ---

#[test]
fn settings_menu_rows_compact_mode() {
    assert_eq!(
        settings_menu_row_callbacks(true),
        vec![
            SETTINGS_COMPACT_OUTPUT_CALLBACK,
            SETTINGS_DELETE_PROGRESS_ON_FINISH_CALLBACK,
            SETTINGS_RESPONSE_STREAMING_CALLBACK,
            SETTINGS_ASSISTANT_FOOTER_CALLBACK,
            SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK,
            SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK,
            SETTINGS_TTS_CALLBACK,
            SETTINGS_PROMPT_QUEUE_CALLBACK,
        ]
    );
}

#[test]
fn settings_menu_rows_full_mode() {
    assert_eq!(
        settings_menu_row_callbacks(false),
        vec![
            SETTINGS_COMPACT_OUTPUT_CALLBACK,
            SETTINGS_THINKING_CONTENT_CALLBACK,
            SETTINGS_DIFF_FILES_CALLBACK,
            SETTINGS_RESPONSE_STREAMING_CALLBACK,
            SETTINGS_ASSISTANT_FOOTER_CALLBACK,
            SETTINGS_PIN_SESSION_DASHBOARD_CALLBACK,
            SETTINGS_BACKGROUND_NOTIFICATIONS_CALLBACK,
            SETTINGS_TTS_CALLBACK,
            SETTINGS_PROMPT_QUEUE_CALLBACK,
        ]
    );
}

#[test]
fn format_settings_row_shape() {
    assert_eq!(format_settings_row("Compact", "On"), "Compact: On");
}
