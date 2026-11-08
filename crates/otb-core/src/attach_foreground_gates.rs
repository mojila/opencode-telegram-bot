//! Attach/foreground pure gates (slice 68, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/managers/attach-manager.ts` (`AttachManager`: single
//!   attached session, busy/idle guards, snapshot copies)
//! - `src/app/managers/foreground-session-state-manager.ts`
//!   (`ForegroundSessionState`: busy map, copies, `clearAll` early return)
//! - `src/app/managers/prompt-attachment-manager.ts`
//!   (`PromptAttachmentManager`: single pending file, confirmation id,
//!   copy-on-read)
//! - `src/app/services/attach-service.ts` (`getAttachBusyStatus`: only a
//!   `"busy"` status counts)
//! - `src/app/services/run-control-service.ts` (`isForegroundBusy` or,
//!   `getBusyDirectories` dedup)
//! - `src/app/services/prompt-attachment-service.ts` (`toRelativePath`:
//!   relative unless empty or escaping)
//! - `src/app/services/busy-reconciliation-service.ts` (pure gates:
//!   `getSessionStatus` miss, `isTerminalStatus`, foreground grace window)
//!
//! Pure sync data-structure logic only: no Telegram I/O, no network/fetch,
//! no timers, no `logger` calls. `reason` arguments only feed TS
//! `logger.info`, so they are dropped here (`_reason`).
//!
//! Deviations from TS (all behavior-preserving for the covered cases):
//! - `get_attach_busy_status(statuses, session_id)` takes the status map
//!   first (TS order is `(sessionId, statuses)`). The map value is
//!   `Option<String>`: `None` models a status entry with a missing
//!   `type` field.
//! - `is_foreground_busy` / `get_busy_directories` take plain values
//!   instead of the TS `AppContainer` dependency picks (no DI container in
//!   this port).
//! - `ForegroundSessionState::mark_busy` takes an explicit `now_ms`
//!   instead of calling `Date.now()` internally.
//! - `is_within_foreground_busy_grace_period(marked_at, now_ms)` takes the
//!   two timestamps instead of the session object, and uses saturating
//!   subtraction (a `now_ms` before `marked_at` yields `0 < grace`, the
//!   same `true` TS computes with a negative difference).
//! - `to_relative_path` is lexical (`Path::strip_prefix`) without Node
//!   `path.relative` normalization of `.`/`..` segments; separators follow
//!   `std::path` (same visible result on POSIX).
//! - `is_terminal_status(Option<&str>)` has no equivalent of an entry with
//!   `type: undefined`; any `Some` other than `"idle"`/`"error"` is
//!   non-terminal, matching TS.
//! - `busy_sessions()` is sorted by `session_id` for determinism (TS
//!   returns `Map` insertion order; Rust `HashMap` has none).
//! - `is_attached_session` treats `Some("")` for `directory` like `None`
//!   (TS `if (directory && ...)` also skips the empty string).
//! - `PromptAttachmentManager` is an instantiable struct here while TS
//!   exports a singleton; `set` still drops a previous confirmation id,
//!   matching the TS reassignment.

use std::collections::HashMap;

/// Minimum gap between reconciliations
/// (TS: `RECONCILE_MIN_INTERVAL_MS`).
pub const RECONCILE_MIN_INTERVAL_MS: u64 = 10_000;

/// Freshly marked foreground sessions are skipped by reconciliation
/// (TS: `FOREGROUND_BUSY_RECONCILE_GRACE_MS`).
pub const FOREGROUND_BUSY_RECONCILE_GRACE_MS: u64 = 2_000;

/// Attached session snapshot (TS: `AttachedSessionState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedSessionState {
    pub session_id: String,
    pub directory: String,
    pub busy: bool,
}

/// Single attached session (TS: `AttachManager`).
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct AttachManager {
    state: Option<AttachedSessionState>,
}

