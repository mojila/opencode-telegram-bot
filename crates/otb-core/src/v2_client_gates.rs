//! V2 client pure gates (slice 109, GREEN).
//!
//! Rust port of the pure remainder in `src/opencode/v2/client.ts`
//! (`createV2OpencodeClient` + `findRegisteredV2Server`):
//! - timing / paging budgets (`HEARTBEAT_INTERVAL_MS` 10000,
//!   `MESSAGE_PAGE_SIZE` 200, `MAX_MESSAGE_PAGES` 50,
//!   `SESSION_PAGE_SIZE` 100, `COMPLETION_POLL_INTERVAL_MS` 250,
//!   `COMPLETION_POLL_ATTEMPTS` 40, `SERVER_IDENTITY_TIMEOUT_MS` 2000)
//! - `isNotFound` split (`name.endsWith("NotFoundError")`, else
//!   `ClientError` + `UnexpectedStatus` + 404 status)
//! - `isUnsupportedGoalError` split (command-not-found name/tag/suffix,
//!   `rpc.method_not_found` / `rpc.unavailable` type, rpc-only
//!   `ClientError` + `UnexpectedStatus` + 404)
//! - `parseModelRef` (`"provider/id"` split, `separator <= 0` miss)
//! - `listMessages` page cap (`min(limit * 3, 200)`), message / session
//!   cursor break gates (missing cursor, already-seen cursor, empty page)
//! - `admitPrompt` text (`system ? system + "\n\n" + text : text`) and
//!   delivery default (`delivery ?? "queue"`), `runCommand` skill text
//!   (`text || "/" + command`)
//! - heartbeat id (`heartbeat:<now_ms>`) and the unknown-request /
//!   no-completed-reply messages
//! - `applySelection` gates (skip without agent and model, agent switch
//!   on difference, model switch on id/provider difference or defined
//!   variant difference) and `goal.get` `output ?? null` gate.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `toV1*` / `toV2PromptInput` / `toFormAnswer` /
//!   `isGoalInternalMetadata` mapping bodies (`v2_mappers` owner; this
//!   slice owns only the client-side emit/drop/id/paging decisions
//!   around those calls).
//! - `createV2EventTranslator` stream translation
//!   (`v2_event_translator_gates` owner), subscription lifecycle /
//!   busy-idle routing (`event_subscription` / `event_transport_gates`
//!   owners).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `OpenCodeClient` network calls, `Promise.race` identity timeout,
//!   `setTimeout` poll delays, `Date.now()` clocks (arrive as explicit
//!   `now_ms` args), `Service.discover()` registration lookup, the
//!   `translatedStream` queue/generator plumbing.
//!
//! Deviations: none intended. `String.length` / `indexOf` parity fixtures
//! use ASCII ids where JS UTF-16 code units and Rust `char`s agree.
//! `""` counts as missing for system text, model refs, deliveries and
//! directories (TS truthiness / `??` on empty strings carried explicitly).

/// Keepalive heartbeat cadence (TS: `HEARTBEAT_INTERVAL_MS`).
pub const HEARTBEAT_INTERVAL_MS: u64 = 10_000;

/// Message page size for full history (TS: `MESSAGE_PAGE_SIZE`).
pub const MESSAGE_PAGE_SIZE: u64 = 200;

/// Maximum message pages per full-history read (TS: `MAX_MESSAGE_PAGES`).
pub const MAX_MESSAGE_PAGES: u32 = 50;

/// Session page size for un-limited lists (TS: `SESSION_PAGE_SIZE`).
pub const SESSION_PAGE_SIZE: u64 = 100;

/// Delay between completion polls (TS: `COMPLETION_POLL_INTERVAL_MS`).
pub const COMPLETION_POLL_INTERVAL_MS: u64 = 250;

/// Completion poll budget (TS: `COMPLETION_POLL_ATTEMPTS`).
pub const COMPLETION_POLL_ATTEMPTS: u32 = 40;

/// Server-identity read budget (TS: `SERVER_IDENTITY_TIMEOUT_MS`).
pub const SERVER_IDENTITY_TIMEOUT_MS: u64 = 2_000;

/// `true` when an error name already declares "not found"
/// (TS: `error.name.endsWith("NotFoundError")` in `isNotFound`).
pub fn is_not_found_name(name: &str) -> bool {
    name.ends_with("NotFoundError")
}

/// `true` for an undeclared 404 client answer
/// (TS: `name === "ClientError" && reason === "UnexpectedStatus" &&
/// cause.status === 404` in `isNotFound`).
pub fn is_not_found_client_status(name: &str, reason: &str, status: u16) -> bool {
    name == "ClientError" && reason == "UnexpectedStatus" && status == 404
}

/// `true` for a missing-goal-plugin command answer
/// (TS: `name === "CommandNotFoundError" || tag === "CommandNotFoundError"
/// || name.endsWith("CommandNotFoundError")` in `isUnsupportedGoalError`).
pub fn is_goal_command_not_found(name: &str, tag: &str) -> bool {
    name == "CommandNotFoundError"
        || tag == "CommandNotFoundError"
        || name.ends_with("CommandNotFoundError")
}

