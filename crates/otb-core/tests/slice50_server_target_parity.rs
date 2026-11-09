//! Parity port of the server-target / local-start pure gates (slice 50, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/opencode/server-target.ts` (`describeOpencodeServerUrl`
//!   protocol/host/path describe with query/hash dropped and the
//!   configured-URL fallback, `switchOpencodeServer` same-or-null gate,
//!   `getActiveOpencodeServer` / `isOnConfiguredOpencodeServer` selectors)
//! - `src/opencode/server-health.ts` (`reportFailedHealthCause`
//!   unauthorized/mismatch keys and log lines; unknown logs nothing)
//! - `src/opencode/local-start.ts` (`getUrlPort` integer-port gate, the
//!   registered-server key/message with both reasons, the
//!   executable-mismatch key/message)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::server_target_gates::*;

// --- describe_opencode_server_url (01-16) ---

#[test]
fn slice50_01_bare_host_and_port() {
    assert_eq!(
        describe_opencode_server_url("http://localhost:4096"),
        "http://localhost:4096"
    );
}

#[test]
fn slice50_02_explicit_port_is_kept() {
    assert_eq!(
        describe_opencode_server_url("http://example.com:5000"),
        "http://example.com:5000"
    );
}

#[test]
fn slice50_03_http_default_port_is_elided() {
    assert_eq!(
        describe_opencode_server_url("http://example.com:80/x"),
        "http://example.com/x"
    );
}

#[test]
fn slice50_04_https_default_port_is_elided() {
    assert_eq!(
        describe_opencode_server_url("https://example.com:443"),
        "https://example.com"
    );
}

#[test]
fn slice50_05_non_default_https_port_is_kept() {
    assert_eq!(
        describe_opencode_server_url("https://example.com:8443/api"),
        "https://example.com:8443/api"
    );
}

#[test]
fn slice50_06_root_path_is_stripped() {
    assert_eq!(
        describe_opencode_server_url("http://example.com:4096/"),
        "http://example.com:4096"
    );
}

#[test]
fn slice50_07_subpath_is_kept() {
    assert_eq!(
        describe_opencode_server_url("http://example.com:4096/some/path"),
        "http://example.com:4096/some/path"
    );
}

#[test]
fn slice50_08_query_is_dropped() {
    assert_eq!(
        describe_opencode_server_url("http://example.com:4096/p?x=1"),
        "http://example.com:4096/p"
    );
}

#[test]
fn slice50_09_fragment_is_dropped() {
    assert_eq!(
        describe_opencode_server_url("http://example.com:4096/p#frag"),
        "http://example.com:4096/p"
    );
}

#[test]
fn slice50_10_bare_query_without_path() {
    assert_eq!(
        describe_opencode_server_url("http://example.com:4096?x=1"),
        "http://example.com:4096"
    );
}

#[test]
fn slice50_11_scheme_and_host_are_lowercased_path_case_kept() {
    assert_eq!(
        describe_opencode_server_url("HTTP://Example.COM:8080/Path"),
        "http://example.com:8080/Path"
    );
}

#[test]
fn slice50_12_userinfo_is_stripped() {
    assert_eq!(
        describe_opencode_server_url("http://user:secret@example.com:4096/x"),
        "http://example.com:4096/x"
    );
}

#[test]
fn slice50_13_ipv6_loopback_keeps_brackets() {
    assert_eq!(
        describe_opencode_server_url("http://[::1]:4096/x"),
        "http://[::1]:4096/x"
    );
}

#[test]
fn slice50_14_garbage_falls_back() {
    assert_eq!(
        describe_opencode_server_url("not a url"),
        "the configured OPENCODE_API_URL"
    );
}

#[test]
fn slice50_15_empty_falls_back() {
    assert_eq!(
        describe_opencode_server_url(""),
        "the configured OPENCODE_API_URL"
    );
}

#[test]
fn slice50_16_whitespace_in_authority_falls_back() {
    assert_eq!(
        describe_opencode_server_url("http://exa mple.com:4096/x"),
        "the configured OPENCODE_API_URL"
    );
}

// --- switch / active selectors (17-24) ---

#[test]
fn slice50_17_null_server_selects_none() {
    assert_eq!(select_switched_server("http://localhost:4096", None), None);
}

#[test]
fn slice50_18_same_url_selects_none_even_with_credentials() {
    let server = OpencodeServerTarget {
        url: "http://localhost:4096".to_string(),
        username: Some("user".to_string()),
        password: Some("pass".to_string()),
    };
    assert_eq!(
        select_switched_server("http://localhost:4096", Some(server)),
        None
    );
}

