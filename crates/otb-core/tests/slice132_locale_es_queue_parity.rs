//! Slice 132 parity: Spanish queue-table gates (RED).
//!
//! Mirrors the pure TS contract in `src/i18n/es.ts` (lines 467-476) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `es` templates for the 8 queue keys
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
//! `es` status lookup + formatting (`locale_es_status_gates`, slice 131).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_es_queue_gates` is ported (only the constant-value tests pass).

use otb_core::locale_es_queue_gates::*;

// ---- constants (3) ----

#[test]
fn es_queue_locale_value_parity() {
    assert_eq!(ES_QUEUE_LOCALE, "es");
}

#[test]
fn es_queue_keys_value_parity() {
    assert_eq!(ES_QUEUE_KEYS.len(), 8);
    assert!(ES_QUEUE_KEYS.contains(&"queue.added"));
    assert!(ES_QUEUE_KEYS.contains(&"queue.full"));
    assert!(ES_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(ES_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn es_queue_templates_value_parity() {
    assert_eq!(
        ES_QUEUE_ADDED,
        "📥 Añadido a la cola ({count}/{max}). Se enviará cuando termine la tarea actual."
    );
    assert_eq!(
        ES_QUEUE_FULL,
        "⚠️ La cola está llena ({max}). Elimina un mensaje o espera a que termine la tarea actual."
    );
    assert_eq!(
        ES_QUEUE_MEDIA_LIMIT,
        "⚠️ Los archivos multimedia en cola están limitados a {maxSizeMb} MiB. Espera a que se envíe un elemento."
    );
    assert_eq!(
        ES_QUEUE_REMOVED,
        "🗑 Mensaje eliminado de la cola."
    );
    assert_eq!(
        ES_QUEUE_NOT_FOUND,
        "Este mensaje ya no está en la cola."
    );
    assert_eq!(
        ES_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        ES_QUEUE_DISABLED_HINT,
        "La cola de mensajes se activa en /settings."
    );
    assert_eq!(ES_KEYBOARD_UPDATED, "⌨️ Teclado actualizado");
}

// ---- lookup (2) ----

#[test]
fn es_queue_lookup_added_parity() {
    assert_eq!(
        lookup_es_queue_template("queue.added"),
        Some("📥 Añadido a la cola ({count}/{max}). Se enviará cuando termine la tarea actual.")
    );
}

#[test]
fn es_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_es_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_es_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn es_queue_placeholders_match_en_parity() {
    assert!(is_es_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 Añadido a la cola ({count}/{max}). Se enviará cuando termine la tarea actual."
    ));
    assert!(is_es_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ Teclado actualizado"
    ));
}

#[test]
fn es_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_es_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ La cola está llena. Elimina un mensaje."
    ));
    assert!(!is_es_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "Este mensaje ya no está en la cola. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn es_queue_added_line_parity() {
    assert_eq!(
        format_es_queue_added(1, 5),
        "📥 Añadido a la cola (1/5). Se enviará cuando termine la tarea actual."
    );
}

#[test]
fn es_queue_full_line_parity() {
    assert_eq!(
        format_es_queue_full(5),
        "⚠️ La cola está llena (5). Elimina un mensaje o espera a que termine la tarea actual."
    );
}
