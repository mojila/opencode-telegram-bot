//! Per-session tool message batching and send ordering.
//!
//! Rust port of `src/app/formatters/tool-message-batcher.ts`
//! (`ToolMessageBatcher` + `ToolMessageBatcherOptions`).
//!
//! TS -> Rust name mapping:
//! - `ToolMessageBatcher` -> [`ToolMessageBatcher`]
//! - `enqueue` -> [`ToolMessageBatcher::enqueue`]
//! - `sendTextNow` -> [`ToolMessageBatcher::send_text_now`]
//! - `enqueueUniqueByPrefix` -> [`ToolMessageBatcher::enqueue_unique_by_prefix`]
//! - `enqueueFile` -> [`ToolMessageBatcher::enqueue_file`]
//! - `flushSession` -> [`ToolMessageBatcher::flush_session`]
//! - `flushAll` -> [`ToolMessageBatcher::flush_all`]
//! - `clearSession` -> [`ToolMessageBatcher::clear_session`]
//! - `clearAll` -> [`ToolMessageBatcher::clear_all`]
//!
//! Sync mapping (contract for the GREEN implementation):
//! - TS delivers through `sendText` / `sendFile` callbacks chained per
//!   session (`sessionTasks`) with `await`; the port delivers synchronously
//!   through `FnMut` callbacks, so "immediate" means visible after the
//!   matching [`ToolMessageBatcher::flush_session`] /
//!   [`ToolMessageBatcher::flush_all`] call.
//! - TS `takeTextGate(sessionId)` returns a promise each queued text awaits;
//!   the port models it as [`TakeTextGateFn`], returning `true` while the
//!   text must wait (stays pending) and `false` to send. The gate is
//!   consulted once per pending text per flush and never for files.
//! - TS `clearSession` / `clearAll` bump `generation` so queued-but-waiting
//!   sends are dropped; the port drops stale-generation sends the same way.
//! - TS `sendTextNow` trims and drops empty messages / empty session ids;
//!   the port keeps that rule.

use std::collections::VecDeque;

/// One queued send. Texts and files share a single queue so delivery order
/// across both kinds is preserved. (TS: per-session promise chain.)
enum PendingSend {
    Text {
        session: String,
        text: String,
        generation: u64,
    },
    File {
        session: String,
        file: CodeFileData,
        generation: u64,
    },
}

/// File payload for [`ToolMessageBatcher::enqueue_file`].
/// (TS: `CodeFileData` in `src/app/formatters/summary-formatter.ts`.)
#[derive(Debug, Clone, PartialEq)]
pub struct CodeFileData {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub caption: String,
}

/// Sends a text message for a session. (TS: `SendTextCallback`.)
pub type SendTextFn = Box<dyn FnMut(&str, &str)>;

/// Sends a file message for a session. (TS: `SendFileCallback`.)
pub type SendFileFn = Box<dyn FnMut(&str, &CodeFileData)>;

/// Decides whether a queued text message must wait: `true` keeps it pending,
/// `false` lets it send. Consulted once per pending text per flush, never
/// for files. (TS: `takeTextGate`, which returns a promise to await or
/// `undefined` to send immediately.)
pub type TakeTextGateFn = Box<dyn FnMut(&str) -> bool>;

/// Construction options. (TS: `ToolMessageBatcherOptions`.)
pub struct ToolMessageBatcherOptions {
    pub send_text: SendTextFn,
    pub send_file: SendFileFn,
    pub take_text_gate: Option<TakeTextGateFn>,
}

/// Batches tool messages per session, preserving order across text and file
/// sends and dropping pending sends when the session (or everything) is
/// cleared. (TS: `ToolMessageBatcher`.)
#[allow(dead_code)]
pub struct ToolMessageBatcher {
    send_text: SendTextFn,
    send_file: SendFileFn,
    take_text_gate: Option<TakeTextGateFn>,
    generation: u64,
    pending: VecDeque<PendingSend>,
}

impl ToolMessageBatcher {
    /// Creates a batcher from its send callbacks. (TS: `constructor`.)
    pub fn new(options: ToolMessageBatcherOptions) -> Self {
        Self {
            send_text: options.send_text,
            send_file: options.send_file,
            take_text_gate: options.take_text_gate,
            generation: 0,
            pending: VecDeque::new(),
        }
    }

