//! Prompt-queue manager pure gates (slice 67, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/managers/prompt-queue-manager.ts` (`PromptQueueManager`:
//!   bounded in-memory queue of prepared prompts with OpenCode-inbox
//!   reservations, a raw-media-byte budget, handover at `/detach`, and a
//!   bounded memory of delivered inbox ids)
//!
//! Every test in
//! `crates/otb-core/tests/slice67_prompt_queue_manager_parity.rs` covers
//! this ported module (step 2: GREEN).
//!
//! Pure sync data-structure logic only: no Telegram I/O, no network/fetch,
//! no timers, no `logger` calls. The `reason` arguments TS threads through
//! `handOver`/`clear` only feed `logger.info`, so they are dropped here.
//!
//! DEVIATIONS from TS:
//! 1. `mediaBytes` is `i64` here; TS uses `number`. Non-finite floats have
//!    no representation: callers normalize them before calling, and parity
//!    fixtures use integers where the two coincide (`NaN >= 0` is false in
//!    TS, and a negative `i64` is rejected the same way).
//! 2. `MAX_REMEMBERED_DELIVERED_INBOX_IDS` is module-private in TS; it is
//!    `pub` here so parity tests can assert the eviction bound directly.
//! 3. `InboxDelivery` is reused from `crate::prompt_queue_events`
//!    (re-exported) instead of redeclared.
//! 4. `fileParts`/`photos` are `Vec<String>` payloads here; TS carries
//!    `FilePartInput[]`/`TelegramPhotoInput[]`. Only presence, count, and
//!    copy semantics matter to the queue contract.

pub use crate::prompt_queue_events::InboxDelivery;

/// Maximum prompts (items plus in-flight reservations) the queue holds
/// (TS: `MAX_QUEUED_PROMPTS`).
pub const MAX_QUEUED_PROMPTS: usize = 5;

/// Maximum raw Telegram media bytes retained by all queued prompts
/// (TS: `MAX_QUEUED_MEDIA_BYTES`).
pub const MAX_QUEUED_MEDIA_BYTES: i64 = 20 * 1024 * 1024;

/// Recently delivered, unmatched OpenCode inbox ids kept for late admissions
/// (TS: `MAX_REMEMBERED_DELIVERED_INBOX_IDS`, module-private there).
pub const MAX_REMEMBERED_DELIVERED_INBOX_IDS: usize = 20;

/// How a queued prompt answers (TS: `responseMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    TextOnly,
    TextAndTts,
}

/// Where a prompt waits in the OpenCode V2 session inbox
/// (TS: `QueuedPromptInbox`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPromptInbox {
    pub session_id: String,
    pub inbox_id: String,
    pub delivery: InboxDelivery,
}

/// A prompt waiting in the queue (TS: `QueuedPrompt`).
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedPrompt {
    pub id: String,
    pub text: String,
    pub file_parts: Vec<String>,
    pub photos: Vec<String>,
    pub display_text: String,
    pub response_mode: Option<ResponseMode>,
    pub media_bytes: i64,
    /// Set when the prompt waits in OpenCode rather than in the bot: this
    /// item only mirrors it.
    pub inbox: Option<QueuedPromptInbox>,
}

/// A prepared prompt offered to the queue (TS: `QueuedPromptInput`).
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedPromptInput {
    pub text: String,
    pub file_parts: Vec<String>,
    pub photos: Vec<String>,
    pub display_text: Option<String>,
    pub response_mode: Option<ResponseMode>,
    /// Raw media bytes from Telegram `file_size` metadata. `None` behaves
    /// like TS `undefined` (counts as `0`).
    pub media_bytes: Option<i64>,
}

/// A reservation turned into a mirror item (TS: `InboxPromptInput`).
#[derive(Debug, Clone, PartialEq)]
pub struct InboxPromptInput {
    pub display_text: String,
    pub response_mode: Option<ResponseMode>,
    pub inbox: QueuedPromptInbox,
}

/// Bounded in-memory queue of prepared prompts (TS: `PromptQueueManager`).
#[derive(Debug, Default)]
pub struct PromptQueueManager {
    items: Vec<QueuedPrompt>,
    next_id: u64,
    queued_media_bytes: i64,
    reservations: std::collections::HashSet<String>,
    handed_over_reservations: std::collections::HashMap<String, String>,
    delivered_inbox_ids: Vec<String>,
}

