//! Cron matcher pure gates (slice 75, GREEN).
//!
//! Rust port of the pure remainder in
//! `src/app/services/scheduled-task-next-run-service.ts`:
//! - `MINUTE_MS` / `MAX_SEARCH_MINUTES` search budget
//! - `MONTH_ALIASES` / `WEEKDAY_ALIASES` lookups (`normalizeWeekday` 7 -> 0)
//! - `parseFieldValue` (alias fold, integer + range check, exact
//!   `Invalid cron field value: <raw>` message)
//! - `expandFieldBase` (`*` full range, `start-end` range with
//!   `Invalid cron field range: <base>` on shape or `start > end`,
//!   single value)
//! - `expandFieldToken` (`base/step` with `Invalid cron field step: <token>`
//!   on bad step; `*` steps from `min`, ranged bases step by position)
//! - `parseCronField` (comma split, blank-token
//!   `Invalid cron field: <field>`, `wildcard` flag, value set)
//! - `parseCronExpression` (five whitespace-separated parts, exact
//!   `Unsupported cron expression: <cron>` message, month/weekday aliases)
//! - `matchesCronField` set membership
//! - `matchesCron` day semantics (both wildcards always match; one
//!   wildcard defers to the other side; otherwise dom OR dow) plus the
//!   minute/hour/month conjunction
//! - `computeNextCronRunAt` search-start alignment
//!   (`floor(from / MINUTE_MS) * MINUTE_MS + MINUTE_MS`)
//! - `computeNextRunAt` once-branch gate (unparseable or past runAt -> `None`)
//!
//! Timezone resolution (`Intl.DateTimeFormat` zoned parts) and the minute
//! scan loop stay with the caller — here matching runs over an injected
//! [`ZonedDateParts`] so the gates stay pure and testable. `isTaskDue`
//! (disabled-cron / missing / NaN / due gates) already lives in
//! `stores_runtime` / `scheduled_tasks`; this module does not duplicate it.
//!
//! DEVIATIONS from TS (behavior-preserving for the covered cases):
//! 1. TS `Set<number>` maps to `BTreeSet<i64>` (sorted, no `NaN`); parity
//!    fixtures use integer field values only.
//! 2. TS alias lookup lowercases + trims before consulting the alias map;
//!    the port does the same (ASCII lowercase + trim).
//! 3. TS `Number.parseInt` accepts a trailing garbage tail (`"5x"` -> `5`)
//!    and TS `expandFieldToken` accepts a garbage-tailed step; the port
//!    requires exact ASCII-digit tokens, so `"5x"` (value) and `"5/x"`
//!    (step) are errors. Covered parity cases expect the strict errors.
//! 4. TS `base.split("-")` silently drops a third range segment
//!    (`"1-2-3"` parses as `1-2`); the port rejects anything that is not
//!    exactly `start-end` with `Invalid cron field range: <base>`.
//! 5. TS `token.split("/")` silently drops a third step segment
//!    (`"1/2/3"` steps by `2`); the port treats a non-integer step
//!    remainder as `Invalid cron field step: <token>`.
//! 6. TS throws `Error` with the messages above; the port returns
//!    `Err(message)` with the identical text.
//! 7. `next_cron_search_start_ms` uses Euclidean division so negative
//!    (pre-epoch) inputs still floor toward the past like TS
//!    `Math.floor`; positive inputs are identical either way.

use std::collections::BTreeSet;

/// One minute in millis (TS: `MINUTE_MS`).
pub const CRON_MINUTE_MS: i64 = 60_000;

/// Minute-scan budget for `computeNextCronRunAt`
/// (TS: `MAX_SEARCH_MINUTES = 60 * 24 * 366 * 2`).
pub const CRON_MAX_SEARCH_MINUTES: u32 = 60 * 24 * 366 * 2;

/// One cron field matcher: the raw `*` flag plus its value set
/// (TS: `CronFieldMatcher`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronFieldMatcher {
    pub wildcard: bool,
    pub values: BTreeSet<i64>,
}

