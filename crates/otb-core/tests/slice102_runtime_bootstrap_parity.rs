//! Slice 102 parity: runtime bootstrap env-file gates (RED).
//!
//! Mirrors the pure TS contract in `src/runtime/bootstrap.ts`:
//! `DEFAULT_API_URLS`, `DEFAULT_SERVER_USERNAME`, fallback model defaults,
//! `isPositiveInteger`, `isValidHttpUrl`, `validateRuntimeEnvValues`,
//! `normalizeEnvLineEndings`, `removeEnvKey`, `parseEnvAssignmentLine`,
//! `buildTemplateKeySet`, `collectActiveEnvAssignments`,
//! `collectCustomEnvAssignments`, `renderEnvAssignment`,
//! `finalizeEnvContent`, `buildFlatEnvFileContent`, `buildEnvFileContent`,
//! `getWizardServerVersionDefault` and `pickServerPassword`.
//!
//! Already covered elsewhere and NOT re-tested here:
//! CLI args / runtime mode / Telegram client wiring (`bot_bootstrap_gates`)
//! and health-port / node-version / service-manager gates
//! (`runtime_service_gates`).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `runtime_bootstrap_gates` is ported (only the constant-value tests pass).

use otb_core::runtime_bootstrap_gates::*;

// ---- constants (5) ----

#[test]
fn default_api_url_v1_value_parity() {
    assert_eq!(DEFAULT_API_URL_V1, "http://localhost:4096");
}

#[test]
fn default_api_url_v2_value_parity() {
    assert_eq!(DEFAULT_API_URL_V2, "http://127.0.0.1:49374");
}

#[test]
fn default_server_username_value_parity() {
    assert_eq!(DEFAULT_SERVER_USERNAME, "opencode");
}

#[test]
fn fallback_model_provider_value_parity() {
    assert_eq!(FALLBACK_MODEL_PROVIDER, "opencode");
}

#[test]
fn fallback_model_id_value_parity() {
    assert_eq!(FALLBACK_MODEL_ID, "big-pickle");
}

// ---- isPositiveInteger (3) ----

#[test]
fn positive_integer_accepts_plain_digits_parity() {
    assert!(is_positive_integer("1"));
    assert!(is_positive_integer("123"));
}

#[test]
fn positive_integer_rejects_zero_and_leading_zero_parity() {
    assert!(!is_positive_integer("0"));
    assert!(!is_positive_integer("01"));
    assert!(!is_positive_integer("007"));
}

#[test]
fn positive_integer_rejects_empty_and_non_digits_parity() {
    assert!(!is_positive_integer(""));
    assert!(!is_positive_integer("12a"));
    assert!(!is_positive_integer(" 123"));
    assert!(!is_positive_integer("-5"));
}

// ---- isValidHttpUrl (4) ----

#[test]
fn http_url_accepts_http_parity() {
    assert!(is_valid_http_url("http://localhost:4096"));
}

#[test]
fn http_url_accepts_https_parity() {
    assert!(is_valid_http_url("https://example.com/api"));
}

#[test]
fn http_url_rejects_wrong_scheme_parity() {
    assert!(!is_valid_http_url("ftp://example.com/x"));
    assert!(!is_valid_http_url("ws://example.com/x"));
}

#[test]
fn http_url_rejects_bare_and_empty_parity() {
    assert!(!is_valid_http_url("not-a-url"));
    assert!(!is_valid_http_url(""));
    assert!(!is_valid_http_url("localhost:4096"));
}

// ---- validateRuntimeEnvValues (6) ----

#[test]
fn valid_env_passes_parity() {
    let result = validate_runtime_env(
        Some("tok:123"),
        Some("42"),
        Some("opencode"),
        Some("big-pickle"),
        None,
    );
    assert!(result.is_valid);
    assert_eq!(result.reason, None);
    let with_url = validate_runtime_env(
        Some("tok:123"),
        Some("42"),
        Some("opencode"),
        Some("big-pickle"),
        Some("http://localhost:4096"),
    );
    assert!(with_url.is_valid);
}

