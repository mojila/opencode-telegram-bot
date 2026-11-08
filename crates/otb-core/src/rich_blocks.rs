//! Rich-block conversion pure gates (slice 44).
//!
//! Rust port of the pure block conversion in:
//! - `src/bot/render/rich-blocks.ts`
//!   (`toRichText`, `isTextRenderedList`, `textListMarker`,
//!   `liftTextListContent`, `toRichBlock`, `createParagraphBlock`,
//!   `countRichBlocks`, `countRichChars`, `RICH_TABLE_MAX_COLUMNS`)
//! - `src/bot/render/part-signature.ts`
//!   (`getTelegramRenderedPartSignature`)
//! - `src/bot/render/limits.ts` (budgets, re-exported for reference)
//!
//! Pure sync data-transform logic only: no Telegram SDK, no timers, no I/O.
//!
//! Parity notes / conventions for the step-2 port:
//! - TS `RichText` is `string | RichText[] | object`. It is modelled here as
//!   [`RichText`]: `Str` for a plain string, `Arr` for an array, `Styled` for
//!   `{ type, text }` wrappers, `Url` for `{ type: "url", text, url }`.
//!   Empty text nodes contribute nothing; zero parts collapse to `Str("")`
//!   and a single part collapses to itself (TS: `toRichText`).
//! - TS `strike` maps to the `strikethrough` rich kind.
//! - Heading size clamps to `1..=6` via
//!   `Math.min(6, Math.max(1, Math.floor(level)))`.
//! - Tables wider than [`RICH_TABLE_MAX_COLUMNS`] columns degrade to a `pre`
//!   block holding the aligned plain-text table; a table with zero rows
//!   becomes an empty paragraph. Header cells carry `is_header: true`, every
//!   cell defaults to `align: "left"` and `valign: "top"`, and tables are
//!   `is_bordered: true`.
//! - Signature JSON encoding (TS `JSON.stringify`): entities encode as
//!   `{"type":<kind>,"offset":<n>,"length":<n>}` joined by `,` inside `[]`;
//!   blocks encode with `type` first, e.g. a paragraph as
//!   `{"type":"paragraph","text":"..."}` and a divider as
//!   `{"type":"divider"}`. The signature is
//!   `plain\n<entities>\n<fallbackText>` for `source == "plain"` or empty
//!   blocks, else `blocks\n<blocksJson>`.

/// Telegram rejects rich tables wider than this; such tables fall back to
/// preformatted text (TS: `RICH_TABLE_MAX_COLUMNS`).
pub const RICH_TABLE_MAX_COLUMNS: usize = 20;

/// Budget for rich content delivered as native blocks
/// (TS: `DEFAULT_MAX_PART_CHARS` in `src/bot/render/limits.ts`).
pub const DEFAULT_MAX_PART_CHARS: usize = 32_000;

/// Block-count budget for rich content delivered as native blocks
/// (TS: `DEFAULT_MAX_PART_BLOCKS` in `src/bot/render/limits.ts`).
pub const DEFAULT_MAX_PART_BLOCKS: usize = 480;

/// Budget for text delivered as a plain Telegram message
/// (TS: `PLAIN_MAX_PART_CHARS` in `src/bot/render/limits.ts`).
pub const PLAIN_MAX_PART_CHARS: usize = 3_800;

/// Hard Telegram limit for text messages
/// (TS: `TELEGRAM_TEXT_MESSAGE_LIMIT` in `src/bot/render/limits.ts`).
pub const TELEGRAM_TEXT_MESSAGE_LIMIT: usize = 4_096;

/// Parsed inline content (TS: `InlineNode` in `src/bot/render/types.ts`).
#[derive(Debug, Clone, PartialEq)]
pub enum InlineNode {
    Text(String),
    Bold(Vec<InlineNode>),
    Italic(Vec<InlineNode>),
    Strike(Vec<InlineNode>),
    Underline(Vec<InlineNode>),
    Spoiler(Vec<InlineNode>),
    Subscript(Vec<InlineNode>),
    Superscript(Vec<InlineNode>),
    Marked(Vec<InlineNode>),
    Code(String),
    Link { text: Vec<InlineNode>, url: String },
}

/// Column alignment of a markdown table (TS: `TableColumnAlign`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAlign {
    Left,
    Center,
    Right,
}

