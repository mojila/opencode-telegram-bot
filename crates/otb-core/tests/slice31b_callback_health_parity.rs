//! Parity port of callback / health / retry pure gates (slice 31b, RED).
//!
//! Mirrors:
//! - `tests/bot/callbacks/callback-router.test.ts` (12 its: prefix dispatch,
//!   unknown/colonless callbacks, scope on throw)
//! - `tests/i18n/callback-answer-length.test.ts` (200-char answer budget)
//! - `tests/bot/callbacks/permission-callback-handler.test.ts`
//!   (reply validity, request-gone classification, action split)
//! - `tests/bot/callbacks/question-callback-handler.test.ts`
//!   (action/index parse, last-question gate)
//! - `tests/bot/handlers/agent.test.ts` + `variant.test.ts` + `model.test.ts`
//!   (`prefix:` name-strip subset)
//! - `tests/opencode/server-health.test.ts` (9 its: once/always dedupe,
//!   unauthorized/mismatch/unknown causes)
//! - `tests/utils/telegram-rate-limit-retry.test.ts` (13 its: 429 delay,
//!   transient codes, exponential backoff + 8000 cap, unsent codes)
//! - `tests/utils/opencode-error.test.ts` (unavailable markers, not-found
//!   name, extract fallback chain)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::callback_health_gates::*;

// --- callback router (6) ---

#[test]
fn slice31b_01_parses_prefix_before_first_colon() {
    assert_eq!(
        parse_callback_prefix("agent:subagent"),
        Some("agent".to_string())
    );
    assert_eq!(
        parse_callback_prefix("model:anthropic:claude"),
        Some("model".to_string())
    );
}

#[test]
fn slice31b_02_rejects_data_without_separator() {
    assert_eq!(parse_callback_prefix("noseparator"), None);
}

#[test]
fn slice31b_03_rejects_leading_separator() {
    assert_eq!(parse_callback_prefix(":action"), None);
    assert_eq!(parse_callback_prefix(""), None);
}

#[test]
fn slice31b_04_takes_text_before_first_of_many_colons() {
    assert_eq!(
        parse_callback_prefix("question:select:0:1"),
        Some("question".to_string())
    );
}

#[test]
fn slice31b_05_knows_registered_prefixes() {
    for prefix in [
        "agent",
        "permission",
        "question",
        "model",
        "variant",
        "settings",
        "task",
    ] {
        assert!(is_known_callback_prefix(prefix), "prefix={prefix}");
    }
    assert!(!is_known_callback_prefix("nonexistent"));
}

#[test]
fn slice31b_06_routes_prefixes_to_error_scopes() {
    assert_eq!(route_error_scope("permission"), Some("permission"));
    assert_eq!(route_error_scope("question"), Some("question"));
    assert_eq!(route_error_scope("task"), Some("taskCreation"));
    assert_eq!(route_error_scope("settings"), Some("none"));
    assert_eq!(route_error_scope("agent"), Some("interaction"));
    assert_eq!(route_error_scope("nonexistent"), None);
}

// --- feedback (3) ---

#[test]
fn slice31b_07_short_answer_passes_through() {
    assert_eq!(truncate_callback_answer("ok"), "ok");
}

#[test]
fn slice31b_08_answer_at_budget_passes_through() {
    let text = "x".repeat(CALLBACK_ANSWER_MAX_LENGTH);
    assert_eq!(truncate_callback_answer(&text), text);
}

#[test]
fn slice31b_09_long_answer_is_cut_to_budget() {
    let text = "y".repeat(CALLBACK_ANSWER_MAX_LENGTH + 50);
    let cut = truncate_callback_answer(&text);
    assert_eq!(cut.chars().count(), CALLBACK_ANSWER_MAX_LENGTH);
    assert_eq!(cut, "y".repeat(CALLBACK_ANSWER_MAX_LENGTH));
}

// --- permission reply validity (3) ---

#[test]
fn slice31b_10_accepts_once_always_reject() {
    assert!(is_permission_reply("once"));
    assert!(is_permission_reply("always"));
    assert!(is_permission_reply("reject"));
}

#[test]
fn slice31b_11_rejects_other_replies() {
    assert!(!is_permission_reply("allow"));
    assert!(!is_permission_reply("deny"));
    assert!(!is_permission_reply(""));
}

#[test]
fn slice31b_12_reply_match_is_case_sensitive() {
    assert!(!is_permission_reply("Once"));
    assert!(!is_permission_reply("REJECT"));
}

// --- permission request-gone classification (4) ---

#[test]
fn slice31b_13_tag_marks_gone_request() {
    let err = PermissionErrorShape {
        tag: Some("PermissionNotFoundError".to_string()),
        ..Default::default()
    };
    assert!(is_permission_request_not_found(&err));
}

#[test]
fn slice31b_14_name_marks_gone_request() {
    let err = PermissionErrorShape {
        name: Some("NotFoundError".to_string()),
        ..Default::default()
    };
    assert!(is_permission_request_not_found(&err));
}