#[test]
fn missing_or_blank_token_fails_first_parity() {
    assert_eq!(
        validate_runtime_env(None, Some("42"), Some("p"), Some("m"), None).reason.as_deref(),
        Some("Missing TELEGRAM_BOT_TOKEN")
    );
    assert_eq!(
        validate_runtime_env(Some("   "), Some("42"), Some("p"), Some("m"), None)
            .reason
            .as_deref(),
        Some("Missing TELEGRAM_BOT_TOKEN")
    );
}

#[test]
fn invalid_user_id_fails_parity() {
    for bad in ["", "0", "01", "abc", "-7", "12a"] {
        let result = validate_runtime_env(Some("tok:1"), Some(bad), Some("p"), Some("m"), None);
        assert!(!result.is_valid, "user id {bad:?} should fail");
        assert_eq!(result.reason.as_deref(), Some("Invalid TELEGRAM_ALLOWED_USER_ID"));
    }
}

#[test]
fn missing_provider_or_model_fails_parity() {
    let no_provider = validate_runtime_env(Some("tok:1"), Some("5"), None, Some("m"), None);
    assert_eq!(no_provider.reason.as_deref(), Some("Missing OPENCODE_MODEL_PROVIDER"));
    let blank_provider = validate_runtime_env(Some("tok:1"), Some("5"), Some("  "), Some("m"), None);
    assert_eq!(
        blank_provider.reason.as_deref(),
        Some("Missing OPENCODE_MODEL_PROVIDER")
    );
    let no_model = validate_runtime_env(Some("tok:1"), Some("5"), Some("p"), None, None);
    assert_eq!(no_model.reason.as_deref(), Some("Missing OPENCODE_MODEL_ID"));
}

#[test]
fn invalid_api_url_fails_parity() {
    let result = validate_runtime_env(
        Some("tok:1"),
        Some("5"),
        Some("p"),
        Some("m"),
        Some("not-a-url"),
    );
    assert!(!result.is_valid);
    assert_eq!(result.reason.as_deref(), Some("Invalid OPENCODE_API_URL"));
}

#[test]
fn blank_api_url_is_allowed_parity() {
    let blank = validate_runtime_env(Some("tok:1"), Some("5"), Some("p"), Some("m"), Some("   "));
    assert!(blank.is_valid);
    assert_eq!(blank.reason, None);
}

// ---- normalizeEnvLineEndings (2) ----

#[test]
fn env_endings_split_crlf_parity() {
    assert_eq!(
        normalize_env_line_endings("A=1\r\nB=2\r\n"),
        vec!["A=1".to_string(), "B=2".to_string()]
    );
}

#[test]
fn env_endings_drop_trailing_blanks_but_keep_interior_parity() {
    assert_eq!(
        normalize_env_line_endings("A=1\n\nB=2\n\n"),
        vec!["A=1".to_string(), "".to_string(), "B=2".to_string()]
    );
}

// ---- removeEnvKey (2) ----

#[test]
fn remove_env_key_plain_and_export_parity() {
    let lines = vec![
        "A=1".to_string(),
        "export B=2".to_string(),
        "  B =3".to_string(),
        "C=4".to_string(),
    ];
    assert_eq!(
        remove_env_key(&lines, "B"),
        vec!["A=1".to_string(), "C=4".to_string()]
    );
}

#[test]
fn remove_env_key_keeps_similar_prefix_parity() {
    let lines = vec!["FOO=1".to_string(), "FOOBAR=2".to_string()];
    assert_eq!(remove_env_key(&lines, "FOO"), vec!["FOOBAR=2".to_string()]);
}

// ---- parseEnvAssignmentLine (4) ----

#[test]
fn parse_simple_assignment_parity() {
    assert_eq!(
        parse_env_assignment_line("FOO=bar"),
        Some(ParsedEnvAssignment {
            key: "FOO".to_string(),
            raw_value: "bar".to_string(),
            is_commented: false,
        })
    );
}

