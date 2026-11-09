//! Parity port of `tests/app/services/event-router.test.ts`
//! (session identity, heartbeat, busy marking, cache ingest, goal
//! short-circuit, foreground resolution, full dispatch).

use otb_core::event_router::{
    get_event_session_id, is_goal_changed_event, is_heartbeat_event, resolve_foreground_session_id,
    route_event, should_ingest_session_info, should_mark_attached_busy,
    should_mark_attached_busy_for_snapshot,
};

#[test]
fn router_reads_session_id_with_fallback_order() {
    assert_eq!(
        get_event_session_id(Some("s1"), Some("info"), Some("part")),
        Some("s1".to_string())
    );
    assert_eq!(
        get_event_session_id(None, Some("info"), Some("part")),
        Some("info".to_string())
    );
    assert_eq!(
        get_event_session_id(None, None, Some("part")),
        Some("part".to_string())
    );
    assert_eq!(get_event_session_id(None, None, None), None);
    assert_eq!(get_event_session_id(Some(""), None, None), None);
}

#[test]
fn router_detects_heartbeat() {
    assert!(is_heartbeat_event("server.heartbeat"));
    assert!(!is_heartbeat_event("session.idle"));
}

#[test]
fn router_marks_busy_from_progress_events() {
    assert!(should_mark_attached_busy(
        "message.part.delta",
        false,
        false
    ));
    assert!(should_mark_attached_busy("question.asked", false, false));
    assert!(should_mark_attached_busy("session.status", false, true));
    assert!(!should_mark_attached_busy("session.status", false, false));
    assert!(should_mark_attached_busy("message.updated", true, false));
    assert!(!should_mark_attached_busy("message.updated", false, false));
    assert!(!should_mark_attached_busy("session.idle", false, false));
}

#[test]
fn router_marks_busy_only_for_the_attached_session() {
    assert!(should_mark_attached_busy_for_snapshot(
        Some("s1"),
        Some("s1"),
        "message.part.delta",
        false,
        false
    ));
    assert!(!should_mark_attached_busy_for_snapshot(
        Some("s1"),
        Some("s2"),
        "message.part.delta",
        false,
        false
    ));
    assert!(!should_mark_attached_busy_for_snapshot(
        None,
        Some("s1"),
        "message.part.delta",
        false,
        false
    ));
    assert!(!should_mark_attached_busy_for_snapshot(
        Some("s1"),
        None,
        "message.part.delta",
        false,
        false
    ));
}

#[test]
fn router_ingests_cache_only_for_session_events_with_directory() {
    assert!(should_ingest_session_info("session.created", true));
    assert!(should_ingest_session_info("session.updated", true));
    assert!(!should_ingest_session_info("session.created", false));
    assert!(!should_ingest_session_info("session.idle", true));
}

#[test]
fn router_short_circuits_goal_changed() {
    assert!(is_goal_changed_event("goal.changed"));
    assert!(!is_goal_changed_event("session.idle"));
}

#[test]
fn router_resolves_foreground_session_for_background_tracking() {
    assert_eq!(
        resolve_foreground_session_id(Some("s1"), true),
        Some("s1".to_string())
    );
    assert_eq!(resolve_foreground_session_id(Some("s1"), false), None);
    assert_eq!(resolve_foreground_session_id(None, true), None);
}

#[test]
fn router_dispatches_heartbeat_as_subscription_wide_work() {
    let decision = route_event(
        "server.heartbeat",
        "d:/repo",
        None,
        None,
        false,
        false,
        false,
        false,
    );
    assert!(decision.reconcile_busy);
    assert!(!decision.mark_attached_busy);
    assert!(!decision.ingest_session_cache);
    assert!(!decision.goal_changed);
    assert!(decision.forward_to_aggregator);
}

#[test]
fn router_dispatches_attached_progress_and_cache_and_goal() {
    let busy = route_event(
        "message.part.delta",
        "d:/repo",
        Some("s1"),
        Some("s1"),
        false,
        false,
        false,
        false,
    );
    assert!(busy.mark_attached_busy);
    assert!(busy.forward_to_aggregator);

    let cache = route_event(
        "session.created",
        "d:/repo",
        None,
        None,
        false,
        false,
        true,
        false,
    );
    assert!(cache.ingest_session_cache);

    let goal = route_event(
        "goal.changed",
        "d:/repo",
        None,
        Some("s1"),
        false,
        false,
        false,
        true,
    );
    assert!(goal.goal_changed);
    assert!(!goal.forward_to_aggregator);
    assert_eq!(goal.foreground_session_id, None);
}
