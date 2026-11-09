//! Parity port of pure type/config logic (slice 21b, RED).
//!
//! Mirrors `tests/app/types/agent.test.ts`,
//! `tests/app/types/model.test.ts`, `tests/config.test.ts` (61 its),
//! `tests/app/stores/settings-store.test.ts` (pure subset), and
//! `tests/bot/commands/definitions.test.ts`.
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::types_config::*;

// --- agent.ts (4 its) ---

#[test]
fn slice21b_01_agent_emoji_known() {
    assert_eq!(get_agent_emoji("build"), "🛠️");
    assert_eq!(get_agent_emoji("plan"), "📋");
}

#[test]
fn slice21b_02_agent_emoji_fallback() {
    assert_eq!(get_agent_emoji("custom-agent"), "🤖");
}

#[test]
fn slice21b_03_agent_display_name() {
    assert_eq!(get_agent_display_name("build"), "🛠️ Build");
    assert_eq!(get_agent_display_name("customAgent"), "🤖 CustomAgent");
}

#[test]
fn slice21b_04_agent_button_label() {
    assert_eq!(get_agent_button_label("build"), "🛠️ Build Agent");
    assert_eq!(
        get_agent_button_label("customAgent"),
        "🤖 CustomAgent Agent"
    );
}

// --- model.ts (4 its) ---

#[test]
fn slice21b_05_model_button_no_truncation() {
    assert_eq!(
        format_model_for_button("openai", "gpt-4o"),
        "🧠 openai\ngpt-4o"
    );
}

#[test]
fn slice21b_06_model_button_truncation() {
    assert_eq!(
        format_model_for_button("very-long-provider-name", "very-long-model-name-v2-preview"),
        "🧠 very-long-pr...\nvery-long-model-n..."
    );
}

#[test]
fn slice21b_07_model_display() {
    assert_eq!(
        format_model_for_display("anthropic", "claude-sonnet"),
        "anthropic / claude-sonnet"
    );
}

#[test]
fn slice21b_08_is_same_model() {
    let a = ModelInfo {
        provider_id: "p".into(),
        model_id: "m".into(),
        variant: None,
    };
    let b = ModelInfo {
        provider_id: "p".into(),
        model_id: "m".into(),
        variant: None,
    };
    let c = ModelInfo {
        provider_id: "p".into(),
        model_id: "m".into(),
        variant: Some("v".into()),
    };
    assert!(is_same_model(&a, &b));
    assert!(!is_same_model(&a, &c));
}

// --- server mode/version (8 its) ---

#[test]
fn slice21b_09_server_version_default_v2() {
    assert_eq!(resolve_server_version(None), "v2");
    assert_eq!(resolve_server_version(Some("")), "v2");
    assert_eq!(resolve_server_version(Some("   ")), "v2");
}

#[test]
fn slice21b_10_server_version_v2_case_insensitive() {
    assert_eq!(resolve_server_version(Some(" V2 ")), "v2");
    assert_eq!(resolve_server_version(Some("2")), "v2");
    assert_eq!(resolve_server_version(Some("banana")), "v2");
}

#[test]
fn slice21b_11_server_version_explicit_v1_pin() {
    assert_eq!(resolve_server_version(Some("v1")), "v1");
    assert_eq!(resolve_server_version(Some(" V1 ")), "v1");
    assert_eq!(resolve_server_version(Some("1")), "v1");
}

#[test]
fn slice21b_12_mode_empty_v2_is_service() {
    assert_eq!(resolve_server_mode("v2", None), Ok("service"));
    assert_eq!(resolve_server_mode("v2", Some("   ")), Ok("service"));
}

#[test]
fn slice21b_13_mode_standalone_accepted() {
    assert_eq!(
        resolve_server_mode("v2", Some("Standalone")),
        Ok("standalone")
    );
}

#[test]
fn slice21b_14_mode_unknown_v2_errors() {
    assert!(resolve_server_mode("v2", Some("banana")).is_err());
}

#[test]
fn slice21b_15_mode_unknown_v1_ignored() {
    assert_eq!(resolve_server_mode("v1", Some("banana")), Ok("service"));
}

