//! Parity port of bot-bootstrap / CLI / runtime-mode gates (slice 38, RED).
//!
//! Mirrors representative `it`s from:
//! - `tests/cli/args.test.ts` (default start, explicit start + modes,
//!   daemon flag, unknown command/mode/option, start-only guards,
//!   missing `--mode` value, help flags)
//! - `tests/runtime/mode.test.ts` (default, argv resolution, invalid
//!   syntax error, explicit-wins, env store normalization)
//! - `tests/bot/telegram-server-error-retry-policy.test.ts` (safe retry
//!   methods, creation methods not retried)
//! - `tests/bot/telegram-client-options.test.ts` (direct default, IPv4
//!   agent, proxy + IPv4 precedence, custom root wiring)
//! - `src/bot/index.ts` (`withBotLinkPreviewDefault` text/default gates,
//!   startup-managed + chat-delivery method sets)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::bot_bootstrap_gates::*;

// --- is_cli_command (3) ---

#[test]
fn slice38_01_start_status_stop_config_are_commands() {
    assert!(is_cli_command("start"));
    assert!(is_cli_command("status"));
    assert!(is_cli_command("stop"));
    assert!(is_cli_command("config"));
}

#[test]
fn slice38_02_unknown_words_are_not_commands() {
    assert!(!is_cli_command("deploy"));
    assert!(!is_cli_command(""));
    assert!(!is_cli_command("Start"));
}

#[test]
fn slice38_03_normalize_cli_mode_accepts_only_two_modes() {
    assert_eq!(normalize_cli_mode("installed"), Some(RuntimeMode::Installed));
    assert_eq!(normalize_cli_mode("sources"), Some(RuntimeMode::Sources));
    assert_eq!(normalize_cli_mode("qa"), None);
    assert_eq!(normalize_cli_mode(""), None);
    assert_eq!(normalize_cli_mode("Installed"), None);
}

// --- parse_cli_args defaults + start (5) ---

#[test]
fn slice38_04_empty_argv_defaults_to_start() {
    assert_eq!(
        parse_cli_args(&[]),
        ParsedCliArgs {
            command: CliCommand::Start,
            mode: None,
            daemon: false,
            show_help: false,
            error: None,
        }
    );
}

#[test]
fn slice38_05_explicit_start_with_installed_mode() {
    let parsed = parse_cli_args(&["start", "--mode", "installed"]);
    assert_eq!(parsed.command, CliCommand::Start);
    assert_eq!(parsed.mode, Some(RuntimeMode::Installed));
    assert!(!parsed.daemon);
    assert!(!parsed.show_help);
    assert_eq!(parsed.error, None);
}

#[test]
fn slice38_06_start_with_sources_mode() {
    let parsed = parse_cli_args(&["start", "--mode", "sources"]);
    assert_eq!(parsed.mode, Some(RuntimeMode::Sources));
    assert!(!parsed.show_help);
}

#[test]
fn slice38_07_start_with_mode_equals_syntax() {
    let parsed = parse_cli_args(&["start", "--mode=installed"]);
    assert_eq!(parsed.mode, Some(RuntimeMode::Installed));
    assert!(!parsed.show_help);
}

#[test]
fn slice38_08_daemon_flag_for_start() {
    let parsed = parse_cli_args(&["start", "--daemon"]);
    assert!(parsed.daemon);
    assert!(!parsed.show_help);
    assert_eq!(parsed.error, None);
}

// --- parse_cli_args errors (7) ---

#[test]
fn slice38_09_unknown_command_shows_help_with_message() {
    let parsed = parse_cli_args(&["deploy"]);
    assert_eq!(parsed.command, CliCommand::Start);
    assert!(parsed.show_help);
    assert!(parsed.error.as_deref().unwrap().contains("Unknown command"));
}

#[test]
fn slice38_10_invalid_mode_value_shows_help() {
    let parsed = parse_cli_args(&["start", "--mode=invalid"]);
    assert!(!parsed.daemon);
    assert!(parsed.show_help);
    assert!(
        parsed
            .error
            .as_deref()
            .unwrap()
            .contains("Invalid mode value")
    );
}

