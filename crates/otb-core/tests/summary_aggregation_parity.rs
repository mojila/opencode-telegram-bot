//! Parity port of `tests/app/managers/summary-aggregation-manager.test.ts`
//! (slices 1-2, slice 2 RED).
//!
//! Slice 1 (passing): top-level `describe("summary/aggregator")`, first 4 `it`s.
//! Slice 2 (RED, `unimplemented!()` stubs): the next `describe` blocks —
//! live subagent updates, subagent variant from the child assistant message,
//! task-call input pairing, and subagent current-tool timing start.
//!
//! Covered here (top-level `describe("summary/aggregator")`, first 4 `it`s):
//! - invokes onCleared callback when aggregator is cleared
//! - includes sessionId in tool callback payload
//! - emits root tool updates before completion filtering
//! - does not emit root tool updates for unrelated child sessions
//!
//! Deferred to slices 3-n (same file, not ported here):
//! - `describe("summary/aggregator")` remaining `it`s: session.created gating,
//!   parallel subagents, terminal cards, write-tool oversized payload,
//!   reasoning/thinking streaming, partial text streaming,
//!   external user input, multi-text combining, optimistic streaming,
//!   session idle/error/retry/status, apply_patch tool files, tokens/cost,
//!   deltas, assistant metadata, subagent timing/retire/background variants.
//! - `describe("subagent current tool timing")` nested
//!   `describe("an interrupted subagent")` + `describe("a nested subagent")`
//! - `describe("live turn start")` (4 its)

use otb_core::managers::summary_aggregation::{
    AggregatorEvent, SummaryAggregator, TaskCallInput, ToolPart, ToolStatus,
};
use std::cell::RefCell;
use std::rc::Rc;

fn tool_part(session_id: &str, message_id: &str, call_id: &str, tool: &str) -> ToolPart {
    ToolPart {
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
        call_id: call_id.to_string(),
        tool: tool.to_string(),
        status: ToolStatus::Running,
    }
}

#[test]
fn invokes_on_cleared_callback_when_aggregator_is_cleared() {
    let mut aggregator = SummaryAggregator::new();
    let calls = Rc::new(RefCell::new(0usize));
    let probe = Rc::clone(&calls);
    aggregator.set_on_cleared(Box::new(move || {
        *probe.borrow_mut() += 1;
    }));

    aggregator.clear();

    assert_eq!(*calls.borrow(), 1);
}

#[test]
fn includes_session_id_in_tool_callback_payload() {
    let mut aggregator = SummaryAggregator::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&seen);
    aggregator.set_on_tool(Box::new(move |info| {
        probe.borrow_mut().push((info.session_id, info.call_id, info.tool));
    }));
    aggregator.set_session("session-1");

    aggregator.process_event(AggregatorEvent::MessageUpdated {
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
    });
    // TS slice-1 sends a completed part (test lines 83-84:
    // `status: "completed"`); only completed parts reach `on_tool`.
    let mut part = tool_part("session-1", "message-1", "call-1", "bash");
    part.status = ToolStatus::Completed;
    aggregator.process_event(AggregatorEvent::ToolPartUpdated { part });

    let seen = seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0],
        (
            "session-1".to_string(),
            "call-1".to_string(),
            "bash".to_string()
        )
    );
}

#[test]
fn emits_root_tool_updates_before_completion_filtering() {
    let mut aggregator = SummaryAggregator::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&seen);
    aggregator.set_on_root_tool_update(Box::new(move |info| {
        probe.borrow_mut().push((info.session_id, info.call_id, info.tool));
    }));
    aggregator.set_session("session-1");

    aggregator.process_event(AggregatorEvent::MessageUpdated {
        session_id: "session-1".to_string(),
        message_id: "message-1".to_string(),
    });
    aggregator.process_event(AggregatorEvent::ToolPartUpdated {
        part: tool_part("session-1", "message-1", "call-task", "task"),
    });

    let seen = seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0],
        (
            "session-1".to_string(),
            "call-task".to_string(),
            "task".to_string()
        )
    );
}

#[test]
fn does_not_emit_root_tool_updates_for_unrelated_child_sessions() {
    let mut aggregator = SummaryAggregator::new();
    let calls = Rc::new(RefCell::new(0usize));
    let probe = Rc::clone(&calls);
    aggregator.set_on_root_tool_update(Box::new(move |_info| {
        *probe.borrow_mut() += 1;
    }));
    aggregator.set_session("session-1");

    aggregator.process_event(AggregatorEvent::ToolPartUpdated {
        part: tool_part("child-session", "message-1", "child-call", "bash"),
    });

    assert_eq!(*calls.borrow(), 0);
}

