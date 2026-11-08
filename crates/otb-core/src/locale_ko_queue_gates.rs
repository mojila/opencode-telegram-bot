//! Korean queue-table gates (slice 148, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/ko.ts` for the queue
//! command surface (`QUEUE_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `queue.added` (`📥 대기열에 추가되었습니다 ...`)
//! - `queue.full` (`⚠️ 대기열이 가득 찼습니다 ...`)
//! - `queue.media_limit` (`⚠️ 대기열 미디어는 ...`)
//! - `queue.removed` (`🗑 대기열에서 메시지를 삭제했습니다.`)
//! - `queue.not_found` (`이 메시지는 더 이상 대기열에 없습니다.`)
//! - `queue.steer_added` (untranslated fallback, same as `en`)
//! - `queue.disabled_hint` (`메시지 대기열은 /settings에서 ...`)
//! - `keyboard.updated` (`⌨️ 키보드가 업데이트되었습니다`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/ko.ts` lines 461-468 carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 466-473 are the placeholder reference (`en`
//!   keeps `Added to queue ...`, `Queue is full ...`, `Queued media ...`,
//!   `Message removed ...`, `Keyboard updated`, ...).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (queue keys
//!   included, `ko` falls back to `en` in `t_full`).
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. `locale_de_queue_gates` (slice 124) owns only the `de`
//!   queue lookup. `locale_fr_status_gates` (slice 127) owns only the `fr`
//!   status lookup. `locale_fr_queue_gates` (slice 128) owns only the `fr`
//!   queue lookup. `locale_es_status_gates` (slice 131) owns only the `es`
//!   status lookup. `locale_es_queue_gates` (slice 132) owns only the `es`
//!   queue lookup. `locale_it_status_gates` (slice 135) owns only the `it`
//!   status lookup. `locale_it_queue_gates` (slice 136) owns only the `it`
//!   queue lookup. `locale_pt_status_gates` (slice 139) owns only the `pt`
//!   status lookup. `locale_pt_queue_gates` (slice 140) owns only the `pt`
//!   queue lookup. `locale_id_status_gates` (slice 143) owns only the `id`
//!   status lookup. `locale_id_queue_gates` (slice 144) owns only the `id`
//!   queue lookup. `locale_ko_status_gates` (slice 147) owns only the `ko`
//!   status lookup. No Rust owner embeds the `ko` queue templates yet.
//! - This slice owns only the `ko` queue lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `ko.ts` file reads (arrive as explicit `key` args here),
//!   per-locale test looping, `t()` fallback wiring.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback (`crate::i18n` owner)
//! - `FULL_KEYS` / `QUEUE_KEYS` category lists + en/ru verbatim tables +
//!   interpolation (`crate::i18n_full` owner)
//! - placeholder set-equality / missing / extra helpers
//!   (`locale_placeholder_gates` owner, slice 122)
//! - registry labels / `getLocaleOptions` / `getLocale` precedence
//!   (`locale_registry_gates` owner, slice 121)
//! - `de` status lookup + status line formatting
//!   (`locale_de_status_gates` owner, slice 123)
//! - `de` queue lookup + queue line formatting
//!   (`locale_de_queue_gates` owner, slice 124)
//! - `fr` status lookup + status line formatting
//!   (`locale_fr_status_gates` owner, slice 127)
//! - `fr` queue lookup + queue line formatting
//!   (`locale_fr_queue_gates` owner, slice 128)
//! - `es` status lookup + status line formatting
//!   (`locale_es_status_gates` owner, slice 131)
//! - `es` queue lookup + queue line formatting
//!   (`locale_es_queue_gates` owner, slice 132)
//! - `it` status lookup + status line formatting
//!   (`locale_it_status_gates` owner, slice 135)
//! - `it` queue lookup + queue line formatting
//!   (`locale_it_queue_gates` owner, slice 136)
//! - `pt` status lookup + status line formatting
//!   (`locale_pt_status_gates` owner, slice 139)
//! - `pt` queue lookup + queue line formatting
//!   (`locale_pt_queue_gates` owner, slice 140)
//! - `id` status lookup + status line formatting
//!   (`locale_id_status_gates` owner, slice 143)
//! - `id` queue lookup + queue line formatting
//!   (`locale_id_queue_gates` owner, slice 144)
//! - `ko` status lookup + status line formatting
//!   (`locale_ko_status_gates` owner, slice 147).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice148_locale_ko_queue_parity.rs`.

/// Locale code for this table (TS: `ko` in `SUPPORTED_LOCALES`).
pub const KO_QUEUE_LOCALE: &str = "ko";

/// Queue keys covered here (subset of `QUEUE_KEYS` in `crate::i18n_full`,
/// plus `keyboard.updated` which ships with the queue UI in `ko.ts`).
pub const KO_QUEUE_KEYS: [&str; 8] = [
    "queue.added",
    "queue.full",
    "queue.media_limit",
    "queue.removed",
    "queue.not_found",
    "queue.steer_added",
    "queue.disabled_hint",
    "keyboard.updated",
];