/// Five parsed cron fields (TS: `ParsedCronExpression`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCronExpression {
    pub minute: CronFieldMatcher,
    pub hour: CronFieldMatcher,
    pub day_of_month: CronFieldMatcher,
    pub month: CronFieldMatcher,
    pub day_of_week: CronFieldMatcher,
}

/// Timezone-resolved calendar parts (TS: `ZonedDateParts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZonedDateParts {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub minute: i64,
    /// Sunday-first weekday: `0` (Sunday) through `6` (Saturday).
    pub weekday: i64,
}

/// `7 -> 0`, everything else unchanged (TS: `normalizeWeekday`).
pub fn normalize_weekday(value: i64) -> i64 {
    if value == 7 { 0 } else { value }
}

/// Lowercased month alias to 1-12 (`jan` -> `1`, ..., `dec` -> `12`).
/// Returns `None` for unknown names (TS: `MONTH_ALIASES`).
pub fn month_alias(name: &str) -> Option<i64> {
    match name.trim().to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

/// Lowercased weekday alias to Sunday-first 0-6 (`sun` -> `0`, ..., `sat` -> `6`).
/// Returns `None` for unknown names (TS: `WEEKDAY_ALIASES`).
pub fn weekday_alias(name: &str) -> Option<i64> {
    match name.trim().to_ascii_lowercase().as_str() {
        "sun" => Some(0),
        "mon" => Some(1),
        "tue" => Some(2),
        "wed" => Some(3),
        "thu" => Some(4),
        "fri" => Some(5),
        "sat" => Some(6),
        _ => None,
    }
}

/// Strict ASCII integer: optional leading `+`/`-` then digits only.
/// (TS `Number.parseInt` would accept a garbage tail; the port rejects it.)
fn parse_strict_int(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let body = trimmed
        .strip_prefix(|c| c == '+' || c == '-')
        .unwrap_or(trimmed);
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    trimmed.parse::<i64>().ok()
}

/// Numeric field value with `min..=max` range check.
/// `Err("Invalid cron field value: <raw>")` on non-integer or out-of-range
/// input (TS: `parseFieldValue` without aliases).
pub fn parse_field_value(raw: &str, min: i64, max: i64) -> Result<i64, String> {
    match parse_strict_int(&raw.trim().to_ascii_lowercase()) {
        Some(value) if value >= min && value <= max => Ok(value),
        _ => Err(format!("Invalid cron field value: {raw}")),
    }
}

/// Month field value: alias first, then numeric 1-12
/// (TS: `parseFieldValue` with `MONTH_ALIASES`).
pub fn parse_month_value(raw: &str) -> Result<i64, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if let Some(value) = month_alias(&normalized) {
        return Ok(value);
    }
    match parse_strict_int(&normalized) {
        Some(value) if (1..=12).contains(&value) => Ok(value),
        _ => Err(format!("Invalid cron field value: {raw}")),
    }
}

