//! Minimal attach + event-subscription live path (thin wiring only).
//!
//! TS parity:
//! - `src/app/services/attach-service.ts:327-344` (`attachToSession` chain:
//!   follow, prepare, `ensureEventSubscription`, `setSession`, attach, show).
//! - `src/bot/services/event-subscription-service.ts:142-184`
//!   (`ensureEventSubscription`: empty-directory guard, handler wiring,
//!   `subscribeToEvents` fire-and-forget).
//! - `src/opencode/events.ts:234-495` (reconnect loop; the Rust loop lives in
//!   [`crate::opencode::OpencodeClient::subscribe_events`]).
//! - `src/app/services/event-router.ts:29-37` (`getEventSessionId`:
//!   `properties.sessionID || properties.info?.sessionID ||
//!   properties.part?.sessionID`).
//!
//! Only runs when polling is enabled (`OTB_ENABLE_POLLING=1`); the default
//! stays scaffold-only. Pumps mapped events into a process-wide
//! [`SummaryAggregator`](otb_core::managers::summary_aggregation::SummaryAggregator).
//!
//! The live single interaction slot (TS: `interactionManager` singleton)
//! starts real question/permission holds here: `hold_question_slot` /
//! `hold_permission_slot` open the slot plus the pending request identity the
//! answer branch needs, and `release_interaction_slot` closes both once
//! OpenCode resolves the request (accepted or already-settled `gone`).
//! Plain-text answers route while a question hold is live (see
//! `held_question`); anything else held keeps blocking prompts per the
//! PRODUCT.md single-slot rule.
//!
//! ASSUMPTIONS (explicit):
//! 1. V2 tool-name normalization (`shell` -> `bash`, `subagent` -> `task`,
//!    `patch` -> `apply_patch` via
//!    [`otb_core::v2_mappers::to_v1_tool_name`]) is idempotent over V1 names
//!    (V1 `bash`/`task`/`apply_patch` pass through unchanged), so the pump
//!    applies it unconditionally instead of branching on server version.
//! 2. The task-call agent fallback chain (`subagent_type`, then `agent` /
//!    `subagent` / `type`) mirrors
//!    [`otb_core::v2_mappers::to_v1_tool_input`] precedence for `task` input.
//! 3. A directory switch aborts the previous stream task (TS:
//!    `streamAbortController.abort()` on directory change). The old task's
//!    in-flight callback, if any, still maps into the aggregator, which
//!    filters by current session.
//!
//! Still deferred (dispatcher follow-ups, no new deps): reconnect run-ending
//! (`endRunLostWithServer` / `forgetLiveTurn` / inbox reconciliation),
//! goal-service follow/refresh, server-side session-cache chain resolve,
//! pending question/permission list reload, missed-turn start, main-keyboard
//! refresh, and Telegram sends from the pump (the aggregator tracks state;
//! poll/permission card draws stay behind the telegram-ux hooks
//! [`set_poll_card_hook`] / [`set_permission_card_hook`], and the idle
//! footer behind [`set_session_idle_footer_hook`]; completed-run hydration
//! for that footer stays a dispatcher follow-up).
//! The pump triages every reconnect through [`reconnect_restore_plan`] and
//! [`should_restore_pending_now`] (trace-only; the slot stays the single
//! gate), records the mark via [`mark_reconnect_for_restore`], and registers
//! seen child links via [`ingest_restored_child`]; the dispatcher drains the
//! mark with [`drain_reconnect_restore`] once the session busy state is
//! known.

use std::sync::{Mutex, MutexGuard, OnceLock};

use serde_json::Value;

use otb_core::managers::summary_aggregation::{
    AggregatorEvent, SummaryAggregator, TaskCallInput, ToolPart, ToolStatus,
};

/// The session the live path follows (TS: `attachManager` snapshot subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedSession {
    pub session_id: String,
    pub directory: String,
}

struct AttachRuntime {
    attached: Option<AttachedSession>,
    subscribed_directory: Option<String>,
    subscription: Option<tokio::task::JoinHandle<()>>,
    pending_question: Option<PendingQuestion>,
    pending_permission: Option<PendingPermission>,
    pending_reconnect_restart: Option<bool>,
}

impl AttachRuntime {
    const fn new() -> Self {
        Self {
            attached: None,
            subscribed_directory: None,
            subscription: None,
            pending_question: None,
            pending_permission: None,
            pending_reconnect_restart: None,
        }
    }
}

static RUNTIME: OnceLock<Mutex<AttachRuntime>> = OnceLock::new();

// The live single interaction slot (TS: `interactionManager` singleton).
// Holds open while a question or permission awaits its Telegram answer;
// the prompt/attach busy guard reads it so a held slot blocks double
// prompts per the PRODUCT.md single-slot rule.
static SLOT: OnceLock<Mutex<otb_core::interaction_state::InteractionSlot>> = OnceLock::new();

// The followed-session aggregator. Process-wide behind a `Mutex`: the
// `SummaryAggregator` TS-parity callbacks are `FnMut + Send` (parity tests
// capture via `Arc<Mutex<..>>`), so the SSE pump spawned with
// `tokio::spawn` can share it from any worker thread. Every live-path
// access locks briefly to apply one event; lock failure drops the event.
static AGGREGATOR: OnceLock<Mutex<SummaryAggregator>> = OnceLock::new();

fn aggregator() -> &'static Mutex<SummaryAggregator> {
    AGGREGATOR.get_or_init(|| Mutex::new(SummaryAggregator::new()))
}

fn runtime() -> &'static Mutex<AttachRuntime> {
    RUNTIME.get_or_init(|| Mutex::new(AttachRuntime::new()))
}

fn slot() -> &'static Mutex<otb_core::interaction_state::InteractionSlot> {
    SLOT.get_or_init(|| Mutex::new(otb_core::interaction_state::InteractionSlot::new()))
}

/// Kind held by the single live interaction slot, if any (TS:
/// `interactionManager.getSnapshot()?.kind`).
pub fn interaction_slot_kind() -> Option<String> {
    slot()
        .lock()
        .ok()
        .and_then(|slot| slot.kind().map(str::to_string))
}

/// A question awaiting its Telegram answer (TS: `QuestionManager` request
/// subset — the `requestID` plus the owning session and directory the
/// `question.reply` / `question.reject` calls need).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingQuestion {
    pub request_id: String,
    pub session_id: String,
    pub directory: String,
}

/// Permission requests awaiting one Telegram tap (TS: `PermissionManager`
/// pending subset — every request ID the visible prompt answers together,
/// plus the owning session and directory the `permission.reply` calls
/// need).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPermission {
    pub request_ids: Vec<String>,
    pub session_id: String,
    pub directory: String,
}

/// Hold the single slot for a question (TS: `questionManager.startQuestions`
/// opening the slot plus `syncQuestionInteractionState`). `expected_input`
/// is `mixed`: option taps arrive as callbacks while a custom answer arrives
/// as plain text. Any permission hold is dropped first (single-slot rule).
/// Blank request/session identities never hold.
///
/// Live caller: the SSE pump in [`ensure_event_subscription`] holds here on
/// `question.asked` / V2 `form.created` (prompt rendering itself stays
/// behind [`set_poll_card_hook`).
pub fn hold_question_slot(request_id: &str, session_id: &str, directory: &str) {
    if request_id.trim().is_empty() || session_id.trim().is_empty() {
        tracing::warn!("[otb-bot] attach: refusing blank question hold");
        return;
    }
    if let Ok(mut slot) = slot().lock() {
        slot.start("question", "mixed", None, None);
    }
    if let Ok(mut rt) = runtime().lock() {
        rt.pending_permission = None;
        rt.pending_question = Some(PendingQuestion {
            request_id: request_id.to_string(),
            session_id: session_id.to_string(),
            directory: directory.to_string(),
        });
    }
    tracing::info!("[otb-bot] attach: question slot held");
}

