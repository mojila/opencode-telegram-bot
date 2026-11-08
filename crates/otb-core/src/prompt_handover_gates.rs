//! Prompt-handover pure state machine (slice 48, GREEN).
//!
//! Rust port of the pure in-memory remainder in:
//! - `src/app/managers/prompt-handover-manager.ts`
//!   (`PromptHandoverManager`: `recordDetach` new-vs-refresh,
//!   `takeTicket` / `wasDetachedSince` arrival-ticket gate, `addPrompt` /
//!   `addInboxEntry` misses when no record, `takeNextPrompt` FIFO shift,
//!   `setTurnInFlight`, `hasPendingPrompts` prompts-or-flight,
//!   `forgetInboxId` across sessions, `withdraw` / `withdrawAll`
//!   inbox-entry return)
//!
//! Step 2 (GREEN): pure port landed; the parity tests in
//! `crates/otb-core/tests/slice48_prompt_handover_parity.rs`
//! assert the TS contract.
//!
//! No grammy/Telegram I/O, no network/fetch, no timers live here — only
//! the detach bookkeeping the delivery loop in
//! `src/bot/handlers/prompt-handover.ts` drives.
//!
//! DEVIATIONS from TS:
//! 1. `recordDetach` returns the record in TS; the port returns the new
//!    `detach_seq` (`u64`) and tests read the record back via `get`.
//! 2. `IncomingPrompt` file/photo payloads (`FilePartInput`,
//!    `TelegramPhotoInput`) are modelled as opaque `String` entries: the
//!    manager never inspects them, it only carries them.
//! 3. `withdraw` / `withdrawAll` take a `reason: &str` for signature parity
//!    with TS; the reason only feeds the TS log line.
//! 4. `InboxDelivery` is reused from `crate::prompt_queue_events`
//!    (TS: `"steer" | "queue"` in `prompt-queue-manager.ts`).

use std::collections::BTreeMap;

use crate::prompt_queue_events::InboxDelivery;

/// Agent, model and variant the chat had when detached (TS:
/// `HandoverSelection`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoverSelection {
    pub agent: Option<String>,
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

/// A user prompt as the handover carries it (TS: `IncomingPrompt`: text
/// plus opaque file/photo payloads).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IncomingPrompt {
    pub text: String,
    pub file_parts: Vec<String>,
    pub photos: Vec<String>,
}

/// A V1 prompt the bot still holds for a detached session (TS:
/// `HandedOverPrompt`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandedOverPrompt {
    pub text: String,
    pub file_parts: Vec<String>,
    pub photos: Vec<String>,
    pub selection: HandoverSelection,
}

/// Where a prompt waits in the OpenCode V2 session inbox (TS:
/// `QueuedPromptInbox`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPromptInbox {
    pub session_id: String,
    pub inbox_id: String,
    pub delivery: InboxDelivery,
}

/// What waits for one session the bot was detached from (TS:
/// `SessionHandover`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionHandover {
    pub session_id: String,
    pub directory: String,
    pub selection: HandoverSelection,
    pub prompts: Vec<HandedOverPrompt>,
    pub inbox_entries: Vec<QueuedPromptInbox>,
    pub detach_seq: u64,
    pub turn_in_flight: bool,
}

/// The busy session a message arrived for, taken before preparation (TS:
/// `ArrivalTicket`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrivalTicket {
    pub session_id: String,
    pub detach_seq: u64,
}

/// Prompt-handover bookkeeping (TS: `PromptHandoverManager` singleton).
#[derive(Debug, Default)]
pub struct PromptHandover {
    sessions: BTreeMap<String, SessionHandover>,
    detach_seq: u64,
}