/// One entry of a list (TS: `TelegramListItem`).
#[derive(Debug, Clone, PartialEq)]
pub struct TelegramListItem {
    pub blocks: Vec<TelegramBlock>,
    pub checked: Option<bool>,
}

/// A parsed content block (TS: `TelegramBlock`).
#[derive(Debug, Clone, PartialEq)]
pub enum TelegramBlock {
    Paragraph { inlines: Vec<InlineNode> },
    Heading { level: i64, inlines: Vec<InlineNode> },
    Blockquote { blocks: Vec<TelegramBlock> },
    List { ordered: bool, items: Vec<TelegramListItem>, start: Option<u64> },
    Code { language: Option<String>, text: String },
    Table { rows: Vec<Vec<String>>, align: Option<Vec<TableAlign>> },
    Rule,
    Plain { text: String },
}

/// Rich inline content (TS: `RichText` from `grammy/types`).
#[derive(Debug, Clone, PartialEq)]
pub enum RichText {
    /// A plain string payload.
    Str(String),
    /// An array payload of rich parts.
    Arr(Vec<RichText>),
    /// A `{ type, text }` styled wrapper (`bold`, `italic`,
    /// `strikethrough`, `underline`, `spoiler`, `subscript`,
    /// `superscript`, `marked`, `code`, ...).
    Styled { kind: String, text: Box<RichText> },
    /// A `{ type: "url", text, url }` link.
    Url { text: Box<RichText>, url: String },
}

/// One cell of a native rich table.
#[derive(Debug, Clone, PartialEq)]
pub struct RichTableCell {
    pub text: String,
    pub is_header: bool,
    pub align: TableAlign,
}

/// One item of a natively rendered rich list.
#[derive(Debug, Clone, PartialEq)]
pub struct RichListItem {
    pub blocks: Vec<RichBlock>,
}

/// A native rich block ready for Telegram (TS: `TelegramRichBlock`).
#[derive(Debug, Clone, PartialEq)]
pub enum RichBlock {
    Paragraph { text: RichText },
    Heading { size: u8, text: RichText },
    Blockquote { blocks: Vec<RichBlock> },
    List { items: Vec<RichListItem> },
    Pre { language: Option<String>, text: RichText },
    Table { cells: Vec<Vec<RichTableCell>>, is_bordered: bool },
    Divider,
}

/// Which representation of a part was delivered to Telegram
/// (TS: `TelegramRenderedPart["source"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSource {
    Blocks,
    Plain,
}

/// One entity attached to a plain-text part (TS: `MessageEntity`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartEntity {
    pub kind: String,
    pub offset: usize,
    pub length: usize,
}

/// A message part as delivered to Telegram (TS: `TelegramRenderedPart`).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedPart {
    pub blocks: Vec<RichBlock>,
    pub fallback_text: String,
    pub source: PartSource,
    pub entities: Vec<PartEntity>,
}

/// Convert inline nodes to rich text: empty text nodes are dropped, zero
/// parts collapse to `""`, one part collapses to itself
/// (TS: `toRichText`).
pub fn to_rich_text(nodes: &[InlineNode]) -> RichText {
    let parts = to_rich_text_nodes(nodes);
    if parts.is_empty() {
        RichText::Str(String::new())
    } else if parts.len() == 1 {
        parts.into_iter().next().unwrap_or(RichText::Str(String::new()))
    } else {
        RichText::Arr(parts)
    }
}

/// One rich part per non-empty inline node (TS: `toRichTextNodes`).
fn to_rich_text_nodes(nodes: &[InlineNode]) -> Vec<RichText> {
    let mut parts: Vec<RichText> = Vec::new();
    for node in nodes {
        match node {
            InlineNode::Text(text) => {
                if !text.is_empty() {
                    parts.push(RichText::Str(text.clone()));
                }
            }
            InlineNode::Bold(children) => parts.push(styled_rich_text("bold", children)),
            InlineNode::Italic(children) => parts.push(styled_rich_text("italic", children)),
            InlineNode::Strike(children) => {
                parts.push(styled_rich_text("strikethrough", children));
            }
            InlineNode::Underline(children) => {
                parts.push(styled_rich_text("underline", children));
            }
            InlineNode::Spoiler(children) => parts.push(styled_rich_text("spoiler", children)),
            InlineNode::Subscript(children) => {
                parts.push(styled_rich_text("subscript", children));
            }
            InlineNode::Superscript(children) => {
                parts.push(styled_rich_text("superscript", children));
            }
            InlineNode::Marked(children) => parts.push(styled_rich_text("marked", children)),
            InlineNode::Code(text) => parts.push(RichText::Styled {
                kind: "code".to_string(),
                text: Box::new(RichText::Str(text.clone())),
            }),
            InlineNode::Link { text, url } => parts.push(RichText::Url {
                text: Box::new(to_rich_text(text)),
                url: url.clone(),
            }),
        }
    }
    parts
}