#[test]
fn slice50_19_different_url_is_selected() {
    let server = OpencodeServerTarget {
        url: "http://other:5000".to_string(),
        username: None,
        password: None,
    };
    assert_eq!(
        select_switched_server("http://localhost:4096", Some(server.clone())),
        Some(server)
    );
}

#[test]
fn slice50_20_selected_server_keeps_credentials() {
    let server = OpencodeServerTarget {
        url: "http://other:5000".to_string(),
        username: Some("u".to_string()),
        password: Some("p".to_string()),
    };
    let selected = select_switched_server("http://localhost:4096", Some(server));
    let selected = selected.expect("different server is selected");
    assert_eq!(selected.username.as_deref(), Some("u"));
    assert_eq!(selected.password.as_deref(), Some("p"));
}

#[test]
fn slice50_21_active_falls_back_to_configured() {
    assert_eq!(
        active_server_url("http://localhost:4096", None),
        "http://localhost:4096"
    );
}

#[test]
fn slice50_22_active_prefers_switched() {
    assert_eq!(
        active_server_url("http://localhost:4096", Some("http://other:5000")),
        "http://other:5000"
    );
}

#[test]
fn slice50_23_no_switch_means_on_configured() {
    assert!(is_on_configured_server(None));
}

#[test]
fn slice50_24_switch_means_off_configured() {
    assert!(!is_on_configured_server(Some("http://other:5000")));
}

// --- get_url_port (25-36) ---

#[test]
fn slice50_25_explicit_port() {
    assert_eq!(get_url_port("http://example.com:5000"), Some(5000));
}

#[test]
fn slice50_26_missing_port_is_none() {
    assert_eq!(get_url_port("http://example.com"), None);
}

#[test]
fn slice50_27_http_default_port_is_none() {
    assert_eq!(get_url_port("http://example.com:80"), None);
}

#[test]
fn slice50_28_https_default_port_with_path_is_none() {
    assert_eq!(get_url_port("https://example.com:443/x"), None);
}

#[test]
fn slice50_29_explicit_https_port() {
    assert_eq!(get_url_port("https://example.com:8443"), Some(8443));
}

#[test]
fn slice50_30_port_with_path_and_query() {
    assert_eq!(
        get_url_port("http://localhost:4096/some/path?x=1"),
        Some(4096)
    );
}

#[test]
fn slice50_31_uppercase_url_port() {
    assert_eq!(get_url_port("HTTP://EXAMPLE.COM:5000"), Some(5000));
}

#[test]
fn slice50_32_ipv6_port() {
    assert_eq!(get_url_port("http://[::1]:4096/"), Some(4096));
}

#[test]
fn slice50_33_garbage_is_none() {
    assert_eq!(get_url_port("not a url"), None);
}

#[test]
fn slice50_34_empty_is_none() {
    assert_eq!(get_url_port(""), None);
}

#[test]
fn slice50_35_non_numeric_port_is_none() {
    assert_eq!(get_url_port("http://example.com:abc"), None);
}

#[test]
fn slice50_36_out_of_range_port_is_none() {
    assert_eq!(get_url_port("http://example.com:99999"), None);
}

// --- reportFailedHealthCause keys / messages (37-42) ---

#[test]
fn slice50_37_unauthorized_key() {
    assert_eq!(unauthorized_health_report_key(), "unauthorized");
}

#[test]
fn slice50_38_unauthorized_message() {
    assert_eq!(
        format_unauthorized_health_message("http://localhost:4096"),
        "[OpenCode] Authentication failed at http://localhost:4096: \
         check OPENCODE_SERVER_USERNAME and OPENCODE_SERVER_PASSWORD or OPENCODE_PASSWORD"
    );
}

#[test]
fn slice50_39_mismatch_key_with_version_actual() {
    assert_eq!(
        mismatch_health_report_key("OpenCode 1.9.0"),
        "mismatch:OpenCode 1.9.0"
    );
}

#[test]
fn slice50_40_mismatch_key_with_unknown_version_actual() {
    assert_eq!(
        mismatch_health_report_key("an OpenCode server"),
        "mismatch:an OpenCode server"
    );
}

#[test]
fn slice50_41_mismatch_message_configured_v2() {
    assert_eq!(
        format_mismatch_health_message("v2", "http://localhost:4096", "OpenCode 1.9.0", "v1"),
        "[OpenCode] Server version mismatch: OPENCODE_SERVER_VERSION=v2, \
         but the server at http://localhost:4096 is OpenCode 1.9.0 (API v1). \
         Set OPENCODE_SERVER_VERSION=v1 and restart the bot, or run an \
         OpenCode v2 server at this address."
    );
}

