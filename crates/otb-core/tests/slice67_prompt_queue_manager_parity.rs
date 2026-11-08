//! Slice 67 parity: prompt-queue manager gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/app/managers/prompt-queue-manager.ts` (`PromptQueueManager`:
//! bounded queue, OpenCode-inbox reservations, media-byte budget, handover
//! at /detach, delivered-inbox-id memory).
//!
//! Every test MUST FAIL until `prompt_queue_manager_gates` is ported
//! (step 2).

use otb_core::prompt_queue_manager_gates::*;

fn text_input(text: &str) -> QueuedPromptInput {
    QueuedPromptInput {
        text: text.to_string(),
        file_parts: Vec::new(),
        photos: Vec::new(),
        display_text: None,
        response_mode: None,
        media_bytes: None,
    }
}

fn inbox_input(display: &str, session: &str, inbox: &str) -> InboxPromptInput {
    InboxPromptInput {
        display_text: display.to_string(),
        response_mode: None,
        inbox: QueuedPromptInbox {
            session_id: session.to_string(),
            inbox_id: inbox.to_string(),
            delivery: InboxDelivery::Queue,
        },
    }
}

fn fill(queue: &mut PromptQueueManager, n: usize) {
    for i in 0..n {
        assert!(
            queue.add(text_input(&format!("prompt {i}"))).is_some(),
            "fill {i}/{n}"
        );
    }
}

// --- budgets ---

#[test]
fn slice67_max_queued_prompts_parity() {
    assert_eq!(MAX_QUEUED_PROMPTS, 5);
}

#[test]
fn slice67_max_queued_media_bytes_parity() {
    assert_eq!(MAX_QUEUED_MEDIA_BYTES, 20 * 1024 * 1024);
}

#[test]
fn slice67_max_remembered_delivered_ids_parity() {
    assert_eq!(MAX_REMEMBERED_DELIVERED_INBOX_IDS, 20);
}

// --- add ---

#[test]
fn slice67_add_first_id_parity() {
    let mut queue = PromptQueueManager::new();
    let item = queue.add(text_input("hello")).expect("queued");
    assert_eq!(item.id, "queued-1");
}

#[test]
fn slice67_add_ids_increment_parity() {
    let mut queue = PromptQueueManager::new();
    let first = queue.add(text_input("a")).expect("first");
    let second = queue.add(text_input("b")).expect("second");
    assert_eq!(first.id, "queued-1");
    assert_eq!(second.id, "queued-2");
}

#[test]
fn slice67_add_stores_trimmed_text_parity() {
    let mut queue = PromptQueueManager::new();
    let item = queue.add(text_input("  hi  ")).expect("queued");
    assert_eq!(item.text, "hi");
}

#[test]
fn slice67_add_display_defaults_to_text_parity() {
    let mut queue = PromptQueueManager::new();
    let item = queue.add(text_input("hello")).expect("queued");
    assert_eq!(item.display_text, "hello");
}

#[test]
fn slice67_add_explicit_display_text_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("hello");
    input.display_text = Some("custom".to_string());
    let item = queue.add(input).expect("queued");
    assert_eq!(item.display_text, "custom");
}

#[test]
fn slice67_add_blank_text_no_attachments_rejected_parity() {
    let mut queue = PromptQueueManager::new();
    assert_eq!(queue.add(text_input("   ")), None);
    assert_eq!(queue.size(), 0);
}

#[test]
fn slice67_add_whitespace_display_rejected_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("hello");
    input.display_text = Some("   ".to_string());
    assert_eq!(queue.add(input), None);
}

#[test]
fn slice67_add_attachment_fallback_display_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("   ");
    input.file_parts = vec!["file-a".to_string()];
    let item = queue.add(input).expect("queued");
    assert_eq!(item.display_text, "[Attachment]");
}

#[test]
fn slice67_add_full_queue_rejected_parity() {
    let mut queue = PromptQueueManager::new();
    fill(&mut queue, MAX_QUEUED_PROMPTS);
    assert_eq!(queue.add(text_input("one too many")), None);
    assert_eq!(queue.size(), MAX_QUEUED_PROMPTS);
}

