//! Slice 160 parity: Chinese queue-table gates (GREEN).
//!
//! Mirrors the pure TS contract in `src/i18n/zh.ts` (lines 419-426) for the
//! queue surface (`QUEUE_KEYS` in `crate::i18n_full` plus `keyboard.updated`):
//! - verbatim `zh` templates for the 8 queue keys
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
//! `ar` status lookup + formatting (`locale_ar_status_gates`, slice 155),
//! `ar` queue lookup + formatting (`locale_ar_queue_gates`, slice 156),
//! `zh` status lookup + formatting (`locale_zh_status_gates`, slice 159).

use otb_core::locale_zh_queue_gates::*;

// ---- constants (3) ----

#[test]
fn zh_queue_locale_value_parity() {
    assert_eq!(ZH_QUEUE_LOCALE, "zh");
}

#[test]
fn zh_queue_keys_value_parity() {
    assert_eq!(ZH_QUEUE_KEYS.len(), 8);
    assert!(ZH_QUEUE_KEYS.contains(&"queue.added"));
    assert!(ZH_QUEUE_KEYS.contains(&"queue.full"));
    assert!(ZH_QUEUE_KEYS.contains(&"keyboard.updated"));
    assert!(ZH_QUEUE_KEYS.contains(&"queue.steer_added"));
}

#[test]
fn zh_queue_templates_value_parity() {
    assert_eq!(
        ZH_QUEUE_ADDED,
        "📥 已加入队列（{count}/{max}）。当前任务完成后将自动发送。"
    );
    assert_eq!(
        ZH_QUEUE_FULL,
        "⚠️ 队列已满（{max}）。请删除一条消息或等待当前任务完成。"
    );
    assert_eq!(
        ZH_QUEUE_MEDIA_LIMIT,
        "⚠️ 队列媒体总大小限制为 {maxSizeMb} MiB。请等待一个项目发送后重试。"
    );
    assert_eq!(ZH_QUEUE_REMOVED, "🗑 消息已从队列中移除。");
    assert_eq!(ZH_QUEUE_NOT_FOUND, "该消息已不在队列中。");
    assert_eq!(
        ZH_QUEUE_STEER_ADDED,
        "📥 Added to the current task ({count}/{max})."
    );
    assert_eq!(
        ZH_QUEUE_DISABLED_HINT,
        "可在 /settings 中开启消息队列。"
    );
    assert_eq!(ZH_KEYBOARD_UPDATED, "⌨️ 键盘已更新");
}

// ---- lookup (2) ----

#[test]
fn zh_queue_lookup_added_parity() {
    assert_eq!(
        lookup_zh_queue_template("queue.added"),
        Some("📥 已加入队列（{count}/{max}）。当前任务完成后将自动发送。")
    );
}

#[test]
fn zh_queue_lookup_unknown_returns_none_parity() {
    assert_eq!(lookup_zh_queue_template("queue.unknown_key"), None);
    assert_eq!(lookup_zh_queue_template("status.line.health"), None);
}

// ---- placeholder equality (2) ----

#[test]
fn zh_queue_placeholders_match_en_parity() {
    assert!(is_zh_queue_placeholder_ok(
        "📥 Added to queue ({count}/{max}). It will be sent when the current task finishes.",
        "📥 已加入队列（{count}/{max}）。当前任务完成后将自动发送。"
    ));
    assert!(is_zh_queue_placeholder_ok(
        "⌨️ Keyboard updated",
        "⌨️ 键盘已更新"
    ));
}

#[test]
fn zh_queue_placeholders_reject_mismatch_parity() {
    assert!(!is_zh_queue_placeholder_ok(
        "⚠️ Queue is full ({max}). Remove a message or wait for the current task to finish.",
        "⚠️ 队列已满。请删除一条消息。"
    ));
    assert!(!is_zh_queue_placeholder_ok(
        "This message is no longer in the queue.",
        "该消息已不在队列中。 {extra}"
    ));
}

// ---- formatting (2) ----

#[test]
fn zh_queue_added_line_parity() {
    assert_eq!(
        format_zh_queue_added(1, 5),
        "📥 已加入队列（1/5）。当前任务完成后将自动发送。"
    );
}

#[test]
fn zh_queue_full_line_parity() {
    assert_eq!(
        format_zh_queue_full(5),
        "⚠️ 队列已满（5）。请删除一条消息或等待当前任务完成。"
    );
}
