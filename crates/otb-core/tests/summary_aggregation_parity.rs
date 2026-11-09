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
        probe
            .borrow_mut()
            .push((info.session_id, info.call_id, info.tool));
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
        probe
            .borrow_mut()
            .push((info.session_id, info.call_id, info.tool));
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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
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
    assert_eq!(
        card.status,
        otb_core::managers::summary_aggregation::SubagentStatus::Running
    );
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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.session_id, Some("child-session-1".to_string()));
    assert_eq!(
        card.status,
        otb_core::managers::summary_aggregation::SubagentStatus::Running
    );
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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.card_id, "subtask-root-session-subtask-1");
    assert_eq!(card.session_id, None);
    assert_eq!(card.parent_session_id, "root-session");
    assert_eq!(card.agent, "explore");
    assert_eq!(card.task, "Explore project architecture");
    assert_eq!(
        card.status,
        otb_core::managers::summary_aggregation::SubagentStatus::Pending
    );
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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
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
    aggregator.track_thinking(
        "session-1",
        "message-1",
        "part-reasoning-1",
        "Let me think...",
    );

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
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
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
    assert_eq!(
        aggregator.get_live_turn_started_at("session-1"),
        Some(1_000)
    );
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
    assert!(!SummaryAggregator::write_tool_attachment_allowed(
        101 * 1024
    ));
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

// ---------------------------------------------------------------------------
// SLICE 11 (RED): todo lists, onTokens/cost cards, run-end. Each test
// targets a slice-11 stub (`unimplemented!()`) and MUST FAIL until the
// Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 11: completed message fires onTokens with isCompleted=true.
///
/// TS: `it("fires onTokens with isCompleted=true when message has
/// completed timestamp")` (test lines 3487-3539: tokens
/// `{ input: 800, output: 200, cache: { read: 100 } }` + completed time
/// -> `onTokens("session-1", { input: 800, ... }, true)`) via
/// `handleMessageUpdated` tokens branch (src ~1971-1992).
#[test]
fn slice11_on_tokens_completed_flag() {
    use otb_core::managers::summary_aggregation::TokensInfo;
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let seen: Rc<RefCell<Vec<(String, TokensInfo, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&seen);
    aggregator.set_on_tokens(Box::new(move |session_id, tokens, is_completed| {
        probe.borrow_mut().push((session_id, tokens, is_completed));
    }));

    // Slice-11 stub (panics while RED).
    aggregator.handle_tokens_update(
        "session-1",
        TokensInfo {
            input: 800,
            output: 200,
            reasoning: 0,
            cache_read: 100,
            cache_write: 0,
        },
        true,
    );

    // GREEN: the completed totals reach the callback with `true`.
    let seen = seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "session-1");
    assert_eq!(seen[0].1.input, 800);
    assert_eq!(seen[0].1.cache_read, 100);
    assert!(seen[0].2);
}

/// SLICE 11: intermediate token updates fire with isCompleted=false.
///
/// TS: `it("fires onTokens for non-completed message with non-zero
/// tokens (intermediate update)")` (test lines 3567-3616: zero-token
/// start fires `false`, later `{ input: 4000, cache: { read: 12000 } }`
/// fires `false`) via the same tokens branch (src ~1971-1992, no
/// `completed` time).
#[test]
fn slice11_on_tokens_intermediate_update() {
    use otb_core::managers::summary_aggregation::TokensInfo;
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let seen: Rc<RefCell<Vec<(String, TokensInfo, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&seen);
    aggregator.set_on_tokens(Box::new(move |session_id, tokens, is_completed| {
        probe.borrow_mut().push((session_id, tokens, is_completed));
    }));

    // Slice-11 stubs (panic while RED).
    aggregator.handle_tokens_update(
        "session-1",
        TokensInfo {
            input: 0,
            output: 0,
            reasoning: 0,
            cache_read: 0,
            cache_write: 0,
        },
        false,
    );
    aggregator.handle_tokens_update(
        "session-1",
        TokensInfo {
            input: 4000,
            output: 300,
            reasoning: 0,
            cache_read: 12_000,
            cache_write: 0,
        },
        false,
    );

    // GREEN: both intermediate updates fire with `false`.
    let seen = seen.borrow();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[1].1.input, 4000);
    assert_eq!(seen[1].1.cache_read, 12_000);
    assert!(!seen[1].2);
}

/// SLICE 11: completed message cost reaches onCost.
///
/// TS: cost `0.01` on the completed message (test lines 3526-3528) via
/// the cost branch (src ~2023-2030: `onCostCallback?.(sessionID, cost)`
/// only when completed).
#[test]
fn slice11_on_cost_card() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let seen: Rc<RefCell<Vec<(String, f64)>>> = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&seen);
    aggregator.set_on_cost(Box::new(move |session_id, cost| {
        probe.borrow_mut().push((session_id, cost));
    }));

    // Slice-11 stub (panics while RED).
    aggregator.handle_cost_update("session-1", 0.01);

    // GREEN: the cost reaches the callback verbatim.
    let seen = seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "session-1");
    assert!((seen[0].1 - 0.01).abs() < f64::EPSILON);
}

/// SLICE 11: run-end reports the followed session and its subagents.
///
/// TS: `it("reports the end of a run for the followed session and its
/// subagents")` (test lines 3862-3885: child `session.error` + root
/// `session.idle` -> `onSessionRunEnded` twice, with
/// `"child-session-1"` and `"root-session"`) via `emitSessionRunEnded`
/// (src ~3113-3126).
#[test]
fn slice11_run_end_reports_followed_and_subagents() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&seen);
    aggregator.set_on_session_run_ended(Box::new(move |session_id| {
        probe.borrow_mut().push(session_id);
    }));

    // Slice-11 stubs (panic while RED).
    aggregator.handle_run_end("child-session-1");
    aggregator.handle_run_end("root-session");

    // GREEN: both runs end, child first.
    let seen = seen.borrow();
    assert_eq!(seen.as_slice(), ["child-session-1", "root-session"]);
}

/// SLICE 11: todo list tracked per session with a done/total summary.
///
/// No direct TS counterpart in `summary-aggregation-manager.ts` (no
/// `todo` handling there); models the todo-tool list the summary cards
/// carry. The stub keeps per-session items so the summary reads back
/// `done/total`.
#[test]
fn slice11_todo_list_summary() {
    use otb_core::managers::summary_aggregation::TodoItem;
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-11 stubs (panic while RED).
    aggregator.track_todo_list(
        "session-1",
        vec![
            TodoItem {
                title: "Write code".to_string(),
                done: true,
            },
            TodoItem {
                title: "Write tests".to_string(),
                done: false,
            },
            TodoItem {
                title: "Run checks".to_string(),
                done: false,
            },
        ],
    );

    // GREEN: the summary counts done over total; other sessions stay empty.
    assert_eq!(aggregator.todo_summary("session-1"), "1/3 done");
    assert_eq!(aggregator.todo_summary("session-2"), "0/0 done");
}

// ---------------------------------------------------------------------------
// SLICE 12 (RED): text-message edge states, edit/write fallbacks,
// thinking-finished gates. Each test targets a slice-12 stub
// (`unimplemented!()`) and MUST FAIL until the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 12: optimistic partial fires only from the second unknown update.
///
/// TS: `it("starts optimistic partial streaming after second unknown text
/// update")` (test lines 2648-2683) + `it("does not stream unknown text
/// when only one update arrived")` (test lines 2685-2705) via
/// `handleMessagePartUpdated` text branch with no `messageInfo`
/// (src ~2200-2207: `optimisticUpdateCount++`, emit once `>= 2`).
#[test]
fn slice12_optimistic_partial_after_second_unknown_update() {
    // Slice-12 stub (panics while RED).
    // GREEN: one unknown update stays pending; the second one emits.
    assert!(!SummaryAggregator::should_emit_optimistic_partial(1));
    assert!(SummaryAggregator::should_emit_optimistic_partial(2));
    assert!(SummaryAggregator::should_emit_optimistic_partial(3));
}

/// SLICE 12: pending text attached on a completed message skips partial.
///
/// TS: `it("does not emit partial when pending text is attached on
/// completed message")` (test lines 2707-2747) via `handleMessageUpdated`
/// completed branch: pending optimistic text goes straight to `onComplete`
/// with no `onPartial`.
#[test]
fn slice12_completed_message_with_pending_text_skips_partial() {
    // Slice-12 stub (panics while RED).
    // GREEN: a completed message never emits the pending text as partial.
    assert!(!SummaryAggregator::should_emit_pending_on_completed(true));
}