/// Wrap converted children in a `{ type, text }` styled node
/// (TS: one arm of `toRichTextNodes`).
fn styled_rich_text(kind: &str, children: &[InlineNode]) -> RichText {
    RichText::Styled { kind: kind.to_string(), text: Box::new(to_rich_text(children)) }
}

/// Lists Telegram cannot render the way the product needs, so they are
/// written out as text: ordered lists and task lists
/// (TS: `isTextRenderedList`).
pub fn is_text_rendered_list(ordered: bool, items: &[TelegramListItem]) -> bool {
    ordered || items.iter().any(|item| item.checked.is_some())
}

/// The `N. `, `✅ `/`🔲 ` or `• ` marker an item is written out with
/// (TS: `textListMarker`). `start` defaults to 1 when `None`.
pub fn text_list_marker(
    item: &TelegramListItem,
    index: usize,
    ordered: bool,
    start: Option<u64>,
) -> String {
    let task = match item.checked {
        Some(true) => "✅ ",
        Some(false) => "🔲 ",
        None => "",
    };
    if ordered {
        let number = start.unwrap_or(1).saturating_add(index as u64);
        format!("{number}. {task}")
    } else if task.is_empty() {
        "• ".to_string()
    } else {
        task.to_string()
    }
}

/// Lift natively renderable content out of text-written lists, keeping order
/// and running numbering across the pieces
/// (TS: `liftTextListContent`).
pub fn lift_text_list_content(blocks: &[TelegramBlock]) -> Vec<TelegramBlock> {
    let mut out: Vec<TelegramBlock> = Vec::new();
    for block in blocks {
        let (ordered, items, start) = match block {
            TelegramBlock::List { ordered, items, start } => (*ordered, items, *start),
            _ => {
                out.push(block.clone());
                continue;
            }
        };
        if !is_text_rendered_list(ordered, items) {
            out.push(block.clone());
            continue;
        }

        let mut piece_items: Vec<TelegramListItem> = Vec::new();
        let mut start_number = start.unwrap_or(1);
        let flush_piece = |piece_items: &mut Vec<TelegramListItem>,
                               start_number: &mut u64,
                               out: &mut Vec<TelegramBlock>| {
            if piece_items.is_empty() {
                return;
            }
            let count = piece_items.len() as u64;
            out.push(TelegramBlock::List {
                ordered,
                items: std::mem::take(piece_items),
                start: Some(*start_number),
            });
            *start_number = start_number.saturating_add(count);
        };

        for item in items {
            let (leading, trailing): (Vec<TelegramBlock>, &[TelegramBlock]) =
                match item.blocks.first() {
                    Some(TelegramBlock::Paragraph { .. }) => {
                        (item.blocks[..1].to_vec(), &item.blocks[1..])
                    }
                    _ => (Vec::new(), &item.blocks),
                };
            // Lifting starts at the first block that has a native form; keeping
            // the split at one point preserves the order the item was written in.
            let lift_from = trailing.iter().position(|inner| !stays_inside_item(inner));
            let (keep, lifted): (&[TelegramBlock], &[TelegramBlock]) = match lift_from {
                None => (trailing, &[]),
                Some(at) => (&trailing[..at], &trailing[at..]),
            };
            let mut kept = leading;
            kept.extend(keep.iter().cloned());
            piece_items.push(TelegramListItem { blocks: kept, checked: item.checked });
            if !lifted.is_empty() {
                flush_piece(&mut piece_items, &mut start_number, &mut out);
                out.extend(lift_text_list_content(lifted));
            }
        }

        flush_piece(&mut piece_items, &mut start_number, &mut out);
    }
    out
}