/// Hold the single slot for permission requests (TS:
/// `permissionManager.startPermission` plus `syncPermissionInteractionState`).
/// `expected_input` is `callback`: the answer arrives as a `permission:*`
/// tap. Any question hold is dropped first (single-slot rule). Blank
/// session identities and empty request lists never hold.
///
/// Live caller: the SSE pump in [`ensure_event_subscription`] holds here on
/// `permission.asked` (V1 `properties` or V2 `data` shape); card rendering
/// stays behind [`set_permission_card_hook`].
pub fn hold_permission_slot(request_ids: &[String], session_id: &str, directory: &str) {
    let ids: Vec<String> = request_ids
        .iter()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .collect();
    if session_id.trim().is_empty() || ids.is_empty() {
        tracing::warn!("[otb-bot] attach: refusing blank permission hold");
        return;
    }
    if let Ok(mut slot) = slot().lock() {
        slot.start("permission", "callback", None, None);
    }
    if let Ok(mut rt) = runtime().lock() {
        rt.pending_question = None;
        rt.pending_permission = Some(PendingPermission {
            request_ids: ids,
            session_id: session_id.to_string(),
            directory: directory.to_string(),
        });
    }
    tracing::info!("[otb-bot] attach: permission slot held");
}

/// The question the slot currently holds an answer for, if any.
pub fn held_question() -> Option<PendingQuestion> {
    runtime().lock().ok()?.pending_question.clone()
}

/// The permission requests the slot currently holds taps for, if any.
pub fn held_permission() -> Option<PendingPermission> {
    runtime().lock().ok()?.pending_permission.clone()
}

/// Poll card data prepped from a live `question.asked` / V2 `form.created`
/// envelope (TS: `QuestionRequest` subset — request + owning session +
/// visible question count/headers). Rendering/sending stays with the
/// telegram-ux hook ([`set_poll_card_hook`]); the pump only preps this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollCardData {
    pub request_id: String,
    pub session_id: String,
    pub directory: String,
    pub question_count: usize,
    pub headers: Vec<String>,
}

/// Permission card data prepped from a live `permission.asked` envelope
/// (TS: `PermissionRequest` subset — request IDs + owning session +
/// permission kind + pattern count). Rendering/sending stays with the
/// telegram-ux hook ([`set_permission_card_hook`]); the pump only preps
/// this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionCardData {
    pub request_ids: Vec<String>,
    pub session_id: String,
    pub directory: String,
    pub permission: String,
    pub pattern_count: usize,
}

/// Live poll-card hook (telegram-ux owns rendering/sends; the pump calls it
/// best-effort after holding the slot).
pub type PollCardHook = Box<dyn Fn(PollCardData) + Send>;
/// Live permission-card hook (telegram-ux owns rendering/sends; the pump
/// calls it best-effort after holding the slot).
pub type PermissionCardHook = Box<dyn Fn(PermissionCardData) + Send>;

static POLL_HOOK: OnceLock<Mutex<Option<PollCardHook>>> = OnceLock::new();
static PERM_HOOK: OnceLock<Mutex<Option<PermissionCardHook>>> = OnceLock::new();

fn poll_hook() -> &'static Mutex<Option<PollCardHook>> {
    POLL_HOOK.get_or_init(|| Mutex::new(None))
}

fn perm_hook() -> &'static Mutex<Option<PermissionCardHook>> {
    PERM_HOOK.get_or_init(|| Mutex::new(None))
}

/// Install the telegram-ux poll-card renderer (or `None` to clear).
/// Pure wiring; never fails.
pub fn set_poll_card_hook(hook: Option<PollCardHook>) {
    if let Ok(mut slot) = poll_hook().lock() {
        *slot = hook;
    }
}

/// Install the telegram-ux permission-card renderer (or `None` to clear).
/// Pure wiring; never fails.
pub fn set_permission_card_hook(hook: Option<PermissionCardHook>) {
    if let Ok(mut slot) = perm_hook().lock() {
        *slot = hook;
    }
}

/// Whether a poll-card renderer is currently installed (test observability
/// for the zero-`chat_id` no-op guard in telegram-ux `install_card_hooks`).
pub fn has_poll_card_hook() -> bool {
    poll_hook()
        .lock()
        .ok()
        .map(|hook| hook.is_some())
        .unwrap_or(false)
}

/// Whether a permission-card renderer is currently installed (test
/// observability for the zero-`chat_id` no-op guard).
pub fn has_permission_card_hook() -> bool {
    perm_hook()
        .lock()
        .ok()
        .map(|hook| hook.is_some())
        .unwrap_or(false)
}

fn emit_poll_card(card: PollCardData) {
    let hook = poll_hook().lock().ok().and_then(|mut hook| hook.take());
    if let Some(hook) = hook {
        hook(card.clone());
        if let Ok(mut slot) = poll_hook().lock() {
            *slot = Some(hook);
        }
    }
}

fn emit_permission_card(card: PermissionCardData) {
    let hook = perm_hook().lock().ok().and_then(|mut hook| hook.take());
    if let Some(hook) = hook {
        hook(card.clone());
        if let Ok(mut slot) = perm_hook().lock() {
            *slot = Some(hook);
        }
    }
}

/// Read a non-empty string field under either camelCase or snake_case.
/// Missing/non-string/blank reads as absent.
fn str_field_any(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(str::to_string)
    })
}

fn question_form_node(payload: &Value) -> Option<&Value> {
    payload
        .get("data")
        .and_then(|data| data.get("form"))
        .or_else(|| payload.get("properties"))
}

fn permission_node(payload: &Value) -> Option<&Value> {
    payload
        .get("properties")
        .or_else(|| payload.get("data"))
        .or(Some(payload))
}

/// Detect a live question ask (TS: V1 `question.asked` `properties`
/// `{id, sessionID, questions}` plus V2 `form.created` `data.form`
/// `{id, sessionID, fields/title}` admitted via
/// [`otb_core::v2_mappers::to_v1_question`] field parity). Returns the
/// pending identity the slot holds; card text stays with the poll-card
/// hook. Anything else returns `None`.
pub fn question_asked_from_envelope(event_type: &str, payload: &Value) -> Option<PendingQuestion> {
    match event_type {
        "question.asked" => {
            let props = payload.get("properties")?;
            let request_id = str_field_any(props, &["id", "requestID", "requestId"])?;
            let session_id = str_field_any(props, &["sessionID", "session_id"])?;
            let directory = str_field_any(payload, &["directory"])
                .or_else(|| str_field_any(props, &["directory"]))
                .unwrap_or_default();
            Some(PendingQuestion {
                request_id,
                session_id,
                directory,
            })
        }
        "form.created" => {
            let form = payload
                .get("data")
                .and_then(|data| data.get("form"))
                .or_else(|| payload.get("properties"))?;
            let request_id = str_field_any(form, &["id", "requestID", "requestId"])?;
            let session_id = str_field_any(form, &["sessionID", "session_id"])?;
            let directory = str_field_any(payload, &["directory"])
                .or_else(|| str_field_any(form, &["directory"]))
                .unwrap_or_default();
            Some(PendingQuestion {
                request_id,
                session_id,
                directory,
            })
        }
        _ => None,
    }
}

/// Detect a live permission ask (TS: `permission.asked`; V1 `properties`
/// `{id, sessionID, permission, patterns}` and V2 `data`
/// `{id, sessionID, action, resources}` admitted via
/// [`otb_core::v2_mappers::to_v1_permission`] parity). Returns the pending
/// identity the slot holds; anything else returns `None`.
pub fn permission_asked_from_envelope(
    event_type: &str,
    payload: &Value,
) -> Option<PendingPermission> {
    if event_type != "permission.asked" {
        return None;
    }
    let node = permission_node(payload)?;
    let request_id = str_field_any(node, &["id", "requestID", "requestId"])?;
    let session_id = str_field_any(node, &["sessionID", "session_id"])?;
    let directory = str_field_any(payload, &["directory"])
        .or_else(|| str_field_any(node, &["directory"]))
        .unwrap_or_default();
    Some(PendingPermission {
        request_ids: vec![request_id],
        session_id,
        directory,
    })
}

