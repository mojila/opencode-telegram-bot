//! German goal-table gates (slice 125, RED).
//!
//! Rust port of the pure remainder in `src/i18n/de.ts` for the goal
//! command surface (`GOAL_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `pinned.line.goal` (`◎ Goal: {status} — {objective}`)
//! - `goal.none`, `goal.input.prompt`, `goal.button.details`,
//!   `goal.button.pause`, `goal.button.resume`, `goal.start`,
//!   `goal.too_long`, `goal.status.*`, `goal.confirm.start`,
//!   `goal.confirm.resume`, `goal.notice.paused`,
//!   `goal.notice.complete`, `goal.abort.paused`, `goal.abort.failed`,
//!   `goal.reason.round_limit`, `goal.reason.wait_limit`,
//!   `goal.reason.not_progress`
//!   (all currently English fallback, untranslated in `de.ts` lines 758-805).
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/de.ts` lines 758-805 carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 485-527 are the placeholder reference (`en`
//!   keeps `◎ Goal: {status} — {objective}`, `A goal is limited to ...`,
//!   `Reached the limit of {rounds} rounds`, ...). `de` matches `en`
//!   verbatim here (no translation yet).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (goal keys
//!   included, `de` falls back to `en` in `t_full`).
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. `locale_de_queue_gates` (slice 124) owns only the `de`
//!   queue lookup. No Rust owner embeds the `de` goal templates yet.
//! - This slice owns only the `de` goal lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `de.ts` file reads (arrive as explicit `key` args here),
//!   per-locale test looping, `t()` fallback wiring.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback (`crate::i18n` owner)
//! - `FULL_KEYS` / `GOAL_KEYS` category lists + en/ru verbatim tables +
//!   interpolation (`crate::i18n_full` owner)
//! - placeholder set-equality / missing / extra helpers
//!   (`locale_placeholder_gates` owner, slice 122)
//! - registry labels / `getLocaleOptions` / `getLocale` precedence
//!   (`locale_registry_gates` owner, slice 121)
//! - `de` status lookup + status line formatting
//!   (`locale_de_status_gates` owner, slice 123)
//! - `de` queue lookup + queue line formatting
//!   (`locale_de_queue_gates` owner, slice 124).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice125_locale_de_goal_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locale code for this table (TS: `de` in `SUPPORTED_LOCALES`).
pub const DE_GOAL_LOCALE: &str = "de";

/// Goal keys covered here (`GOAL_KEYS` in `crate::i18n_full`).
pub const DE_GOAL_KEYS: [&str; 22] = [
    "pinned.line.goal",
    "goal.none",
    "goal.input.prompt",
    "goal.button.details",
    "goal.button.pause",
    "goal.button.resume",
    "goal.start",
    "goal.too_long",
    "goal.status.active",
    "goal.status.paused",
    "goal.status.complete",
    "goal.status.rounds",
    "goal.status.updated",
    "goal.confirm.start",
    "goal.confirm.resume",
    "goal.notice.paused",
    "goal.notice.complete",
    "goal.abort.paused",
    "goal.abort.failed",
    "goal.reason.round_limit",
    "goal.reason.wait_limit",
    "goal.reason.not_progress",
];

/// Verbatim `de` templates (TS: `src/i18n/de.ts` lines 758-800).
/// Currently English fallback (untranslated).
pub const DE_PINNED_LINE_GOAL: &str = "◎ Goal: {status} — {objective}";
pub const DE_GOAL_NONE: &str =
    "No goal is set.\n\nUse /goal, then send the objective in the next message.";
pub const DE_GOAL_INPUT_PROMPT: &str =
    "Send the objective for this session in your next text message. Describe the result to achieve.";
pub const DE_GOAL_BUTTON_DETAILS: &str = "◎ {status} · {objective}";
pub const DE_GOAL_BUTTON_PAUSE: &str = "⏸ Pause";
pub const DE_GOAL_BUTTON_RESUME: &str = "▶ Continue";
pub const DE_GOAL_START: &str = "✅ Start";
pub const DE_GOAL_TOO_LONG: &str = "A goal is limited to {limit} characters (got {count}).";
pub const DE_GOAL_STATUS_ACTIVE: &str = "active";
pub const DE_GOAL_STATUS_PAUSED: &str = "paused";
pub const DE_GOAL_STATUS_COMPLETE: &str = "complete";
pub const DE_GOAL_STATUS_ROUNDS: &str = "{rounds}/{maxRounds}";
pub const DE_GOAL_STATUS_UPDATED: &str = "Updated {time}";
pub const DE_GOAL_CONFIRM_START: &str = "Start this goal?\n\n{objective}";
pub const DE_GOAL_CONFIRM_RESUME: &str = "Resume this goal?\n\n{objective}";
pub const DE_GOAL_NOTICE_PAUSED: &str = "◎ Goal {status}\n{reason}";
pub const DE_GOAL_NOTICE_COMPLETE: &str = "◎ Goal {status}\n{reason}";
pub const DE_GOAL_ABORT_PAUSED: &str = "Goal paused: {reason}";
pub const DE_GOAL_ABORT_FAILED: &str = "Goal pause failed: {reason}";
pub const DE_GOAL_REASON_ROUND_LIMIT: &str = "Reached the limit of {rounds} rounds";
pub const DE_GOAL_REASON_WAIT_LIMIT: &str =
    "Nothing ran for {minutes} minutes while the goal waited on background work";
pub const DE_GOAL_REASON_NOT_PROGRESS: &str = "Not making progress: {reason}";

/// Verbatim `de` goal template for a key, or `None` outside this slice
/// (TS: `de[key]` for the 22 goal keys above).
pub fn lookup_de_goal_template(key: &str) -> Option<&'static str> {
    match key {
        "pinned.line.goal" => Some(DE_PINNED_LINE_GOAL),
        "goal.none" => Some(DE_GOAL_NONE),
        "goal.input.prompt" => Some(DE_GOAL_INPUT_PROMPT),
        "goal.button.details" => Some(DE_GOAL_BUTTON_DETAILS),
        "goal.button.pause" => Some(DE_GOAL_BUTTON_PAUSE),
        "goal.button.resume" => Some(DE_GOAL_BUTTON_RESUME),
        "goal.start" => Some(DE_GOAL_START),
        "goal.too_long" => Some(DE_GOAL_TOO_LONG),
        "goal.status.active" => Some(DE_GOAL_STATUS_ACTIVE),
        "goal.status.paused" => Some(DE_GOAL_STATUS_PAUSED),
        "goal.status.complete" => Some(DE_GOAL_STATUS_COMPLETE),
        "goal.status.rounds" => Some(DE_GOAL_STATUS_ROUNDS),
        "goal.status.updated" => Some(DE_GOAL_STATUS_UPDATED),
        "goal.confirm.start" => Some(DE_GOAL_CONFIRM_START),
        "goal.confirm.resume" => Some(DE_GOAL_CONFIRM_RESUME),
        "goal.notice.paused" => Some(DE_GOAL_NOTICE_PAUSED),
        "goal.notice.complete" => Some(DE_GOAL_NOTICE_COMPLETE),
        "goal.abort.paused" => Some(DE_GOAL_ABORT_PAUSED),
        "goal.abort.failed" => Some(DE_GOAL_ABORT_FAILED),
        "goal.reason.round_limit" => Some(DE_GOAL_REASON_ROUND_LIMIT),
        "goal.reason.wait_limit" => Some(DE_GOAL_REASON_WAIT_LIMIT),
        "goal.reason.not_progress" => Some(DE_GOAL_REASON_NOT_PROGRESS),
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
pub fn is_de_goal_placeholder_ok(en_template: &str, de_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(de_template)
}

/// `A goal is limited to {limit} characters (got {count}).` with counts
/// filled in (TS: `t("goal.too_long", { limit, count })` call site).
pub fn format_de_goal_too_long(limit: usize, count: usize) -> String {
    let limit = limit.to_string();
    let count = count.to_string();
    interpolate_de(DE_GOAL_TOO_LONG, &[("limit", &limit), ("count", &count)])
}

/// `Reached the limit of {rounds} rounds` with the round count filled in
/// (TS: `t("goal.reason.round_limit", { rounds })` call site).
pub fn format_de_goal_round_limit(rounds: u32) -> String {
    let rounds = rounds.to_string();
    interpolate_de(DE_GOAL_REASON_ROUND_LIMIT, &[("rounds", &rounds)])
}