impl PromptHandover {
    /// Fresh, empty bookkeeping (TS: singleton construction + reset state).
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            detach_seq: 0,
        }
    }

    /// Records a detach; refreshes the record when one exists (TS:
    /// `recordDetach`: bumps the global seq, updates directory/selection/
    /// `detachSeq`, keeps prompts/inbox entries). Returns the new seq.
    pub fn record_detach(
        &mut self,
        session_id: &str,
        directory: &str,
        selection: HandoverSelection,
    ) -> u64 {
        self.detach_seq += 1;
        let seq = self.detach_seq;
        match self.sessions.get_mut(session_id) {
            Some(existing) => {
                existing.directory = directory.to_string();
                existing.selection = selection;
                existing.detach_seq = seq;
            }
            None => {
                self.sessions.insert(
                    session_id.to_string(),
                    SessionHandover {
                        session_id: session_id.to_string(),
                        directory: directory.to_string(),
                        selection,
                        prompts: Vec::new(),
                        inbox_entries: Vec::new(),
                        detach_seq: seq,
                        turn_in_flight: false,
                    },
                );
            }
        }
        seq
    }

    /// The record for a session, if any (TS: `get`).
    pub fn get(&self, session_id: &str) -> Option<&SessionHandover> {
        self.sessions.get(session_id)
    }

    /// Captures the current seq for a session (TS: `takeTicket`).
    pub fn take_ticket(&self, session_id: &str) -> ArrivalTicket {
        ArrivalTicket {
            session_id: session_id.to_string(),
            detach_seq: self.detach_seq,
        }
    }

    /// Whether the ticket's session was detached after arrival (TS:
    /// `wasDetachedSince`: record exists and its seq is newer).
    pub fn was_detached_since(&self, ticket: &ArrivalTicket) -> bool {
        match self.sessions.get(&ticket.session_id) {
            Some(record) => record.detach_seq > ticket.detach_seq,
            None => false,
        }
    }

    /// Holds a V1 prompt for a detached session (TS: `addPrompt`;
    /// `false` when no record).
    pub fn add_prompt(&mut self, session_id: &str, prompt: HandedOverPrompt) -> bool {
        match self.sessions.get_mut(session_id) {
            Some(record) => {
                record.prompts.push(prompt);
                true
            }
            None => false,
        }
    }

    /// Holds a V2 inbox entry for a detached session (TS:
    /// `addInboxEntry`; `false` when no record for the entry's session).
    pub fn add_inbox_entry(&mut self, inbox: QueuedPromptInbox) -> bool {
        let session_id = inbox.session_id.clone();
        match self.sessions.get_mut(&session_id) {
            Some(record) => {
                record.inbox_entries.push(inbox);
                true
            }
            None => false,
        }
    }

    /// Shifts the oldest held prompt (TS: `takeNextPrompt`; `None` when
    /// the session is unknown or holds nothing).
    pub fn take_next_prompt(&mut self, session_id: &str) -> Option<HandedOverPrompt> {
        let record = self.sessions.get_mut(session_id)?;
        if record.prompts.is_empty() {
            return None;
        }
        Some(record.prompts.remove(0))
    }

    /// Marks whether the session's handed-over turn is in flight (TS:
    /// `setTurnInFlight`; no-op when unknown).
    pub fn set_turn_in_flight(&mut self, session_id: &str, in_flight: bool) {
        if let Some(record) = self.sessions.get_mut(session_id) {
            record.turn_in_flight = in_flight;
        }
    }

    /// Whether V1 prompts are still to be sent or answered (TS:
    /// `hasPendingPrompts`: prompts remain or a turn is in flight).
    pub fn has_pending_prompts(&self, session_id: &str) -> bool {
        match self.sessions.get(session_id) {
            Some(record) => !record.prompts.is_empty() || record.turn_in_flight,
            None => false,
        }
    }

    /// OpenCode picked the prompt up: it can no longer be withdrawn (TS:
    /// `forgetInboxId`: drops the id from every session).
    pub fn forget_inbox_id(&mut self, inbox_id: &str) {
        for record in self.sessions.values_mut() {
            record.inbox_entries.retain(|entry| entry.inbox_id != inbox_id);
        }
    }

    /// Drops everything handed over to the session (TS: `withdraw`:
    /// deletes the record, returns its inbox entries; `[]` when unknown).
    pub fn withdraw(&mut self, session_id: &str, _reason: &str) -> Vec<QueuedPromptInbox> {
        match self.sessions.remove(session_id) {
            Some(record) => record.inbox_entries,
            None => Vec::new(),
        }
    }

    /// Drops everything handed over to any session (TS: `withdrawAll`).
    pub fn withdraw_all(&mut self, reason: &str) -> Vec<QueuedPromptInbox> {
        // TS fans out over a snapshot of keys and calls `withdraw` per
        // session; draining here is equivalent (reason only feeds logs).
        let ids: Vec<String> = self.sessions.keys().cloned().collect();
        let mut out = Vec::new();
        for id in ids {
            out.extend(self.withdraw(&id, reason));
        }
        out
    }

    /// Forgets all sessions and restarts the seq (TS: `__resetForTests`).
    pub fn reset(&mut self) {
        self.sessions.clear();
        self.detach_seq = 0;
    }
}
