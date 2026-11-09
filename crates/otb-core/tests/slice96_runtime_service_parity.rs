//! Slice 96 parity: runtime service / health-port / node-version / logger gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/runtime/node-version.ts` (`LOWEST_SUPPORTED_MAJOR`,
//! `getUnsupportedNodeVersionMessage` major/minor/minimum-minor gates),
//! `src/runtime/health-port.ts` (`BOT_HEALTH_PORT_ENV`,
//! `resolveHealthPort` trim / digit / ceiling gates),
//! `src/runtime/service/env.ts` (`SERVICE_CHILD_ENV_KEY`,
//! `SERVICE_STATE_PATH_ENV_KEY`, child marker, state-path blank gate,
//! child-env spread-overwrite),
//! `src/runtime/service/manager.ts` pure subset
//! (`SERVICE_STATE_FILE_NAME`, `PROCESS_EXIT_POLL_MS`,
//! `DEFAULT_STOP_TIMEOUT_MS`, `sanitizeTimestampForFile`,
//! `isValidServiceState`),
//! `src/utils/logger.ts` pure subset (`DEFAULT_LOG_LEVEL`,
//! `DEFAULT_LOG_RETENTION`, `LOG_LEVELS` ranks, `normalizeLogLevel`,
//! `parsePositiveInteger`) and
//! `src/cli/failure-exit.ts` (`LOG_FLUSH_TIMEOUT_MS`,
//! `EXIT_RUNTIME_ERROR`).
//!
//! Already covered elsewhere and NOT re-tested here:
//! `src/runtime/mode.ts` / `container.ts` / `paths.ts` (`stores_runtime`),
//! `src/cli/args.ts` (`bot_bootstrap_gates`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `runtime_service_gates` is ported (only the constant-value tests pass).

use otb_core::runtime_service_gates::*;

// ---- constants (6) ----

#[test]
fn lowest_supported_node_major_value_parity() {
    assert_eq!(LOWEST_SUPPORTED_NODE_MAJOR, 22);
}

#[test]
fn supported_node_versions_value_parity() {
    assert_eq!(SUPPORTED_NODE_VERSIONS, "22.14+, 23.6+, or 24+");
}

#[test]
fn bot_health_port_env_value_parity() {
    assert_eq!(BOT_HEALTH_PORT_ENV, "BOT_HEALTH_PORT");
}

#[test]
fn service_state_file_name_value_parity() {
    assert_eq!(SERVICE_STATE_FILE_NAME, "bot-service.json");
}

#[test]
fn service_timing_constants_value_parity() {
    assert_eq!(PROCESS_EXIT_POLL_MS, 100);
    assert_eq!(DEFAULT_STOP_TIMEOUT_MS, 5000);
    assert_eq!(CONTAINER_DEFAULT_HEALTH_PORT, 3100);
    assert_eq!(MAX_PORT, 65535);
}

#[test]
fn cli_failure_constants_value_parity() {
    assert_eq!(LOG_FLUSH_TIMEOUT_MS, 1000);
    assert_eq!(EXIT_RUNTIME_ERROR, 1);
    assert_eq!(DEFAULT_LOG_LEVEL, "info");
    assert_eq!(DEFAULT_LOG_RETENTION, 10);
}

// ---- node version (8) ----

#[test]
fn node_supported_at_22_14_parity() {
    assert_eq!(get_unsupported_node_version_message("22.14.0"), None);
}

#[test]
fn node_rejected_below_22_14_parity() {
    let message = get_unsupported_node_version_message("22.13.9").expect("unsupported message");
    assert!(message.contains("22.14+, 23.6+, or 24+"));
    assert!(message.contains("v22.13.9"));
    assert!(message.contains("https://nodejs.org"));
}

#[test]
fn node_supported_at_23_6_parity() {
    assert_eq!(get_unsupported_node_version_message("23.6.0"), None);
}

#[test]
fn node_rejected_below_23_6_parity() {
    assert!(get_unsupported_node_version_message("23.5.2").is_some());
}

#[test]
fn node_supported_on_24_and_later_parity() {
    assert_eq!(get_unsupported_node_version_message("24.0.0"), None);
    assert_eq!(get_unsupported_node_version_message("25.1.3"), None);
}

#[test]
fn node_rejected_below_lowest_major_parity() {
    assert!(get_unsupported_node_version_message("21.99.0").is_some());
    assert!(get_unsupported_node_version_message("20.18.1").is_some());
}

#[test]
fn node_unparseable_version_reads_supported_parity() {
    assert_eq!(get_unsupported_node_version_message("abc"), None);
    assert_eq!(get_unsupported_node_version_message(""), None);
}

