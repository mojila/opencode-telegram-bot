//! Parity port of the prompt-handover pure state machine (slice 48, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/app/managers/prompt-handover-manager.ts`
//!   (`PromptHandoverManager`: `recordDetach` new-vs-refresh, `get`,
//!   `takeTicket` / `wasDetachedSince`, `addPrompt` / `addInboxEntry`,
//!   `takeNextPrompt` FIFO, `setTurnInFlight` / `hasPendingPrompts`,
//!   `forgetInboxId`, `withdraw` / `withdrawAll`)
//! - `tests/bot/handlers/prompt-handover.test.ts` (V1 detach holds queue
//!   prompts with the selection at /detach; V2 inbox entries wait in
//!   OpenCode; /abort and /opencode_stop withdraw; prepared-across-detach
//!   follows its session only when detached since the ticket)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::prompt_handover_gates::*;
use otb_core::prompt_queue_events::InboxDelivery;

fn selection() -> HandoverSelection {
    HandoverSelection {
        agent: Some("build".to_string()),
        provider_id: "p".to_string(),
        model_id: "m".to_string(),
        variant: Some("high".to_string()),
    }
}

fn handed(text: &str) -> HandedOverPrompt {
    HandedOverPrompt {
        text: text.to_string(),
        file_parts: Vec::new(),
        photos: Vec::new(),
        selection: selection(),
    }
}

fn inbox(session_id: &str, inbox_id: &str) -> QueuedPromptInbox {
    QueuedPromptInbox {
        session_id: session_id.to_string(),
        inbox_id: inbox_id.to_string(),
        delivery: InboxDelivery::Steer,
    }
}

fn detached(handover: &mut PromptHandover) -> u64 {
    handover.record_detach("ses-1", "D:/repo", selection())
}

// --- record_detach (01-09) ---

#[test]
fn slice48_01_first_detach_seq_is_one() {
    let mut handover = PromptHandover::new();
    assert_eq!(detached(&mut handover), 1);
}

#[test]
fn slice48_02_second_detach_seq_is_two() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert_eq!(handover.record_detach("ses-2", "D:/other", selection()), 2);
}

#[test]
fn slice48_03_new_record_starts_empty_and_grounded() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    let record = handover.get("ses-1").expect("record");
    assert_eq!(record.session_id, "ses-1");
    assert_eq!(record.directory, "D:/repo");
    assert_eq!(record.selection, selection());
    assert!(record.prompts.is_empty());
    assert!(record.inbox_entries.is_empty());
    assert_eq!(record.detach_seq, 1);
    assert!(!record.turn_in_flight);
}

#[test]
fn slice48_04_redetach_refreshes_directory() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-1", "D:/moved", selection());
    assert_eq!(handover.get("ses-1").expect("record").directory, "D:/moved");
}

#[test]
fn slice48_05_redetach_refreshes_selection() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    let next = HandoverSelection {
        agent: None,
        provider_id: "x".to_string(),
        model_id: "y".to_string(),
        variant: None,
    };
    handover.record_detach("ses-1", "D:/repo", next.clone());
    assert_eq!(handover.get("ses-1").expect("record").selection, next);
}

#[test]
fn slice48_06_redetach_bumps_seq() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-1", "D:/repo", selection());
    assert_eq!(handover.get("ses-1").expect("record").detach_seq, 2);
}

#[test]
fn slice48_07_redetach_keeps_held_prompts() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.add_prompt("ses-1", handed("first")));
    handover.record_detach("ses-1", "D:/repo", selection());
    let record = handover.get("ses-1").expect("record");
    assert_eq!(record.prompts.len(), 1);
    assert_eq!(record.prompts[0].text, "first");
}

#[test]
fn slice48_08_redetach_keeps_inbox_entries() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.add_inbox_entry(inbox("ses-1", "msg-1")));
    handover.record_detach("ses-1", "D:/repo", selection());
    assert_eq!(handover.get("ses-1").expect("record").inbox_entries.len(), 1);
}

