//! Goal-manager + rename-flow pure gates (slice 71, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/managers/goal-manager.ts` (`GoalManager`: followed-session
//!   goal, announced-key once-only paused/complete notices, pause-notice
//!   suppression, control-token claim, follow/clear resets, `noticeKey`)
//! - `src/app/managers/rename-manager.ts` (`RenameManager`: rename
//!   interaction payload — start waiting, confirmation message id,
//!   active-message match, waiting/session-info reads, clear)
//! - `src/app/formatters/goal-text.ts` (`formatGoalStatus` status-to-label
//!   select; the `t(...)` lookups stay with the caller, which injects the
//!   already-localized labels)
//!
//! No `randomBytes`, `logger`, `InteractionManager`, or `t(...)` locale
//! lookups live here — only the state machines, key, and label select.
//! Callers wire randomness, persistence, interaction slots, and i18n.
//!
//! DEVIATIONS from TS (behavior-preserving for the covered cases):
//! 1. `randomBytes(4).toString("hex")` control tokens become a hex-rendered
//!    `u64` counter (`0000000000000001`, ...). Tests treat tokens opaquely
//!    (equality, claim validity, rotation), so the port is observationally
//!    identical without an RNG.
//! 2. `RenameManager` delegates to `InteractionManager` in TS. The port
//!    inlines the rename payload (`RenameFlow`) so the flow stays testable
//!    without the interaction slot machine (already ported in
//!    `interaction_state`); the kind/payload contract is unchanged.
//! 3. Telegram message ids are JS `number`s in TS; the port uses `i64`.

/// Goal lifecycle status (TS: `GoalStatus` in `src/app/types/goal.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    Active,
    Paused,
    Complete,
}

/// Terminal notice status (TS: `GoalNotice["status"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalNoticeStatus {
    Paused,
    Complete,
}

/// Server-owned session goal (TS: `GoalState` in `src/app/types/goal.ts`;
/// `rounds`/`maxRounds` ride along for snapshot fidelity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalState {
    pub id: String,
    pub objective: String,
    pub status: GoalStatus,
    pub reason: Option<String>,
    pub rounds: u32,
    pub max_rounds: u32,
    pub updated_at: i64,
}

/// Announced pause/completion (TS: `GoalNotice`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalNotice {
    pub status: GoalNoticeStatus,
    pub reason: Option<String>,
}

/// Followed-session goal tracker (TS: `GoalManager`).
pub struct GoalManager {
    session_id: Option<String>,
    goal: Option<GoalState>,
    announced_key: Option<String>,
    pause_notice_suppressed: bool,
    control_token: String,
    token_counter: u64,
}

impl GoalManager {
    /// Empty tracker (TS: field initializers; token starts rotated once).
    pub fn new() -> Self {
        let mut mgr = Self {
            session_id: None,
            goal: None,
            announced_key: None,
            pause_notice_suppressed: false,
            control_token: String::new(),
            token_counter: 0,
        };
        mgr.rotate_token();
        mgr
    }

    fn rotate_token(&mut self) {
        self.token_counter = self.token_counter.wrapping_add(1);
        self.control_token = format!("{:016x}", self.token_counter);
    }

    /// Current control token (TS: `getControlToken`).
    pub fn control_token(&self) -> &str {
        &self.control_token
    }

    /// Claims single-flight control; rotates the token on success
    /// (TS: `claimControl`).
    pub fn claim_control(&mut self, token: &str) -> bool {
        if token != self.control_token || self.goal.is_none() || self.session_id.is_none() {
            return false;
        }
        self.rotate_token();
        true
    }

    /// Drops session, goal, announced key, and suppression (TS: `clear`).
    pub fn clear(&mut self) {
        self.session_id = None;
        self.goal = None;
        self.announced_key = None;
        self.pause_notice_suppressed = false;
        self.rotate_token();
    }

    /// Current followed session and goal (TS: `snapshot`).
    pub fn snapshot(&self) -> (Option<&str>, Option<&GoalState>) {
        (self.session_id.as_deref(), self.goal.as_ref())
    }

    /// Follows a new session, resetting goal state; same session is a no-op
    /// (TS: `follow`).
    pub fn follow(&mut self, session_id: &str) {
        if self.session_id.as_deref() == Some(session_id) {
            return;
        }
        self.session_id = Some(session_id.to_string());
        self.goal = None;
        self.announced_key = None;
        self.pause_notice_suppressed = false;
        self.rotate_token();
    }

    /// Swallows the next paused notice (TS: `suppressPauseNotice`).
    pub fn suppress_pause_notice(&mut self) {
        self.pause_notice_suppressed = true;
    }

    /// Releases a pending pause suppression (TS: `clearPauseSuppression`).
    pub fn clear_pause_suppression(&mut self) {
        self.pause_notice_suppressed = false;
    }

