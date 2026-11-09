//! Slice 128 parity: French queue-table gates (RED).
//!
//! Mirrors the pure TS contract in `src/i18n/fr.ts` (lines 473-482) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `fr` templates for the 8 queue keys
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
//! `fr` status lookup + formatting (`locale_fr_status_gates`, slice 127).
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `locale_fr_queue_gates` is ported (only the constant-value tests pass).

use otb_core::locale_fr_queue_gates::*;

// ---- constants (3) ----

#[test]
fn fr_queue_locale_value_parity() {
    assert_eq!(FR_QUEUE_LOCALE, "fr");
}

#[test]
fn fr_queue_keys_value_parity() {
    assert_eq!(FR_QUEUE_KEYS.len(), 8);
    assert!(FR_QUEUE_KEYS.contains(&"queue.added"));
    assert!(FR_QUEUE_KEYS.contains(&"queue.full"));
    assert!(FR_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(FR_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn fr_queue_templates_value_parity() {
    assert_eq!(
        FR_QUEUE_ADDED,
        "📥 Ajouté à la file d'attente ({count}/{max}). Le message sera envoyé à la fin de la tâche en cours."
    );
    assert_eq!(
        FR_QUEUE_FULL,
        "⚠️ La file d'attente est pleine ({max}). Supprimez un message ou attendez la fin de la tâche en cours."
    );
    assert_eq!(
        FR_QUEUE_MEDIA_LIMIT,
        "⚠️ Les médias en file sont limités à {maxSizeMb} MiB. Attendez l'envoi d'un élément."
    );
    assert_eq!(FR_QUEUE_REMOVED, "🗑 Message retiré de la file d'attente.");
    assert_eq!(
        FR_QUEUE_NOT_FOUND,
        "Ce message n'est plus dans la file d'attente."
    );
    assert_eq!(
        FR_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        FR_QUEUE_DISABLED_HINT,
        "La file d'attente des messages s'active dans /settings."
    );
    assert_eq!(FR_KEYBOARD_UPDATED, "⌨️ Clavier mis à jour");
}

// ---- lookup (2) ----

#[test]
fn fr_queue_lookup_added_parity() {
    assert_eq!(
        lookup_fr_queue_template("queue.added"),
        Some("📥 Ajouté à la file d'attente ({count}/{max}). Le message sera envoyé à la fin de la tâche en cours.")
    );
}

#[test]
fn fr_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_fr_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_fr_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn fr_queue_placeholders_match_en_parity() {
    assert!(is_fr_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 Ajouté à la file d'attente ({count}/{max}). Le message sera envoyé à la fin de la tâche en cours."
    ));
    assert!(is_fr_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ Clavier mis à jour"
    ));
}

#[test]
fn fr_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_fr_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ La file d'attente est pleine. Supprimez un message."
    ));
    assert!(!is_fr_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "Ce message n'est plus dans la file d'attente. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn fr_queue_added_line_parity() {
    assert_eq!(
        format_fr_queue_added(1, 5),
        "📥 Ajouté à la file d'attente (1/5). Le message sera envoyé à la fin de la tâche en cours."
    );
}

#[test]
fn fr_queue_full_line_parity() {
    assert_eq!(
        format_fr_queue_full(5),
        "⚠️ La file d'attente est pleine (5). Supprimez un message ou attendez la fin de la tâche en cours."
    );
}
