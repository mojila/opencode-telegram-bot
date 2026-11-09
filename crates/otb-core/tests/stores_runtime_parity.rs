//! Parity port of the stores/runtime/opencode-client pure subset (slice 30b, RED).
//!
//! Mirrors representative `it`s from:
//! - `tests/app/stores/settings-store.test.ts` (streaming/TTS/queue/preset gates)
//! - `tests/app/stores/scheduled-task-store.test.ts` (agent backfill, enabled
//!   default, toggle noop, session-ignore TTL)
//! - `tests/config.test.ts` remainder already covered by `types_config`
//!   (mode/proxy/port/TTS defaults) — here only the preset/TTS/streaming
//!   validators not yet ported
//! - `tests/app/bootstrap/app-container.test.ts` (heartbeat cadence)
//! - `tests/app/services/scheduled-task-session-ignore-service` TTL via the
//!   store tests + `tests/app/services/scheduled-task-next-run-service`
//!   due-gate shape
//! - `tests/opencode/server-target.test.ts` + `tests/opencode/client.test.ts`
//!   (describe/switch/probe/auth pure helpers)
//! - `tests/app/start-bot-app.test.ts` (fatal/retryable codes, 60s cap)
//!
//! Step 1 (RED): these tests assert the TS contract against
//! `otb_core::stores_runtime` stubs and MUST FAIL with `unimplemented!()`
//! panics until the Rust port lands (step 2). No FS, env, network, or timers.

use otb_core::stores_runtime::{
    activation_pending_after_enable, backfill_task_agent, build_basic_auth, capped_retry_delay_ms,
    classify_probe, cron_toggle_is_noop, default_task_enabled, describe_server_url,
    has_invalid_mode_syntax, heartbeat_should_log, is_container_runtime, is_enabled_flag,
    is_fatal_token_code, is_fresh_ignore, is_known_preset_key, is_retryable_server_code,
    is_task_due_ms, is_valid_preset_streaming_value, is_valid_preset_tts_value, join_probe_url,
    map_set_prompt_queue_enabled, map_set_prompt_queue_mode, next_startup_retry_delay_ms,
    normalize_response_streaming_mode, normalize_runtime_mode, normalize_tts_mode,
    parse_mode_from_argv, probe_route, resolve_app_home, resolve_prompt_queue_mode,
    resolve_runtime_mode, resolve_switched_server, should_ignore_session, HEARTBEAT_INTERVAL_MS,
    HEARTBEAT_LOG_EVERY_TICKS, SESSION_IGNORE_TTL_MS,
};

// --- response-streaming normalization (settings-store: responseStreamingMode) ---

#[test]
fn slice30b_01_streaming_draft_stays_draft() {
    assert_eq!(normalize_response_streaming_mode(Some("draft")), "draft");
}

#[test]
fn slice30b_02_streaming_falls_back_to_edit() {
    assert_eq!(normalize_response_streaming_mode(None), "edit");
    assert_eq!(normalize_response_streaming_mode(Some("DRAFT")), "edit");
    assert_eq!(normalize_response_streaming_mode(Some("")), "edit");
}

// --- TTS mode normalization (settings-store: getTtsMode) ---

#[test]
fn slice30b_03_tts_known_modes_kept() {
    assert_eq!(normalize_tts_mode(None), "off");
    assert_eq!(normalize_tts_mode(Some("all")), "all");
    assert_eq!(normalize_tts_mode(Some("auto")), "auto");
}

#[test]
fn slice30b_04_tts_invalid_falls_back_to_off() {
    assert_eq!(normalize_tts_mode(Some("banana")), "off");
    assert_eq!(normalize_tts_mode(Some("")), "off");
}

// --- prompt-queue resolution (getPromptQueueMode / setPromptQueueMode) ---

#[test]
fn slice30b_05_prompt_queue_v1_reads_boolean() {
    assert_eq!(resolve_prompt_queue_mode("v1", None, None), "off");
    assert_eq!(resolve_prompt_queue_mode("v1", Some(true), None), "queue");
    assert_eq!(resolve_prompt_queue_mode("v1", Some(false), None), "off");
}

