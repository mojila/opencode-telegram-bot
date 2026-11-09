//! Slice 173 parity: task-goal gates (toggle outcomes, task details,
//! dashboard goal line, goal notices, session-ignore guards).
//!
//! Mirrors the pure TS contract in:
//! - `src/bot/callbacks/scheduled-task-callback-handler.ts`
//!   (`handleTaskListCallback` toggle outcome branches: still-stored answers
//!   `tasklist.inactive_callback` and keeps the menu, gone clears the
//!   interaction and deletes the menu message; `refreshTask(...) ?? task`
//!   display pick; `formatTaskDetails` composition and the `formatDateTime`
//!   missing-date `"-"` fallback)
//! - `src/app/formatters/goal-text.ts` (`formatDashboardGoalLine` template
//!   render, `formatGoalNotice` complete/paused key pick,
//!   `formatObjectiveTooLong` limit/count params)
//! - `src/app/services/scheduled-task-session-ignore-service.ts`
//!   (`SCHEDULED_TASK_SESSION_IGNORE_TTL_MS`, `isFreshIgnore`, prune,
//!   ignored check, register/remove/cleanup next-state)
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `tasklist:open` / `tasklist:toggle` parses and button labels
//!   (`recent_scheduled_gates` owner), toggle/delete allow gates and prompt
//!   truncation (`task_callback_gates` owner)
//! - `GOAL_OBJECTIVE_LIMIT` value / truncation / flattening
//!   (`session_gates` owner), goal status-text composition
//!   (`goal_reason_gates` owner)
//! - `/tasklist` sort order (`lifecycle_command_gates` owner)
//! - grammy `ctx`/`Api` I/O, store reads/writes, `t()` lookups, `Intl`
//!   rendering (callers)

use otb_core::session_gates::GOAL_OBJECTIVE_LIMIT;
use otb_core::task_goal_gates::*;

// ---- decide_toggle_missing_task (2) ----

#[test]
fn toggle_missing_when_still_stored_answers_inactive_parity() {
    // Arrange: the store still holds the task.
    // Act + Assert: answer inactive, keep the menu.
    assert_eq!(
        decide_toggle_missing_task(true),
        ToggleMissingOutcome::AnswerInactive
    );
}

#[test]
fn toggle_missing_when_gone_clears_and_deletes_parity() {
    // Arrange: the task is gone from the store.
    // Act + Assert: clear the interaction and delete the menu message.
    assert_eq!(
        decide_toggle_missing_task(false),
        ToggleMissingOutcome::ClearAndDelete
    );
}

// ---- prefer_refreshed_task (2) ----

#[test]
fn prefer_refreshed_task_uses_refreshed_row_when_tracked_parity() {
    // Arrange: the runtime still tracks the task.
    // Act: offer the refreshed row over the toggled row.
    let picked = prefer_refreshed_task(Some("refreshed"), "toggled");
    // Assert: the refreshed row wins.
    assert_eq!(picked, "refreshed");
}

#[test]
fn prefer_refreshed_task_keeps_toggled_row_when_not_tracked_parity() {
    // Arrange: the runtime no longer tracks the task.
    // Act: offer no refreshed row.
    let picked: &str = prefer_refreshed_task(None, "toggled");
    // Assert: the toggled row stays on screen.
    assert_eq!(picked, "toggled");
}

// ---- resolve_task_date_label (2) ----

#[test]
fn task_date_label_returns_preformatted_date_parity() {
    // Arrange: a preformatted run date.
    // Act + Assert: it passes through.
    assert_eq!(
        resolve_task_date_label(Some("2026-11-09 10:00")),
        "2026-11-09 10:00"
    );
}

#[test]
fn task_date_label_falls_back_to_dash_when_missing_parity() {
    // Arrange: no date (TS `!dateIso` / `Date.parse` NaN).
    // Act + Assert: the `"-"` fallback.
    assert_eq!(resolve_task_date_label(None), "-");
}

// ---- render_task_details (1) ----