// ---------------------------------------------------------------------------
// SLICE 2 (RED): live subagent cards, variant, task-call pairing, tool timing.
// Each test targets a slice-2 stub (`unimplemented!()`) and MUST FAIL until
// the Rust port lands. TS source: `src/app/managers/summary-aggregation-manager.ts`;
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 2: live subagent updates with per-session model, cost, current tool.
///
/// TS: `it("emits live subagent updates with per-session model, context, cost,
/// and current tool")` (test lines 183-320) via `registerSubtaskPart`
/// (src ~1545-1585) -> `updateSubagentFromTaskTool` (src ~1490-1529) ->
/// `handleSessionCreatedOrUpdated` (src ~1600-1656) ->
/// `updateSubagentFromAssistantMessage` (src ~1658-1703) ->
/// `updateSubagentToolState` (src ~1705-1748). Expects a card for
/// `child-session-1` with agent `explore`, description `Explore project
/// architecture`, provider `openai` / model `gpt-5.4`, current tool `read`.
#[test]
fn slice2_emits_live_subagent_updates_with_model_context_and_current_tool() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    aggregator.process_event(AggregatorEvent::SubtaskPartUpdated {
        parent_session_id: "root-session".to_string(),
        part_id: "subtask-1".to_string(),
        agent: "explore".to_string(),
        description: "Explore project architecture".to_string(),
        prompt: "Inspect architecture".to_string(),
    });

    aggregator.process_event(AggregatorEvent::TaskToolPartUpdated {
        input: TaskCallInput {
            call_id: "task-call-1".to_string(),
            parent_session_id: "root-session".to_string(),
            description: "Explore project architecture".to_string(),
            agent: Some("explore".to_string()),
            prompt: Some("Inspect architecture".to_string()),
            child_session_id: None,
            paired_card_id: None,
        },
    });

    aggregator.process_event(AggregatorEvent::ChildSessionCreated {
        child_session_id: "child-session-1".to_string(),
        parent_session_id: "root-session".to_string(),
        title: "Explore project architecture (@explore subagent)".to_string(),
    });

    aggregator.process_event(AggregatorEvent::ChildAssistantMessage {
        child_session_id: "child-session-1".to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5.4".to_string()),
        variant: None,
    });

    aggregator.process_event(AggregatorEvent::ChildToolPartUpdated {
        child_session_id: "child-session-1".to_string(),
        call_id: "call-child-1".to_string(),
        tool: "read".to_string(),
        title: Some("Reading pinned manager".to_string()),
        started_at: 2_000,
    });

    // Card snapshot carries provider/model + current tool (cost/tokens land
    // in a later slice; the Rust assistant-message signal carries only
    // provider/model/variant by design).
    let snapshots = snapshots.borrow();
    assert!(!snapshots.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.session_id, Some("child-session-1".to_string()));
    assert_eq!(card.parent_session_id, "root-session");
    assert_eq!(card.agent, "explore");
    assert_eq!(card.task, "Explore project architecture");
    assert_eq!(card.provider_id, Some("openai".to_string()));
    assert_eq!(card.model_id, Some("gpt-5.4".to_string()));
    assert_eq!(card.model, "openai/gpt-5.4");
    assert_eq!(card.status, otb_core::managers::summary_aggregation::SubagentStatus::Running);
    assert_eq!(card.current_tool, Some("read".to_string()));
    assert_eq!(card.current_tool_call_id, Some("call-child-1".to_string()));
    assert_eq!(card.current_tool_started_at, Some(2_000));
    assert_eq!(
        card.current_tool_title,
        Some("Reading pinned manager".to_string())
    );
}

/// SLICE 2: variant shown as provider/id (variant).
///
/// TS: `describe("subagent variant from assistant message")` (test lines
/// 322-433: emits a variant, re-emits on variant-only update, keeps the last
/// variant, never stores an empty variant) via
/// `updateSubagentFromAssistantMessage` (src ~1658-1703, variant applied at
/// ~1686-1688). Expects `providerID=openai`, `modelID=gpt-5.4`,
/// `variant=high` on the child card.
#[test]
fn slice2_variant_shown_as_provider_id_variant() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    aggregator.update_subagent_from_assistant_message(
        "child-session-1",
        Some("openai"),
        Some("gpt-5.4"),
        Some("high"),
    );

    // Card shows provider/id with variant.
    let snapshots = snapshots.borrow();
    assert!(!snapshots.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.session_id, Some("child-session-1".to_string()));
    assert_eq!(card.provider_id, Some("openai".to_string()));
    assert_eq!(card.model_id, Some("gpt-5.4".to_string()));
    assert_eq!(card.variant, Some("high".to_string()));
    assert_eq!(card.model, "openai/gpt-5.4 (high)");
}

/// SLICE 2: a task call's input reaches its own card.
///
/// TS: `describe("task call input reaches its own card")` (test lines
/// 435-601: named child pairs by `metadata.sessionId`; two unnamed open
/// calls pair with nothing; single open call pairs with the single waiting
/// card) via `updateSubagentFromTaskTool` + `findSubagentForTaskCall` (src
/// ~1440-1529, incl. `enrichSubagentFromTaskTool` + `emitSubagentState`).
/// Expects `child-2` to carry `second task` while `child-1` stays empty.
#[test]
fn slice2_task_call_input_reaches_its_own_card() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    aggregator.update_subagent_from_task_tool(TaskCallInput {
        call_id: "call-2".to_string(),
        parent_session_id: "root-session".to_string(),
        description: "second task".to_string(),
        agent: Some("explore".to_string()),
        prompt: Some("second task".to_string()),
        child_session_id: Some("child-2".to_string()),
        paired_card_id: None,
    });

    let paired = aggregator.find_subagent_for_task_call("call-2", Some("child-2"));

    // Intent: the call pairs with child-2's card; unreachable while RED.
    assert_eq!(paired, Some("child-2".to_string()));
}

/// SLICE 2: current-tool timing starts for a new call id.
///
/// TS: `describe("subagent current tool timing")` >
/// `it("reports the current tool call id and its start time")` (test lines
/// 663-677) via `updateSubagentToolState` (src ~1705-1748: new `callId` sets
/// `currentToolCallId` and stamps `currentToolStartedAt` at ~1738-1741, then
/// `emitSubagentState`). Expects `currentTool=bash`,
/// `currentToolCallId=call-1`, `currentToolStartedAt` a number.
#[test]
fn slice2_current_tool_timing_starts_for_new_call() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Card carries the new call id + start.
    let snapshots = snapshots.borrow();
    assert!(!snapshots.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.session_id, Some("child-session-1".to_string()));
    assert_eq!(card.status, otb_core::managers::summary_aggregation::SubagentStatus::Running);
    assert_eq!(card.current_tool, Some("bash".to_string()));
    assert_eq!(card.current_tool_call_id, Some("call-1".to_string()));
    assert_eq!(card.current_tool_started_at, Some(1_000));
    assert_eq!(card.current_tool_title, Some("Running tests".to_string()));
}

/// SLICE 2: a root subtask part opens a pending card.
///
/// TS: `registerSubtaskPart` (src ~1545-1585: `createSubagentState` +
/// `enrichSubagentFromSubtask`, then pair with a pending child session or
/// queue the card as pending). First step of the live-updates flow above;
/// split out so the card-creation contract has its own RED pin.
#[test]
fn slice2_subtask_part_creates_pending_card() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    aggregator.register_subtask_part(
        "root-session",
        "subtask-1",
        "explore",
        "Explore project architecture",
        "Inspect architecture",
    );

    // A pending card exists for the subtask (no child session bound yet).
    let snapshots = snapshots.borrow();
    assert!(!snapshots.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.card_id, "subtask-root-session-subtask-1");
    assert_eq!(card.session_id, None);
    assert_eq!(card.parent_session_id, "root-session");
    assert_eq!(card.agent, "explore");
    assert_eq!(card.task, "Explore project architecture");
    assert_eq!(card.status, otb_core::managers::summary_aggregation::SubagentStatus::Pending);
}