#[test]
fn slice50_42_mismatch_message_configured_v1() {
    assert_eq!(
        format_mismatch_health_message("v1", "https://srv:8443/x", "an OpenCode server", "v2"),
        "[OpenCode] Server version mismatch: OPENCODE_SERVER_VERSION=v1, \
         but the server at https://srv:8443/x is an OpenCode server (API v2). \
         Set OPENCODE_SERVER_VERSION=v2 and restart the bot, or run an \
         OpenCode v1 server at this address."
    );
}

// --- registered-server / executable-mismatch reports (43-48) ---

#[test]
fn slice50_43_registered_server_key() {
    assert_eq!(
        registered_server_report_key("https://reg.example.com:4100"),
        "registered:https://reg.example.com:4100"
    );
}

#[test]
fn slice50_44_registered_rejected_credentials_message() {
    assert_eq!(
        format_registered_server_message(
            "http://127.0.0.1:4100",
            RegisteredServerProblem::RejectedCredentials
        ),
        "[OpenCode] Not connecting to the registered OpenCode v2 background \
         server at http://127.0.0.1:4100: it rejected the credentials from \
         the OpenCode registry. Not starting another local server either, \
         since that would replace it; stop that server (opencode service \
         stop) or fix it."
    );
}

#[test]
fn slice50_45_registered_not_answering_message() {
    assert_eq!(
        format_registered_server_message(
            "http://127.0.0.1:4100",
            RegisteredServerProblem::NotAnswering
        ),
        "[OpenCode] Not connecting to the registered OpenCode v2 background \
         server at http://127.0.0.1:4100: it did not answer as an OpenCode \
         v2 server. Not starting another local server either, since that \
         would replace it; stop that server (opencode service stop) or fix it."
    );
}

#[test]
fn slice50_46_registered_problem_texts() {
    assert_eq!(
        registered_server_problem_text(RegisteredServerProblem::RejectedCredentials),
        "rejected the credentials from the OpenCode registry"
    );
    assert_eq!(
        registered_server_problem_text(RegisteredServerProblem::NotAnswering),
        "did not answer as an OpenCode v2 server"
    );
}

#[test]
fn slice50_47_executable_mismatch_key() {
    assert_eq!(executable_mismatch_report_key("2.0.1"), "executable:2.0.1");
}

#[test]
fn slice50_48_executable_mismatch_message() {
    assert_eq!(
        format_executable_mismatch_message("v1", "2.0.1", "v2"),
        "[OpenCode] Server version mismatch: OPENCODE_SERVER_VERSION=v1, \
         but the local opencode executable is OpenCode 2.0.1 (API v2). Set \
         OPENCODE_SERVER_VERSION=v2 and restart the bot, or make an \
         OpenCode v1 opencode executable the first one on PATH."
    );
}

// --- boot-time version negotiation (V2-default cutover) ---

#[test]
fn slice50_49_negotiate_explicit_pins_skip_fallback() {
    // Explicit pins probe only that version: probes are ignored.
    assert_eq!(negotiate_server_version(Some("v1"), true, true), "v1");
    assert_eq!(negotiate_server_version(Some("v1"), false, false), "v1");
    assert_eq!(negotiate_server_version(Some(" 1 "), false, true), "v1");
    assert_eq!(negotiate_server_version(Some("v2"), false, true), "v2");
    assert_eq!(negotiate_server_version(Some("2"), true, false), "v2");
}

#[test]
fn slice50_50_negotiate_auto_probes_v2_first() {
    // Auto (unset/unknown): V2 `api/info` wins when both answer.
    assert_eq!(negotiate_server_version(None, true, true), "v2");
    assert_eq!(negotiate_server_version(Some(""), true, true), "v2");
    assert_eq!(negotiate_server_version(Some("banana"), true, true), "v2");
}

#[test]
fn slice50_51_negotiate_auto_falls_back_to_v1_then_v2_default() {
    // V2 silent but V1 `global/health` healthy -> adopt V1 for boot.
    assert_eq!(negotiate_server_version(None, false, true), "v1");
    assert_eq!(negotiate_server_version(Some("banana"), false, true), "v1");
    // Neither answers -> V2 default.
    assert_eq!(negotiate_server_version(None, false, false), "v2");
    assert_eq!(negotiate_server_version(Some(""), false, false), "v2");
}
