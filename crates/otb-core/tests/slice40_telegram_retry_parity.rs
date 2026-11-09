//! Parity port of Telegram retry/backoff pure gates (slice 40, RED).
//!
//! Mirrors representative cases from:
//! - `tests/utils/telegram-rate-limit-retry.test.ts` (retry_after params,
//!   wrapped fetch codes, system-code-over-URL-text precedence, transient
//!   network opt-in, non-retryable errors, 5xx detection, exponential
//!   backoff, backoff cap, retryable-by-default vs opt-in routing)
//! - `src/utils/telegram-rate-limit-retry.ts` (every `is*` / `get*`
//!   branch: status vs `error_code`, `retry_after` params vs message vs
//!   fallback, nested `.error`/`.cause` code/type resolution, pre-TLS
//!   `ECONNRESET` wording, `request-timeout` type, backoff normalize/cap)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::telegram_retry_gates::*;

fn err() -> TelegramCallError {
    TelegramCallError::default()
}

fn msg(s: &str) -> TelegramCallError {
    TelegramCallError {
        message: Some(s.to_string()),
        ..Default::default()
    }
}

fn coded(code: &str) -> TelegramCallError {
    TelegramCallError {
        code: Some(code.to_string()),
        ..Default::default()
    }
}

fn nested_coded(code: &str) -> TelegramCallError {
    TelegramCallError {
        nested_code: Some(code.to_string()),
        ..Default::default()
    }
}

// --- constants (3) ---

#[test]
fn slice40_01_transient_server_codes_are_5xx_set() {
    assert_eq!(TRANSIENT_SERVER_ERROR_CODES, [500, 502, 503, 504]);
    assert_eq!(MAX_SERVER_ERROR_BACKOFF_MS, 8000);
}

#[test]
fn slice40_02_connect_not_established_codes() {
    assert!(CONNECT_NOT_ESTABLISHED_CODES.contains(&"ECONNREFUSED"));
    assert!(CONNECT_NOT_ESTABLISHED_CODES.contains(&"ENOTFOUND"));
    assert!(CONNECT_NOT_ESTABLISHED_CODES.contains(&"EAI_AGAIN"));
    assert!(CONNECT_NOT_ESTABLISHED_CODES.contains(&"ENETUNREACH"));
    assert!(CONNECT_NOT_ESTABLISHED_CODES.contains(&"EHOSTUNREACH"));
    assert!(!CONNECT_NOT_ESTABLISHED_CODES.contains(&"ECONNRESET"));
}

#[test]
fn slice40_03_transient_network_codes_extend_connect_set() {
    for code in CONNECT_NOT_ESTABLISHED_CODES {
        assert!(TRANSIENT_NETWORK_ERROR_CODES.contains(&code), "{code}");
    }
    for code in ["ECONNABORTED", "ECONNRESET", "EPIPE", "EPROTO", "ETIMEDOUT"] {
        assert!(TRANSIENT_NETWORK_ERROR_CODES.contains(&code), "{code}");
    }
    assert!(!TRANSIENT_NETWORK_ERROR_CODES.contains(&"EUNKNOWN"));
}

// --- is_telegram_rate_limit_error (8) ---

#[test]
fn slice40_04_status_429_is_rate_limited() {
    assert!(is_telegram_rate_limit_error(&TelegramCallError {
        status: Some(429),
        ..err()
    }));
}

#[test]
fn slice40_05_error_code_429_is_rate_limited() {
    assert!(is_telegram_rate_limit_error(&TelegramCallError {
        error_code: Some(429),
        ..err()
    }));
}

#[test]
fn slice40_06_429_word_in_message_is_rate_limited() {
    assert!(is_telegram_rate_limit_error(&msg("429: Too Many Requests")));
    assert!(is_telegram_rate_limit_error(&msg(
        "Call failed! (429: Too Many Requests: retry after 1)"
    )));
}

#[test]
fn slice40_07_too_many_requests_phrase_is_rate_limited() {
    assert!(is_telegram_rate_limit_error(&msg(
        "Too Many Requests: retry after 2"
    )));
    assert!(is_telegram_rate_limit_error(&msg("TOO MANY REQUESTS")));
}

