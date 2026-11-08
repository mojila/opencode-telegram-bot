//! Indonesian goal-table gates (slice 145, GREEN).
//!
//! Rust port of the pure remainder in `src/i18n/id.ts` for the goal
//! command surface (`GOAL_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `pinned.line.goal` (`◎ Goal: {status} — {objective}`)
//! - `goal.none`, `goal.input.prompt`, `goal.button.details`,
//!   `goal.button.pause`, `goal.button.resume`, `goal.start`,
//!   `goal.too_long`, `goal.status.*`, `goal.confirm.start`,
//!   `goal.confirm.resume`, `goal.notice.paused`,
//!   `goal.notice.complete`, `goal.abort.paused`, `goal.abort.failed`,
//!   `goal.reason.round_limit`, `goal.reason.wait_limit`,
//!   `goal.reason.not_progress`
//!   (all currently English fallback, untranslated in `id.ts` lines 731-773).
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/id.ts` lines 731-732 (`pinned.line.goal` / `goal.none`),
//!   734 (`goal.input.prompt`), 737-739 (`goal.button.details` / `pause` /
//!   `resume`), 741 (`goal.start`), 748 (`goal.too_long`), 749-753
//!   (`goal.status.active` / `paused` / `complete` / `rounds` / `updated`),
//!   756-757 (`goal.confirm.start` / `resume`), 760-761
//!   (`goal.notice.paused` / `complete`), 763-764 (`goal.abort.paused` /
//!   `failed`), 771-773 (`goal.reason.round_limit` / `wait_limit` /
//!   `not_progress`) carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 485-527 are the placeholder reference (`id`
//!   matches `en` verbatim here, no translation yet — same as `pt` in
//!   slice 141, `it` in slice 137, `es` in slice 133 and `fr` in
//!   slice 129).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (goal keys
//!   included, `id` falls back to `en` in `t_full`).
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
//!   owns only the `it` queue lookup. `locale_it_goal_gates` (slice 137)
//!   owns only the 22 `it` goal keys. `locale_it_goal2_gates` (slice 138)
//!   owns only the 26 `it` goal-remainder keys.
//!   `locale_pt_status_gates` (slice 139) owns only the `pt` status
//!   lookup. `locale_pt_queue_gates` (slice 140) owns only the `pt`
//!   queue lookup. `locale_pt_goal_gates` (slice 141) owns only the 22
//!   `pt` goal keys. `locale_pt_goal2_gates` (slice 142) owns only the
//!   26 `pt` goal-remainder keys. `locale_id_status_gates` (slice 143)
//!   owns only the `id` status lookup. `locale_id_queue_gates`
//!   (slice 144) owns only the `id` queue lookup. No Rust owner embeds
//!   the `id` goal templates yet.
//! - This slice owns only the `id` goal lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `id.ts` file reads (arrive as explicit `key` args here),
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
//!   (`locale_it_queue_gates` owner, slice 136)
//! - `it` goal (first 22 keys) lookup + placeholder-equality + line-format
//!   (`locale_it_goal_gates` owner, slice 137)
//! - `it` goal-remainder (26 keys) lookup + placeholder-equality + line-format
//!   (`locale_it_goal2_gates` owner, slice 138)
//! - `pt` status lookup + status line formatting
//!   (`locale_pt_status_gates` owner, slice 139)
//! - `pt` queue lookup + queue line formatting
//!   (`locale_pt_queue_gates` owner, slice 140)
//! - `pt` goal (first 22 keys) lookup + placeholder-equality + line-format
//!   (`locale_pt_goal_gates` owner, slice 141)
//! - `pt` goal-remainder (26 keys) lookup + placeholder-equality + line-format
//!   (`locale_pt_goal2_gates` owner, slice 142)
//! - `id` status lookup + status line formatting
//!   (`locale_id_status_gates` owner, slice 143)
//! - `id` queue lookup + queue line formatting
//!   (`locale_id_queue_gates` owner, slice 144).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice145_locale_id_goal_parity.rs`.

/// Locale code for this table (TS: `id` in `SUPPORTED_LOCALES`).
pub const ID_GOAL_LOCALE: &str = "id";

