//! Slice 124 parity: German queue-table gates (RED).
//!
//! Mirrors the pure TS contract in `src/i18n/de.ts` (lines 471-480) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `de` templates for the 8 queue keys
//! - per-key placeholder equality against `en`
//!   (`tests/i18n/placeholders.test.ts` loop)
//! - `t(key, params)` line formatting (`src/i18n/index.ts` `interpolate`
//!   behind `src/bot/handlers/prompt-queue-dispatch.ts`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! placeholder set-equality helpers (`locale_placeholder_gates`, slice 122),
//! registry / `SUPPORTED_LOCALES` / `getLocale` (`crate::i18n` +
//! `locale_registry_gates`), en/ru tables + interpolation (`crate::i18n_full`),
//! `de` status lookup + formatting (`locale_de_status_gates`, slice 123).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_de_queue_gates` is ported (only the constant-value tests pass).

use otb_core::locale_de_queue_gates::*;

// ---- constants (3) ----

#[test]
fn de_queue_locale_value_parity() {
    assert_eq!(DE_QUEUE_LOCALE, "de");
}

#[test]
fn de_queue_keys_value_parity() {
    assert_eq!(DE_QUEUE_KEYS.len(), 8);
    assert!(DE_QUEUE_KEYS.contains(&"queue.added"));
    assert!(DE_QUEUE_KEYS.contains(&"queue.full"));
    assert!(DE_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(DE_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn de_queue_templates_value_parity() {
    assert_eq!(
        DE_QUEUE_ADDED,
        "📥 Zur Warteschlange hinzugefügt ({count}/{max}). Die Nachricht wird gesendet, sobald die aktuelle Aufgabe abgeschlossen ist."
    );
    assert_eq!(
        DE_QUEUE_FULL,
        "⚠️ Die Warteschlange ist voll ({max}). Entferne eine Nachricht oder warte, bis die aktuelle Aufgabe abgeschlossen ist."
    );
    assert_eq!(
        DE_QUEUE_MEDIA_LIMIT,
        "⚠️ Medien in der Warteschlange sind auf {maxSizeMb} MiB begrenzt. Warte, bis ein Eintrag gesendet wurde."
    );
    assert_eq!(
        DE_QUEUE_REMOVED,
        "🗑 Nachricht aus der Warteschlange entfernt."
    );
    assert_eq!(
        DE_QUEUE_NOT_FOUND,
        "Diese Nachricht ist nicht mehr in der Warteschlange."
    );
    assert_eq!(
        DE_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        DE_QUEUE_DISABLED_HINT,
        "Die Nachrichtenwarteschlange lässt sich in /settings aktivieren."
    );
    assert_eq!(DE_KEYBOARD_UPDATED, "⌨️ Tastatur aktualisiert");
}

// ---- lookup (2) ----

#[test]
fn de_queue_lookup_added_parity() {
    assert_eq!(
        lookup_de_queue_template("queue.added"),
        Some("📥 Zur Warteschlange hinzugefügt ({count}/{max}). Die Nachricht wird gesendet, sobald die aktuelle Aufgabe abgeschlossen ist.")
    );
}

#[test]
fn de_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_de_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_de_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn de_queue_placeholders_match_en_parity() {
    assert!(is_de_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 Zur Warteschlange hinzugefügt ({count}/{max}). Die Nachricht wird gesendet, sobald die aktuelle Aufgabe abgeschlossen ist."
    ));
    assert!(is_de_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ Tastatur aktualisiert"
    ));
}

#[test]
fn de_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_de_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ Die Warteschlange ist voll. Entferne eine Nachricht."
    ));
    assert!(!is_de_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "Diese Nachricht ist nicht mehr in der Warteschlange. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn de_queue_added_line_parity() {
    assert_eq!(
        format_de_queue_added(1, 5),
        "📥 Zur Warteschlange hinzugefügt (1/5). Die Nachricht wird gesendet, sobald die aktuelle Aufgabe abgeschlossen ist."
    );
}

#[test]
fn de_queue_full_line_parity() {
    assert_eq!(
        format_de_queue_full(5),
        "⚠️ Die Warteschlange ist voll (5). Entferne eine Nachricht oder warte, bis die aktuelle Aufgabe abgeschlossen ist."
    );
}