#[test]
fn slice40_08_400_status_and_message_are_not_rate_limited() {
    assert!(!is_telegram_rate_limit_error(&TelegramCallError {
        error_code: Some(400),
        ..err()
    }));
    assert!(!is_telegram_rate_limit_error(&msg("400: Bad Request")));
}

#[test]
fn slice40_09_system_code_suppresses_429_message_sniffing() {
    assert!(!is_telegram_rate_limit_error(&TelegramCallError {
        message: Some(
            "request to https://api.telegram.org/file/bot***/files/429/retry after 99/x.pdf failed"
                .to_string(),
        ),
        code: Some("ECONNREFUSED".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_10_status_wins_over_contradicting_message() {
    assert!(!is_telegram_rate_limit_error(&TelegramCallError {
        status: Some(400),
        message: Some("too many requests".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_11_empty_error_is_not_rate_limited() {
    assert!(!is_telegram_rate_limit_error(&err()));
    assert!(!is_telegram_rate_limit_error(&msg(
        "Network request for 'sendMessage' failed!"
    )));
}

// --- is_transient_telegram_server_error (8) ---

#[test]
fn slice40_12_error_code_502_is_transient_server_error() {
    assert!(is_transient_telegram_server_error(&TelegramCallError {
        error_code: Some(502),
        description: Some("Bad Gateway".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_13_all_transient_codes_detected() {
    for code in [500, 502, 503, 504] {
        assert!(
            is_transient_telegram_server_error(&TelegramCallError {
                error_code: Some(code),
                ..err()
            }),
            "{code}"
        );
    }
}

#[test]
fn slice40_14_status_502_is_transient_server_error() {
    assert!(is_transient_telegram_server_error(&TelegramCallError {
        status: Some(503),
        ..err()
    }));
}

#[test]
fn slice40_15_502_in_message_is_transient_server_error() {
    assert!(is_transient_telegram_server_error(&msg(
        "Call to 'sendMessage' failed! (502: Bad Gateway)"
    )));
}

#[test]
fn slice40_16_400_is_not_transient_server_error() {
    assert!(!is_transient_telegram_server_error(&TelegramCallError {
        error_code: Some(400),
        ..err()
    }));
    assert!(!is_transient_telegram_server_error(&msg(
        "400: Bad Request"
    )));
}

#[test]
fn slice40_17_system_code_suppresses_502_url_text() {
    let networkish = TelegramCallError {
        message: Some(
            "request to https://api.telegram.org/file/bot***/files/502/x.pdf failed".to_string(),
        ),
        code: Some("ECONNREFUSED".to_string()),
        ..err()
    };
    assert!(!is_transient_telegram_server_error(&networkish));
}

#[test]
fn slice40_18_429_is_not_a_transient_server_error() {
    assert!(!is_transient_telegram_server_error(&TelegramCallError {
        error_code: Some(429),
        ..err()
    }));
}

#[test]
fn slice40_19_empty_error_is_not_transient_server_error() {
    assert!(!is_transient_telegram_server_error(&err()));
}

// --- is_unsent_telegram_network_error (10) ---

#[test]
fn slice40_20_wrapped_refused_is_unsent() {
    assert!(is_unsent_telegram_network_error(&nested_coded(
        "ECONNREFUSED"
    )));
    assert!(is_unsent_telegram_network_error(&nested_coded("ENOTFOUND")));
    assert!(is_unsent_telegram_network_error(&nested_coded("EAI_AGAIN")));
}

#[test]
fn slice40_21_wrapped_unreachable_is_unsent() {
    assert!(is_unsent_telegram_network_error(&nested_coded(
        "ENETUNREACH"
    )));
    assert!(is_unsent_telegram_network_error(&nested_coded(
        "EHOSTUNREACH"
    )));
}

#[test]
fn slice40_22_top_level_code_is_unsent() {
    assert!(is_unsent_telegram_network_error(&coded("ECONNREFUSED")));
    assert!(is_unsent_telegram_network_error(&coded("ENOTFOUND")));
}

#[test]
fn slice40_23_request_timeout_type_is_unsent() {
    assert!(is_unsent_telegram_network_error(&TelegramCallError {
        nested_type: Some("request-timeout".to_string()),
        ..err()
    }));
    assert!(is_unsent_telegram_network_error(&TelegramCallError {
        type_field: Some("request-timeout".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_24_bare_reset_after_write_is_not_unsent() {
    assert!(!is_unsent_telegram_network_error(&nested_coded(
        "ECONNRESET"
    )));
}

#[test]
fn slice40_25_pre_tls_reset_wording_is_unsent() {
    assert!(is_unsent_telegram_network_error(&TelegramCallError {
        message: Some(
            "Client network socket disconnected before secure TLS connection was established"
                .to_string(),
        ),
        code: Some("ECONNRESET".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_26_reset_with_other_message_is_not_unsent() {
    assert!(!is_unsent_telegram_network_error(&TelegramCallError {
        message: Some("socket hang up".to_string()),
        code: Some("ECONNRESET".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_27_plain_failure_is_not_unsent() {
    assert!(!is_unsent_telegram_network_error(&msg(
        "Network request for 'sendMessage' failed!"
    )));
    assert!(!is_unsent_telegram_network_error(&err()));
}

#[test]
fn slice40_28_unknown_code_is_not_unsent() {
    assert!(!is_unsent_telegram_network_error(&nested_coded("EUNKNOWN")));
    assert!(!is_unsent_telegram_network_error(&nested_coded(
        "ETIMEDOUT"
    )));
}

#[test]
fn slice40_29_other_timeout_type_is_not_unsent() {
    assert!(!is_unsent_telegram_network_error(&TelegramCallError {
        type_field: Some("connect-timeout".to_string()),
        ..err()
    }));
}

// --- is_transient_telegram_network_error (7) ---

#[test]
fn slice40_30_reset_is_transient_network_error() {
    assert!(is_transient_telegram_network_error(&nested_coded(
        "ECONNRESET"
    )));
    assert!(is_transient_telegram_network_error(&coded("ECONNRESET")));
}

#[test]
fn slice40_31_broad_transient_codes_detected() {
    for code in ["ECONNABORTED", "EPIPE", "EPROTO", "ETIMEDOUT"] {
        assert!(
            is_transient_telegram_network_error(&nested_coded(code)),
            "{code}"
        );
    }
}

#[test]
fn slice40_32_connect_codes_are_transient_network_errors() {
    assert!(is_transient_telegram_network_error(&nested_coded(
        "ECONNREFUSED"
    )));
    assert!(is_transient_telegram_network_error(&nested_coded(
        "EAI_AGAIN"
    )));
}

#[test]
fn slice40_33_request_timeout_type_is_transient() {
    assert!(is_transient_telegram_network_error(&TelegramCallError {
        nested_type: Some("request-timeout".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_34_plain_and_unknown_are_not_transient_network() {
    assert!(!is_transient_telegram_network_error(&err()));
    assert!(!is_transient_telegram_network_error(&msg("socket hang up")));
    assert!(!is_transient_telegram_network_error(&nested_coded(
        "EUNKNOWN"
    )));
}

#[test]
fn slice40_35_cause_nested_code_resolves_like_error_nesting() {
    assert!(is_transient_telegram_network_error(&TelegramCallError {
        nested_code: Some("ECONNRESET".to_string()),
        ..err()
    }));
    assert!(is_unsent_telegram_network_error(&TelegramCallError {
        nested_code: Some("ECONNREFUSED".to_string()),
        ..err()
    }));
}

#[test]
fn slice40_36_transient_implies_retryable_but_unsent_is_narrower() {
    let reset = nested_coded("ECONNRESET");
    assert!(is_transient_telegram_network_error(&reset));
    assert!(!is_unsent_telegram_network_error(&reset));
    let refused = nested_coded("ECONNREFUSED");
    assert!(is_transient_telegram_network_error(&refused));
    assert!(is_unsent_telegram_network_error(&refused));
}

// --- server_error_backoff_ms (5) ---

#[test]
fn slice40_37_backoff_first_attempt_is_base() {
    assert_eq!(server_error_backoff_ms(0, 500), 500);
    assert_eq!(server_error_backoff_ms(0, 1000), 1000);
}

#[test]
fn slice40_38_backoff_doubles_per_attempt() {
    assert_eq!(server_error_backoff_ms(1, 500), 1000);
    assert_eq!(server_error_backoff_ms(2, 500), 2000);
    assert_eq!(server_error_backoff_ms(3, 500), 4000);
}

#[test]
fn slice40_39_backoff_caps_at_8000() {
    assert_eq!(server_error_backoff_ms(20, 1000), 8000);
    assert_eq!(server_error_backoff_ms(4, 500), 8000);
    assert_eq!(server_error_backoff_ms(100, 1), 8000);
}

#[test]
fn slice40_40_backoff_floors_and_minimums() {
    assert_eq!(server_error_backoff_ms(0, 0), 1);
    assert!(server_error_backoff_ms(2, 333) >= 1);
}

#[test]
fn slice40_41_backoff_never_exceeds_cap() {
    for attempt in 0..30 {
        assert!(server_error_backoff_ms(attempt, 1000) <= MAX_SERVER_ERROR_BACKOFF_MS);
    }
}

// --- get_telegram_retry_after_ms (7) ---

#[test]
fn slice40_42_retry_after_params_win_for_429() {
    assert_eq!(
        get_telegram_retry_after_ms(
            &TelegramCallError {
                error_code: Some(429),
                retry_after_secs: Some(3.0),
                ..err()
            },
            1000,
            0
        ),
        Some(3000)
    );
}

#[test]
fn slice40_43_retry_after_message_parsed_for_429() {
    assert_eq!(
        get_telegram_retry_after_ms(&msg("429: Too Many Requests: retry after 1"), 1000, 0),
        Some(1000)
    );
}

#[test]
fn slice40_44_429_without_info_falls_back() {
    assert_eq!(
        get_telegram_retry_after_ms(
            &TelegramCallError {
                error_code: Some(429),
                ..err()
            },
            500,
            0
        ),
        Some(500)
    );
}

#[test]
fn slice40_45_non_positive_retry_after_falls_back() {
    assert_eq!(
        get_telegram_retry_after_ms(
            &TelegramCallError {
                error_code: Some(429),
                retry_after_secs: Some(0.0),
                ..err()
            },
            500,
            0
        ),
        Some(500)
    );
}

#[test]
fn slice40_46_networkish_url_uses_backoff_not_rate_limit() {
    let networkish = TelegramCallError {
        message: Some(
            "request to https://api.telegram.org/file/bot***/files/429/retry after 99/x.pdf failed"
                .to_string(),
        ),
        code: Some("ECONNREFUSED".to_string()),
        ..err()
    };
    assert_eq!(get_telegram_retry_after_ms(&networkish, 500, 0), Some(500));
}

#[test]
fn slice40_47_unsent_network_error_uses_backoff() {
    assert_eq!(
        get_telegram_retry_after_ms(&nested_coded("ECONNREFUSED"), 500, 0),
        Some(500)
    );
    assert_eq!(
        get_telegram_retry_after_ms(&nested_coded("ECONNREFUSED"), 500, 1),
        Some(1000)
    );
}

#[test]
fn slice40_48_transient_server_error_uses_backoff() {
    let gateway = TelegramCallError {
        error_code: Some(502),
        ..err()
    };
    assert_eq!(get_telegram_retry_after_ms(&gateway, 500, 0), Some(500));
    assert_eq!(get_telegram_retry_after_ms(&gateway, 500, 1), Some(1000));
    assert_eq!(get_telegram_retry_after_ms(&gateway, 500, 2), Some(2000));
    assert_eq!(
        get_telegram_retry_after_ms(
            &TelegramCallError {
                error_code: Some(503),
                ..err()
            },
            1000,
            20
        ),
        Some(8000)
    );
}

#[test]
fn slice40_49_non_retryable_errors_yield_none() {
    assert_eq!(
        get_telegram_retry_after_ms(&msg("400: Bad Request"), 500, 0),
        None
    );
    assert_eq!(
        get_telegram_retry_after_ms(
            &TelegramCallError {
                error_code: Some(400),
                ..err()
            },
            500,
            0
        ),
        None
    );
    assert_eq!(get_telegram_retry_after_ms(&err(), 500, 0), None);
}

#[test]
fn slice40_50_fallback_floor_is_one_ms() {
    assert_eq!(
        get_telegram_retry_after_ms(
            &TelegramCallError {
                error_code: Some(429),
                ..err()
            },
            0,
            0
        ),
        Some(1)
    );
}