#[test]
fn slice31b_15_message_marks_gone_request_case_insensitively() {
    let err = PermissionErrorShape {
        message: Some("Permission Request Not Found".to_string()),
        ..Default::default()
    };
    assert!(is_permission_request_not_found(&err));
}

#[test]
fn slice31b_16_data_message_marks_gone_request() {
    let err = PermissionErrorShape {
        data_message: Some("permission request not found".to_string()),
        ..Default::default()
    };
    assert!(is_permission_request_not_found(&err));
    let other = PermissionErrorShape {
        message: Some("boom".to_string()),
        ..Default::default()
    };
    assert!(!is_permission_request_not_found(&other));
}

// --- permission action split (2) ---

#[test]
fn slice31b_17_parses_permission_action() {
    assert_eq!(
        parse_permission_callback_action("permission:once:req-1"),
        Some("once".to_string())
    );
    assert_eq!(
        parse_permission_callback_action("permission:reject:req-1"),
        Some("reject".to_string())
    );
}

#[test]
fn slice31b_18_rejects_wrong_prefix_or_missing_action() {
    assert_eq!(parse_permission_callback_action("model:once:req-1"), None);
    assert_eq!(parse_permission_callback_action("permission"), None);
}

// --- question callback parse (6) ---

#[test]
fn slice31b_19_parses_select_with_option() {
    assert_eq!(
        parse_question_callback("question:select:0:1"),
        Some(ParsedQuestionCallback {
            action: QuestionCallbackAction::Select,
            question_index: 0,
            option_index: Some(1)
        })
    );
}

#[test]
fn slice31b_20_parses_submit() {
    assert_eq!(
        parse_question_callback("question:submit:2"),
        Some(ParsedQuestionCallback {
            action: QuestionCallbackAction::Submit,
            question_index: 2,
            option_index: None
        })
    );
}

#[test]
fn slice31b_21_parses_custom() {
    assert_eq!(
        parse_question_callback("question:custom:1"),
        Some(ParsedQuestionCallback {
            action: QuestionCallbackAction::Custom,
            question_index: 1,
            option_index: None
        })
    );
}

#[test]
fn slice31b_22_parses_toggle_custom() {
    assert_eq!(
        parse_question_callback("question:toggle_custom:0"),
        Some(ParsedQuestionCallback {
            action: QuestionCallbackAction::ToggleCustom,
            question_index: 0,
            option_index: None
        })
    );
}

#[test]
fn slice31b_23_parses_cancel() {
    assert_eq!(
        parse_question_callback("question:cancel:0"),
        Some(ParsedQuestionCallback {
            action: QuestionCallbackAction::Cancel,
            question_index: 0,
            option_index: None
        })
    );
}

#[test]
fn slice31b_24_rejects_unknown_action_prefix_or_index() {
    assert_eq!(parse_question_callback("question:explode:0"), None);
    assert_eq!(parse_question_callback("permission:select:0:1"), None);
    assert_eq!(parse_question_callback("question:select"), None);
}

// --- last question gate (2) ---

#[test]
fn slice31b_25_detects_last_question() {
    assert!(is_last_question(2, 3));
    assert!(!is_last_question(0, 3));
}

#[test]
fn slice31b_26_single_question_poll_is_last() {
    assert!(is_last_question(0, 1));
}

// --- prefixed selection callbacks (2) ---

#[test]
fn slice31b_27_strips_prefix_name() {
    assert_eq!(
        parse_prefixed_callback_name("agent", "agent:build"),
        Some("build".to_string())
    );
    assert_eq!(
        parse_prefixed_callback_name("variant", "variant:concise"),
        Some("concise".to_string())
    );
}

#[test]
fn slice31b_28_rejects_prefix_mismatch() {
    assert_eq!(parse_prefixed_callback_name("agent", "model:build"), None);
    assert_eq!(parse_prefixed_callback_name("agent", "agent"), None);
}

// --- problem-report dedupe (4) ---

#[test]
fn slice31b_29_always_mode_reports_every_time() {
    let mut dedupe = ProblemDedupe::new();
    assert!(dedupe.should_report("mismatch:x", ProblemReportMode::Always));
    assert!(dedupe.should_report("mismatch:x", ProblemReportMode::Always));
}

#[test]
fn slice31b_30_once_mode_reports_first_problem() {
    let mut dedupe = ProblemDedupe::new();
    assert!(dedupe.should_report("unauthorized", ProblemReportMode::Once));
}

#[test]
fn slice31b_31_once_mode_suppresses_repeat_key() {
    let mut dedupe = ProblemDedupe::new();
    assert!(dedupe.should_report("mismatch:x", ProblemReportMode::Once));
    assert!(!dedupe.should_report("mismatch:x", ProblemReportMode::Once));
}

#[test]
fn slice31b_32_once_mode_reports_changed_key() {
    let mut dedupe = ProblemDedupe::new();
    assert!(dedupe.should_report("mismatch:x", ProblemReportMode::Once));
    assert!(dedupe.should_report("unauthorized", ProblemReportMode::Once));
}

// --- failed-health cause classification (4) ---