#[test]
fn slice30b_06_prompt_queue_v2_ignores_boolean() {
    assert_eq!(resolve_prompt_queue_mode("v2", None, None), "steer");
    assert_eq!(resolve_prompt_queue_mode("v2", Some(true), None), "steer");
    assert_eq!(resolve_prompt_queue_mode("v2", None, Some("off")), "off");
}

#[test]
fn slice30b_07_set_maps_enabled_only_on_v1() {
    assert_eq!(map_set_prompt_queue_enabled("v1", "queue"), Some(true));
    assert_eq!(map_set_prompt_queue_enabled("v1", "off"), Some(false));
    assert_eq!(map_set_prompt_queue_enabled("v2", "off"), None);
}

#[test]
fn slice30b_08_set_maps_mode_only_on_v2() {
    assert_eq!(map_set_prompt_queue_mode("v2", "off"), Some("off"));
    assert_eq!(map_set_prompt_queue_mode("v2", "steer"), Some("steer"));
    assert_eq!(map_set_prompt_queue_mode("v1", "queue"), None);
}

// --- preset validation (applyInitialSettingsPreset) ---

#[test]
fn slice30b_09_preset_known_keys() {
    assert!(is_known_preset_key("ttsMode"));
    assert!(is_known_preset_key("compactOutputMode"));
    assert!(is_known_preset_key("responseStreamingMode"));
    assert!(is_known_preset_key("promptQueueEnabled"));
    assert!(is_known_preset_key("showBackgroundSessionNotifications"));
}

#[test]
fn slice30b_10_preset_unknown_keys_rejected() {
    assert!(!is_known_preset_key("unknownKey"));
    assert!(!is_known_preset_key(""));
    assert!(!is_known_preset_key("ttsEnabled"));
}

#[test]
fn slice30b_11_preset_tts_values() {
    assert!(is_valid_preset_tts_value("off"));
    assert!(is_valid_preset_tts_value("all"));
    assert!(is_valid_preset_tts_value("auto"));
    assert!(!is_valid_preset_tts_value("banana"));
}

#[test]
fn slice30b_12_preset_streaming_values() {
    assert!(is_valid_preset_streaming_value("edit"));
    assert!(is_valid_preset_streaming_value("draft"));
    assert!(!is_valid_preset_streaming_value("raw"));
}

// --- scheduled-task backfill (cloneScheduledTask) ---

#[test]
fn slice30b_13_agent_backfill_build() {
    assert_eq!(backfill_task_agent(None), "build");
    assert_eq!(backfill_task_agent(Some("build")), "build");
}

#[test]
fn slice30b_14_agent_keeps_custom() {
    assert_eq!(backfill_task_agent(Some("explore")), "explore");
}

#[test]
fn slice30b_15_enabled_defaults_true() {
    assert!(default_task_enabled(None));
    assert!(default_task_enabled(Some(true)));
    assert!(!default_task_enabled(Some(false)));
}

// --- toggle guard (scheduled-task-store: toggleScheduledTask) ---

#[test]
fn slice30b_16_toggle_noop_for_once_kind() {
    assert!(cron_toggle_is_noop("once", true, false));
    assert!(cron_toggle_is_noop("once", false, true));
}

#[test]
fn slice30b_17_toggle_noop_for_same_state() {
    assert!(cron_toggle_is_noop("cron", true, true));
    assert!(cron_toggle_is_noop("cron", false, false));
}

#[test]
fn slice30b_18_toggle_writes_on_flip() {
    assert!(!cron_toggle_is_noop("cron", true, false));
    assert!(!cron_toggle_is_noop("cron", false, true));
}

#[test]
fn slice30b_19_activation_pending_only_on_enable() {
    assert_eq!(activation_pending_after_enable(true), Some(true));
    assert_eq!(activation_pending_after_enable(false), None);
}

// --- session-ignore TTL (24h) ---

#[test]
fn slice30b_20_ignore_ttl_is_24h() {
    assert_eq!(SESSION_IGNORE_TTL_MS, 24 * 60 * 60 * 1000);
}

#[test]
fn slice30b_21_fresh_ignore_within_ttl() {
    let now = 1_746_000_000_000i64;
    assert!(is_fresh_ignore(now - 2 * 60 * 60 * 1000, now));
}

#[test]
fn slice30b_22_stale_ignore_beyond_ttl() {
    let now = 1_746_000_000_000i64;
    assert!(!is_fresh_ignore(now - 25 * 60 * 60 * 1000, now));
    assert!(!is_fresh_ignore(now - SESSION_IGNORE_TTL_MS, now));
}

