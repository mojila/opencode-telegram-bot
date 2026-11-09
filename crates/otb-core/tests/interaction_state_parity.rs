//! Parity port of question/permission/queue/interaction pure logic (slice 27b, RED).
//!
//! Mirrors:
//! - `tests/app/managers/question-manager.test.ts` (18 its)
//! - `tests/app/managers/prompt-queue-manager.test.ts` (22 its)
//! - `tests/app/managers/interaction-manager.test.ts` pure subset (~30 its:
//!   slot start/transition/expiry/clear, waiting-queue order/replace/at-head/
//!   per-session drops, shared-slot question<->permission gates; async
//!   listener-release timing is NOT ported — no Telegram/event loop here)
//! - permission gating via `interaction-manager.test.ts` stateful section
//!   (`question_active` / `stale` / `resolved` / generation behaviour; there
//!   is no dedicated `permission-manager.test.ts`)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::interaction_state::*;

// --- helpers ---

fn single_question() -> Question {
    Question {
        question: "Pick one option".to_string(),
        header: "single".to_string(),
        multiple: false,
        options: vec![
            QuestionOption {
                label: "Yes".to_string(),
                description: "accept".to_string(),
                value: None,
            },
            QuestionOption {
                label: "No".to_string(),
                description: "decline".to_string(),
                value: None,
            },
        ],
    }
}

fn multiple_question() -> Question {
    Question {
        question: "Pick multiple options".to_string(),
        header: "multiple".to_string(),
        multiple: true,
        options: vec![
            QuestionOption {
                label: "Alpha".to_string(),
                description: "first".to_string(),
                value: None,
            },
            QuestionOption {
                label: "Beta".to_string(),
                description: "second".to_string(),
                value: None,
            },
            QuestionOption {
                label: "Gamma".to_string(),
                description: "third".to_string(),
                value: None,
            },
        ],
    }
}

fn permission_request(id: &str, session: &str) -> PermissionRequest {
    PermissionRequest {
        id: id.to_string(),
        session_id: session.to_string(),
        permission: "bash".to_string(),
        patterns: vec![id.to_string()],
    }
}

// --- question poll lifecycle (question-manager.test.ts) ---

#[test]
fn slice27b_01_poll_starts_and_moves_through_questions() {
    let mut poll = QuestionPoll::new();
    assert!(poll.start_questions(
        vec![single_question(), multiple_question()],
        "req-1",
        "session-1"
    ));
    assert!(poll.is_active());
    assert_eq!(poll.request_id().as_deref(), Some("req-1"));
    assert_eq!(poll.current_index(), 0);
    assert_eq!(
        poll.current_question().map(|q| q.question),
        Some("Pick one option".to_string())
    );
    poll.next_question();
    assert_eq!(poll.current_index(), 1);
    poll.next_question();
    assert!(!poll.has_next_question());
    assert_eq!(poll.current_question(), None);
}

#[test]
fn slice27b_02_poll_replaced_only_by_same_session() {
    let mut poll = QuestionPoll::new();
    assert!(poll.start_questions(vec![single_question()], "req-a", "session-a"));
    // Other-session poll must not replace (TS: startQuestions -> false).
    assert!(!poll.start_questions(vec![multiple_question()], "req-b", "session-b"));
    assert_eq!(poll.request_id().as_deref(), Some("req-a"));
    assert!(poll.start_questions(vec![multiple_question()], "req-a2", "session-a"));
    assert_eq!(poll.request_id().as_deref(), Some("req-a2"));
}

#[test]
fn slice27b_03_answer_send_lifecycle_and_settle_flag() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![single_question()], "req-1", "session-1");
    poll.start_custom_input(0);
    assert!(!poll.is_answering());
    poll.start_answer();
    assert!(poll.is_answering());
    assert!(poll.is_settling_from_telegram());
    assert!(!poll.is_waiting_for_custom_input(0));
    poll.note_settled_while_sending(SettledOutcome::Cancelled);
    assert_eq!(
        poll.settled_while_sending(),
        Some(SettledOutcome::Cancelled)
    );
    poll.fail_answer();
    assert!(!poll.is_answering());
    assert_eq!(poll.settled_while_sending(), None);
    assert!(poll.is_active());
}