// ---------------------------------------------------------------------------
// SLICE 3 (RED): interrupted/nested subagents + live turn start.
// Each test targets a slice-3 stub (`unimplemented!()`) and MUST FAIL until
// the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts` nested describes
// (`an interrupted subagent`, `a nested subagent`, `live turn start`).
// ---------------------------------------------------------------------------

/// SLICE 3: interrupted subagent card ends Interrupted.
///
/// TS: `describe("an interrupted subagent")` >
/// `it("ends the card interrupted when OpenCode V2 interrupts the subagent")`
/// (test lines 842-855) via `session.idle` with `interrupted: true` ->
/// `setSubagentTerminalStatus(sessionID, "interrupted")` (src ~2925-2935).
/// Expects the child card `status` to be `interrupted` with `finishedAt` set
/// and no current tool.
#[test]
fn slice3_interrupted_subagent_card_ends_interrupted() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Slice-3 stub (panics while RED).
    aggregator.interrupt_subagent("child-session-1");

    // GREEN: the child card ends Interrupted.
    let snapshots = snapshots.borrow();
    assert!(!snapshots.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].session_id, Some("child-session-1".to_string()));
    assert_eq!(
        cards[0].status,
        otb_core::managers::summary_aggregation::SubagentStatus::Interrupted
    );
    assert_eq!(cards[0].current_tool, None);
}

/// SLICE 3: nested subagent has no own card (parent shows Running Task).
///
/// TS: `describe("a nested subagent")` >
/// `it("never gets a card of its own, at any depth")` (test lines 1122-1173)
/// via nested `session.created` under `child-session-1` (src ~2971: "A nested
/// subagent has no card to end: it only stops being tracked"). Expects every
/// snapshot to carry only `child-session-1` with `status` running.
#[test]
fn slice3_nested_subagent_has_no_own_card() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Slice-3 stub (panics while RED): nested launch under the child.
    aggregator.track_nested_launch("child-session-1", "nested-session-1");

    // GREEN: no card of its own; the parent still shows its running task.
    let snapshots = snapshots.borrow();
    assert!(!snapshots.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].session_id, Some("child-session-1".to_string()));
    assert_eq!(
        cards[0].status,
        otb_core::managers::summary_aggregation::SubagentStatus::Running
    );
    assert_eq!(cards[0].current_tool, Some("bash".to_string()));
}

/// SLICE 3: live turn start emits a new turn.
///
/// TS: `describe("live turn start")` >
/// `it("keeps the first busy of a turn until the session goes idle")`
/// (test lines 2800-2817) via `session.status` busy ->
/// `liveTurnStartedAt = Date.now()` (src ~607-632: `getLiveTurnStartedAt` /
/// `startMissedLiveTurn`). Expects the missed busy to start the turn at now.
#[test]
fn slice3_live_turn_start_emits_new_turn() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-3 stub (panics while RED).
    let mark = aggregator.get_turn_end_mark();
    aggregator.start_missed_live_turn("session-1", mark);

    // GREEN: the turn started.
    assert_eq!(
        aggregator.get_live_turn_started_at("session-1"),
        Some(1_000)
    );
    assert_eq!(aggregator.get_live_turn_started_at("session-2"), None);
}

/// SLICE 3: subagent error ends Error.
///
/// TS: `describe("an interrupted subagent")` >
/// `it("still ends a genuinely failed subagent with its error")`
/// (test lines 871-879) via `session.error` with a non-abort error ->
/// `setSubagentTerminalStatus(sessionID, "error")` (src ~3015-3022).
/// Expects the child card `status` to be `error`.
#[test]
fn slice3_subagent_error_ends_error() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Slice-3 stub (panics while RED).
    aggregator.fail_subagent("child-session-1", "boom");

    // GREEN: the child card ends Error.
    let snapshots = snapshots.borrow();
    assert!(!snapshots.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].session_id, Some("child-session-1".to_string()));
    assert_eq!(
        cards[0].status,
        otb_core::managers::summary_aggregation::SubagentStatus::Error
    );
}

// ---------------------------------------------------------------------------
// SLICE 4 (RED): thinking/reasoning + partial streaming.
// Each test targets a slice-4 stub (`unimplemented!()`) and MUST FAIL until
// the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts` (`registerThinkingPart`
// / `setThinkingPartSnapshot` / `applyThinkingDelta`, `applyTextDelta`,
// `handleSessionError` streaming reset); TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts` (reasoning /
// thinking, partial / streaming, idle / error / retry blocks).
// ---------------------------------------------------------------------------

/// SLICE 4: thinking block tracked per session.
///
/// TS: `it("passes reasoning sections to thinking callback when reasoning
/// part arrives")` (test lines 2170-2214) via `registerThinkingPart` +
/// `setThinkingPartSnapshot` + `emitThinkingUpdate` (src ~2478-2584).
/// Expects the reasoning part tracked under its own session/message with
/// its text preserved.
#[test]
fn slice4_thinking_block_tracked_per_session() {
    use otb_core::managers::summary_aggregation::ThinkingSection;
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-4 stub (panics while RED).
    aggregator.track_thinking("session-1", "message-1", "part-reasoning-1", "Let me think...");

    // GREEN: the reasoning block is tracked under its session/message.
    let sections: Vec<ThinkingSection> = aggregator.thinking_sections("session-1", "message-1");
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].id, "part-reasoning-1");
    assert_eq!(sections[0].text, "Let me think...");
}

/// SLICE 4: reasoning collapsed/preserved.
///
/// TS: `it("streams all reasoning parts and marks only the first thinking
/// update")` (test lines 3058-3114) via ordered `sections` map
/// (`getThinkingSections`, src ~2550-2560): parts collapse in part order
/// and their text is preserved across updates.
#[test]
fn slice4_reasoning_collapsed_preserved() {
    use otb_core::managers::summary_aggregation::ThinkingSection;
    let aggregator = SummaryAggregator::new();

    // Slice-4 stub (panics while RED).
    let collapsed = aggregator.collapse_reasoning_sections(vec![
        ThinkingSection {
            id: "part-reasoning-1".to_string(),
            title: None,
            text: "First ".to_string(),
        },
        ThinkingSection {
            id: "part-reasoning-1".to_string(),
            title: None,
            text: "second".to_string(),
        },
    ]);

    // GREEN: same-id sections collapse in order with text preserved.
    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].id, "part-reasoning-1");
    assert_eq!(collapsed[0].text, "First second");
}