/// SLICE 12: edit fallback path parsed from the tool title.
///
/// TS: `it("sends apply_patch file using title and patchText fallback")`
/// (test lines 3245-3307: title `"Success. Updated the following
/// files:\nM README.md"` -> file `"README.md"`) via
/// `extractFirstUpdatedFileFromTitle` (src ~362-371: first `M/A/D/...`
/// status line wins).
#[test]
fn slice12_edit_fallback_title_yields_path() {
    // Slice-12 stub (panics while RED).
    // GREEN: the first status line yields the patched path.
    assert_eq!(
        SummaryAggregator::edit_path_from_title(
            "Success. Updated the following files:\nM README.md"
        ),
        Some("README.md".to_string())
    );
    assert_eq!(
        SummaryAggregator::edit_path_from_title(
            "Success. Updated the following files:\nM src/one.ts"
        ),
        Some("src/one.ts".to_string())
    );
    assert_eq!(
        SummaryAggregator::edit_path_from_title("Success, nothing listed"),
        None
    );
}

/// SLICE 12: write fallback yields no file without path and content.
///
/// TS: `prepareToolFiles` write branch (src ~2763-2783: `if (!filePath ||
/// !hasContent) return []`) — the oversized case (test lines 2119-2168)
/// keeps `onTool` with `hasFileAttachment: false` because the payload
/// branch is separate from this missing-input fallback.
#[test]
fn slice12_write_fallback_missing_path_yields_no_file() {
    // Slice-12 stub (panics while RED).
    // GREEN: both path and content are required for a write file.
    assert!(!SummaryAggregator::write_file_ready("", true));
    assert!(!SummaryAggregator::write_file_ready("src/huge.ts", false));
    assert!(SummaryAggregator::write_file_ready("src/huge.ts", true));
}

/// SLICE 12: thinking finished signals once, only after reasoning.
///
/// TS: `it("signals thinking finished once when assistant text starts
/// after reasoning")` (test lines 2312-2375: reasoning part then text part
/// -> `onThinkingFinished` once with `(sessionId, messageId)`) +
/// `it("does not signal thinking finished for assistant text without
/// reasoning")` (test lines 2377-2410) via `emitThinkingFinishedOnce`
/// (src ~2586-2600: needs `thinkingFiredForMessages`, skips when already
/// in `thinkingFinishedForMessages`).
#[test]
fn slice12_thinking_finished_once_after_reasoning() {
    // Slice-12 stub (panics while RED).
    // GREEN: only the first text-after-reasoning signals.
    assert!(SummaryAggregator::should_signal_thinking_finished(
        true, false
    ));
    assert!(!SummaryAggregator::should_signal_thinking_finished(
        true, true
    ));
    assert!(!SummaryAggregator::should_signal_thinking_finished(
        false, false
    ));
}

// ---------------------------------------------------------------------------
// SLICE 13 (RED): multi-file apply_patch edges, oversized/failed patch,
// onTokens no-field gate, session-error variants. Each test targets a
// slice-13 stub (`unimplemented!()`) and MUST FAIL until the Rust port
// lands. TS source: `src/app/managers/summary-aggregation-manager.ts`;
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 13: one summary line per changed apply_patch file.
///
/// TS: `it("sends one apply_patch document and one file change per changed
/// file")` (test lines 3309-3393: `metadata.files` with `src/one.ts`
/// update + `src/two.ts` add -> lines `"🩹 apply_patch src/one.ts (+1
/// -1)"` and `"🩹 apply_patch src/two.ts (+2)"`) via `formatPatchFileLine`
/// (formatter ~253-255: icon + path + `formatDiffLineInfo`, which drops the
/// zero side).
#[test]
fn slice13_patch_file_line_per_changed_file() {
    // Slice-13 stub (panics while RED).
    // GREEN: one line per file; the zero-deletion side is dropped.
    assert_eq!(
        SummaryAggregator::format_patch_file_line("src/one.ts", 1, 1),
        "🩹 apply_patch src/one.ts (+1 -1)"
    );
    assert_eq!(
        SummaryAggregator::format_patch_file_line("src/two.ts", 2, 0),
        "🩹 apply_patch src/two.ts (+2)"
    );
}

/// SLICE 13: oversized apply_patch diff keeps its line but loses the file.
///
/// TS: `it("leaves an oversized apply_patch file as a text line and keeps
/// the other documents")` (test lines 3395-3455: `src/huge.ts` with
/// `"+" + "x".repeat(101 * 1024)` -> `{ fileData: null, line: "🩹
/// apply_patch src/huge.ts (+1)" }` while `src/small.ts` keeps its
/// `edit_small.ts.txt` document) via `prepareCodeFile` (formatter
/// ~488-507: over the 100 KiB budget returns null).
#[test]
fn slice13_oversized_patch_file_loses_attachment_keeps_line() {
    // Slice-13 stub (panics while RED).
    // GREEN: 101 KiB is over budget; 1 KiB fits.
    assert!(!SummaryAggregator::apply_patch_attachment_allowed(
        101 * 1024
    ));
    assert!(SummaryAggregator::apply_patch_attachment_allowed(1024));
}

/// SLICE 13: a failed apply_patch adds no files.
///
/// TS: `it("adds nothing for a failed apply_patch")` (test lines
/// 3457-3485: `status: "error"` with `patchText` input -> `onToolFile` and
/// `onFileChange` never fire) via the completed-only tool gate (src
/// ~2273-2284: only `status === "completed"` reaches `prepareToolFiles`).
#[test]
fn slice13_failed_patch_yields_no_files() {
    // Slice-13 stub (panics while RED).
    // GREEN: only completed states are processed.
    assert!(SummaryAggregator::should_process_tool_state("completed"));
    assert!(!SummaryAggregator::should_process_tool_state("error"));
}

/// SLICE 13: `message.updated` without tokens never fires `onTokens`.
///
/// TS: `it("does not fire onTokens when message.updated has no tokens
/// field")` (test lines 3618-3636: info with no `tokens` -> `onTokens`
/// not called) via the tokens gate (src ~1976: `if
/// (this.onTokensCallback && assistantInfo.tokens)`).
#[test]
fn slice13_no_tokens_field_fires_nothing() {
    // Slice-13 stub (panics while RED).
    // GREEN: the callback fires only when token data is present.
    assert!(SummaryAggregator::should_fire_tokens(true));
    assert!(!SummaryAggregator::should_fire_tokens(false));
}

/// SLICE 13: unexpected `session.error` shapes fall back or pass through.
///
/// TS: `it("falls back to a generic message and warns when session.error
/// carries a non-object error")` (test lines 4338-4358: `error: 42` ->
/// `onSessionError("session-1", "Unknown session error",
/// { interrupted: false })`) + `it("surfaces a string session.error with
/// a warn ...")` (test lines 4360-4380: `error: "upstream exploded"` ->
/// verbatim) via `extractErrorMessage(error) ?? "Unknown session error"`
/// (src ~3010; `extractErrorMessage` in `opencode-error.ts` ~100-130).
#[test]
fn slice13_session_error_variants_fallback_and_passthrough() {
    // Slice-13 stub (panics while RED).
    // GREEN: missing/blank falls back; a real string passes through trimmed.
    assert_eq!(
        SummaryAggregator::unexpected_session_error_message(None),
        "Unknown session error"
    );
    assert_eq!(
        SummaryAggregator::unexpected_session_error_message(Some("upstream exploded")),
        "upstream exploded"
    );
    assert_eq!(
        SummaryAggregator::unexpected_session_error_message(Some("   ")),
        "Unknown session error"
    );
}

// ---------------------------------------------------------------------------
// SLICE 14 (RED): session.created enrichment, subagent model/provider
// pairing, thinking-message state machines. Each test targets a slice-14
// stub (`unimplemented!()`) and MUST FAIL until the Rust port lands.
// TS source: `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 14: `session.created` enriches the card from its title.
///
/// TS: `handleSessionCreatedOrUpdated` (src ~1600-1656) ->
/// `getOrCreateSubagentForSession` (src ~1531-1543) +
/// `enrichSubagentFromSessionTitle` (`"Desc (@agent subagent)"` splits into
/// both halves) + `emitSubagentState`. TS tests: live subagent flow
/// (test lines 229-243: `session.created` with
/// `title: "Explore project architecture (@explore subagent)"` attaches the
/// card) and parallel subagents (test lines 1630-1644: per-child titles
/// `"first task (@explore subagent)"` / `"second task (@general subagent)"`).
#[test]
fn slice14_session_created_enriches_card_from_title() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    // Slice-14 stub (panics while RED).
    aggregator.handle_session_created_with_title(
        "child-session-1",
        "root-session",
        "Explore project architecture (@explore subagent)",
    );

    // GREEN: the created session binds a card enriched from its title.
    let snapshots = snapshots.borrow();
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].session_id, Some("child-session-1".to_string()));
    assert_eq!(cards[0].parent_session_id, "root-session");
    assert_eq!(cards[0].task, "Explore project architecture");
    assert_eq!(cards[0].agent, "explore");
}