/// Settle outcome for a live question envelope (TS: `question.replied` /
/// V2 `form.replied` -> answered, `question.rejected` / V2 `form.cancelled`
/// -> cancelled, via [`otb_core::v2_event_translator_gates`] mapping).
/// Returns the owning session, request, and outcome; anything else is `None`.
pub fn question_settled_from_envelope(
    event_type: &str,
    payload: &Value,
) -> Option<(String, String, &'static str)> {
    let (node, outcome) = match event_type {
        "question.replied" => (payload.get("properties")?, "answered"),
        "question.rejected" => (payload.get("properties")?, "cancelled"),
        "form.replied" | "form.cancelled" => {
            let outcome = if event_type == "form.replied" {
                "answered"
            } else {
                "cancelled"
            };
            let node = payload.get("data").or_else(|| payload.get("properties"))?;
            (node, outcome)
        }
        _ => return None,
    };
    let session_id = str_field_any(node, &["sessionID", "session_id"])?;
    let request_id = str_field_any(node, &["requestID", "requestId", "id"])?;
    Some((session_id, request_id, outcome))
}

/// Detect a live permission reply (TS: `permission.replied`
/// `{sessionID, requestID}`; V2 same names via the translator).
/// Returns the owning session, request, and whether it allows (`reject`
/// maps to `false`, every other reply to `true`); anything else is `None`.
pub fn permission_replied_from_envelope(
    event_type: &str,
    payload: &Value,
) -> Option<(String, String, bool)> {
    if event_type != "permission.replied" {
        return None;
    }
    let node = payload.get("properties").or_else(|| payload.get("data"))?;
    let session_id = str_field_any(node, &["sessionID", "session_id"])?;
    let request_id = str_field_any(node, &["requestID", "requestId", "id"])?;
    let reply = node.get("reply").and_then(Value::as_str).unwrap_or("");
    Some((session_id, request_id, reply != "reject"))
}

/// Live `session.idle` signal feeding the assistant-run footer (TS:
/// `summary-aggregation-manager.ts` `isInterruptedIdle` /
/// `readInterruptReason` plus `session-lifecycle-handler.ts:70-91`
/// shutdown/suppress/interrupted-without-response footerless gates).
/// Anything else (heartbeats, `session.error`, aborts) yields `None`:
/// aborts and errors stay footerless because TS `closeForegroundRun` never
/// runs for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionIdleSignal {
    pub session_id: String,
    pub interrupted: bool,
    pub shutdown_idle: bool,
}

/// Parse one live `session.idle` envelope into its footer signal (pure;
/// testable). Reads V1 `properties` (`sessionID`, `interrupted`, `reason`)
/// like the TS idle gate; blank sessions and non-idle types read as absent.
pub fn session_idle_from_envelope(event_type: &str, payload: &Value) -> Option<SessionIdleSignal> {
    if event_type != "session.idle" {
        return None;
    }
    let props = payload.get("properties")?;
    let session_id = str_field_any(props, &["sessionID", "session_id"])?;
    let interrupted = props
        .get("interrupted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let shutdown_idle =
        interrupted && props.get("reason").and_then(Value::as_str) == Some("shutdown");
    Some(SessionIdleSignal {
        session_id,
        interrupted,
        shutdown_idle,
    })
}

/// Live idle-footer hook (telegram-ux owns rendering/sends; the pump calls
/// it best-effort after gating the idle to the followed session).
pub type SessionIdleFooterHook = Box<dyn Fn(SessionIdleSignal) + Send>;

static IDLE_FOOTER_HOOK: OnceLock<Mutex<Option<SessionIdleFooterHook>>> = OnceLock::new();

fn idle_footer_hook() -> &'static Mutex<Option<SessionIdleFooterHook>> {
    IDLE_FOOTER_HOOK.get_or_init(|| Mutex::new(None))
}

/// Install the telegram-ux idle-footer sender (or `None` to clear).
/// Pure wiring; never fails.
pub fn set_session_idle_footer_hook(hook: Option<SessionIdleFooterHook>) {
    if let Ok(mut slot) = idle_footer_hook().lock() {
        *slot = hook;
    }
}

/// Whether an idle-footer sender is currently installed (test observability
/// for the zero-`chat_id` no-op guard in telegram-ux
/// `install_session_idle_footer_hook`).
pub fn has_session_idle_footer_hook() -> bool {
    idle_footer_hook()
        .lock()
        .ok()
        .map(|hook| hook.is_some())
        .unwrap_or(false)
}

fn emit_session_idle_footer(signal: SessionIdleSignal) {
    let hook = idle_footer_hook()
        .lock()
        .ok()
        .and_then(|mut hook| hook.take());
    if let Some(hook) = hook {
        hook(signal.clone());
        if let Ok(mut slot) = idle_footer_hook().lock() {
            *slot = Some(hook);
        }
    }
}

/// Live pinned-dashboard hook (telegram-ux owns render/send/edit; the pump
/// calls it best-effort with a merged snapshot once the SSE/tool-event
/// mapping lands via `telegram::apply_pinned_patch`, so this only exposes
/// the hook — the pump stays a rust-engineer follow-up).
pub type PinnedDashboardHook = Box<dyn Fn(crate::telegram::PinnedDashboardSignal) + Send>;

static PINNED_DASHBOARD_HOOK: OnceLock<Mutex<Option<PinnedDashboardHook>>> = OnceLock::new();

fn pinned_dashboard_hook() -> &'static Mutex<Option<PinnedDashboardHook>> {
    PINNED_DASHBOARD_HOOK.get_or_init(|| Mutex::new(None))
}

/// Install the telegram-ux pinned-dashboard sender (or `None` to clear).
/// Pure wiring; never fails.
pub fn set_pinned_dashboard_hook(hook: Option<PinnedDashboardHook>) {
    if let Ok(mut slot) = pinned_dashboard_hook().lock() {
        *slot = hook;
    }
}

/// Whether a pinned-dashboard sender is currently installed (test
/// observability for the zero-`chat_id` no-op guard in telegram-ux
/// `install_pinned_dashboard_hook`).
pub fn has_pinned_dashboard_hook() -> bool {
    pinned_dashboard_hook()
        .lock()
        .ok()
        .map(|hook| hook.is_some())
        .unwrap_or(false)
}

fn emit_pinned_dashboard(signal: crate::telegram::PinnedDashboardSignal) {
    let hook = pinned_dashboard_hook()
        .lock()
        .ok()
        .and_then(|mut hook| hook.take());
    if let Some(hook) = hook {
        hook(signal);
        if let Ok(mut slot) = pinned_dashboard_hook().lock() {
            *slot = Some(hook);
        }
    }
}

/// Last idle that emitted its footer (TS `idleEmitted` duplicate-`session.idle`
/// collapse, consecutive-signal approximation until the dispatcher owns run
/// state; see [`note_live_activity`]).
static LAST_IDLE_FOOTER: OnceLock<Mutex<Option<SessionIdleSignal>>> = OnceLock::new();

fn last_idle_footer() -> &'static Mutex<Option<SessionIdleSignal>> {
    LAST_IDLE_FOOTER.get_or_init(|| Mutex::new(None))
}

/// Whether `next` repeats the last emitted idle (pure; testable).
fn is_duplicate_idle_signal(last: Option<&SessionIdleSignal>, next: &SessionIdleSignal) -> bool {
    last == Some(next)
}

/// Whether this idle may emit its footer: the first signal for a run emits,
/// an identical back-to-back repeat (duplicate `session.idle` for the same
/// run) does not. [`note_live_activity`] re-arms after real turn events.
fn should_emit_session_idle(signal: &SessionIdleSignal) -> bool {
    let mut last = match last_idle_footer().lock() {
        Ok(last) => last,
        Err(_) => return true,
    };
    if is_duplicate_idle_signal(last.as_ref(), signal) {
        return false;
    }
    *last = Some(signal.clone());
    true
}