#[test]
fn slice31b_33_configured_unauthorized_wins() {
    assert_eq!(
        classify_health_cause(true, ProbeKind::Found, "1.0.0", "v2"),
        FailedHealthCause::Unauthorized
    );
}

#[test]
fn slice31b_34_other_version_found_is_mismatch() {
    assert_eq!(
        classify_health_cause(false, ProbeKind::Found, "1.2.3", "v2"),
        FailedHealthCause::Mismatch {
            actual: "OpenCode 1.2.3".to_string(),
            other_version: "v2".to_string()
        }
    );
}

#[test]
fn slice31b_35_locked_other_version_is_mismatch_without_exact_version() {
    assert_eq!(
        classify_health_cause(false, ProbeKind::Unauthorized, "", "v1"),
        FailedHealthCause::Mismatch {
            actual: "an OpenCode server".to_string(),
            other_version: "v1".to_string()
        }
    );
}

#[test]
fn slice31b_36_silent_server_is_unknown() {
    assert_eq!(
        classify_health_cause(false, ProbeKind::Other, "", "v2"),
        FailedHealthCause::Unknown
    );
}

// --- telegram retry (7) ---

#[test]
fn slice31b_37_detects_transient_server_codes() {
    for code in [500, 502, 503, 504] {
        assert!(is_transient_server_error_code(code), "code={code}");
    }
    assert!(!is_transient_server_error_code(400));
    assert!(!is_transient_server_error_code(429));
}

#[test]
fn slice31b_38_detects_rate_limit_status() {
    assert!(is_rate_limit_status(429));
    assert!(!is_rate_limit_status(502));
}

#[test]
fn slice31b_39_backs_off_exponentially() {
    assert_eq!(server_error_backoff_ms(0, 500), 500);
    assert_eq!(server_error_backoff_ms(1, 500), 1000);
    assert_eq!(server_error_backoff_ms(2, 500), 2000);
}

#[test]
fn slice31b_40_caps_backoff_at_8000() {
    assert_eq!(
        server_error_backoff_ms(20, 1000),
        MAX_SERVER_ERROR_BACKOFF_MS
    );
    assert_eq!(MAX_SERVER_ERROR_BACKOFF_MS, 8_000);
}

#[test]
fn slice31b_41_detects_unsent_network_codes() {
    for code in [
        "ECONNREFUSED",
        "ENOTFOUND",
        "EAI_AGAIN",
        "ENETUNREACH",
        "EHOSTUNREACH",
    ] {
        assert!(is_unsent_network_code(code), "code={code}");
    }
    assert!(!is_unsent_network_code("ECONNRESET"));
}

#[test]
fn slice31b_42_detects_transient_network_codes() {
    for code in ["ECONNRESET", "EPIPE", "ETIMEDOUT", "ECONNREFUSED"] {
        assert!(is_transient_network_code(code), "code={code}");
    }
    assert!(!is_transient_network_code("EACCES"));
}

#[test]
fn slice31b_43_resolves_retry_after_or_fallback() {
    assert_eq!(rate_limit_retry_after_ms(Some(3), 1000), 3000);
    assert_eq!(rate_limit_retry_after_ms(None, 1000), 1000);
}

// --- opencode errors (5) ---

#[test]
fn slice31b_44_detects_expected_unavailable_messages() {
    assert!(is_expected_unavailable_message("fetch failed"));
    assert!(is_expected_unavailable_message(
        "request failed: econnrefused"
    ));
    assert!(is_expected_unavailable_message("connect ECONNRESET"));
    assert!(!is_expected_unavailable_message("timeout"));
}

#[test]
fn slice31b_45_detects_not_found_name() {
    assert!(is_not_found_error_name("NotFoundError"));
    assert!(!is_not_found_error_name("UnknownError"));
}

#[test]
fn slice31b_46_detects_server_unavailable_markers() {
    assert!(has_server_unavailable_marker("fetch failed"));
    assert!(has_server_unavailable_marker("connection refused"));
    assert!(!has_server_unavailable_marker("timeout"));
}

#[test]
fn slice31b_47_extracts_message_with_fallback_chain() {
    let direct = ErrorMessageShape {
        message: Some("boom".to_string()),
        ..Default::default()
    };
    assert_eq!(extract_error_message(&direct), Some("boom".to_string()));
    let via_data = ErrorMessageShape {
        message: Some("   ".to_string()),
        data_message: Some("Model not found.".to_string()),
        ..Default::default()
    };
    assert_eq!(
        extract_error_message(&via_data),
        Some("Model not found.".to_string())
    );
    let via_name = ErrorMessageShape {
        name: Some("UnknownError".to_string()),
        ..Default::default()
    };
    assert_eq!(
        extract_error_message(&via_name),
        Some("UnknownError".to_string())
    );
    assert_eq!(extract_error_message(&ErrorMessageShape::default()), None);
}

#[test]
fn slice31b_48_extract_trims_whitespace() {
    let shape = ErrorMessageShape {
        message: Some("  oops  ".to_string()),
        ..Default::default()
    };
    assert_eq!(extract_error_message(&shape), Some("oops".to_string()));
}