#[test]
fn slice27b_04_fail_answer_drops_single_select_custom() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![single_question()], "req-1", "session-1");
    poll.set_custom_answer(0, "my own");
    poll.start_answer();
    poll.fail_answer();
    assert!(!poll.has_custom_answer(0));
}

#[test]
fn slice27b_05_fail_answer_keeps_multi_select_custom_row() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![multiple_question()], "req-1", "session-1");
    poll.set_custom_answer(0, "my own");
    poll.start_answer();
    poll.fail_answer();
    assert_eq!(poll.answer_items(0), vec!["my own".to_string()]);
}

#[test]
fn slice27b_06_in_flight_first_ending_wins() {
    let mut poll = QuestionPoll::new();
    poll.track_in_flight("req-1", "session-1");
    poll.track_in_flight("req-2", "session-2");
    poll.track_in_flight("req-3", "session-1");
    poll.end_in_flight("req-1", InFlightEnding::NotAnswered);
    poll.end_in_flight("req-1", InFlightEnding::Cancelled);
    poll.end_in_flight_for_session("session-1", InFlightEnding::Answered);
    poll.end_in_flight("req-untracked", InFlightEnding::Cancelled);
    assert_eq!(
        poll.in_flight_ending("req-1"),
        Some(InFlightEnding::NotAnswered)
    );
    assert_eq!(poll.in_flight_ending("req-2"), None);
    assert_eq!(
        poll.in_flight_ending("req-3"),
        Some(InFlightEnding::Answered)
    );
    assert_eq!(poll.in_flight_ending("req-untracked"), None);
    poll.end_all_in_flight(InFlightEnding::NotAnswered);
    assert_eq!(
        poll.in_flight_ending("req-2"),
        Some(InFlightEnding::NotAnswered)
    );
    poll.track_in_flight("req-2", "session-2");
    assert_eq!(
        poll.in_flight_ending("req-2"),
        Some(InFlightEnding::NotAnswered)
    );
    poll.untrack_in_flight("req-2");
    assert_eq!(poll.in_flight_ending("req-2"), None);
}

#[test]
fn slice27b_07_dismissal_lifecycle_and_last_cancel_failed() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![single_question()], "req-1", "session-1");
    poll.start_custom_input(0);
    assert!(!poll.is_dismissing());
    assert!(!poll.is_settling_from_telegram());
    poll.start_dismissal();
    assert!(poll.is_dismissing());
    assert!(poll.is_settling_from_telegram());
    assert!(!poll.is_waiting_for_custom_input(0));
    poll.note_settled_while_sending(SettledOutcome::Answered);
    poll.note_settled_while_sending(SettledOutcome::Cancelled);
    assert_eq!(poll.settled_while_sending(), Some(SettledOutcome::Answered));
    poll.fail_dismissal();
    assert!(!poll.is_dismissing());
    assert_eq!(poll.settled_while_sending(), None);
    assert!(poll.has_last_cancel_failed());
    poll.note_settled_while_sending(SettledOutcome::Cancelled);
    assert_eq!(poll.settled_while_sending(), None);
    poll.clear_last_cancel_failed();
    assert!(!poll.has_last_cancel_failed());
}

#[test]
fn slice27b_08_clear_drops_custom_input_wait() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![single_question()], "req-1", "session-1");
    poll.start_custom_input(0);
    assert!(poll.is_waiting_for_custom_input(0));
    poll.clear();
    assert!(!poll.is_active());
    assert!(!poll.is_waiting_for_custom_input(0));
}

#[test]
fn slice27b_09_new_poll_resets_previous_selection_state() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![single_question()], "req-old", "session-1");
    poll.select_option(0, 1);
    poll.add_message_id(42);
    poll.start_questions(vec![multiple_question()], "req-new", "session-1");
    assert_eq!(poll.request_id().as_deref(), Some("req-new"));
    assert_eq!(poll.total_questions(), 1);
    assert!(poll.selected_options(0).is_empty());
    assert!(poll.message_ids().is_empty());
}

