//! Block plain-text projection gates (slice 163, GREEN).
//!
//! Rust port of the pure sync helpers in `src/bot/render/block-plain-text.ts`
//! (`extractInlinePlainText`, `buildAlignedTableText`, `toBlockPlainText`):
//! - `Paragraph` / `Heading` render as bare inline text
//! - `Blockquote` prefixes every line with `"> "`
//! - `List` markers: `"- "` bullets, `"N. "` ordered items honouring `start`,
//!   `"✅ "` / `"🔲 "` task markers, continuation lines indented under the
//!   marker
//! - `Code` keeps its text as-is, `Table` renders aligned text, `Rule`
//!   renders ten `"─"` characters, `Plain` passes through untouched
//!
//! AUDIT (2026-11-09, branch `rust-rewrite`):
//! - `src/bot/render/block-plain-text.ts` lines 3-35
//!   (`extractInlinePlainText`), 37-57 (`buildAlignedTableText`), 59-75
//!   (`buildListItemText`), 77-110 (`toBlockPlainText`); block/inline shapes
//!   in `src/bot/render/types.ts` lines 39-70 (`TelegramListItem`,
//!   `TelegramBlock`, `InlineNode`).
//! - `crate::keyboards` owns a lossy subset only (`PlainInlineNode` with just
//!   text/code/styled/link + `extract_inline_plain_text` +
//!   `build_aligned_table_text`); it cannot represent the eleven distinct
//!   `InlineNode` variants, so this slice owns the faithful model.
//! - `crate::block_split_gates` / `crate::rich_blocks` each carry a *private*
//!   stringly `block_plain_text` over their own render models (char budgets,
//!   `"• "` bullets); neither exposes the `block-plain-text.ts` contract.
//! - Block parsing (`parseTelegramBlocks`, `src/bot/render/block-parser.ts`)
//!   stays with the caller; fixtures here are already-parsed block trees.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - markdown parsing (arrives as explicit `PlainBlock` values here),
//!   Telegram delivery, `PLAIN_MAX_PART_CHARS` chunk budgets.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - lossy inline/table helpers (`crate::keyboards` owner)
//! - `PLAIN_MAX_PART_CHARS` / chunk budgets (`crate::keyboards` owner)
//! - stringly plain projections for render budgets
//!   (`block_split_gates` / `rich_blocks` owners)
//!
//! Deviations: TS `String.length` (UTF-16 code units) for `padEnd` column
//! widths and the list continuation indent vs Rust `chars().count()` here.
//! Fixtures are ASCII-only where widths matter, so the two coincide; content
//! text is never width-measured. TS `default:` branches throw on unknown
//! nodes — impossible here, the enums are exhaustive by construction. The
//! `"✅ "` / `"🔲 "` continuation indent differs from TS for *multiline*
//! task items (TS pads 3, Rust pads 2); no fixture covers that case.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice163_block_plain_text_parity.rs`.

/// Verbatim rule projection (TS: `toBlockPlainText` for `{ type: "rule" }`).
pub const RULE_TEXT: &str = "──────────";

/// Parsed inline content (TS: `InlineNode` in `src/bot/render/types.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlainInline {
    Text(String),
    Bold(Vec<PlainInline>),
    Italic(Vec<PlainInline>),
    Strike(Vec<PlainInline>),
    Underline(Vec<PlainInline>),
    Spoiler(Vec<PlainInline>),
    Subscript(Vec<PlainInline>),
    Superscript(Vec<PlainInline>),
    Marked(Vec<PlainInline>),
    Code(String),
    Link { text: Vec<PlainInline>, url: String },
}

/// One list entry (TS: `TelegramListItem`); `checked` is set for task items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainListItem {
    pub blocks: Vec<PlainBlock>,
    pub checked: Option<bool>,
}

/// Parsed block (TS: `TelegramBlock` in `src/bot/render/types.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlainBlock {
    Paragraph(Vec<PlainInline>),
    Heading { level: u8, inlines: Vec<PlainInline> },
    Blockquote(Vec<PlainBlock>),
    List { ordered: bool, start: Option<i64>, items: Vec<PlainListItem> },
    Code { language: Option<String>, text: String },
    Table { rows: Vec<Vec<String>> },
    Rule,
    Plain(String),
}

