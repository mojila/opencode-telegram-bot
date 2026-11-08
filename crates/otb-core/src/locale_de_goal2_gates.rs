//! German goal-remainder gates (slice 126, RED).
//!
//! Rust port of the pure remainder in `src/i18n/de.ts` for the goal
//! command surface NOT covered by slice 125 (`GOAL_KEYS` in
//! `crates/otb-core/src/i18n_full.rs` covers only 22 keys):
//! - `goal.arguments_not_supported`
//! - `goal.input.empty`, `goal.input.reserved`
//! - `goal.button.clear`
//! - `goal.continue`, `goal.delete`, `goal.deleted`, `goal.unsupported`,
//!   `goal.read_error`, `goal.action_error`
//! - `goal.confirm.warning`, `goal.confirm.round_limit`,
//!   `goal.confirm.confirm`, `goal.confirm.cancel`
//! - `goal.notice.no_reason`
//! - `goal.reason.paused_by_user`, `goal.reason.stopped_by_user`,
//!   `goal.reason.server_restarted`, `goal.reason.stopped_after_error`,
//!   `goal.reason.no_verdict`, `goal.reason.review_too_large`,
//!   `goal.reason.session_failed`, `goal.reason.no_active`,
//!   `goal.reason.no_paused`, `goal.reason.none_set`,
//!   `goal.reason.objective_limit`
//!   (all currently English fallback, untranslated in `de.ts` lines 760-805).
//!
//! AUDIT (2026-11-08, branch `rust-rewrite`):
//! - `src/i18n/de.ts` lines 760 (`goal.arguments_not_supported`), 762-763
//!   (`goal.input.empty` / `reserved`), 767 (`goal.button.clear`), 769-774
//!   (`goal.continue` / `delete` / `deleted` / `unsupported` / `read_error` /
//!   `action_error`), 781-782 (`goal.confirm.warning` / `round_limit`),
//!   785-786 (`goal.confirm.confirm` / `cancel`), 789
//!   (`goal.notice.no_reason`), 792-797 (`goal.reason.paused_by_user` /
//!   `stopped_by_user` / `server_restarted` / `stopped_after_error` /
//!   `no_verdict` / `review_too_large`), 801-805
//!   (`goal.reason.session_failed` / `no_active` / `no_paused` / `none_set` /
//!   `objective_limit`) carry the templates verbatim; `src/i18n/en.ts`
//!   lines 487-532 are the placeholder reference (`de` matches `en`
//!   verbatim here, no translation yet).
//! - `crate::i18n` embeds only 8 keys for all 12 locales;
//!   `crate::i18n_full` embeds 71 keys for `en`/`ru` only, with `GOAL_KEYS`
//!   covering just the 22 slice-125 goal keys (`de` falls back to `en` in
//!   `t_full`). No Rust owner embeds the 26 `de` goal-remainder templates.
//! - `locale_placeholder_gates` (slice 122) owns only the set-equality
//!   predicates. `locale_de_status_gates` (slice 123) owns only the `de`
//!   status lookup. `locale_de_queue_gates` (slice 124) owns only the `de`
//!   queue lookup. `locale_de_goal_gates` (slice 125) owns only the first
//!   22 `de` goal keys.
//! - This slice owns only the `de` goal-remainder lookup +
//!   placeholder-equality + line-format gates; full 12-locale x 654-key
//!   table codegen stays future work.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `de.ts` file reads (arrive as explicit `key` args here),
//!   per-locale test looping, `t()` fallback wiring.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `SUPPORTED_LOCALES` order, `resolveSupportedLocale`, `normalizeLocale`,
//!   `getDateLocale`, `t` fallback (`crate::i18n` owner)
//! - `FULL_KEYS` / `GOAL_KEYS` (22 keys) + en/ru verbatim tables +
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
//!   (`locale_de_goal_gates` owner, slice 125).
//!
//! Deviations: none intended. `{name}` matching is ASCII
//! `[A-Za-z0-9_]+` exactly like the TS regex; `text.length` parity uses
//! ASCII-only fixtures where UTF-16 units and Rust `char`s coincide.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice126_locale_de_goal2_parity.rs`
//! assert the TS contract and MUST FAIL with `unimplemented!()` panics
//! until the Rust port lands (step 2). Only the constant-value tests pass.