#[test]
fn slice27b_10_single_replaces_multi_toggles_selection() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(
        vec![single_question(), multiple_question()],
        "req-2",
        "session-1",
    );
    poll.select_option(0, 0);
    poll.select_option(0, 1);
    assert_eq!(poll.selected_options(0), vec![1]);
    assert_eq!(poll.selected_answer(0), "* No: decline".to_string());
    poll.select_option(1, 0);
    poll.select_option(1, 1);
    poll.select_option(1, 0);
    assert_eq!(poll.selected_options(1), vec![1]);
    assert_eq!(poll.selected_answer(1), "* Beta: second");
}

#[test]
fn slice27b_11_custom_answers_appended_after_options() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(
        vec![single_question(), multiple_question()],
        "req-3",
        "session-1",
    );
    poll.select_option(0, 1);
    poll.select_option(1, 0);
    poll.set_custom_answer(1, "Custom response for question #2");
    assert!(poll.has_custom_answer(1));
    assert_eq!(
        poll.custom_answer(1).as_deref(),
        Some("Custom response for question #2")
    );
    assert!(poll.is_custom_answer_selected(1));
    assert_eq!(
        poll.all_answers(),
        vec![
            ("Pick one option".to_string(), "* No: decline".to_string()),
            (
                "Pick multiple options".to_string(),
                "* Alpha: first\nCustom response for question #2".to_string()
            ),
        ]
    );
}

#[test]
fn slice27b_12_multi_answer_items_and_custom_toggle() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![multiple_question()], "req-3c", "session-1");
    poll.set_custom_answer(0, "Line one\nline two");
    poll.select_option(0, 2);
    poll.select_option(0, 0);
    assert_eq!(
        poll.answer_items(0),
        vec![
            "* Gamma: third".to_string(),
            "* Alpha: first".to_string(),
            "Line one\nline two".to_string()
        ]
    );
    poll.toggle_custom_answer(0);
    assert!(!poll.is_custom_answer_selected(0));
    assert_eq!(
        poll.answer_items(0),
        vec!["* Gamma: third".to_string(), "* Alpha: first".to_string()]
    );
    poll.select_option(0, 2);
    poll.select_option(0, 0);
    assert!(!poll.has_answer(0));
    poll.toggle_custom_answer(0);
    assert!(poll.has_answer(0));
    assert_eq!(poll.answer_items(0), vec!["Line one\nline two".to_string()]);
}

#[test]
fn slice27b_13_single_custom_answer_splits_lines_first() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![single_question()], "req-3d", "session-1");
    poll.set_custom_answer(0, "First line\nSecond line");
    assert!(!poll.is_custom_answer_selected(0));
    assert_eq!(
        poll.answer_items(0),
        vec!["First line".to_string(), "Second line".to_string()]
    );
    assert_eq!(
        poll.all_answers(),
        vec![(
            "Pick one option".to_string(),
            "First line\nSecond line".to_string()
        )]
    );
}

#[test]
fn slice27b_14_reply_items_send_choice_values() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(
        vec![
            Question {
                question: "Allow web search?".to_string(),
                header: "search".to_string(),
                multiple: false,
                options: vec![
                    QuestionOption {
                        label: "Allow search via Exa".to_string(),
                        description: String::new(),
                        value: Some("allow".to_string()),
                    },
                    QuestionOption {
                        label: "Disable web search".to_string(),
                        description: String::new(),
                        value: Some("disable".to_string()),
                    },
                ],
            },
            Question {
                question: "Pick colours".to_string(),
                header: "colours".to_string(),
                multiple: true,
                options: vec![
                    QuestionOption {
                        label: "Red".to_string(),
                        description: "The colour red".to_string(),
                        value: Some("r".to_string()),
                    },
                    QuestionOption {
                        label: "Green".to_string(),
                        description: "Line one\nline two".to_string(),
                        value: Some("g".to_string()),
                    },
                ],
            },
        ],
        "req-values",
        "session-1",
    );
    poll.select_option(0, 1);
    poll.select_option(1, 1);
    poll.select_option(1, 0);
    poll.set_custom_answer(1, "Blue");
    assert_eq!(poll.reply_items(0), vec!["disable".to_string()]);
    assert_eq!(
        poll.reply_items(1),
        vec!["g".to_string(), "r".to_string(), "Blue".to_string()]
    );
    assert_eq!(
        poll.all_answers(),
        vec![
            (
                "Allow web search?".to_string(),
                "* Disable web search: ".to_string()
            ),
            (
                "Pick colours".to_string(),
                "* Green: Line one\nline two\n* Red: The colour red\nBlue".to_string()
            ),
        ]
    );
}