#[test]
fn render_task_details_substitutes_all_placeholders_parity() {
    // Arrange: a locale-owned template plus resolved fields.
    let template =
        "{prompt}|{project}|{schedule}|{cronLine}|{timezone}|{nextRunAt}|{lastRunAt}|{runCount}";
    // Act: fill the template.
    let rendered = render_task_details(
        template, "prompt", "project", "schedule", "cron", "tz", "next", "last", "3",
    );
    // Assert: every placeholder is substituted.
    assert_eq!(rendered, "prompt|project|schedule|cron|tz|next|last|3");
}

// ---- task detail helpers (4) ----

#[test]
fn task_model_label_appends_variant_when_set_parity() {
    // Arrange: provider/model plus a variant.
    // Act + Assert: `providerID/modelID (variant)`.
    assert_eq!(
        format_task_model_label("anthropic", "claude", Some("plan")),
        "anthropic/claude (plan)"
    );
}

#[test]
fn task_model_label_omits_variant_when_missing_or_empty_parity() {
    // Arrange: no variant (or an empty one).
    // Act + Assert: bare `providerID/modelID`.
    assert_eq!(format_task_model_label("a", "b", None), "a/b");
    assert_eq!(format_task_model_label("a", "b", Some("")), "a/b");
}

#[test]
fn task_cron_block_appends_newline_for_cron_only_parity() {
    // Arrange: a resolved cron line.
    // Act + Assert: cron tasks keep `line\n`, others get empty.
    assert_eq!(task_cron_block(true, "cron"), "cron\n");
    assert_eq!(task_cron_block(false, "cron"), "");
}

#[test]
fn task_project_block_joins_worktree_mode_model_parity() {
    // Arrange: resolved worktree/mode/model lines.
    // Act: compose the project block.
    let block = format_task_project_block("wt", "mode", "model");
    // Assert: newline-joined in order.
    assert_eq!(block, "wt\nmode\nmodel");
}

// ---- render_dashboard_goal_line (1) ----

#[test]
fn dashboard_goal_line_substitutes_status_and_objective_parity() {
    // Arrange: the locale-owned `pinned.line.goal` template.
    let template = "[{status}] {objective}";
    // Act: fill status + truncated objective.
    let rendered = render_dashboard_goal_line(template, "active", "ship it");
    // Assert: both placeholders substituted.
    assert_eq!(rendered, "[active] ship it");
}

// ---- select_goal_notice_key (2) ----

#[test]
fn goal_notice_complete_picks_complete_key_parity() {
    // Arrange: a completed goal.
    // Act + Assert: `goal.notice.complete`.
    assert_eq!(select_goal_notice_key(true), "goal.notice.complete");
}

#[test]
fn goal_notice_paused_picks_paused_key_parity() {
    // Arrange: a paused goal.
    // Act + Assert: `goal.notice.paused`.
    assert_eq!(select_goal_notice_key(false), "goal.notice.paused");
}

// ---- resolve_goal_notice_reason (2) ----

#[test]
fn goal_notice_reason_prefers_localized_reason_parity() {
    // Arrange: an already-localized reason plus the no-reason fallback.
    // Act + Assert: the localized reason wins.
    assert_eq!(
        resolve_goal_notice_reason(Some("done"), "no reason"),
        "done"
    );
}

#[test]
fn goal_notice_reason_falls_back_to_no_reason_line_parity() {
    // Arrange: no reason on the notice.
    // Act + Assert: the resolved `goal.notice.no_reason` line.
    assert_eq!(resolve_goal_notice_reason(None, "no reason"), "no reason");
}

// ---- objective_too_long_params (2) ----

#[test]
fn objective_too_long_params_carries_shared_limit_parity() {
    // Arrange: any over-limit length.
    // Act: pack the params.
    let params = objective_too_long_params(GOAL_OBJECTIVE_LIMIT + 1);
    // Assert: the shared server limit travels as `limit`.
    assert_eq!(params.limit, GOAL_OBJECTIVE_LIMIT);
    assert_eq!(params.limit, 4000);
}

#[test]
fn objective_too_long_params_echoes_actual_length_as_count_parity() {
    // Arrange: a concrete over-limit length.
    // Act: pack the params.
    let params = objective_too_long_params(4001);
    // Assert: the actual length travels as `count`.
    assert_eq!(params.count, 4001);
}

// ---- session-ignore guards ----

fn ignore(session_id: &str, created_at_ms: Option<i64>) -> SessionIgnore {
    SessionIgnore {
        session_id: session_id.to_string(),
        created_at_ms,
    }
}

