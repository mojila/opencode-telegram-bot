//! Slice 144 parity: Indonesian queue-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/id.ts` (lines 452-461) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `id` templates for the 8 queue keys
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
//! `id` status lookup + formatting (`locale_id_status_gates`, slice 143).

use otb_core::locale_id_queue_gates::*;

// ---- constants (3) ----

#[test]
fn id_queue_locale_value_parity() {
    assert_eq!(ID_QUEUE_LOCALE, "id");
}

#[test]
fn id_queue_keys_value_parity() {
    assert_eq!(ID_QUEUE_KEYS.len(), 8);
    assert!(ID_QUEUE_KEYS.contains(&"queue.added"));
    assert!(ID_QUEUE_KEYS.contains(&"queue.full"));
    assert!(ID_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(ID_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn id_queue_templates_value_parity() {
    assert_eq!(
        ID_QUEUE_ADDED,
        "📥 Ditambahkan ke antrean ({count}/{max}). Akan dikirim setelah tugas saat ini selesai."
    );
    assert_eq!(
        ID_QUEUE_FULL,
        "⚠️ Antrean penuh ({max}). Hapus pesan atau tunggu tugas saat ini selesai."
    );
    assert_eq!(
        ID_QUEUE_MEDIA_LIMIT,
        "⚠️ Media dalam antrean dibatasi {maxSizeMb} MiB. Tunggu hingga satu item terkirim, lalu coba lagi."
    );
    assert_eq!(
        ID_QUEUE_REMOVED,
        "🗑 Pesan dihapus dari antrean."
    );
    assert_eq!(
        ID_QUEUE_NOT_FOUND,
        "Pesan ini sudah tidak ada di antrean."
    );
    assert_eq!(
        ID_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        ID_QUEUE_DISABLED_HINT,
        "Aktifkan antrean pesan di /settings."
    );
    assert_eq!(ID_KEYBOARD_UPDATED, "⌨️ Keyboard diperbarui");
}

// ---- lookup (2) ----

#[test]
fn id_queue_lookup_added_parity() {
    assert_eq!(
        lookup_id_queue_template("queue.added"),
        Some("📥 Ditambahkan ke antrean ({count}/{max}). Akan dikirim setelah tugas saat ini selesai.")
    );
}

#[test]
fn id_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_id_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_id_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn id_queue_placeholders_match_en_parity() {
    assert!(is_id_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 Ditambahkan ke antrean ({count}/{max}). Akan dikirim setelah tugas saat ini selesai."
    ));
    assert!(is_id_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ Keyboard diperbarui"
    ));
}

#[test]
fn id_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_id_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ Antrean penuh. Hapus pesan."
    ));
    assert!(!is_id_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "Pesan ini sudah tidak ada di antrean. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn id_queue_added_line_parity() {
    assert_eq!(
        format_id_queue_added(1, 5),
        "📥 Ditambahkan ke antrean (1/5). Akan dikirim setelah tugas saat ini selesai."
    );
}

#[test]
fn id_queue_full_line_parity() {
    assert_eq!(
        format_id_queue_full(5),
        "⚠️ Antrean penuh (5). Hapus pesan atau tunggu tugas saat ini selesai."
    );
}