#[test]
fn slice27b_15_reply_falls_back_to_display_without_value() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(
        vec![single_question(), multiple_question(), single_question()],
        "req-v1",
        "session-1",
    );
    poll.select_option(0, 0);
    poll.select_option(1, 2);
    poll.select_option(1, 0);
    poll.set_custom_answer(1, "Line one\nline two");
    poll.set_custom_answer(2, "First line\nSecond line");
    for index in 0..3 {
        assert_eq!(poll.reply_items(index), poll.answer_items(index));
    }
}

#[test]
fn slice27b_16_toggle_without_custom_answer_is_noop() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![multiple_question()], "req-3e", "session-1");
    poll.toggle_custom_answer(0);
    assert!(!poll.is_custom_answer_selected(0));
    assert!(!poll.has_answer(0));
}

#[test]
fn slice27b_17_custom_input_and_active_message_tracking() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(
        vec![single_question(), multiple_question()],
        "req-3b",
        "session-1",
    );
    assert_eq!(poll.active_message_id(), None);
    assert!(!poll.is_waiting_for_custom_input(0));
    poll.set_active_message_id(123);
    assert!(poll.is_active_message(123));
    assert!(!poll.is_active_message(999));
    poll.start_custom_input(0);
    assert!(poll.is_waiting_for_custom_input(0));
    poll.next_question();
    assert_eq!(poll.active_message_id(), None);
    assert!(!poll.is_waiting_for_custom_input(0));
}

#[test]
fn slice27b_18_message_ids_copied_cancel_clears() {
    let mut poll = QuestionPoll::new();
    poll.start_questions(vec![single_question()], "req-4", "session-1");
    poll.add_message_id(10);
    poll.add_message_id(11);
    let mut ids = poll.message_ids();
    ids.push(999);
    assert_eq!(poll.message_ids(), vec![10, 11]);
    poll.cancel();
    assert!(!poll.is_active());
    poll.clear();
    assert_eq!(poll.total_questions(), 0);
    assert_eq!(poll.request_id(), None);
    assert_eq!(poll.current_question(), None);
}

// --- shared-slot question <-> permission gates ---

#[test]
fn slice27b_19_poll_refused_while_permissions_on_screen() {
    assert!(!poll_start_allowed(Some("permission"), None, "session-1"));
    let mut registry = PermissionRegistry::new();
    assert_eq!(
        registry.start_permission(permission_request("perm-1", "session-1"), 101, 0),
        StartPermissionResult::Started
    );
    assert_eq!(registry.pending_count(), 1);
}

#[test]
fn slice27b_20_permission_refused_while_poll_on_screen() {
    assert_eq!(
        permission_start_decision(Some("question"), None),
        StartPermissionResult::QuestionActive
    );
    assert!(!poll_start_allowed(Some("permission"), None, "session-1"));
}

#[test]
fn slice27b_21_stale_generation_drops_permission() {
    let registry = PermissionRegistry::new();
    assert_eq!(
        registry.drop_reason("perm-2", 999),
        Some(StartPermissionResult::Stale)
    );
    assert_eq!(
        permission_start_decision(None, Some(&StartPermissionResult::Stale)),
        StartPermissionResult::Stale
    );
}

