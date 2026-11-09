//! French queue-table gates (slice 128, RED).
//!
//! Rust port of the pure remainder in `src/i18n/fr.ts` for the queue
//! command surface (`QUEUE_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `queue.added` (`📥 Ajouté à la file d'attente ...`)
//! - `queue.full` (`⚠️ La file d'attente est pleine ...`)
//! - `queue.media_limit` (`⚠️ Les médias en file sont limités ...`)
//! - `queue.removed` (`🗑 Message retiré de la file d'attente.`)
//! - `queue.not_found` (`Ce message n'est plus ...`)
//! - `queue.steer_added` (untranslated fallback, same as `en`)
//! - `queue.disabled_hint` (`La file d'attente des messages ...`)
//! - `keyboard.updated` (`⌨️ Clavier mis à jour`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/fr.ts` lines 473-482 carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 466-473 are the placeholder reference (`en`
//!   keeps `Added to queue ...`, `Queue is full ...`, `Queued media ...`,
//!   `Message removed ...`, `Keyboard updated`, ...).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (queue keys
//!   included, `fr` falls back to `en` in `t_full`).
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. `locale_de_queue_gates` (slice 124) owns only the `de`
//!   queue lookup. `locale_fr_status_gates` (slice 127) owns only the `fr`
//!   status lookup. No Rust owner embeds the `fr` queue templates yet.
//! - This slice owns only the `fr` queue lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `fr.ts` file reads (arrive as explicit `key` args here),
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
//!   (`locale_fr_status_gates` owner, slice 127).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice128_locale_fr_queue_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locale code for this table (TS: `fr` in `SUPPORTED_LOCALES`).
pub const FR_QUEUE_LOCALE: &str = "fr";

/// Queue keys covered here (subset of `QUEUE_KEYS` in `crate::i18n_full`,
/// plus `keyboard.updated` which ships with the queue UI in `fr.ts`).
pub const FR_QUEUE_KEYS: [&str; 8] = [
    "queue.added",
    "queue.full",
    "queue.media_limit",
    "queue.removed",
    "queue.not_found",
    "queue.steer_added",
    "queue.disabled_hint",
    "keyboard.updated",
];

/// Verbatim `fr` template (TS: `src/i18n/fr.ts` lines 473-474).
pub const FR_QUEUE_ADDED: &str = "📥 Ajouté à la file d'attente ({count}/{max}). Le message sera envoyé à la fin de la tâche en cours.";
/// Verbatim `fr` queue-full template (TS: `src/i18n/fr.ts` lines 476-477).
pub const FR_QUEUE_FULL: &str =
    "⚠️ La file d'attente est pleine ({max}). Supprimez un message ou attendez la fin de la tâche en cours.";
/// Verbatim `fr` media-limit template (TS: `src/i18n/fr.ts` line 475).
pub const FR_QUEUE_MEDIA_LIMIT: &str =
    "⚠️ Les médias en file sont limités à {maxSizeMb} MiB. Attendez l'envoi d'un élément.";
/// Verbatim `fr` queue-removed template (TS: `src/i18n/fr.ts` line 478).
pub const FR_QUEUE_REMOVED: &str = "🗑 Message retiré de la file d'attente.";
/// Verbatim `fr` queue-not-found template (TS: `src/i18n/fr.ts` line 479).
pub const FR_QUEUE_NOT_FOUND: &str = "Ce message n'est plus dans la file d'attente.";
/// Untranslated fallback: `fr` keeps the `en` steer template verbatim
/// (TS: `src/i18n/fr.ts` line 480).
pub const FR_QUEUE_STEER_ADDED: &str = "📥 Added to the current task ({count}/{max}).";
/// Verbatim `fr` queue-disabled hint (TS: `src/i18n/fr.ts` line 481).
pub const FR_QUEUE_DISABLED_HINT: &str = "La file d'attente des messages s'active dans /settings.";
/// Verbatim `fr` keyboard-updated notice (TS: `src/i18n/fr.ts` line 482).
pub const FR_KEYBOARD_UPDATED: &str = "⌨️ Clavier mis à jour";

/// Verbatim `fr` queue template for a key, or `None` outside this slice
/// (TS: `fr[key]` for the 8 queue keys above).
pub fn lookup_fr_queue_template(key: &str) -> Option<&'static str> {
    match key {
        "queue.added" => Some(FR_QUEUE_ADDED),
        "queue.full" => Some(FR_QUEUE_FULL),
        "queue.media_limit" => Some(FR_QUEUE_MEDIA_LIMIT),
        "queue.removed" => Some(FR_QUEUE_REMOVED),
        "queue.not_found" => Some(FR_QUEUE_NOT_FOUND),
        "queue.steer_added" => Some(FR_QUEUE_STEER_ADDED),
        "queue.disabled_hint" => Some(FR_QUEUE_DISABLED_HINT),
        "keyboard.updated" => Some(FR_KEYBOARD_UPDATED),
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

fn interpolate_fr(template: &str, params: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < template.len() {
        let rest = &template[i..];
        if rest.starts_with('{') {
            if let Some(end) = rest.find('}') {
                let name = &rest[1..end];
                if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
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

/// `true` when the `fr` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_fr_queue_placeholder_ok(en_template: &str, fr_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(fr_template)
}

/// `📥 Ajouté à la file d'attente (count/max) ...` line with counts filled
/// in (TS: `t("queue.added", { count, max })` call site in
/// `src/bot/handlers/prompt-queue-dispatch.ts`).
pub fn format_fr_queue_added(count: u32, max: u32) -> String {
    let count = count.to_string();
    let max = max.to_string();
    interpolate_fr(FR_QUEUE_ADDED, &[("count", &count), ("max", &max)])
}

/// `⚠️ La file d'attente est pleine (max) ...` line with the limit filled in
/// (TS: `t("queue.full", { max })` call site).
pub fn format_fr_queue_full(max: u32) -> String {
    let max = max.to_string();
    interpolate_fr(FR_QUEUE_FULL, &[("max", &max)])
}