/// Locale code for this table (TS: `de` in `SUPPORTED_LOCALES`).
pub const DE_GOAL2_LOCALE: &str = "de";

/// Goal-remainder keys covered here (the `goal.*` keys in `src/i18n/de.ts`
/// lines 760-805 outside the 22 `GOAL_KEYS` in `crate::i18n_full`).
pub const DE_GOAL2_KEYS: [&str; 26] = [
    "goal.arguments_not_supported",
    "goal.input.empty",
    "goal.input.reserved",
    "goal.button.clear",
    "goal.continue",
    "goal.delete",
    "goal.deleted",
    "goal.unsupported",
    "goal.read_error",
    "goal.action_error",
    "goal.confirm.warning",
    "goal.confirm.round_limit",
    "goal.confirm.confirm",
    "goal.confirm.cancel",
    "goal.notice.no_reason",
    "goal.reason.paused_by_user",
    "goal.reason.stopped_by_user",
    "goal.reason.server_restarted",
    "goal.reason.stopped_after_error",
    "goal.reason.no_verdict",
    "goal.reason.review_too_large",
    "goal.reason.session_failed",
    "goal.reason.no_active",
    "goal.reason.no_paused",
    "goal.reason.none_set",
    "goal.reason.objective_limit",
];

/// Verbatim `de` templates (TS: `src/i18n/de.ts` lines 760-805).
/// Currently English fallback (untranslated).
pub const DE_GOAL_ARGUMENTS_NOT_SUPPORTED: &str =
    "Use /goal without arguments, then follow the prompts or use the goal buttons.";
pub const DE_GOAL_INPUT_EMPTY: &str =
    "The objective cannot be empty. Send its text or tap Cancel.";
pub const DE_GOAL_INPUT_RESERVED: &str =
    "That word is reserved by OpenCode for goal controls. Describe the objective in more detail, or tap Cancel.";
pub const DE_GOAL_BUTTON_CLEAR: &str = "× Delete";
pub const DE_GOAL_CONTINUE: &str = "▶ Continue";
pub const DE_GOAL_DELETE: &str = "🗑 Delete goal";
pub const DE_GOAL_DELETED: &str = "Goal deleted. Use /goal to create a new one.";
pub const DE_GOAL_UNSUPPORTED: &str =
    "This OpenCode server does not support session goals.";
pub const DE_GOAL_READ_ERROR: &str = "Couldn't read the session goal.";
pub const DE_GOAL_ACTION_ERROR: &str = "Couldn't update the session goal.";
pub const DE_GOAL_CONFIRM_WARNING: &str = "This goal continues on its own after each step. Continuation and judging spend tokens. There is no cost cap. The server stops it after its round limit.";
pub const DE_GOAL_CONFIRM_ROUND_LIMIT: &str = "Round limit: {rounds}.";
pub const DE_GOAL_CONFIRM_CONFIRM: &str = "Confirm";
pub const DE_GOAL_CONFIRM_CANCEL: &str = "❌ Cancel";
pub const DE_GOAL_NOTICE_NO_REASON: &str = "No reason reported.";
pub const DE_GOAL_REASON_PAUSED_BY_USER: &str = "Paused by the user";
pub const DE_GOAL_REASON_STOPPED_BY_USER: &str = "Stopped by the user";
pub const DE_GOAL_REASON_SERVER_RESTARTED: &str =
    "The server restarted while the goal was active";
pub const DE_GOAL_REASON_STOPPED_AFTER_ERROR: &str =
    "The goal stopped after an error";
pub const DE_GOAL_REASON_NO_VERDICT: &str =
    "The goal reviewer gave no usable verdict";
pub const DE_GOAL_REASON_REVIEW_TOO_LARGE: &str =
    "The goal reviewer's request is too large for the model";