#[test]
fn slice27b_22_resolved_request_drops_permission() {
    let mut registry = PermissionRegistry::new();
    registry.settle_request("perm-1");
    assert!(registry.is_resolved("perm-1"));
    assert_eq!(
        registry.drop_reason("perm-1", 0),
        Some(StartPermissionResult::Resolved)
    );
    assert_eq!(
        registry.start_permission(permission_request("perm-1", "session-1"), 101, 0),
        StartPermissionResult::Resolved
    );
}

#[test]
fn slice27b_23_permission_signature_groups_equivalent_requests() {
    let first = permission_request("perm-1", "session-1");
    let mut same_patterns = permission_request("perm-2", "session-1");
    same_patterns.patterns = first.patterns.clone();
    assert_eq!(
        PermissionRegistry::request_signature(&first),
        PermissionRegistry::request_signature(&same_patterns)
    );
    let mut different = permission_request("perm-3", "session-1");
    different.patterns = vec!["other".to_string()];
    assert_ne!(
        PermissionRegistry::request_signature(&first),
        PermissionRegistry::request_signature(&different)
    );
}

#[test]
fn slice27b_24_permission_send_gate_single_flight() {
    let mut registry = PermissionRegistry::new();
    assert_eq!(
        registry.start_permission(permission_request("perm-1", "session-1"), 101, 0),
        StartPermissionResult::Started
    );
    assert!(registry.mark_sending(101));
    assert!(registry.is_sending(101));
    assert!(!registry.mark_sending(101));
}

#[test]
fn slice27b_25_question_format_option_line() {
    let option = QuestionOption {
        label: "No".to_string(),
        description: "decline".to_string(),
        value: None,
    };
    assert_eq!(format_option_line(&option), "* No: decline".to_string());
}

// --- prompt queue (prompt-queue-manager.test.ts) ---

#[test]
fn slice27b_26_queue_starts_empty() {
    let queue = PromptQueue::new();
    assert_eq!(queue.size(), 0);
    assert!(queue.list_texts().is_empty());
    assert!(!queue.is_full());
}

#[test]
fn slice27b_27_queue_keeps_insertion_order() {
    let mut queue = PromptQueue::new();
    queue.add("first", 0);
    queue.add("second", 0);
    queue.add("third", 0);
    assert_eq!(
        queue.list_texts(),
        vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string()
        ]
    );
}

#[test]
fn slice27b_28_queue_trims_and_rejects_blank() {
    let mut queue = PromptQueue::new();
    assert_eq!(
        queue.add("  spaced  ", 0).map(|item| item.text),
        Some("spaced".to_string())
    );
    assert_eq!(queue.add("   ", 0), None);
    assert_eq!(queue.size(), 1);
}

#[test]
fn slice27b_29_queue_rejects_beyond_cap() {
    let mut queue = PromptQueue::new();
    for index in 0..MAX_QUEUED_PROMPTS {
        assert!(queue.add(&format!("prompt {index}"), 0).is_some());
    }
    assert!(queue.is_full());
    assert_eq!(queue.add("overflow", 0), None);
    assert_eq!(queue.size(), MAX_QUEUED_PROMPTS);
}

#[test]
fn slice27b_30_queue_removes_middle_keeps_order() {
    let mut queue = PromptQueue::new();
    queue.add("first", 0);
    let second = queue.add("second", 0).expect("queued");
    queue.add("third", 0);
    assert_eq!(
        queue.remove_by_id(&second.id).map(|item| item.text),
        Some("second".to_string())
    );
    assert_eq!(
        queue.list_texts(),
        vec!["first".to_string(), "third".to_string()]
    );
}

#[test]
fn slice27b_31_queue_remove_unknown_returns_none() {
    let mut queue = PromptQueue::new();
    queue.add("first", 0);
    assert_eq!(queue.remove_by_id("queued-999"), None);
    assert_eq!(queue.size(), 1);
}

#[test]
fn slice27b_32_queue_releases_media_on_remove() {
    let mut queue = PromptQueue::new();
    let queued = queue
        .add("photo", MAX_QUEUED_MEDIA_BYTES - 1)
        .expect("queued");
    queue.remove_by_id(&queued.id);
    assert_eq!(queue.media_size(), 0);
    assert!(queue.can_accept_media(MAX_QUEUED_MEDIA_BYTES));
}

