//! Parity port of slice-20 remainder in
//! `src/app/managers/summary-aggregation-manager.ts` (subagent
//! terminal/cost, background stale-drop, nested guards, reselect + pick
//! release) and `src/bot/callbacks/session-callback-handler.ts`.

use otb_core::managers::summary_aggregation::SummaryAggregator;

#[test]
fn slice20_terminal_card_emits_only_on_change_with_card() {
    assert!(SummaryAggregator::should_emit_terminal_card(true, true));
    assert!(!SummaryAggregator::should_emit_terminal_card(false, true));
    assert!(!SummaryAggregator::should_emit_terminal_card(true, false));
}

#[test]
fn slice20_cost_fires_for_defined_value() {
    assert!(SummaryAggregator::cost_fires_for_update(Some(1.5)));
    assert!(!SummaryAggregator::cost_fires_for_update(None));
}

#[test]
fn slice20_stale_background_updates_drop_after_retire() {
    assert!(SummaryAggregator::should_drop_stale_background_update(true));
    assert!(!SummaryAggregator::should_drop_stale_background_update(
        false
    ));
}

#[test]
fn slice20_nested_launches_stay_cardless() {
    assert!(SummaryAggregator::should_guard_nested_launch(true));
    assert!(!SummaryAggregator::should_guard_nested_launch(false));
}

#[test]
fn slice20_reselect_same_session_keeps_live_state() {
    assert!(SummaryAggregator::should_keep_live_state_on_reselect(
        Some("s1"),
        "s1"
    ));
    assert!(!SummaryAggregator::should_keep_live_state_on_reselect(
        Some("s1"),
        "s2"
    ));
    assert!(!SummaryAggregator::should_keep_live_state_on_reselect(
        None, "s1"
    ));
}

#[test]
fn slice20_pick_release_drains_with_current_generation() {
    assert_eq!(SummaryAggregator::pick_release_generation(3, 5), 5);
    assert_eq!(SummaryAggregator::pick_release_generation(5, 5), 5);
}

#[test]
fn slice20_finished_identities_are_remembered_without_eviction() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root");
    aggregator.record_pending_background_intent("child-1");
    assert!(aggregator.take_pending_background_intent("child-1"));
    assert!(!aggregator.take_pending_background_intent("child-1"));
}