#[test]
fn slice30b_23_ignore_needs_match_and_fresh() {
    assert!(should_ignore_session(true, true));
    assert!(!should_ignore_session(true, false));
    assert!(!should_ignore_session(false, true));
}

// --- task due gate (isTaskDue) ---

#[test]
fn slice30b_24_due_when_past() {
    assert!(is_task_due_ms(Some(1000), 2000));
    assert!(is_task_due_ms(Some(2000), 2000));
}

#[test]
fn slice30b_25_not_due_when_missing_or_future() {
    assert!(!is_task_due_ms(None, 2000));
    assert!(!is_task_due_ms(Some(3000), 2000));
}

// --- runtime mode (runtime/mode.ts) ---

#[test]
fn slice30b_26_normalize_runtime_mode() {
    assert_eq!(normalize_runtime_mode("installed"), Some("installed"));
    assert_eq!(normalize_runtime_mode("sources"), Some("sources"));
    assert_eq!(normalize_runtime_mode("banana"), None);
}

#[test]
fn slice30b_27_parse_mode_argv_forms() {
    assert_eq!(
        parse_mode_from_argv(&["--mode", "installed"]),
        Some("installed")
    );
    assert_eq!(parse_mode_from_argv(&["--mode=sources"]), Some("sources"));
    assert_eq!(parse_mode_from_argv(&[]), None);
}

#[test]
fn slice30b_28_parse_mode_last_wins() {
    assert_eq!(
        parse_mode_from_argv(&["--mode", "sources", "--mode=installed"]),
        Some("installed")
    );
}

#[test]
fn slice30b_29_invalid_mode_syntax() {
    assert!(!has_invalid_mode_syntax(&["--mode", "installed"]));
    assert!(has_invalid_mode_syntax(&["--mode", "banana"]));
    assert!(has_invalid_mode_syntax(&["--mode"]));
    assert!(has_invalid_mode_syntax(&["--mode="]));
}

#[test]
fn slice30b_30_resolve_runtime_explicit_wins() {
    let out = resolve_runtime_mode(Some("installed"), &["--mode", "sources"], "sources");
    assert_eq!(out.mode, "installed");
    assert_eq!(out.error, None);
}

#[test]
fn slice30b_31_resolve_runtime_invalid_argv_errors() {
    let out = resolve_runtime_mode(None, &["--mode", "banana"], "sources");
    assert_eq!(out.mode, "sources");
    assert!(out.error.is_some());
}

#[test]
fn slice30b_32_resolve_runtime_argv_or_default() {
    let from_argv = resolve_runtime_mode(None, &["--mode=installed"], "sources");
    assert_eq!(from_argv.mode, "installed");
    assert_eq!(from_argv.error, None);
    let from_default = resolve_runtime_mode(None, &[], "sources");
    assert_eq!(from_default.mode, "sources");
    assert_eq!(from_default.error, None);
}

// --- container flags (runtime/container.ts) ---

#[test]
fn slice30b_33_enabled_flag_on() {
    assert!(is_enabled_flag(Some("1")));
    assert!(is_enabled_flag(Some("true")));
    assert!(is_enabled_flag(Some("yes")));
}

#[test]
fn slice30b_34_enabled_flag_off() {
    assert!(!is_enabled_flag(None));
    assert!(!is_enabled_flag(Some("")));
    assert!(!is_enabled_flag(Some("0")));
    assert!(!is_enabled_flag(Some("false")));
    assert!(!is_enabled_flag(Some("no")));
}

#[test]
fn slice30b_35_container_runtime_sources() {
    assert!(is_container_runtime(Some("1"), false));
    assert!(!is_container_runtime(None, false));
    assert!(is_container_runtime(None, true));
}

// --- heartbeat (app-container.ts) ---

#[test]
fn slice30b_36_heartbeat_cadence_consts() {
    assert_eq!(HEARTBEAT_INTERVAL_MS, 5000);
    assert_eq!(HEARTBEAT_LOG_EVERY_TICKS, 6);
}