#[test]
fn parse_export_assignment_parity() {
    let parsed = parse_env_assignment_line("export FOO = bar=baz").expect("parses");
    assert_eq!(parsed.key, "FOO");
    assert_eq!(parsed.raw_value, " bar=baz");
    assert!(!parsed.is_commented);
}

#[test]
fn parse_commented_assignment_marks_commented_parity() {
    let parsed = parse_env_assignment_line("# FOO=bar").expect("parses");
    assert_eq!(parsed.key, "FOO");
    assert_eq!(parsed.raw_value, "bar");
    assert!(parsed.is_commented);
}

#[test]
fn parse_non_assignment_is_none_parity() {
    assert_eq!(parse_env_assignment_line("just words"), None);
    assert_eq!(parse_env_assignment_line(""), None);
    assert_eq!(parse_env_assignment_line("# no equals here"), None);
}

// ---- template / active / custom (5) ----

#[test]
fn template_keys_include_commented_parity() {
    let keys = build_template_key_set("# FOO=1\nBAR=2\n");
    assert!(keys.contains(&"FOO".to_string()));
    assert!(keys.contains(&"BAR".to_string()));
}

#[test]
fn active_assignments_last_wins_and_skip_commented_parity() {
    let found = collect_active_env_assignments("A=1\n# A=2\nA=3\nB=x\n");
    assert!(found.contains(&("A".to_string(), "3".to_string())));
    assert!(found.contains(&("B".to_string(), "x".to_string())));
    assert_eq!(found.len(), 2);
}

#[test]
fn custom_assignments_keep_non_template_order_parity() {
    let custom = collect_custom_env_assignments(
        "TEMPLATE_K=1\nCUSTOM_B=2\nCUSTOM_A=3\n",
        &["TEMPLATE_K".to_string()],
    );
    assert_eq!(custom, vec!["CUSTOM_B=2".to_string(), "CUSTOM_A=3".to_string()]);
}

#[test]
fn custom_assignments_dedupe_to_last_in_order_parity() {
    let custom =
        collect_custom_env_assignments("Z=1\nZ=2\nT=0\n", &["T".to_string()]);
    assert_eq!(custom, vec!["Z=2".to_string()]);
}

#[test]
fn custom_assignments_empty_when_all_template_parity() {
    let custom =
        collect_custom_env_assignments("A=1\nB=2\n", &["A".to_string(), "B".to_string()]);
    assert!(custom.is_empty());
}

// ---- render / finalize (3) ----

#[test]
fn render_assignment_joins_equals_parity() {
    assert_eq!(render_env_assignment("FOO", "bar"), "FOO=bar");
    assert_eq!(render_env_assignment("FOO", " a=b"), "FOO= a=b");
}

#[test]
fn finalize_joins_with_trailing_newline_parity() {
    assert_eq!(
        finalize_env_content(&["A=1".to_string(), "B=2".to_string()]),
        "A=1\nB=2\n"
    );
}

#[test]
fn finalize_empty_is_single_newline_parity() {
    assert_eq!(finalize_env_content(&[]), "\n");
}

// ---- buildFlatEnvFileContent (3) ----

fn sample_wizard_values() -> WizardEnvValues {
    WizardEnvValues {
        locale: "en".to_string(),
        token: "tok:123".to_string(),
        allowed_user_id: "42".to_string(),
        server_version: ServerVersion::V2,
        api_url: None,
        server_username: "opencode".to_string(),
        server_password: None,
        model_provider: "opencode".to_string(),
        model_id: "big-pickle".to_string(),
    }
}

#[test]
fn flat_rewrites_keys_in_order_parity() {
    let content = build_flat_env_file_content("", &sample_wizard_values());
    let token_pos = content.find("TELEGRAM_BOT_TOKEN=tok:123").expect("token line");
    let user_pos = content.find("TELEGRAM_ALLOWED_USER_ID=42").expect("user line");
    let provider_pos = content.find("OPENCODE_MODEL_PROVIDER=opencode").expect("provider");
    assert!(token_pos < user_pos && user_pos < provider_pos);
    assert!(content.ends_with('\n'));
}