#[test]
fn slice38_11_bare_mode_value_rejected() {
    let parsed = parse_cli_args(&["start", "--mode", "qa"]);
    assert!(parsed.show_help);
    assert!(
        parsed
            .error
            .as_deref()
            .unwrap()
            .contains("Invalid mode value")
    );
}

#[test]
fn slice38_12_mode_without_value_reports_requires_value() {
    let parsed = parse_cli_args(&["start", "--mode"]);
    assert!(parsed.show_help);
    assert!(parsed.error.as_deref().unwrap().contains("requires a value"));
}

#[test]
fn slice38_13_mode_flag_value_rejected() {
    let parsed = parse_cli_args(&["start", "--mode", "--daemon"]);
    assert!(parsed.show_help);
    assert!(parsed.error.as_deref().unwrap().contains("requires a value"));
}

#[test]
fn slice38_14_mode_rejected_for_non_start_commands() {
    let parsed = parse_cli_args(&["status", "--mode", "sources"]);
    assert_eq!(parsed.command, CliCommand::Status);
    assert!(parsed.show_help);
    assert!(
        parsed
            .error
            .as_deref()
            .unwrap()
            .contains("supported only for the start command")
    );
}

#[test]
fn slice38_15_daemon_rejected_for_non_start_commands() {
    let parsed = parse_cli_args(&["status", "--daemon"]);
    assert_eq!(parsed.command, CliCommand::Status);
    assert!(parsed.daemon);
    assert!(parsed.show_help);
    assert!(parsed.error.as_deref().unwrap().contains("--daemon"));
}

// --- parse_cli_args help + options (5) ---

#[test]
fn slice38_16_help_flag_shows_help() {
    let parsed = parse_cli_args(&["--help"]);
    assert!(parsed.show_help);
    assert_eq!(parsed.error, None);
}

#[test]
fn slice38_17_short_help_flag_shows_help() {
    assert!(parse_cli_args(&["-h"]).show_help);
}

#[test]
fn slice38_18_status_stop_config_parse() {
    assert_eq!(parse_cli_args(&["status"]).command, CliCommand::Status);
    assert_eq!(parse_cli_args(&["stop"]).command, CliCommand::Stop);
    assert_eq!(parse_cli_args(&["config"]).command, CliCommand::Config);
}

#[test]
fn slice38_19_unknown_option_shows_help() {
    let parsed = parse_cli_args(&["start", "--nope"]);
    assert!(parsed.show_help);
    assert!(
        parsed
            .error
            .as_deref()
            .unwrap()
            .contains("Unknown option")
    );
}

#[test]
fn slice38_20_error_message_builders() {
    assert_eq!(cli_unknown_command_message("deploy"), "Unknown command: deploy");
    assert!(
        cli_invalid_mode_message("qa")
            .contains("Invalid mode value: qa")
    );
    assert_eq!(cli_unknown_option_message("--nope"), "Unknown option: --nope");
}

// --- runtime mode normalize/parse (5) ---

#[test]
fn slice38_21_normalize_runtime_mode() {
    assert_eq!(
        normalize_runtime_mode("installed"),
        Some(RuntimeMode::Installed)
    );
    assert_eq!(normalize_runtime_mode("sources"), Some(RuntimeMode::Sources));
    assert_eq!(normalize_runtime_mode("qa"), None);
    assert_eq!(normalize_runtime_mode(""), None);
}

#[test]
fn slice38_22_parse_mode_from_argv_space_form() {
    assert_eq!(
        parse_mode_from_argv(&["--mode", "installed"]),
        Some(RuntimeMode::Installed)
    );
    assert_eq!(
        parse_mode_from_argv(&["start", "--mode", "sources"]),
        Some(RuntimeMode::Sources)
    );
}

#[test]
fn slice38_23_parse_mode_from_argv_equals_form() {
    assert_eq!(
        parse_mode_from_argv(&["--mode=installed"]),
        Some(RuntimeMode::Installed)
    );
    assert_eq!(parse_mode_from_argv(&[]), None);
}