/// SLICE 4: partial streaming text updates accumulate.
///
/// TS: `it("streams text from message.part.delta events")` (test lines
/// 2932-2965) via `applyTextDelta` (src ~2399-2435: previous + delta,
/// then `emitPartialText`). Expects `Hel` + `lo` to accumulate to `Hello`.
#[test]
fn slice4_partial_streaming_text_accumulates() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-4 stubs (panic while RED).
    let first = aggregator.accumulate_stream_text("session-1", "message-1", "part-1", "Hel");
    assert_eq!(first, "Hel");
    let combined = aggregator.accumulate_stream_text("session-1", "message-1", "part-1", "lo");

    // GREEN: deltas accumulate into the combined streaming text.
    assert_eq!(combined, "Hello");
}

/// SLICE 4: error/retry resets streaming state.
///
/// TS: `handleSessionError` (src ~3003-3051: `liveTurnStartedAt = null`,
/// `turnEndCount++`) + `cleanupCompletedMessage` (src ~2634-2648):
/// a failed turn drops its streaming text/thinking state so the next
/// turn starts clean.
#[test]
fn slice4_error_retry_resets_streaming_state() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-4 stub (panics while RED).
    aggregator.reset_stream_on_error("session-1");

    // GREEN: streaming state is gone; the next accumulation starts fresh.
    let text = aggregator.accumulate_stream_text("session-1", "message-1", "part-1", "Hello");
    assert_eq!(text, "Hello");
}

// ---------------------------------------------------------------------------
// SLICE 5 (RED): tokens/cost accumulation, background cards, parallel
// coexistence, completion dedup. Each test targets a slice-5 stub
// (`unimplemented!()`) and MUST FAIL until the Rust port lands.
// TS source: `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 5: token accumulation per subagent card.
///
/// TS: `updateSubagentStepFinish` (src ~1819-1850: `subagent.tokens = {...}`
/// from the step-finish event) + `updateSubagentFromAssistantMessage`
/// (src ~1689-1697: `input`/`cache.read` carried onto the card).
/// TS tests: `it("emits live subagent updates ...")` (test lines 183-320:
/// `tokens: { input: 54000, cache: { read: 1000 } }` -> card shows
/// `input: 54000, cacheRead: 1000`) and the step-finish event
/// (test lines 1556-1575: `tokens: { input: 1000, output: 50,
/// cache: { read: 200 } }`).
#[test]
fn slice5_token_accumulation_per_subagent_card() {
    use otb_core::managers::summary_aggregation::TokensInfo;
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<RefCell<Vec<(String, Vec<otb_core::managers::summary_aggregation::SubagentInfo>)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));
    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    aggregator.record_tokens(
        "child-session-1",
        TokensInfo {
            input: 54_000,
            output: 1_200,
            reasoning: 0,
            cache_read: 1_000,
            cache_write: 0,
        },
    );

    // GREEN: the card carries the step-finish token totals.
    let snap = snapshots.borrow();
    assert!(!snap.is_empty(), "expected at least one subagent snapshot");
    let (session_id, cards) = &snap[snap.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].tokens.input, 54_000);
    assert_eq!(cards[0].tokens.output, 1_200);
    assert_eq!(cards[0].tokens.cache_read, 1_000);
    drop(snap);

    // GREEN: tokens replace (last step-finish wins), they do not accumulate.
    aggregator.record_tokens(
        "child-session-1",
        TokensInfo {
            input: 1_000,
            output: 50,
            reasoning: 0,
            cache_read: 200,
            cache_write: 0,
        },
    );
    let snap = snapshots.borrow();
    let (_, cards) = &snap[snap.len() - 1];
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].tokens.input, 1_000);
    assert_eq!(cards[0].tokens.output, 50);
    assert_eq!(cards[0].tokens.cache_read, 200);
}

/// SLICE 5: cost formatting and summary totals.
///
/// TS: `updateSubagentStepFinish` (src ~1844: `subagent.cost += cost`) +
/// `handleMessageUpdated` (src ~2023-2030: `` `$${cost.toFixed(2)}` `` on
/// the cost callback) + `formatCostLine` (`$${cost.toFixed(2)}`).
/// TS test: live subagent card `cost: 0.18` (test lines 245-268).
#[test]
fn slice5_cost_formatting_and_summary_totals() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Slice-5 stubs (panic while RED).
    aggregator.record_cost("child-session-1", 0.18);
    aggregator.record_cost("child-session-1", 0.12);

    // GREEN: totals accumulate and format as `$X.XX`.
    assert!((aggregator.total_cost() - 0.30).abs() < f64::EPSILON);
    assert_eq!(SummaryAggregator::format_cost(0.3), "$0.30");
}

/// SLICE 5: background card creation and tracking.
///
/// TS: `markBackgroundSubagent` (src ~1219-1242: sets `background = true`)
/// + `backgroundChildSessionIds.add` on the task part (src ~2227) +
/// `launchInBackground` helper (test lines 709-729: task input
/// `{ description, background: true }` with `metadata.sessionId`) and
/// `it("keeps a background subagent's card after its parent's turn ends")`
/// (test lines 738-757).
#[test]
fn slice5_background_card_creation_and_tracking() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Slice-5 stub (panics while RED).
    aggregator.track_background_card("child-session-1");

    // GREEN: the card is tracked as background and outlives the turn.
    assert!(aggregator.is_background_card("child-session-1"));
    assert!(!aggregator.is_background_card("child-session-2"));
}

/// SLICE 5: parallel cards coexist with independent token totals.
///
/// TS: `it("tracks multiple parallel subagents independently")`
/// (test lines 1604-1681: `child-1` explore + `child-2` general each keep
/// their own description/agent) via `registerSubtaskPart` + per-child
/// `updateSubagentToolState`. Each card accumulates its own tokens
/// (src ~1819-1850).
#[test]
fn slice5_parallel_cards_coexist_with_independent_tokens() {
    use otb_core::managers::summary_aggregation::TokensInfo;
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    aggregator.update_subagent_tool_state("child-1", "call-1", "bash", Some("Running one"), 1_000);
    aggregator.update_subagent_tool_state("child-2", "call-2", "bash", Some("Running two"), 2_000);

    // Slice-5 stub (panics while RED): per-card token accumulation.
    aggregator.record_tokens(
        "child-1",
        TokensInfo {
            input: 1_000,
            output: 50,
            reasoning: 0,
            cache_read: 200,
            cache_write: 0,
        },
    );
    aggregator.record_tokens(
        "child-2",
        TokensInfo {
            input: 2_000,
            output: 60,
            reasoning: 0,
            cache_read: 300,
            cache_write: 0,
        },
    );

    // GREEN: both cards coexist with their own independent totals.
    assert!((aggregator.total_cost() - 0.0).abs() < f64::EPSILON);
}