#[test]
fn fresh_ignore_requires_parsed_date_within_ttl_parity() {
    // Arrange: a row created just now.
    // Act + Assert: fresh.
    assert!(is_fresh_ignore(Some(1_000), 1_001));
}

#[test]
fn fresh_ignore_rejects_missing_date_parity() {
    // Arrange: a row with missing/unparseable `createdAt` (TS NaN).
    // Act + Assert: never fresh.
    assert!(!is_fresh_ignore(None, 1_000));
}

#[test]
fn fresh_ignore_rejects_expired_row_parity() {
    // Arrange: a row older than the TTL.
    // Act + Assert: stale.
    assert!(!is_fresh_ignore(
        Some(0),
        SCHEDULED_TASK_SESSION_IGNORE_TTL_MS + 1
    ));
}

#[test]
fn prune_expired_ignores_keeps_order_and_drops_stale_parity() {
    // Arrange: fresh, expired, and missing-date rows.
    let now_ms = 10_000;
    let ignores = vec![
        ignore("fresh", Some(now_ms)),
        ignore(
            "stale",
            Some(now_ms - SCHEDULED_TASK_SESSION_IGNORE_TTL_MS - 1),
        ),
        ignore("nodate", None),
    ];
    // Act: prune at now.
    let pruned = prune_expired_ignores(&ignores, now_ms);
    // Assert: only the fresh row survives, in order.
    assert_eq!(pruned, vec![ignore("fresh", Some(now_ms))]);
}

#[test]
fn session_ignored_matches_session_only_when_fresh_parity() {
    // Arrange: a fresh ignore for ses-a and a stale one for ses-b.
    let now_ms = 10_000;
    let ignores = vec![
        ignore("ses-a", Some(now_ms)),
        ignore(
            "ses-b",
            Some(now_ms - SCHEDULED_TASK_SESSION_IGNORE_TTL_MS - 1),
        ),
    ];
    // Act + Assert: ses-a ignored, ses-b and unknown sessions not.
    assert!(is_session_ignored(&ignores, "ses-a", now_ms));
    assert!(!is_session_ignored(&ignores, "ses-b", now_ms));
    assert!(!is_session_ignored(&ignores, "ses-c", now_ms));
}

#[test]
fn register_ignore_next_prunes_replaces_and_appends_parity() {
    // Arrange: a stale row, a fresh row for another session, and an old row
    // for the registering session.
    let now_ms = 10_000;
    let ignores = vec![
        ignore(
            "stale",
            Some(now_ms - SCHEDULED_TASK_SESSION_IGNORE_TTL_MS - 1),
        ),
        ignore("other", Some(now_ms)),
        ignore("ses-a", Some(now_ms - 1)),
    ];
    // Act: register ses-a at now.
    let next = register_ignore_next(&ignores, "ses-a", Some(now_ms), now_ms);
    // Assert: stale pruned, old ses-a row replaced, new row appended last.
    assert_eq!(
        next,
        vec![ignore("other", Some(now_ms)), ignore("ses-a", Some(now_ms))]
    );
}

#[test]
fn remove_ignore_next_drops_only_matching_session_parity() {
    // Arrange: ignores for two sessions.
    let ignores = vec![ignore("ses-a", Some(1)), ignore("ses-b", Some(2))];
    // Act: remove ses-a.
    let next = remove_ignore_next(&ignores, "ses-a");
    // Assert: only ses-b remains.
    assert_eq!(next, vec![ignore("ses-b", Some(2))]);
}

#[test]
fn cleanup_ignores_next_returns_pruned_state_and_removed_count_parity() {
    // Arrange: two stale rows and one fresh row.
    let now_ms = 10_000;
    let ignores = vec![
        ignore(
            "stale-a",
            Some(now_ms - SCHEDULED_TASK_SESSION_IGNORE_TTL_MS - 1),
        ),
        ignore("fresh", Some(now_ms)),
        ignore("nodate", None),
    ];
    // Act: cleanup at now.
    let (next, removed) = cleanup_ignores_next(&ignores, now_ms);
    // Assert: fresh row kept, two dropped.
    assert_eq!(next, vec![ignore("fresh", Some(now_ms))]);
    assert_eq!(removed, 2);
}
