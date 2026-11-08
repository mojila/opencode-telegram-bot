//! Slice 136 parity: Italian queue-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/it.ts` (lines 466-473) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `it` templates for the 8 queue keys
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
//! `it` status lookup + formatting (`locale_it_status_gates`, slice 135).

use otb_core::locale_it_queue_gates::*;

// ---- constants (3) ----

#[test]
fn it_queue_locale_value_parity() {
    assert_eq!(IT_QUEUE_LOCALE, "it");
}

#[test]
fn it_queue_keys_value_parity() {
    assert_eq!(IT_QUEUE_KEYS.len(), 8);
    assert!(IT_QUEUE_KEYS.contains(&"queue.added"));
    assert!(IT_QUEUE_KEYS.contains(&"queue.full"));
    assert!(IT_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(IT_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn it_queue_templates_value_parity() {
    assert_eq!(
        IT_QUEUE_ADDED,
        "📥 Aggiunto alla coda ({count}/{max}). Verrà inviato quando l'attività corrente termina."
    );
    assert_eq!(
        IT_QUEUE_FULL,
        "⚠️ La coda è piena ({max}). Rimuovi un messaggio o attendi che l'attività corrente termini."
    );
    assert_eq!(
        IT_QUEUE_MEDIA_LIMIT,
        "⚠️ I media in coda sono limitati a {maxSizeMb} MiB. Attendi l'invio di un elemento e riprova."
    );
    assert_eq!(
        IT_QUEUE_REMOVED,
        "🗑 Messaggio rimosso dalla coda."
    );
    assert_eq!(
        IT_QUEUE_NOT_FOUND,
        "Questo messaggio non è più in coda."
    );
    assert_eq!(
        IT_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        IT_QUEUE_DISABLED_HINT,
        "La coda dei messaggi può essere attivata in /settings."
    );
    assert_eq!(IT_KEYBOARD_UPDATED, "⌨️ Tastiera aggiornata");
}

// ---- lookup (2) ----

#[test]
fn it_queue_lookup_added_parity() {
    assert_eq!(
        lookup_it_queue_template("queue.added"),
        Some("📥 Aggiunto alla coda ({count}/{max}). Verrà inviato quando l'attività corrente termina.")
    );
}

#[test]
fn it_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_it_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_it_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn it_queue_placeholders_match_en_parity() {
    assert!(is_it_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 Aggiunto alla coda ({count}/{max}). Verrà inviato quando l'attività corrente termina."
    ));
    assert!(is_it_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ Tastiera aggiornata"
    ));
}

#[test]
fn it_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_it_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ La coda è piena. Rimuovi un messaggio."
    ));
    assert!(!is_it_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "Questo messaggio non è più in coda. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn it_queue_added_line_parity() {
    assert_eq!(
        format_it_queue_added(1, 5),
        "📥 Aggiunto alla coda (1/5). Verrà inviato quando l'attività corrente termina."
    );
}

#[test]
fn it_queue_full_line_parity() {
    assert_eq!(
        format_it_queue_full(5),
        "⚠️ La coda è piena (5). Rimuovi un messaggio o attendi che l'attività corrente termini."
    );
}
