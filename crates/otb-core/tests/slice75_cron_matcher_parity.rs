//! Slice 75 parity: cron matcher pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/app/services/scheduled-task-next-run-service.ts`:
//! - `normalizeWeekday` (7 -> 0)
//! - `MONTH_ALIASES` / `WEEKDAY_ALIASES` lookups
//! - `parseFieldValue` numeric + month/weekday alias forms
//! - `expandFieldBase` (`*`, ranges, singles, range errors)
//! - `expandFieldToken` (`/step` forms, step errors)
//! - `parseCronField` (comma split, blank-token error, wildcard flag)
//! - `parseCronExpression` (five parts, unsupported-expression error)
//! - `matchesCronField` membership, `matchesCron` day semantics + full match
//! - `computeNextCronRunAt` search-start alignment
//! - `computeNextRunAt` once-branch gate
//!
//! Every test MUST FAIL until `cron_matcher_gates` is ported (step 2).

use otb_core::cron_matcher_gates::*;
use std::collections::BTreeSet;

// --- normalize_weekday ---

#[test]
fn slice75_normalize_weekday_seven_to_sunday_parity() {
    assert_eq!(normalize_weekday(7), 0);
}

#[test]
fn slice75_normalize_weekday_sunday_kept_parity() {
    assert_eq!(normalize_weekday(0), 0);
}

// --- month_alias ---

#[test]
fn slice75_month_alias_jan_parity() {
    assert_eq!(month_alias("jan"), Some(1));
}

#[test]
fn slice75_month_alias_unknown_none_parity() {
    assert_eq!(month_alias("foo"), None);
}

// --- weekday_alias ---

#[test]
fn slice75_weekday_alias_sun_parity() {
    assert_eq!(weekday_alias("sun"), Some(0));
}

#[test]
fn slice75_weekday_alias_sat_parity() {
    assert_eq!(weekday_alias("sat"), Some(6));
}

#[test]
fn slice75_weekday_alias_unknown_none_parity() {
    assert_eq!(weekday_alias("funday"), None);
}

// --- parse_field_value ---

#[test]
fn slice75_parse_field_value_plain_parity() {
    assert_eq!(parse_field_value("5", 0, 59), Ok(5));
}

#[test]
fn slice75_parse_field_value_trims_whitespace_parity() {
    assert_eq!(parse_field_value("  7 ", 0, 59), Ok(7));
}

#[test]
fn slice75_parse_field_value_below_min_fails_parity() {
    assert_eq!(
        parse_field_value("-1", 0, 59),
        Err("Invalid cron field value: -1".to_string())
    );
}

#[test]
fn slice75_parse_field_value_above_max_fails_parity() {
    assert_eq!(
        parse_field_value("60", 0, 59),
        Err("Invalid cron field value: 60".to_string())
    );
}

#[test]
fn slice75_parse_field_value_garbage_fails_parity() {
    assert_eq!(
        parse_field_value("5x", 0, 59),
        Err("Invalid cron field value: 5x".to_string())
    );
}

// --- parse_month_value / parse_weekday_value ---

#[test]
fn slice75_parse_month_value_alias_parity() {
    assert_eq!(parse_month_value("mar"), Ok(3));
}

#[test]
fn slice75_parse_month_value_alias_case_insensitive_parity() {
    assert_eq!(parse_month_value("JAN"), Ok(1));
}

#[test]
fn slice75_parse_month_value_invalid_parity() {
    assert_eq!(
        parse_month_value("13"),
        Err("Invalid cron field value: 13".to_string())
    );
}

#[test]
fn slice75_parse_weekday_value_alias_parity() {
    assert_eq!(parse_weekday_value("mon"), Ok(1));
}

#[test]
fn slice75_parse_weekday_value_seven_normalizes_parity() {
    assert_eq!(parse_weekday_value("7"), Ok(0));
}

// --- expand_field_base ---

#[test]
fn slice75_expand_base_star_full_range_parity() {
    assert_eq!(expand_field_base("*", 0, 2), Ok(vec![0, 1, 2]));
}

#[test]
fn slice75_expand_base_range_span_parity() {
    assert_eq!(expand_field_base("1-3", 0, 59), Ok(vec![1, 2, 3]));
}

#[test]
fn slice75_expand_base_reversed_range_fails_parity() {
    assert_eq!(
        expand_field_base("5-2", 0, 59),
        Err("Invalid cron field range: 5-2".to_string())
    );
}

// --- expand_field_token ---

#[test]
fn slice75_expand_token_star_step_parity() {
    assert_eq!(expand_field_token("*/15", 0, 59), Ok(vec![0, 15, 30, 45]));
}

#[test]
fn slice75_expand_token_range_step_parity() {
    assert_eq!(expand_field_token("0-10/5", 0, 59), Ok(vec![0, 5, 10]));
}

#[test]
fn slice75_expand_token_zero_step_fails_parity() {
    assert_eq!(
        expand_field_token("*/0", 0, 59),
        Err("Invalid cron field step: */0".to_string())
    );
}

#[test]
fn slice75_expand_token_garbage_step_fails_parity() {
    assert_eq!(
        expand_field_token("5/x", 0, 59),
        Err("Invalid cron field step: 5/x".to_string())
    );
}

// --- parse_cron_field ---

#[test]
fn slice75_parse_field_star_wildcard_parity() {
    let field = parse_cron_field("*", 0, 59).expect("star parses");
    assert!(field.wildcard);
    assert_eq!(field.values.len(), 60);
    assert!(field.values.contains(&0));
    assert!(field.values.contains(&59));
}

