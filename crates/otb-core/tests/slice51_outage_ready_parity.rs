//! Parity port of the outage-notice / ready-lifecycle / quoted-notice pure
//! gates (slice 51, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/app/services/telegram-outage-notice-service.ts`
//!   (`TelegramOutageNoticeService`: undelivered/skipped flags, burst
//!   suppression, chat-interval gate, take/report/finish flush guard)
//! - `src/opencode/ready-lifecycle.ts` (`OpencodeReadyLifecycle`: once-only
//!   ready, unavailable-only-when-ready, subscribe/unsubscribe fan-out)
//! - `src/opencode/ready-refresh.ts` (`isModelCatalogComplete`,
//!   `isModelCatalogReadyForRestore`, timeout constants, the late-change
//!   gate)
//! - `src/utils/telegram-markdown.ts` (`escapePlainTextForTelegramMarkdownV2`)
//! - `src/app/services/quoted-notification.ts` (plain/markdown quoting,
//!   `buildQuotedNotification` separator rule)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::outage_ready_gates::*;
use std::cell::RefCell;
use std::rc::Rc;

// --- outage service: due-notice flags (01-07) ---

#[test]
fn slice51_01_initial_take_due_is_none() {
    let mut svc = TelegramOutageNoticeService::new();
    assert_eq!(svc.take_due_notices(), None);
}

#[test]
fn slice51_02_mark_undelivered_yields_only_undelivered() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: true,
            skipped: false,
        })
    );
}

#[test]
fn slice51_03_mark_skipped_yields_only_skipped() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_messages_skipped(1_000);
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: false,
            skipped: true,
        })
    );
}

#[test]
fn slice51_04_both_marks_yield_both_due() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    svc.mark_messages_skipped(1_000);
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: true,
            skipped: true,
        })
    );
}

#[test]
fn slice51_05_second_take_while_in_flight_is_none() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    assert!(svc.take_due_notices().is_some());
    assert_eq!(svc.take_due_notices(), None);
}

#[test]
fn slice51_06_finish_flush_allows_retake_while_pending() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    assert!(svc.take_due_notices().is_some());
    svc.finish_flush();
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: true,
            skipped: false,
        })
    );
}

#[test]
fn slice51_07_take_after_all_reported_delivered_is_none() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    svc.mark_messages_skipped(1_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Undelivered, true);
    svc.report_notice_result(OutageNoticeKind::Skipped, true);
    svc.finish_flush();
    assert_eq!(svc.take_due_notices(), None);
}

// --- outage service: report results (08-13) ---

#[test]
fn slice51_08_report_undelivered_true_clears_flag() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Undelivered, true);
    svc.finish_flush();
    assert_eq!(svc.take_due_notices(), None);
}

#[test]
fn slice51_09_report_undelivered_false_keeps_flag() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Undelivered, false);
    svc.finish_flush();
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: true,
            skipped: false,
        })
    );
}

#[test]
fn slice51_10_report_skipped_true_clears_flag() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_messages_skipped(1_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Skipped, true);
    svc.finish_flush();
    assert_eq!(svc.take_due_notices(), None);
}

#[test]
fn slice51_11_report_skipped_false_keeps_flag() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_messages_skipped(1_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Skipped, false);
    svc.finish_flush();
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: false,
            skipped: true,
        })
    );
}

#[test]
fn slice51_12_report_undelivered_leaves_skipped_pending() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    svc.mark_messages_skipped(1_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Undelivered, true);
    svc.finish_flush();
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: false,
            skipped: true,
        })
    );
}

#[test]
fn slice51_13_report_skipped_leaves_undelivered_pending() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    svc.mark_messages_skipped(1_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Skipped, true);
    svc.finish_flush();
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: true,
            skipped: false,
        })
    );
}

// --- outage service: burst suppression (14-17) ---

#[test]
fn slice51_14_mark_within_gap_after_consume_stays_suppressed() {
    // The second mark refreshes the stale stamp: 60s after the second mark
    // the burst still holds even though 119s passed since the first.
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_messages_skipped(1_000);
    svc.mark_messages_skipped(60_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Skipped, true);
    svc.finish_flush();
    svc.mark_messages_skipped(120_000);
    svc.finish_flush();
    assert_eq!(svc.take_due_notices(), None);
}

#[test]
fn slice51_15_mark_after_gap_rearms_pending() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_messages_skipped(1_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Skipped, true);
    svc.finish_flush();
    svc.mark_messages_skipped(1_000 + OUTAGE_BURST_GAP_MS + 1);
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: false,
            skipped: true,
        })
    );
}

#[test]
fn slice51_16_close_burst_then_mark_rearms_immediately() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_messages_skipped(1_000);
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Skipped, true);
    svc.finish_flush();
    svc.close_burst();
    svc.mark_messages_skipped(1_001);
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: false,
            skipped: true,
        })
    );
}