impl AttachManager {
    /// Creates a detached manager.
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Attaches to a session, clearing the busy flag (TS: `attach`).
    pub fn attach(&mut self, session_id: &str, directory: &str) {
        self.state = Some(AttachedSessionState {
            session_id: session_id.to_string(),
            directory: directory.to_string(),
            busy: false,
        });
    }

    /// Clears the attached session (TS: `clear`; log-only `reason` dropped).
    pub fn clear(&mut self, _reason: &str) {
        if self.state.is_none() {
            return;
        }
        self.state = None;
    }

    /// Snapshot copy of the attached session (TS: `getSnapshot`).
    pub fn snapshot(&self) -> Option<AttachedSessionState> {
        self.state.clone()
    }

    /// Whether any session is attached (TS: `isAttached`).
    pub fn is_attached(&self) -> bool {
        self.state.is_some()
    }

    /// Whether `session_id` is the attached session, optionally also
    /// requiring `directory` (TS: `isAttachedSession`).
    pub fn is_attached_session(
        &self,
        session_id: Option<&str>,
        directory: Option<&str>,
    ) -> bool {
        let state = match &self.state {
            Some(state) => state,
            None => return false,
        };
        let session_id = match session_id {
            Some(id) if !id.is_empty() => id,
            _ => return false,
        };
        if state.session_id != session_id {
            return false;
        }
        if let Some(dir) = directory {
            if !dir.is_empty() && state.directory != dir {
                return false;
            }
        }
        true
    }

    /// Whether the attached session is busy (TS: `isBusy`).
    pub fn is_busy(&self) -> bool {
        matches!(&self.state, Some(state) if state.busy)
    }

    /// Marks the attached session busy; false when detached, foreign, or
    /// already busy (TS: `markBusy`).
    pub fn mark_busy(&mut self, session_id: &str) -> bool {
        match &mut self.state {
            Some(state) if state.session_id == session_id && !state.busy => {
                state.busy = true;
                true
            }
            _ => false,
        }
    }

    /// Marks the attached session idle; false when detached, foreign, or
    /// already idle (TS: `markIdle`).
    pub fn mark_idle(&mut self, session_id: &str) -> bool {
        match &mut self.state {
            Some(state) if state.session_id == session_id && state.busy => {
                state.busy = false;
                true
            }
            _ => false,
        }
    }
}

/// A busy foreground session (TS: `ForegroundBusySession`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundBusySession {
    pub session_id: String,
    pub directory: String,
    pub marked_at: u64,
}

/// Foreground busy-session map (TS: `ForegroundSessionState`).
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct ForegroundSessionState {
    sessions: HashMap<String, ForegroundBusySession>,
}

impl ForegroundSessionState {
    /// Creates an empty state.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Marks a session busy; ignores empty ids/directories
    /// (TS: `markBusy`; `now_ms` stands in for `Date.now()`).
    pub fn mark_busy(&mut self, session_id: &str, directory: &str, now_ms: u64) {
        if session_id.is_empty() || directory.is_empty() {
            return;
        }
        self.sessions.insert(
            session_id.to_string(),
            ForegroundBusySession {
                session_id: session_id.to_string(),
                directory: directory.to_string(),
                marked_at: now_ms,
            },
        );
    }

    /// Marks a session idle; ignores empty ids (TS: `markIdle`).
    pub fn mark_idle(&mut self, session_id: &str) {
        if session_id.is_empty() {
            return;
        }
        self.sessions.remove(session_id);
    }

    /// Copy of the busy sessions (TS: `getBusySessions`).
    pub fn busy_sessions(&self) -> Vec<ForegroundBusySession> {
        let mut out: Vec<ForegroundBusySession> = self.sessions.values().cloned().collect();
        out.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        out
    }

    /// Whether any session is busy (TS: `isBusy`).
    pub fn is_busy(&self) -> bool {
        !self.sessions.is_empty()
    }

    /// Clears all busy state (TS: `clearAll`; log-only `reason` dropped).
    pub fn clear_all(&mut self, _reason: &str) {
        if self.sessions.is_empty() {
            return;
        }
        self.sessions.clear();
    }

