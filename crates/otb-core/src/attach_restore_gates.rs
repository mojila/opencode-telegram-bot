//! Attach-restore orchestration pure gates (slice 103, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/attach-service.ts`
//!   (`attachToSession` restore gate: `(!alreadyAttached ||
//!   forceFullRestore) && !questionManager.isActive() &&
//!   !permissionManager.isActive()`; `busy ? markBusy : markIdle` mark
//!   choice; `restorePendingQuestions` show-vs-wait gate
//!   (`!isActive() && startQuestions(...)` shows, otherwise waits);
//!   `restorePendingPermissions` question-active wait branch;
//!   `listPendingQuestions` / `listPendingPermissions` error-or-no-data
//!   skip; `belongsToFollowedSession` missing-chain gate;
//!   `isQuestionTracked` shown-or-waiting OR gate;
//!   `isPermissionTracked` has-or-resolved-or-waiting OR gate;
//!   `settleRequestsGoneWhileDisconnected` waiting-drop gate
//!   (`!pendingIds.has(...)` drops), shown-question close gate
//!   (`shownId && !pending.has && current matches &&
//!   !isSettlingFromTelegram() && presentation`), restart-vs-outside
//!   close choice, shown-permission settle gate
//!   (`!pending.has && settleable.has(...)`) plus the
//!   `changes.length > 0 && presentation` apply gate;
//!   `restoreAttachedCurrentSession` in-flight join gate and
//!   failed-join retry gate;
//!   `performAttachedCurrentSessionRestore` missing-project/session
//!   skip and session/project directory-mismatch skip;
//!   `dropSavedSessionIfMissing` not-found drop gate;
//!   `restorePendingInteractionsAfterReconnect` detached skip and
//!   restart-only missed-turn gate; `startMissedLiveTurn`
//!   no-error-plus-busy gate; `detachAttachedSession` detached skip;
//!   `markAttachedSessionBusy` / `markAttachedSessionIdle` unchanged-mark
//!   skip; `syncPinnedAttachState` missing-presentation skip)
//!
//! Already ported elsewhere and NOT repeated here:
//! - `getAttachBusyStatus` (only `"busy"` counts; see
//!   `attach_foreground_gates`).
//! - live question/permission flow and event-handler dispatch predicates
//!   (see `question_permission_flow_gates`, `event_handler_gates`); this
//!   slice owns only the restore/reconnect orchestration gates above.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `Bot`/`Api` I/O, `opencodeClient.session.*` /
//!   `question.list` / `permission.list` calls, `attachManager` /
//!   `questionManager` / `permissionManager` / `interactionManager` /
//!   `summaryAggregator` / `goalManager` / `goalService` /
//!   `pinnedMessageManager` mutation and reads (snapshots arrive as
//!   explicit bools/ids), `resolveSessionParentChain` /
//!   `ensureEventSubscription` / `isOpencodeServerHealthy` awaits,
//!   `setTimeout`/`Promise.all` waits, `Date.now()` clocks,
//!   `getCurrentProject` / `getCurrentSession` / `setCurrentProject` /
//!   `clearSession` reads/writes, `resetStreamThrottle`, `logger`.
//!
//! Deviations: none intended. Manager snapshots arrive as explicit bools
//! (`question_active`, `permission_active`, `shown_matches`,
//! `waiting_or_releasing`, `has_request`, `is_resolved`,
//! `current_matches`, `settling_from_telegram`, `still_settleable`),
//! `attachPresentation` null-ness arrives as `has_presentation`, pending-id
//! membership arrives as precomputed `pending_has*` bools, and the
//! `chain` null-ness / `error` presence arrive as `has_chain` /
//! `has_error` bools, so every gate stays pure and testable on every host.
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity tests in
//! `crates/otb-core/tests/slice103_attach_restore_parity.rs` assert this
//! contract.

/// `true` when the attach loads pending questions and permissions
/// (TS: `(!alreadyAttached || forceFullRestore) &&
/// !questionManager.isActive() && !permissionManager.isActive()`).
pub fn should_restore_pending(
    already_attached: bool,
    force_full_restore: bool,
    question_active: bool,
    permission_active: bool,
) -> bool {
    (!already_attached || force_full_restore) && !question_active && !permission_active
}

/// `true` when a restored poll is shown instead of waiting
/// (TS: `!questionManager.isActive() && startQuestions(...)`).
pub fn should_show_restored_question(question_active: bool, started: bool) -> bool {
    !question_active && started
}

/// `true` when a restored permission waits behind the on-screen question
/// (TS: `if (questionManager.isActive()) waitPermission(...)`).
pub fn should_wait_restored_permission(question_active: bool) -> bool {
    question_active
}

/// `true` when a pending-list load is skipped as failed
/// (TS: `if (error || !data)` in `listPendingQuestions` /
/// `listPendingPermissions`).
pub fn should_skip_pending_restore(has_error: bool, has_data: bool) -> bool {
    has_error || !has_data
}

/// `true` when a pending request belongs to the followed session
/// (TS: `if (!chain) return false` in `belongsToFollowedSession`).
pub fn is_followed_request(has_chain: bool) -> bool {
    has_chain
}

/// `true` for a poll already on screen or waiting, so a restore must not
/// show it again (TS: `getRequestID() === requestID ||
/// isWaitingOrReleasing(requestID)` in `isQuestionTracked`).
pub fn is_question_tracked(shown_matches: bool, waiting_or_releasing: bool) -> bool {
    shown_matches || waiting_or_releasing
}