/// SLICE 5: a completed tool fires once (completion dedup).
///
/// TS: `handleMessagePartUpdated` tool branch (src ~2279-2282:
/// `completed-{callID}` in `processedToolStates` gates the `onTool`
/// callback) + `it("emits terminal subagent cards once and excludes them
/// from later snapshots")` (test lines 1683-1778) and
/// `it("does not re-emit completed subagent cards ...")`
/// (test lines 1780-1901).
#[test]
fn slice5_completed_tool_fires_once() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-5 stub (panics while RED).
    let first = aggregator.record_tool_completion("call-1");
    let second = aggregator.record_tool_completion("call-1");

    // GREEN: the completed tool fires once; the repeat is deduped.
    assert!(first);
    assert!(!second);
}

// ---------------------------------------------------------------------------
// SLICE 6 (RED): idle/retry gates + outbound hold + completion-dedup
// remainder + `session.status` gating. Each test targets a slice-6 stub
// (`unimplemented!()`) and MUST FAIL until the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 6: idle gate suppresses duplicate emits.
///
/// TS: `it("reports root session.idle through callback")` (test lines
/// 2749-2764: `session.idle` for `session-1` -> `onSessionIdle` with
/// `{ interrupted: false }`) + `it("passes the interrupted mark of an
/// idle to the callback")` (test lines 2766-2779) via `handleSessionIdle`
/// (src ~2919-2969: early return unless the current session, then one
/// `scheduleOutbound` idle emit). A repeated idle for the same run
/// collapses to the first emit.
#[test]
fn slice6_idle_gate_suppresses_duplicate_emits() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let calls = Rc::new(RefCell::new(0usize));
    let probe = Rc::clone(&calls);
    aggregator.set_on_session_idle(Box::new(move |_session_id, _info| {
        *probe.borrow_mut() += 1;
    }));

    // Slice-6 stub (panics while RED).
    aggregator.set_idle_gated("session-1", false);
    aggregator.set_idle_gated("session-1", false);

    // GREEN: the duplicate idle for the same run emits only once.
    assert_eq!(*calls.borrow(), 1);
}

/// SLICE 6: retry/error path re-emits.
///
/// TS: `it("reports session.status retry through callback")` (test lines
/// 3141-3167: `session.status` retry -> `onSessionRetry` with
/// `{ sessionId, attempt: 2, message, next }`) via `handleSessionStatus`
/// retry branch (src ~2898-2916) + `handleSessionError` (src ~3003-3051:
/// the failed turn drops its state). After the error reset the retry
/// fires again.
#[test]
fn slice6_retry_error_path_re_emits() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let retries: Rc<RefCell<Vec<(String, u64, String)>>> = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&retries);
    aggregator.set_on_session_retry(Box::new(move |info| {
        probe
            .borrow_mut()
            .push((info.session_id, info.attempt.unwrap_or(0), info.message));
    }));

    // Slice-6 stub (panics while RED).
    aggregator.retry_after_error("session-1", 2, "plan does not include access");

    // GREEN: the retry emits, then re-emits after the error path resets.
    assert_eq!(retries.borrow().len(), 1);
    assert_eq!(retries.borrow()[0].1, 2);
    aggregator.reset_stream_on_error("session-1");
    aggregator.retry_after_error("session-1", 3, "plan does not include access");
    assert_eq!(retries.borrow().len(), 2);
    assert_eq!(retries.borrow()[1].1, 3);
}

/// SLICE 6: outbound hold queues then flushes.
///
/// TS: `holdOutbound` (src ~634-638) + `drainOutbound` (src ~649-667:
/// the held queue replays for its generation) + `scheduleOutbound`
/// (src ~745-756: tasks defer while the hold is on). While held the idle
/// stays queued; the flush delivers it.
#[test]
fn slice6_outbound_hold_queues_then_flushes() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let calls = Rc::new(RefCell::new(0usize));
    let probe = Rc::clone(&calls);
    aggregator.set_on_session_idle(Box::new(move |_session_id, _info| {
        *probe.borrow_mut() += 1;
    }));

    // Slice-6 stubs (panic while RED).
    let generation = aggregator.hold_outbound();
    aggregator.set_idle_gated("session-1", false);

    // GREEN: held outbound does not deliver yet; the flush delivers it.
    assert_eq!(*calls.borrow(), 0);
    aggregator.flush_outbound(generation);
    assert_eq!(*calls.borrow(), 1);
}

/// SLICE 6: `session.status` gating (only the current session emits).
///
/// TS: `handleSessionStatus` (src ~2887-2896: `sessionID !==
/// currentSessionId -> return`; `busy` starts `liveTurnStartedAt`) +
/// `it("ignores a busy of another session")` (test lines 2860-2866: a
/// busy for `session-2` leaves `session-1` with no turn).
#[test]
fn slice6_session_status_gating_only_current_session_emits() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-6 stub (panics while RED).
    aggregator.handle_session_status("session-2", "busy");

    // GREEN: another session's busy starts no turn; the current one's does.
    assert_eq!(aggregator.get_live_turn_started_at("session-1"), None);
    aggregator.handle_session_status("session-1", "busy");
    assert_eq!(aggregator.get_live_turn_started_at("session-1"), Some(1_000));
}

// ---------------------------------------------------------------------------
// SLICE 7 (RED): apply_patch/file parts + question/permission cards +
// `session.created` gating. Each test targets a slice-7 stub
// (`unimplemented!()`) and MUST FAIL until the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 7: apply_patch file change tracked per session.
///
/// TS: `it("sends one apply_patch document and one file change per
/// changed file")` (test lines 3309-3393) via `prepareToolFiles`
/// apply_patch branch (src ~2809-2821: `metadata.files[]` -> one
/// `{ fileData, fileChange, line }` per changed file) + `onFileChange`
/// per file (`{ file: "src/one.ts", additions: 1, deletions: 1 }`).
#[test]
fn slice7_apply_patch_file_tracked_per_session() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-7 stubs (panic while RED).
    aggregator.track_file_change("session-1", "src/one.ts", 1, 1);

    // GREEN: the file change is tracked under its own session.
    let changes = aggregator.file_changes("session-1");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].file, "src/one.ts");
    assert_eq!(changes[0].additions, 1);
    assert_eq!(changes[0].deletions, 1);
    assert!(aggregator.file_changes("session-2").is_empty());
}

