//! Parity port of error classification / formatting pure gates (slice 41, RED).
//!
//! Mirrors representative cases from:
//! - `tests/utils/error-format.test.ts` (Error stack formatting, `{}` →
//!   `"unknown error"`, clipping with `...`)
//! - `tests/utils/opencode-error.test.ts` (`extractErrorMessage` precedence
//!   and fall-through, `isServerUnavailableError` markers plus `cause`
//!   walking)
//! - `src/utils/type-guards.ts` (`isRecord`)
//! - `src/utils/opencode-error.ts` (every marker list in
//!   `isExpectedOpencodeUnavailableError`; `isOpencodeNotFoundError` name
//!   match; every `isServerUnavailableError` branch: string / `Error`
//!   `name: message` / record `code` / `message` / nested `cause`)
//! - `src/utils/error-format.ts` (stack-vs-`name: message` fallback,
//!   string passthrough, JSON object rendering, trim, blank/`{}` →
//!   `"unknown error"`, `maxLength` clipping)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::error_gates::*;

fn instance(name: &str, message: &str) -> ErrorValue {
    ErrorValue::Instance(ErrorInstance {
        name: name.to_string(),
        message: message.to_string(),
        stack: None,
        cause: None,
    })
}

fn stacked(name: &str, message: &str, stack: &str) -> ErrorValue {
    ErrorValue::Instance(ErrorInstance {
        name: name.to_string(),
        message: message.to_string(),
        stack: Some(stack.to_string()),
        cause: None,
    })
}

fn caused(message: &str, cause: ErrorValue) -> ErrorValue {
    ErrorValue::Instance(ErrorInstance {
        name: "Error".to_string(),
        message: message.to_string(),
        stack: None,
        cause: Some(Box::new(cause)),
    })
}

fn record(r: ErrorRecord) -> ErrorValue {
    ErrorValue::Record(r)
}

// --- constants (2) ---

#[test]
fn slice41_01_default_max_error_details_length_is_1500() {
    assert_eq!(DEFAULT_MAX_ERROR_DETAILS_LENGTH, 1500);
}

#[test]
fn slice41_02_server_unavailable_markers() {
    assert_eq!(
        SERVER_UNAVAILABLE_ERROR_MARKERS,
        [
            "fetch failed",
            "econnrefused",
            "connection refused",
            "connect refused"
        ]
    );
}

// --- is_record_value (5) ---

#[test]
fn slice41_03_is_record_record_is_true() {
    assert!(is_record_value(&record(ErrorRecord::default())));
}

#[test]
fn slice41_04_is_record_error_instance_is_true() {
    assert!(is_record_value(&instance("Error", "boom")));
}

#[test]
fn slice41_05_is_record_text_is_false() {
    assert!(!is_record_value(&ErrorValue::Text("boom".to_string())));
}

#[test]
fn slice41_06_is_record_number_is_false() {
    assert!(!is_record_value(&ErrorValue::Number(42.0)));
}

#[test]
fn slice41_07_is_record_null_is_false() {
    assert!(!is_record_value(&ErrorValue::Null));
}

// --- is_expected_opencode_unavailable_error (11) ---

#[test]
fn slice41_08_expected_null_is_false() {
    assert!(!is_expected_opencode_unavailable_error(&ErrorValue::Null));
}

#[test]
fn slice41_09_expected_fetch_failed_is_true() {
    assert!(is_expected_opencode_unavailable_error(&instance(
        "Error",
        "fetch failed"
    )));
}

#[test]
fn slice41_10_expected_failed_to_fetch_is_true() {
    assert!(is_expected_opencode_unavailable_error(&instance(
        "TypeError",
        "failed to fetch the session"
    )));
}

#[test]
fn slice41_11_expected_econnrefused_matches_case_insensitively() {
    assert!(is_expected_opencode_unavailable_error(&instance(
        "Error",
        "connect ECONNREFUSED 127.0.0.1:4096"
    )));
}

#[test]
fn slice41_12_expected_econnreset_is_true() {
    assert!(is_expected_opencode_unavailable_error(&instance(
        "Error",
        "read ECONNRESET"
    )));
}