#[test]
fn slice67_add_over_media_cap_rejected_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("big");
    input.media_bytes = Some(MAX_QUEUED_MEDIA_BYTES + 1);
    assert_eq!(queue.add(input), None);
    assert_eq!(queue.size(), 0);
}

#[test]
fn slice67_add_response_mode_kept_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("speak");
    input.response_mode = Some(ResponseMode::TextAndTts);
    let item = queue.add(input).expect("queued");
    assert_eq!(item.response_mode, Some(ResponseMode::TextAndTts));
}

#[test]
fn slice67_add_media_bytes_tracked_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("media");
    input.media_bytes = Some(1024);
    let item = queue.add(input).expect("queued");
    assert_eq!(item.media_bytes, 1024);
    assert_eq!(queue.media_size(), 1024);
}

// --- reserve ---

#[test]
fn slice67_reserve_first_id_parity() {
    let mut queue = PromptQueueManager::new();
    assert_eq!(queue.reserve().as_deref(), Some("reserved-1"));
}

#[test]
fn slice67_reserve_ids_increment_parity() {
    let mut queue = PromptQueueManager::new();
    assert_eq!(queue.reserve().as_deref(), Some("reserved-1"));
    assert_eq!(queue.reserve().as_deref(), Some("reserved-2"));
}

#[test]
fn slice67_reserve_full_returns_none_parity() {
    let mut queue = PromptQueueManager::new();
    fill(&mut queue, MAX_QUEUED_PROMPTS);
    assert_eq!(queue.reserve(), None);
}

#[test]
fn slice67_reserve_counts_toward_full_parity() {
    let mut queue = PromptQueueManager::new();
    for _ in 0..MAX_QUEUED_PROMPTS {
        assert!(queue.reserve().is_some());
    }
    assert!(queue.is_full());
    assert_eq!(queue.reserve(), None);
}

// --- confirmReservation ---

#[test]
fn slice67_confirm_unknown_reservation_none_parity() {
    let mut queue = PromptQueueManager::new();
    assert_eq!(queue.confirm_reservation("reserved-9", inbox_input("hi", "s", "i")), None);
}

#[test]
fn slice67_confirm_creates_mirror_item_parity() {
    let mut queue = PromptQueueManager::new();
    let reservation = queue.reserve().expect("reserved");
    let item = queue
        .confirm_reservation(&reservation, inbox_input("hello", "sess-1", "inbox-1"))
        .expect("mirrored");
    assert_eq!(item.text, "");
    assert_eq!(item.media_bytes, 0);
    assert!(item.file_parts.is_empty());
    assert_eq!(
        item.inbox,
        Some(QueuedPromptInbox {
            session_id: "sess-1".to_string(),
            inbox_id: "inbox-1".to_string(),
            delivery: InboxDelivery::Queue,
        })
    );
}

#[test]
fn slice67_confirm_display_trimmed_or_fallback_parity() {
    let mut queue = PromptQueueManager::new();
    let reservation = queue.reserve().expect("reserved");
    let item = queue
        .confirm_reservation(&reservation, inbox_input("   ", "s", "i"))
        .expect("mirrored");
    assert_eq!(item.display_text, "[Attachment]");
}

#[test]
fn slice67_confirm_consumes_reservation_parity() {
    let mut queue = PromptQueueManager::new();
    let reservation = queue.reserve().expect("reserved");
    let input = inbox_input("hi", "s", "i");
    assert!(
        queue
            .confirm_reservation(&reservation, input.clone())
            .is_some()
    );
    assert_eq!(queue.confirm_reservation(&reservation, input), None);
}

// --- releaseReservation ---

#[test]
fn slice67_release_reservation_frees_slot_parity() {
    let mut queue = PromptQueueManager::new();
    fill(&mut queue, MAX_QUEUED_PROMPTS - 1);
    let reservation = queue.reserve().expect("reserved");
    assert!(queue.is_full());
    assert!(queue.release_reservation(&reservation));
    assert!(!queue.is_full());
    assert!(queue.add(text_input("fits now")).is_some());
}

