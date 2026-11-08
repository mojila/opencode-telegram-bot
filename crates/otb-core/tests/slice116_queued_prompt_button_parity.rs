//! Slice 116 parity: queued-prompt button-press dispatch gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/routers/message-router.ts` (`bot.hears(
//! QUEUED_PROMPT_BUTTON_TEXT_PATTERN, ...)`):
//! - an active interaction blocks before the label is read
//! - a missing label or a label with no queue entry misses (the queue was
//!   drained or cleared after Telegram rendered the keyboard)
//! - an inbox-backed entry withdraws (`removed` / `gone` / error keys)
//! - a locally queued entry is removed by id (`queue.removed`)
//! - an unknown label replies `queue.not_found`
//! - no branch ever falls through to the prompt handler.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `queued_prompt_button_gates` is ported (only constant-value tests pass).

use otb_core::queued_prompt_button_gates::*;

// --- constants (4) ---

#[test]
fn slice116_01_removed_key_is_queue_removed() {
    assert_eq!(QUEUED_PROMPT_REMOVED_KEY, "queue.removed");
}

#[test]
fn slice116_02_not_found_key_is_queue_not_found() {
    assert_eq!(QUEUED_PROMPT_NOT_FOUND_KEY, "queue.not_found");
}

#[test]
fn slice116_03_send_error_key_is_prompt_send_error() {
    assert_eq!(QUEUED_PROMPT_SEND_ERROR_KEY, "bot.prompt_send_error");
}

#[test]
fn slice116_04_blocked_key_is_finish_current() {
    assert_eq!(
        QUEUED_PROMPT_BLOCKED_KEY,
        "interaction.blocked.finish_current"
    );
}

// --- should_lookup_queued_prompt (2) ---

#[test]
fn slice116_05_present_label_scans_the_queue() {
    assert!(should_lookup_queued_prompt(true));
}

#[test]
fn slice116_06_missing_label_skips_lookup() {
    assert!(!should_lookup_queued_prompt(false));
}

// --- select_queued_prompt_button_route (5) ---

#[test]
fn slice116_07_blocked_press_never_reaches_the_queue() {
    assert_eq!(
        select_queued_prompt_button_route(true, true, true),
        QueuedPromptButtonRoute::Blocked
    );
    assert_eq!(
        select_queued_prompt_button_route(true, false, false),
        QueuedPromptButtonRoute::Blocked
    );
}

#[test]
fn slice116_08_inbox_backed_press_withdraws() {
    assert_eq!(
        select_queued_prompt_button_route(false, true, true),
        QueuedPromptButtonRoute::WithdrawInbox
    );
}

#[test]
fn slice116_09_locally_queued_press_removes_by_id() {
    assert_eq!(
        select_queued_prompt_button_route(false, true, false),
        QueuedPromptButtonRoute::RemoveQueued
    );
}

#[test]
fn slice116_10_unknown_label_misses() {
    assert_eq!(
        select_queued_prompt_button_route(false, false, false),
        QueuedPromptButtonRoute::MissNotFound
    );
}

#[test]
fn slice116_11_miss_ignores_stale_inbox_flag() {
    assert_eq!(
        select_queued_prompt_button_route(false, false, true),
        QueuedPromptButtonRoute::MissNotFound
    );
}

// --- queued_prompt_button_reply_key (6) ---

#[test]
fn slice116_12_withdraw_removed_replies_removed() {
    assert_eq!(
        queued_prompt_button_reply_key(QueuedPromptButtonRoute::WithdrawInbox, "removed"),
        "queue.removed"
    );
}

#[test]
fn slice116_13_withdraw_gone_replies_not_found() {
    assert_eq!(
        queued_prompt_button_reply_key(QueuedPromptButtonRoute::WithdrawInbox, "gone"),
        "queue.not_found"
    );
}

#[test]
fn slice116_14_withdraw_error_replies_send_error() {
    assert_eq!(
        queued_prompt_button_reply_key(QueuedPromptButtonRoute::WithdrawInbox, "failed"),
        "bot.prompt_send_error"
    );
}

#[test]
fn slice116_15_removed_press_replies_removed_whatever_withdraw_says() {
    assert_eq!(
        queued_prompt_button_reply_key(QueuedPromptButtonRoute::RemoveQueued, "removed"),
        "queue.removed"
    );
    assert_eq!(
        queued_prompt_button_reply_key(QueuedPromptButtonRoute::RemoveQueued, "gone"),
        "queue.removed"
    );
}

#[test]
fn slice116_16_miss_replies_not_found() {
    assert_eq!(
        queued_prompt_button_reply_key(QueuedPromptButtonRoute::MissNotFound, "removed"),
        "queue.not_found"
    );
}

#[test]
fn slice116_17_blocked_press_replies_finish_current() {
    assert_eq!(
        queued_prompt_button_reply_key(QueuedPromptButtonRoute::Blocked, "removed"),
        "interaction.blocked.finish_current"
    );
}

// --- should_route_button_text_to_prompt (1) ---

#[test]
fn slice116_18_no_branch_falls_through_to_the_prompt_handler() {
    assert!(!should_route_button_text_to_prompt(
        QueuedPromptButtonRoute::Blocked
    ));
    assert!(!should_route_button_text_to_prompt(
        QueuedPromptButtonRoute::WithdrawInbox
    ));
    assert!(!should_route_button_text_to_prompt(
        QueuedPromptButtonRoute::RemoveQueued
    ));
    assert!(!should_route_button_text_to_prompt(
        QueuedPromptButtonRoute::MissNotFound
    ));
}