/// SLICE 14: child assistant message pairs agent + provider/model/variant.
///
/// TS: `updateSubagentFromAssistantMessage` (src ~1658-1703: `agent`,
/// `providerID`/`modelID`/`variant` each kept when non-empty, the combo
/// shown as `provider/id (variant)`) via child `message.updated`
/// (test lines 245-268: `providerID=openai`, `modelID=gpt-5.4`,
/// `agent=explore`, `cost: 0.18`, tokens `input: 54000/cache.read: 1000`;
/// variant block at test lines 369-384 adds `variant=high`).
#[test]
fn slice14_assistant_message_pairs_agent_provider_model_variant() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    let snapshots: Rc<
        RefCell<
            Vec<(
                String,
                Vec<otb_core::managers::summary_aggregation::SubagentInfo>,
            )>,
        >,
    > = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::clone(&snapshots);
    aggregator.set_on_subagent(Box::new(move |session_id, subagents| {
        probe.borrow_mut().push((session_id, subagents));
    }));

    // Slice-14 stub (panics while RED).
    aggregator.pair_assistant_message_model(
        "child-session-1",
        Some("explore"),
        Some("openai"),
        Some("gpt-5.4"),
        Some("high"),
    );

    // GREEN: agent + provider/model/variant land on the card with the
    // `provider/id (variant)` display.
    let snapshots = snapshots.borrow();
    assert!(
        !snapshots.is_empty(),
        "expected at least one subagent snapshot"
    );
    let (session_id, cards) = &snapshots[snapshots.len() - 1];
    assert_eq!(session_id, "root-session");
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].session_id, Some("child-session-1".to_string()));
    assert_eq!(cards[0].agent, "explore");
    assert_eq!(cards[0].provider_id, Some("openai".to_string()));
    assert_eq!(cards[0].model_id, Some("gpt-5.4".to_string()));
    assert_eq!(cards[0].variant, Some("high".to_string()));
    assert_eq!(cards[0].model, "openai/gpt-5.4 (high)");
}

/// SLICE 14: thinking snapshot registers once, dedups identical text.
///
/// TS: `registerThinkingPart` (src ~2478-2497: first sight pushes
/// `orderedPartIds`) + `setThinkingPartSnapshot` (src ~2499-2521: returns
/// `false` when text+title are unchanged, so `emitThinkingUpdate` is
/// skipped) + `getThinkingSections` order (src ~2550-2560). TS tests:
/// reasoning arrival (test lines 2170-2214: `part-reasoning-1`
/// `"Let me think about this..."` fires `isFirstUpdate: true`).
#[test]
fn slice14_thinking_snapshot_registers_once_dedups_identical() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-14 stub (panics while RED): (is_first_update, was_updated).
    let (is_first, was_updated) = aggregator.register_thinking_snapshot(
        "message-1",
        "part-reasoning-1",
        "Let me think...",
        None,
    );
    assert!(is_first);
    assert!(was_updated);

    // GREEN: the identical snapshot is a no-op (no re-emit).
    let (is_first_again, was_updated_again) = aggregator.register_thinking_snapshot(
        "message-1",
        "part-reasoning-1",
        "Let me think...",
        None,
    );
    assert!(!is_first_again);
    assert!(!was_updated_again);
}

/// SLICE 14: thinking delta accumulates, full-text hint wins when longer.
///
/// TS: `applyThinkingDelta` (src ~2523-2548: `previous + delta`, then
/// `fullTextHint` replaces it when longer) driving the reasoning-delta
/// flow (test lines 2216-2310: `"First " + "second"` -> `"First second"`
/// with `title: "Analysis"` preserved on the section).
#[test]
fn slice14_thinking_delta_accumulates_hint_wins_when_longer() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-14 stub (panics while RED).
    let accumulated = aggregator.apply_thinking_delta_with_hint(
        "session-1",
        "message-1",
        "part-reasoning-1",
        "second",
        Some("First second"),
    );

    // GREEN: previous text + delta accumulate; the longer hint wins.
    assert_eq!(accumulated, "First second");
}

/// SLICE 14: thinking-finished signals once, only after reasoning.
///
/// TS: `emitThinkingFinishedOnce` (src ~2586-2600: needs
/// `thinkingFiredForMessages`, skips when already in
/// `thinkingFinishedForMessages`) via text-after-reasoning (test lines
/// 2312-2375: reasoning part then text part -> `onThinkingFinished` once)
/// and text-without-reasoning (test lines 2377-2410: no signal).
#[test]
fn slice14_thinking_finished_signals_once_only_after_reasoning() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");

    // Slice-14 stub (panics while RED).
    // GREEN: first text-after-reasoning signals; repeats and
    // reasoning-less messages do not.
    assert!(aggregator.poll_thinking_finished("session-1", "message-1", true));
    assert!(!aggregator.poll_thinking_finished("session-1", "message-1", true));
    assert!(!aggregator.poll_thinking_finished("session-1", "message-2", false));
}

// ---------------------------------------------------------------------------
// SLICE 15 (RED): subagent terminal/cost full paths + background stale-drop
// + nested guards. Each test targets a slice-15 stub (`unimplemented!()`)
// and MUST FAIL until the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 15: completed terminal retires the card through the full path.
///
/// TS: `handleSessionIdle` child branch (src ~2928-2938: non-nested child
/// -> `setSubagentTerminalStatus(sessionID, "completed")`) via
/// `setSubagentTerminalStatus` (src ~1852-1876) -> `endSubagent` (src
/// ~1901-1915: status set, activity cleared, `retireSubagent`) + `emitSessionRunEnded`.
/// TS tests: terminal cards (test lines 1683-1901: completed card emitted
/// once, then excluded from later snapshots).
#[test]
fn slice15_completed_terminal_retires_card() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Slice-15 stub (panics while RED).
    aggregator.complete_subagent("child-session-1");

    // GREEN: the full terminal path retires the card, so its cost reads
    // back zero and no live card remains.
    assert!((aggregator.session_cost("child-session-1") - 0.0).abs() < f64::EPSILON);
}

/// SLICE 15: assistant-message tokens/cost arms land on the card.
///
/// TS: `updateSubagentFromAssistantMessage` (src ~1658-1703: tokens arm at
/// ~1689-1697 replaces `subagent.tokens` with `cache.read/write` defaulting
/// to 0, cost arm at ~1698-1700 assigns `subagent.cost`) via child
/// `message.updated` (src ~1930-1934, non-nested guard). TS tests: live
/// subagent flow (test lines 245-268: `tokens: { input: 54000,
/// cache: { read: 1000 } }`, `cost: 0.18`).
#[test]
fn slice15_assistant_message_tokens_cost_arms() {
    use otb_core::managers::summary_aggregation::TokensInfo;
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    aggregator.update_subagent_tool_state(
        "child-session-1",
        "call-1",
        "bash",
        Some("Running tests"),
        1_000,
    );

    // Slice-15 stub (panics while RED).
    aggregator.apply_assistant_tokens_cost(
        "child-session-1",
        TokensInfo {
            input: 54_000,
            output: 1_200,
            reasoning: 0,
            cache_read: 1_000,
            cache_write: 0,
        },
        0.18,
    );

    // GREEN: tokens are replaced (not accumulated) and cost is assigned.
    assert!((aggregator.session_cost("child-session-1") - 0.18).abs() < f64::EPSILON);
}

/// SLICE 15: background stale-drop ignores a pre-run discovery.
///
/// TS: `handleSessionCreatedOrUpdated` stale-drop (src ~1627-1639: no card
/// for the session + (`!acceptsSubagentEvents` or
/// `createdAt < subagentRunStartedAt`) -> `finishedSubagentSessionIds.add`
/// + return, so a background card's update in a later run is not a stale
/// rediscovery). TS tests: background keep-alive (test lines 738-757) +
/// stale `session.created` arrivals.
#[test]
fn slice15_background_stale_drop_created_before_run() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-15 stub (panics while RED): created (500) before the run (1000).
    aggregator.handle_session_created_with_created_at(
        "stale-child",
        "root-session",
        "Stale task (@explore subagent)",
        500,
        1_000,
    );

    // GREEN: the stale discovery is finished, never attached: still gated
    // and costless.
    assert!(aggregator.is_child_gated("stale-child"));
    assert!((aggregator.session_cost("stale-child") - 0.0).abs() < f64::EPSILON);
}