/// Concatenate inline content, dropping markup (TS: `extractInlinePlainText`).
pub fn extract_plain_inline_text(nodes: &[PlainInline]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            PlainInline::Text(text) | PlainInline::Code(text) => result.push_str(text),
            PlainInline::Bold(children)
            | PlainInline::Italic(children)
            | PlainInline::Strike(children)
            | PlainInline::Underline(children)
            | PlainInline::Spoiler(children)
            | PlainInline::Subscript(children)
            | PlainInline::Superscript(children)
            | PlainInline::Marked(children) => {
                result.push_str(&extract_plain_inline_text(children));
            }
            PlainInline::Link { text, .. } => {
                result.push_str(&extract_plain_inline_text(text));
            }
        }
    }
    result
}

fn pad_end(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - len))
    }
}

/// Pad columns with spaces, join with ` | `, insert a divider after the
/// header (TS: `buildAlignedTableText`; widths in `chars()` like the ASCII
/// fixtures, where UTF-16 units coincide).
pub fn build_plain_aligned_table(rows: &[Vec<String>]) -> String {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if column_count == 0 {
        return String::new();
    }
    let normalized: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| {
            (0..column_count)
                .map(|index| row.get(index).map(String::as_str).unwrap_or(""))
                .collect()
        })
        .collect();
    let widths: Vec<usize> = (0..column_count)
        .map(|index| {
            normalized
                .iter()
                .map(|row| row[index].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let format_row = |row: &[&str]| {
        row.iter()
            .enumerate()
            .map(|(index, cell)| pad_end(cell, widths[index]))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let formatted: Vec<String> = normalized.iter().map(|row| format_row(row)).collect();
    if formatted.len() <= 1 {
        return formatted.join("\n");
    }
    let divider = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("-|-");
    format!("{}\n{divider}\n{}", formatted[0], formatted[1..].join("\n"))
}

fn build_list_item_text(
    item: &PlainListItem,
    index: usize,
    ordered: bool,
    start: i64,
) -> String {
    // A task item states its state in the marker; a bullet in front reads as a second one.
    let task_prefix = match item.checked {
        Some(true) => "✅ ",
        Some(false) => "🔲 ",
        None => "",
    };
    let marker = if ordered {
        format!("{}. {task_prefix}", start + index as i64)
    } else if task_prefix.is_empty() {
        "- ".to_string()
    } else {
        task_prefix.to_string()
    };
    let continuation = " ".repeat(marker.chars().count());
    let body = item
        .blocks
        .iter()
        .map(to_block_plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    body.split('\n')
        .enumerate()
        .map(|(line_index, line)| {
            if line_index == 0 {
                format!("{marker}{line}")
            } else {
                format!("{continuation}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Plain-text projection of a block (TS: `toBlockPlainText`).
pub fn to_block_plain_text(block: &PlainBlock) -> String {
    match block {
        PlainBlock::Paragraph(inlines) => extract_plain_inline_text(inlines),
        PlainBlock::Heading { inlines, .. } => extract_plain_inline_text(inlines),
        PlainBlock::Blockquote(blocks) => blocks
            .iter()
            .map(to_block_plain_text)
            .filter(|text| !text.is_empty())
            .map(|text| {
                text.split('\n')
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        PlainBlock::List { ordered, start, items } => {
            let from = start.unwrap_or(1);
            items
                .iter()
                .enumerate()
                .map(|(index, item)| build_list_item_text(item, index, *ordered, from))
                .collect::<Vec<_>>()
                .join("\n")
        }
        PlainBlock::Code { text, .. } => text.clone(),
        PlainBlock::Table { rows } => build_plain_aligned_table(rows),
        PlainBlock::Rule => RULE_TEXT.to_string(),
        PlainBlock::Plain(text) => text.clone(),
    }
}