#[test]
fn slice41_13_expected_enotfound_is_true() {
    assert!(is_expected_opencode_unavailable_error(&instance(
        "Error",
        "getaddrinfo ENOTFOUND localhost"
    )));
}

#[test]
fn slice41_14_expected_connectex_is_true() {
    assert!(is_expected_opencode_unavailable_error(&instance(
        "Error",
        "connectex: connection timed out"
    )));
}

#[test]
fn slice41_15_expected_unrelated_timeout_is_false() {
    assert!(!is_expected_opencode_unavailable_error(&instance(
        "Error", "timeout"
    )));
}

#[test]
fn slice41_16_expected_plain_string_is_coerced() {
    assert!(is_expected_opencode_unavailable_error(&ErrorValue::Text(
        "fetch failed".to_string()
    )));
}

#[test]
fn slice41_17_expected_empty_text_is_false() {
    assert!(!is_expected_opencode_unavailable_error(&ErrorValue::Text(
        String::new()
    )));
}

#[test]
fn slice41_18_expected_number_is_false() {
    assert!(!is_expected_opencode_unavailable_error(
        &ErrorValue::Number(500.0)
    ));
}

// --- is_opencode_not_found_error (5) ---

#[test]
fn slice41_19_not_found_name_match_is_true() {
    assert!(is_opencode_not_found_error(&record(ErrorRecord {
        name: Some("NotFoundError".to_string()),
        ..Default::default()
    })));
}

#[test]
fn slice41_20_not_found_wrong_name_is_false() {
    assert!(!is_opencode_not_found_error(&record(ErrorRecord {
        name: Some("UnknownError".to_string()),
        ..Default::default()
    })));
}

#[test]
fn slice41_21_not_found_text_is_false() {
    assert!(!is_opencode_not_found_error(&ErrorValue::Text(
        "NotFoundError".to_string()
    )));
}

#[test]
fn slice41_22_not_found_null_is_false() {
    assert!(!is_opencode_not_found_error(&ErrorValue::Null));
}

#[test]
fn slice41_23_not_found_record_without_name_is_false() {
    assert!(!is_opencode_not_found_error(&record(ErrorRecord {
        message: Some("missing".to_string()),
        ..Default::default()
    })));
}

// --- is_server_unavailable_error (12) ---

#[test]
fn slice41_24_unavailable_error_message_marker_is_true() {
    assert!(is_server_unavailable_error(&instance(
        "Error",
        "fetch failed"
    )));
}

#[test]
fn slice41_25_unavailable_plain_string_marker_is_true() {
    assert!(is_server_unavailable_error(&ErrorValue::Text(
        "econnrefused".to_string()
    )));
}

#[test]
fn slice41_26_unavailable_walks_error_cause_chain() {
    let root = instance("Error", "connection refused");
    let wrapper = caused("wrapped", root);
    assert!(is_server_unavailable_error(&wrapper));
}

#[test]
fn slice41_27_unavailable_record_code_marker_is_true() {
    assert!(is_server_unavailable_error(&record(ErrorRecord {
        code: Some("ECONNREFUSED".to_string()),
        message: Some("boom".to_string()),
        ..Default::default()
    })));
}

#[test]
fn slice41_28_unavailable_record_message_marker_is_true() {
    assert!(is_server_unavailable_error(&record(ErrorRecord {
        message: Some("fetch failed".to_string()),
        ..Default::default()
    })));
}

#[test]
fn slice41_29_unavailable_walks_record_nested_cause() {
    assert!(is_server_unavailable_error(&record(ErrorRecord {
        message: Some("boom".to_string()),
        cause: Some(Box::new(record(ErrorRecord {
            code: Some("ECONNREFUSED".to_string()),
            ..Default::default()
        }))),
        ..Default::default()
    })));
}

#[test]
fn slice41_30_unavailable_unrelated_error_is_false() {
    assert!(!is_server_unavailable_error(&instance("Error", "timeout")));
}

#[test]
fn slice41_31_unavailable_null_is_false() {
    assert!(!is_server_unavailable_error(&ErrorValue::Null));
}