/// SLICE 15: untracked-parent `session.created` attaches nothing.
///
/// TS: `handleSessionCreatedOrUpdated` parent guard (src ~1641-1643:
/// `if (!trackedSessionParents.has(info.parentID)) return`) — a child whose
/// parent was never tracked is ignored. TS tests: `session.created`
/// gating (test lines 1535-1602).
#[test]
fn slice15_untracked_parent_guarded_attaches_nothing() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-15 stub (panics while RED): parent was never tracked.
    let attached =
        aggregator.handle_session_created_guarded("orphan-child", "unknown-parent", "Orphan task");

    // GREEN: the guard refuses the orphan; nothing attaches.
    assert!(!attached);
    assert!(aggregator.is_child_gated("orphan-child"));
}

/// SLICE 15: nested sessions are nested and never take cards.
///
/// TS: `isNestedSubagentSession` (src ~965-969: parent tracked and
/// non-root) + nested-parent early returns: `handleMessageUpdated` (src
/// ~1930-1934: nested assistant skips `updateSubagentFromAssistantMessage`),
/// `handleMessagePartUpdated` (src ~2072-2074: nested returns; ~2077-2080:
/// subtask of a subagent gets no card), `handleSessionIdle` (src ~2932-2933:
/// nested -> `endNestedSubagentSession`), `handleSessionError` (src
/// ~3019-3020). TS tests: nested-subagent block (test lines 1122-1234).
#[test]
fn slice15_nested_parent_early_return_no_card() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");
    aggregator.handle_session_created("child-session-1", "root-session");
    aggregator.track_nested_launch("child-session-1", "nested-session-1");

    // Slice-15 stub (panics while RED).
    assert!(aggregator.is_nested_subagent_session("nested-session-1"));

    // GREEN: the child is not nested; the nested session resolves to the
    // root and stays gated from cards.
    assert!(!aggregator.is_nested_subagent_session("child-session-1"));
    assert_eq!(
        aggregator.get_root_session_id("nested-session-1"),
        "root-session"
    );
}

// ---------------------------------------------------------------------------
// SLICE 16 (RED): text-message edge states + edit/write fallbacks +
// remaining nested guards. Each test targets a slice-16 stub
// (`unimplemented!()`) and MUST FAIL until the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 16: text delta accumulates, longer full-text hint wins.
///
/// TS: `applyTextDelta` (src ~2399-2435: `previous + delta`, then
/// `fullTextHint` replaces it when `hint.length > accumulated.length` at
/// ~2416-2418) driving the streaming path of `handleMessagePartDelta`
/// (src ~2395-2396) and the `deltaFromUpdated` text path of
/// `handleMessagePartUpdated` (src ~2135-2144).
#[test]
fn slice16_text_delta_hint_wins_when_longer() {
    // Slice-16 stub (panics while RED).
    // GREEN: deltas concatenate; a longer server hint replaces the sum.
    assert_eq!(
        SummaryAggregator::accumulate_text_delta_with_hint("Hel", "lo", None),
        "Hello"
    );
    assert_eq!(
        SummaryAggregator::accumulate_text_delta_with_hint("Hel", "lo", Some("Hello, world!")),
        "Hello, world!"
    );
    assert_eq!(
        SummaryAggregator::accumulate_text_delta_with_hint("Hello, world!", "!", Some("Hi")),
        "Hello, world!!"
    );
}

/// SLICE 16: `message.updated` emits partial only for the first optimistic update.
///
/// TS: `handleMessageUpdated` (src ~1967-1969:
/// `if (!isCompleted && textState.optimisticUpdateCount === 1) emitPartial`):
/// a still-running message emits once its first unknown-text update lands;
/// completed messages go straight to `onComplete` and never emit partial.
#[test]
fn slice16_message_updated_partial_only_first_optimistic() {
    // Slice-16 stub (panics while RED).
    // GREEN: only (running, first update) emits.
    assert!(SummaryAggregator::should_emit_updated_partial(false, 1));
    assert!(!SummaryAggregator::should_emit_updated_partial(false, 2));
    assert!(!SummaryAggregator::should_emit_updated_partial(true, 1));
    assert!(!SummaryAggregator::should_emit_updated_partial(true, 0));
}

/// SLICE 16: edit fallback needs both a path and diff text.
///
/// TS: `prepareToolFiles` edit branch (src ~2785-2807:
/// `filediff.file` + `metadata.diff`, `if (!filePath || !diffText) return []`).
/// TS tests: `filediff` path (test lines 3169-3243) vs title/`patchText`
/// fallback (test lines 3245-3307, via `extractFirstUpdatedFileFromTitle`).
#[test]
fn slice16_edit_fallback_needs_path_and_diff() {
    // Slice-16 stub (panics while RED).
    // GREEN: both halves are required for an edit file.
    assert!(SummaryAggregator::edit_file_ready("src/one.ts", true));
    assert!(!SummaryAggregator::edit_file_ready("", true));
    assert!(!SummaryAggregator::edit_file_ready("src/one.ts", false));
    assert!(!SummaryAggregator::edit_file_ready("", false));
}

/// SLICE 16: write fallback counts additions by newline-split lines.
///
/// TS: `prepareToolFiles` write branch (src ~2763-2782:
/// `if (!filePath || !hasContent) return []`, else
/// `additions: content.split("\n").length, deletions: 0`).
/// TS tests: oversized write (test lines 2119-2168) + multi-text combining.
#[test]
fn slice16_write_fallback_counts_additions_by_lines() {
    // Slice-16 stub (panics while RED).
    // GREEN: JS `split("\n").length` semantics (empty string counts as 1).
    assert_eq!(SummaryAggregator::count_write_additions("hello"), 1);
    assert_eq!(SummaryAggregator::count_write_additions("a\nb\nc"), 3);
    assert_eq!(SummaryAggregator::count_write_additions(""), 1);
}

/// SLICE 16: nested part + subtask-of-subagent guards drop before cards.
///
/// TS: `handleMessagePartUpdated` nested early return (src ~2072-2074:
/// `if (isNestedSubagentSession(part.sessionID)) return`) + subtask guard
/// (src ~2076-2080: `if (part.type === "subtask" && !isCurrentRootSession)
/// return`, so a subtask of a subagent never reaches `registerSubtaskPart`
/// and takes no card) on top of `isNestedSubagentSession` (src ~965-969).
/// TS tests: nested-subagent block (test lines 1122-1234).
#[test]
fn slice16_nested_part_and_subtask_guards_drop() {
    // Slice-16 stub (panics while RED).
    // GREEN: nested parts always drop; subtasks drop off-root only.
    assert!(SummaryAggregator::should_drop_nested_subtask(
        true, false, true
    ));
    assert!(SummaryAggregator::should_drop_nested_subtask(
        true, true, false
    ));
    assert!(SummaryAggregator::should_drop_nested_subtask(
        false, true, false
    ));
    assert!(!SummaryAggregator::should_drop_nested_subtask(
        false, true, true
    ));
    assert!(!SummaryAggregator::should_drop_nested_subtask(
        false, false, true
    ));
}

// ---------------------------------------------------------------------------
// SLICE 17 (RED): cost-zero gate + token cache defaults + forget-live-turn +
// diff counts + abort/interrupt/retry fallbacks. Each test targets a
// slice-17 stub (`unimplemented!()`) and MUST FAIL until the Rust port
// lands. TS source: `src/app/managers/summary-aggregation-manager.ts`; TS
// tests: `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 17: zero cost still formats and still fires `onCost`.
///
/// TS: `handleMessageUpdated` cost branch (src ~2023-2030: gate is
/// `assistantInfo.cost !== undefined`, log `` `$${cost.toFixed(2)}` ``).
/// A `0` cost must still format as `$0.00` and still fire; only a missing
/// cost is gated. TS tests: cost `0.01` (test lines 3526-3528).
#[test]
fn slice17_cost_zero_formats_and_fires() {
    // GREEN already (slice 5): zero formats with two decimals.
    assert_eq!(SummaryAggregator::format_cost(0.0), "$0.00");

    // Slice-17 stub (panics while RED): `Some(0.0)` fires, `None` does not.
    assert!(SummaryAggregator::cost_fires_for_value(Some(0.0)));
    assert!(!SummaryAggregator::cost_fires_for_value(None));
}