/// Verbatim `ko` template (TS: `src/i18n/ko.ts` line 461).
pub const KO_QUEUE_ADDED: &str =
    "📥 대기열에 추가되었습니다 ({count}/{max}). 현재 작업이 끝나면 전송됩니다.";
/// Verbatim `ko` queue-full template (TS: `src/i18n/ko.ts` line 462).
pub const KO_QUEUE_FULL: &str =
    "⚠️ 대기열이 가득 찼습니다 ({max}). 메시지를 삭제하거나 현재 작업이 끝날 때까지 기다려 주세요.";
/// Verbatim `ko` media-limit template (TS: `src/i18n/ko.ts` line 463).
pub const KO_QUEUE_MEDIA_LIMIT: &str =
    "⚠️ 대기열 미디어는 총 {maxSizeMb} MiB로 제한됩니다. 항목이 전송된 후 다시 시도하세요.";
/// Verbatim `ko` queue-removed template (TS: `src/i18n/ko.ts` line 464).
pub const KO_QUEUE_REMOVED: &str = "🗑 대기열에서 메시지를 삭제했습니다.";
/// Verbatim `ko` queue-not-found template (TS: `src/i18n/ko.ts` line 465).
pub const KO_QUEUE_NOT_FOUND: &str = "이 메시지는 더 이상 대기열에 없습니다.";
/// Untranslated fallback: `ko` keeps the `en` steer template verbatim
/// (TS: `src/i18n/ko.ts` line 466).
pub const KO_QUEUE_STEER_ADDED: &str = "📥 Added to the current task ({count}/{max}).";
/// Verbatim `ko` queue-disabled hint (TS: `src/i18n/ko.ts` line 467).
pub const KO_QUEUE_DISABLED_HINT: &str = "메시지 대기열은 /settings에서 활성화할 수 있습니다.";
/// Verbatim `ko` keyboard-updated notice (TS: `src/i18n/ko.ts` line 468).
pub const KO_KEYBOARD_UPDATED: &str = "⌨️ 키보드가 업데이트되었습니다";

/// Verbatim `ko` queue template for a key, or `None` outside this slice
/// (TS: `ko[key]` for the 8 queue keys above).
pub fn lookup_ko_queue_template(key: &str) -> Option<&'static str> {
    match key {
        "queue.added" => Some(KO_QUEUE_ADDED),
        "queue.full" => Some(KO_QUEUE_FULL),
        "queue.media_limit" => Some(KO_QUEUE_MEDIA_LIMIT),
        "queue.removed" => Some(KO_QUEUE_REMOVED),
        "queue.not_found" => Some(KO_QUEUE_NOT_FOUND),
        "queue.steer_added" => Some(KO_QUEUE_STEER_ADDED),
        "queue.disabled_hint" => Some(KO_QUEUE_DISABLED_HINT),
        "keyboard.updated" => Some(KO_KEYBOARD_UPDATED),
        _ => None,
    }
}

fn is_placeholder_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn placeholder_set(template: &str) -> std::collections::BTreeSet<String> {
    let bytes = template.as_bytes();
    let mut out = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && is_placeholder_char(bytes[j]) {
                j += 1;
            }
            if j > i + 1 && j < bytes.len() && bytes[j] == b'}' {
                out.insert(template[i..=j].to_string());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn interpolate_ko(template: &str, params: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < template.len() {
        let rest = &template[i..];
        if rest.starts_with('{') {
            if let Some(end) = rest.find('}') {
                let name = &rest[1..end];
                if !name.is_empty()
                    && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    match params.iter().find(|(k, _)| *k == name) {
                        Some((_, value)) => out.push_str(value),
                        None => out.push_str(&rest[..=end]),
                    }
                    i += end + 1;
                    continue;
                }
            }
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// `true` when the `ko` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_ko_queue_placeholder_ok(en_template: &str, ko_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(ko_template)
}

/// `📥 대기열에 추가되었습니다 (count/max) ...` line with counts filled
/// in (TS: `t("queue.added", { count, max })` call site in
/// `src/bot/handlers/prompt-queue-dispatch.ts`).
pub fn format_ko_queue_added(count: u32, max: u32) -> String {
    let count = count.to_string();
    let max = max.to_string();
    interpolate_ko(KO_QUEUE_ADDED, &[("count", &count), ("max", &max)])
}

/// `⚠️ 대기열이 가득 찼습니다 (max) ...` line with the limit filled in
/// (TS: `t("queue.full", { max })` call site).
pub fn format_ko_queue_full(max: u32) -> String {
    let max = max.to_string();
    interpolate_ko(KO_QUEUE_FULL, &[("max", &max)])
}
