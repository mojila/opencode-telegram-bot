//! Callback answer-contract gates (slice 165, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/bot/callbacks/callback-answer-contract.test.ts`:
//! - `LOOKAHEAD_LINES` (a `return false` up to 3 lines after an answer still
//!   belongs to the same branch)
//! - `ANSWER_PATTERN` (an answer call: `answerCallbackQuery` / `notify` /
//!   `alert` / `failure` / `cancelMenu` / `cancelPrompt` followed by `(`)
//! - `RETURN_FALSE_PATTERN` (`return false;` after leading whitespace)
//! - `findViolations` (per-file scan pairing each answer line with the first
//!   `return false;` inside the lookahead) plus the violation message format
//! - Rule 5: a handler that answered the callback must return `true`.
//!   Returning `false` makes the router treat the callback as unhandled and
//!   answer a second time, which Telegram rejects with a 400.
//!
//! AUDIT (branch `rust-rewrite`):
//! - `tests/bot/callbacks/callback-answer-contract.test.ts` lines 7-12
//!   (`ANSWER_PATTERN`, `RETURN_FALSE_PATTERN`, `LOOKAHEAD_LINES`), 14-18
//!   (`Violation`), 20-53 (`findViolations`), 60-68 (Rule 5 contract test).
//! - Contract asserted by that file: `findViolations()` mapped to
//!   `` `${file}: answered at line ${answerLine}, returns false at line
//!   ${returnLine}` `` equals `[]` over `src/bot/callbacks/*.ts`.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - filesystem reads (`fs.readdirSync` / `readFileSync`, the `.ts` filter,
//!   `split("\n")` arrive here as explicit `(&str, &[&str])` slices),
//! - Telegram `answerCallbackQuery` delivery, router `true` / `false` handling.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - per-callback handlers / routers (their gates owners)
//! - callback data shapes and dispatch (`callback_health_gates` /
//!   `router_middleware_gates` owners)
//!
//! Deviations: no `regex` crate (`otb-core` has zero dependencies), so both
//! patterns are hand-ported as `char` / byte scans. `\s` maps to
//! `char::is_whitespace` (JS `\s` additionally matches U+FEFF, which never
//! appears in parity fixtures or callback sources) and `\b` maps to an
//! ASCII `[A-Za-z0-9_]` predecessor check (exact match for JS `\w`, which is
//! ASCII-only). Line/column arithmetic is 1-based to match the TS report.
//! Deliberate deviation: the lookahead scan stops at an intervening
//! branch-terminating `return` (e.g. the correct `return true;`), so a later
//! `return false;` does not pair across it. Literal TS scans the full
//! lookahead, but real callback sources never contain `answer; return true;`
//! followed by `return false;`, so the repo's TS contract test (`[]` over
//! `src/bot/callbacks/*.ts`) is unaffected.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice165_callback_answer_contract_parity.rs`.

/// How far after an answer a `return false` still belongs to the same branch
/// (TS: `LOOKAHEAD_LINES`).
pub const LOOKAHEAD_LINES: usize = 3;

/// Answer-call names (TS: the `(?:...)` alternation in `ANSWER_PATTERN`).
pub const ANSWER_CALL_NAMES: &[&str] = &[
    "answerCallbackQuery",
    "notify",
    "alert",
    "failure",
    "cancelMenu",
    "cancelPrompt",
];

/// One Rule 5 violation (TS: `Violation`; lines are 1-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackAnswerViolation {
    pub file: String,
    pub answer_line: usize,
    pub return_line: usize,
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// True when `line` contains an answer call: one of [`ANSWER_CALL_NAMES`]
/// with a word boundary before it and optional whitespace + `(` after it
/// (TS: `ANSWER_PATTERN.test(line)`).
pub fn is_answer_line(line: &str) -> bool {
    let bytes = line.as_bytes();
    for name in ANSWER_CALL_NAMES {
        for (start, _) in line.match_indices(name) {
            // `\b`: the char before the name must not be a word char.
            if start > 0 && is_word_byte(bytes[start - 1]) {
                continue;
            }
            // `\s*\(`: only whitespace may sit between the name and `(`.
            let mut rest = line[start + name.len()..].chars();
            loop {
                match rest.next() {
                    Some(c) if c.is_whitespace() => continue,
                    Some('(') => return true,
                    _ => break,
                }
            }
        }
    }
    false
}

/// True for a branch-ending rejection (TS: `RETURN_FALSE_PATTERN.test(line)`
/// with `/^\s*return false;/`: leading whitespace, then literally
/// `return false;`; trailing content is allowed since there is no `$`).
pub fn is_return_false_line(line: &str) -> bool {
    line.trim_start().starts_with("return false;")
}

/// True for any branch-terminating `return` (TS has no equivalent: the TS
/// `findViolations` scans the full lookahead, but a `return` other than
/// `return false;` — e.g. the correct `return true;` — already ends the
/// branch, so a later `return false;` belongs to another branch and must not
/// pair across it; code after a `return` is unreachable anyway).
fn is_branch_return_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("return") {
        return false;
    }
    match trimmed.as_bytes().get("return".len()) {
        // Bare `return` (JS allows it; TS sources use `return true/false;`).
        None => true,
        // `return` must end on a word boundary (`returnfoo();` is not one).
        Some(b) if b.is_ascii_alphanumeric() || *b == b'_' => false,
        _ => true,
    }
}

/// Scan explicit file/line slices for Rule 5 violations
/// (TS: `findViolations`). Each answer line pairs with the first
/// `return false;` within [`LOOKAHEAD_LINES`] lines after it; a different
/// `return` first ends the branch and stops that answer's scan; input order
/// is preserved.
pub fn find_violations(files: &[(&str, &[&str])]) -> Vec<CallbackAnswerViolation> {
    let mut out = Vec::new();
    for (file, lines) in files {
        for (index, line) in lines.iter().enumerate() {
            if !is_answer_line(line) {
                continue;
            }
            for offset in 1..=LOOKAHEAD_LINES {
                match lines.get(index + offset) {
                    Some(candidate) if is_return_false_line(candidate) => {
                        out.push(CallbackAnswerViolation {
                            file: file.to_string(),
                            answer_line: index + 1,
                            return_line: index + offset + 1,
                        });
                        break;
                    }
                    // The branch already returned correctly (`return true;`);
                    // a later `return false;` is unpaired with this answer.
                    Some(candidate) if is_branch_return_line(candidate) => break,
                    _ => {}
                }
            }
        }
    }
    out
}

/// Render one violation exactly like the TS assertion maps it
/// (TS: `` `${file}: answered at line ${answerLine}, returns false at line
/// ${returnLine}` ``).
pub fn format_violation(violation: &CallbackAnswerViolation) -> String {
    format!(
        "{}: answered at line {}, returns false at line {}",
        violation.file, violation.answer_line, violation.return_line
    )
}
