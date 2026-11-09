//! Italian goal-table gates (slice 137, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/it.ts` for the goal
//! command surface (`GOAL_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `pinned.line.goal` (`◎ Goal: {status} — {objective}`)
//! - `goal.none`, `goal.input.prompt`, `goal.button.details`,
//!   `goal.button.pause`, `goal.button.resume`, `goal.start`,
//!   `goal.too_long`, `goal.status.*`, `goal.confirm.start`,
//!   `goal.confirm.resume`, `goal.notice.paused`,
//!   `goal.notice.complete`, `goal.abort.paused`, `goal.abort.failed`,
//!   `goal.reason.round_limit`, `goal.reason.wait_limit`,
//!   `goal.reason.not_progress`
//!   (all currently English fallback, untranslated in `it.ts` lines 743-785).
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/it.ts` lines 743-744 (`pinned.line.goal` / `goal.none`),
//!   746 (`goal.input.prompt`), 749-751 (`goal.button.details` / `pause` /
//!   `resume`), 753 (`goal.start`), 760 (`goal.too_long`), 761-765
//!   (`goal.status.active` / `paused` / `complete` / `rounds` / `updated`),
//!   768-769 (`goal.confirm.start` / `resume`), 772-773
//!   (`goal.notice.paused` / `complete`), 775-776 (`goal.abort.paused` /
//!   `failed`), 783-785 (`goal.reason.round_limit` / `wait_limit` /
//!   `not_progress`) carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 485-527 are the placeholder reference (`it`
//!   matches `en` verbatim here, no translation yet — same as `es` in
//!   slice 133 and `fr` in slice 129).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (goal keys
//!   included, `it` falls back to `en` in `t_full`).
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. `locale_de_queue_gates` (slice 124) owns only the `de`
//!   queue lookup. `locale_de_goal_gates` (slice 125) owns only the first
//!   22 `de` goal keys. `locale_de_goal2_gates` (slice 126) owns only the
//!   26 `de` goal-remainder keys. `locale_fr_status_gates` (slice 127)
//!   owns only the `fr` status lookup. `locale_fr_queue_gates` (slice 128)
//!   owns only the `fr` queue lookup. `locale_fr_goal_gates` (slice 129)
//!   owns only the 22 `fr` goal keys. `locale_fr_goal2_gates` (slice 130)
//!   owns only the 26 `fr` goal-remainder keys.
//!   `locale_es_status_gates` (slice 131) owns only the `es` status
//!   lookup. `locale_es_queue_gates` (slice 132) owns only the `es`
//!   queue lookup. `locale_es_goal_gates` (slice 133) owns only the 22
//!   `es` goal keys. `locale_es_goal2_gates` (slice 134) owns only the
//!   26 `es` goal-remainder keys. `locale_it_status_gates` (slice 135)
//!   owns only the `it` status lookup. `locale_it_queue_gates` (slice 136)
//!   owns only the `it` queue lookup. No Rust owner embeds the `it` goal
//!   templates yet.
//! - This slice owns only the `it` goal lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `it.ts` file reads (arrive as explicit `key` args here),
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
//!   (`locale_de_queue_gates` owner, slice 124)
//! - `de` goal (first 22 keys) lookup + placeholder-equality + line-format
//!   (`locale_de_goal_gates` owner, slice 125)
//! - `de` goal-remainder (26 keys) lookup + placeholder-equality + line-format
//!   (`locale_de_goal2_gates` owner, slice 126)
//! - `fr` status lookup + status line formatting
//!   (`locale_fr_status_gates` owner, slice 127)
//! - `fr` queue lookup + queue line formatting
//!   (`locale_fr_queue_gates` owner, slice 128)
//! - `fr` goal (first 22 keys) lookup + placeholder-equality + line-format
//!   (`locale_fr_goal_gates` owner, slice 129)
//! - `fr` goal-remainder (26 keys) lookup + placeholder-equality + line-format
//!   (`locale_fr_goal2_gates` owner, slice 130)
//! - `es` status lookup + status line formatting
//!   (`locale_es_status_gates` owner, slice 131)
//! - `es` queue lookup + queue line formatting
//!   (`locale_es_queue_gates` owner, slice 132)
//! - `es` goal (first 22 keys) lookup + placeholder-equality + line-format
//!   (`locale_es_goal_gates` owner, slice 133)
//! - `es` goal-remainder (26 keys) lookup + placeholder-equality + line-format
//!   (`locale_es_goal2_gates` owner, slice 134)
//! - `it` status lookup + status line formatting
//!   (`locale_it_status_gates` owner, slice 135)
//! - `it` queue lookup + queue line formatting
//!   (`locale_it_queue_gates` owner, slice 136).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice137_locale_it_goal_parity.rs`.

/// Locale code for this table (TS: `it` in `SUPPORTED_LOCALES`).
pub const IT_GOAL_LOCALE: &str = "it";