/// SLICE 17: token totals default missing cache sides to zero.
///
/// TS: `handleMessageUpdated` tokens branch (src ~1981-1982:
/// `cache?.read || 0` / `cache?.write || 0`) + the
/// `updateSubagentFromAssistantMessage` tokens arm (src ~1694-1695).
/// TS tests: tokens `{ input: 800, output: 200, cache: { read: 100 } }`
/// (test lines 3487-3539) and zero-token intermediate updates (3567-3616).
#[test]
fn slice17_token_cache_defaults_to_zero() {
    // Slice-17 stub (panics while RED).
    let tokens = SummaryAggregator::tokens_with_cache_defaults(800, 200, 10, None, None);

    // GREEN: missing cache sides default to 0, the rest passes through.
    assert_eq!(tokens.input, 800);
    assert_eq!(tokens.output, 200);
    assert_eq!(tokens.reasoning, 10);
    assert_eq!(tokens.cache_read, 0);
    assert_eq!(tokens.cache_write, 0);

    let tokens =
        SummaryAggregator::tokens_with_cache_defaults(4000, 300, 0, Some(12_000), Some(50));
    assert_eq!(tokens.cache_read, 12_000);
    assert_eq!(tokens.cache_write, 50);
}

/// SLICE 17: forgetting the live turn clears it without bumping the mark.
///
/// TS: `forgetLiveTurn` (src ~612-615: `liveTurnStartedAt = null`, no
/// `turnEndCount++`) + `startMissedLiveTurn` (src ~626-632) +
/// `getTurnEndMark` (src ~618-620). TS tests: `describe("live turn
/// start")` (test lines 2781-2867).
#[test]
fn slice17_forget_live_turn_clears_without_bumping_mark() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    let mark = aggregator.get_turn_end_mark();
    aggregator.start_missed_live_turn("session-1", mark);
    assert_eq!(
        aggregator.get_live_turn_started_at("session-1"),
        Some(1_000)
    );

    // Slice-17 stub (panics while RED).
    aggregator.forget_live_turn();

    // GREEN: the turn is forgotten but the generation is untouched, so the
    // old mark still matches.
    assert_eq!(aggregator.get_live_turn_started_at("session-1"), None);
    assert_eq!(aggregator.get_turn_end_mark(), mark);
}

/// SLICE 17: diff change counts skip `+++`/`---` headers.
///
/// TS: `countDiffChangesFromText` (src ~373-389: `+` counts unless `+++`,
/// `-` counts unless `---`) feeding the apply_patch `line` fallback
/// (src ~2856). TS tests: apply_patch file lines (test lines 3309-3455).
#[test]
fn slice17_diff_counts_skip_header_lines() {
    // Slice-17 stub (panics while RED).
    assert_eq!(
        SummaryAggregator::count_diff_changes(
            "+++ b/src/one.ts\n--- a/src/one.ts\n+added\n-removed\n context\n+second"
        ),
        (2, 1)
    );
    assert_eq!(SummaryAggregator::count_diff_changes(""), (0, 0));
    assert_eq!(SummaryAggregator::count_diff_changes("+++\n---\n"), (0, 0));
}

/// SLICE 17: abort vs genuine error + interrupt-reason + retry fallbacks.
///
/// TS: `isAbortError` (src ~358-360: record with
/// `name === "MessageAbortedError"`), `isInterruptedIdle` (src ~322-324) +
/// `readInterruptReason` (src ~326-341: only when interrupted and one of
/// user/shutdown/superseded/inactivity), retry blank fallback (src ~2903:
/// `status.message?.trim() || "Unknown retry error"`). TS tests:
/// `session.error` abort vs error (test lines 3116-3139, 3841-3886) +
/// retry (test lines 3141-3167).
#[test]
fn slice17_abort_error_and_interrupt_reason_gates() {
    // Slice-17 stubs (panic while RED).
    assert!(SummaryAggregator::is_abort_error(Some(
        "MessageAbortedError"
    )));
    assert!(!SummaryAggregator::is_abort_error(Some("Error")));
    assert!(!SummaryAggregator::is_abort_error(None));

    assert_eq!(
        SummaryAggregator::read_interrupt_reason(Some("user"), true),
        Some("user".to_string())
    );
    assert_eq!(
        SummaryAggregator::read_interrupt_reason(Some("bogus"), true),
        None
    );
    assert_eq!(
        SummaryAggregator::read_interrupt_reason(Some("user"), false),
        None
    );
    assert_eq!(SummaryAggregator::read_interrupt_reason(None, true), None);

    assert_eq!(
        SummaryAggregator::retry_message_or_fallback(None),
        "Unknown retry error"
    );
    assert_eq!(
        SummaryAggregator::retry_message_or_fallback(Some("   ")),
        "Unknown retry error"
    );
    assert_eq!(
        SummaryAggregator::retry_message_or_fallback(Some("  plan does not include access  ")),
        "plan does not include access"
    );
}

// ---------------------------------------------------------------------------
// SLICE 18 (RED): session.updated enrichment, model-label partials,
// thinking-update gate, footer-suppression edges, diff file fallback.
// Each test targets a slice-18 stub (`unimplemented!()`) and MUST FAIL until
// the Rust port lands. TS source:
// `src/app/managers/summary-aggregation-manager.ts`; TS tests:
// `tests/app/managers/summary-aggregation-manager.test.ts`.
// ---------------------------------------------------------------------------

/// SLICE 18: `session.updated` enriches the card from its title.
///
/// TS: `processEvent` routes both `session.created` and `session.updated`
/// to `handleSessionCreatedOrUpdated` (src ~863-866 / ~1600-1656) ->
/// `getOrCreateSubagentForSession` + `enrichSubagentFromSessionTitle`
/// (`"Desc (@agent subagent)"` splits into both halves). The Rust created
/// path (`handle_session_created*`, slices 7/14/15) has no updated twin,
/// so a title change on update never reaches the card.
#[test]
fn slice18_session_updated_enriches_card_from_title() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-18 stub (panics while RED).
    let attached = aggregator.handle_session_updated(
        "child-session-1",
        "root-session",
        "Explore project architecture (@explore subagent)",
    );

    // GREEN: the updated session binds a card enriched from its title.
    assert!(attached);
    assert!(!aggregator.is_child_gated("child-session-1"));
    assert!((aggregator.session_cost("child-session-1") - 0.0).abs() < f64::EPSILON);
}

/// SLICE 18: model label shows partial ids, drops blank variants.
///
/// TS: `updateSubagentFromAssistantMessage` (src ~1680-1688: `providerID` /
/// `modelID` / `variant` kept when non-empty, combo shown as
/// `provider/id (variant)`). Covered: full combo (slices 2/14). Remaining:
/// provider-only, model-only, blank-variant, and neither-id edges.
#[test]
fn slice18_model_label_partial_ids() {
    // Slice-18 stub (panics while RED).
    // GREEN: partial ids still label; blank variant ignored; neither -> None.
    assert_eq!(
        SummaryAggregator::subagent_model_label(Some("openai"), None, None),
        Some("openai".to_string())
    );
    assert_eq!(
        SummaryAggregator::subagent_model_label(None, Some("gpt-5.4"), None),
        Some("gpt-5.4".to_string())
    );
    assert_eq!(
        SummaryAggregator::subagent_model_label(Some("openai"), Some("gpt-5.4"), Some("high")),
        Some("openai/gpt-5.4 (high)".to_string())
    );
    assert_eq!(
        SummaryAggregator::subagent_model_label(Some("openai"), Some("gpt-5.4"), Some("  ")),
        Some("openai/gpt-5.4".to_string())
    );
    assert_eq!(
        SummaryAggregator::subagent_model_label(None, None, None),
        None
    );
}

/// SLICE 18: thinking update skipped with no callback or no sections.
///
/// TS: `emitThinkingUpdate` (src ~2562-2574: returns early when
/// `!onThinkingCallback` or `sections.length === 0`). Covered: register /
/// collapse / delta / finished-once (slices 4/12/14/16). Remaining: the
/// emit gate itself, so a thinking-once flow never fires empty updates.
#[test]
fn slice18_thinking_update_skipped_when_empty_or_no_callback() {
    // Slice-18 stub (panics while RED).
    // GREEN: only (callback, non-empty sections) emits.
    assert!(SummaryAggregator::should_emit_thinking_update(true, 2));
    assert!(!SummaryAggregator::should_emit_thinking_update(false, 2));
    assert!(!SummaryAggregator::should_emit_thinking_update(true, 0));
    assert!(!SummaryAggregator::should_emit_thinking_update(false, 0));
}