/// Re-arm idle emission after real turn activity (TS: a new turn opens a new
/// run, so the next idle emits again). Heartbeats (`Other`) do not re-arm,
/// so duplicates split only by heartbeats stay collapsed.
fn note_live_activity() {
    if let Ok(mut last) = last_idle_footer().lock() {
        last.take();
    }
}

/// Prep poll-card data from a live question envelope (pure; testable).
/// Counts V1 `questions[]` or V2 `fields[]` and carries each header
/// (`header` else `question` else `title` else `key`, blanks dropped).
/// Returns `None` when the envelope is not a question ask.
pub fn poll_card_from_envelope(event_type: &str, payload: &Value) -> Option<PollCardData> {
    let pending = question_asked_from_envelope(event_type, payload)?;
    let form = question_form_node(payload)?;
    let items = form
        .get("questions")
        .or_else(|| form.get("fields"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut headers = Vec::new();
    for item in &items {
        if let Some(header) = str_field_any(item, &["header", "question", "title", "key"]) {
            headers.push(header);
        }
    }
    let fallback_title = str_field_any(form, &["title"]).unwrap_or_default();
    if headers.is_empty() && !fallback_title.is_empty() {
        headers.push(fallback_title);
    }
    Some(PollCardData {
        request_id: pending.request_id,
        session_id: pending.session_id,
        directory: pending.directory,
        question_count: items.len(),
        headers,
    })
}

/// Prep permission-card data from a live permission envelope (pure;
/// testable). Carries the V1 `permission` (else V2 `action`) kind and the
/// V1 `patterns` (else V2 `resources`) count. Returns `None` when the
/// envelope is not a permission ask.
pub fn permission_card_from_envelope(
    event_type: &str,
    payload: &Value,
) -> Option<PermissionCardData> {
    let pending = permission_asked_from_envelope(event_type, payload)?;
    let node = permission_node(payload)?;
    let permission = str_field_any(node, &["permission", "action"]).unwrap_or_default();
    let pattern_count = node
        .get("patterns")
        .or_else(|| node.get("resources"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    Some(PermissionCardData {
        request_ids: pending.request_ids,
        session_id: pending.session_id,
        directory: pending.directory,
        permission,
        pattern_count,
    })
}

/// Whether a live Q/P envelope session belongs to the followed session
/// (TS: `getFollowedSessionId` + `isForegroundSession` + tracked-child
/// gates). The attached session itself always counts; a known child
/// (session-cache ingest via [`ingest_restored_child`] or a seen
/// `session.created`) counts; anything else is left to the restore chain
/// resolve. No attached session means not followed.
fn envelope_session_followed(session_id: &str) -> bool {
    if session_id.trim().is_empty() {
        return false;
    }
    let attached = runtime().lock().ok().and_then(|rt| rt.attached.clone());
    let Some(attached) = attached else {
        return false;
    };
    if session_id == attached.session_id {
        return true;
    }
    with_aggregator(|agg| !agg.is_child_gated(session_id)).unwrap_or(false)
}

/// Record a restored parent link for session-cache ingest (TS:
/// `belongsToFollowedSession` registering each chain link so later child
/// events follow it). Pure wiring over the aggregator; the server chain
/// resolve stays with the dispatcher restore.
pub fn ingest_restored_child(child_session_id: &str, parent_session_id: &str) {
    if child_session_id.trim().is_empty() || parent_session_id.trim().is_empty() {
        tracing::warn!("[otb-bot] attach: refusing blank restored child ingest");
        return;
    }
    with_aggregator(|agg| agg.handle_session_created(child_session_id, parent_session_id));
    tracing::debug!("[otb-bot] attach: ingested restored child session");
}

/// Whether the attach should load pending questions/permissions now (TS:
/// `(!alreadyAttached || forceFullRestore) &&
/// !questionManager.isActive() && !permissionManager.isActive()` in
/// `attachToSession`; activeness reads the live slot kind here).
pub fn should_restore_pending_now(already_attached: bool, force_full_restore: bool) -> bool {
    let kind = interaction_slot_kind();
    let (question_active, permission_active) = match kind.as_deref() {
        Some("question") => (true, false),
        Some("permission") => (false, true),
        _ => (false, false),
    };
    otb_core::attach_restore_gates::should_restore_pending(
        already_attached,
        force_full_restore,
        question_active,
        permission_active,
    )
}

/// Reconnect restore decision for the dispatcher (TS:
/// `restorePendingInteractionsAfterReconnect` detached skip plus the
/// restart-only missed-turn gate). `server_restarted` mirrors the stream
/// `ReconnectInfo`; `busy` arrives from the session status load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectRestore {
    pub skip: bool,
    pub start_missed_live_turn: bool,
}

/// Pure reconnect-restore plan (testable): skip while detached, otherwise
/// restart only when the server changed, and start the missed turn only
/// when that restart meets a busy session.
pub fn reconnect_restore_plan(
    has_attached: bool,
    server_restarted: Option<bool>,
    busy: bool,
) -> ReconnectRestore {
    let skip = otb_core::attach_restore_gates::should_skip_reconnect_restore(has_attached);
    let restarted = server_restarted == Some(true);
    let start_missed_live_turn = !skip
        && otb_core::attach_restore_gates::should_call_start_missed_live_turn(restarted)
        && otb_core::attach_restore_gates::should_start_missed_live_turn(false, busy);
    ReconnectRestore {
        skip,
        start_missed_live_turn,
    }
}

/// Record the latest reconnect mark for the dispatcher restore (TS:
/// `restorePendingInteractionsAfterReconnect` inputs). The pump calls this
/// on every reconnect; the dispatcher drains it with
/// [`take_reconnect_restore`]. Never fails.
pub fn mark_reconnect_for_restore(server_restarted: Option<bool>) {
    if let Ok(mut rt) = runtime().lock() {
        rt.pending_reconnect_restart = Some(server_restarted.unwrap_or(false));
    }
}

/// Drain the pending reconnect mark, if any (dispatcher restore hook).
pub fn take_reconnect_restore() -> Option<bool> {
    runtime().lock().ok()?.pending_reconnect_restart.take()
}

/// Drain the pending reconnect mark and compute the dispatcher restore
/// plan (TS: `restorePendingInteractionsAfterReconnect` detached skip plus
/// the restart-only missed-turn gate). Returns `None` when no reconnect was
/// recorded since the last drain; the dispatcher calls this once the session
/// busy state is known. Never fails.
pub fn drain_reconnect_restore(busy: bool) -> Option<ReconnectRestore> {
    let restarted = take_reconnect_restore()?;
    let has_attached = runtime()
        .lock()
        .ok()
        .and_then(|rt| rt.attached.clone())
        .is_some();
    let plan = reconnect_restore_plan(has_attached, Some(restarted), busy);
    tracing::info!(
        "[otb-bot] attach: drained reconnect restore (skip={}, start_missed_live_turn={})",
        plan.skip,
        plan.start_missed_live_turn
    );
    Some(plan)
}

/// Release the single slot and forget both pending identities (TS:
/// `clearQuestionInteraction` / `clearPermissionInteraction` plus the
/// manager `clear`). Called once OpenCode resolves the held request —
/// accepted or already-settled (`gone`) — so a waiting interaction can move
/// in. Lock failure still clears the slot first so prompts never wedge.
pub fn release_interaction_slot() {
    if let Ok(mut slot) = slot().lock() {
        slot.clear();
    }
    if let Ok(mut rt) = runtime().lock() {
        rt.pending_question = None;
        rt.pending_permission = None;
    }
    tracing::info!("[otb-bot] attach: interaction slot released");
}

/// Single-slot guard for attach/prompt (TS: the interaction-slot busy check
/// before `attachToSession` + `processUserPrompt` send; PRODUCT.md
/// single-slot rule). Returns the busy-hint locale key when the slot is
/// occupied (copy owned by telegram-ux / i18n), else `None`.
///
/// Lock failure fails open (no block): the attach path never fails by
/// construction.
pub fn prompt_blocked_hint() -> Option<String> {
    let kind = interaction_slot_kind();
    if otb_core::interaction_state::is_prompt_blocked_by_slot(kind.as_deref()) {
        Some(otb_core::interaction_state::blocked_prompt_hint_key(kind.as_deref()).to_string())
    } else {
        None
    }
}

/// Run a closure against the shared aggregator; `None` when the lock is
/// poisoned, in which case the event is dropped.
fn with_aggregator<R>(f: impl FnOnce(&mut SummaryAggregator) -> R) -> Option<R> {
    aggregator().lock().ok().map(|mut agg| f(&mut agg))
}

/// Follow a session and make sure its event stream pumps (TS: `attachToSession`
/// core chain minus presentation/restore).
///
/// Records the attach, points the aggregator at the session (clearing the
/// previous session's state, like TS `setSession`), and ensures the SSE
/// subscription for the directory. Never fails: subscription errors stay
/// inside the spawned pump task (TS: `.catch(log)`).
pub fn attach_session(session_id: &str, directory: &str) {
    let already_attached = runtime()
        .lock()
        .ok()
        .and_then(|rt| rt.attached.clone())
        .is_some_and(|attached| {
            attached.session_id == session_id && attached.directory == directory
        });
    with_aggregator(|agg| agg.set_session(session_id));
    if let Ok(mut rt) = runtime().lock() {
        rt.attached = Some(AttachedSession {
            session_id: session_id.to_string(),
            directory: directory.to_string(),
        });
    }
    tracing::info!("[otb-bot] attach: following session {session_id}");
    // Trace-only restore gate (TS `attachToSession` pending-load guard):
    // the pending list reload itself stays a dispatcher follow-up, so a
    // held slot keeps blocking per the single-slot rule.
    if should_restore_pending_now(already_attached, false) {
        tracing::debug!("[otb-bot] attach: slot free, pending restore may run");
    } else {
        tracing::debug!("[otb-bot] attach: skipping pending restore (attached or slot held)");
    }
    ensure_event_subscription(directory);
}

/// Ensure one live SSE subscription for `directory` (TS:
/// `ensureEventSubscription` guard + same-directory fast path).
///
/// Empty/blank directories never subscribe (TS: `if (!directory) return`).
/// Scaffold default-off: returns without spawning unless polling is enabled
/// (the only caller path is the polling dispatcher). A live subscription for
/// the same directory is kept (TS `isListening` + same directory + stream
/// set); a dead same-directory stream or a directory switch aborts the old
/// task and resubscribes (TS abort + heal gates in `event_subscription`).
pub fn ensure_event_subscription(directory: &str) {
    if !otb_core::event_subscription::should_subscribe(directory) {
        tracing::warn!("[otb-bot] attach: no directory for event subscription");
        return;
    }
    if !crate::telegram::polling_enabled() {
        tracing::debug!("[otb-bot] attach: scaffold only, skipping event subscription");
        return;
    }
    let directory = directory.to_string();
    let mut rt = match runtime().lock() {
        Ok(rt) => rt,
        Err(_) => return,
    };
    let same_directory = rt.subscribed_directory.as_deref() == Some(directory.as_str());
    let has_stream = rt
        .subscription
        .as_ref()
        .is_some_and(|handle| !handle.is_finished());
    if otb_core::event_subscription::should_keep_same_directory_subscription(
        true,
        same_directory,
        has_stream,
    ) {
        tracing::debug!("[otb-bot] attach: event subscription already live for {directory}");
        return;
    }
    if let Some(handle) = rt.subscription.take() {
        handle.abort();
    }
    rt.subscribed_directory = Some(directory.clone());
    // `spawn`: the pump touches the shared `Send` aggregator through the
    // process-wide `Mutex`, so it runs on any worker thread. Only
    // reachable from the polling dispatcher (see `run_telegram_from_env`);
    // the scaffold default-off guard above keeps every other caller out.
    let task_directory = directory.clone();
    rt.subscription = Some(tokio::spawn(async move {
        let client = crate::opencode::OpencodeClient::from_env();
        client
            .subscribe_events(
                &task_directory,
                |envelope| {
                    handle_live_envelope(&envelope.event_type, &envelope.payload, &task_directory);
                },
                |info| {
                    mark_reconnect_for_restore(info.server_restarted);
                    // Trace-only triage (TS `restorePendingInteractionsAfterReconnect`
                    // detached skip): busy is unknown in this sync callback, so the
                    // dispatcher refines the missed-turn half later via
                    // `drain_reconnect_restore(busy)`. The mark above stays
                    // queued for that drain.
                    let has_attached = runtime()
                        .lock()
                        .ok()
                        .and_then(|rt| rt.attached.clone())
                        .is_some();
                    let plan =
                        reconnect_restore_plan(has_attached, info.server_restarted, false);
                    if plan.skip {
                        tracing::debug!(
                            "[otb-bot] attach: reconnect restore skipped (detached)"
                        );
                    } else {
                        tracing::info!(
                            "[otb-bot] attach: reconnect marked for restore (restarted={})",
                            info.server_restarted == Some(true)
                        );
                    }
                    match otb_core::event_subscription::classify_reconnect(info.server_restarted) {
                        otb_core::event_subscription::ReconnectAction::EndRunLost => {
                            tracing::warn!(
                                "[otb-bot] attach: stream reconnected to a restarted server; lost-run ending stays a dispatcher follow-up"
                            );
                        }
                        otb_core::event_subscription::ReconnectAction::StopBackgroundAndForgetTurn => {
                            tracing::info!(
                                "[otb-bot] attach: stream reconnected; background/turn cleanup stays a dispatcher follow-up"
                            );
                        }
                    }
                },
            )
            .await;
    }));
    tracing::info!("[otb-bot] attach: subscribing to OpenCode events for {directory}");
}

/// Handle one live SSE envelope in the pump (TS: `createEventRouter` Q/P
/// branches plus `interaction-handler.ts` present paths, minus Telegram
/// sends). V1 `question.asked` / `permission.asked` and V2 `form.created` /
/// `permission.asked` (via `data`) hold the single slot and prep card data
/// for the telegram-ux hooks; settled/replied envelopes release the slot.
/// Anything else falls through to the aggregator mapping. Best-effort:
/// lock failures drop the envelope.
fn handle_live_envelope(event_type: &str, payload: &Value, directory: &str) {
    if let Some(pending) = question_asked_from_envelope(event_type, payload) {
        if !envelope_session_followed(&pending.session_id) {
            tracing::debug!("[otb-bot] attach: ignoring question for unfollowed session");
            return;
        }
        let dir = if pending.directory.is_empty() {
            directory.to_string()
        } else {
            pending.directory.clone()
        };
        with_aggregator(|agg| agg.open_question(&pending.session_id, &pending.request_id));
        hold_question_slot(&pending.request_id, &pending.session_id, &dir);
        if let Some(card) = poll_card_from_envelope(event_type, payload) {
            let card = PollCardData {
                directory: dir,
                ..card
            };
            tracing::info!("[otb-bot] attach: question asked, poll card prepped");
            emit_poll_card(card);
        }
        return;
    }
    if let Some(pending) = permission_asked_from_envelope(event_type, payload) {
        if !envelope_session_followed(&pending.session_id) {
            tracing::debug!("[otb-bot] attach: ignoring permission for unfollowed session");
            return;
        }
        let dir = if pending.directory.is_empty() {
            directory.to_string()
        } else {
            pending.directory.clone()
        };
        let request_id = pending.request_ids.first().cloned().unwrap_or_default();
        with_aggregator(|agg| agg.request_permission(&pending.session_id, &request_id, ""));
        hold_permission_slot(&pending.request_ids, &pending.session_id, &dir);
        if let Some(card) = permission_card_from_envelope(event_type, payload) {
            let card = PermissionCardData {
                directory: dir,
                ..card
            };
            tracing::info!("[otb-bot] attach: permission asked, card prepped");
            emit_permission_card(card);
        }
        return;
    }
    if let Some((session_id, request_id, _outcome)) =
        question_settled_from_envelope(event_type, payload)
    {
        with_aggregator(|agg| agg.answer_question(&session_id, &request_id));
        release_interaction_slot();
        tracing::info!("[otb-bot] attach: question settled, slot released");
        return;
    }
    if let Some((session_id, request_id, allow)) =
        permission_replied_from_envelope(event_type, payload)
    {
        with_aggregator(|agg| agg.decide_permission(&session_id, &request_id, allow));
        release_interaction_slot();
        tracing::info!("[otb-bot] attach: permission replied, slot released");
        return;
    }
    if let Some(signal) = session_idle_from_envelope(event_type, payload) {
        if !envelope_session_followed(&signal.session_id) {
            tracing::debug!("[otb-bot] attach: ignoring idle for unfollowed session");
            return;
        }
        // Aggregator run bookkeeping stays consistent for the dispatcher;
        // the footer itself flows through the telegram-ux idle hook.
        with_aggregator(|agg| agg.set_idle_gated(&signal.session_id, signal.interrupted));
        if should_emit_session_idle(&signal) {
            tracing::info!("[otb-bot] attach: session idle, footer hook emitted");
            emit_session_idle_footer(signal);
        } else {
            tracing::debug!("[otb-bot] attach: duplicate session idle suppressed");
        }
        return;
    }
    let event = aggregator_event_from_envelope(event_type, payload);
    // Register the parent link first (TS `belongsToFollowedSession`
    // link registration): later Q/P from this child then passes the
    // followed-session gate. The card/title enrichment still runs through
    // `process_event` below.
    if let AggregatorEvent::ChildSessionCreated {
        child_session_id,
        parent_session_id,
        ..
    } = &event
    {
        ingest_restored_child(child_session_id, parent_session_id);
    }
    if matches!(event, AggregatorEvent::Other) {
        return;
    }
    note_live_activity();
    with_aggregator(|agg| agg.process_event(event));
}

/// Map one SSE envelope to the aggregator event model (pure; testable).
///
/// TS parity: `createEventRouter` session-id read
/// (`properties.sessionID || properties.info?.sessionID ||
/// properties.part?.sessionID`) plus the `handleMessagePartUpdated` part
/// shapes (`part.sessionID`, `part.callID`, `part.tool`, `state.status`,
/// `state.input.{description,prompt,subagent_type}`, `state.metadata.sessionId`,
/// subtask `part.{id,agent,description,prompt}`, `session.created/updated`
/// `properties.info.{id,parentID,title}`). Anything unrecognized (heartbeats,
/// status pings, deltas the Rust aggregator does not model yet) maps to
/// [`AggregatorEvent::Other`] and is skipped by the pump.
pub fn aggregator_event_from_envelope(event_type: &str, payload: &Value) -> AggregatorEvent {
    let props = payload.get("properties");
    match event_type {
        "message.updated" => {
            let info = props.and_then(|props| props.get("info"));
            let session_id = info
                .and_then(|info| info.get("sessionID"))
                .and_then(Value::as_str)
                .or_else(|| props.and_then(|props| str_field(props, "sessionID")));
            let message_id = info
                .and_then(|info| info.get("id"))
                .and_then(Value::as_str)
                .or_else(|| props.and_then(|props| str_field(props, "messageID")));
            match (session_id, message_id) {
                (Some(session_id), Some(message_id))
                    if !session_id.is_empty() && !message_id.is_empty() =>
                {
                    AggregatorEvent::MessageUpdated {
                        session_id: session_id.to_string(),
                        message_id: message_id.to_string(),
                    }
                }
                _ => AggregatorEvent::Other,
            }
        }
        "message.part.updated" => {
            let part = props.and_then(|props| props.get("part"));
            let session_id = part
                .and_then(|part| str_field(part, "sessionID"))
                .or_else(|| props.and_then(|props| str_field(props, "sessionID")));
            let Some(session_id) = session_id.filter(|id| !id.is_empty()) else {
                return AggregatorEvent::Other;
            };
            let part_type = part.and_then(|part| str_field(part, "type")).unwrap_or("");
            match part_type {
                "subtask" => {
                    let part_id = part.and_then(|part| str_field(part, "id")).unwrap_or("");
                    if part_id.is_empty() {
                        return AggregatorEvent::Other;
                    }
                    AggregatorEvent::SubtaskPartUpdated {
                        parent_session_id: session_id.to_string(),
                        part_id: part_id.to_string(),
                        agent: part
                            .and_then(|part| str_field(part, "agent"))
                            .unwrap_or_default()
                            .to_string(),
                        description: part
                            .and_then(|part| str_field(part, "description"))
                            .unwrap_or_default()
                            .to_string(),
                        prompt: part
                            .and_then(|part| str_field(part, "prompt"))
                            .unwrap_or_default()
                            .to_string(),
                    }
                }
                "tool" => {
                    let raw_tool = part.and_then(|part| str_field(part, "tool")).unwrap_or("");
                    let tool = otb_core::v2_mappers::to_v1_tool_name(raw_tool).to_string();
                    let call_id = part
                        .and_then(|part| str_field(part, "callID"))
                        .or_else(|| part.and_then(|part| str_field(part, "id")))
                        .or_else(|| props.and_then(|props| str_field(props, "partID")))
                        .unwrap_or("");
                    if call_id.is_empty() || tool.is_empty() {
                        return AggregatorEvent::Other;
                    }
                    let state = part.and_then(|part| part.get("state"));
                    if tool == "task" {
                        let input = state.and_then(|state| state.get("input"));
                        AggregatorEvent::TaskToolPartUpdated {
                            input: TaskCallInput {
                                call_id: call_id.to_string(),
                                parent_session_id: session_id.to_string(),
                                description: input
                                    .and_then(|input| str_field(input, "description"))
                                    .unwrap_or_default()
                                    .to_string(),
                                agent: input
                                    .and_then(|input| {
                                        ["subagent_type", "agent", "subagent", "type"]
                                            .iter()
                                            .find_map(|key| str_field(input, key))
                                    })
                                    .map(str::to_string),
                                prompt: input
                                    .and_then(|input| str_field(input, "prompt"))
                                    .map(str::to_string),
                                child_session_id: state
                                    .and_then(|state| state.get("metadata"))
                                    .and_then(|metadata| str_field(metadata, "sessionId"))
                                    .map(str::to_string),
                                paired_card_id: None,
                            },
                        }
                    } else {
                        let status = state
                            .and_then(|state| str_field(state, "status"))
                            .unwrap_or("");
                        AggregatorEvent::ToolPartUpdated {
                            part: ToolPart {
                                session_id: session_id.to_string(),
                                message_id: part
                                    .and_then(|part| str_field(part, "messageID"))
                                    .or_else(|| {
                                        props.and_then(|props| str_field(props, "messageID"))
                                    })
                                    .unwrap_or_default()
                                    .to_string(),
                                call_id: call_id.to_string(),
                                tool,
                                status: map_tool_status(status),
                            },
                        }
                    }
                }
                _ => AggregatorEvent::Other,
            }
        }
        "session.created" | "session.updated" => {
            let info = props.and_then(|props| props.get("info"));
            let child = info
                .and_then(|info| str_field(info, "id"))
                .or_else(|| info.and_then(|info| str_field(info, "sessionID")));
            let parent = info.and_then(|info| str_field(info, "parentID"));
            match (child, parent) {
                (Some(child), Some(parent)) if !child.is_empty() && !parent.is_empty() => {
                    AggregatorEvent::ChildSessionCreated {
                        child_session_id: child.to_string(),
                        parent_session_id: parent.to_string(),
                        title: info
                            .and_then(|info| str_field(info, "title"))
                            .unwrap_or_default()
                            .to_string(),
                    }
                }
                _ => AggregatorEvent::Other,
            }
        }
        _ => AggregatorEvent::Other,
    }
}

/// String field reader (missing/non-string counts as absent).
fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)?.as_str()
}

/// Tool state status into the aggregator model (TS `ToolState["status"]`).
fn map_tool_status(status: &str) -> ToolStatus {
    match status {
        "running" => ToolStatus::Running,
        "completed" => ToolStatus::Completed,
        "error" => ToolStatus::Error,
        other => ToolStatus::Unknown(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Serializes the slot/runtime-mutating tests below: the statics are
    /// process-wide and Rust runs tests on shared threads, so two tests
    /// must never hold the slot at once.
    static SLOT_TEST_LOCK: Mutex<()> = Mutex::new(());
    /// Serializes the idle-footer dedup tests: the emission guard is
    /// process-wide, so consecutive-duplicate assertions must not race.
    static IDLE_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Hold the idle-dedup lock and reset the emission guard so no
    /// cross-test leak survives.
    fn lock_idle_footer_tests() -> MutexGuard<'static, ()> {
        let guard = IDLE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        note_live_activity();
        guard
    }

    /// Serialize the slot/runtime-mutating tests and recover from a
    /// previously poisoned lock so one failure never cascades into
    /// `PoisonError` flakes in the next test. Resets the slot plus the
    /// runtime pending identities up front so no cross-test leak survives.
    fn lock_slot_tests() -> MutexGuard<'static, ()> {
        let guard = SLOT_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        slot().lock().unwrap_or_else(|e| e.into_inner()).clear();
        {
            let mut rt = runtime().lock().unwrap_or_else(|e| e.into_inner());
            rt.pending_question = None;
            rt.pending_permission = None;
            rt.pending_reconnect_restart = None;
        }
        guard
    }

    #[test]
    fn message_updated_maps_session_and_message_ids() {
        let payload = json!({
            "type": "message.updated",
            "properties": { "info": { "id": "msg_1", "sessionID": "ses_1" } }
        });
        assert_eq!(
            aggregator_event_from_envelope("message.updated", &payload),
            AggregatorEvent::MessageUpdated {
                session_id: "ses_1".to_string(),
                message_id: "msg_1".to_string(),
            }
        );
        let missing = json!({ "type": "message.updated", "properties": {} });
        assert_eq!(
            aggregator_event_from_envelope("message.updated", &missing),
            AggregatorEvent::Other
        );
    }

    #[test]
    fn tool_part_maps_with_v2_name_normalization() {
        // TS `toV1ToolName`: V2 `shell` arrives as V1 `bash`.
        let payload = json!({
            "type": "message.part.updated",
            "properties": {
                "part": {
                    "type": "tool",
                    "sessionID": "ses_1",
                    "messageID": "msg_1",
                    "callID": "call_1",
                    "tool": "shell",
                },
                "sessionID": "ses_1",
            }
        });
        assert_eq!(
            aggregator_event_from_envelope("message.part.updated", &payload),
            AggregatorEvent::ToolPartUpdated {
                part: ToolPart {
                    session_id: "ses_1".to_string(),
                    message_id: "msg_1".to_string(),
                    call_id: "call_1".to_string(),
                    tool: "bash".to_string(),
                    status: ToolStatus::Unknown(String::new()),
                }
            }
        );
    }

    #[test]
    fn task_tool_part_maps_agent_fallbacks_and_child_session() {
        let payload = json!({
            "type": "message.part.updated",
            "properties": {
                "part": {
                    "type": "tool",
                    "sessionID": "ses_1",
                    "callID": "call_9",
                    "tool": "task",
                    "state": {
                        "status": "running",
                        "input": { "description": "dig", "agent": "explore" },
                        "metadata": { "sessionId": "ses_child" },
                    },
                }
            }
        });
        assert_eq!(
            aggregator_event_from_envelope("message.part.updated", &payload),
            AggregatorEvent::TaskToolPartUpdated {
                input: TaskCallInput {
                    call_id: "call_9".to_string(),
                    parent_session_id: "ses_1".to_string(),
                    description: "dig".to_string(),
                    agent: Some("explore".to_string()),
                    prompt: None,
                    child_session_id: Some("ses_child".to_string()),
                    paired_card_id: None,
                }
            }
        );
    }

    #[test]
    fn subtask_part_maps_parent_and_part_shape() {
        let payload = json!({
            "type": "message.part.updated",
            "properties": {
                "part": {
                    "type": "subtask",
                    "sessionID": "ses_1",
                    "id": "part_2",
                    "agent": "explore",
                    "description": "dig",
                    "prompt": "go",
                }
            }
        });
        assert_eq!(
            aggregator_event_from_envelope("message.part.updated", &payload),
            AggregatorEvent::SubtaskPartUpdated {
                parent_session_id: "ses_1".to_string(),
                part_id: "part_2".to_string(),
                agent: "explore".to_string(),
                description: "dig".to_string(),
                prompt: "go".to_string(),
            }
        );
    }

    #[test]
    fn session_created_maps_child_parent_linkage_only() {
        let payload = json!({
            "type": "session.created",
            "properties": {
                "info": { "id": "ses_child", "parentID": "ses_1", "title": "dig" }
            }
        });
        assert_eq!(
            aggregator_event_from_envelope("session.created", &payload),
            AggregatorEvent::ChildSessionCreated {
                child_session_id: "ses_child".to_string(),
                parent_session_id: "ses_1".to_string(),
                title: "dig".to_string(),
            }
        );
        // A root session (no parent) is not a child link: skip it.
        let root = json!({
            "type": "session.created",
            "properties": { "info": { "id": "ses_1" } }
        });
        assert_eq!(
            aggregator_event_from_envelope("session.created", &root),
            AggregatorEvent::Other
        );
    }

    #[test]
    fn unknown_events_map_to_other() {
        let payload = json!({ "type": "server.heartbeat" });
        assert_eq!(
            aggregator_event_from_envelope("server.heartbeat", &payload),
            AggregatorEvent::Other
        );
    }

    #[test]
    fn empty_slot_does_not_block_attach_or_prompt() {
        // Single-slot default: no live hold yet, so the guard stays open
        // and attach/prompt proceeds.
        let _guard = lock_slot_tests();
        release_interaction_slot();
        assert_eq!(interaction_slot_kind(), None);
        assert_eq!(prompt_blocked_hint(), None);
    }

    #[test]
    fn question_hold_blocks_prompts_until_release() {
        // A question hold opens a `question` slot (plain-text answers
        // route; other prompts hint), and release re-opens the guard.
        let _guard = lock_slot_tests();
        release_interaction_slot();
        hold_question_slot("req_1", "ses_1", "/work");
        assert_eq!(interaction_slot_kind().as_deref(), Some("question"));
        assert_eq!(
            held_question(),
            Some(PendingQuestion {
                request_id: "req_1".to_string(),
                session_id: "ses_1".to_string(),
                directory: "/work".to_string(),
            })
        );
        assert_eq!(held_permission(), None);
        assert!(prompt_blocked_hint().is_some());
        release_interaction_slot();
        assert_eq!(interaction_slot_kind(), None);
        assert_eq!(held_question(), None);
        assert_eq!(prompt_blocked_hint(), None);
    }

    #[test]
    fn permission_hold_replaces_question_hold_single_slot_rule() {
        // One interaction at a time: a permission hold drops the pending
        // question and flips the slot kind.
        let _guard = lock_slot_tests();
        release_interaction_slot();
        hold_question_slot("req_1", "ses_1", "/work");
        hold_permission_slot(&["perm_1".to_string()], "ses_1", "/work");
        assert_eq!(interaction_slot_kind().as_deref(), Some("permission"));
        assert_eq!(held_question(), None);
        assert_eq!(
            held_permission(),
            Some(PendingPermission {
                request_ids: vec!["perm_1".to_string()],
                session_id: "ses_1".to_string(),
                directory: "/work".to_string(),
            })
        );
        release_interaction_slot();
        assert_eq!(held_permission(), None);
    }

    #[test]
    fn blank_holds_never_open_the_slot() {
        let _guard = lock_slot_tests();
        release_interaction_slot();
        hold_question_slot("", "ses_1", "/work");
        hold_question_slot("req_1", "  ", "/work");
        hold_permission_slot(&[], "ses_1", "/work");
        hold_permission_slot(&["perm_1".to_string()], "", "/work");
        assert_eq!(interaction_slot_kind(), None);
        assert_eq!(held_question(), None);
        assert_eq!(held_permission(), None);
    }

    #[test]
    fn question_asked_v1_and_v2_form_detect_identities() {
        let v1 = json!({
            "properties": {
                "id": "req_1",
                "sessionID": "ses_1",
                "questions": [{ "header": "Pick", "question": "Pick?" }],
            }
        });
        assert_eq!(
            question_asked_from_envelope("question.asked", &v1),
            Some(PendingQuestion {
                request_id: "req_1".to_string(),
                session_id: "ses_1".to_string(),
                directory: String::new(),
            })
        );
        let v2 = json!({
            "directory": "/work",
            "data": {
                "form": {
                    "id": "req_9",
                    "sessionID": "ses_1",
                    "title": "Deploy?",
                    "fields": [{ "key": "go", "title": "Go?" }],
                }
            }
        });
        assert_eq!(
            question_asked_from_envelope("form.created", &v2),
            Some(PendingQuestion {
                request_id: "req_9".to_string(),
                session_id: "ses_1".to_string(),
                directory: "/work".to_string(),
            })
        );
        let card = poll_card_from_envelope("form.created", &v2).expect("poll card");
        assert_eq!(card.request_id, "req_9");
        assert_eq!(card.headers, vec!["Go?".to_string()]);
    }

    #[test]
    fn permission_asked_v1_and_v2_detect_identities() {
        let v1 = json!({
            "properties": {
                "id": "perm_1",
                "sessionID": "ses_1",
                "permission": "bash",
                "patterns": ["cargo *"],
            }
        });
        assert_eq!(
            permission_asked_from_envelope("permission.asked", &v1),
            Some(PendingPermission {
                request_ids: vec!["perm_1".to_string()],
                session_id: "ses_1".to_string(),
                directory: String::new(),
            })
        );
        let card = permission_card_from_envelope("permission.asked", &v1).expect("perm card");
        assert_eq!(card.permission, "bash");
        assert_eq!(card.pattern_count, 1);
        // V2 raw `data` shape (action/resources) is admitted too.
        let v2 = json!({
            "data": {
                "id": "perm_2",
                "sessionID": "ses_1",
                "action": "edit",
                "resources": ["a.txt", "b.txt"],
            }
        });
        let pending = permission_asked_from_envelope("permission.asked", &v2).expect("v2 perm");
        assert_eq!(pending.request_ids, vec!["perm_2".to_string()]);
        let card = permission_card_from_envelope("permission.asked", &v2).expect("v2 card");
        assert_eq!(card.permission, "edit");
        assert_eq!(card.pattern_count, 2);
    }

    #[test]
    fn settled_and_replied_envelopes_detect_release() {
        let replied = json!({
            "properties": { "sessionID": "ses_1", "requestID": "req_1" }
        });
        assert_eq!(
            question_settled_from_envelope("question.replied", &replied),
            Some(("ses_1".to_string(), "req_1".to_string(), "answered"))
        );
        let cancelled = json!({ "data": { "sessionID": "ses_1", "id": "req_1" } });
        assert_eq!(
            question_settled_from_envelope("form.cancelled", &cancelled),
            Some(("ses_1".to_string(), "req_1".to_string(), "cancelled"))
        );
        let perm = json!({
            "properties": { "sessionID": "ses_1", "requestID": "perm_1", "reply": "reject" }
        });
        assert_eq!(
            permission_replied_from_envelope("permission.replied", &perm),
            Some(("ses_1".to_string(), "perm_1".to_string(), false))
        );
    }

    #[test]
    fn session_idle_signal_parses_v1_shutdown_shape() {
        // Failing-test-first for the unwired footer: the live
        // `session.idle` envelope must parse into its footer signal.
        let shutdown = json!({
            "type": "session.idle",
            "properties": {"sessionID": "ses_1", "interrupted": true, "reason": "shutdown"}
        });
        assert_eq!(
            session_idle_from_envelope("session.idle", &shutdown),
            Some(SessionIdleSignal {
                session_id: "ses_1".to_string(),
                interrupted: true,
                shutdown_idle: true,
            })
        );
        let clean = json!({
            "type": "session.idle",
            "properties": {"sessionID": "ses_1"}
        });
        assert_eq!(
            session_idle_from_envelope("session.idle", &clean),
            Some(SessionIdleSignal {
                session_id: "ses_1".to_string(),
                interrupted: false,
                shutdown_idle: false,
            })
        );
        // A user interrupt is not a shutdown; other types and blank
        // sessions never signal.
        let user = json!({
            "type": "session.idle",
            "properties": {"sessionID": "ses_1", "interrupted": true, "reason": "user"}
        });
        assert_eq!(
            session_idle_from_envelope("session.idle", &user),
            Some(SessionIdleSignal {
                session_id: "ses_1".to_string(),
                interrupted: true,
                shutdown_idle: false,
            })
        );
        let blank = json!({ "type": "session.idle", "properties": {"sessionID": "  "} });
        assert_eq!(session_idle_from_envelope("session.idle", &blank), None);
        assert_eq!(session_idle_from_envelope("session.error", &shutdown), None);
    }

    #[test]
    fn session_idle_dedup_suppresses_consecutive_duplicates() {
        // Failing-test-first: duplicate `session.idle` frames for one run
        // collapse to a single footer emission (TS `idleEmitted` parity);
        // real turn activity re-arms the next idle.
        let _guard = lock_idle_footer_tests();
        note_live_activity();
        let signal = SessionIdleSignal {
            session_id: "ses_1".to_string(),
            interrupted: false,
            shutdown_idle: false,
        };
        assert!(should_emit_session_idle(&signal));
        assert!(!should_emit_session_idle(&signal));
        note_live_activity();
        assert!(should_emit_session_idle(&signal));
        let other = SessionIdleSignal {
            session_id: "ses_2".to_string(),
            interrupted: false,
            shutdown_idle: false,
        };
        assert!(should_emit_session_idle(&other));
        note_live_activity();
    }

    #[test]
    fn restore_hooks_follow_attach_restore_gates() {
        let _guard = lock_slot_tests();
        release_interaction_slot();
        assert!(should_restore_pending_now(false, false));
        hold_question_slot("req_1", "ses_1", "/work");
        assert!(!should_restore_pending_now(false, false));
        release_interaction_slot();
        let plan = reconnect_restore_plan(true, Some(true), true);
        assert!(!plan.skip);
        assert!(plan.start_missed_live_turn);
        let detached = reconnect_restore_plan(false, Some(true), true);
        assert!(detached.skip);
        assert!(!detached.start_missed_live_turn);
        mark_reconnect_for_restore(Some(true));
        assert_eq!(take_reconnect_restore(), Some(true));
        assert_eq!(take_reconnect_restore(), None);
        // The dispatcher drain consumes the mark and plans with real busy.
        mark_reconnect_for_restore(Some(false));
        let drained = drain_reconnect_restore(false).expect("drained plan");
        assert!(!drained.start_missed_live_turn);
        assert_eq!(drain_reconnect_restore(false), None);
        release_interaction_slot();
    }
}
