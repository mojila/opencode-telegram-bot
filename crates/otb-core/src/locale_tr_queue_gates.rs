//! Turkish queue-table gates (slice 152, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/tr.ts` for the queue
//! command surface (`QUEUE_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `queue.added` (`📥 Kuyruğa eklendi ...`)
//! - `queue.full` (`⚠️ Kuyruk dolu ...`)
//! - `queue.media_limit` (`⚠️ Kuyruktaki medya ...`)
//! - `queue.removed` (`🗑 Mesaj kuyruktan kaldırıldı.`)
//! - `queue.not_found` (`Bu mesaj artık kuyrukta değil.`)
//! - `queue.steer_added` (untranslated fallback, same as `en`)
//! - `queue.disabled_hint` (`Mesaj kuyruğu /settings ...`)
//! - `keyboard.updated` (`⌨️ Klavye güncellendi`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/tr.ts` lines 452-461 carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 466-473 are the placeholder reference (`en`
//!   keeps `Added to queue ...`, `Queue is full ...`, `Queued media ...`,
//!   `Message removed ...`, `Keyboard updated`, ...).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (queue keys
//!   included, `tr` falls back to `en` in `t_full`).
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
//!   status lookup. `locale_ko_queue_gates` (slice 148) owns only the `ko`
//!   queue lookup. `locale_tr_status_gates` (slice 151) owns only the `tr`
//!   status lookup. No Rust owner embeds the `tr` queue templates yet.
//! - This slice owns only the `tr` queue lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `tr.ts` file reads (arrive as explicit `key` args here),
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
//!   (`locale_ko_status_gates` owner, slice 147)
//! - `ko` queue lookup + queue line formatting
//!   (`locale_ko_queue_gates` owner, slice 148)
//! - `tr` status lookup + status line formatting
//!   (`locale_tr_status_gates` owner, slice 151).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice152_locale_tr_queue_parity.rs`.

/// Locale code for this table (TS: `tr` in `SUPPORTED_LOCALES`).
pub const TR_QUEUE_LOCALE: &str = "tr";

/// Queue keys covered here (subset of `QUEUE_KEYS` in `crate::i18n_full`,
/// plus `keyboard.updated` which ships with the queue UI in `tr.ts`).
pub const TR_QUEUE_KEYS: [&str; 8] = [
    "queue.added",
    "queue.full",
    "queue.media_limit",
    "queue.removed",
    "queue.not_found",
    "queue.steer_added",
    "queue.disabled_hint",
    "keyboard.updated",
];

/// Verbatim `tr` template (TS: `src/i18n/tr.ts` line 452).
pub const TR_QUEUE_ADDED: &str =
    "📥 Kuyruğa eklendi ({count}/{max}). Geçerli görev bittiğinde gönderilecek.";
/// Verbatim `tr` queue-full template (TS: `src/i18n/tr.ts` line 453).
pub const TR_QUEUE_FULL: &str =
    "⚠️ Kuyruk dolu ({max}). Bir mesajı kaldırın veya geçerli görevin bitmesini bekleyin.";
/// Verbatim `tr` media-limit template (TS: `src/i18n/tr.ts` line 455).
pub const TR_QUEUE_MEDIA_LIMIT: &str =
    "⚠️ Kuyruktaki medya {maxSizeMb} MiB ile sınırlıdır. Bir öğenin gönderilmesini bekleyip tekrar deneyin.";
/// Verbatim `tr` queue-removed template (TS: `src/i18n/tr.ts` line 457).
pub const TR_QUEUE_REMOVED: &str = "🗑 Mesaj kuyruktan kaldırıldı.";
/// Verbatim `tr` queue-not-found template (TS: `src/i18n/tr.ts` line 458).
pub const TR_QUEUE_NOT_FOUND: &str = "Bu mesaj artık kuyrukta değil.";
/// Untranslated fallback: `tr` keeps the `en` steer template verbatim
/// (TS: `src/i18n/tr.ts` line 459).
pub const TR_QUEUE_STEER_ADDED: &str = "📥 Added to the current task ({count}/{max}).";
/// Verbatim `tr` queue-disabled hint (TS: `src/i18n/tr.ts` line 460).
pub const TR_QUEUE_DISABLED_HINT: &str = "Mesaj kuyruğu /settings içinden açılabilir.";
/// Verbatim `tr` keyboard-updated notice (TS: `src/i18n/tr.ts` line 461).
pub const TR_KEYBOARD_UPDATED: &str = "⌨️ Klavye güncellendi";

/// Verbatim `tr` queue template for a key, or `None` outside this slice
/// (TS: `tr[key]` for the 8 queue keys above).
pub fn lookup_tr_queue_template(key: &str) -> Option<&'static str> {
    match key {
        "queue.added" => Some(TR_QUEUE_ADDED),
        "queue.full" => Some(TR_QUEUE_FULL),
        "queue.media_limit" => Some(TR_QUEUE_MEDIA_LIMIT),
        "queue.removed" => Some(TR_QUEUE_REMOVED),
        "queue.not_found" => Some(TR_QUEUE_NOT_FOUND),
        "queue.steer_added" => Some(TR_QUEUE_STEER_ADDED),
        "queue.disabled_hint" => Some(TR_QUEUE_DISABLED_HINT),
        "keyboard.updated" => Some(TR_KEYBOARD_UPDATED),
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

fn interpolate_tr(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `tr` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_tr_queue_placeholder_ok(en_template: &str, tr_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(tr_template)
}

/// `📥 Kuyruğa eklendi (count/max) ...` line with counts filled
/// in (TS: `t("queue.added", { count, max })` call site in
/// `src/bot/handlers/prompt-queue-dispatch.ts`).
pub fn format_tr_queue_added(count: u32, max: u32) -> String {
    let count = count.to_string();
    let max = max.to_string();
    interpolate_tr(TR_QUEUE_ADDED, &[("count", &count), ("max", &max)])
}

/// `⚠️ Kuyruk dolu (max) ...` line with the limit filled in
/// (TS: `t("queue.full", { max })` call site).
pub fn format_tr_queue_full(max: u32) -> String {
    let max = max.to_string();
    interpolate_tr(TR_QUEUE_FULL, &[("max", &max)])
}