#[test]
fn flat_removes_old_key_before_append_parity() {
    let content = build_flat_env_file_content(
        "TELEGRAM_BOT_TOKEN=old\nKEEP=1\n",
        &sample_wizard_values(),
    );
    assert_eq!(content.matches("TELEGRAM_BOT_TOKEN=").count(), 1);
    assert!(content.contains("TELEGRAM_BOT_TOKEN=tok:123"));
    assert!(content.contains("KEEP=1"));
}

#[test]
fn flat_skips_blank_values_parity() {
    let mut values = sample_wizard_values();
    values.api_url = Some("   ".to_string());
    values.server_password = None;
    let content = build_flat_env_file_content("", &values);
    assert!(!content.contains("OPENCODE_API_URL="));
    assert!(!content.contains("OPENCODE_SERVER_PASSWORD="));
}

// ---- buildEnvFileContent (5) ----

#[test]
fn template_missing_falls_back_to_flat_parity() {
    let values = sample_wizard_values();
    assert_eq!(
        build_env_file_content("KEEP=1\n", &values, None),
        build_flat_env_file_content("KEEP=1\n", &values)
    );
    assert_eq!(
        build_env_file_content("KEEP=1\n", &values, Some("")),
        build_flat_env_file_content("KEEP=1\n", &values)
    );
}

#[test]
fn template_wizard_override_wins_parity() {
    let values = sample_wizard_values();
    let content = build_env_file_content(
        "",
        &values,
        Some("TELEGRAM_BOT_TOKEN=\nOTHER=keep\n"),
    );
    assert!(content.contains("TELEGRAM_BOT_TOKEN=tok:123"));
    assert!(content.contains("OTHER=keep"));
}

#[test]
fn template_blank_override_keeps_template_line_parity() {
    let mut values = sample_wizard_values();
    values.api_url = Some("".to_string());
    let template = "OPENCODE_API_URL=http://example.com\n";
    let content = build_env_file_content("", &values, Some(template));
    assert!(content.contains("OPENCODE_API_URL=http://example.com"));
}

#[test]
fn template_existing_value_fills_non_wizard_key_parity() {
    let values = sample_wizard_values();
    let content = build_env_file_content(
        "CUSTOM_EXISTING=yes\n",
        &values,
        Some("CUSTOM_EXISTING=\n"),
    );
    assert!(content.contains("CUSTOM_EXISTING=yes"));
}

#[test]
fn template_appends_custom_after_blank_separator_parity() {
    let values = sample_wizard_values();
    let content = build_env_file_content(
        "MY_CUSTOM=1\n",
        &values,
        Some("TELEGRAM_BOT_TOKEN=\n"),
    );
    assert!(content.contains("TELEGRAM_BOT_TOKEN=tok:123"));
    assert!(content.contains("\n\nMY_CUSTOM=1\n"));
}

// ---- wizard default + password (4) ----

#[test]
fn wizard_default_first_setup_is_v2_parity() {
    assert_eq!(wizard_server_version_default(None), ServerVersion::V2);
}

#[test]
fn wizard_default_saved_choice_parity() {
    assert_eq!(
        wizard_server_version_default(Some("OPENCODE_SERVER_VERSION=v2\n")),
        ServerVersion::V2
    );
    assert_eq!(
        wizard_server_version_default(Some("OPENCODE_SERVER_VERSION= V2 \n")),
        ServerVersion::V2
    );
    assert_eq!(
        wizard_server_version_default(Some("OPENCODE_SERVER_VERSION=v1\n")),
        ServerVersion::V1
    );
    assert_eq!(wizard_server_version_default(Some("OTHER=1\n")), ServerVersion::V1);
}

#[test]
fn pick_password_typed_answer_wins_parity() {
    assert_eq!(
        pick_server_password("secret", Some("old")),
        Some("secret".to_string())
    );
    assert_eq!(pick_server_password("secret", None), Some("secret".to_string()));
}

#[test]
fn pick_password_empty_falls_back_parity() {
    assert_eq!(
        pick_server_password("", Some("old")),
        Some("old".to_string())
    );
    assert_eq!(pick_server_password("", None), None);
}
