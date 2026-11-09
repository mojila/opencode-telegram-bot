//! Parity port of the markdown-normalizer pure gates (slice 43, RED).
//!
//! Mirrors representative cases from:
//! - `tests/bot/render/markdown-normalizer.test.ts` (rendering headings,
//!   quotes, checklists, rules; quote continuation; blank-line reset;
//!   fenced code untouched; malformed emphasis passthrough; parser-safe
//!   normalization)
//! - `src/bot/render/markdown-normalizer.ts` (every branch: fence toggle,
//!   blank reset, heading levels/indent limits, rule variants, quote
//!   normalization, checklist markers, in-quote continuation, preserve
//!   mode)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::markdown_normalizer::{
    normalize_markdown_for_telegram_block_parsing, normalize_markdown_for_telegram_rendering,
};

// --- rendering: headings (01-08) ---

#[test]
fn slice43_01_render_h1_becomes_bold() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("# Main heading"),
        "**Main heading**"
    );
}

#[test]
fn slice43_02_render_all_heading_levels_become_bold() {
    let input = ["# A", "## B", "### C", "#### D", "##### E", "###### F"].join("\n");
    let expected = ["**A**", "**B**", "**C**", "**D**", "**E**", "**F**"].join("\n");
    assert_eq!(normalize_markdown_for_telegram_rendering(&input), expected);
}

#[test]
fn slice43_03_render_heading_trailing_whitespace_stripped() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("## Title   "),
        "**Title**"
    );
}

#[test]
fn slice43_04_render_heading_with_up_to_three_leading_spaces() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("   ### Indented"),
        "**Indented**"
    );
}

#[test]
fn slice43_05_render_four_leading_spaces_not_a_heading() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("    # Code"),
        "    # Code"
    );
}

#[test]
fn slice43_06_render_seven_hashes_not_a_heading() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("####### Too many"),
        "####### Too many"
    );
}

#[test]
fn slice43_07_render_hash_without_space_not_a_heading() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("#nospace"),
        "#nospace"
    );
}

#[test]
fn slice43_08_render_bare_hash_with_space_only_untouched() {
    assert_eq!(normalize_markdown_for_telegram_rendering("# "), "# ");
}

// --- rendering: horizontal rules (09-11) ---

#[test]
fn slice43_09_render_triple_dash_rule_becomes_box_line() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("---"),
        "──────────"
    );
}

#[test]
fn slice43_10_render_rule_variants_become_box_line() {
    for rule in ["***", "___", "- - -", "* * *", "  ---  "] {
        assert_eq!(
            normalize_markdown_for_telegram_rendering(rule),
            "──────────",
            "rule {rule:?} should normalize"
        );
    }
}

#[test]
fn slice43_11_render_rule_lookalikes_untouched() {
    assert_eq!(normalize_markdown_for_telegram_rendering("---x"), "---x");
    assert_eq!(
        normalize_markdown_for_telegram_rendering("**bold**"),
        "**bold**"
    );
}

// --- rendering: checklists (12-16, 37, 39) ---

#[test]
fn slice43_12_render_dash_checklist_markers() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("- [ ] Open"),
        "🔲 Open"
    );
    assert_eq!(
        normalize_markdown_for_telegram_rendering("- [x] Done"),
        "✅ Done"
    );
    assert_eq!(
        normalize_markdown_for_telegram_rendering("- [X] Done"),
        "✅ Done"
    );
}

#[test]
fn slice43_13_render_plus_star_and_numbered_checklist_markers() {
    assert_eq!(normalize_markdown_for_telegram_rendering("+ [ ] a"), "🔲 a");
    assert_eq!(normalize_markdown_for_telegram_rendering("* [x] b"), "✅ b");
    assert_eq!(
        normalize_markdown_for_telegram_rendering("1. [ ] c"),
        "🔲 c"
    );
    assert_eq!(
        normalize_markdown_for_telegram_rendering("12. [X] d"),
        "✅ d"
    );
}

