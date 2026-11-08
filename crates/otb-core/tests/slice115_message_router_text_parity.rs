//! Slice 115 parity: message-router text dispatch gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/routers/message-router.ts` (`bot.on("message:text")`
//! dispatch chain) and its handler order:
//! - `getIncomingPrompt` miss never routes
//! - `"/"`-prefixed text misses before the question check
//! - active question short-circuits everything else
//! - goal -> task -> model-search -> project-search -> session-search ->
//!   rename -> catalog precedence, each stop-on-handled
//! - busy-session queue wins over merge only when every stage missed
//! - otherwise the text merges (`queuePromptForMerging` tail)
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `message_router_text_gates` is ported (only constant-value tests pass).

use otb_core::message_router_text_gates::*;

// --- constant (1) ---

#[test]
fn slice115_01_slash_prefix_is_slash() {
    assert_eq!(MESSAGE_ROUTER_SLASH_PREFIX, "/");
}

// --- should_miss_without_input (2) ---

#[test]
fn slice115_02_missing_input_misses() {
    assert!(should_miss_without_input(false));
}

#[test]
fn slice115_03_present_input_does_not_miss() {
    assert!(!should_miss_without_input(true));
}

// --- should_miss_for_slash (2) ---

#[test]
fn slice115_04_slash_text_with_input_misses() {
    assert!(should_miss_for_slash(true, true));
}

#[test]
fn slice115_05_no_miss_without_input_or_slash() {
    assert!(!should_miss_for_slash(false, true));
    assert!(!should_miss_for_slash(true, false));
    assert!(!should_miss_for_slash(false, false));
}

// --- should_route_to_question (3) ---

#[test]
fn slice115_06_active_question_with_text_routes() {
    assert!(should_route_to_question(true, false, true));
}

#[test]
fn slice115_07_slash_text_never_routes_to_question() {
    assert!(!should_route_to_question(true, true, true));
}

#[test]
fn slice115_08_no_question_without_input_or_when_idle() {
    assert!(!should_route_to_question(false, false, true));
    assert!(!should_route_to_question(true, false, false));
    assert!(!should_route_to_question(false, false, false));
}

// --- should_attempt_goal (2) ---

#[test]
fn slice115_09_plain_text_without_question_attempts_goal() {
    assert!(should_attempt_goal(true, false, false));
}

#[test]
fn slice115_10_no_goal_after_miss_or_question() {
    assert!(!should_attempt_goal(false, false, false));
    assert!(!should_attempt_goal(true, true, false));
    assert!(!should_attempt_goal(true, false, true));
}

// --- should_stop_after_handled (2) ---

#[test]
fn slice115_11_handled_stage_stops_chain() {
    assert!(should_stop_after_handled(true));
}

#[test]
fn slice115_12_missed_stage_continues_chain() {
    assert!(!should_stop_after_handled(false));
}

// --- should_queue_behind_busy (2) ---

#[test]
fn slice115_13_missed_and_queued_queues() {
    assert!(should_queue_behind_busy(true, true));
}

#[test]
fn slice115_14_no_queue_when_handled_or_not_queued() {
    assert!(!should_queue_behind_busy(false, true));
    assert!(!should_queue_behind_busy(true, false));
    assert!(!should_queue_behind_busy(false, false));
}

// --- select_message_router_text_route (6) ---

#[test]
fn slice115_15_no_input_misses() {
    assert_eq!(
        select_message_router_text_route(false, false, false, false, false, false, false, false, false, false, false),
        MessageRouterTextRoute::MissNoInput
    );
}

#[test]
fn slice115_16_slash_misses_before_question() {
    assert_eq!(
        select_message_router_text_route(true, true, true, false, false, false, false, false, false, false, false),
        MessageRouterTextRoute::MissSlash
    );
}

#[test]
fn slice115_17_question_wins_over_all_handlers() {
    assert_eq!(
        select_message_router_text_route(true, false, true, true, true, true, true, true, true, true, true),
        MessageRouterTextRoute::Question
    );
}

#[test]
fn slice115_18_handler_precedence_is_goal_first() {
    assert_eq!(
        select_message_router_text_route(true, false, false, true, true, true, true, true, true, true, false),
        MessageRouterTextRoute::Goal
    );
    assert_eq!(
        select_message_router_text_route(true, false, false, false, true, true, true, true, true, true, false),
        MessageRouterTextRoute::Task
    );
    assert_eq!(
        select_message_router_text_route(true, false, false, false, false, true, true, true, true, true, false),
        MessageRouterTextRoute::ModelSearch
    );
}

#[test]
fn slice115_19_late_stages_win_only_when_early_missed() {
    assert_eq!(
        select_message_router_text_route(true, false, false, false, false, false, true, true, true, true, false),
        MessageRouterTextRoute::ProjectSearch
    );
    assert_eq!(
        select_message_router_text_route(true, false, false, false, false, false, false, true, true, true, false),
        MessageRouterTextRoute::SessionSearch
    );
    assert_eq!(
        select_message_router_text_route(true, false, false, false, false, false, false, false, true, true, false),
        MessageRouterTextRoute::Rename
    );
    assert_eq!(
        select_message_router_text_route(true, false, false, false, false, false, false, false, false, true, false),
        MessageRouterTextRoute::Catalog
    );
}

#[test]
fn slice115_20_queue_wins_over_merge_only_when_all_missed() {
    assert_eq!(
        select_message_router_text_route(true, false, false, false, false, false, false, false, false, false, true),
        MessageRouterTextRoute::Queue
    );
    assert_eq!(
        select_message_router_text_route(true, false, false, false, false, false, false, false, false, false, false),
        MessageRouterTextRoute::Merge
    );
}