#[test]
fn slice67_release_unknown_reservation_false_parity() {
    let mut queue = PromptQueueManager::new();
    assert!(!queue.release_reservation("reserved-1"));
}

// --- findByInboxId ---

#[test]
fn slice67_find_by_inbox_id_hit_parity() {
    let mut queue = PromptQueueManager::new();
    let reservation = queue.reserve().expect("reserved");
    queue
        .confirm_reservation(&reservation, inbox_input("hi", "s", "inbox-7"))
        .expect("mirrored");
    let found = queue.find_by_inbox_id("inbox-7").expect("found");
    assert_eq!(found.display_text, "hi");
}

#[test]
fn slice67_find_by_inbox_id_miss_parity() {
    let queue = PromptQueueManager::new();
    assert_eq!(queue.find_by_inbox_id("nope"), None);
}

#[test]
fn slice67_find_returns_copy_parity() {
    let mut queue = PromptQueueManager::new();
    let reservation = queue.reserve().expect("reserved");
    queue
        .confirm_reservation(&reservation, inbox_input("hi", "s", "inbox-7"))
        .expect("mirrored");
    let mut found = queue.find_by_inbox_id("inbox-7").expect("found");
    found.display_text = "mutated".to_string();
    assert_eq!(
        queue.find_by_inbox_id("inbox-7").expect("again").display_text,
        "hi"
    );
}

// --- delivered inbox ids ---

#[test]
fn slice67_remembered_inbox_id_hit_parity() {
    let mut queue = PromptQueueManager::new();
    queue.remember_delivered_inbox_id("inbox-1");
    assert!(queue.was_inbox_id_delivered("inbox-1"));
}

#[test]
fn slice67_unremembered_inbox_id_miss_parity() {
    let queue = PromptQueueManager::new();
    assert!(!queue.was_inbox_id_delivered("inbox-1"));
}

#[test]
fn slice67_delivered_ids_evict_oldest_parity() {
    let mut queue = PromptQueueManager::new();
    for i in 0..MAX_REMEMBERED_DELIVERED_INBOX_IDS + 1 {
        queue.remember_delivered_inbox_id(&format!("inbox-{i}"));
    }
    assert!(!queue.was_inbox_id_delivered("inbox-0"));
    assert!(queue.was_inbox_id_delivered(&format!(
        "inbox-{MAX_REMEMBERED_DELIVERED_INBOX_IDS}"
    )));
}

// --- list ---

#[test]
fn slice67_list_empty_initially_parity() {
    let queue = PromptQueueManager::new();
    assert_eq!(queue.list(), vec![]);
}

#[test]
fn slice67_list_order_and_copies_parity() {
    let mut queue = PromptQueueManager::new();
    fill(&mut queue, 3);
    let listed = queue.list();
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].display_text, "prompt 0");
    assert_eq!(listed[2].display_text, "prompt 2");
    let mut detached = listed;
    detached.push(detached[0].clone());
    assert_eq!(queue.size(), 3);
}

// --- removeById ---

#[test]
fn slice67_remove_by_id_hit_parity() {
    let mut queue = PromptQueueManager::new();
    let item = queue.add(text_input("bye")).expect("queued");
    let removed = queue.remove_by_id(&item.id).expect("removed");
    assert_eq!(removed.id, item.id);
    assert_eq!(queue.size(), 0);
}

#[test]
fn slice67_remove_by_id_miss_parity() {
    let mut queue = PromptQueueManager::new();
    fill(&mut queue, 1);
    assert_eq!(queue.remove_by_id("queued-99"), None);
    assert_eq!(queue.size(), 1);
}

#[test]
fn slice67_remove_frees_media_budget_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("media");
    input.media_bytes = Some(1024);
    let item = queue.add(input).expect("queued");
    assert_eq!(queue.media_size(), 1024);
    queue.remove_by_id(&item.id).expect("removed");
    assert_eq!(queue.media_size(), 0);
    assert!(queue.can_accept_media(1024));
}