/// A text-written sub-list with nothing native to lift stays inside its item,
/// where it is written out indented (TS: `staysInsideItem`).
fn stays_inside_item(block: &TelegramBlock) -> bool {
    match block {
        TelegramBlock::List { ordered, items, .. } => {
            if !is_text_rendered_list(*ordered, items) {
                return false;
            }
            items.iter().all(|item| {
                item.blocks.iter().all(|inner| {
                    matches!(inner, TelegramBlock::Paragraph { .. }) || stays_inside_item(inner)
                })
            })
        }
        _ => false,
    }
}

pub fn to_rich_block(block: &TelegramBlock) -> RichBlock {
    match block {
        TelegramBlock::Paragraph { inlines } => {
            RichBlock::Paragraph { text: to_rich_text(inlines) }
        }
        TelegramBlock::Heading { level, inlines } => RichBlock::Heading {
            size: (*level).clamp(1, 6) as u8,
            text: to_rich_text(inlines),
        },
        TelegramBlock::Blockquote { blocks } => RichBlock::Blockquote {
            blocks: blocks.iter().map(to_rich_block).collect(),
        },
        TelegramBlock::List { ordered, items, start } => {
            if is_text_rendered_list(*ordered, items) {
                to_text_list_block(items, *ordered, start.unwrap_or(1))
            } else {
                RichBlock::List {
                    items: items
                        .iter()
                        .map(|item| RichListItem {
                            blocks: item.blocks.iter().map(to_rich_block).collect(),
                        })
                        .collect(),
                }
            }
        }
        TelegramBlock::Code { language, text } => RichBlock::Pre {
            language: language.clone(),
            text: RichText::Str(text.clone()),
        },
        TelegramBlock::Table { rows, align } => {
            if rows.is_empty() {
                RichBlock::Paragraph { text: RichText::Str(String::new()) }
            } else {
                to_table_block(rows, align.as_deref())
            }
        }
        TelegramBlock::Rule => RichBlock::Divider,
        TelegramBlock::Plain { text } => {
            RichBlock::Paragraph { text: RichText::Str(text.clone()) }
        }
    }
}

/// Length in UTF-16 code units, matching TS `string.length` for markers,
/// indentation and table column widths.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Pad with spaces to a UTF-16 width, matching TS `padEnd` with ASCII spaces.
fn pad_end_to_width(cell: &str, width: usize) -> String {
    let mut out = cell.to_string();
    let mut len = utf16_len(&out);
    while len < width {
        out.push(' ');
        len += 1;
    }
    out
}

/// Pad columns, join with ` | `, insert a `---` divider after the header
/// (TS: `buildAlignedTableText`; width in UTF-16 units like TS `length`).
fn build_aligned_table_text(rows: &[Vec<String>]) -> String {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let normalized: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| {
            (0..column_count)
                .map(|index| row.get(index).map(String::as_str).unwrap_or(""))
                .collect()
        })
        .collect();
    let widths: Vec<usize> = (0..column_count)
        .map(|index| normalized.iter().map(|row| utf16_len(row[index])).max().unwrap_or(0))
        .collect();
    let format_row = |row: &[&str]| -> String {
        row.iter()
            .enumerate()
            .map(|(index, cell)| pad_end_to_width(cell, widths[index]))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let formatted: Vec<String> = normalized.iter().map(|row| format_row(row)).collect();
    if formatted.len() <= 1 {
        return formatted.join("\n");
    }
    let divider = widths.iter().map(|width| "-".repeat(*width)).collect::<Vec<_>>().join("-|-");
    let mut lines = vec![formatted[0].clone(), divider];
    lines.extend(formatted.into_iter().skip(1));
    lines.join("\n")
}

/// Concatenate inline content, dropping markup (TS: `extractInlinePlainText`).
fn inline_plain_text(nodes: &[InlineNode]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            InlineNode::Text(text) | InlineNode::Code(text) => result.push_str(text),
            InlineNode::Bold(children)
            | InlineNode::Italic(children)
            | InlineNode::Strike(children)
            | InlineNode::Underline(children)
            | InlineNode::Spoiler(children)
            | InlineNode::Subscript(children)
            | InlineNode::Superscript(children)
            | InlineNode::Marked(children) => result.push_str(&inline_plain_text(children)),
            InlineNode::Link { text, .. } => result.push_str(&inline_plain_text(text)),
        }
    }
    result
}

