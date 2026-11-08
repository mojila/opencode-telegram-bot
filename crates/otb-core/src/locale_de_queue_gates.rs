//! German queue-table gates (slice 124, RED).
//!
//! Rust port of the pure remainder in `src/i18n/de.ts` for the queue
//! command surface (`QUEUE_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `queue.added` (`📥 Zur Warteschlange hinzugefügt ...`)
//! - `queue.full` (`⚠️ Die Warteschlange ist voll ...`)
//! - `queue.media_limit` (`⚠️ Medien in der Warteschlange ...`)
//! - `queue.removed` (`🗑 Nachricht aus der Warteschlange entfernt.`)
//! - `queue.not_found` (`Diese Nachricht ist nicht mehr ...`)
//! - `queue.steer_added` (untranslated fallback, same as `en`)
//! - `queue.disabled_hint` (`Die Nachrichtenwarteschlange ...`)
//! - `keyboard.updated` (`⌨️ Tastatur aktualisiert`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/de.ts` lines 471-480 carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 466-473 are the placeholder reference (`en`
//!   keeps `Added to queue ...`, `Queue is full ...`, `Queued media ...`,
//!   `Message removed ...`, `Keyboard updated`, ...).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (queue keys
//!   included, `de` falls back to `en` in `t_full`).
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. No Rust owner embeds the `de` queue templates yet.
//! - This slice owns only the `de` queue lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `de.ts` file reads (arrive as explicit `key` args here),
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
//!   (`locale_de_status_gates` owner, slice 123).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice124_locale_de_queue_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locale code for this table (TS: `de` in `SUPPORTED_LOCALES`).
pub const DE_QUEUE_LOCALE: &str = "de";

/// Queue keys covered here (subset of `QUEUE_KEYS` in `crate::i18n_full`,
/// plus `keyboard.updated` which ships with the queue UI in `de.ts`).
pub const DE_QUEUE_KEYS: [&str; 8] = [
    "queue.added",
    "queue.full",
    "queue.media_limit",
    "queue.removed",
    "queue.not_found",
    "queue.steer_added",
    "queue.disabled_hint",
    "keyboard.updated",
];

/// Verbatim `de` template (TS: `src/i18n/de.ts` lines 471-472).
pub const DE_QUEUE_ADDED: &str = "📥 Zur Warteschlange hinzugefügt ({count}/{max}). Die Nachricht wird gesendet, sobald die aktuelle Aufgabe abgeschlossen ist.";
/// Verbatim `de` queue-full template (TS: `src/i18n/de.ts` lines 474-475).
pub const DE_QUEUE_FULL: &str =
    "⚠️ Die Warteschlange ist voll ({max}). Entferne eine Nachricht oder warte, bis die aktuelle Aufgabe abgeschlossen ist.";
/// Verbatim `de` media-limit template (TS: `src/i18n/de.ts` line 473).
pub const DE_QUEUE_MEDIA_LIMIT: &str = "⚠️ Medien in der Warteschlange sind auf {maxSizeMb} MiB begrenzt. Warte, bis ein Eintrag gesendet wurde.";
/// Verbatim `de` queue-removed template (TS: `src/i18n/de.ts` line 476).
pub const DE_QUEUE_REMOVED: &str = "🗑 Nachricht aus der Warteschlange entfernt.";
/// Verbatim `de` queue-not-found template (TS: `src/i18n/de.ts` line 477).
pub const DE_QUEUE_NOT_FOUND: &str = "Diese Nachricht ist nicht mehr in der Warteschlange.";
/// Untranslated fallback: `de` keeps the `en` steer template verbatim
/// (TS: `src/i18n/de.ts` line 478).
pub const DE_QUEUE_STEER_ADDED: &str = "📥 Added to the current task ({count}/{max}).";
/// Verbatim `de` queue-disabled hint (TS: `src/i18n/de.ts` line 479).
pub const DE_QUEUE_DISABLED_HINT: &str = "Die Nachrichtenwarteschlange lässt sich in /settings aktivieren.";
/// Verbatim `de` keyboard-updated notice (TS: `src/i18n/de.ts` line 480).
pub const DE_KEYBOARD_UPDATED: &str = "⌨️ Tastatur aktualisiert";

/// Verbatim `de` queue template for a key, or `None` outside this slice
/// (TS: `de[key]` for the 8 queue keys above).
pub fn lookup_de_queue_template(key: &str) -> Option<&'static str> {
    match key {
        "queue.added" => Some(DE_QUEUE_ADDED),
        "queue.full" => Some(DE_QUEUE_FULL),
        "queue.media_limit" => Some(DE_QUEUE_MEDIA_LIMIT),
        "queue.removed" => Some(DE_QUEUE_REMOVED),
        "queue.not_found" => Some(DE_QUEUE_NOT_FOUND),
        "queue.steer_added" => Some(DE_QUEUE_STEER_ADDED),
        "queue.disabled_hint" => Some(DE_QUEUE_DISABLED_HINT),
        "keyboard.updated" => Some(DE_KEYBOARD_UPDATED),
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

fn interpolate_de(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `de` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_de_queue_placeholder_ok(en_template: &str, de_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(de_template)
}

/// `📥 Zur Warteschlange hinzugefügt (count/max) ...` line with counts filled
/// in (TS: `t("queue.added", { count, max })` call site in
/// `src/bot/handlers/prompt-queue-dispatch.ts`).
pub fn format_de_queue_added(count: u32, max: u32) -> String {
    let count = count.to_string();
    let max = max.to_string();
    interpolate_de(DE_QUEUE_ADDED, &[("count", &count), ("max", &max)])
}

/// `⚠️ Die Warteschlange ist voll (max) ...` line with the limit filled in
/// (TS: `t("queue.full", { max })` call site).
pub fn format_de_queue_full(max: u32) -> String {
    let max = max.to_string();
    interpolate_de(DE_QUEUE_FULL, &[("max", &max)])
}
