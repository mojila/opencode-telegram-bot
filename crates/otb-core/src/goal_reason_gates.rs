//! Goal reason + confirmation gates (slice 65, GREEN).
//!
//! Rust port of the pure remainder of
//! `src/app/formatters/goal-text.ts` that `session_gates`
//! (`parseGoalObjective`, `flattenGoalObjective`, `truncateGoalObjective`)
//! does not cover:
//! - `localizeGoalReason` exact server sentences plus the five patterned
//!   reasons (round limit, wait limit, not-progress, session-failed,
//!   objective limit)
//! - `isNoActiveGoalReason` (`"No active goal to pause."` or the localized
//!   `goal.reason.no_active` line, compared after trimming)
//! - `fitConfirmation` (`CONFIRMATION_LIMIT` budget, ellipsis cut)
//! - `formatGoalWarning` / `formatGoalConfirmation` composition
//! - `formatAbortWithGoalPause` composition
//! - `formatGoalStatusText` line composition
//!
//! No `t(...)` locale lookups or `Intl` date formatting live here — only
//! name sets, pattern classifiers, and string composition. Callers resolve
//! localized lines first and pass them in.
//!
//! DEVIATIONS from TS:
//! 1. `localizeGoalReason` returns localized text via `t(...)` in TS. The
//!    port splits it: [`classify_goal_reason`] returns the structured
//!    [`GoalReasonKind`] (pattern captures included) and the caller
//!    localizes. `Unknown` means "return the reason verbatim".
//! 2. `isNoActiveGoalReason` reads `t("goal.reason.no_active")` in TS. The
//!    port takes the pre-resolved `localized_no_active` string; the server
//!    English sentence is [`SERVER_NO_ACTIVE_GOAL_REASON`].
//! 3. `formatGoalWarning` / `formatGoalConfirmation` / `formatAbortWithGoalPause`
//!    / `formatGoalStatusText` interpolate `t(...)` lines in TS. The port
//!    takes those lines pre-resolved (`round_limit_line`, `body`,
//!    `paused_clause` / `failed_clause`, `status`, `rounds_line`,
//!    `reason_line`, `updated_line`).
//! 4. JS `String.length` counts UTF-16 code units. [`fit_confirmation`]
//!    budgets in UTF-16 code units (like `session_gates::parse_goal_objective`)
//!    and cuts at `char` boundaries; ASCII output is identical to TS.
//! 5. No regex crate: the five `localizeGoalReason` patterns are hand-ported
//!    as prefix/suffix scans. `\d` maps to `is_ascii_digit` (exact match for
//!    JS `\d`, which is ASCII-only) and `[\s\S]+` maps to "at least one
//!    `char`" (TS `+` never matches empty).
//! 6. `fitConfirmation` is private in TS; the port exposes it as
//!    [`fit_confirmation`] so the confirmation composition stays testable.
//! 7. `String.prototype.trimEnd` (Unicode WhiteSpace + LineTerminator) maps
//!    to Rust `str::trim_end` (`char::is_whitespace`); identical on ASCII,
//!    may differ on exotic Unicode whitespace.

// ---------------------------------------------------------------------------
// Confirmation budget (`goal-text.ts`)
// ---------------------------------------------------------------------------

/// Combined warning + body cap (TS: `CONFIRMATION_LIMIT`).
pub const CONFIRMATION_LIMIT: usize = 3800;

/// Ellipsis appended to a cut confirmation body (TS: `ELLIPSIS`).
pub const GOAL_CONFIRMATION_ELLIPSIS: &str = "…";

/// Server English sentence for a missing active goal
/// (TS: `"No active goal to pause."` in `isNoActiveGoalReason`).
pub const SERVER_NO_ACTIVE_GOAL_REASON: &str = "No active goal to pause.";

// ---------------------------------------------------------------------------
// Reason classification (`localizeGoalReason`)
// ---------------------------------------------------------------------------