#[test]
fn slice38_24_parse_mode_last_wins() {
    assert_eq!(
        parse_mode_from_argv(&["--mode", "sources", "--mode=installed"]),
        Some(RuntimeMode::Installed)
    );
}

#[test]
fn slice38_25_parse_mode_missing_value_is_none() {
    assert_eq!(parse_mode_from_argv(&["--mode"]), None);
    assert_eq!(parse_mode_from_argv(&["--mode", "--daemon"]), None);
}

// --- has_invalid_mode_syntax + resolve (6) ---

#[test]
fn slice38_26_valid_syntax_is_not_invalid() {
    assert!(!has_invalid_mode_syntax(&[]));
    assert!(!has_invalid_mode_syntax(&["--mode", "installed"]));
    assert!(!has_invalid_mode_syntax(&["--mode=installed"]));
}

#[test]
fn slice38_27_unknown_mode_is_invalid_syntax() {
    assert!(has_invalid_mode_syntax(&["--mode", "qa"]));
    assert!(has_invalid_mode_syntax(&["--mode=invalid"]));
}

#[test]
fn slice38_28_missing_mode_value_is_invalid_syntax() {
    assert!(has_invalid_mode_syntax(&["--mode"]));
    assert!(has_invalid_mode_syntax(&["--mode", "--daemon"]));
}

#[test]
fn slice38_29_resolve_defaults_without_argv() {
    assert_eq!(
        resolve_runtime_mode(RuntimeMode::Sources, &[], None),
        ResolvedRuntimeMode {
            mode: RuntimeMode::Sources,
            error: None,
        }
    );
}

#[test]
fn slice38_30_resolve_argv_overrides_default() {
    let out = resolve_runtime_mode(RuntimeMode::Sources, &["--mode", "installed"], None);
    assert_eq!(out.mode, RuntimeMode::Installed);
    assert_eq!(out.error, None);
    let back = resolve_runtime_mode(RuntimeMode::Installed, &["--mode", "sources"], None);
    assert_eq!(back.mode, RuntimeMode::Sources);
}

#[test]
fn slice38_31_resolve_invalid_argv_keeps_default_with_error() {
    let out = resolve_runtime_mode(RuntimeMode::Installed, &["--mode=invalid"], None);
    assert_eq!(out.mode, RuntimeMode::Installed);
    assert!(out.error.as_deref().unwrap().contains("Invalid value for --mode"));
}

#[test]
fn slice38_32_resolve_explicit_mode_wins_over_argv() {
    let out = resolve_runtime_mode(
        RuntimeMode::Sources,
        &["--mode", "sources"],
        Some(RuntimeMode::Installed),
    );
    assert_eq!(out.mode, RuntimeMode::Installed);
    assert_eq!(out.error, None);
}

#[test]
fn slice38_33_env_mode_normalization_falls_back_to_sources() {
    assert_eq!(normalize_env_mode(Some("installed")), RuntimeMode::Installed);
    assert_eq!(normalize_env_mode(Some("sources")), RuntimeMode::Sources);
    assert_eq!(normalize_env_mode(Some("qa")), RuntimeMode::Sources);
    assert_eq!(normalize_env_mode(Some("")), RuntimeMode::Sources);
    assert_eq!(normalize_env_mode(None), RuntimeMode::Sources);
}

// --- retry/startup/delivery sets (6) ---

#[test]
fn slice38_34_safe_state_updates_retry_after_5xx() {
    for method in [
        "editMessageReplyMarkup",
        "editMessageText",
        "sendChatAction",
        "sendMessageDraft",
        "sendRichMessageDraft",
    ] {
        assert!(should_retry_telegram_server_error(method), "{method}");
    }
}

#[test]
fn slice38_35_message_creation_does_not_retry_5xx() {
    for method in [
        "sendMessage",
        "sendRichMessage",
        "sendDocument",
        "sendAudio",
        "sendPhoto",
        "deleteMessage",
        "unknownMethod",
    ] {
        assert!(!should_retry_telegram_server_error(method), "{method}");
    }
}