    /// Stores a read/change for the followed session; announces a pause or
    /// completion once when asked (TS: `apply`).
    pub fn apply(
        &mut self,
        session_id: &str,
        goal: Option<GoalState>,
        announce: bool,
    ) -> Option<GoalNotice> {
        if self.session_id.as_deref() != Some(session_id) {
            return None;
        }

        let control_changed = match (&goal, &self.goal) {
            (None, None) => false,
            (None, Some(_)) | (Some(_), None) => true,
            (Some(next), Some(current)) => {
                next.id != current.id
                    || next.status != current.status
                    || next.objective != current.objective
            }
        };
        if control_changed {
            self.rotate_token();
        }
        self.goal = goal;
        let current = self.goal.as_ref();
        let is_terminal = matches!(
            current.map(|g| g.status),
            Some(GoalStatus::Paused) | Some(GoalStatus::Complete)
        );
        if !is_terminal {
            self.announced_key = None;
            return None;
        }
        let current = current.expect("terminal goal is present");
        let key = notice_key(current);
        if !announce || self.announced_key.as_deref() == Some(&key) {
            self.announced_key = Some(key);
            return None;
        }
        self.announced_key = Some(key);
        if current.status == GoalStatus::Paused && self.pause_notice_suppressed {
            self.pause_notice_suppressed = false;
            return None;
        }
        Some(GoalNotice {
            status: match current.status {
                GoalStatus::Paused => GoalNoticeStatus::Paused,
                GoalStatus::Complete => GoalNoticeStatus::Complete,
                GoalStatus::Active => unreachable!("active goals return early"),
            },
            reason: current.reason.clone(),
        })
    }

    /// Records a paused/complete goal as already announced
    /// (TS: `markAnnounced`).
    pub fn mark_announced(&mut self, goal: &GoalState) {
        if matches!(goal.status, GoalStatus::Paused | GoalStatus::Complete) {
            self.announced_key = Some(notice_key(goal));
        }
    }
}

impl Default for GoalManager {
    fn default() -> Self {
        Self::new()
    }
}

fn goal_status_slug(status: GoalStatus) -> &'static str {
    match status {
        GoalStatus::Active => "active",
        GoalStatus::Paused => "paused",
        GoalStatus::Complete => "complete",
    }
}

/// Announced-notice dedupe key (TS: `noticeKey`).
pub fn notice_key(goal: &GoalState) -> String {
    format!(
        "{}:{}:{}",
        goal.id,
        goal_status_slug(goal.status),
        goal.updated_at
    )
}

/// Status-to-label select (TS: `formatGoalStatus`; labels are injected
/// pre-localized).
pub fn select_goal_status_label<'a>(
    status: GoalStatus,
    paused_label: &'a str,
    complete_label: &'a str,
    active_label: &'a str,
) -> &'a str {
    match status {
        GoalStatus::Paused => paused_label,
        GoalStatus::Complete => complete_label,
        GoalStatus::Active => active_label,
    }
}

/// Pending rename interaction (TS: `RenameState` payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePayload {
    pub session_id: String,
    pub directory: String,
    pub current_title: String,
    pub message_id: Option<i64>,
}

/// Rename waiting-room over its payload (TS: `RenameManager` core).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenameFlow {
    state: Option<RenamePayload>,
}

impl RenameFlow {
    /// Idle flow (TS: no active rename interaction).
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Opens the rename wait for a session (TS: `startWaiting`).
    pub fn start_waiting(&mut self, session_id: &str, directory: &str, current_title: &str) {
        self.state = Some(RenamePayload {
            session_id: session_id.to_string(),
            directory: directory.to_string(),
            current_title: current_title.to_string(),
            message_id: None,
        });
    }

    /// Records the confirmation message (TS: `setMessageId`; no-op idle).
    pub fn set_message_id(&mut self, message_id: i64) {
        if let Some(state) = self.state.as_mut() {
            state.message_id = Some(message_id);
        }
    }

    /// Confirmation message, if any (TS: `getMessageId`).
    pub fn message_id(&self) -> Option<i64> {
        self.state.as_ref().and_then(|state| state.message_id)
    }

    /// Whether the message carries the active rename wait
    /// (TS: `isActiveMessage`).
    pub fn is_active_message(&self, message_id: Option<i64>) -> bool {
        match (self.message_id(), message_id) {
            (Some(active), Some(candidate)) => active == candidate,
            _ => false,
        }
    }

    /// Whether a rename name is awaited (TS: `isWaitingForName`).
    pub fn is_waiting_for_name(&self) -> bool {
        self.state.is_some()
    }

    /// Session, directory, and title under rename, if any
    /// (TS: `getSessionInfo`).
    pub fn session_info(&self) -> Option<(&str, &str, &str)> {
        let state = self.state.as_ref()?;
        if state.session_id.is_empty() {
            return None;
        }
        Some((
            state.session_id.as_str(),
            state.directory.as_str(),
            state.current_title.as_str(),
        ))
    }

    /// Drops the rename wait (TS: `clear`).
    pub fn clear(&mut self) {
        self.state = None;
    }
}
