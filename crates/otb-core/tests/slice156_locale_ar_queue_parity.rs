//! Slice 156 parity: Arabic queue-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/ar.ts` (lines 442-449) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `ar` templates for the 8 queue keys
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
//! `tr` status lookup + formatting (`locale_tr_status_gates`, slice 151),
//! `tr` queue lookup + formatting (`locale_tr_queue_gates`, slice 152),
//! `ar` status lookup + formatting (`locale_ar_status_gates`, slice 155).

use otb_core::locale_ar_queue_gates::*;

// ---- constants (3) ----

#[test]
fn ar_queue_locale_value_parity() {
    assert_eq!(AR_QUEUE_LOCALE, "ar");
}

#[test]
fn ar_queue_keys_value_parity() {
    assert_eq!(AR_QUEUE_KEYS.len(), 8);
    assert!(AR_QUEUE_KEYS.contains(&"queue.added"));
    assert!(AR_QUEUE_KEYS.contains(&"queue.full"));
    assert!(AR_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(AR_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn ar_queue_templates_value_parity() {
    assert_eq!(
        AR_QUEUE_ADDED,
        "📥 أُضيفت إلى قائمة الانتظار ({count}/{max}). ستُرسل بعد انتهاء المهمة الحالية."
    );
    assert_eq!(
        AR_QUEUE_FULL,
        "⚠️ قائمة الانتظار ممتلئة ({max}). احذف رسالة أو انتظر انتهاء المهمة الحالية."
    );
    assert_eq!(
        AR_QUEUE_MEDIA_LIMIT,
        "⚠️ الوسائط في قائمة الانتظار محدودة بـ {maxSizeMb} MiB. انتظر إرسال عنصر ثم أعد المحاولة."
    );
    assert_eq!(AR_QUEUE_REMOVED, "🗑 تمت إزالة الرسالة من قائمة الانتظار.");
    assert_eq!(AR_QUEUE_NOT_FOUND, "لم تعد هذه الرسالة في قائمة الانتظار.");
    assert_eq!(
        AR_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        AR_QUEUE_DISABLED_HINT,
        "يمكن تفعيل قائمة انتظار الرسائل من /settings."
    );
    assert_eq!(AR_KEYBOARD_UPDATED, "⌨️ تم تحديث لوحة التحكم");
}

// ---- lookup (2) ----

#[test]
fn ar_queue_lookup_added_parity() {
    assert_eq!(
        lookup_ar_queue_template("queue.added"),
        Some("📥 أُضيفت إلى قائمة الانتظار ({count}/{max}). ستُرسل بعد انتهاء المهمة الحالية.")
    );
}

#[test]
fn ar_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ar_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_ar_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ar_queue_placeholders_match_en_parity() {
    assert!(is_ar_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 أُضيفت إلى قائمة الانتظار ({count}/{max}). ستُرسل بعد انتهاء المهمة الحالية."
    ));
    assert!(is_ar_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ تم تحديث لوحة التحكم"
    ));
}

#[test]
fn ar_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_ar_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ قائمة الانتظار ممتلئة. احذف رسالة."
    ));
    assert!(!is_ar_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "لم تعد هذه الرسالة في قائمة الانتظار. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn ar_queue_added_line_parity() {
    assert_eq!(
        format_ar_queue_added(1, 5),
        "📥 أُضيفت إلى قائمة الانتظار (1/5). ستُرسل بعد انتهاء المهمة الحالية."
    );
}

#[test]
fn ar_queue_full_line_parity() {
    assert_eq!(
        format_ar_queue_full(5),
        "⚠️ قائمة الانتظار ممتلئة (5). احذف رسالة أو انتظر انتهاء المهمة الحالية."
    );
}