/// Goal keys covered here (`GOAL_KEYS` in `crate::i18n_full`).
pub const ID_GOAL_KEYS: [&str; 22] = [
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

/// Verbatim `id` templates (TS: `src/i18n/id.ts` lines 731-773).
/// Currently English fallback (untranslated).
pub const ID_PINNED_LINE_GOAL: &str = "◎ Goal: {status} — {objective}";
pub const ID_GOAL_NONE: &str =
    "No goal is set.\n\nUse /goal, then send the objective in the next message.";
pub const ID_GOAL_INPUT_PROMPT: &str =
    "Send the objective for this session in your next text message. Describe the result to achieve.";
pub const ID_GOAL_BUTTON_DETAILS: &str = "◎ {status} · {objective}";
pub const ID_GOAL_BUTTON_PAUSE: &str = "⏸ Pause";
pub const ID_GOAL_BUTTON_RESUME: &str = "▶ Continue";
pub const ID_GOAL_START: &str = "✅ Start";
pub const ID_GOAL_TOO_LONG: &str = "A goal is limited to {limit} characters (got {count}).";
pub const ID_GOAL_STATUS_ACTIVE: &str = "active";
pub const ID_GOAL_STATUS_PAUSED: &str = "paused";
pub const ID_GOAL_STATUS_COMPLETE: &str = "complete";
pub const ID_GOAL_STATUS_ROUNDS: &str = "{rounds}/{maxRounds}";
pub const ID_GOAL_STATUS_UPDATED: &str = "Updated {time}";
pub const ID_GOAL_CONFIRM_START: &str = "Start this goal?\n\n{objective}";
pub const ID_GOAL_CONFIRM_RESUME: &str = "Resume this goal?\n\n{objective}";
pub const ID_GOAL_NOTICE_PAUSED: &str = "◎ Goal {status}\n{reason}";
pub const ID_GOAL_NOTICE_COMPLETE: &str = "◎ Goal {status}\n{reason}";
pub const ID_GOAL_ABORT_PAUSED: &str = "Goal paused: {reason}";
pub const ID_GOAL_ABORT_FAILED: &str = "Goal pause failed: {reason}";
pub const ID_GOAL_REASON_ROUND_LIMIT: &str = "Reached the limit of {rounds} rounds";
pub const ID_GOAL_REASON_WAIT_LIMIT: &str =
    "Nothing ran for {minutes} minutes while the goal waited on background work";
pub const ID_GOAL_REASON_NOT_PROGRESS: &str = "Not making progress: {reason}";

/// Verbatim `id` goal template for a key, or `None` outside this slice
/// (TS: `id[key]` for the 22 goal keys above).
pub fn lookup_id_goal_template(key: &str) -> Option<&'static str> {
    match key {
        "pinned.line.goal" => Some(ID_PINNED_LINE_GOAL),
        "goal.none" => Some(ID_GOAL_NONE),
        "goal.input.prompt" => Some(ID_GOAL_INPUT_PROMPT),
        "goal.button.details" => Some(ID_GOAL_BUTTON_DETAILS),
        "goal.button.pause" => Some(ID_GOAL_BUTTON_PAUSE),
        "goal.button.resume" => Some(ID_GOAL_BUTTON_RESUME),
        "goal.start" => Some(ID_GOAL_START),
        "goal.too_long" => Some(ID_GOAL_TOO_LONG),
        "goal.status.active" => Some(ID_GOAL_STATUS_ACTIVE),
        "goal.status.paused" => Some(ID_GOAL_STATUS_PAUSED),
        "goal.status.complete" => Some(ID_GOAL_STATUS_COMPLETE),
        "goal.status.rounds" => Some(ID_GOAL_STATUS_ROUNDS),
        "goal.status.updated" => Some(ID_GOAL_STATUS_UPDATED),
        "goal.confirm.start" => Some(ID_GOAL_CONFIRM_START),
        "goal.confirm.resume" => Some(ID_GOAL_CONFIRM_RESUME),
        "goal.notice.paused" => Some(ID_GOAL_NOTICE_PAUSED),
        "goal.notice.complete" => Some(ID_GOAL_NOTICE_COMPLETE),
        "goal.abort.paused" => Some(ID_GOAL_ABORT_PAUSED),
        "goal.abort.failed" => Some(ID_GOAL_ABORT_FAILED),
        "goal.reason.round_limit" => Some(ID_GOAL_REASON_ROUND_LIMIT),
        "goal.reason.wait_limit" => Some(ID_GOAL_REASON_WAIT_LIMIT),
        "goal.reason.not_progress" => Some(ID_GOAL_REASON_NOT_PROGRESS),
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

fn interpolate_id(template: &str, params: &[(&str, &str)]) -> String {
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

/// `true` when the `id` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_id_goal_placeholder_ok(en_template: &str, id_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(id_template)
}

/// `A goal is limited to {limit} characters (got {count}).` with counts
/// filled in (TS: `t("goal.too_long", { limit, count })` call site).
pub fn format_id_goal_too_long(limit: usize, count: usize) -> String {
    let limit = limit.to_string();
    let count = count.to_string();
    interpolate_id(ID_GOAL_TOO_LONG, &[("limit", &limit), ("count", &count)])
}

/// `Reached the limit of {rounds} rounds` with the round count filled in
/// (TS: `t("goal.reason.round_limit", { rounds })` call site).
pub fn format_id_goal_round_limit(rounds: u32) -> String {
    let rounds = rounds.to_string();
    interpolate_id(ID_GOAL_REASON_ROUND_LIMIT, &[("rounds", &rounds)])
}