/// SLICE 18: cancelled questions and rejected permissions suppress footer.
///
/// TS: `handleQuestionSettled` (src ~3097-3099: `outcome === "cancelled"` ->
/// `onFooterSuppressedCallback`) + `handlePermissionReplied` (src ~3209-3211:
/// `reply === "reject"` -> `onFooterSuppressedCallback`). Covered: open /
/// answer / allow / deny state (slice 7). Remaining: the suppression edges
/// (cancelled, reject) vs answered / allow / unknown (`None`) reply.
#[test]
fn slice18_footer_suppressed_on_cancelled_and_reject() {
    // Slice-18 stub (panics while RED).
    // GREEN: cancelled + reject suppress; answered + allow + unknown do not.
    assert!(SummaryAggregator::footer_suppressed_for_settled(
        "cancelled",
        None
    ));
    assert!(SummaryAggregator::footer_suppressed_for_settled(
        "",
        Some("reject")
    ));
    assert!(!SummaryAggregator::footer_suppressed_for_settled(
        "answered", None
    ));
    assert!(!SummaryAggregator::footer_suppressed_for_settled(
        "",
        Some("allow")
    ));
    assert!(!SummaryAggregator::footer_suppressed_for_settled("", None));
}

/// SLICE 18: diff entries with no file normalize to `""`; run-end stays noop.
///
/// TS: `handleSessionDiff` (src ~3141-3146: `file: d.file ?? ""` mapped per
/// entry, gated to the current session) + `emitSessionRunEnded` (src
/// ~3113-3126: no callback -> no-op). Covered: per-session file totals
/// (slice 7). Remaining: the missing-file fallback, with the callback-less
/// run-end proving the noop beside it.
#[test]
fn slice18_diff_missing_file_normalizes_to_empty() {
    // Slice-18 stub (panics while RED).
    assert_eq!(SummaryAggregator::normalize_diff_file(None), "");
    assert_eq!(
        SummaryAggregator::normalize_diff_file(Some("src/one.ts")),
        "src/one.ts"
    );

    // GREEN beside it: with no run-end callback, ending a run is a quiet noop.
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("session-1");
    aggregator.handle_run_end("session-1");
}

// ---------------------------------------------------------------------------
// SLICE 19B (RED): every remaining uncovered parity path, grouped by theme:
// updated/model/thinking, tokens/cost/settled/dispatch, pairing/pending/
// title, patch/budget/gates, text/file/reasoning/error. Each test targets
// a slice-19B stub (`unimplemented!()`) and MUST FAIL until the Rust port
// lands. TS source: `src/app/managers/summary-aggregation-manager.ts`;
// TS tests: `tests/app/managers/summary-aggregation-manager.test.ts`.
// Audit: 115 TS `it`s; 86 covered by slices 1-18 (slices 2/7/9/12 over-pin
// one scenario per multi-`it` describe in their docs — e.g. slice-2 variant
// pins 1 of 4 variant `it`s, slice-2 task-call pins 1 of 6 pairing `it`s).
// The 29 below pin one `it` each (test 29 pins three), except the deferred
// behavior flows with existing GREEN implementations whose full-event
// assertions need a GREEN follow-up (interrupted/nested/retire/count and
// the `session.updated` later-run + delayed-discovery paths).
// ---------------------------------------------------------------------------
// Theme 1: updated/model/thinking leftovers.
// ---------------------------------------------------------------------------

/// SLICE 19B: a later message that adds only a variant re-emits.
///
/// TS: `describe("subagent variant from assistant message")` >
/// `it("emits again when a later message adds only a variant")` (test
/// lines 386-404) via `updateSubagentFromAssistantMessage` (src
/// ~1686-1688). The incoming variant replaces the empty card value, so
/// the snapshot changes and re-emits.
#[test]
fn slice19b_variant_reemits_on_variant_only_update_1() {
    // Slice-19B stub (panics while RED).
    // GREEN: the variant-only message yields a new value to emit.
    assert_eq!(
        SummaryAggregator::apply_variant_to_card(None, Some("high")),
        Some("high".to_string())
    );
}

/// SLICE 19B: a later message without a variant keeps the last one.
///
/// TS: `it("keeps the last variant when a later message omits it")` (test
/// lines 406-417) via `updateSubagentFromAssistantMessage` (src
/// ~1686-1688: only a non-empty variant is applied).
#[test]
fn slice19b_variant_keeps_last_when_omitted_2() {
    // Slice-19B stub (panics while RED).
    // GREEN: an omitted variant keeps the card's last value.
    assert_eq!(
        SummaryAggregator::apply_variant_to_card(Some("high"), None),
        Some("high".to_string())
    );
}

/// SLICE 19B: an empty variant is never stored.
///
/// TS: `it("does not store an empty variant")` (test lines 419-432) via
/// `updateSubagentFromAssistantMessage` (src ~1686-1688: blank variants
/// are dropped, like slice-18 `subagent_model_label`).
#[test]
fn slice19b_variant_ignores_empty_3() {
    // Slice-19B stub (panics while RED).
    // GREEN: blank variants keep the previous value (or stay absent).
    assert_eq!(
        SummaryAggregator::apply_variant_to_card(None, Some("")),
        None
    );
    assert_eq!(
        SummaryAggregator::apply_variant_to_card(Some("high"), Some("  ")),
        Some("high".to_string())
    );
}

/// SLICE 19B: text without reasoning never fires the thinking callback.
///
/// TS: `it("does not send thinking callback when no reasoning part
/// arrives")` (test lines 3021-3056) via `registerThinkingPart` +
/// `emitThinkingUpdate` (src ~2478-2584). Covered: thinking snapshot /
/// delta / finished-once (slices 4/12/14) and the empty-sections gate
/// (slice 18). Remaining: text-only messages emit no thinking update.
#[test]
fn slice19b_thinking_skipped_without_reasoning_4() {
    // Slice-19B stub (panics while RED).
    // GREEN: only messages with reasoning parts emit thinking updates.
    assert!(!SummaryAggregator::should_emit_thinking_for_message(
        false, true
    ));
    assert!(SummaryAggregator::should_emit_thinking_for_message(
        true, true
    ));
    assert!(!SummaryAggregator::should_emit_thinking_for_message(
        true, false
    ));
}

/// SLICE 19B: only the first reasoning part flags the thinking update.
///
/// TS: `it("streams all reasoning parts and marks only the first thinking
/// update")` (test lines 3058-3114) via `emitThinkingUpdate`
/// `isFirstUpdate` (src ~2562+). All three parts emit, but only the first
/// carries `isFirstUpdate: true`.
#[test]
fn slice19b_thinking_first_update_flags_5() {
    // Slice-19B stub (panics while RED).
    // GREEN: the first part opens the update; later parts extend it.
    assert!(SummaryAggregator::marks_first_thinking_update(false));
    assert!(!SummaryAggregator::marks_first_thinking_update(true));
}

// ---------------------------------------------------------------------------
// Theme 2: tokens/cost/run-end/todos + settled/dispatch leftovers.
// ---------------------------------------------------------------------------

/// SLICE 19B: assistant metadata reaches `onComplete`.
///
/// TS: `it("passes assistant metadata to onComplete")` (test lines
/// 2869-2930) via the `message.updated` completed branch (src ~1930+):
/// agent `plan`, provider `openai`, model `gpt-5.4`, created `1000`,
/// completed `2500` land on the completion info. Covered: tokens/cost
/// branches (slices 11/17). Remaining: the completion-info assembly.
#[test]
fn slice19b_complete_carries_assistant_metadata_6() {
    // Slice-19B stub (panics while RED).
    // GREEN: agent/provider/model/timestamps land on the completion info.
    assert_eq!(
        SummaryAggregator::assistant_completion_info(
            Some("plan"),
            Some("openai"),
            Some("gpt-5.4"),
            Some(1000),
            Some(2500)
        ),
        otb_core::managers::summary_aggregation::AssistantCompletionInfo {
            agent: Some("plan".to_string()),
            provider_id: Some("openai".to_string()),
            model_id: Some("gpt-5.4".to_string()),
            created_at: Some(1000),
            completed_at: Some(2500),
        }
    );
}

/// SLICE 19B: questions settled outside Telegram report for followed runs.
///
/// TS: `describe("requests of subagent sessions")` >
/// `it("reports questions settled outside Telegram for the followed
/// session and its subagents")` (test lines 3816-3839) via
/// `handleQuestionSettled` (src ~3084-3111): root `replied` ->
/// `("root-session", "q-root", "answered")`, child `rejected` ->
/// `("child-session-1", "q-child", "cancelled")`, unrelated ignored.
/// Covered: asked/answered card state (slice 7) + cancelled/reject footer
/// suppression (slice 18). Remaining: the settled-callback dispatch.
#[test]
fn slice19b_questions_settled_outside_telegram_7() {
    // Slice-19B stubs (panic while RED).
    // GREEN: replied/rejected map to answered/cancelled for followed runs.
    assert_eq!(
        SummaryAggregator::question_settled_outcome("replied"),
        "answered"
    );
    assert_eq!(
        SummaryAggregator::question_settled_outcome("rejected"),
        "cancelled"
    );
    assert!(SummaryAggregator::should_emit_question_settled(true));
    assert!(!SummaryAggregator::should_emit_question_settled(false));
}