#[test]
fn node_missing_minor_reads_supported_parity() {
    assert_eq!(get_unsupported_node_version_message("22"), None);
}

// ---- health port (8) ----

#[test]
fn health_port_container_default_parity() {
    assert_eq!(resolve_health_port(None, true), 3100);
    assert_eq!(resolve_health_port(Some(""), true), 3100);
}

#[test]
fn health_port_bare_default_is_off_parity() {
    assert_eq!(resolve_health_port(None, false), 0);
    assert_eq!(resolve_health_port(Some("   "), false), 0);
}

#[test]
fn health_port_valid_value_kept_parity() {
    assert_eq!(resolve_health_port(Some("4096"), false), 4096);
    assert_eq!(resolve_health_port(Some("65535"), true), 65535);
}

#[test]
fn health_port_over_ceiling_falls_back_parity() {
    assert_eq!(resolve_health_port(Some("65536"), false), 0);
    assert_eq!(resolve_health_port(Some("99999"), true), 3100);
}

#[test]
fn health_port_non_numeric_falls_back_parity() {
    assert_eq!(resolve_health_port(Some("abc"), false), 0);
    assert_eq!(resolve_health_port(Some("-1"), true), 3100);
}

#[test]
fn health_port_trailing_junk_falls_back_parity() {
    // `^\d+$` rejects what `parseInt` would otherwise accept.
    assert_eq!(resolve_health_port(Some("3100abc"), false), 0);
    assert_eq!(resolve_health_port(Some("3.5"), false), 0);
}

#[test]
fn health_port_zero_means_off_parity() {
    assert_eq!(resolve_health_port(Some("0"), false), 0);
    assert_eq!(resolve_health_port(Some("0"), true), 0);
}

#[test]
fn health_port_trims_surrounding_space_parity() {
    assert_eq!(resolve_health_port(Some("  3100  "), false), 3100);
}

// ---- service env (7) ----

#[test]
fn service_env_keys_value_parity() {
    assert_eq!(SERVICE_CHILD_ENV_KEY, "OPENCODE_TELEGRAM_SERVICE_CHILD");
    assert_eq!(
        SERVICE_STATE_PATH_ENV_KEY,
        "OPENCODE_TELEGRAM_SERVICE_STATE_PATH"
    );
    assert_eq!(SERVICE_CHILD_ENV_VALUE, "1");
}

#[test]
fn service_child_marker_gate_parity() {
    assert!(is_service_child_process(Some("1")));
    assert!(!is_service_child_process(Some("0")));
    assert!(!is_service_child_process(Some("true")));
    assert!(!is_service_child_process(None));
}

#[test]
fn service_state_path_kept_parity() {
    assert_eq!(
        get_service_state_path_from_env(Some("/tmp/bot-service.json")),
        Some("/tmp/bot-service.json".to_string())
    );
}

#[test]
fn service_state_path_blank_gate_parity() {
    assert_eq!(get_service_state_path_from_env(None), None);
    assert_eq!(get_service_state_path_from_env(Some("")), None);
    assert_eq!(get_service_state_path_from_env(Some("   ")), None);
}

#[test]
fn service_child_env_merges_base_parity() {
    let entries = build_service_child_entries(&[("A", "1"), ("B", "2")], "/tmp/s.json");
    assert!(entries.contains(&("A".to_string(), "1".to_string())));
    assert!(entries.contains(&(SERVICE_CHILD_ENV_KEY.to_string(), "1".to_string())));
    assert!(entries.contains(&(
        SERVICE_STATE_PATH_ENV_KEY.to_string(),
        "/tmp/s.json".to_string()
    )));
}

#[test]
fn service_child_env_overwrites_collisions_parity() {
    let entries = build_service_child_entries(
        &[(SERVICE_CHILD_ENV_KEY, "0"), ("KEEP", "yes")],
        "/tmp/s.json",
    );
    let child: Vec<_> = entries
        .iter()
        .filter(|(key, _)| key == SERVICE_CHILD_ENV_KEY)
        .collect();
    assert_eq!(child.len(), 1);
    assert_eq!(child[0].1, "1");
}

#[test]
fn service_child_env_empty_base_parity() {
    let entries = build_service_child_entries(&[], "/tmp/s.json");
    assert_eq!(entries.len(), 2);
}

// ---- timestamps (5) ----

#[test]
fn sanitize_timestamp_full_parity() {
    assert_eq!(
        sanitize_timestamp_for_file("2026-11-08T12:34:56"),
        "2026-11-08_12-34-56"
    );
}

#[test]
fn sanitize_timestamp_without_markers_parity() {
    assert_eq!(sanitize_timestamp_for_file("plain"), "plain");
    assert_eq!(sanitize_timestamp_for_file("a:b:c"), "a-b-c");
}