/// Goal keys covered here (`GOAL_KEYS` in `crate::i18n_full`).
pub const IT_GOAL_KEYS: [&str; 22] = [
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

/// Verbatim `it` templates (TS: `src/i18n/it.ts` lines 743-785).
/// Currently English fallback (untranslated).
pub const IT_PINNED_LINE_GOAL: &str = "◎ Goal: {status} — {objective}";
pub const IT_GOAL_NONE: &str =
    "No goal is set.\n\nUse /goal, then send the objective in the next message.";
pub const IT_GOAL_INPUT_PROMPT: &str =
    "Send the objective for this session in your next text message. Describe the result to achieve.";
pub const IT_GOAL_BUTTON_DETAILS: &str = "◎ {status} · {objective}";
pub const IT_GOAL_BUTTON_PAUSE: &str = "⏸ Pause";
pub const IT_GOAL_BUTTON_RESUME: &str = "▶ Continue";
pub const IT_GOAL_START: &str = "✅ Start";
pub const IT_GOAL_TOO_LONG: &str = "A goal is limited to {limit} characters (got {count}).";
pub const IT_GOAL_STATUS_ACTIVE: &str = "active";
pub const IT_GOAL_STATUS_PAUSED: &str = "paused";
pub const IT_GOAL_STATUS_COMPLETE: &str = "complete";
pub const IT_GOAL_STATUS_ROUNDS: &str = "{rounds}/{maxRounds}";
pub const IT_GOAL_STATUS_UPDATED: &str = "Updated {time}";
pub const IT_GOAL_CONFIRM_START: &str = "Start this goal?\n\n{objective}";
pub const IT_GOAL_CONFIRM_RESUME: &str = "Resume this goal?\n\n{objective}";
pub const IT_GOAL_NOTICE_PAUSED: &str = "◎ Goal {status}\n{reason}";
pub const IT_GOAL_NOTICE_COMPLETE: &str = "◎ Goal {status}\n{reason}";
pub const IT_GOAL_ABORT_PAUSED: &str = "Goal paused: {reason}";
pub const IT_GOAL_ABORT_FAILED: &str = "Goal pause failed: {reason}";
pub const IT_GOAL_REASON_ROUND_LIMIT: &str = "Reached the limit of {rounds} rounds";
pub const IT_GOAL_REASON_WAIT_LIMIT: &str =
    "Nothing ran for {minutes} minutes while the goal waited on background work";
pub const IT_GOAL_REASON_NOT_PROGRESS: &str = "Not making progress: {reason}";

/// Verbatim `it` goal template for a key, or `None` outside this slice
/// (TS: `it[key]` for the 22 goal keys above).
pub fn lookup_it_goal_template(key: &str) -> Option<&'static str> {
    match key {
        "pinned.line.goal" => Some(IT_PINNED_LINE_GOAL),
        "goal.none" => Some(IT_GOAL_NONE),
        "goal.input.prompt" => Some(IT_GOAL_INPUT_PROMPT),
        "goal.button.details" => Some(IT_GOAL_BUTTON_DETAILS),
        "goal.button.pause" => Some(IT_GOAL_BUTTON_PAUSE),
        "goal.button.resume" => Some(IT_GOAL_BUTTON_RESUME),
        "goal.start" => Some(IT_GOAL_START),
        "goal.too_long" => Some(IT_GOAL_TOO_LONG),
        "goal.status.active" => Some(IT_GOAL_STATUS_ACTIVE),
        "goal.status.paused" => Some(IT_GOAL_STATUS_PAUSED),
        "goal.status.complete" => Some(IT_GOAL_STATUS_COMPLETE),
        "goal.status.rounds" => Some(IT_GOAL_STATUS_ROUNDS),
        "goal.status.updated" => Some(IT_GOAL_STATUS_UPDATED),
        "goal.confirm.start" => Some(IT_GOAL_CONFIRM_START),
        "goal.confirm.resume" => Some(IT_GOAL_CONFIRM_RESUME),
        "goal.notice.paused" => Some(IT_GOAL_NOTICE_PAUSED),
        "goal.notice.complete" => Some(IT_GOAL_NOTICE_COMPLETE),
        "goal.abort.paused" => Some(IT_GOAL_ABORT_PAUSED),
        "goal.abort.failed" => Some(IT_GOAL_ABORT_FAILED),
        "goal.reason.round_limit" => Some(IT_GOAL_REASON_ROUND_LIMIT),
        "goal.reason.wait_limit" => Some(IT_GOAL_REASON_WAIT_LIMIT),
        "goal.reason.not_progress" => Some(IT_GOAL_REASON_NOT_PROGRESS),
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

fn interpolate_it(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `it` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_it_goal_placeholder_ok(en_template: &str, it_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(it_template)
}

/// `A goal is limited to {limit} characters (got {count}).` with counts
/// filled in (TS: `t("goal.too_long", { limit, count })` call site).
pub fn format_it_goal_too_long(limit: usize, count: usize) -> String {
    let limit = limit.to_string();
    let count = count.to_string();
    interpolate_it(IT_GOAL_TOO_LONG, &[("limit", &limit), ("count", &count)])
}

/// `Reached the limit of {rounds} rounds` with the round count filled in
/// (TS: `t("goal.reason.round_limit", { rounds })` call site).
pub fn format_it_goal_round_limit(rounds: u32) -> String {
    let rounds = rounds.to_string();
    interpolate_it(IT_GOAL_REASON_ROUND_LIMIT, &[("rounds", &rounds)])
}