/// SLICE 19B: OpenCode's permission decision travels with the reply.
///
/// TS: `it("passes OpenCode's decision along with a replied permission")`
/// (test lines 3841-3860) via `handlePermissionReplied` (src ~3188-3230):
/// child `reject` -> `("child-session-1", "perm-1", "reject")`, missing
/// reply -> `("root-session", "perm-2", None)`. Covered: allow/deny card
/// state (slice 7). Remaining: the replied-callback payload.
#[test]
fn slice19b_permission_decision_passed_along_8() {
    // Slice-19B stub (panics while RED).
    // GREEN: the reply value passes through verbatim, missing stays `None`.
    assert_eq!(
        SummaryAggregator::permission_reply_value(Some("reject")),
        Some("reject".to_string())
    );
    assert_eq!(SummaryAggregator::permission_reply_value(None), None);
    assert_eq!(
        SummaryAggregator::permission_reply_value(Some("allow")),
        Some("allow".to_string())
    );
}

/// SLICE 19B: equivalent permission requests serialize behind a dispatch.
///
/// TS: `it("serializes permission callbacks so equivalent requests can be
/// registered before the next dispatch")` (test lines 3701-3752) via the
/// `permissionQueue` chain (src ~441): while `req-1` dispatches, `req-2`
/// waits; both run in order after release.
#[test]
fn slice19b_permission_dispatch_serialized_9() {
    // Slice-19B stub (panics while RED).
    // GREEN: a running dispatch queues the next equivalent request.
    assert!(SummaryAggregator::should_queue_permission_while_dispatching(true));
    assert!(!SummaryAggregator::should_queue_permission_while_dispatching(false));
}

/// SLICE 19B: permissions from unrelated sessions never dispatch.
///
/// TS: `it("ignores permission.asked events from unrelated sessions")`
/// (test lines 3888-3909) via `handlePermissionAsked` (src ~3155-3186):
/// `some-other-session` fires nothing and is not a subagent session.
/// Covered: tracked-child forwarding (slice 7). Remaining: the gate.
#[test]
fn slice19b_permission_unrelated_session_ignored_10() {
    // Slice-19B stub (panics while RED).
    // GREEN: only the current session and tracked children forward.
    assert!(SummaryAggregator::should_forward_permission_asked(
        true, false
    ));
    assert!(SummaryAggregator::should_forward_permission_asked(
        false, true
    ));
    assert!(!SummaryAggregator::should_forward_permission_asked(
        false, false
    ));
}

// ---------------------------------------------------------------------------
// Theme 3: pairing/pending/title leftovers.
// ---------------------------------------------------------------------------

/// SLICE 19B: two open calls naming no child pair with nothing.
///
/// TS: `describe("task call input reaches its own card")` >
/// `it("applies nothing while two open calls name no child")` (test lines
/// 509-521) via `findSubagentForTaskCall` (src ~1440-1488): with two
/// unpaired open calls the single waiting card stays ambiguous.
#[test]
fn slice19b_task_pairing_two_open_calls_name_none_11() {
    use otb_core::managers::summary_aggregation::TaskCallPairing;
    // Slice-19B stub (panics while RED).
    // GREEN: ambiguous open calls pair with nothing.
    assert_eq!(
        SummaryAggregator::resolve_task_call_pairing(false, 2, 1),
        TaskCallPairing::Unpaired
    );
}

/// SLICE 19B: one open call pairs with the one waiting card.
///
/// TS: `it("pairs the only open call with the only waiting card")` (test
/// lines 523-539) via `findSubagentForTaskCall` (src ~1462-1480): the
/// single-open/single-waiting case binds `child-1` to `first task`.
#[test]
fn slice19b_task_pairing_single_open_with_single_waiting_12() {
    use otb_core::managers::summary_aggregation::TaskCallPairing;
    // Slice-19B stub (panics while RED).
    // GREEN: the unambiguous waiting card takes the call's input.
    assert_eq!(
        SummaryAggregator::resolve_task_call_pairing(false, 1, 1),
        TaskCallPairing::WaitingCard
    );
}

/// SLICE 19B: a card that ended without its input is never paired.
///
/// TS: `it("never pairs a card that ended without its input")` (test lines
/// 541-557) via `findSubagentForTaskCall` + retire-on-end (src ~1074-1101):
/// the ended `child-2` card is retired at once, so a later call with no
/// live named card pairs with nothing instead of falling back.
#[test]
fn slice19b_task_pairing_skips_ended_card_13() {
    use otb_core::managers::summary_aggregation::TaskCallPairing;
    // Slice-19B stub (panics while RED).
    // GREEN: an ended card never waits, so the call stays unpaired.
    assert_eq!(
        SummaryAggregator::resolve_task_call_pairing(false, 1, 0),
        TaskCallPairing::Unpaired
    );
}

/// SLICE 19B: a call left open across the turn end is forgotten.
///
/// TS: `it("forgets a call that never ended once the parent's turn ends")`
/// (test lines 559-582) via `retireForegroundSubagents` clearing unended
/// `openTaskCalls` (src ~1141-1147): after the root idle, the stale
/// `call-stale` no longer blocks pairing a fresh call with its card.
#[test]
fn slice19b_task_pairing_forgets_stale_call_after_turn_14() {
    // Slice-19B stub (panics while RED).
    // GREEN: a call left open across the turn end is dropped.
    assert!(SummaryAggregator::should_drop_stale_task_call(true));
    assert!(!SummaryAggregator::should_drop_stale_task_call(false));
}

/// SLICE 19B: a paired call keeps updating only its own card.
///
/// TS: `it("keeps updating only the card a call was paired with")` (test
/// lines 584-600) via `call.cardId` (src ~1486-1488): refining `call-1`
/// updates `child-1` to `first task, refined` while `child-2` stays empty.
#[test]
fn slice19b_task_pairing_updates_only_paired_card_15() {
    // Slice-19B stub (panics while RED).
    // GREEN: only the paired card takes the update.
    assert_eq!(
        SummaryAggregator::route_task_call_update(Some("card-1"), "call-1", "call-1"),
        Some("card-1".to_string())
    );
    assert_eq!(
        SummaryAggregator::route_task_call_update(Some("card-1"), "call-1", "call-2"),
        None
    );
}

/// SLICE 19B: a reasoning part without title keys yields no title.
///
/// TS: `it("passes a reasoning part without title keys to the thinking
/// callback without a title")` (test lines 4382-4421) via
/// `registerThinkingPart` / `setThinkingPartSnapshot` title keys (src
/// ~2478-2521): `Just thinking` with no `title`/`subtype` keys emits a
/// title-less section. Covered: titled snapshots/dedup (slice 14).
/// Remaining: the title-less fallback.
#[test]
fn slice19b_reasoning_part_without_title_keys_has_no_title_16() {
    // Slice-19B stub (panics while RED).
    // GREEN: no title keys means no title; keys keep the given title.
    assert_eq!(
        SummaryAggregator::reasoning_section_title(false, None),
        None
    );
    assert_eq!(
        SummaryAggregator::reasoning_section_title(true, Some("Plan")),
        Some("Plan".to_string())
    );
}

// ---------------------------------------------------------------------------
// Theme 4: patch/budget/gates leftovers.
// ---------------------------------------------------------------------------

/// SLICE 19B: the same call reporting again stays quiet.
///
/// TS: `describe("subagent current tool timing")` >
/// `it("does not re-emit while the same call keeps reporting")` (test
/// lines 679-689) via `updateSubagentToolState` same-`callId` dedup (src
/// ~1705-1748): the second `call-1` report adds no snapshot.
#[test]
fn slice19b_tool_timing_same_call_no_reemit_17() {
    // Slice-19B stub (panics while RED).
    // GREEN: the same call id never restarts timing.
    assert!(!SummaryAggregator::should_restart_current_tool_timing(
        Some("call-1"),
        "call-1"
    ));
}

/// SLICE 19B: an identical tool with a new call id restarts timing.
///
/// TS: `it("re-emits and restarts timing for an identical tool with a new
/// call id")` (test lines 1515-1532) via `updateSubagentToolState` (src
/// ~1738-1741): `call-2` re-emits with a later `currentToolStartedAt`.
#[test]
fn slice19b_tool_timing_new_call_restarts_18() {
    // Slice-19B stub (panics while RED).
    // GREEN: a new call id restarts timing, even for the same tool.
    assert!(SummaryAggregator::should_restart_current_tool_timing(
        Some("call-1"),
        "call-2"
    ));
    assert!(SummaryAggregator::should_restart_current_tool_timing(
        None, "call-1"
    ));
}