#[test]
fn slice51_17_undelivered_mark_never_burst_suppressed() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.mark_assistant_reply_undelivered();
    assert!(svc.take_due_notices().is_some());
    svc.report_notice_result(OutageNoticeKind::Undelivered, true);
    svc.finish_flush();
    svc.mark_assistant_reply_undelivered();
    assert_eq!(
        svc.take_due_notices(),
        Some(OutageNoticesDue {
            undelivered: true,
            skipped: false,
        })
    );
}

// --- outage service: chat interval (18-21) ---

#[test]
fn slice51_18_ms_until_allowed_zero_before_any_send() {
    let svc = TelegramOutageNoticeService::new();
    assert_eq!(svc.ms_until_allowed_send(50_000), 0);
}

#[test]
fn slice51_19_ms_until_allowed_full_interval_right_after_note() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.note_chat_send_succeeded(50_000);
    assert_eq!(svc.ms_until_allowed_send(50_000), OUTAGE_CHAT_INTERVAL_MS);
}

#[test]
fn slice51_20_ms_until_allowed_counts_down() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.note_chat_send_succeeded(50_000);
    assert_eq!(svc.ms_until_allowed_send(50_400), 600);
}

#[test]
fn slice51_21_ms_until_allowed_zero_after_interval_and_never_negative() {
    let mut svc = TelegramOutageNoticeService::new();
    svc.note_chat_send_succeeded(50_000);
    assert_eq!(svc.ms_until_allowed_send(51_000), 0);
    assert_eq!(svc.ms_until_allowed_send(500_000), 0);
}

// --- ready lifecycle (22-30) ---

#[test]
fn slice51_22_initial_lifecycle_is_not_ready() {
    let lifecycle = OpencodeReadyLifecycle::new();
    assert!(!lifecycle.is_ready());
}

#[test]
fn slice51_23_first_notify_ready_returns_true_and_marks_ready() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    assert!(lifecycle.notify_ready("startup"));
    assert!(lifecycle.is_ready());
}

#[test]
fn slice51_24_second_notify_ready_is_ignored() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    assert!(lifecycle.notify_ready("startup"));
    assert!(!lifecycle.notify_ready("again"));
    assert!(lifecycle.is_ready());
}

#[test]
fn slice51_25_notify_unavailable_when_not_ready_is_ignored() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    assert!(!lifecycle.notify_unavailable("early"));
    assert!(!lifecycle.is_ready());
}

#[test]
fn slice51_26_notify_unavailable_after_ready_clears() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    assert!(lifecycle.notify_ready("startup"));
    assert!(lifecycle.notify_unavailable("drop"));
    assert!(!lifecycle.is_ready());
}

#[test]
fn slice51_27_notify_ready_again_after_unavailable() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    assert!(lifecycle.notify_ready("startup"));
    assert!(lifecycle.notify_unavailable("drop"));
    assert!(lifecycle.notify_ready("recovered"));
    assert!(lifecycle.is_ready());
}

#[test]
fn slice51_28_on_ready_handler_receives_reason() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    let seen = Rc::new(RefCell::new(Vec::<String>::new()));
    let seen_clone = Rc::clone(&seen);
    lifecycle.on_ready(Box::new(move |reason| {
        seen_clone.borrow_mut().push(reason.to_string());
    }));
    assert!(lifecycle.notify_ready("startup"));
    assert_eq!(*seen.borrow(), vec!["startup".to_string()]);
}

#[test]
fn slice51_29_all_handlers_called_in_order() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    let calls = Rc::new(RefCell::new(Vec::<u32>::new()));
    for id in [1u32, 2u32] {
        let calls_clone = Rc::clone(&calls);
        lifecycle.on_ready(Box::new(move |_| {
            calls_clone.borrow_mut().push(id);
        }));
    }
    assert!(lifecycle.notify_ready("startup"));
    assert_eq!(*calls.borrow(), vec![1u32, 2u32]);
}

#[test]
fn slice51_30_removed_handler_is_not_called() {
    let mut lifecycle = OpencodeReadyLifecycle::new();
    let calls = Rc::new(RefCell::new(0u32));
    let calls_clone = Rc::clone(&calls);
    let id = lifecycle.on_ready(Box::new(move |_| {
        *calls_clone.borrow_mut() += 1;
    }));
    assert!(lifecycle.remove_ready_handler(id));
    assert!(!lifecycle.remove_ready_handler(id));
    assert!(lifecycle.notify_ready("startup"));
    assert_eq!(*calls.borrow(), 0u32);
}

// --- catalog readiness gates (31-38) ---

#[test]
fn slice51_31_catalog_complete_when_available_and_complete() {
    assert!(is_model_catalog_complete(StoredModelReconcile {
        catalog_available: true,
        catalog_complete: true,
        selected_model_listed: true,
        stored_model_replaced: false,
    }));
}

