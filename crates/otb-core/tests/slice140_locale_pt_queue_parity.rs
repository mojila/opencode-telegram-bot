//! Slice 140 parity: Portuguese queue-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/pt.ts` (lines 468-477) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `pt` templates for the 8 queue keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind `src/bot/handlers/prompt-queue-dispatch.ts`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `de` status lookup + formatting (`locale_de_status_gates`, slice 123),
//! `de` queue lookup + formatting (`locale_de_queue_gates`, slice 124),
//! `fr` status lookup + formatting (`locale_fr_status_gates`, slice 127),
//! `fr` queue lookup + formatting (`locale_fr_queue_gates`, slice 128),
//! `es` status lookup + formatting (`locale_es_status_gates`, slice 131),
//! `es` queue lookup + formatting (`locale_es_queue_gates`, slice 132),
//! `it` status lookup + formatting (`locale_it_status_gates`, slice 135),
//! `it` queue lookup + formatting (`locale_it_queue_gates`, slice 136),
//! `pt` status lookup + formatting (`locale_pt_status_gates`, slice 139).

use otb_core::locale_pt_queue_gates::*;

// ---- constants (3) ----

#[test]
fn pt_queue_locale_value_parity() {
    assert_eq!(PT_QUEUE_LOCALE, "pt");
}

#[test]
fn pt_queue_keys_value_parity() {
    assert_eq!(PT_QUEUE_KEYS.len(), 8);
    assert!(PT_QUEUE_KEYS.contains(&"queue.added"));
    assert!(PT_QUEUE_KEYS.contains(&"queue.full"));
    assert!(PT_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(PT_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn pt_queue_templates_value_parity() {
    assert_eq!(
        PT_QUEUE_ADDED,
        "📥 Adicionado à fila ({count}/{max}). Será enviado quando a tarefa atual terminar."
    );
    assert_eq!(
        PT_QUEUE_FULL,
        "⚠️ A fila está cheia ({max}). Remova uma mensagem ou aguarde o término da tarefa atual."
    );
    assert_eq!(
        PT_QUEUE_MEDIA_LIMIT,
        "⚠️ A mídia na fila está limitada a {maxSizeMb} MiB. Aguarde o envio de um item."
    );
    assert_eq!(
        PT_QUEUE_REMOVED,
        "🗑 Mensagem removida da fila."
    );
    assert_eq!(
        PT_QUEUE_NOT_FOUND,
        "Esta mensagem não está mais na fila."
    );
    assert_eq!(
        PT_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        PT_QUEUE_DISABLED_HINT,
        "A fila de mensagens pode ser ativada em /settings."
    );
    assert_eq!(PT_KEYBOARD_UPDATED, "⌨️ Teclado atualizado");
}

// ---- lookup (2) ----

#[test]
fn pt_queue_lookup_added_parity() {
    assert_eq!(
        lookup_pt_queue_template("queue.added"),
        Some("📥 Adicionado à fila ({count}/{max}). Será enviado quando a tarefa atual terminar.")
    );
}

#[test]
fn pt_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_pt_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_pt_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn pt_queue_placeholders_match_en_parity() {
    assert!(is_pt_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 Adicionado à fila ({count}/{max}). Será enviado quando a tarefa atual terminar."
    ));
    assert!(is_pt_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ Teclado atualizado"
    ));
}

#[test]
fn pt_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_pt_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ A fila está cheia. Remova uma mensagem."
    ));
    assert!(!is_pt_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "Esta mensagem não está mais na fila. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn pt_queue_added_line_parity() {
    assert_eq!(
        format_pt_queue_added(1, 5),
        "📥 Adicionado à fila (1/5). Será enviado quando a tarefa atual terminar."
    );
}

#[test]
fn pt_queue_full_line_parity() {
    assert_eq!(
        format_pt_queue_full(5),
        "⚠️ A fila está cheia (5). Remova uma mensagem ou aguarde o término da tarefa atual."
    );
}
