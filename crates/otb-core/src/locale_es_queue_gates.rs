//! Spanish queue-table gates (slice 132, RED).
//!
//! Rust port of the pure remainder in `src/i18n/es.ts` for the queue
//! command surface (`QUEUE_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `queue.added` (`📥 Añadido a la cola ...`)
//! - `queue.full` (`⚠️ La cola está llena ...`)
//! - `queue.media_limit` (`⚠️ Los archivos multimedia en cola ...`)
//! - `queue.removed` (`🗑 Mensaje eliminado de la cola.`)
//! - `queue.not_found` (`Este mensaje ya no está en la cola.`)
//! - `queue.steer_added` (untranslated fallback, same as `en`)
//! - `queue.disabled_hint` (`La cola de mensajes ...`)
//! - `keyboard.updated` (`⌨️ Teclado actualizado`)
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/es.ts` lines 467-476 carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 466-473 are the placeholder reference (`en`
//!   keeps `Added to queue ...`, `Queue is full ...`, `Queued media ...`,
//!   `Message removed ...`, `Keyboard updated`, ...).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (queue keys
//!   included, `es` falls back to `en` in `t_full`).
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. `locale_de_queue_gates` (slice 124) owns only the `de`
//!   queue lookup. `locale_fr_status_gates` (slice 127) owns only the `fr`
//!   status lookup. `locale_fr_queue_gates` (slice 128) owns only the `fr`
//!   queue lookup. `locale_es_status_gates` (slice 131) owns only the `es`
//!   status lookup. No Rust owner embeds the `es` queue templates yet.
//! - This slice owns only the `es` queue lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `es.ts` file reads (arrive as explicit `key` args here),
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
//!   (`locale_es_status_gates` owner, slice 131).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice132_locale_es_queue_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locale code for this table (TS: `es` in `SUPPORTED_LOCALES`).
pub const ES_QUEUE_LOCALE: &str = "es";

/// Queue keys covered here (subset of `QUEUE_KEYS` in `crate::i18n_full`,
/// plus `keyboard.updated` which ships with the queue UI in `es.ts`).
pub const ES_QUEUE_KEYS: [&str; 8] = [
    "queue.added",
    "queue.full",
    "queue.media_limit",
    "queue.removed",
    "queue.not_found",
    "queue.steer_added",
    "queue.disabled_hint",
    "keyboard.updated",
];

/// Verbatim `es` template (TS: `src/i18n/es.ts` lines 467-468).
pub const ES_QUEUE_ADDED: &str = "📥 Añadido a la cola ({count}/{max}). Se enviará cuando termine la tarea actual.";
/// Verbatim `es` queue-full template (TS: `src/i18n/es.ts` lines 470-471).
pub const ES_QUEUE_FULL: &str =
    "⚠️ La cola está llena ({max}). Elimina un mensaje o espera a que termine la tarea actual.";
/// Verbatim `es` media-limit template (TS: `src/i18n/es.ts` line 469).
pub const ES_QUEUE_MEDIA_LIMIT: &str = "⚠️ Los archivos multimedia en cola están limitados a {maxSizeMb} MiB. Espera a que se envíe un elemento.";
/// Verbatim `es` queue-removed template (TS: `src/i18n/es.ts` line 472).
pub const ES_QUEUE_REMOVED: &str = "🗑 Mensaje eliminado de la cola.";
/// Verbatim `es` queue-not-found template (TS: `src/i18n/es.ts` line 473).
pub const ES_QUEUE_NOT_FOUND: &str = "Este mensaje ya no está en la cola.";
/// Untranslated fallback: `es` keeps the `en` steer template verbatim
/// (TS: `src/i18n/es.ts` line 474).
pub const ES_QUEUE_STEER_ADDED: &str = "📥 Added to the current task ({count}/{max}).";
/// Verbatim `es` queue-disabled hint (TS: `src/i18n/es.ts` line 475).
pub const ES_QUEUE_DISABLED_HINT: &str = "La cola de mensajes se activa en /settings.";
/// Verbatim `es` keyboard-updated notice (TS: `src/i18n/es.ts` line 476).
pub const ES_KEYBOARD_UPDATED: &str = "⌨️ Teclado actualizado";

/// Verbatim `es` queue template for a key, or `None` outside this slice
/// (TS: `es[key]` for the 8 queue keys above).
pub fn lookup_es_queue_template(key: &str) -> Option<&'static str> {
    match key {
        "queue.added" => Some(ES_QUEUE_ADDED),
        "queue.full" => Some(ES_QUEUE_FULL),
        "queue.media_limit" => Some(ES_QUEUE_MEDIA_LIMIT),
        "queue.removed" => Some(ES_QUEUE_REMOVED),
        "queue.not_found" => Some(ES_QUEUE_NOT_FOUND),
        "queue.steer_added" => Some(ES_QUEUE_STEER_ADDED),
        "queue.disabled_hint" => Some(ES_QUEUE_DISABLED_HINT),
        "keyboard.updated" => Some(ES_KEYBOARD_UPDATED),
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

fn interpolate_es(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `es` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_es_queue_placeholder_ok(en_template: &str, es_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(es_template)
}

/// `📥 Añadido a la cola (count/max) ...` line with counts filled
/// in (TS: `t("queue.added", { count, max })` call site in
/// `src/bot/handlers/prompt-queue-dispatch.ts`).
pub fn format_es_queue_added(count: u32, max: u32) -> String {
    let count = count.to_string();
    let max = max.to_string();
    interpolate_es(ES_QUEUE_ADDED, &[("count", &count), ("max", &max)])
}

/// `⚠️ La cola está llena (max) ...` line with the limit filled in
/// (TS: `t("queue.full", { max })` call site).
pub fn format_es_queue_full(max: u32) -> String {
    let max = max.to_string();
    interpolate_es(ES_QUEUE_FULL, &[("max", &max)])
}