#[test]
fn slice48_09_detach_tracks_sessions_independently() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-2", "D:/other", selection());
    assert_eq!(handover.get("ses-1").expect("s1").detach_seq, 1);
    assert_eq!(handover.get("ses-2").expect("s2").detach_seq, 2);
}

// --- get (10-12) ---

#[test]
fn slice48_10_get_missing_session_is_none() {
    let handover = PromptHandover::new();
    assert!(handover.get("nope").is_none());
}

#[test]
fn slice48_11_get_returns_record_after_detach() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.get("ses-1").is_some());
}

#[test]
fn slice48_12_get_missing_after_withdraw() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.withdraw("ses-1", "abort_command");
    assert!(handover.get("ses-1").is_none());
}

// --- take_ticket / was_detached_since (13-18) ---

#[test]
fn slice48_13_ticket_before_detach_counts_as_detached_since() {
    let mut handover = PromptHandover::new();
    let ticket = handover.take_ticket("ses-1");
    detached(&mut handover);
    assert!(handover.was_detached_since(&ticket));
}

#[test]
fn slice48_14_ticket_after_detach_does_not_count() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    let ticket = handover.take_ticket("ses-1");
    assert!(!handover.was_detached_since(&ticket));
}

#[test]
fn slice48_15_ticket_for_never_detached_session_is_false() {
    let handover = PromptHandover::new();
    let ticket = handover.take_ticket("ghost");
    assert!(!handover.was_detached_since(&ticket));
}

#[test]
fn slice48_16_older_ticket_stays_valid_after_second_detach() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    let ticket = handover.take_ticket("ses-1");
    handover.record_detach("ses-1", "D:/repo", selection());
    assert!(handover.was_detached_since(&ticket));
}

#[test]
fn slice48_17_ticket_belongs_to_its_session_only() {
    let mut handover = PromptHandover::new();
    let ticket = handover.take_ticket("ses-1");
    handover.record_detach("ses-2", "D:/other", selection());
    assert!(!handover.was_detached_since(&ticket));
}

#[test]
fn slice48_18_ticket_captures_current_seq() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-2", "D:/other", selection());
    let ticket = handover.take_ticket("ses-1");
    assert_eq!(ticket.session_id, "ses-1");
    assert_eq!(ticket.detach_seq, 2);
}

// --- add_prompt (19-23) ---

#[test]
fn slice48_19_add_prompt_stores_text_with_selection() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.add_prompt("ses-1", handed("hello")));
    let record = handover.get("ses-1").expect("record");
    assert_eq!(record.prompts.len(), 1);
    assert_eq!(record.prompts[0].text, "hello");
    assert_eq!(record.prompts[0].selection, selection());
}

#[test]
fn slice48_20_add_prompt_without_record_is_false() {
    let mut handover = PromptHandover::new();
    assert!(!handover.add_prompt("ghost", handed("hello")));
}

#[test]
fn slice48_21_add_prompt_preserves_arrival_order() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.add_prompt("ses-1", handed("first")));
    assert!(handover.add_prompt("ses-1", handed("second")));
    let record = handover.get("ses-1").expect("record");
    assert_eq!(record.prompts.len(), 2);
    assert_eq!(record.prompts[0].text, "first");
    assert_eq!(record.prompts[1].text, "second");
}

#[test]
fn slice48_22_add_prompt_carries_attachments() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    let prompt = HandedOverPrompt {
        text: "see".to_string(),
        file_parts: vec!["part-1".to_string()],
        photos: vec!["photo-1".to_string()],
        selection: selection(),
    };
    assert!(handover.add_prompt("ses-1", prompt));
    let stored = &handover.get("ses-1").expect("record").prompts[0];
    assert_eq!(stored.file_parts, vec!["part-1".to_string()]);
    assert_eq!(stored.photos, vec!["photo-1".to_string()]);
}

