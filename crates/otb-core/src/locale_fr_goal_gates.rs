//! French goal-table gates (slice 129, RED).
//!
//! Rust port of the pure remainder in `src/i18n/fr.ts` for the goal
//! command surface (`GOAL_KEYS` in `crates/otb-core/src/i18n_full.rs`):
//! - `pinned.line.goal` (`◎ Goal: {status} — {objective}`)
//! - `goal.none`, `goal.input.prompt`, `goal.button.details`,
//!   `goal.button.pause`, `goal.button.resume`, `goal.start`,
//!   `goal.too_long`, `goal.status.*`, `goal.confirm.start`,
//!   `goal.confirm.resume`, `goal.notice.paused`,
//!   `goal.notice.complete`, `goal.abort.paused`, `goal.abort.failed`,
//!   `goal.reason.round_limit`, `goal.reason.wait_limit`,
//!   `goal.reason.not_progress`
//!   (all currently English fallback, untranslated in `fr.ts` lines 758-800).
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/fr.ts` lines 758-759 (`pinned.line.goal` / `goal.none`),
//!   761 (`goal.input.prompt`), 764-766 (`goal.button.details` / `pause` /
//!   `resume`), 768 (`goal.start`), 775 (`goal.too_long`), 776-780
//!   (`goal.status.active` / `paused` / `complete` / `rounds` / `updated`),
//!   783-784 (`goal.confirm.start` / `resume`), 787-788
//!   (`goal.notice.paused` / `complete`), 790-791 (`goal.abort.paused` /
//!   `failed`), 798-800 (`goal.reason.round_limit` / `wait_limit` /
//!   `not_progress`) carry the templates above verbatim;
//!   `src/i18n/en.ts` lines 485-527 are the placeholder reference (`fr`
//!   matches `en` verbatim here, no translation yet — same as `de` in
//!   slice 125).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only (goal keys
//!   included, `fr` falls back to `en` in `t_full`).
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. `locale_de_queue_gates` (slice 124) owns only the `de`
//!   queue lookup. `locale_de_goal_gates` (slice 125) owns only the first
//!   22 `de` goal keys. `locale_de_goal2_gates` (slice 126) owns only the
//!   26 `de` goal-remainder keys. `locale_fr_status_gates` (slice 127)
//!   owns only the `fr` status lookup. `locale_fr_queue_gates` (slice 128)
//!   owns only the `fr` queue lookup. No Rust owner embeds the `fr` goal
//!   templates yet.
//! - This slice owns only the `fr` goal lookup + placeholder-equality +
//!   line-format gates; full 12-locale x 654-key table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `fr.ts` file reads (arrive as explicit `key` args here),
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
//!   (`locale_fr_queue_gates` owner, slice 128).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice129_locale_fr_goal_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locale code for this table (TS: `fr` in `SUPPORTED_LOCALES`).
pub const FR_GOAL_LOCALE: &str = "fr";

/// Goal keys covered here (`GOAL_KEYS` in `crate::i18n_full`).
pub const FR_GOAL_KEYS: [&str; 22] = [
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

/// Verbatim `fr` templates (TS: `src/i18n/fr.ts` lines 758-800).
/// Currently English fallback (untranslated).
pub const FR_PINNED_LINE_GOAL: &str = "◎ Goal: {status} — {objective}";
pub const FR_GOAL_NONE: &str =
    "No goal is set.\n\nUse /goal, then send the objective in the next message.";
pub const FR_GOAL_INPUT_PROMPT: &str =
    "Send the objective for this session in your next text message. Describe the result to achieve.";
pub const FR_GOAL_BUTTON_DETAILS: &str = "◎ {status} · {objective}";
pub const FR_GOAL_BUTTON_PAUSE: &str = "⏸ Pause";
pub const FR_GOAL_BUTTON_RESUME: &str = "▶ Continue";
pub const FR_GOAL_START: &str = "✅ Start";
pub const FR_GOAL_TOO_LONG: &str = "A goal is limited to {limit} characters (got {count}).";
pub const FR_GOAL_STATUS_ACTIVE: &str = "active";
pub const FR_GOAL_STATUS_PAUSED: &str = "paused";
pub const FR_GOAL_STATUS_COMPLETE: &str = "complete";
pub const FR_GOAL_STATUS_ROUNDS: &str = "{rounds}/{maxRounds}";
pub const FR_GOAL_STATUS_UPDATED: &str = "Updated {time}";
pub const FR_GOAL_CONFIRM_START: &str = "Start this goal?\n\n{objective}";
pub const FR_GOAL_CONFIRM_RESUME: &str = "Resume this goal?\n\n{objective}";
pub const FR_GOAL_NOTICE_PAUSED: &str = "◎ Goal {status}\n{reason}";
pub const FR_GOAL_NOTICE_COMPLETE: &str = "◎ Goal {status}\n{reason}";
pub const FR_GOAL_ABORT_PAUSED: &str = "Goal paused: {reason}";
pub const FR_GOAL_ABORT_FAILED: &str = "Goal pause failed: {reason}";
pub const FR_GOAL_REASON_ROUND_LIMIT: &str = "Reached the limit of {rounds} rounds";
pub const FR_GOAL_REASON_WAIT_LIMIT: &str =
    "Nothing ran for {minutes} minutes while the goal waited on background work";
pub const FR_GOAL_REASON_NOT_PROGRESS: &str = "Not making progress: {reason}";

/// Verbatim `fr` goal template for a key, or `None` outside this slice
/// (TS: `fr[key]` for the 22 goal keys above).
pub fn lookup_fr_goal_template(key: &str) -> Option<&'static str> {
    match key {
        "pinned.line.goal" => Some(FR_PINNED_LINE_GOAL),
        "goal.none" => Some(FR_GOAL_NONE),
        "goal.input.prompt" => Some(FR_GOAL_INPUT_PROMPT),
        "goal.button.details" => Some(FR_GOAL_BUTTON_DETAILS),
        "goal.button.pause" => Some(FR_GOAL_BUTTON_PAUSE),
        "goal.button.resume" => Some(FR_GOAL_BUTTON_RESUME),
        "goal.start" => Some(FR_GOAL_START),
        "goal.too_long" => Some(FR_GOAL_TOO_LONG),
        "goal.status.active" => Some(FR_GOAL_STATUS_ACTIVE),
        "goal.status.paused" => Some(FR_GOAL_STATUS_PAUSED),
        "goal.status.complete" => Some(FR_GOAL_STATUS_COMPLETE),
        "goal.status.rounds" => Some(FR_GOAL_STATUS_ROUNDS),
        "goal.status.updated" => Some(FR_GOAL_STATUS_UPDATED),
        "goal.confirm.start" => Some(FR_GOAL_CONFIRM_START),
        "goal.confirm.resume" => Some(FR_GOAL_CONFIRM_RESUME),
        "goal.notice.paused" => Some(FR_GOAL_NOTICE_PAUSED),
        "goal.notice.complete" => Some(FR_GOAL_NOTICE_COMPLETE),
        "goal.abort.paused" => Some(FR_GOAL_ABORT_PAUSED),
        "goal.abort.failed" => Some(FR_GOAL_ABORT_FAILED),
        "goal.reason.round_limit" => Some(FR_GOAL_REASON_ROUND_LIMIT),
        "goal.reason.wait_limit" => Some(FR_GOAL_REASON_WAIT_LIMIT),
        "goal.reason.not_progress" => Some(FR_GOAL_REASON_NOT_PROGRESS),
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

/// `true` when the `fr` template keeps the `en` placeholder set exactly
/// (TS: per-key placeholder-equality loop in
/// `tests/i18n/placeholders.test.ts`).
pub fn is_fr_goal_placeholder_ok(en_template: &str, fr_template: &str) -> bool {
    placeholder_set(en_template) == placeholder_set(fr_template)
}

/// `A goal is limited to {limit} characters (got {count}).` with counts
/// filled in (TS: `t("goal.too_long", { limit, count })` call site).
pub fn format_fr_goal_too_long(limit: usize, count: usize) -> String {
    let limit = limit.to_string();
    let count = count.to_string();
    interpolate_fr(FR_GOAL_TOO_LONG, &[("limit", &limit), ("count", &count)])
}

/// `Reached the limit of {rounds} rounds` with the round count filled in
/// (TS: `t("goal.reason.round_limit", { rounds })` call site).
pub fn format_fr_goal_round_limit(rounds: u32) -> String {
    let rounds = rounds.to_string();
    interpolate_fr(FR_GOAL_REASON_ROUND_LIMIT, &[("rounds", &rounds)])
}