#[test]
fn slice43_14_render_indented_checklist_keeps_indent() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("  - [ ] task"),
        "  🔲 task"
    );
}

#[test]
fn slice43_15_render_empty_checkbox_without_text_untouched() {
    assert_eq!(normalize_markdown_for_telegram_rendering("- [ ]"), "- [ ]");
}

#[test]
fn slice43_16_render_invalid_checkbox_state_untouched() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("- [y] task"),
        "- [y] task"
    );
}

#[test]
fn slice43_37_render_numbered_checked_task() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("1. [x] Done"),
        "✅ Done"
    );
}

#[test]
fn slice43_39_render_checklist_without_space_after_bracket_untouched() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("- [ ]task"),
        "- [ ]task"
    );
}

// --- rendering: quotes (17-25, 33-34, 38) ---

#[test]
fn slice43_17_render_quote_spacing_normalized() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> quoted"),
        "> quoted"
    );
    assert_eq!(
        normalize_markdown_for_telegram_rendering(">quoted"),
        "> quoted"
    );
}

#[test]
fn slice43_18_render_bare_continuation_line_gains_quote_prefix() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> quoted line\nQuote continues"),
        "> quoted line\n> Quote continues"
    );
}

#[test]
fn slice43_19_render_blank_line_resets_quote_mode() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> quoted line\n\nPlain line"),
        "> quoted line\n\nPlain line"
    );
}

#[test]
fn slice43_20_render_whitespace_only_line_resets_quote_and_is_kept() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> q\n   \nplain"),
        "> q\n   \nplain"
    );
}

#[test]
fn slice43_21_render_heading_resets_quote_mode() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> q\n# H\nplain"),
        "> q\n**H**\nplain"
    );
}

#[test]
fn slice43_22_render_rule_resets_quote_mode() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> q\n---\nplain"),
        "> q\n──────────\nplain"
    );
}

#[test]
fn slice43_23_render_checklist_inside_quote_normalized() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> - [ ] Review"),
        "> 🔲 Review"
    );
}

#[test]
fn slice43_24_render_checked_task_inside_quote() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> - [x] Done"),
        "> ✅ Done"
    );
}

#[test]
fn slice43_25_render_checklist_after_quote_joins_quote_normalized() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> quoted\n- [x] Outside quote"),
        "> quoted\n> ✅ Outside quote"
    );
}

#[test]
fn slice43_33_render_bare_gt_becomes_prefixed_empty_quote() {
    assert_eq!(normalize_markdown_for_telegram_rendering(">"), "> ");
}

#[test]
fn slice43_34_render_nested_quote_preserved() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> > nested"),
        "> > nested"
    );
}

#[test]
fn slice43_38_render_quote_extra_leading_spaces_collapsed() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering(">   hello"),
        "> hello"
    );
}

// --- rendering: code fences (26-29) ---

#[test]
fn slice43_26_render_fenced_code_untouched() {
    let input = [
        "```ts",
        "# Not a heading",
        "> not a quote",
        "- [ ] not a checklist",
        "---",
        "```",
    ]
    .join("\n");
    assert_eq!(normalize_markdown_for_telegram_rendering(&input), input);
}

#[test]
fn slice43_27_render_indented_fence_toggles_code_mode() {
    let input = ["  ```ts", "# H", "  ```", "after"].join("\n");
    assert_eq!(normalize_markdown_for_telegram_rendering(&input), input);
}

#[test]
fn slice43_28_render_unclosed_fence_keeps_rest_untouched() {
    let input = ["```", "# H", "> q", "- [ ] t"].join("\n");
    assert_eq!(normalize_markdown_for_telegram_rendering(&input), input);
}

#[test]
fn slice43_29_render_fence_resets_quote_mode() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("> q\n```\nplain"),
        "> q\n```\nplain"
    );
}