#[test]
fn slice48_23_add_prompt_goes_to_its_own_session() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-2", "D:/other", selection());
    assert!(handover.add_prompt("ses-2", handed("two")));
    assert!(handover.get("ses-1").expect("s1").prompts.is_empty());
    assert_eq!(handover.get("ses-2").expect("s2").prompts.len(), 1);
}

// --- add_inbox_entry (24-27) ---

#[test]
fn slice48_24_add_inbox_entry_stores_entry() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.add_inbox_entry(inbox("ses-1", "msg-1")));
    assert_eq!(
        handover.get("ses-1").expect("record").inbox_entries,
        vec![inbox("ses-1", "msg-1")]
    );
}

#[test]
fn slice48_25_add_inbox_entry_without_record_is_false() {
    let mut handover = PromptHandover::new();
    assert!(!handover.add_inbox_entry(inbox("ghost", "msg-1")));
}

#[test]
fn slice48_26_add_inbox_entry_routes_by_entry_session() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-2", "D:/other", selection());
    assert!(handover.add_inbox_entry(inbox("ses-2", "msg-2")));
    assert!(handover.get("ses-1").expect("s1").inbox_entries.is_empty());
    assert_eq!(handover.get("ses-2").expect("s2").inbox_entries.len(), 1);
}

#[test]
fn slice48_27_add_inbox_entry_accumulates() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.add_inbox_entry(inbox("ses-1", "msg-1")));
    assert!(handover.add_inbox_entry(inbox("ses-1", "msg-2")));
    assert_eq!(handover.get("ses-1").expect("record").inbox_entries.len(), 2);
}

// --- take_next_prompt (28-32) ---

#[test]
fn slice48_28_take_next_prompt_shifts_oldest_first() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_prompt("ses-1", handed("first"));
    handover.add_prompt("ses-1", handed("second"));
    assert_eq!(handover.take_next_prompt("ses-1").map(|p| p.text), Some("first".to_string()));
    assert_eq!(handover.get("ses-1").expect("record").prompts.len(), 1);
}

#[test]
fn slice48_29_take_next_prompt_empty_is_none() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(handover.take_next_prompt("ses-1").is_none());
}

#[test]
fn slice48_30_take_next_prompt_unknown_session_is_none() {
    let mut handover = PromptHandover::new();
    assert!(handover.take_next_prompt("ghost").is_none());
}

#[test]
fn slice48_31_take_next_prompt_drains_to_none() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_prompt("ses-1", handed("only"));
    assert!(handover.take_next_prompt("ses-1").is_some());
    assert!(handover.take_next_prompt("ses-1").is_none());
}

#[test]
fn slice48_32_take_next_prompt_carries_selection() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_prompt("ses-1", handed("hello"));
    let taken = handover.take_next_prompt("ses-1").expect("prompt");
    assert_eq!(taken.selection, selection());
}

// --- set_turn_in_flight / has_pending_prompts (33-39) ---

#[test]
fn slice48_33_unknown_session_has_no_pending_prompts() {
    let handover = PromptHandover::new();
    assert!(!handover.has_pending_prompts("ghost"));
}

#[test]
fn slice48_34_fresh_detach_has_no_pending_prompts() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    assert!(!handover.has_pending_prompts("ses-1"));
}

#[test]
fn slice48_35_held_prompt_counts_as_pending() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_prompt("ses-1", handed("waiting"));
    assert!(handover.has_pending_prompts("ses-1"));
}

#[test]
fn slice48_36_turn_in_flight_counts_as_pending_without_prompts() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.set_turn_in_flight("ses-1", true);
    assert!(handover.has_pending_prompts("ses-1"));
}

#[test]
fn slice48_37_clearing_flight_clears_pending_when_empty() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.set_turn_in_flight("ses-1", true);
    handover.set_turn_in_flight("ses-1", false);
    assert!(!handover.has_pending_prompts("ses-1"));
}