#[test]
fn slice27b_33_queue_takes_fifo() {
    let mut queue = PromptQueue::new();
    queue.add("first", 0);
    queue.add("second", 0);
    assert_eq!(
        queue.take_next().map(|item| item.text),
        Some("first".to_string())
    );
    assert_eq!(
        queue.take_next().map(|item| item.text),
        Some("second".to_string())
    );
    assert_eq!(queue.take_next(), None);
}

#[test]
fn slice27b_34_queue_attachment_without_text() {
    let mut queue = PromptQueue::new();
    let item = queue.add_attachment("  ").expect("queued");
    assert_eq!(item.display_text, "[Attachment]");
    assert_eq!(
        queue.take_next().map(|taken| taken.display_text),
        Some("[Attachment]".to_string())
    );
}

#[test]
fn slice27b_35_queue_caps_media_and_releases_on_take() {
    let mut queue = PromptQueue::new();
    let under_cap = MAX_QUEUED_MEDIA_BYTES - 1;
    assert!(queue.add("album one", under_cap).is_some());
    assert!(!queue.can_accept_media(2));
    assert_eq!(queue.add("album two", 2), None);
    queue.take_next();
    assert_eq!(queue.media_size(), 0);
    assert!(queue.add("album two", 2).is_some());
}

#[test]
fn slice27b_36_queue_frees_slot_after_take() {
    let mut queue = PromptQueue::new();
    for index in 0..MAX_QUEUED_PROMPTS {
        queue.add(&format!("prompt {index}"), 0);
    }
    queue.take_next();
    assert!(!queue.is_full());
    assert!(queue.add("late", 0).is_some());
}

#[test]
fn slice27b_37_queue_clear_empties() {
    let mut queue = PromptQueue::new();
    queue.add("first", 0);
    queue.add("second", 0);
    queue.clear();
    assert_eq!(queue.size(), 0);
}

#[test]
fn slice27b_38_queue_reservation_counts_toward_cap() {
    let mut queue = PromptQueue::new();
    for index in 0..MAX_QUEUED_PROMPTS - 1 {
        queue.add(&format!("prompt {index}"), 0);
    }
    assert!(queue.reserve().is_some());
    assert!(queue.is_full());
    assert_eq!(queue.reserve(), None);
    assert_eq!(queue.size(), MAX_QUEUED_PROMPTS - 1);
}

#[test]
fn slice27b_39_queue_reservation_confirms_to_mirror_item() {
    let mut queue = PromptQueue::new();
    let reservation = queue.reserve().expect("reservation");
    let item = queue
        .confirm_reservation(&reservation, "Also check the tests", "msg-1")
        .expect("mirror");
    assert_eq!(item.display_text, "Also check the tests");
    assert_eq!(item.media_bytes, 0);
    assert_eq!(
        queue.find_by_inbox_id("msg-1").map(|found| found.id),
        Some(item.id)
    );
    assert_eq!(queue.size(), 1);
    assert!(!queue.is_full());
}

#[test]
fn slice27b_40_queue_blank_mirror_shows_attachment() {
    let mut queue = PromptQueue::new();
    let reservation = queue.reserve().expect("reservation");
    let item = queue
        .confirm_reservation(&reservation, "  ", "msg-1")
        .expect("mirror");
    assert_eq!(item.display_text, "[Attachment]");
}

#[test]
fn slice27b_41_queue_handover_empties_and_frees_cap() {
    let mut queue = PromptQueue::new();
    queue.add("first", 100);
    queue.add("second", 0);
    let handed_over = queue.hand_over("ses-1");
    assert_eq!(
        handed_over
            .iter()
            .map(|item| item.text.clone())
            .collect::<Vec<_>>(),
        vec!["first".to_string(), "second".to_string()]
    );
    assert_eq!(queue.size(), 0);
    assert_eq!(queue.media_size(), 0);
}