#[test]
fn slice41_32_unavailable_marker_matches_case_insensitively() {
    assert!(is_server_unavailable_error(&ErrorValue::Text(
        "FETCH FAILED".to_string()
    )));
}

#[test]
fn slice41_33_unavailable_connection_refused_is_true() {
    assert!(is_server_unavailable_error(&instance(
        "Error",
        "connection refused by peer"
    )));
}

#[test]
fn slice41_34_unavailable_connect_refused_is_true() {
    assert!(is_server_unavailable_error(&ErrorValue::Text(
        "connect refused".to_string()
    )));
}

#[test]
fn slice41_35_unavailable_empty_text_is_false() {
    assert!(!is_server_unavailable_error(&ErrorValue::Text(
        String::new()
    )));
}

// --- extract_error_message (8) ---

#[test]
fn slice41_36_extract_error_instance_message() {
    assert_eq!(
        extract_error_message(&instance("Error", "boom")),
        Some("boom".to_string())
    );
}

#[test]
fn slice41_37_extract_string_trimmed() {
    assert_eq!(
        extract_error_message(&ErrorValue::Text("  oops  ".to_string())),
        Some("oops".to_string())
    );
}

#[test]
fn slice41_38_extract_sdk_style_data_message() {
    assert_eq!(
        extract_error_message(&record(ErrorRecord {
            name: Some("UnknownError".to_string()),
            data_message: Some("Model not found.".to_string()),
            ..Default::default()
        })),
        Some("Model not found.".to_string())
    );
}

#[test]
fn slice41_39_extract_whitespace_data_message_falls_back_to_message() {
    assert_eq!(
        extract_error_message(&record(ErrorRecord {
            name: Some("UnknownError".to_string()),
            message: Some("fallback".to_string()),
            data_message: Some("   ".to_string()),
            ..Default::default()
        })),
        Some("fallback".to_string())
    );
}

#[test]
fn slice41_40_extract_falls_back_to_name() {
    assert_eq!(
        extract_error_message(&record(ErrorRecord {
            name: Some("UnknownError".to_string()),
            ..Default::default()
        })),
        Some("UnknownError".to_string())
    );
}

#[test]
fn slice41_41_extract_null_is_none() {
    assert_eq!(extract_error_message(&ErrorValue::Null), None);
}

#[test]
fn slice41_42_extract_number_is_none() {
    assert_eq!(extract_error_message(&ErrorValue::Number(42.0)), None);
}

#[test]
fn slice41_43_extract_empty_record_is_none() {
    assert_eq!(extract_error_message(&record(ErrorRecord::default())), None);
}

// --- format_error_details (7) ---

#[test]
fn slice41_44_format_error_uses_stack() {
    let details = format_error_details(&stacked("Error", "boom", "Error: boom\n    at here"), None);
    assert!(details.contains("boom"), "{details}");
}

#[test]
fn slice41_45_format_error_without_stack_uses_name_and_message() {
    let details = format_error_details(&instance("CustomError", "boom"), None);
    assert!(details.contains("CustomError"), "{details}");
    assert!(details.contains("boom"), "{details}");
}

#[test]
fn slice41_46_format_empty_record_is_unknown_error() {
    assert_eq!(
        format_error_details(&record(ErrorRecord::default()), None),
        "unknown error"
    );
}

#[test]
fn slice41_47_format_clips_long_details() {
    let details = format_error_details(&ErrorValue::Text("x".repeat(100)), Some(16));
    assert_eq!(details.len(), 16);
    assert!(details.ends_with("..."), "{details}");
}

#[test]
fn slice41_48_format_whitespace_string_is_unknown_error() {
    assert_eq!(
        format_error_details(&ErrorValue::Text("   ".to_string()), None),
        "unknown error"
    );
}

#[test]
fn slice41_49_format_string_passes_through_trimmed() {
    assert_eq!(
        format_error_details(&ErrorValue::Text("  oops  ".to_string()), None),
        "oops"
    );
}

#[test]
fn slice41_50_format_record_renders_message_content() {
    let details = format_error_details(
        &record(ErrorRecord {
            message: Some("Model not found.".to_string()),
            ..Default::default()
        }),
        None,
    );
    assert!(details.contains("Model not found."), "{details}");
}
