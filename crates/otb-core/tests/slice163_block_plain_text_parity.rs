//! Slice 163 parity: block plain-text projection gates.
//!
//! Mirrors the pure TS contract in
//! `src/bot/render/block-plain-text.ts` (`extractInlinePlainText`,
//! `buildAlignedTableText`, `toBlockPlainText`), asserted by
//! `tests/bot/render/block-plain-text.test.ts`:
//! - inline extraction drops formatting and keeps text content
//! - table column padding, divider row, ragged-row normalization
//! - paragraph/heading bare text, quote `"> "` prefixes, bullet and
//!   ordered markers, inline-span text, task-state markers, nested-list
//!   indent, code as-is, aligned tables, rule text, plain passthrough
//!
//! Already covered elsewhere and NOT re-tested here:
//! - lossy inline/table helpers (`crate::keyboards` owner),
//!   `PLAIN_MAX_PART_CHARS` chunk budgets (`crate::keyboards` owner)
//! - markdown parsing (`parseTelegramBlocks` stays with the caller;
//!   fixtures here are already-parsed block trees, so the `"\n\n"` join
//!   between sibling top-level blocks in the TS `plainTextOf` helper is
//!   out of scope — each block is asserted on its own)
//! - stringly plain projections for render budgets
//!   (`block_split_gates` / `rich_blocks` owners)

use otb_core::block_plain_text_gates::*;
use PlainBlock::*;
use PlainInline as I;

fn text(value: &str) -> PlainInline {
    I::Text(value.to_string())
}

fn para(value: &str) -> PlainBlock {
    Paragraph(vec![text(value)])
}

fn item(text_value: &str, checked: Option<bool>) -> PlainListItem {
    PlainListItem { blocks: vec![para(text_value)], checked }
}

// ---- constants (1) ----

#[test]
fn block_plain_rule_text_parity() {
    assert_eq!(RULE_TEXT, "─".repeat(10));
    assert_eq!(RULE_TEXT.chars().count(), 10);
}

// ---- extractInlinePlainText (1) ----

#[test]
fn block_plain_extract_drops_formatting_parity() {
    let nodes = vec![
        text("a "),
        I::Bold(vec![I::Italic(vec![text("b")])]),
        I::Code("c".to_string()),
        I::Link { text: vec![text("d")], url: "https://example.com".to_string() },
    ];
    assert_eq!(extract_plain_inline_text(&nodes), "a bcd");
}

// ---- buildAlignedTableText (2) ----

#[test]
fn block_plain_table_pads_columns_and_divider_parity() {
    let rows = vec![
        vec!["id".to_string(), "name".to_string()],
        vec!["1".to_string(), "long value".to_string()],
    ];
    assert_eq!(
        build_plain_aligned_table(&rows),
        ["id | name      ", "---|-----------", "1  | long value"].join("\n")
    );
}

#[test]
fn block_plain_table_normalizes_ragged_rows_parity() {
    let rows = vec![vec!["a".to_string(), "b".to_string()], vec!["c".to_string()]];
    assert_eq!(
        build_plain_aligned_table(&rows),
        ["a | b", "--|--", "c |  "].join("\n")
    );
}

// ---- toBlockPlainText (10) ----

#[test]
fn block_plain_paragraph_and_heading_bare_text_parity() {
    assert_eq!(to_block_plain_text(&para("Some text")), "Some text");
    assert_eq!(
        to_block_plain_text(&Heading { level: 2, inlines: vec![text("Title")] }),
        "Title"
    );
}

#[test]
fn block_plain_quote_prefixes_lines_parity() {
    let block = Blockquote(vec![para("first"), para("second")]);
    assert_eq!(to_block_plain_text(&block), "> first\n> second");
}

#[test]
fn block_plain_bullet_and_ordered_markers_parity() {
    let bullets = List {
        ordered: false,
        start: None,
        items: vec![item("a", None), item("b", None)],
    };
    assert_eq!(to_block_plain_text(&bullets), "- a\n- b");
    let ordered = List {
        ordered: true,
        start: None,
        items: vec![item("a", None), item("b", None)],
    };
    assert_eq!(to_block_plain_text(&ordered), "1. a\n2. b");
}

#[test]
fn block_plain_keeps_inline_span_text_parity() {
    let block = Paragraph(vec![
        text("H"),
        I::Subscript(vec![text("2")]),
        text("O, м"),
        I::Superscript(vec![text("2")]),
        text(", "),
        I::Marked(vec![text("note")]),
        text(", "),
        I::Code("Ctrl".to_string()),
    ]);
    assert_eq!(to_block_plain_text(&block), "H2O, м2, note, Ctrl");
}

#[test]
fn block_plain_task_markers_parity() {
    let tasks = List {
        ordered: false,
        start: None,
        items: vec![item("done", Some(true)), item("open", Some(false))],
    };
    assert_eq!(to_block_plain_text(&tasks), "✅ done\n🔲 open");
}

#[test]
fn block_plain_nested_list_indent_parity() {
    let nested = List {
        ordered: true,
        start: None,
        items: vec![item("child", None)],
    };
    let parent = PlainListItem {
        blocks: vec![para("parent"), nested],
        checked: None,
    };
    let block = List {
        ordered: true,
        start: None,
        items: vec![parent, item("next", None)],
    };
    assert_eq!(
        to_block_plain_text(&block),
        ["1. parent", "   1. child", "2. next"].join("\n")
    );
}

#[test]
fn block_plain_code_kept_as_is_parity() {
    let block = Code { language: Some("js".to_string()), text: "const a = 1;".to_string() };
    assert_eq!(to_block_plain_text(&block), "const a = 1;");
}

#[test]
fn block_plain_table_block_aligned_parity() {
    let block = Table {
        rows: vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ],
    };
    assert_eq!(to_block_plain_text(&block), ["a | b", "--|--", "1 | 2"].join("\n"));
}

#[test]
fn block_plain_rule_parity() {
    assert_eq!(to_block_plain_text(&Rule), RULE_TEXT);
}

#[test]
fn block_plain_passthrough_parity() {
    assert_eq!(to_block_plain_text(&Plain("## raw".to_string())), "## raw");
}