impl PromptQueueManager {
    /// Creates an empty queue.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            queued_media_bytes: 0,
            reservations: std::collections::HashSet::new(),
            handed_over_reservations: std::collections::HashMap::new(),
            delivered_inbox_ids: Vec::new(),
        }
    }

    /// Queues a prepared prompt; `None` when it is empty, the queue is
    /// full, or the media budget refuses it (TS: `add`).
    pub fn add(&mut self, input: QueuedPromptInput) -> Option<QueuedPrompt> {
        let normalized_text = input.text.trim().to_string();
        let fallback = if normalized_text.is_empty() {
            "[Attachment]".to_string()
        } else {
            normalized_text.clone()
        };
        let display_text = input
            .display_text
            .as_deref()
            .map(|s| s.trim().to_string())
            .unwrap_or(fallback)
            .trim()
            .to_string();
        let media_bytes = input.media_bytes.unwrap_or(0);
        if (normalized_text.is_empty() && input.file_parts.is_empty() && input.photos.is_empty())
            || display_text.is_empty()
            || self.is_full()
            || !self.can_accept_media(media_bytes)
        {
            return None;
        }

        let item = QueuedPrompt {
            id: format!("queued-{}", self.next_id),
            text: normalized_text,
            file_parts: input.file_parts.clone(),
            photos: input.photos.clone(),
            display_text,
            response_mode: input.response_mode,
            media_bytes,
            inbox: None,
        };
        self.next_id += 1;
        self.items.push(item.clone());
        self.queued_media_bytes += media_bytes;
        Some(item)
    }

    /// Holds a slot for an admission to OpenCode; `None` when the queue is
    /// full (TS: `reserve`).
    pub fn reserve(&mut self) -> Option<String> {
        if self.is_full() {
            return None;
        }
        let reservation_id = format!("reserved-{}", self.next_id);
        self.next_id += 1;
        self.reservations.insert(reservation_id.clone());
        Some(reservation_id)
    }

    /// Turns a reservation into a mirror item. Returns `None` when the
    /// reservation is gone: the queue was cleared while the admission was
    /// on its way (TS: `confirmReservation`).
    pub fn confirm_reservation(
        &mut self,
        reservation_id: &str,
        input: InboxPromptInput,
    ) -> Option<QueuedPrompt> {
        if !self.reservations.remove(reservation_id) {
            return None;
        }
        let trimmed = input.display_text.trim();
        let display_text = if trimmed.is_empty() {
            "[Attachment]".to_string()
        } else {
            trimmed.to_string()
        };
        let item = QueuedPrompt {
            id: format!("queued-{}", self.next_id),
            text: String::new(),
            file_parts: Vec::new(),
            photos: Vec::new(),
            display_text,
            response_mode: input.response_mode,
            media_bytes: 0,
            inbox: Some(QueuedPromptInbox {
                session_id: input.inbox.session_id.clone(),
                inbox_id: input.inbox.inbox_id.clone(),
                delivery: input.inbox.delivery.clone(),
            }),
        };
        self.next_id += 1;
        self.items.push(item.clone());
        Some(item)
    }

    /// Releases a reservation; `false` when a clear already dropped it
    /// (TS: `releaseReservation`).
    pub fn release_reservation(&mut self, reservation_id: &str) -> bool {
        self.reservations.remove(reservation_id)
    }

    /// Finds the mirror item carrying an inbox id, by copy
    /// (TS: `findByInboxId`).
    pub fn find_by_inbox_id(&self, inbox_id: &str) -> Option<QueuedPrompt> {
        self.items
            .iter()
            .find(|candidate| {
                candidate
                    .inbox
                    .as_ref()
                    .is_some_and(|inbox| inbox.inbox_id == inbox_id)
            })
            .cloned()
    }

    /// Remembers an inbox id delivered before any item carried it
    /// (TS: `rememberDeliveredInboxId`).
    pub fn remember_delivered_inbox_id(&mut self, inbox_id: &str) {
        self.delivered_inbox_ids.push(inbox_id.to_string());
        if self.delivered_inbox_ids.len() > MAX_REMEMBERED_DELIVERED_INBOX_IDS {
            self.delivered_inbox_ids.remove(0);
        }
    }

    /// Whether an inbox id was remembered as delivered
    /// (TS: `wasInboxIdDelivered`).
    pub fn was_inbox_id_delivered(&self, inbox_id: &str) -> bool {
        self.delivered_inbox_ids.iter().any(|id| id == inbox_id)
    }

    /// Lists the queued items, by copy (TS: `list`).
    pub fn list(&self) -> Vec<QueuedPrompt> {
        self.items.clone()
    }

    /// Removes the item with an id, or `None` (TS: `removeById`).
    pub fn remove_by_id(&mut self, id: &str) -> Option<QueuedPrompt> {
        let index = self.items.iter().position(|item| item.id == id)?;
        let removed = self.items.remove(index);
        self.queued_media_bytes -= removed.media_bytes;
        Some(removed)
    }

    /// Takes the oldest queued prompt, or `None` (TS: `takeNext`).
    pub fn take_next(&mut self) -> Option<QueuedPrompt> {
        if self.items.is_empty() {
            return None;
        }
        let item = self.items.remove(0);
        self.queued_media_bytes -= item.media_bytes;
        Some(item)
    }

    /// Number of queued items (TS: `size`).
    pub fn size(&self) -> usize {
        self.items.len()
    }

    /// Whether items plus reservations reach the cap (TS: `isFull`).
    pub fn is_full(&self) -> bool {
        self.items.len() + self.reservations.len() >= MAX_QUEUED_PROMPTS
    }

    /// Whether `media_bytes` still fits the media budget
    /// (TS: `canAcceptMedia`).
    pub fn can_accept_media(&self, media_bytes: i64) -> bool {
        if media_bytes < 0 {
            return false;
        }
        match self.queued_media_bytes.checked_add(media_bytes) {
            Some(total) => total <= MAX_QUEUED_MEDIA_BYTES,
            None => false,
        }
    }

    /// Raw media bytes retained by the queued items (TS: `mediaSize`).
    pub fn media_size(&self) -> i64 {
        self.queued_media_bytes
    }

    /// Empties the queue for `/detach` and returns the items it removed.
    /// Admissions still on their way stay with the detached session: their
    /// reservations are handed over rather than dropped
    /// (TS: `handOver`; the log-only `reason` is dropped).
    pub fn hand_over(&mut self, session_id: &str) -> Vec<QueuedPrompt> {
        for reservation_id in self.reservations.iter() {
            self.handed_over_reservations
                .insert(reservation_id.clone(), session_id.to_string());
        }
        let removed = std::mem::take(&mut self.items);
        self.reservations.clear();
        self.queued_media_bytes = 0;
        removed
    }

    /// Releases a handed-over reservation; `false` when it was not handed
    /// over or was withdrawn (TS: `releaseHandedOverReservation`).
    pub fn release_handed_over_reservation(&mut self, reservation_id: &str) -> bool {
        self.handed_over_reservations.remove(reservation_id).is_some()
    }

    /// Withdraws handed-over reservations of one session, or of all when
    /// none is given (TS: `withdrawHandedOverReservations`).
    pub fn withdraw_handed_over_reservations(&mut self, session_id: Option<&str>) {
        let to_withdraw: Vec<String> = self
            .handed_over_reservations
            .iter()
            .filter(|(_, reserved_session_id)| {
                session_id.map_or(true, |wanted| *reserved_session_id == wanted)
            })
            .map(|(reservation_id, _)| reservation_id.clone())
            .collect();
        for reservation_id in to_withdraw {
            self.handed_over_reservations.remove(&reservation_id);
        }
    }

    /// Empties the queue, reservations included, and returns the items it
    /// removed (TS: `clear`; the log-only `reason` is dropped).
    pub fn clear(&mut self) -> Vec<QueuedPrompt> {
        self.delivered_inbox_ids.clear();
        if self.items.is_empty() && self.reservations.is_empty() {
            return Vec::new();
        }
        let removed = std::mem::take(&mut self.items);
        self.reservations.clear();
        self.queued_media_bytes = 0;
        removed
    }

    /// Test helper: restores the empty initial state (TS: `__resetForTests`
    /// on the singleton).
    pub fn reset_for_tests(&mut self) {
        self.items.clear();
        self.next_id = 1;
        self.queued_media_bytes = 0;
        self.reservations.clear();
        self.handed_over_reservations.clear();
        self.delivered_inbox_ids.clear();
    }
}