#[test]
fn slice27b_42_queue_remembers_delivered_inbox_until_clear() {
    let mut queue = PromptQueue::new();
    queue.remember_delivered_inbox_id("msg-1");
    assert!(queue.was_inbox_id_delivered("msg-1"));
    assert!(!queue.was_inbox_id_delivered("msg-2"));
    queue.clear();
    assert!(!queue.was_inbox_id_delivered("msg-1"));
}

// --- interaction slot + waiting queue (interaction-manager.test.ts) ---

#[test]
fn slice27b_43_slot_starts_with_defaults() {
    let mut slot = InteractionSlot::new();
    let state = slot.start("custom", "callback", None, None);
    assert_eq!(state.kind, "custom");
    assert_eq!(state.expected_input, "callback");
    assert_eq!(
        state.allowed_commands,
        default_allowed_interaction_commands()
    );
    assert_eq!(state.expires_at_ms, None);
    assert!(slot.is_active());
}

#[test]
fn slice27b_44_slot_normalizes_and_dedupes_commands() {
    assert_eq!(
        normalize_allowed_commands(Some(&[
            "/Help",
            "status",
            "/help",
            " /STATUS@MyBot ",
            "detach",
            "",
            " / "
        ])),
        vec![
            "/help".to_string(),
            "/status".to_string(),
            "/detach".to_string()
        ]
    );
}

#[test]
fn slice27b_45_slot_transition_updates_input_and_expiry() {
    let mut slot = InteractionSlot::new();
    slot.start("custom", "text", None, None);
    let next = slot
        .transition(Some("mixed"), Some(&["/abort"]), Some(Some(5000)))
        .expect("transition");
    assert_eq!(next.kind, "custom");
    assert_eq!(next.expected_input, "mixed");
    assert_eq!(next.allowed_commands, vec!["/abort".to_string()]);
    assert!(next.expires_at_ms.is_some());
}

#[test]
fn slice27b_46_waiting_queue_replaces_same_session_poll() {
    let mut waiting = WaitingQueue::new();
    waiting.wait_question("req-b-old", "session-b");
    waiting.wait_question("req-c", "session-c");
    waiting.wait_question("req-b-new", "session-b");
    assert_eq!(
        waiting.waiting_question_ids(),
        vec!["req-b-new".to_string(), "req-c".to_string()]
    );
}

#[test]
fn slice27b_47_waiting_queue_dedupes_permission_and_drops() {
    let mut waiting = WaitingQueue::new();
    waiting.wait_permission("perm-1", "session-1");
    waiting.wait_permission("perm-1", "session-1");
    waiting.wait_permission("perm-2", "session-1");
    assert_eq!(
        waiting.waiting_permission_ids(),
        vec!["perm-1".to_string(), "perm-2".to_string()]
    );
    waiting.drop_waiting_permission("perm-1");
    assert_eq!(waiting.waiting_kind(), Some(WaitingKind::Permission));
    waiting.drop_waiting_permission("perm-2");
    assert_eq!(waiting.waiting_kind(), None);
    waiting.wait_question("req-1", "session-1");
    assert!(waiting.drop_waiting_question("req-1"));
    assert!(!waiting.drop_waiting_question("req-other"));
    assert_eq!(waiting.waiting_kind(), None);
}

#[test]
fn slice27b_48_waiting_queue_at_head_and_session_drops() {
    let mut waiting = WaitingQueue::new();
    waiting.wait_question("req-c", "session-c");
    waiting.wait_question_at_head("req-b", "session-b");
    waiting.wait_permission_at_head("perm-1", "session-1");
    assert_eq!(waiting.waiting_kind(), Some(WaitingKind::Permission));
    assert_eq!(
        waiting.waiting_question_ids(),
        vec!["req-b".to_string(), "req-c".to_string()]
    );
    waiting.drop_waiting_permissions_for_session("session-1");
    waiting.drop_waiting_questions_for_session("session-b");
    assert_eq!(waiting.waiting_permission_ids(), Vec::<String>::new());
    assert_eq!(waiting.waiting_question_ids(), vec!["req-c".to_string()]);
    waiting.reset();
    assert_eq!(waiting.waiting_kind(), None);
    assert!(waiting.waiting_question_ids().is_empty());
}