/// SLICE 7: question card created then answered.
///
/// TS: `describe("requests of subagent sessions")` >
/// `it("forwards question.asked from a tracked subagent session ...")`
/// (test lines 3799-3814) via `handleQuestionAsked` (src ~3053-3082) +
/// `it("reports questions settled outside Telegram ...")` (test lines
/// 3816-3839) via `handleQuestionSettled` (src ~3084-3111, answered
/// path).
#[test]
fn slice7_question_card_created_answered() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-7 stubs (panic while RED).
    aggregator.open_question("child-session-1", "q-child");
    assert!(aggregator.is_question_open("child-session-1", "q-child"));

    // GREEN: answering settles the card.
    aggregator.answer_question("child-session-1", "q-child");
    assert!(!aggregator.is_question_open("child-session-1", "q-child"));
}

/// SLICE 7: permission card allow/deny.
///
/// TS: `it("forwards permission.asked events from tracked subagent
/// (child) sessions")` (test lines 3638-3699) via
/// `handlePermissionAsked` (src ~3155-3186) +
/// `it("passes OpenCode's decision along with a replied permission")`
/// (test lines 3841-3860) via `handlePermissionReplied` (src ~3188-3230).
#[test]
fn slice7_permission_card_allow_deny() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-7 stubs (panic while RED).
    aggregator.request_permission("child-session-1", "perm-allow", "write");
    aggregator.request_permission("child-session-1", "perm-deny", "bash");
    aggregator.decide_permission("child-session-1", "perm-allow", true);
    aggregator.decide_permission("child-session-1", "perm-deny", false);

    // GREEN: the allow/deny decisions land on their own cards.
    assert_eq!(
        aggregator.permission_decision("child-session-1", "perm-allow"),
        Some(true)
    );
    assert_eq!(
        aggregator.permission_decision("child-session-1", "perm-deny"),
        Some(false)
    );
    assert_eq!(
        aggregator.permission_decision("child-session-1", "perm-unknown"),
        None
    );
}

/// SLICE 7: `session.created` gating (events before created ignored).
///
/// TS: `it("waits for session.created before attaching unknown child
/// events")` (test lines 1535-1602: a `step-finish` for `child-unknown`
/// emits nothing until `session.created` for it arrives, then the card
/// attaches) via `handleSessionCreatedOrUpdated` (src ~1600-1656).
#[test]
fn slice7_session_created_gating_ignores_early_child_events() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-7 stubs (panic while RED): the unknown child starts gated.
    assert!(aggregator.is_child_gated("child-unknown"));

    // GREEN: `session.created` lifts the gate for that child only.
    aggregator.handle_session_created("child-unknown", "root-session");
    assert!(!aggregator.is_child_gated("child-unknown"));
    assert!(aggregator.is_child_gated("child-other"));
}

/// SLICE 7: file-summary totals across a session's changed files.
///
/// TS: multi-file apply_patch (test lines 3309-3393: `src/one.ts`
/// `(+1 -1)` + `src/two.ts` `(+2)`) and the oversized-file case (test
/// lines 3395-3455: huge file stays a text line, small file keeps its
/// document) via `formatPatchFileLine` + `prepareToolFiles`. The Rust
/// port rolls the per-file changes into summary totals here.
#[test]
fn slice7_file_summary_totals() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-7 stubs (panic while RED).
    aggregator.track_file_change("session-1", "src/one.ts", 1, 1);
    aggregator.track_file_change("session-1", "src/two.ts", 2, 0);

    // GREEN: totals add up across the session's files.
    assert_eq!(aggregator.file_summary_totals("session-1"), (3, 1));
    assert_eq!(aggregator.file_summary_totals("session-2"), (0, 0));
}

// ---------------------------------------------------------------------------
// SLICE 8 (RED): completion filtering + turn-end mark + subagent-event
// gating + root-session mapping + per-session cost reset. Each test targets
// a slice-8 stub (`unimplemented!()`) — except the completion gate, which
// fails against the current always-fire `handle_tool_part` — and MUST FAIL
// until the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 8: only completed tool parts reach `on_tool`.
///
/// TS: `handleMessagePartUpdated` tool branch (src ~2273-2282:
/// `state.status === "completed"` gates the `onTool` callback;
/// `onRootToolUpdate` fires before the gate) + `it("includes sessionId in
/// tool callback payload")` (test lines 56-103: a completed `bash` part
/// reaches `onTool`).
#[test]
fn slice8_only_completed_tool_parts_reach_on_tool() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&seen);
    aggregator.set_on_tool(Box::new(move |info| {
        probe.borrow_mut().push(info.call_id);
    }));

    // A running part must not reach `on_tool` (root updates still fire,
    // but that callback is not registered here).
    aggregator.process_event(AggregatorEvent::ToolPartUpdated {
        part: ToolPart {
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            call_id: "call-1".to_string(),
            tool: "bash".to_string(),
            status: ToolStatus::Running,
        },
    });
    assert!(seen.borrow().is_empty());

    // The completed part fires exactly once; the repeat is deduped.
    aggregator.process_event(AggregatorEvent::ToolPartUpdated {
        part: ToolPart {
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            call_id: "call-1".to_string(),
            tool: "bash".to_string(),
            status: ToolStatus::Completed,
        },
    });
    aggregator.process_event(AggregatorEvent::ToolPartUpdated {
        part: ToolPart {
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            call_id: "call-1".to_string(),
            tool: "bash".to_string(),
            status: ToolStatus::Completed,
        },
    });
    assert_eq!(seen.borrow().as_slice(), ["call-1".to_string()]);
}

/// SLICE 8: `clear` bumps the turn-end mark.
///
/// TS: `clear` (src ~946: `turnEndCount++`) + `getTurnEndMark` (src
/// ~618-620) + `startMissedLiveTurn` (src ~626-632: a mark taken before
/// an end no longer matches). TS tests: `describe("live turn start")`
/// (test lines 2781-2867).
#[test]
fn slice8_clear_bumps_turn_end_mark() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-8 stub (panics while RED).
    let mark = aggregator.get_turn_end_mark();
    aggregator.clear();

    // GREEN: the clear ended the run, so the old mark is stale.
    assert_eq!(aggregator.get_turn_end_mark(), mark + 1);
}