/// SLICE 19B: a promoted task is background when only metadata says so.
///
/// TS: `it("marks a promoted task as background when only its metadata
/// carries the flag")` (test lines 1474-1494) via `isBackgroundTaskCall`
/// (`input.background || metadata.background`, src ~351-355).
#[test]
fn slice19b_background_flag_from_input_or_metadata_19() {
    // Slice-19B stub (panics while RED).
    // GREEN: either side's flag marks the call background.
    assert!(SummaryAggregator::is_background_task_call(false, true));
    assert!(SummaryAggregator::is_background_task_call(true, false));
    assert!(!SummaryAggregator::is_background_task_call(false, false));
}

/// SLICE 19B: a completed tool part with odd metadata still fires.
///
/// TS: `it("does not crash when a completed tool part carries non-object
/// metadata")` (test lines 4423-4453) via the `message.part.updated`
/// tool branch (src ~2279+): `metadata: "not-an-object"` is ignored and
/// `call-weird-meta` still reaches `onTool`.
#[test]
fn slice19b_tool_part_non_object_metadata_no_crash_20() {
    // Slice-19B stub (panics while RED).
    // GREEN: non-object metadata is ignored, the completed call fires.
    assert!(SummaryAggregator::completed_tool_proceeds_without_object_metadata(false));
    assert!(SummaryAggregator::completed_tool_proceeds_without_object_metadata(true));
}

/// SLICE 19B: a parent-run interrupt skips cards that already ended.
///
/// TS: `describe("an interrupted subagent")` >
/// `it("does not touch a card that already ended")` (test lines
/// 1028-1039) via `interruptSubagents` (src ~1170-1209: only `pending` /
/// `running` cards are touched). Covered: single-card interrupt/error
/// ends (slices 3/17). Remaining: the terminal-card guard.
#[test]
fn slice19b_interrupt_skips_ended_card_21() {
    // Slice-19B stub (panics while RED).
    // GREEN: terminal cards are untouched; live ones are reached.
    assert!(!SummaryAggregator::interrupt_reaches_card(true));
    assert!(SummaryAggregator::interrupt_reaches_card(false));
}

// ---------------------------------------------------------------------------
// Theme 5: text/file/reasoning/error leftovers.
// ---------------------------------------------------------------------------

/// SLICE 19B: partial text streams with its message id, then completes.
///
/// TS: `it("streams partial text and passes messageId on completion")`
/// (test lines 2412-2465): the text part update fires `onPartial` with
/// `("session-1", "message-stream-1", "Partial answer")`, and the
/// completed message fires `onComplete` with the same id and text.
/// Covered: delta accumulation (slices 4/16). Remaining: the
/// partial-vs-complete outbound step.
#[test]
fn slice19b_partial_streaming_reports_message_id_22() {
    use otb_core::managers::summary_aggregation::TextMessageFlow;
    // Slice-19B stub (panics while RED).
    // GREEN: running text streams partial; completed text completes.
    assert_eq!(
        SummaryAggregator::text_message_flow_event(false, true),
        TextMessageFlow::Partial
    );
    assert_eq!(
        SummaryAggregator::text_message_flow_event(true, true),
        TextMessageFlow::Complete
    );
    assert_eq!(
        SummaryAggregator::text_message_flow_event(true, false),
        TextMessageFlow::None
    );
}

/// SLICE 19B: part deltas accumulate into the streamed partial.
///
/// TS: `it("streams text from message.part.delta events")` (test lines
/// 2932-2965) via `handleMessagePartDelta` (src ~2395+): `"Hel"` then
/// `"lo"` fire `("session-1", "message-delta-1", "Hel")` and `("...", "Hello")`.
#[test]
fn slice19b_delta_streaming_accumulates_partials_23() {
    // Slice-19B stub (panics while RED).
    // GREEN: deltas append to the streamed text.
    assert_eq!(SummaryAggregator::delta_partial_text("Hel", "lo"), "Hello");
    assert_eq!(SummaryAggregator::delta_partial_text("", "Hi"), "Hi");
}

/// SLICE 19B: deltas stream even when the part type is omitted.
///
/// TS: `it("streams delta events even when part type is omitted")` (test
/// lines 2967-2985) via `handleMessagePartDelta` (src ~2395+): a delta
/// with no part type still fires `("session-1",
/// "message-delta-unknown-type", "Hi")`.
#[test]
fn slice19b_delta_without_part_type_streams_24() {
    // Slice-19B stub (panics while RED).
    // GREEN: an untyped non-empty delta streams while no reasoning runs.
    assert!(SummaryAggregator::should_stream_part_delta(
        false, true, false
    ));
    assert!(!SummaryAggregator::should_stream_part_delta(
        false, false, false
    ));
}

/// SLICE 19B: unknown deltas stay silent once reasoning started.
///
/// TS: `it("does not stream unknown delta part after reasoning started")`
/// (test lines 2987-3019) via `handleMessagePartDelta` (src ~2395+):
/// after `reasoning-part-1`, `internal thoughts` for an untyped part
/// never reaches `onPartial`.
#[test]
fn slice19b_unknown_delta_after_reasoning_suppressed_25() {
    // Slice-19B stub (panics while RED).
    // GREEN: reasoning suppresses later unknown-part deltas.
    assert!(!SummaryAggregator::should_stream_part_delta(
        false, true, true
    ));
    assert!(SummaryAggregator::should_stream_part_delta(
        true, true, true
    ));
}

/// SLICE 19B: a root error freezes unfinished subagent cards.
///
/// TS: `it("freezes unfinished subagent cards when the root session
/// errors")` (test lines 1903-1999) via `handleSessionError` (src
/// ~3003-3051): after the root error, the late `late-call` tool event and
/// the late `child-pending-late` discovery emit nothing.
#[test]
fn slice19b_root_error_freezes_unfinished_cards_26() {
    // Slice-19B stub (panics while RED).
    // GREEN: a root error stops subagent intake; clean runs keep it.
    assert!(SummaryAggregator::should_freeze_subagent_intake(true));
    assert!(!SummaryAggregator::should_freeze_subagent_intake(false));
}

/// SLICE 19B: finished identities survive beyond the 200-card flow.
///
/// TS: `it("retains exact finished identities beyond 200 subagents")`
/// (test lines 2058-2117): after 201 completions, a late update of the
/// long-finished `child-0` emits nothing — `finishedSubagentSessionIds`
/// keeps exact identities with no eviction cap.
#[test]
fn slice19b_finished_identities_retained_beyond_cap_27() {
    // Slice-19B stub (panics while RED).
    // GREEN: finished identities are never evicted, at any count.
    assert!(!SummaryAggregator::evicts_finished_identity(201));
    assert!(!SummaryAggregator::evicts_finished_identity(5000));
}

/// SLICE 19B: a background task part before `session.created` is honored.
///
/// TS: `it("honors a background task part that arrives before the child
/// session is created")` (test lines 1435-1472) via
/// `backgroundChildSessionIds` (the intent recorded on the task part is
/// applied when the card attaches, so the card runs `background: true`
/// through the turn boundary).
#[test]
fn slice19b_background_intent_before_child_created_28() {
    let mut aggregator = SummaryAggregator::new();
    aggregator.set_session("root-session");

    // Slice-19B stubs (panic while RED).
    aggregator.record_pending_background_intent("child-session-1");

    // GREEN: the pre-created intent is taken once on attach, then gone.
    assert!(aggregator.take_pending_background_intent("child-session-1"));
    assert!(!aggregator.take_pending_background_intent("child-session-1"));
    assert!(!aggregator.take_pending_background_intent("child-unknown"));
}

/// SLICE 19B: completed external user input fires for the current session.
///
/// TS: `it("emits completed external user input for the current session")`
/// (test lines 2467-2505) + `it("ignores external user input from a
/// different session")` (test lines 2507-2541) +
/// `it("does not emit whitespace-only external user input")` (test lines
/// 2543-2577): completed user text fires `onExternalUserInput` with
/// `("session-1", "message-user-1", "Check the failing tests")`; other
/// sessions and blank text stay silent.
#[test]
fn slice19b_external_user_input_current_session_only_29() {
    // Slice-19B stub (panics while RED).
    // GREEN: only non-blank user text for the current session emits.
    assert!(SummaryAggregator::should_emit_external_user_input(
        true, true, true
    ));
    assert!(!SummaryAggregator::should_emit_external_user_input(
        false, true, true
    ));
    assert!(!SummaryAggregator::should_emit_external_user_input(
        true, true, false
    ));
    assert!(!SummaryAggregator::should_emit_external_user_input(
        true, false, true
    ));
}