/// `true` for a missing-goal-plugin RPC type
/// (TS: `type === "rpc.method_not_found" || type === "rpc.unavailable"`
/// in `isUnsupportedGoalError`).
pub fn is_goal_rpc_unavailable(error_type: &str) -> bool {
    error_type == "rpc.method_not_found" || error_type == "rpc.unavailable"
}

/// `true` for the rpc-only 404 goal fallback
/// (TS: `source === "rpc" && name === "ClientError" &&
/// reason === "UnexpectedStatus" && cause.status === 404`).
pub fn is_goal_404_for_rpc(source_is_rpc: bool, name: &str, reason: &str, status: u16) -> bool {
    source_is_rpc && name == "ClientError" && reason == "UnexpectedStatus" && status == 404
}

/// Split `"provider/id"` into its parts, or `None` when blank or
/// separator-missing/leading (TS: `parseModelRef`: falsy -> undefined,
/// `separator <= 0` -> undefined).
pub fn parse_model_ref(model: Option<&str>) -> Option<(String, String)> {
    let model = model?;
    if model.is_empty() {
        return None;
    }
    let separator = model.find('/')?;
    if separator == 0 {
        return None;
    }
    Some((
        model[..separator].to_string(),
        model[separator + 1..].to_string(),
    ))
}

/// Over-fetch cap for a limited history read
/// (TS: `Math.min(limit * 3, MESSAGE_PAGE_SIZE)` in `listMessages`).
pub fn message_page_limit(limit: u64) -> u64 {
    limit.saturating_mul(3).min(MESSAGE_PAGE_SIZE)
}

/// `true` when message paging stops (TS: `!next || data.length === 0`
/// in `listMessages`).
pub fn should_break_message_pages(has_next: bool, page_empty: bool) -> bool {
    !has_next || page_empty
}

/// `true` when session paging stops (TS: `!next || seen.has(next) ||
/// data.length === 0` in the unlimited `session.list` loop).
pub fn should_break_session_list(has_next: bool, already_seen: bool, page_empty: bool) -> bool {
    !has_next || already_seen || page_empty
}

/// Prompt text with the optional system preface
/// (TS: `system ? system + "\n\n" + text : text` in `admitPrompt`).
pub fn resolve_prompt_text(system: Option<&str>, text: &str) -> String {
    match system {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}\n\n{text}"),
        _ => text.to_string(),
    }
}

/// Inbox delivery with the queue default
/// (TS: `delivery ?? "queue"` in `admitPrompt`; `""` counts as missing).
pub fn default_delivery(delivery: Option<&str>) -> &str {
    match delivery {
        Some(value) if !value.is_empty() => value,
        _ => "queue",
    }
}

/// Skill-command prompt text with the `/command` fallback
/// (TS: `text || "/" + command` in `runCommand`).
pub fn skill_prompt_text(command: &str, text: &str) -> String {
    if text.is_empty() {
        format!("/{command}")
    } else {
        text.to_string()
    }
}

/// Heartbeat event id for an idle-but-healthy stream
/// (TS: `` `heartbeat:${Date.now()}` `` in `onActivity`).
pub fn heartbeat_event_id(now_ms: i64) -> String {
    format!("heartbeat:{now_ms}")
}

/// Unknown-question error text (TS: `` `Unknown question request:
/// ${requestID}` `` in `findForm`).
pub fn unknown_question_message(request_id: &str) -> String {
    format!("Unknown question request: {request_id}")
}

/// Unknown-permission error text (TS: `` `Unknown permission request:
/// ${requestID}` `` in `permission.reply`).
pub fn unknown_permission_message(request_id: &str) -> String {
    format!("Unknown permission request: {request_id}")
}

/// No-completed-reply error text (TS: `` `No completed assistant reply
/// in session ${sessionID}` `` in `lastAssistantMessage`).
pub fn no_completed_reply_message(session_id: &str) -> String {
    format!("No completed assistant reply in session {session_id}")
}

/// `true` when agent/model selection is skipped entirely
/// (TS: `if (!agent && !model) return;` in `applySelection`).
pub fn should_skip_apply_selection(has_agent: bool, has_model: bool) -> bool {
    !has_agent && !has_model
}

/// `true` when the session agent must switch
/// (TS: `agent && session.agent !== agent` in `applySelection`).
pub fn should_switch_agent(session_agent: Option<&str>, desired: Option<&str>) -> bool {
    match desired {
        Some(next) if !next.is_empty() => session_agent != desired,
        _ => false,
    }
}

/// `true` when the session model must switch
/// (TS: `model && (id !== model.id || providerID !== model.providerID ||
/// (variant !== undefined && variant !== model.variant))`).
pub fn should_switch_model(
    current_id: Option<&str>,
    current_provider: Option<&str>,
    current_variant: Option<&str>,
    has_model: bool,
    desired_id: &str,
    desired_provider: &str,
    desired_variant: Option<&str>,
) -> bool {
    if !has_model {
        return false;
    }
    if current_id.map_or(true, |current| current != desired_id) {
        return true;
    }
    if current_provider.map_or(true, |current| current != desired_provider) {
        return true;
    }
    match desired_variant {
        Some(next) => current_variant != Some(next),
        None => false,
    }
}

/// `true` when a goal `get` answer carries `null`
/// (TS: `result.output ?? null` in `goal.get`).
pub fn goal_output_is_null(has_output: bool) -> bool {
    !has_output
}