#[test]
fn slice21b_16_api_url_defaults() {
    assert_eq!(
        resolve_api_url("v1", "service", None).as_deref(),
        Ok("http://localhost:4096")
    );
    assert_eq!(
        resolve_api_url("v2", "service", None).as_deref(),
        Ok("http://127.0.0.1:49374")
    );
}

#[test]
fn slice21b_17_api_url_standalone_requires_explicit() {
    assert!(resolve_api_url("v2", "standalone", None).is_err());
    assert_eq!(
        resolve_api_url("v2", "standalone", Some("http://127.0.0.1:4097")).as_deref(),
        Ok("http://127.0.0.1:4097")
    );
}

#[test]
fn slice21b_18_api_url_explicit_kept() {
    assert_eq!(
        resolve_api_url("v2", "service", Some("http://127.0.0.1:4096")).as_deref(),
        Ok("http://127.0.0.1:4096")
    );
}

// --- boolean/int/locale/mode parsing (12 its) ---

#[test]
fn slice21b_19_bool_truthy_falsy() {
    assert!(parse_bool_env(Some("true"), false));
    assert!(parse_bool_env(Some("off"), true) == false);
}

#[test]
fn slice21b_20_bool_invalid_falls_back() {
    assert!(parse_bool_env(Some("banana"), true));
    assert!(!parse_bool_env(None, false));
}

#[test]
fn slice21b_21_positive_int_parses() {
    assert_eq!(parse_positive_int_env(Some("25"), 10), 25);
}

#[test]
fn slice21b_22_positive_int_fallback() {
    assert_eq!(parse_positive_int_env(Some("zero"), 10), 10);
    assert_eq!(parse_positive_int_env(None, 10), 10);
    assert_eq!(parse_positive_int_env(Some("0"), 10), 10);
}

#[test]
fn slice21b_23_non_negative_int_accepts_zero() {
    assert_eq!(parse_non_negative_int_env(Some("0"), 1500), 0);
    assert_eq!(parse_non_negative_int_env(Some("-1"), 1500), 1500);
}

#[test]
fn slice21b_24_message_format_mode() {
    assert_eq!(parse_message_format_mode(None), "markdown");
    assert_eq!(parse_message_format_mode(Some("raw")), "raw");
    assert_eq!(parse_message_format_mode(Some("MARKDOWN")), "markdown");
    assert_eq!(parse_message_format_mode(Some("html")), "markdown");
}

#[test]
fn slice21b_25_path_list_empty() {
    assert!(parse_path_list(None).is_empty());
    assert!(parse_path_list(Some("")).is_empty());
}

#[test]
fn slice21b_26_path_list_parses_and_trims() {
    assert_eq!(
        parse_path_list(Some("/a,/b,/c")),
        vec!["/a".to_string(), "/b".to_string(), "/c".to_string()]
    );
    assert_eq!(
        parse_path_list(Some("  /a , ,/b  ")),
        vec!["/a".to_string(), "/b".to_string()]
    );
}

#[test]
fn slice21b_27_locale_supported() {
    assert_eq!(normalize_locale_tag(Some("fr"), "en"), "fr");
}

#[test]
fn slice21b_28_locale_regional_normalized() {
    assert_eq!(normalize_locale_tag(Some("ru-RU"), "en"), "ru");
}

#[test]
fn slice21b_29_locale_fallback() {
    assert_eq!(normalize_locale_tag(Some("xx"), "en"), "en");
}

#[test]
fn slice21b_30_initial_preset_empty() {
    assert_eq!(parse_initial_settings_preset(None).as_deref(), Ok("{}"));
}

// --- initial preset / telegram proxy (8 its) ---