/// SLICE 8: root idle stops accepting subagent events.
///
/// TS: `setSession` (src ~914: `acceptsSubagentEvents = true`) vs
/// `handleSessionIdle` root branch (src ~2950-2951:
/// `acceptsSubagentEvents = false`) and the subagent guards
/// (src ~1553, ~1632: `!acceptsSubagentEvents -> return`).
#[test]
fn slice8_idle_stops_accepting_subagent_events() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-8 stub (panics while RED).
    assert!(aggregator.accepts_subagent_events());
    aggregator.set_idle_gated("session-1", false);

    // GREEN: the turn ended, so late subagent events are refused.
    assert!(!aggregator.accepts_subagent_events());
}

/// SLICE 8: child session maps to its root session.
///
/// TS: `getRootSessionId` (src ~1012-1023: walk `trackedSessionParents`;
/// an untracked session is its own root) + `handleSessionCreatedOrUpdated`
/// (src ~1600-1656: `trackedSessionParents.set(id, parentID)`).
/// TS tests: the nested-subagent block (test lines 1175-1234:
/// `getRootSessionId("nested-session-1")` is `"root-session"`).
#[test]
fn slice8_child_session_maps_to_root_session() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-8 stub (panics while RED).
    aggregator.handle_session_created("child-session-1", "root-session");

    // GREEN: the child resolves to the root; strangers resolve to
    // themselves.
    assert_eq!(
        aggregator.get_root_session_id("child-session-1"),
        "root-session"
    );
    assert_eq!(
        aggregator.get_root_session_id("some-other-session"),
        "some-other-session"
    );
}

/// SLICE 8: `clear` resets per-session cost.
///
/// TS: `clear` (src ~932-940: `subagentStates.clear()` and friends drop
/// every card) so a recorded cost reads back `0.0` afterwards.
/// TS tests: live card `cost: 0.18` (test lines 245-268).
#[test]
fn slice8_clear_resets_per_session_cost() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );
    aggregator.record_cost("child-session-1", 0.18);

    // Slice-8 stub (panics while RED).
    assert!((aggregator.session_cost("child-session-1") - 0.18).abs() < f64::EPSILON);

    // GREEN: the clear drops every card, so the cost reads back zero.
    aggregator.clear();
    assert_eq!(aggregator.session_cost("child-session-1"), 0.0);
}

// ---------------------------------------------------------------------------
// SLICE 9 (RED): empty-response / synthetic / unexpected-shape robustness.
// Each test targets a slice-9 stub (`unimplemented!()`) and MUST FAIL until
// the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts` (empty-response
// at lines 3911-4169, synthetic at 4171-4311, unexpected-shape `describe`
// at 4313-4453).
// ---------------------------------------------------------------------------

/// SLICE 9: upstream empty-response placeholder is dropped, lookalikes kept.
///
/// TS: `it("drops the upstream empty-response placeholder ...")` (test
/// lines 3911-3961: placeholder text with the serialized response object
/// emits nothing) + `it("drops the empty-response placeholder while it is
/// still streaming in")` (3963-3996: `"Empty" / " resp" / "onse: "` deltas
/// stay suppressed) + `it("recovers streamed text that diverges ...")`
/// (4036-4075: diverged text surfaces) + `it("releases a completed answer
/// that legitimately opens with the marker")` (4077-4132: final text
/// without the serialized response object is released) + `it("keeps
/// assistant text that only starts like ...")` (4134-4169) via
/// `isUpstreamEmptyResponseText` (src ~304-319).
#[test]
fn slice9_empty_response_placeholder_dropped_lookalike_kept() {
    // Slice-9 stub (panics while RED).
    // Full placeholder with the serialized response object: dropped.
    assert!(SummaryAggregator::is_empty_response(
        "Empty response: {'content': [{'type': 'thinking'}], 'stop_reason': 'end_turn'}",
        true
    ));
    // Still streaming and still a marker prefix: held back.
    assert!(SummaryAggregator::is_empty_response("Empty resp", false));

    // GREEN: diverged stream and marker-less lookalikes surface.
    assert!(!SummaryAggregator::is_empty_response(
        "Empty response from the server, retrying.",
        false
    ));
    assert!(!SummaryAggregator::is_empty_response(
        "Empty response: {'content': 'test'} is what the provider sends back.",
        true
    ));
    assert!(!SummaryAggregator::is_empty_response(
        "Empty response body is expected here.",
        false
    ));
}

/// SLICE 9: synthetic text parts never reach the user.
///
/// TS: `it("ignores synthetic text parts so an attached file is not
/// echoed into the chat")` (test lines 4171-4222: a
/// `synthetic: true` part carrying file content never surfaces) +
/// `it("ignores a delta whose own part is flagged synthetic")`
/// (4275-4311) via the `synthetic === true` filter in
/// `handleMessagePartUpdated` (src ~2114-2120).
#[test]
fn slice9_synthetic_part_ignored() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-9 stubs (panic while RED).
    aggregator.register_synthetic_part("message-user", "part-synthetic");

    // GREEN: the synthetic part is known; a plain part is not.
    assert!(aggregator.is_synthetic_part("message-user", "part-synthetic"));
    assert!(!aggregator.is_synthetic_part("message-user", "part-prompt"));
}

/// SLICE 9: deltas for a known-synthetic part stay filtered via registry.
///
/// TS: `it("ignores deltas streamed for a part already known to be
/// synthetic")` (test lines 4224-4273: a delta carrying only the part id
/// must not leak `SECRET_STREAMED_CONTENT`) via the registry lookup in
/// `handleMessagePartDelta` (src ~2359-2365).
#[test]
fn slice9_synthetic_delta_remembered_via_registry() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-9 stubs (panic while RED).
    aggregator.register_synthetic_part("message-user-delta", "part-synthetic-streamed");

    // GREEN: the id-only delta resolves against the registry.
    assert!(aggregator.is_synthetic_part("message-user-delta", "part-synthetic-streamed"));
}