// --- rendering: misc passthrough (30-32, 35-36, 40) ---

#[test]
fn slice43_30_render_malformed_emphasis_passthrough() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("*text: *value**"),
        "*text: *value**"
    );
}

#[test]
fn slice43_31_render_empty_string_stays_empty() {
    assert_eq!(normalize_markdown_for_telegram_rendering(""), "");
}

#[test]
fn slice43_32_render_plain_text_untouched() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("just plain\nsecond line"),
        "just plain\nsecond line"
    );
}

#[test]
fn slice43_35_render_combined_doc_example() {
    let input = [
        "# Main heading",
        "",
        "> quoted line",
        "Quote continues",
        "",
        "- [ ] Open task",
        "- [x] Done task",
        "1. [ ] Numbered task",
        "",
        "---",
    ]
    .join("\n");
    let expected = [
        "**Main heading**",
        "",
        "> quoted line",
        "> Quote continues",
        "",
        "🔲 Open task",
        "✅ Done task",
        "🔲 Numbered task",
        "",
        "──────────",
    ]
    .join("\n");
    assert_eq!(normalize_markdown_for_telegram_rendering(&input), expected);
}

#[test]
fn slice43_36_render_quote_checklist_doc_example() {
    let input = ["> - [ ] Review", "Follow-up line", "- [x] Outside quote"].join("\n");
    let expected = ["> 🔲 Review", "> Follow-up line", "> ✅ Outside quote"].join("\n");
    assert_eq!(normalize_markdown_for_telegram_rendering(&input), expected);
}

#[test]
fn slice43_40_render_rule_with_leading_spaces() {
    assert_eq!(
        normalize_markdown_for_telegram_rendering("  ---"),
        "──────────"
    );
}

// --- block-parsing mode (41-48) ---

#[test]
fn slice43_41_parse_heading_markup_preserved() {
    assert_eq!(
        normalize_markdown_for_telegram_block_parsing("# Main heading"),
        "# Main heading"
    );
}

#[test]
fn slice43_42_parse_rule_markup_preserved() {
    assert_eq!(normalize_markdown_for_telegram_block_parsing("---"), "---");
}

#[test]
fn slice43_43_parse_checklist_markup_preserved() {
    assert_eq!(
        normalize_markdown_for_telegram_block_parsing("- [x] Done task"),
        "- [x] Done task"
    );
}

#[test]
fn slice43_44_parse_quote_continuation_still_prefixed() {
    assert_eq!(
        normalize_markdown_for_telegram_block_parsing("> q\ncontinued"),
        "> q\n> continued"
    );
}

#[test]
fn slice43_45_parse_checklist_after_quote_gets_prefix_but_keeps_markup() {
    assert_eq!(
        normalize_markdown_for_telegram_block_parsing("> quoted\n- [ ] task"),
        "> quoted\n> - [ ] task"
    );
}

#[test]
fn slice43_46_parse_checklist_inside_quote_preserved() {
    assert_eq!(
        normalize_markdown_for_telegram_block_parsing("> - [ ] Review"),
        "> - [ ] Review"
    );
}

#[test]
fn slice43_47_parse_fenced_code_untouched() {
    let input = ["```ts", "# Not a heading", "- [ ] not a checklist", "```"].join("\n");
    assert_eq!(normalize_markdown_for_telegram_block_parsing(&input), input);
}

#[test]
fn slice43_48_parse_combined_doc_example() {
    let input = [
        "# Main heading",
        "",
        "> - [ ] Review",
        "Follow-up line",
        "",
        "- [x] Done task",
        "",
        "---",
    ]
    .join("\n");
    let expected = [
        "# Main heading",
        "",
        "> - [ ] Review",
        "> Follow-up line",
        "",
        "- [x] Done task",
        "",
        "---",
    ]
    .join("\n");
    assert_eq!(
        normalize_markdown_for_telegram_block_parsing(&input),
        expected
    );
}