    /// Queues a text message for immediate (ordered) delivery.
    /// (TS: `enqueue`.)
    pub fn enqueue(&mut self, session_id: &str, message: &str) {
        self.send_text_now(session_id, message, "enqueue");
    }

    /// Sends a text message immediately with a debug reason. Empty
    /// (post-trim) messages and empty session ids are dropped.
    /// (TS: `sendTextNow`.)
    pub fn send_text_now(&mut self, session_id: &str, message: &str, _reason: &str) {
        let normalized = message.trim();
        if session_id.is_empty() || normalized.is_empty() {
            return;
        }
        let generation = self.generation;
        self.pending.push_back(PendingSend::Text {
            session: session_id.to_string(),
            text: normalized.to_string(),
            generation,
        });
    }

    /// Queues a text message keyed by prefix for dedup purposes; current TS
    /// behaviour forwards straight to the immediate text path.
    /// (TS: `enqueueUniqueByPrefix`.)
    pub fn enqueue_unique_by_prefix(&mut self, session_id: &str, message: &str, _prefix: &str) {
        self.send_text_now(session_id, message, "enqueue_unique_by_prefix");
    }

    /// Queues a file for immediate (ordered, never gated) delivery.
    /// (TS: `enqueueFile`.)
    pub fn enqueue_file(&mut self, session_id: &str, file: CodeFileData) {
        if session_id.is_empty() {
            return;
        }
        let generation = self.generation;
        self.pending.push_back(PendingSend::File {
            session: session_id.to_string(),
            file,
            generation,
        });
    }

    /// Delivers everything pending for a session, in queue order.
    /// (TS: `flushSession`.)
    pub fn flush_session(&mut self, session_id: &str, _reason: &str) {
        let pending = std::mem::take(&mut self.pending);
        let mut kept = VecDeque::with_capacity(pending.len());
        for item in pending {
            if item.session() != session_id {
                kept.push_back(item);
                continue;
            }
            self.deliver_or_keep(item, &mut kept);
        }
        self.pending = kept;
    }

    /// Delivers everything pending across sessions, in queue order.
    /// (TS: `flushAll`.)
    pub fn flush_all(&mut self, _reason: &str) {
        let pending = std::mem::take(&mut self.pending);
        let mut kept = VecDeque::with_capacity(pending.len());
        for item in pending {
            self.deliver_or_keep(item, &mut kept);
        }
        self.pending = kept;
    }

    /// Drops pending (gated) sends for one session.
    /// (TS: `clearSession`.)
    pub fn clear_session(&mut self, _session_id: &str, _reason: &str) {
        self.generation += 1;
    }

    /// Drops all pending (gated) sends. (TS: `clearAll`.)
    pub fn clear_all(&mut self, _reason: &str) {
        self.generation += 1;
    }

    /// Delivers one queued send if it is still fresh and ungated, otherwise
    /// keeps it (gated text) or drops it (stale generation).
    fn deliver_or_keep(&mut self, item: PendingSend, kept: &mut VecDeque<PendingSend>) {
        if item.generation() != self.generation {
            return;
        }
        match item {
            PendingSend::Text {
                session,
                text,
                generation,
            } => {
                let wait = match self.take_text_gate.as_mut() {
                    Some(gate) => gate(&session),
                    None => false,
                };
                if wait {
                    kept.push_back(PendingSend::Text {
                        session,
                        text,
                        generation,
                    });
                } else {
                    (self.send_text)(&session, &text);
                }
            }
            PendingSend::File { session, file, .. } => {
                (self.send_file)(&session, &file);
            }
        }
    }
}

impl PendingSend {
    fn session(&self) -> &str {
        match self {
            PendingSend::Text { session, .. } => session,
            PendingSend::File { session, .. } => session,
        }
    }

    fn generation(&self) -> u64 {
        match self {
            PendingSend::Text { generation, .. } => *generation,
            PendingSend::File { generation, .. } => *generation,
        }
    }
}