#[test]
fn slice38_36_startup_managed_methods_bypass_transformer_retry() {
    assert!(is_startup_managed_method("deleteWebhook"));
    assert!(is_startup_managed_method("getMe"));
    assert!(is_startup_managed_method("getWebhookInfo"));
    assert!(!is_startup_managed_method("sendMessage"));
    assert!(!is_startup_managed_method("getUpdates"));
}

#[test]
fn slice38_37_chat_delivery_methods_are_tracked() {
    for method in [
        "sendMessage",
        "sendRichMessage",
        "editMessageText",
        "sendDocument",
        "sendAudio",
        "sendPhoto",
    ] {
        assert!(is_chat_delivery_method(method), "{method}");
    }
    assert!(!is_chat_delivery_method("getMe"));
    assert!(!is_chat_delivery_method("sendChatAction"));
}

// --- link preview default (4) ---

#[test]
fn slice38_38_text_send_or_edit_without_choice_gets_default() {
    assert!(should_apply_link_preview_default("sendMessage", true, false));
    assert!(should_apply_link_preview_default("editMessageText", true, false));
}

#[test]
fn slice38_39_caller_choice_is_kept() {
    assert!(!should_apply_link_preview_default("sendMessage", true, true));
    assert!(!should_apply_link_preview_default("editMessageText", true, true));
}

#[test]
fn slice38_40_non_text_payloads_keep_flowing_untouched() {
    assert!(!should_apply_link_preview_default("sendMessage", false, false));
    assert!(!should_apply_link_preview_default("editMessageText", false, false));
}

#[test]
fn slice38_41_other_methods_are_never_touched() {
    assert!(!should_apply_link_preview_default("sendRichMessage", true, false));
    assert!(!should_apply_link_preview_default("sendDocument", true, false));
    assert!(!should_apply_link_preview_default("editMessageReplyMarkup", true, false));
}

// --- client wiring gates (7) ---

#[test]
fn slice38_42_client_configured_for_custom_root_or_secret() {
    assert!(should_configure_telegram_client("https://tg-proxy.example.com", ""));
    assert!(should_configure_telegram_client("", "secret-abc"));
    assert!(should_configure_telegram_client("https://tg-proxy.example.com", "secret-abc"));
    assert!(!should_configure_telegram_client("", ""));
}

#[test]
fn slice38_43_forward_proxy_wired_when_url_present() {
    assert!(should_use_forward_proxy("https://proxy.example.com:8443"));
    assert!(should_use_forward_proxy("socks5://proxy.example.com:1080"));
    assert!(!should_use_forward_proxy(""));
}

#[test]
fn slice38_44_socks_scheme_picked_by_prefix() {
    assert_eq!(proxy_agent_kind("socks5://proxy.example.com:1080"), "socks");
    assert_eq!(proxy_agent_kind("socks://proxy.example.com:1080"), "socks");
    assert_eq!(proxy_agent_kind("https://proxy.example.com:8443"), "https");
    assert_eq!(proxy_agent_kind("http://proxy.example.com:8080"), "https");
    assert_eq!(proxy_agent_kind(""), "none");
}

#[test]
fn slice38_45_proxy_credentials_redacted_for_logs() {
    assert_eq!(
        redact_proxy_credentials("https://user:pass@proxy.example.com:8443"),
        "https://***@proxy.example.com:8443"
    );
}

#[test]
fn slice38_46_credential_free_url_passes_through() {
    assert_eq!(
        redact_proxy_credentials("https://proxy.example.com:8443"),
        "https://proxy.example.com:8443"
    );
}

#[test]
fn slice38_47_forward_proxy_wins_over_ipv4_direct() {
    // TS wires `proxyUrl` in the `if` branch and only reaches the
    // `forceIpv4` branch in `else if`: any non-empty proxy URL means the
    // direct IPv4 agent is NOT the path taken.
    assert!(should_use_forward_proxy("https://proxy.example.com:8443"));
    assert!(!should_use_forward_proxy(""));
}

#[test]
fn slice38_48_installed_and_sources_modes_are_distinct() {
    assert_ne!(RuntimeMode::Sources, RuntimeMode::Installed);
    assert_ne!(CliCommand::Start, CliCommand::Status);
}