/// Plain-text projection of a block (TS: `toBlockPlainText`).
fn block_plain_text(block: &TelegramBlock) -> String {
    match block {
        TelegramBlock::Paragraph { inlines } | TelegramBlock::Heading { inlines, .. } => {
            inline_plain_text(inlines)
        }
        TelegramBlock::Blockquote { blocks } => blocks
            .iter()
            .map(block_plain_text)
            .filter(|text| !text.is_empty())
            .map(|text| {
                text.split('\n').map(|line| format!("> {line}")).collect::<Vec<_>>().join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        TelegramBlock::List { ordered, items, start } => {
            let from = start.unwrap_or(1);
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let task = match item.checked {
                        Some(true) => "✅ ",
                        Some(false) => "🔲 ",
                        None => "",
                    };
                    let marker = if *ordered {
                        format!("{}. {task}", from.saturating_add(index as u64))
                    } else if task.is_empty() {
                        "- ".to_string()
                    } else {
                        task.to_string()
                    };
                    let continuation = " ".repeat(utf16_len(&marker));
                    let body = item
                        .blocks
                        .iter()
                        .map(block_plain_text)
                        .filter(|text| !text.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    body.split('\n')
                        .enumerate()
                        .map(|(line_index, line)| {
                            format!(
                                "{}{line}",
                                if line_index == 0 { marker.as_str() } else { continuation.as_str() }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        TelegramBlock::Code { text, .. } => text.clone(),
        TelegramBlock::Table { rows, .. } => build_aligned_table_text(rows),
        TelegramBlock::Rule => "──────────".to_string(),
        TelegramBlock::Plain { text } => text.clone(),
    }
}

/// Inline content of an item for the text form of a list: only a leading
/// paragraph keeps its formatting, the rest is appended as indented plain
/// text (TS: `toTextItemNodes`).
fn to_text_item_nodes(item: &TelegramListItem, indent: &str) -> Vec<RichText> {
    let (leading_inlines, trailing_blocks): (&[InlineNode], &[TelegramBlock]) =
        match item.blocks.first() {
            Some(TelegramBlock::Paragraph { inlines }) => (inlines, &item.blocks[1..]),
            _ => (&[], &item.blocks),
        };
    let mut nodes = to_rich_text_nodes(leading_inlines);
    for block in trailing_blocks {
        let text = block_plain_text(block);
        if text.is_empty() {
            continue;
        }
        let indented =
            text.split('\n').map(|line| format!("{indent}{line}")).collect::<Vec<_>>().join("\n");
        nodes.push(RichText::Str(format!("\n{indented}")));
    }
    nodes
}

/// A list written out as text becomes a single paragraph of marker-headed
/// lines (TS: `toTextListBlock`).
fn to_text_list_block(
    items: &[TelegramListItem],
    ordered: bool,
    start: u64,
) -> RichBlock {
    let mut text: Vec<RichText> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let marker = text_list_marker(item, index, ordered, Some(start));
        if index == 0 {
            text.push(RichText::Str(marker.clone()));
        } else {
            text.push(RichText::Str(format!("\n{marker}")));
        }
        text.extend(to_text_item_nodes(item, &" ".repeat(utf16_len(&marker))));
    }
    RichBlock::Paragraph { text: RichText::Arr(text) }
}

/// Native rich-table cells: first row is the header, every cell defaults to
/// left alignment (TS: `toTableCells`).
fn to_table_cells(rows: &[Vec<String>], align: Option<&[TableAlign]>) -> Vec<Vec<RichTableCell>> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    rows.iter()
        .enumerate()
        .map(|(row_index, row)| {
            (0..column_count)
                .map(|column_index| RichTableCell {
                    text: row.get(column_index).cloned().unwrap_or_default(),
                    is_header: row_index == 0,
                    align: align.and_then(|values| values.get(column_index).copied()).unwrap_or(TableAlign::Left),
                })
                .collect()
        })
        .collect()
}

/// Tables wider than [`RICH_TABLE_MAX_COLUMNS`] degrade to preformatted
/// aligned text (TS: `toTableBlock`).
fn to_table_block(rows: &[Vec<String>], align: Option<&[TableAlign]>) -> RichBlock {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if column_count > RICH_TABLE_MAX_COLUMNS {
        return RichBlock::Pre {
            language: None,
            text: RichText::Str(build_aligned_table_text(rows)),
        };
    }
    RichBlock::Table { cells: to_table_cells(rows, align), is_bordered: true }
}

/// Build a native paragraph block from raw text
/// (TS: `createParagraphBlock`).
pub fn create_paragraph_block(text: &str) -> RichBlock {
    RichBlock::Paragraph { text: RichText::Str(text.to_string()) }
}

/// Stable identity of what was actually delivered to Telegram
/// (TS: `getTelegramRenderedPartSignature`).
pub fn get_telegram_rendered_part_signature(part: &RenderedPart) -> String {
    if part.source == PartSource::Plain || part.blocks.is_empty() {
        // Reasoning keeps the same text when it collapses, only the entity changes.
        let entities = if part.entities.is_empty() {
            String::new()
        } else {
            format!(
                "[{}]",
                part.entities
                    .iter()
                    .map(|entity| format!(
                        "{{\"type\":{},\"offset\":{},\"length\":{}}}",
                        json_string(&entity.kind),
                        entity.offset,
                        entity.length
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        return format!("plain\n{entities}\n{}", part.fallback_text);
    }

    format!(
        "blocks\n[{}]",
        part.blocks.iter().map(rich_block_json).collect::<Vec<_>>().join(",")
    )
}

/// Encode a string exactly as TS `JSON.stringify` does for the ASCII-safe
/// subset: quotes, backslashes and C0 controls are escaped, other content
/// (including non-BMP emoji such as ✅/🔲) passes through verbatim.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            ch if (ch as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Encode rich inline content (TS: `JSON.stringify` of grammy `RichText`).
fn rich_text_json(text: &RichText) -> String {
    match text {
        RichText::Str(value) => json_string(value),
        RichText::Arr(parts) => format!(
            "[{}]",
            parts.iter().map(rich_text_json).collect::<Vec<_>>().join(",")
        ),
        RichText::Styled { kind, text } => {
            format!("{{\"type\":{},\"text\":{}}}", json_string(kind), rich_text_json(text))
        }
        RichText::Url { text, url } => format!(
            "{{\"type\":\"url\",\"text\":{},\"url\":{}}}",
            rich_text_json(text),
            json_string(url)
        ),
    }
}

/// Table alignment as grammy serialises it.
fn table_align_json(align: TableAlign) -> &'static str {
    match align {
        TableAlign::Left => "left",
        TableAlign::Center => "center",
        TableAlign::Right => "right",
    }
}

/// Encode one native rich block (TS: `JSON.stringify` of `TelegramRichBlock`;
/// key order follows the TS object literals).
fn rich_block_json(block: &RichBlock) -> String {
    match block {
        RichBlock::Paragraph { text } => {
            format!("{{\"type\":\"paragraph\",\"text\":{}}}", rich_text_json(text))
        }
        RichBlock::Heading { size, text } => format!(
            "{{\"type\":\"heading\",\"text\":{},\"size\":{size}}}",
            rich_text_json(text)
        ),
        RichBlock::Blockquote { blocks } => format!(
            "{{\"type\":\"blockquote\",\"blocks\":[{}]}}",
            blocks.iter().map(rich_block_json).collect::<Vec<_>>().join(",")
        ),
        RichBlock::List { items } => format!(
            "{{\"type\":\"list\",\"items\":[{}]}}",
            items
                .iter()
                .map(|item| format!(
                    "{{\"blocks\":[{}]}}",
                    item.blocks.iter().map(rich_block_json).collect::<Vec<_>>().join(",")
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        RichBlock::Pre { language, text } => {
            let mut out =
                format!("{{\"type\":\"pre\",\"text\":{}}}", rich_text_json(text));
            if let Some(language) = language {
                out.pop();
                out.push_str(&format!(",\"language\":{}}}", json_string(language)));
            }
            out
        }
        RichBlock::Table { cells, is_bordered } => format!(
            "{{\"type\":\"table\",\"cells\":[{}],\"is_bordered\":{is_bordered}}}",
            cells
                .iter()
                .map(|row| format!(
                    "[{}]",
                    row.iter()
                        .map(|cell| {
                            let mut out =
                                format!("{{\"text\":{}", json_string(&cell.text));
                            if cell.is_header {
                                out.push_str(",\"is_header\":true");
                            }
                            out.push_str(&format!(
                                ",\"align\":\"{}\",\"valign\":\"top\"}}",
                                table_align_json(cell.align)
                            ));
                            out
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        RichBlock::Divider => "{\"type\":\"divider\"}".to_string(),
    }
}