pub const DE_GOAL_REASON_SESSION_FAILED: &str = "The session failed: {reason}";
pub const DE_GOAL_REASON_NO_ACTIVE: &str = "No active goal to pause.";
pub const DE_GOAL_REASON_NO_PAUSED: &str = "No paused goal to resume.";
pub const DE_GOAL_REASON_NONE_SET: &str = "No goal is set.";
pub const DE_GOAL_REASON_OBJECTIVE_LIMIT: &str =
    "A goal objective is limited to {limit} characters (got {count}).";

/// Verbatim `de` goal-remainder template for a key, or `None` outside this
/// slice (TS: `de[key]` for the 26 goal-remainder keys above).
pub fn lookup_de_goal2_template(key: &str) -> Option<&'static str> {
    match key {
        "goal.arguments_not_supported" => Some(DE_GOAL_ARGUMENTS_NOT_SUPPORTED),
        "goal.input.empty" => Some(DE_GOAL_INPUT_EMPTY),
        "goal.input.reserved" => Some(DE_GOAL_INPUT_RESERVED),
        "goal.button.clear" => Some(DE_GOAL_BUTTON_CLEAR),
        "goal.continue" => Some(DE_GOAL_CONTINUE),
        "goal.delete" => Some(DE_GOAL_DELETE),
        "goal.deleted" => Some(DE_GOAL_DELETED),
        "goal.unsupported" => Some(DE_GOAL_UNSUPPORTED),
        "goal.read_error" => Some(DE_GOAL_READ_ERROR),
        "goal.action_error" => Some(DE_GOAL_ACTION_ERROR),
        "goal.confirm.warning" => Some(DE_GOAL_CONFIRM_WARNING),
        "goal.confirm.round_limit" => Some(DE_GOAL_CONFIRM_ROUND_LIMIT),
        "goal.confirm.confirm" => Some(DE_GOAL_CONFIRM_CONFIRM),
        "goal.confirm.cancel" => Some(DE_GOAL_CONFIRM_CANCEL),
        "goal.notice.no_reason" => Some(DE_GOAL_NOTICE_NO_REASON),
        "goal.reason.paused_by_user" => Some(DE_GOAL_REASON_PAUSED_BY_USER),
        "goal.reason.stopped_by_user" => Some(DE_GOAL_REASON_STOPPED_BY_USER),
        "goal.reason.server_restarted" => Some(DE_GOAL_REASON_SERVER_RESTARTED),
        "goal.reason.stopped_after_error" => {
            Some(DE_GOAL_REASON_STOPPED_AFTER_ERROR)
        }
        "goal.reason.no_verdict" => Some(DE_GOAL_REASON_NO_VERDICT),
        "goal.reason.review_too_large" => Some(DE_GOAL_REASON_REVIEW_TOO_LARGE),
        "goal.reason.session_failed" => Some(DE_GOAL_REASON_SESSION_FAILED),
        "goal.reason.no_active" => Some(DE_GOAL_REASON_NO_ACTIVE),
        "goal.reason.no_paused" => Some(DE_GOAL_REASON_NO_PAUSED),
        "goal.reason.none_set" => Some(DE_GOAL_REASON_NONE_SET),
        "goal.reason.objective_limit" => Some(DE_GOAL_REASON_OBJECTIVE_LIMIT),
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
pub fn is_de_goal2_placeholder_ok(
    en_template: &str,
    de_template: &str,
) -> bool {
    placeholder_set(en_template) == placeholder_set(de_template)
}

/// `A goal objective is limited to {limit} characters (got {count}).` with
/// counts filled in (TS: `t("goal.reason.objective_limit", { limit, count })`
/// call site).
pub fn format_de_goal2_objective_limit(limit: usize, count: usize) -> String {
    let limit = limit.to_string();
    let count = count.to_string();
    interpolate_de(
        DE_GOAL_REASON_OBJECTIVE_LIMIT,
        &[("limit", &limit), ("count", &count)],
    )
}

/// `Round limit: {rounds}.` with the round count filled in
/// (TS: `t("goal.confirm.round_limit", { rounds })` call site).
pub fn format_de_goal2_confirm_round_limit(rounds: u32) -> String {
    let rounds = rounds.to_string();
    interpolate_de(DE_GOAL_CONFIRM_ROUND_LIMIT, &[("rounds", &rounds)])
}