/// Weekday field value: alias first, then numeric 0-7 normalized with
/// `normalize_weekday` (TS: `parseFieldValue` with `WEEKDAY_ALIASES`).
pub fn parse_weekday_value(raw: &str) -> Result<i64, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if let Some(value) = weekday_alias(&normalized) {
        return Ok(value);
    }
    match parse_strict_int(&normalized) {
        Some(value) if (0..=7).contains(&value) => Ok(normalize_weekday(value)),
        _ => Err(format!("Invalid cron field value: {raw}")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Numeric,
    Month,
    Weekday,
}

fn parse_single_value(raw: &str, min: i64, max: i64, kind: FieldKind) -> Result<i64, String> {
    match kind {
        FieldKind::Numeric => parse_field_value(raw, min, max),
        FieldKind::Month => parse_month_value(raw),
        FieldKind::Weekday => parse_weekday_value(raw),
    }
}

fn expand_base_inner(
    base: &str,
    min: i64,
    max: i64,
    kind: FieldKind,
) -> Result<Vec<i64>, String> {
    if base.trim() == "*" {
        return Ok((min..=max).collect());
    }

    if base.contains('-') {
        let segments: Vec<&str> = base.split('-').collect();
        if segments.len() != 2 {
            return Err(format!("Invalid cron field range: {base}"));
        }
        let start = parse_single_value(segments[0], min, max, kind)?;
        let end = parse_single_value(segments[1], min, max, kind)?;
        if start > end {
            return Err(format!("Invalid cron field range: {base}"));
        }
        return Ok((start..=end).collect());
    }

    Ok(vec![parse_single_value(base, min, max, kind)?])
}

fn expand_token_inner(
    token: &str,
    min: i64,
    max: i64,
    kind: FieldKind,
) -> Result<Vec<i64>, String> {
    // Mirror TS `token.split("/")` destructuring: the base is the first
    // segment and the step is the second; anything past that is folded into
    // the step remainder so strict integer parsing rejects it (deviation 5).
    let mut segments = token.split('/');
    let base_raw = segments.next().unwrap_or("");
    let step_raw = segments.next();
    let rest: Vec<&str> = segments.collect();
    let base_values = expand_base_inner(base_raw, min, max, kind)?;

    let Some(step_text) = step_raw else {
        return Ok(base_values);
    };
    let step_source = if rest.is_empty() {
        step_text.to_string()
    } else {
        std::iter::once(step_text)
            .chain(rest.into_iter())
            .collect::<Vec<_>>()
            .join("/")
    };
    let step = match parse_strict_int(&step_source) {
        Some(step) if step > 0 => step,
        _ => return Err(format!("Invalid cron field step: {token}")),
    };

    if base_raw.trim() == "*" {
        Ok(base_values
            .into_iter()
            .filter(|value| (value - min) % step == 0)
            .collect())
    } else {
        Ok(base_values
            .into_iter()
            .enumerate()
            .filter(|(index, _)| (*index as i64) % step == 0)
            .map(|(_, value)| value)
            .collect())
    }
}

fn parse_field_inner(
    field: &str,
    min: i64,
    max: i64,
    kind: FieldKind,
) -> Result<CronFieldMatcher, String> {
    let normalized_field = field.trim().to_ascii_lowercase();
    let mut values = BTreeSet::new();

    for token in normalized_field.split(',') {
        let trimmed_token = token.trim();
        if trimmed_token.is_empty() {
            return Err(format!("Invalid cron field: {field}"));
        }
        for value in expand_token_inner(trimmed_token, min, max, kind)? {
            let mapped = match kind {
                FieldKind::Weekday => normalize_weekday(value),
                _ => value,
            };
            values.insert(mapped);
        }
    }

    Ok(CronFieldMatcher {
        wildcard: normalized_field == "*",
        values,
    })
}

/// Expands one range base: `*` -> full `min..=max`, `start-end` -> span,
/// otherwise a single value. `Err("Invalid cron field range: <base>")` on
/// bad shape or `start > end` (TS: `expandFieldBase`).
pub fn expand_field_base(base: &str, min: i64, max: i64) -> Result<Vec<i64>, String> {
    expand_base_inner(base, min, max, FieldKind::Numeric)
}

/// Expands one comma-token with an optional `/step` suffix.
/// `Err("Invalid cron field step: <token>")` on a bad step
/// (TS: `expandFieldToken`).
pub fn expand_field_token(token: &str, min: i64, max: i64) -> Result<Vec<i64>, String> {
    expand_token_inner(token, min, max, FieldKind::Numeric)
}

/// Parses one comma-separated cron field: blank tokens fail with
/// `Err("Invalid cron field: <field>")`; `wildcard` is true only for the
/// exact trimmed-lowercased `"*"` field (TS: `parseCronField`, numeric).
pub fn parse_cron_field(field: &str, min: i64, max: i64) -> Result<CronFieldMatcher, String> {
    parse_field_inner(field, min, max, FieldKind::Numeric)
}

/// Parses a month field with month aliases (TS: `parseCronField(month, 1, 12,
/// MONTH_ALIASES)`).
pub fn parse_cron_month_field(field: &str) -> Result<CronFieldMatcher, String> {
    parse_field_inner(field, 1, 12, FieldKind::Month)
}

/// Parses a weekday field with weekday aliases + 7 -> 0 normalization
/// (TS: `parseCronField(dayOfWeek, 0, 7, WEEKDAY_ALIASES, normalizeWeekday)`).
pub fn parse_cron_weekday_field(field: &str) -> Result<CronFieldMatcher, String> {
    parse_field_inner(field, 0, 7, FieldKind::Weekday)
}

/// Parses a five-field cron expression. `Err("Unsupported cron expression:
/// <cron>")` unless exactly five non-empty parts are present
/// (TS: `parseCronExpression`).
pub fn parse_cron_expression(cron: &str) -> Result<ParsedCronExpression, String> {
    let parts: Vec<&str> = cron.trim().split_whitespace().collect();
    if parts.len() != 5 || parts.iter().any(|part| part.is_empty()) {
        return Err(format!("Unsupported cron expression: {cron}"));
    }
    Ok(ParsedCronExpression {
        minute: parse_field_inner(parts[0], 0, 59, FieldKind::Numeric)?,
        hour: parse_field_inner(parts[1], 0, 23, FieldKind::Numeric)?,
        day_of_month: parse_field_inner(parts[2], 1, 31, FieldKind::Numeric)?,
        month: parse_field_inner(parts[3], 1, 12, FieldKind::Month)?,
        day_of_week: parse_field_inner(parts[4], 0, 7, FieldKind::Weekday)?,
    })
}

/// Set membership (TS: `matchesCronField`).
pub fn cron_field_matches(field: &CronFieldMatcher, value: i64) -> bool {
    field.values.contains(&value)
}

/// Day-side semantics of `matchesCron`: both wildcards always match; a
/// single wildcard defers to the other side; otherwise dom OR dow
/// (TS: the `dayMatch` branch).
pub fn cron_day_matches(
    dom_wildcard: bool,
    dow_wildcard: bool,
    dom_match: bool,
    dow_match: bool,
) -> bool {
    if dom_wildcard && dow_wildcard {
        true
    } else if dom_wildcard {
        dow_match
    } else if dow_wildcard {
        dom_match
    } else {
        dom_match || dow_match
    }
}

/// Full match over injected zoned parts (TS: `matchesCron`).
pub fn cron_expression_matches(expression: &ParsedCronExpression, parts: &ZonedDateParts) -> bool {
    let minute_match = cron_field_matches(&expression.minute, parts.minute);
    let hour_match = cron_field_matches(&expression.hour, parts.hour);
    let month_match = cron_field_matches(&expression.month, parts.month);
    let dom_match = cron_field_matches(&expression.day_of_month, parts.day);
    let dow_match = cron_field_matches(&expression.day_of_week, parts.weekday);
    let day_match = cron_day_matches(
        expression.day_of_month.wildcard,
        expression.day_of_week.wildcard,
        dom_match,
        dow_match,
    );
    minute_match && hour_match && month_match && day_match
}

/// Search-start alignment: the next whole minute after `from_ms`
/// (TS: `Math.floor(from / MINUTE_MS) * MINUTE_MS + MINUTE_MS`).
pub fn next_cron_search_start_ms(from_ms: i64) -> i64 {
    from_ms.div_euclid(CRON_MINUTE_MS) * CRON_MINUTE_MS + CRON_MINUTE_MS
}

/// Once-task gate: `None` when `run_at_ms` is missing (`NaN` in TS) or not
/// after `from_ms`, otherwise the run instant back
/// (TS: `computeNextRunAt` once branch).
pub fn once_next_run_ms(run_at_ms: Option<i64>, from_ms: i64) -> Option<i64> {
    match run_at_ms {
        Some(run_at) if run_at > from_ms => Some(run_at),
        _ => None,
    }
}