    /// Test hook overwriting `markedAt` (TS: `__setMarkedAtForTests`).
    pub fn set_marked_at_for_tests(&mut self, session_id: &str, marked_at: u64) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.marked_at = marked_at;
        }
    }
}

/// File picked in /ls until the next prompt consumes it
/// (TS: `PendingAttachment`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAttachment {
    pub absolute_path: String,
    pub worktree: String,
    pub confirmation_message_id: Option<i64>,
}

/// Single pending file holder (TS: `PromptAttachmentManager`).
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct PromptAttachmentManager {
    state: Option<PendingAttachment>,
}

impl PromptAttachmentManager {
    /// Creates an empty holder.
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Stores the picked file (TS: `set`).
    pub fn set(&mut self, absolute_path: &str, worktree: &str) {
        self.state = Some(PendingAttachment {
            absolute_path: absolute_path.to_string(),
            worktree: worktree.to_string(),
            confirmation_message_id: None,
        });
    }

    /// Remembers the confirmation message; noop when empty
    /// (TS: `setConfirmationMessageId`).
    pub fn set_confirmation_message_id(&mut self, message_id: i64) {
        if let Some(state) = &mut self.state {
            state.confirmation_message_id = Some(message_id);
        }
    }

    /// Copy of the pending attachment (TS: `get`).
    pub fn get(&self) -> Option<PendingAttachment> {
        self.state.clone()
    }

    /// Clears the pending attachment (TS: `clear`; log-only `reason`
    /// dropped).
    pub fn clear(&mut self, _reason: &str) {
        if self.state.is_none() {
            return;
        }
        self.state = None;
    }

    /// Restores the empty state (TS: `__resetForTests`).
    pub fn reset_for_tests(&mut self) {
        self.state = None;
    }
}

/// Only a `"busy"` status counts (TS: `getAttachBusyStatus`).
pub fn get_attach_busy_status(
    statuses: Option<&HashMap<String, Option<String>>>,
    session_id: &str,
) -> bool {
    matches!(
        statuses
            .and_then(|map| map.get(session_id))
            .and_then(|status| status.as_deref()),
        Some("busy")
    )
}

/// Busy when either side is busy (TS: `isForegroundBusy`).
pub fn is_foreground_busy(foreground_busy: bool, attach_busy: bool) -> bool {
    foreground_busy || attach_busy
}

/// Directories with busy state, foreground first then attached, deduped
/// (TS: `getBusyDirectories`).
pub fn get_busy_directories(
    foreground: &[ForegroundBusySession],
    attached: Option<&AttachedSessionState>,
) -> Vec<String> {
    let mut directories: Vec<String> = Vec::new();
    for session in foreground {
        if !directories.contains(&session.directory) {
            directories.push(session.directory.clone());
        }
    }
    if let Some(attached) = attached {
        if attached.busy && !directories.contains(&attached.directory) {
            directories.push(attached.directory.clone());
        }
    }
    directories
}

/// Path relative to the worktree unless empty or escaping
/// (TS: `toRelativePath`).
pub fn to_relative_path(absolute_path: &str, worktree: &str) -> String {
    use std::path::Path;
    match Path::new(absolute_path).strip_prefix(Path::new(worktree)) {
        Ok(relative) => {
            let relative = relative.to_string_lossy().into_owned();
            if relative.is_empty() || relative.starts_with("..") {
                absolute_path.to_string()
            } else {
                relative
            }
        }
        Err(_) => absolute_path.to_string(),
    }
}

/// Missing, `"idle"`, and `"error"` statuses are terminal
/// (TS: `isTerminalStatus`).
pub fn is_terminal_status(status_type: Option<&str>) -> bool {
    matches!(status_type, None | Some("idle") | Some("error"))
}

/// Freshly marked sessions are skipped by reconciliation
/// (TS: `isWithinForegroundBusyGracePeriod`).
pub fn is_within_foreground_busy_grace_period(marked_at: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(marked_at) < FOREGROUND_BUSY_RECONCILE_GRACE_MS
}
