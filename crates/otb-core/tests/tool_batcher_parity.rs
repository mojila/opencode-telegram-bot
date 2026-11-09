//! Parity port of `tests/app/formatters/tool-message-batcher.test.ts`.
//!
//! Step 1 (RED, cycle 4): these tests assert the TS contract and MUST FAIL
//! with `unimplemented!()` panics until the Rust port lands (step 2).
//!
//! Sync mapping: TS delivers through promise-chained `sendText` / `sendFile`
//! callbacks, so its tests observe delivery via `waitFor` without flushing.
//! The Rust port delivers synchronously through `FnMut` callbacks, so each
//! test below enqueues and then calls `flush_session` before asserting on
//! the shared recorder vecs. Gate semantics map TS `takeTextGate` (a promise
//! to await, or `undefined` to send) to `TakeTextGateFn` (`true` = must wait,
//! `false` = send). Drop-on-clear maps TS `generation` so that texts still
//! pending behind a gate at clear time are never delivered.

use otb_core::formatters::tool_batcher::{
    CodeFileData, TakeTextGateFn, ToolMessageBatcher, ToolMessageBatcherOptions,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

fn file_data(name: &str) -> CodeFileData {
    CodeFileData {
        filename: name.to_string(),
        bytes: b"content".to_vec(),
        caption: "caption".to_string(),
    }
}

type Shared<T> = Rc<RefCell<T>>;

struct Recorders {
    texts: Shared<Vec<(String, String)>>,
    files: Shared<Vec<(String, String)>>,
    gate_calls: Shared<Vec<String>>,
}

fn recorders() -> Recorders {
    Recorders {
        texts: Rc::new(RefCell::new(Vec::new())),
        files: Rc::new(RefCell::new(Vec::new())),
        gate_calls: Rc::new(RefCell::new(Vec::new())),
    }
}

fn build(rec: &Recorders, take_text_gate: Option<TakeTextGateFn>) -> ToolMessageBatcher {
    let texts = Rc::clone(&rec.texts);
    let files = Rc::clone(&rec.files);
    ToolMessageBatcher::new(ToolMessageBatcherOptions {
        send_text: Box::new(move |session: &str, text: &str| {
            texts
                .borrow_mut()
                .push((session.to_string(), text.to_string()));
        }),
        send_file: Box::new(move |session: &str, file: &CodeFileData| {
            files
                .borrow_mut()
                .push((session.to_string(), file.filename.clone()));
        }),
        take_text_gate,
    })
}

fn build_simple(rec: &Recorders) -> ToolMessageBatcher {
    build(rec, None)
}

/// Gate that stays blocked while `flag` is set, recording each consultation.
fn flag_gate(flag: Rc<Cell<bool>>, gate_calls: Shared<Vec<String>>) -> TakeTextGateFn {
    Box::new(move |session: &str| {
        gate_calls.borrow_mut().push(session.to_string());
        flag.get()
    })
}

/// Gate that replays a per-call script (`true` = wait), then passes.
fn script_gate(script: Shared<VecDeque<bool>>, gate_calls: Shared<Vec<String>>) -> TakeTextGateFn {
    Box::new(move |session: &str| {
        gate_calls.borrow_mut().push(session.to_string());
        script.borrow_mut().pop_front().unwrap_or(false)
    })
}

#[test]
fn sends_text_message_immediately_when_enqueued() {
    let rec = recorders();
    let mut batcher = build_simple(&rec);

    batcher.enqueue("s1", "tool message");
    batcher.flush_session("s1", "test");

    assert_eq!(
        *rec.texts.borrow(),
        vec![("s1".to_string(), "tool message".to_string())]
    );
    assert!(rec.files.borrow().is_empty());
}

#[test]
fn sends_text_immediately_when_requested_outside_the_queue() {
    let rec = recorders();
    let mut batcher = build_simple(&rec);

    batcher.send_text_now("s1", "thinking", "thinking_started_streaming");
    batcher.flush_session("s1", "test");

    assert_eq!(
        *rec.texts.borrow(),
        vec![("s1".to_string(), "thinking".to_string())]
    );
    assert!(rec.files.borrow().is_empty());
}

#[test]
fn sends_file_immediately_when_enqueued() {
    let rec = recorders();
    let mut batcher = build_simple(&rec);

    batcher.enqueue_file("s1", file_data("edit_a.ts.txt"));
    batcher.flush_session("s1", "test");

    assert_eq!(
        *rec.files.borrow(),
        vec![("s1".to_string(), "edit_a.ts.txt".to_string())]
    );
    assert!(rec.texts.borrow().is_empty());
}

#[test]
fn flush_session_waits_for_in_flight_gated_send() {
    // Gated texts stay pending across flushes until the gate opens; the
    // second flush then delivers. (TS: `flushSession` awaits the in-flight
    // send before resolving.)
    let rec = recorders();
    let blocked = Rc::new(Cell::new(true));
    let mut batcher = build(
        &rec,
        Some(flag_gate(Rc::clone(&blocked), Rc::clone(&rec.gate_calls))),
    );

    batcher.send_text_now("s1", "thinking", "thinking_started");
    batcher.flush_session("s1", "assistant_message_completed");
    assert!(rec.texts.borrow().is_empty());

    blocked.set(false);
    batcher.flush_session("s1", "assistant_message_completed");
    assert_eq!(
        *rec.texts.borrow(),
        vec![("s1".to_string(), "thinking".to_string())]
    );
}

#[test]
fn drops_pending_sends_for_a_cleared_session() {
    // Both texts are still pending behind the gate at clear time, so neither
    // is delivered even after the gate opens. (TS: the queued-behind send is
    // dropped by the generation bump; the sync port has no in-flight
    // delivery, so every gated send pending at clear time is dropped.)
    let rec = recorders();
    let blocked = Rc::new(Cell::new(true));
    let mut batcher = build(
        &rec,
        Some(flag_gate(Rc::clone(&blocked), Rc::clone(&rec.gate_calls))),
    );

    batcher.enqueue("s1", "first");
    batcher.enqueue("s1", "second");
    batcher.flush_session("s1", "before_clear");
    assert!(rec.texts.borrow().is_empty());

    batcher.clear_session("s1", "test_clear");
    blocked.set(false);
    batcher.flush_session("s1", "after_clear");
    assert!(rec.texts.borrow().is_empty());
}

#[test]
fn preserves_order_for_immediate_mixed_sends() {
    let order: Shared<Vec<String>> = Rc::new(RefCell::new(Vec::new()));
    let order_texts = Rc::clone(&order);
    let order_files = Rc::clone(&order);
    let mut batcher = ToolMessageBatcher::new(ToolMessageBatcherOptions {
        send_text: Box::new(move |_session: &str, text: &str| {
            order_texts.borrow_mut().push(format!("text:{text}"));
        }),
        send_file: Box::new(move |_session: &str, file: &CodeFileData| {
            order_files
                .borrow_mut()
                .push(format!("file:{}", file.filename));
        }),
        take_text_gate: None,
    });

    batcher.enqueue("s1", "first");
    batcher.enqueue_file("s1", file_data("edit_d.ts.txt"));
    batcher.enqueue("s1", "second");
    batcher.flush_session("s1", "test");

    assert_eq!(
        *order.borrow(),
        vec![
            "text:first".to_string(),
            "file:edit_d.ts.txt".to_string(),
            "text:second".to_string(),
        ]
    );
}

#[test]
fn waits_for_the_gate_a_text_was_queued_with() {
    // The gate is taken once per text at queue time: the first text captures
    // "wait" and stays pending, the second captures "pass" and is delivered
    // by the same flush. (TS: first `takeTextGate` call returns a gate
    // promise, later calls return `undefined`.)
    let rec = recorders();
    let script: Shared<VecDeque<bool>> = Rc::new(RefCell::new(VecDeque::from([true, false])));
    let mut batcher = build(
        &rec,
        Some(script_gate(Rc::clone(&script), Rc::clone(&rec.gate_calls))),
    );

    batcher.send_text_now("s1", "gated", "test");
    batcher.send_text_now("s1", "free", "test");
    batcher.flush_session("s1", "test");

    assert_eq!(
        *rec.texts.borrow(),
        vec![("s1".to_string(), "free".to_string())]
    );
    assert_eq!(
        *rec.gate_calls.borrow(),
        vec!["s1".to_string(), "s1".to_string()]
    );
}

#[test]
fn never_gates_a_file() {
    // Even with a gate that would block every text, the file is delivered
    // and the gate is never consulted.
    let rec = recorders();
    let mut batcher = build(
        &rec,
        Some(script_gate(
            Rc::new(RefCell::new(VecDeque::from([true]))),
            Rc::clone(&rec.gate_calls),
        )),
    );

    batcher.enqueue_file("s1", file_data("file.ts"));
    batcher.flush_session("s1", "test");

    assert_eq!(
        *rec.files.borrow(),
        vec![("s1".to_string(), "file.ts".to_string())]
    );
    assert!(rec.gate_calls.borrow().is_empty());
}