/// Structured shape of a goal `reason` string
/// (TS: the exact table + five patterns in `localizeGoalReason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalReasonKind<'a> {
    /// `"Paused by the user"`.
    PausedByUser,
    /// `"Stopped by the user"`.
    StoppedByUser,
    /// `"The server restarted while the goal was active"`.
    ServerRestarted,
    /// `"The goal stopped after an error"`.
    StoppedAfterError,
    /// `"The goal reviewer gave no usable verdict"`.
    NoVerdict,
    /// `"The goal reviewer's request is too large for the model"`.
    ReviewTooLarge,
    /// `"No active goal to pause."`.
    NoActive,
    /// `"No paused goal to resume."`.
    NoPaused,
    /// `"No goal is set."`.
    NoneSet,
    /// `/^Reached the limit of (\d+) rounds$/`.
    RoundLimit { rounds: &'a str },
    /// `/^Nothing ran for (\d+) minutes while the goal waited on background work$/`.
    WaitLimit { minutes: &'a str },
    /// `/^Not making progress: ([\s\S]+)$/`.
    NotProgress { reason: &'a str },
    /// `/^The session failed: ([\s\S]+)$/`.
    SessionFailed { reason: &'a str },
    /// `/^A goal objective is limited to (\d+) characters \(got (\d+)\)\.$/`.
    ObjectiveLimit { limit: &'a str, count: &'a str },
    /// Anything else: callers return it verbatim.
    Unknown,
}

/// Classify a goal `reason` without localizing it
/// (TS: the match order inside `localizeGoalReason`: exact table first,
/// then round limit, wait limit, not-progress, session-failed,
/// objective limit, else passthrough).
pub fn classify_goal_reason(reason: &str) -> GoalReasonKind<'_> {
    match reason {
        "Paused by the user" => return GoalReasonKind::PausedByUser,
        "Stopped by the user" => return GoalReasonKind::StoppedByUser,
        "The server restarted while the goal was active" => {
            return GoalReasonKind::ServerRestarted
        }
        "The goal stopped after an error" => return GoalReasonKind::StoppedAfterError,
        "The goal reviewer gave no usable verdict" => return GoalReasonKind::NoVerdict,
        "The goal reviewer's request is too large for the model" => {
            return GoalReasonKind::ReviewTooLarge
        }
        "No active goal to pause." => return GoalReasonKind::NoActive,
        "No paused goal to resume." => return GoalReasonKind::NoPaused,
        "No goal is set." => return GoalReasonKind::NoneSet,
        _ => {}
    }

    if let Some(rounds) = strip_affixed_digits(reason, "Reached the limit of ", " rounds") {
        return GoalReasonKind::RoundLimit { rounds };
    }

    if let Some(minutes) = strip_affixed_digits(
        reason,
        "Nothing ran for ",
        " minutes while the goal waited on background work",
    ) {
        return GoalReasonKind::WaitLimit { minutes };
    }

    if let Some(tail) = reason.strip_prefix("Not making progress: ") {
        if !tail.is_empty() {
            return GoalReasonKind::NotProgress { reason: tail };
        }
        return GoalReasonKind::Unknown;
    }

    if let Some(tail) = reason.strip_prefix("The session failed: ") {
        if !tail.is_empty() {
            return GoalReasonKind::SessionFailed { reason: tail };
        }
        return GoalReasonKind::Unknown;
    }

    if let Some((limit, count)) = parse_objective_limit(reason) {
        return GoalReasonKind::ObjectiveLimit { limit, count };
    }

    GoalReasonKind::Unknown
}

/// Match `prefix + (\d+) + suffix` anchored at both ends, returning the digit
/// slice. Empty digit runs and non-ASCII-digit bytes do not match
/// (TS: `/^...(\d+)...$/`, `\d` is ASCII-only).
fn strip_affixed_digits<'a>(reason: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let rest = reason.strip_prefix(prefix)?;
    let middle = rest.strip_suffix(suffix)?;
    if middle.is_empty() || !middle.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(middle)
}

/// Match `A goal objective is limited to (\d+) characters (got (\d+)).`
/// anchored at both ends (TS: the `tooLong` regex).
fn parse_objective_limit(reason: &str) -> Option<(&str, &str)> {
    let rest = reason.strip_prefix("A goal objective is limited to ")?;
    let digits_end = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits_end == 0 {
        return None;
    }
    let limit = &rest[..digits_end];
    let rest = &rest[digits_end..];
    let rest = rest.strip_prefix(" characters (got ")?;
    let digits_end = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits_end == 0 {
        return None;
    }
    let count = &rest[..digits_end];
    let rest = &rest[digits_end..];
    if rest != ")." {
        return None;
    }
    Some((limit, count))
}

// ---------------------------------------------------------------------------
// No-active-goal check (`isNoActiveGoalReason`)
// ---------------------------------------------------------------------------

/// Whether the reason is the "no active goal" sentence, server English or
/// localized (TS: `reason.trim()` against `"No active goal to pause."` and
/// `t("goal.reason.no_active")`).
pub fn is_no_active_goal_reason(reason: &str, localized_no_active: &str) -> bool {
    let trimmed = reason.trim();
    trimmed == SERVER_NO_ACTIVE_GOAL_REASON || trimmed == localized_no_active
}

// ---------------------------------------------------------------------------
// Confirmation fitting (`fitConfirmation`, `formatGoalWarning`,
// `formatGoalConfirmation`)
// ---------------------------------------------------------------------------

/// Length in UTF-16 code units (mirrors JS `String.length`).
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Take the longest `char`-boundary prefix of `s` whose UTF-16 length is at
/// most `budget_units` (mirrors `body.slice(0, n)` without splitting a
/// surrogate pair).
fn take_utf16_prefix(s: &str, budget_units: usize) -> &str {
    let mut used = 0usize;
    let mut end = 0usize;
    for (idx, ch) in s.char_indices() {
        let w = ch.len_utf16();
        if used + w > budget_units {
            break;
        }
        used += w;
        end = idx + ch.len_utf8();
    }
    &s[..end]
}

/// Fit `body` under the confirmation budget below `warning`
/// (TS: `fitConfirmation`: `warning\n\nbody` when it fits, else the warning
/// alone when `room <= ELLIPSIS.length`, else `warning\n\n` plus the body cut
/// to `room - ELLIPSIS.length` chars, trailing whitespace trimmed, plus `…`).
pub fn fit_confirmation(warning: &str, body: &str) -> String {
    let combined_units = utf16_len(warning) + 2 + utf16_len(body);
    if combined_units <= CONFIRMATION_LIMIT {
        return format!("{warning}\n\n{body}");
    }

    let warning_units = utf16_len(warning);
    let room = CONFIRMATION_LIMIT
        .saturating_sub(warning_units)
        .saturating_sub(2);
    let ellipsis_units = utf16_len(GOAL_CONFIRMATION_ELLIPSIS);
    if room <= ellipsis_units {
        return warning.to_string();
    }
    let cut = take_utf16_prefix(body, room - ellipsis_units);
    let cut = cut.trim_end();
    format!("{warning}\n\n{cut}{GOAL_CONFIRMATION_ELLIPSIS}")
}

/// Warning plus the optional pre-resolved round-limit line
/// (TS: `formatGoalWarning`: `t("goal.confirm.warning")`, optionally
/// followed by `\n` and `t("goal.confirm.round_limit", ...)`).
pub fn format_goal_warning(warning: &str, round_limit_line: Option<&str>) -> String {
    match round_limit_line {
        Some(line) => format!("{warning}\n{line}"),
        None => warning.to_string(),
    }
}

/// Warning plus the pre-resolved start/resume body, fitted to the budget
/// (TS: `formatGoalConfirmation` feeds `formatGoalWarning` and the
/// `goal.confirm.start` / `goal.confirm.resume` line into `fitConfirmation`).
pub fn format_goal_confirmation(warning: &str, body: &str) -> String {
    fit_confirmation(warning, body)
}

// ---------------------------------------------------------------------------
// Abort-with-pause composition (`formatAbortWithGoalPause`)
// ---------------------------------------------------------------------------

/// Outcome of pausing the goal alongside an abort
/// (TS: `AbortPauseOutcome`: silent, paused with reason, failed with reason).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortPauseOutcome<'a> {
    /// Goal pause stays silent: the abort text goes out unchanged.
    Silent,
    /// Goal paused: the pre-resolved paused clause heads the abort text.
    Paused { reason: &'a str },
    /// Goal pause failed: the pre-resolved failed clause heads the abort text.
    Failed { reason: &'a str },
}

/// Prepend the pause outcome clause to the abort text
/// (TS: `formatAbortWithGoalPause`: silent returns `abortText`, otherwise
/// `clause\n\nabortText` with the paused/failed `goal.abort.*` line).
pub fn format_abort_with_goal_pause(
    abort_text: &str,
    outcome: &AbortPauseOutcome,
    paused_clause: &str,
    failed_clause: &str,
) -> String {
    match outcome {
        AbortPauseOutcome::Silent => abort_text.to_string(),
        AbortPauseOutcome::Paused { .. } => format!("{paused_clause}\n\n{abort_text}"),
        AbortPauseOutcome::Failed { .. } => format!("{failed_clause}\n\n{abort_text}"),
    }
}

// ---------------------------------------------------------------------------
// Status text composition (`formatGoalStatusText`)
// ---------------------------------------------------------------------------

/// Stack the status lines for display
/// (TS: `formatGoalStatusText`: `objective`, blank, `◎ status`, rounds line,
/// optional reason line, updated line, joined with `\n`).
pub fn format_goal_status_text(
    objective: &str,
    status: &str,
    rounds_line: &str,
    reason_line: Option<&str>,
    updated_line: &str,
) -> String {
    let mut lines = vec![objective.to_string(), String::new(), format!("◎ {status}")];
    lines.push(rounds_line.to_string());
    if let Some(reason) = reason_line {
        lines.push(reason.to_string());
    }
    lines.push(updated_line.to_string());
    lines.join("\n")
}
