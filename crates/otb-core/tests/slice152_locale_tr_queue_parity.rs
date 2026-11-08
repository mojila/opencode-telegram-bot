//! Slice 152 parity: Turkish queue-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/tr.ts` (lines 452-461) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `tr` templates for the 8 queue keys
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
//! `pt` status lookup + formatting (`locale_pt_status_gates`, slice 139),
//! `pt` queue lookup + formatting (`locale_pt_queue_gates`, slice 140),
//! `id` status lookup + formatting (`locale_id_status_gates`, slice 143),
//! `id` queue lookup + formatting (`locale_id_queue_gates`, slice 144),
//! `ko` status lookup + formatting (`locale_ko_status_gates`, slice 147),
//! `ko` queue lookup + formatting (`locale_ko_queue_gates`, slice 148),
//! `tr` status lookup + formatting (`locale_tr_status_gates`, slice 151).

use otb_core::locale_tr_queue_gates::*;

// ---- constants (3) ----

#[test]
fn tr_queue_locale_value_parity() {
    assert_eq!(TR_QUEUE_LOCALE, "tr");
}

#[test]
fn tr_queue_keys_value_parity() {
    assert_eq!(TR_QUEUE_KEYS.len(), 8);
    assert!(TR_QUEUE_KEYS.contains(&"queue.added"));
    assert!(TR_QUEUE_KEYS.contains(&"queue.full"));
    assert!(TR_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(TR_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn tr_queue_templates_value_parity() {
    assert_eq!(
        TR_QUEUE_ADDED,
        "📥 Kuyruğa eklendi ({count}/{max}). Geçerli görev bittiğinde gönderilecek."
    );
    assert_eq!(
        TR_QUEUE_FULL,
        "⚠️ Kuyruk dolu ({max}). Bir mesajı kaldırın veya geçerli görevin bitmesini bekleyin."
    );
    assert_eq!(
        TR_QUEUE_MEDIA_LIMIT,
        "⚠️ Kuyruktaki medya {maxSizeMb} MiB ile sınırlıdır. Bir öğenin gönderilmesini bekleyip tekrar deneyin."
    );
    assert_eq!(TR_QUEUE_REMOVED, "🗑 Mesaj kuyruktan kaldırıldı.");
    assert_eq!(TR_QUEUE_NOT_FOUND, "Bu mesaj artık kuyrukta değil.");
    assert_eq!(
        TR_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        TR_QUEUE_DISABLED_HINT,
        "Mesaj kuyruğu /settings içinden açılabilir."
    );
    assert_eq!(TR_KEYBOARD_UPDATED, "⌨️ Klavye güncellendi");
}

// ---- lookup (2) ----

#[test]
fn tr_queue_lookup_added_parity() {
    assert_eq!(
        lookup_tr_queue_template("queue.added"),
        Some("📥 Kuyruğa eklendi ({count}/{max}). Geçerli görev bittiğinde gönderilecek.")
    );
}

#[test]
fn tr_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_tr_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_tr_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn tr_queue_placeholders_match_en_parity() {
    assert!(is_tr_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 Kuyruğa eklendi ({count}/{max}). Geçerli görev bittiğinde gönderilecek."
    ));
    assert!(is_tr_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ Klavye güncellendi"
    ));
}

#[test]
fn tr_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_tr_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ Kuyruk dolu. Bir mesajı kaldırın."
    ));
    assert!(!is_tr_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "Bu mesaj artık kuyrukta değil. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn tr_queue_added_line_parity() {
    assert_eq!(
        format_tr_queue_added(1, 5),
        "📥 Kuyruğa eklendi (1/5). Geçerli görev bittiğinde gönderilecek."
    );
}

#[test]
fn tr_queue_full_line_parity() {
    assert_eq!(
        format_tr_queue_full(5),
        "⚠️ Kuyruk dolu (5). Bir mesajı kaldırın veya geçerli görevin bitmesini bekleyin."
    );
}