#[test]
fn sanitize_timestamp_replaces_first_t_only_parity() {
    assert_eq!(sanitize_timestamp_for_file("T0T1:2"), "_0T1-2");
}

#[test]
fn service_log_file_name_parity() {
    assert_eq!(
        service_log_file_name("2026-11-08T12:34:56"),
        "bot-service-2026-11-08_12-34-56.log"
    );
}

#[test]
fn service_log_file_name_sanitizes_parity() {
    let name = service_log_file_name("2026-01-02T03:04:05");
    assert!(name.starts_with("bot-service-"));
    assert!(name.ends_with(".log"));
    assert!(!name.contains(':'));
}

// ---- service state (6) ----

fn valid_state() -> ServiceStateShape {
    ServiceStateShape {
        pid: Some(123.0),
        started_at: Some("2026-11-08T12:00:00".to_string()),
        log_file_path: Some("/tmp/bot-service.log".to_string()),
        mode: Some("daemon".to_string()),
    }
}

#[test]
fn service_state_valid_parity() {
    assert!(is_valid_service_state(&valid_state()));
}

#[test]
fn service_state_rejects_bad_pid_parity() {
    assert!(!is_valid_service_state(&ServiceStateShape {
        pid: Some(0.0),
        ..valid_state()
    }));
    assert!(!is_valid_service_state(&ServiceStateShape {
        pid: Some(-3.0),
        ..valid_state()
    }));
    assert!(!is_valid_service_state(&ServiceStateShape {
        pid: None,
        ..valid_state()
    }));
}

#[test]
fn service_state_rejects_fractional_pid_parity() {
    assert!(!is_valid_service_state(&ServiceStateShape {
        pid: Some(1.5),
        ..valid_state()
    }));
}

#[test]
fn service_state_rejects_blank_fields_parity() {
    assert!(!is_valid_service_state(&ServiceStateShape {
        started_at: Some(String::new()),
        ..valid_state()
    }));
    assert!(!is_valid_service_state(&ServiceStateShape {
        log_file_path: Some(String::new()),
        ..valid_state()
    }));
}

#[test]
fn service_state_rejects_wrong_mode_parity() {
    assert!(!is_valid_service_state(&ServiceStateShape {
        mode: Some("service".to_string()),
        ..valid_state()
    }));
    assert!(!is_valid_service_state(&ServiceStateShape {
        mode: None,
        ..valid_state()
    }));
}

#[test]
fn service_state_rejects_empty_parity() {
    assert!(!is_valid_service_state(&ServiceStateShape::default()));
}

// ---- logger (8) ----

#[test]
fn log_level_normalize_known_parity() {
    assert_eq!(normalize_log_level("debug"), "debug");
    assert_eq!(normalize_log_level("info"), "info");
    assert_eq!(normalize_log_level("warn"), "warn");
    assert_eq!(normalize_log_level("error"), "error");
}

#[test]
fn log_level_normalize_unknown_falls_back_parity() {
    assert_eq!(normalize_log_level("verbose"), "info");
    assert_eq!(normalize_log_level(""), "info");
}

#[test]
fn log_level_normalize_is_case_sensitive_parity() {
    assert_eq!(normalize_log_level("INFO"), "info");
    assert_eq!(normalize_log_level("Debug"), "info");
}

#[test]
fn parse_positive_integer_valid_parity() {
    assert_eq!(parse_positive_integer(Some("5"), 10), 5);
    assert_eq!(parse_positive_integer(Some("  7  "), 10), 7);
}

#[test]
fn parse_positive_integer_junk_falls_back_parity() {
    assert_eq!(parse_positive_integer(Some("abc"), 10), 10);
    assert_eq!(parse_positive_integer(None, 10), 10);
    assert_eq!(parse_positive_integer(Some(""), 10), 10);
}

#[test]
fn parse_positive_integer_non_positive_falls_back_parity() {
    assert_eq!(parse_positive_integer(Some("0"), 10), 10);
    assert_eq!(parse_positive_integer(Some("-3"), 10), 10);
}

#[test]
fn log_level_rank_table_parity() {
    assert_eq!(log_level_rank("debug"), Some(0));
    assert_eq!(log_level_rank("info"), Some(1));
    assert_eq!(log_level_rank("warn"), Some(2));
    assert_eq!(log_level_rank("error"), Some(3));
    assert_eq!(log_level_rank("verbose"), None);
}

#[test]
fn should_log_floor_gate_parity() {
    assert!(should_log("error", "warn"));
    assert!(should_log("warn", "warn"));
    assert!(!should_log("debug", "info"));
    assert!(should_log("info", "info"));
}