/// `true` for a permission already on screen, settled or waiting, so a
/// restore must not show it again (TS: `hasRequest(...) ||
/// isResolved(...) || isWaitingOrReleasing(...)` in
/// `isPermissionTracked`).
pub fn is_permission_tracked(
    has_request: bool,
    is_resolved: bool,
    waiting_or_releasing: bool,
) -> bool {
    has_request || is_resolved || waiting_or_releasing
}

/// `true` when a waiting request tracked before the reconnect left
/// OpenCode's pending lists and leaves the queue
/// (TS: `if (!pendingIds.has(requestID)) drop...`).
pub fn should_drop_waiting_while_gone(pending_has: bool) -> bool {
    !pending_has
}

/// `true` when the on-screen poll is closed because OpenCode no longer has
/// it pending (TS: `shownId && !pendingIds.has(shownId) &&
/// getRequestID() === shownId && !isSettlingFromTelegram() &&
/// attachPresentation`).
pub fn should_close_shown_question(
    has_shown: bool,
    pending_has_shown: bool,
    current_matches: bool,
    settling_from_telegram: bool,
    has_presentation: bool,
) -> bool {
    has_shown
        && !pending_has_shown
        && current_matches
        && !settling_from_telegram
        && has_presentation
}

/// `true` when the closed poll ends as not answered (a server restart took
/// it with it) rather than as settled outside Telegram
/// (TS: `serverRestarted ? closeQuestionNotAnswered :
/// closeQuestionSettledOutside`).
pub fn should_close_question_as_not_answered(server_restarted: bool) -> bool {
    server_restarted
}

/// `true` when an on-screen permission gone from the pending lists is
/// settled locally (TS: `!pendingIds.has(requestID) &&
/// settleable.has(requestID)`).
pub fn should_settle_shown_permission(pending_has: bool, still_settleable: bool) -> bool {
    !pending_has && still_settleable
}

/// `true` when settled-permission changes are drawn
/// (TS: `if (changes.length > 0 && attachPresentation)`).
pub fn should_apply_permission_settle_changes(
    change_count: usize,
    has_presentation: bool,
) -> bool {
    change_count > 0 && has_presentation
}

/// `true` when a new restore joins the one already in flight
/// (TS: `if (restoreInFlight)` in `restoreAttachedCurrentSession`).
pub fn should_join_restore(has_inflight: bool) -> bool {
    has_inflight
}

/// `true` when a caller retries with its own attempt after the joined
/// restore failed and nobody else retried yet
/// (TS: `if (joined) return true; if (restoreInFlight) join; retry`).
pub fn should_retry_joined_restore(joined_ok: bool, still_inflight: bool) -> bool {
    !joined_ok && !still_inflight
}

/// `true` when the startup restore is skipped for lack of saved state
/// (TS: `if (!currentProject || !currentSession) return false`).
pub fn should_skip_startup_restore(has_project: bool, has_session: bool) -> bool {
    !has_project || !has_session
}

/// `true` when the startup restore is skipped on a project/session
/// directory mismatch (TS: `currentSession.directory !==
/// currentProject.worktree`).
pub fn should_skip_restore_on_mismatch(session_dir: &str, project_dir: &str) -> bool {
    session_dir != project_dir
}

/// `true` when a saved session missing from the server stops being current
/// (TS: `if (!isOpencodeNotFoundError(error)) return false` in
/// `dropSavedSessionIfMissing`).
pub fn should_drop_missing_session(is_not_found: bool) -> bool {
    is_not_found
}

/// `true` when the reconnect restore is skipped while detached
/// (TS: `if (!attached) return;` in
/// `restorePendingInteractionsAfterReconnect`).
pub fn should_skip_reconnect_restore(has_attached: bool) -> bool {
    !has_attached
}

/// `true` when the reconnect also restarts the missed live turn (only a
/// reconnect to another server process resumes a turn unseen)
/// (TS: `serverRestarted ? startMissedLiveTurn(...) : undefined`).
pub fn should_call_start_missed_live_turn(server_restarted: bool) -> bool {
    server_restarted
}

/// `true` when a busy session starts its missed turn from now
/// (TS: no `error` plus `getAttachBusyStatus(...)` in
/// `startMissedLiveTurn`).
pub fn should_start_missed_live_turn(has_error: bool, busy: bool) -> bool {
    !has_error && busy
}

/// `true` when a detach is skipped while nothing is attached
/// (TS: `if (!attachManager.isAttached()) return;`).
pub fn should_skip_detach(is_attached: bool) -> bool {
    !is_attached
}

/// `true` when the pinned attach state is synced after a busy/idle mark
/// changed it (TS: `if (!markBusy(...) / !markIdle(...)) return;`).
pub fn should_sync_attach_state(mark_changed: bool) -> bool {
    mark_changed
}

/// `true` when the pinned attach state sync runs at all
/// (TS: `if (!attachPresentation) return;` in `syncPinnedAttachState`).
pub fn should_sync_pinned_attach(has_presentation: bool) -> bool {
    has_presentation
}

/// `true` when the attached session is marked busy (otherwise it is marked
/// idle) after its status loads (TS: `if (busy) markBusy(...) else
/// markIdle(...)` in `attachToSession`).
pub fn should_mark_session_busy(busy: bool) -> bool {
    busy
}