#[test]
fn slice21b_31_initial_preset_valid_object() {
    let out = parse_initial_settings_preset(Some(r#"{"a":false}"#)).unwrap();
    assert!(out.contains("\"a\""));
}

#[test]
fn slice21b_32_initial_preset_invalid_json() {
    assert!(parse_initial_settings_preset(Some("{not valid}")).is_err());
}

#[test]
fn slice21b_33_initial_preset_array_rejected() {
    assert!(parse_initial_settings_preset(Some(r#"["a"]"#)).is_err());
}

#[test]
fn slice21b_34_api_root_trailing_slash() {
    assert_eq!(
        normalize_api_root("https://tg-proxy.example.com/"),
        "https://tg-proxy.example.com"
    );
    assert_eq!(
        normalize_api_root("https://tg-proxy.example.com///"),
        "https://tg-proxy.example.com"
    );
    assert_eq!(
        normalize_api_root("https://tg-proxy.example.com"),
        "https://tg-proxy.example.com"
    );
}

#[test]
fn slice21b_35_proxy_url_alone_ok() {
    assert!(validate_telegram_proxy("socks5://forward.example.com:1080", "", "").is_ok());
    assert!(validate_telegram_proxy("", "", "").is_ok());
}

#[test]
fn slice21b_36_proxy_url_plus_api_root_rejected() {
    assert!(validate_telegram_proxy(
        "socks5://forward.example.com:1080",
        "https://tg-proxy.example.com",
        ""
    )
    .is_err());
}

#[test]
fn slice21b_37_proxy_secret_requires_api_root() {
    assert!(validate_telegram_proxy("", "", "shared-secret").is_err());
    assert!(validate_telegram_proxy("", "https://tg-proxy.example.com", "shared-secret").is_ok());
}

// --- commands (3 its) ---

#[test]
fn slice21b_38_v2_has_reload_and_goal() {
    let cmds = active_commands("v2");
    assert!(cmds.contains(&"reload".to_string()));
    assert!(cmds.contains(&"goal".to_string()));
    let reload_pos = cmds.iter().position(|c| c == "reload").unwrap();
    let stop_pos = cmds.iter().position(|c| c == "opencode_stop").unwrap();
    assert_eq!(reload_pos, stop_pos + 1);
}

#[test]
fn slice21b_39_v1_hides_reload_and_goal() {
    let cmds = active_commands("v1");
    assert!(!cmds.contains(&"reload".to_string()));
    assert!(!cmds.contains(&"goal".to_string()));
}

#[test]
fn slice21b_40_builtin_names_include_start() {
    let names = builtin_command_names("v2");
    assert!(names.contains(&"start".to_string()));
    assert!(names.contains(&"reload".to_string()));
    assert!(!builtin_command_names("v1").contains(&"reload".to_string()));
}

// --- health port (3 its) ---

#[test]
fn slice21b_41_health_off_outside_container() {
    assert_eq!(resolve_health_port(None, false), 0);
}

#[test]
fn slice21b_42_health_default_in_container() {
    assert_eq!(resolve_health_port(None, true), 3100);
    assert_eq!(resolve_health_port(Some("0"), true), 0);
}

#[test]
fn slice21b_43_health_valid_and_overflow() {
    assert_eq!(resolve_health_port(Some("65535"), false), 65535);
    assert_eq!(resolve_health_port(Some("65536"), false), 0);
    assert_eq!(resolve_health_port(Some("65536"), true), 3100);
}

// --- TTS + settings defaults (5 its) ---

#[test]
fn slice21b_44_tts_defaults_per_provider() {
    assert_eq!(tts_default_model("elevenlabs"), "eleven_flash_v2_5");
    assert_eq!(tts_default_voice("elevenlabs"), "21m00Tcm4TlvDq8ikWAM");
    assert_eq!(
        parse_tts_provider(Some("elevenlabs"), "openai"),
        "elevenlabs"
    );
}

#[test]
fn slice21b_45_settings_display_defaults() {
    assert!(!default_compact_output_mode());
    assert!(default_show_thinking());
    assert!(default_show_assistant_run_footer());
    assert!(default_pinned_dashboard_enabled());
    assert!(default_show_background_session_notifications());
    assert!(default_track_background_sessions());
}

#[test]
fn slice21b_46_prompt_queue_defaults_per_version() {
    assert_eq!(default_prompt_queue_mode("v1"), "off");
    assert_eq!(default_prompt_queue_mode("v2"), "steer");
}

#[test]
fn slice21b_47_tts_enabled_migration() {
    assert_eq!(migrate_tts_enabled(true), "all");
    assert_eq!(migrate_tts_enabled(false), "off");
}

#[test]
fn slice21b_48_agent_emoji_table_covers_known() {
    let table = agent_emoji_table();
    let get = |k: &str| table.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
    assert_eq!(get("build"), Some("🛠️"));
    assert_eq!(get("general"), Some("💬"));
}