#[test]
fn slice30b_37_heartbeat_logs_every_sixth_tick() {
    assert!(heartbeat_should_log(6));
    assert!(heartbeat_should_log(12));
    assert!(!heartbeat_should_log(5));
    assert!(!heartbeat_should_log(1));
}

// --- paths (runtime/paths.ts) ---

#[test]
fn slice30b_38_home_override_wins() {
    assert_eq!(
        resolve_app_home("sources", Some("/tmp/home"), "/cwd", "/installed"),
        "/tmp/home".to_string()
    );
    assert_eq!(
        resolve_app_home("installed", Some("  /tmp/home  "), "/cwd", "/installed"),
        "/tmp/home".to_string()
    );
}

#[test]
fn slice30b_39_home_mode_branching() {
    assert_eq!(
        resolve_app_home("sources", None, "/cwd", "/installed"),
        "/cwd".to_string()
    );
    assert_eq!(
        resolve_app_home("installed", None, "/cwd", "/installed"),
        "/installed".to_string()
    );
}

// --- server target (opencode/server-target.ts) ---

#[test]
fn slice30b_40_describe_strips_credentials() {
    assert_eq!(
        describe_server_url("http://user:secret@127.0.0.1:4096"),
        "http://127.0.0.1:4096".to_string()
    );
}

#[test]
fn slice30b_41_describe_falls_back_on_invalid() {
    assert_eq!(
        describe_server_url("not a url"),
        "the configured OPENCODE_API_URL".to_string()
    );
}

#[test]
fn slice30b_42_switched_server_resolution() {
    assert_eq!(
        resolve_switched_server(None, "http://127.0.0.1:49374"),
        None
    );
    assert_eq!(
        resolve_switched_server(Some("http://127.0.0.1:49374"), "http://127.0.0.1:49374"),
        None
    );
    assert_eq!(
        resolve_switched_server(Some("http://127.0.0.1:28894"), "http://127.0.0.1:49374"),
        Some("http://127.0.0.1:28894".to_string())
    );
}

// --- client probe/auth (opencode/client.ts) ---

#[test]
fn slice30b_43_probe_routes() {
    assert_eq!(probe_route("v1"), "global/health");
    assert_eq!(probe_route("v2"), "api/info");
}

#[test]
fn slice30b_44_basic_auth_header() {
    assert_eq!(build_basic_auth(Some("opencode"), None), None);
    assert_eq!(build_basic_auth(None, Some("")), None);
    assert_eq!(
        build_basic_auth(Some("opencode"), Some("secret")),
        Some("Basic b3BlbmNvZGU6c2VjcmV0".to_string())
    );
}

#[test]
fn slice30b_45_classify_probe() {
    assert_eq!(classify_probe(401, false, false, false), "unauthorized");
    assert_eq!(classify_probe(200, true, true, true), "found");
    assert_eq!(classify_probe(200, false, true, true), "none");
    assert_eq!(classify_probe(500, true, false, false), "none");
}

#[test]
fn slice30b_46_join_probe_url_single_slash() {
    assert_eq!(
        join_probe_url("http://127.0.0.1:49374", "api/info"),
        "http://127.0.0.1:49374/api/info".to_string()
    );
    assert_eq!(
        join_probe_url("http://127.0.0.1:49374/", "api/info"),
        "http://127.0.0.1:49374/api/info".to_string()
    );
}

// --- bootstrap retry (start-bot-app.ts) ---

#[test]
fn slice30b_47_fatal_and_retryable_codes() {
    assert!(is_fatal_token_code(Some(401)));
    assert!(is_fatal_token_code(Some(404)));
    assert!(!is_fatal_token_code(Some(409)));
    assert!(!is_fatal_token_code(None));
    assert!(is_retryable_server_code(Some(502)));
    assert!(is_retryable_server_code(Some(500)));
    assert!(!is_retryable_server_code(Some(429)));
    assert!(!is_retryable_server_code(None));
}

#[test]
fn slice30b_48_retry_delay_cap_and_backoff() {
    assert_eq!(capped_retry_delay_ms(120_000), 60_000);
    assert_eq!(capped_retry_delay_ms(1000), 1000);
    assert_eq!(next_startup_retry_delay_ms(1, None), 1000);
    assert_eq!(next_startup_retry_delay_ms(7, None), 60_000);
    assert_eq!(next_startup_retry_delay_ms(1, Some(120_000)), 60_000);
}
