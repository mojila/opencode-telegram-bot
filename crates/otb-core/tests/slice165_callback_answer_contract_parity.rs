//! Slice 165 parity: callback answer-contract gates.
//!
//! Mirrors the pure TS contract in
//! `tests/bot/callbacks/callback-answer-contract.test.ts`
//! (`ANSWER_PATTERN`, `RETURN_FALSE_PATTERN`, `LOOKAHEAD_LINES`,
//! `findViolations` + the violation message format), over
//! `src/bot/callbacks/*.ts`:
//! - an answer call (`answerCallbackQuery` / `notify` / `alert` / `failure` /
//!   `cancelMenu` / `cancelPrompt` + `(`) followed within 3 lines by
//!   `return false;` is one Rule 5 violation (answering then returning
//!   `false` double-answers, which Telegram rejects with a 400)
//! - clean handler sources map to `[]`, exactly like the TS assertion
//!
//! Already covered elsewhere and NOT re-tested here:
//! - per-callback handlers / routers (their gates owners)
//! - filesystem reads (`fs.readdirSync` / `readFileSync` stay with the
//!   caller; fixtures here pass explicit file/line slices)
//! - Telegram `answerCallbackQuery` delivery (callers)

use otb_core::callback_answer_contract_gates::*;

// ---- LOOKAHEAD_LINES (1) ----

#[test]
fn callback_contract_lookahead_is_three_lines_parity() {
    assert_eq!(LOOKAHEAD_LINES, 3);
}

// ---- ANSWER_CALL_NAMES (1) ----

#[test]
fn callback_contract_answer_names_parity() {
    assert_eq!(
        ANSWER_CALL_NAMES,
        [
            "answerCallbackQuery",
            "notify",
            "alert",
            "failure",
            "cancelMenu",
            "cancelPrompt",
        ]
    );
}

// ---- is_answer_line (3) ----

#[test]
fn callback_contract_answer_line_matches_each_name_parity() {
    assert!(is_answer_line("await answerCallbackQuery(ctx);"));
    assert!(is_answer_line("await ctx.notify('saved');"));
    assert!(is_answer_line("await ctx.alert(text);"));
    assert!(is_answer_line("await ctx.failure(err);"));
    assert!(is_answer_line("await cancelMenu(ctx);"));
    assert!(is_answer_line("await cancelPrompt(ctx, id);"));
}

#[test]
fn callback_contract_answer_line_allows_space_before_paren_parity() {
    assert!(is_answer_line("notify (ctx);"));
    assert!(is_answer_line("cancelMenu\t(ctx);"));
}

#[test]
fn callback_contract_answer_line_word_boundary_parity() {
    // Preceded by a word char: no `\b`, so no match.
    assert!(!is_answer_line("myanswerCallbackQuery(ctx);"));
    assert!(!is_answer_line("xnotify(ctx);"));
    // Name must be followed by `\s*\(`: a longer identifier is no match.
    assert!(!is_answer_line("alertBox(ctx);"));
    assert!(!is_answer_line("cancelMenuBar(ctx);"));
    // Not one of the six names.
    assert!(!is_answer_line("answerCallbackQuer(ctx);"));
    assert!(!is_answer_line(""));
    assert!(!is_answer_line("return false;"));
}

// ---- is_return_false_line (2) ----

#[test]
fn callback_contract_return_false_line_parity() {
    assert!(is_return_false_line("return false;"));
    assert!(is_return_false_line("  return false;"));
    assert!(is_return_false_line("\treturn false;"));
    // No `$` anchor in TS: trailing content still matches.
    assert!(is_return_false_line("return false; // handled below"));
}

#[test]
fn callback_contract_return_false_line_rejects_parity() {
    assert!(!is_return_false_line("return false"));
    assert!(!is_return_false_line("return true;"));
    assert!(!is_return_false_line("xreturn false;"));
    assert!(!is_return_false_line(""));
    assert!(!is_return_false_line("await answerCallbackQuery(ctx);"));
}

// ---- find_violations (3) ----

#[test]
fn callback_contract_flags_return_within_lookahead_parity() {
    let one_after = find_violations(&[(
        "handler.ts",
        &["await answerCallbackQuery(ctx);", "return false;"] as &[&str],
    )]);
    assert_eq!(
        one_after,
        vec![CallbackAnswerViolation {
            file: "handler.ts".to_string(),
            answer_line: 1,
            return_line: 2,
        }]
    );

    // Boundary: three lines after still belongs to the same branch.
    let three_after = find_violations(&[(
        "handler.ts",
        &[
            "await notify(ctx);",
            "const ok = true;",
            "if (ok) {",
            "return false;",
        ] as &[&str],
    )]);
    assert_eq!(three_after.len(), 1);
    assert_eq!(three_after[0].answer_line, 1);
    assert_eq!(three_after[0].return_line, 4);

    // Four lines after is beyond the lookahead: no violation.
    let four_after = find_violations(&[(
        "handler.ts",
        &[
            "await notify(ctx);",
            "const a = 1;",
            "const b = 2;",
            "const c = 3;",
            "return false;",
        ] as &[&str],
    )]);
    assert_eq!(four_after, vec![]);
}

#[test]
fn callback_contract_ignores_unpaired_lines_parity() {
    // Answer with no rejection, and rejection with no answer.
    let clean = find_violations(&[(
        "clean.ts",
        &[
            "await answerCallbackQuery(ctx);",
            "return true;",
            "return false;",
        ] as &[&str],
    )]);
    assert_eq!(clean, vec![]);

    // Only the first `return false;` pairs with one answer.
    let single = find_violations(&[(
        "handler.ts",
        &[
            "await alert(ctx);",
            "return false;",
            "return false;",
        ] as &[&str],
    )]);
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].return_line, 2);
}

#[test]
fn callback_contract_aggregates_files_in_order_parity() {
    let violations = find_violations(&[
        (
            "first.ts",
            &["await notify(ctx);", "return false;", "const x = 1;"] as &[&str],
        ),
        ("second.ts", &["const y = 2;", "return true;"] as &[&str]),
        (
            "third.ts",
            &["await cancelPrompt(ctx);", "x;", "y;", "return false;"] as &[&str],
        ),
    ]);
    assert_eq!(
        violations
            .iter()
            .map(|violation| violation.file.clone())
            .collect::<Vec<_>>(),
        vec!["first.ts".to_string(), "third.ts".to_string()]
    );
    assert_eq!(violations[0].answer_line, 1);
    assert_eq!(violations[1].answer_line, 1);
    assert_eq!(violations[1].return_line, 4);
}

// ---- format_violation (1) ----

#[test]
fn callback_contract_violation_message_format_parity() {
    // TS maps violations to `` `${file}: answered at line ${answerLine},
    // returns false at line ${returnLine}` `` and expects `[]` on clean
    // sources; a violating source renders exactly one such line.
    let clean: Vec<String> = find_violations(&[(
        "clean.ts",
        &["await answerCallbackQuery(ctx);", "return true;"] as &[&str],
    )])
    .iter()
    .map(format_violation)
    .collect();
    assert_eq!(clean, Vec::<String>::new());

    let violating: Vec<String> = find_violations(&[(
        "handler.ts",
        &["const x = 0;", "await alert(ctx);", "return false;"] as &[&str],
    )])
    .iter()
    .map(format_violation)
    .collect();
    assert_eq!(
        violating,
        vec!["handler.ts: answered at line 2, returns false at line 3".to_string()]
    );
}