#[test]
fn slice75_parse_field_list_not_wildcard_parity() {
    let field = parse_cron_field("1,2,3", 0, 59).expect("list parses");
    assert!(!field.wildcard);
    assert_eq!(
        field.values,
        BTreeSet::from([1, 2, 3])
    );
}

#[test]
fn slice75_parse_field_blank_token_fails_parity() {
    assert_eq!(
        parse_cron_field("1,,2", 0, 59),
        Err("Invalid cron field: 1,,2".to_string())
    );
}

#[test]
fn slice75_parse_month_field_alias_set_parity() {
    let field = parse_cron_month_field("jan").expect("month alias parses");
    assert!(!field.wildcard);
    assert_eq!(field.values, BTreeSet::from([1]));
}

#[test]
fn slice75_parse_weekday_field_sunday_alias_parity() {
    let field = parse_cron_weekday_field("sun").expect("weekday alias parses");
    assert!(field.values.contains(&0));
}

// --- parse_cron_expression ---

#[test]
fn slice75_parse_expression_every_minute_parity() {
    let expr = parse_cron_expression("* * * * *").expect("every minute parses");
    assert!(expr.minute.wildcard);
    assert!(expr.hour.wildcard);
    assert!(expr.day_of_month.wildcard);
    assert!(expr.month.wildcard);
    assert!(expr.day_of_week.wildcard);
}

#[test]
fn slice75_parse_expression_daily_noon_parity() {
    let expr = parse_cron_expression("0 12 * * *").expect("daily parses");
    assert_eq!(expr.minute.values, BTreeSet::from([0]));
    assert_eq!(expr.hour.values, BTreeSet::from([12]));
    assert!(expr.day_of_month.wildcard);
}

#[test]
fn slice75_parse_expression_month_alias_parity() {
    let expr = parse_cron_expression("0 9 * jan *").expect("month alias parses");
    assert_eq!(expr.month.values, BTreeSet::from([1]));
}

#[test]
fn slice75_parse_expression_too_few_parts_fails_parity() {
    assert_eq!(
        parse_cron_expression("* * * *"),
        Err("Unsupported cron expression: * * * *".to_string())
    );
}

#[test]
fn slice75_parse_expression_too_many_parts_fails_parity() {
    assert_eq!(
        parse_cron_expression("* * * * * *"),
        Err("Unsupported cron expression: * * * * * *".to_string())
    );
}

#[test]
fn slice75_parse_expression_bad_field_fails_parity() {
    assert!(parse_cron_expression("99 * * * *").is_err());
}

// --- cron_field_matches ---

#[test]
fn slice75_field_matches_hit_parity() {
    let field = parse_cron_field("5", 0, 59).expect("field parses");
    assert!(cron_field_matches(&field, 5));
}

#[test]
fn slice75_field_matches_miss_parity() {
    let field = parse_cron_field("5", 0, 59).expect("field parses");
    assert!(!cron_field_matches(&field, 6));
}

// --- cron_day_matches ---

#[test]
fn slice75_day_both_wildcards_match_parity() {
    assert!(cron_day_matches(true, true, false, false));
}

#[test]
fn slice75_day_dom_wildcard_defers_to_dow_hit_parity() {
    assert!(cron_day_matches(true, false, false, true));
}

#[test]
fn slice75_day_dom_wildcard_defers_to_dow_miss_parity() {
    assert!(!cron_day_matches(true, false, false, false));
}

#[test]
fn slice75_day_neither_wildcard_or_semantics_parity() {
    assert!(cron_day_matches(false, false, false, true));
    assert!(cron_day_matches(false, false, true, false));
    assert!(!cron_day_matches(false, false, false, false));
}

// --- cron_expression_matches ---

fn slice75_noon_parts() -> ZonedDateParts {
    ZonedDateParts {
        year: 2026,
        month: 11,
        day: 8,
        hour: 12,
        minute: 0,
        weekday: 0,
    }
}

#[test]
fn slice75_expression_matches_daily_noon_parity() {
    let expr = parse_cron_expression("0 12 * * *").expect("daily parses");
    assert!(cron_expression_matches(&expr, &slice75_noon_parts()));
}

#[test]
fn slice75_expression_mismatched_minute_parity() {
    let expr = parse_cron_expression("30 12 * * *").expect("parses");
    assert!(!cron_expression_matches(&expr, &slice75_noon_parts()));
}

// --- next_cron_search_start_ms ---

#[test]
fn slice75_search_start_aligns_next_minute_parity() {
    // 61_000ms -> floor(61_000 / 60_000) * 60_000 + 60_000 = 120_000.
    assert_eq!(next_cron_search_start_ms(61_000), 120_000);
}

#[test]
fn slice75_search_start_on_exact_minute_advances_parity() {
    // Exact boundaries still advance one full minute (TS adds MINUTE_MS).
    assert_eq!(next_cron_search_start_ms(60_000), 120_000);
}

// --- once_next_run_ms ---

#[test]
fn slice75_once_future_kept_parity() {
    assert_eq!(once_next_run_ms(Some(200_000), 100_000), Some(200_000));
}

#[test]
fn slice75_once_past_is_none_parity() {
    assert_eq!(once_next_run_ms(Some(50_000), 100_000), None);
}

#[test]
fn slice75_once_missing_is_none_parity() {
    assert_eq!(once_next_run_ms(None, 100_000), None);
}