#[test]
fn slice51_32_catalog_not_complete_when_unavailable() {
    assert!(!is_model_catalog_complete(StoredModelReconcile {
        catalog_available: false,
        catalog_complete: false,
        selected_model_listed: false,
        stored_model_replaced: false,
    }));
}

#[test]
fn slice51_33_catalog_not_complete_when_incomplete() {
    assert!(!is_model_catalog_complete(StoredModelReconcile {
        catalog_available: true,
        catalog_complete: false,
        selected_model_listed: true,
        stored_model_replaced: false,
    }));
}

#[test]
fn slice51_34_restore_ready_when_listed_but_incomplete() {
    assert!(is_model_catalog_ready_for_restore(StoredModelReconcile {
        catalog_available: true,
        catalog_complete: false,
        selected_model_listed: true,
        stored_model_replaced: false,
    }));
}

#[test]
fn slice51_35_restore_ready_when_complete_even_unlisted() {
    assert!(is_model_catalog_ready_for_restore(StoredModelReconcile {
        catalog_available: true,
        catalog_complete: true,
        selected_model_listed: false,
        stored_model_replaced: true,
    }));
}

#[test]
fn slice51_36_restore_not_ready_when_unavailable() {
    assert!(!is_model_catalog_ready_for_restore(StoredModelReconcile {
        catalog_available: false,
        catalog_complete: false,
        selected_model_listed: false,
        stored_model_replaced: false,
    }));
}

#[test]
fn slice51_37_restore_not_ready_when_neither_listed_nor_complete() {
    assert!(!is_model_catalog_ready_for_restore(StoredModelReconcile {
        catalog_available: true,
        catalog_complete: false,
        selected_model_listed: false,
        stored_model_replaced: false,
    }));
}

#[test]
fn slice51_38_catalog_timeout_constants_match_ts() {
    assert_eq!(MODEL_CATALOG_WAIT_TIMEOUT_MS, 3000);
    assert_eq!(MODEL_CATALOG_POLL_INTERVAL_MS, 500);
    assert_eq!(MODEL_CATALOG_WARMUP_POLL_INTERVAL_MS, 5000);
}

// --- late catalog change gate (39-41) ---

#[test]
fn slice51_39_late_change_when_stored_model_replaced() {
    assert!(should_note_late_catalog_change(true, true, true));
}

#[test]
fn slice51_40_late_change_when_newly_listed() {
    assert!(should_note_late_catalog_change(false, false, true));
}

#[test]
fn slice51_41_no_late_change_when_nothing_changed() {
    assert!(!should_note_late_catalog_change(false, true, true));
    assert!(!should_note_late_catalog_change(false, false, false));
}

// --- markdown escape + quoted notices (42-50) ---

#[test]
fn slice51_42_escape_marks_each_reserved_char() {
    assert_eq!(
        escape_plain_text_for_telegram_markdown_v2("_*[]()~`>#+-=|{}.!\\"),
        "\\_\\*\\[\\]\\(\\)\\~\\`\\>\\#\\+\\-\\=\\|\\{\\}\\.\\!\\\\"
    );
}

#[test]
fn slice51_43_escape_leaves_plain_text_untouched() {
    assert_eq!(
        escape_plain_text_for_telegram_markdown_v2("hello world 123"),
        "hello world 123"
    );
}

#[test]
fn slice51_44_escape_empty_stays_empty() {
    assert_eq!(escape_plain_text_for_telegram_markdown_v2(""), "");
}

#[test]
fn slice51_45_quoted_plain_single_line() {
    assert_eq!(build_quoted_plain_text("hello"), "> hello");
}

#[test]
fn slice51_46_quoted_plain_multiline_and_empty_line() {
    assert_eq!(build_quoted_plain_text("a\n\nb"), "> a\n>\n> b");
}

#[test]
fn slice51_47_quoted_markdown_escapes_body() {
    assert_eq!(build_quoted_markdown_text("a*b"), "> a\\*b");
}

#[test]
fn slice51_48_quoted_notification_default_blank_line() {
    let notice = build_quoted_notification("Ti*tle", "a_b", None);
    assert_eq!(
        notice,
        QuotedNotification {
            text: "Ti\\*tle\n\n> a\\_b".to_string(),
            raw_fallback_text: "Ti*tle\n\n> a_b".to_string(),
        }
    );
}

#[test]
fn slice51_49_quoted_notification_explicit_false_single_newline() {
    let notice = build_quoted_notification("title", "body", Some(false));
    assert_eq!(notice.text, "title\n> body");
    assert_eq!(notice.raw_fallback_text, "title\n> body");
}

#[test]
fn slice51_50_quoted_notification_explicit_true_blank_line() {
    let notice = build_quoted_notification("title", "l1\nl2", Some(true));
    assert_eq!(notice.text, "title\n\n> l1\n> l2");
    assert_eq!(notice.raw_fallback_text, "title\n\n> l1\n> l2");
}
