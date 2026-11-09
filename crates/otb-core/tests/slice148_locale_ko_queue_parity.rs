//! Slice 148 parity: Korean queue-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/ko.ts` (lines 461-468) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `ko` templates for the 8 queue keys
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
//! `ko` status lookup + formatting (`locale_ko_status_gates`, slice 147).

use otb_core::locale_ko_queue_gates::*;

// ---- constants (3) ----

#[test]
fn ko_queue_locale_value_parity() {
    assert_eq!(KO_QUEUE_LOCALE, "ko");
}

#[test]
fn ko_queue_keys_value_parity() {
    assert_eq!(KO_QUEUE_KEYS.len(), 8);
    assert!(KO_QUEUE_KEYS.contains(&"queue.added"));
    assert!(KO_QUEUE_KEYS.contains(&"queue.full"));
    assert!(KO_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(KO_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn ko_queue_templates_value_parity() {
    assert_eq!(
        KO_QUEUE_ADDED,
        "📥 대기열에 추가되었습니다 ({count}/{max}). 현재 작업이 끝나면 전송됩니다."
    );
    assert_eq!(
        KO_QUEUE_FULL,
        "⚠️ 대기열이 가득 찼습니다 ({max}). 메시지를 삭제하거나 현재 작업이 끝날 때까지 기다려 주세요."
    );
    assert_eq!(
        KO_QUEUE_MEDIA_LIMIT,
        "⚠️ 대기열 미디어는 총 {maxSizeMb} MiB로 제한됩니다. 항목이 전송된 후 다시 시도하세요."
    );
    assert_eq!(KO_QUEUE_REMOVED, "🗑 대기열에서 메시지를 삭제했습니다.");
    assert_eq!(KO_QUEUE_NOT_FOUND, "이 메시지는 더 이상 대기열에 없습니다.");
    assert_eq!(
        KO_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        KO_QUEUE_DISABLED_HINT,
        "메시지 대기열은 /settings에서 활성화할 수 있습니다."
    );
    assert_eq!(KO_KEYBOARD_UPDATED, "⌨️ 키보드가 업데이트되었습니다");
}

// ---- lookup (2) ----

#[test]
fn ko_queue_lookup_added_parity() {
    assert_eq!(
        lookup_ko_queue_template("queue.added"),
        Some("📥 대기열에 추가되었습니다 ({count}/{max}). 현재 작업이 끝나면 전송됩니다.")
    );
}

#[test]
fn ko_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_ko_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_ko_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn ko_queue_placeholders_match_en_parity() {
    assert!(is_ko_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 대기열에 추가되었습니다 ({count}/{max}). 현재 작업이 끝나면 전송됩니다."
    ));
    assert!(is_ko_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ 키보드가 업데이트되었습니다"
    ));
}

#[test]
fn ko_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_ko_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ 대기열이 가득 찼습니다. 메시지를 삭제하세요."
    ));
    assert!(!is_ko_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "이 메시지는 더 이상 대기열에 없습니다. {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn ko_queue_added_line_parity() {
    assert_eq!(
        format_ko_queue_added(1, 5),
        "📥 대기열에 추가되었습니다 (1/5). 현재 작업이 끝나면 전송됩니다."
    );
}

#[test]
fn ko_queue_full_line_parity() {
    assert_eq!(
        format_ko_queue_full(5),
        "⚠️ 대기열이 가득 찼습니다 (5). 메시지를 삭제하거나 현재 작업이 끝날 때까지 기다려 주세요."
    );
}