// --- takeNext ---

#[test]
fn slice67_take_next_fifo_parity() {
    let mut queue = PromptQueueManager::new();
    fill(&mut queue, 3);
    let first = queue.take_next().expect("first");
    let second = queue.take_next().expect("second");
    assert_eq!(first.display_text, "prompt 0");
    assert_eq!(second.display_text, "prompt 1");
    assert_eq!(queue.size(), 1);
}

#[test]
fn slice67_take_next_empty_none_parity() {
    let mut queue = PromptQueueManager::new();
    assert_eq!(queue.take_next(), None);
}

#[test]
fn slice67_take_next_frees_media_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("media");
    input.media_bytes = Some(2048);
    queue.add(input).expect("queued");
    queue.take_next().expect("taken");
    assert_eq!(queue.media_size(), 0);
}

// --- size / isFull ---

#[test]
fn slice67_is_full_boundary_parity() {
    let mut queue = PromptQueueManager::new();
    assert!(!queue.is_full());
    fill(&mut queue, MAX_QUEUED_PROMPTS - 1);
    assert!(!queue.is_full());
    queue.add(text_input("last")).expect("queued");
    assert!(queue.is_full());
}

// --- canAcceptMedia ---

#[test]
fn slice67_can_accept_media_exact_fit_parity() {
    let queue = PromptQueueManager::new();
    assert!(queue.can_accept_media(MAX_QUEUED_MEDIA_BYTES));
}

#[test]
fn slice67_can_accept_media_over_by_one_parity() {
    let queue = PromptQueueManager::new();
    assert!(!queue.can_accept_media(MAX_QUEUED_MEDIA_BYTES + 1));
}

#[test]
fn slice67_can_accept_media_negative_rejected_parity() {
    let queue = PromptQueueManager::new();
    assert!(!queue.can_accept_media(-1));
}

// --- handOver ---

#[test]
fn slice67_hand_over_returns_and_empties_parity() {
    let mut queue = PromptQueueManager::new();
    let mut input = text_input("media");
    input.media_bytes = Some(512);
    queue.add(input).expect("queued");
    queue.add(text_input("plain")).expect("queued");
    let removed = queue.hand_over("sess-1");
    assert_eq!(removed.len(), 2);
    assert_eq!(queue.size(), 0);
    assert_eq!(queue.media_size(), 0);
}

#[test]
fn slice67_hand_over_moves_reservations_parity() {
    let mut queue = PromptQueueManager::new();
    let reservation = queue.reserve().expect("reserved");
    queue.hand_over("sess-1");
    assert!(!queue.release_reservation(&reservation));
    assert!(queue.release_handed_over_reservation(&reservation));
    assert!(!queue.release_handed_over_reservation(&reservation));
}

#[test]
fn slice67_withdraw_handed_over_scoped_parity() {
    let mut queue = PromptQueueManager::new();
    let first = queue.reserve().expect("first");
    queue.hand_over("sess-1");
    let second = queue.reserve().expect("second");
    queue.hand_over("sess-2");
    queue.withdraw_handed_over_reservations(Some("sess-1"));
    assert!(!queue.release_handed_over_reservation(&first));
    assert!(queue.release_handed_over_reservation(&second));
}

// --- clear ---

#[test]
fn slice67_clear_empties_and_drops_reservations_parity() {
    let mut queue = PromptQueueManager::new();
    queue.add(text_input("queued")).expect("queued");
    let reservation = queue.reserve().expect("reserved");
    queue.remember_delivered_inbox_id("inbox-1");
    let removed = queue.clear();
    assert_eq!(removed.len(), 1);
    assert_eq!(queue.size(), 0);
    assert_eq!(queue.media_size(), 0);
    assert!(!queue.release_reservation(&reservation));
    assert_eq!(
        queue.confirm_reservation(&reservation, inbox_input("late", "s", "i")),
        None
    );
    assert!(!queue.was_inbox_id_delivered("inbox-1"));
    assert!(queue.add(text_input("fresh")).is_some());
}