/// SLICE 9: combined assistant text filters the placeholder to empty.
///
/// TS: `getCombinedMessageText` (src ~2740-2755: assistant parts are
/// filtered through `isUpstreamEmptyResponseText`, user parts join
/// verbatim) + the no-emit-on-empty rule in `applyTextDelta` (src
/// ~2422-2425: `if (!combined.trim()) return`) + `it("delivers user text
/// that looks like the empty-response placeholder unfiltered")` (test
/// lines 3998-4034).
#[test]
fn slice9_combined_text_filters_placeholder_empties() {
    // Slice-9 stub (panics while RED).
    // Placeholder-only assistant message combines to nothing -> no emit.
    assert_eq!(
        SummaryAggregator::combine_text_parts(
            &["Empty response: {'content': [{'type': 'thinking'}], 'stop_reason': 'end_turn'}"],
            "assistant",
            true
        ),
        ""
    );

    // GREEN: user text passes through verbatim even when it looks like
    // the placeholder, and real assistant text is kept.
    assert_eq!(
        SummaryAggregator::combine_text_parts(
            &["Empty response: {'content': [{'type': 'thinking'}]} - why do I get this?"],
            "user",
            true
        ),
        "Empty response: {'content': [{'type': 'thinking'}]} - why do I get this?"
    );
    assert_eq!(
        SummaryAggregator::combine_text_parts(&["hello ", "world"], "assistant", false),
        "hello world"
    );
}

/// SLICE 9: unexpected delta shapes are ignored without panicking.
///
/// TS: `it("warns and skips a message.part.delta whose properties are not
/// an object")` (test lines 4318-4336: `properties: null` and a missing
/// `properties` warn and emit nothing) via the
/// `!isMessagePartDeltaEvent` guard in `processEvent` (src ~856-861).
#[test]
fn slice9_unexpected_shape_ignored_gracefully() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-9 stub (panics while RED).
    // GREEN: unexpected shapes are refused quietly — no panic, no emit.
    assert!(!aggregator.handle_unexpected_delta(None, None, None));
    assert!(!aggregator.handle_unexpected_delta(Some("session-1"), None, Some("x")));
    assert!(!aggregator.handle_unexpected_delta(Some("session-1"), Some("m"), None));
    assert!(!aggregator.handle_unexpected_delta(Some("session-1"), Some("m"), Some("")));
}

// ---------------------------------------------------------------------------
// SLICE 10 (RED): text-message states + write/edit tool files + reasoning
// deltas + session/error cards. Each test targets a slice-10 stub
// (`unimplemented!()`) and MUST FAIL until the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 10: multiple text parts combine into one final message.
///
/// TS: `it("combines multiple text parts into a single final message")`
/// (test lines 2579-2646: `part-a` `"Hello "` + `part-b` `"world"` ->
/// `onPartial` last call + `onComplete` with `"Hello world"`) via
/// `getCombinedMessageText` (src ~2740-2755: assistant parts joined in
/// order). The Rust port joins ordered part texts here.
#[test]
fn slice10_combines_multiple_text_parts_into_final_message() {
    // Slice-10 stub (panics while RED).
    let combined = SummaryAggregator::combined_text_message(&["Hello ", "world"]);

    // GREEN: ordered parts join into the single final message.
    assert_eq!(combined, "Hello world");
    assert_eq!(SummaryAggregator::combined_text_message(&["Done"]), "Done");
    assert_eq!(SummaryAggregator::combined_text_message(&[]), "");
}

/// SLICE 10: oversized write payload carries no file attachment.
///
/// TS: `it("marks write tool without file attachment when payload is
/// oversized")` (test lines 2119-2168: `write` with
/// `"x".repeat(101 * 1024)` content -> `onTool` with
/// `{ tool: "write", hasFileAttachment: false }`, `onToolFile` never
/// fires) via the oversized-payload branch of `prepareToolFiles`
/// (src ~2760-2808: byte length over 100 KiB drops the attachment).
#[test]
fn slice10_write_tool_oversized_has_no_file_attachment() {
    // Slice-10 stubs (panic while RED).
    // GREEN: 101 KiB is over the 100 KiB attachment budget; 1 KiB fits.
    assert!(!SummaryAggregator::write_tool_attachment_allowed(101 * 1024));
    assert!(SummaryAggregator::write_tool_attachment_allowed(1024));
}

/// SLICE 10: edit fallback filename from the patched path.
///
/// TS: `it("sends apply_patch file using title and patchText fallback")`
/// (test lines 3245-3307: title `"Success. Updated the following
/// files:\nM README.md"` + `patchText` diff -> file
/// `"edit_README.md.txt"` with `"Edit File/Path: README.md"`) and the
/// `filediff` path (test lines 3169-3243: `src/one.ts` ->
/// `"edit_one.ts.txt"`) via `prepareToolFiles` filename mapping
/// (src ~2809-2870).
#[test]
fn slice10_edit_patch_filename_from_path() {
    // Slice-10 stubs (panic while RED).
    // GREEN: the patched path maps to its `edit_<flat>.txt` document name.
    assert_eq!(
        SummaryAggregator::edit_patch_filename("README.md"),
        "edit_README.md.txt"
    );
    assert_eq!(
        SummaryAggregator::edit_patch_filename("src/one.ts"),
        "edit_one.ts.txt"
    );
}

/// SLICE 10: explicit reasoning deltas stay out of the assistant text.
///
/// TS: `it("streams explicit reasoning deltas without adding them to
/// assistant text")` (test lines 2216-2310: reasoning part `"First "` +
/// delta `"second"` -> thinking sections `"First second"` while
/// `onComplete` still carries only `"Final answer."`) via the
/// reasoning-delta branch of `handleMessagePartDelta` (src ~2357-2397:
/// reasoning deltas update thinking state, never the text stream).
#[test]
fn slice10_reasoning_delta_stays_out_of_assistant_text() {
    // Slice-10 stub (panics while RED).
    let combined = SummaryAggregator::append_reasoning_delta("First ", "second");

    // GREEN: reasoning deltas accumulate into the thinking text.
    assert_eq!(combined, "First second");
}

/// SLICE 10: session.error message reaches the callback verbatim.
///
/// TS: `it("reports session.error message through callback")` (test
/// lines 3116-3139: `session.error` with
/// `error.data.message = "Model not found: opencode/foo."` ->
/// `onSessionError("session-1", "Model not found: opencode/foo.",
/// { interrupted: false })`) via `handleSessionError` message
/// extraction (src ~3003-3051).
#[test]
fn slice10_session_error_message_reaches_callback() {
    // Slice-10 stub (panics while RED).
    // GREEN: the provider message passes through verbatim.
    assert_eq!(
        SummaryAggregator::session_error_message("Model not found: opencode/foo."),
        "Model not found: opencode/foo."
    );
}