#[test]
fn slice48_38_drained_prompts_without_flight_are_not_pending() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_prompt("ses-1", handed("only"));
    handover.take_next_prompt("ses-1");
    assert!(!handover.has_pending_prompts("ses-1"));
}

#[test]
fn slice48_39_flight_flag_on_unknown_session_is_noop() {
    let mut handover = PromptHandover::new();
    handover.set_turn_in_flight("ghost", true);
    assert!(!handover.has_pending_prompts("ghost"));
}

// --- forget_inbox_id (40-42) ---

#[test]
fn slice48_40_forget_inbox_id_removes_entry() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_inbox_entry(inbox("ses-1", "msg-1"));
    handover.forget_inbox_id("msg-1");
    assert!(handover.get("ses-1").expect("record").inbox_entries.is_empty());
}

#[test]
fn slice48_41_forget_inbox_id_spans_sessions() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-2", "D:/other", selection());
    handover.add_inbox_entry(inbox("ses-1", "msg-9"));
    handover.add_inbox_entry(inbox("ses-2", "msg-9"));
    handover.forget_inbox_id("msg-9");
    assert!(handover.get("ses-1").expect("s1").inbox_entries.is_empty());
    assert!(handover.get("ses-2").expect("s2").inbox_entries.is_empty());
}

#[test]
fn slice48_42_forget_unknown_inbox_id_keeps_entries() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_inbox_entry(inbox("ses-1", "msg-1"));
    handover.forget_inbox_id("msg-other");
    assert_eq!(handover.get("ses-1").expect("record").inbox_entries.len(), 1);
}

// --- withdraw / withdraw_all / reset (43-48) ---

#[test]
fn slice48_43_withdraw_returns_inbox_entries() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_inbox_entry(inbox("ses-1", "msg-1"));
    handover.add_inbox_entry(inbox("ses-1", "msg-2"));
    let mut returned = handover.withdraw("ses-1", "abort_command");
    returned.sort_by(|a, b| a.inbox_id.cmp(&b.inbox_id));
    assert_eq!(returned, vec![inbox("ses-1", "msg-1"), inbox("ses-1", "msg-2")]);
}

#[test]
fn slice48_44_withdraw_drops_prompts_and_record() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_prompt("ses-1", handed("held"));
    assert!(handover.withdraw("ses-1", "abort_command").is_empty());
    assert!(handover.get("ses-1").is_none());
    assert!(!handover.has_pending_prompts("ses-1"));
}

#[test]
fn slice48_45_withdraw_unknown_session_returns_empty() {
    let mut handover = PromptHandover::new();
    assert!(handover.withdraw("ghost", "abort_command").is_empty());
}

#[test]
fn slice48_46_withdraw_all_returns_every_session_entry() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.record_detach("ses-2", "D:/other", selection());
    handover.add_inbox_entry(inbox("ses-1", "msg-1"));
    handover.add_inbox_entry(inbox("ses-2", "msg-2"));
    let mut returned = handover.withdraw_all("opencode_stop");
    returned.sort_by(|a, b| a.inbox_id.cmp(&b.inbox_id));
    assert_eq!(returned, vec![inbox("ses-1", "msg-1"), inbox("ses-2", "msg-2")]);
    assert!(handover.get("ses-1").is_none());
    assert!(handover.get("ses-2").is_none());
}

#[test]
fn slice48_47_withdraw_all_empty_is_empty() {
    let mut handover = PromptHandover::new();
    assert!(handover.withdraw_all("opencode_stop").is_empty());
}

#[test]
fn slice48_48_reset_clears_sessions_and_restarts_seq() {
    let mut handover = PromptHandover::new();
    detached(&mut handover);
    handover.add_prompt("ses-1", handed("held"));
    handover.reset();
    assert!(handover.get("ses-1").is_none());
    assert_eq!(handover.record_detach("ses-1", "D:/repo", selection()), 1);
}
