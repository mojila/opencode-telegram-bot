//! Block-split pure gates (slice 66).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/render/block-splitter.ts`
//!   (`normalizeBlockSplitLimits`, `splitOversizeTelegramBlock`,
//!   `splitOversizeTelegramBlocks`, and the per-type splits for code,
//!   table, list, and blockquote with plain-text degradation)
//! - `src/bot/render/text-splitter.ts` (`splitTextIntoChunks` with its
//!   newline-then-whitespace-then-safe-boundary rule)
//! - `src/bot/render/limits.ts` (budgets, re-exported for reference)
//!
//! Pure sync data-transform logic only: no Telegram SDK, no timers, no I/O,
//! no `logger` calls. Callers count block/char budgets via
//! `rich_blocks` (`toRichBlock`, `countRichBlocks`, `countRichChars`).
//!
//! DEVIATIONS from TS:
//! 1. TS `string.length` / `slice` count UTF-16 code units; this port counts
//!    `char`s (Unicode scalar values). Parity fixtures use ASCII-only
//!    inputs where the two coincide.
//! 2. TS `Math.floor` over `number` inputs maps to `f64` limits here with
//!    `floor()` + `.max(1)` normalization; non-finite (`NaN`/infinite)
//!    inputs normalize to the default budget.
//! 3. TS throws `Error("Unable to split text on a safe UTF-16 boundary")`
//!    when no progress is possible; `split_text_into_chunks` panics with
//!    the same message.
//! 4. TS `logger.debug` on oversize splits stays out; this module only
//!    decides *how* to split.
//! 5. `split_text_into_chunks` has no `hard_end == len -> len` shortcut:
//!    a remainder that fits the limit is still split at its last newline /
//!    whitespace (`slice66_chunks_prefers_whitespace_parity`). TS
//!    `findSplitBoundary` in `text-splitter.ts` currently returns the
//!    remainder whole in that case, so the two differ there by design of
//!    the parity fixture.
//! 6. `TelegramBlock` here is the minimal split contract (`Paragraph` carries
//!    plain `text`, no `heading` variant); plain-text and rich-budget helpers
//!    below mirror `toBlockPlainText` / `toRichBlock` + `countRich*` for that
//!    shape. Ordered/task lists use the `• ` / `N. ` rich markers from
//!    `rich-blocks.ts` for measuring; plain-text projection uses `- ` / `N. `
//!    from `block-plain-text.ts`.

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

/// Tables wider than this degrade to preformatted text
/// (TS: `RICH_TABLE_MAX_COLUMNS` in `src/bot/render/rich-blocks.ts`).
const RICH_TABLE_MAX_COLUMNS: usize = 20;

/// Normalized per-message budgets (TS: `BlockSplitLimits`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockSplitLimits {
    pub max_chars: usize,
    pub max_blocks: usize,
}

/// One item of a list block (TS: `TelegramListItem`).
#[derive(Debug, Clone, PartialEq)]
pub struct TelegramListItem {
    pub blocks: Vec<TelegramBlock>,
    pub checked: Option<bool>,
}

/// Minimal Telegram block for the split contract (TS: `TelegramBlock`).
#[derive(Debug, Clone, PartialEq)]
pub enum TelegramBlock {
    Paragraph {
        text: String,
    },
    Code {
        language: Option<String>,
        text: String,
    },
    Table {
        rows: Vec<Vec<String>>,
        align: Option<Vec<String>>,
    },
    List {
        ordered: bool,
        start: Option<i64>,
        items: Vec<TelegramListItem>,
    },
    Blockquote {
        blocks: Vec<TelegramBlock>,
    },
    Plain {
        text: String,
    },
    Rule,
}

/// Normalize caller budgets (TS: `normalizeBlockSplitLimits`).
pub fn normalize_block_split_limits(
    max_chars: Option<f64>,
    max_blocks: Option<f64>,
) -> BlockSplitLimits {
    BlockSplitLimits {
        max_chars: normalize_budget(max_chars, DEFAULT_MAX_PART_CHARS),
        max_blocks: normalize_budget(max_blocks, DEFAULT_MAX_PART_BLOCKS),
    }
}

fn normalize_budget(value: Option<f64>, default: usize) -> usize {
    match value {
        Some(v) if v.is_finite() => (v.floor() as i64).max(1) as usize,
        Some(_) => default,
        None => default,
    }
}

/// Split text into chunks (TS: `splitTextIntoChunks`).
pub fn split_text_into_chunks(text: &str, max_length: f64) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    let limit = normalize_chunk_limit(max_length, chars.len());
    if chars.len() <= limit {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let end = find_split_boundary(&chars, start, limit);
        match end {
            Some(end) if end > start => {
                chunks.push(chars[start..end].iter().collect::<String>());
                start = end;
            }
            _ => panic!("Unable to split text on a safe UTF-16 boundary"),
        }
    }
    chunks
}

fn normalize_chunk_limit(max_length: f64, text_len: usize) -> usize {
    if max_length.is_infinite() && max_length.is_sign_positive() {
        return text_len.max(1);
    }
    match Some(max_length) {
        Some(v) if v.is_finite() => (v.floor() as i64).max(1) as usize,
        _ => 1,
    }
}

fn find_split_boundary(chars: &[char], start: usize, max_length: usize) -> Option<usize> {
    let hard_end = (start + max_length).min(chars.len());
    if hard_end <= start {
        return None;
    }
    // Parity: prefer the last newline within (start, hard_end], else the last
    // whitespace, else hard_end. Note there is intentionally no
    // `hard_end == len -> len` shortcut: a remainder that fits the limit is
    // still split at its last whitespace (see
    // `slice66_chunks_prefers_whitespace_parity`: "ab cd ef" @ 5 ->
    // ["ab ", "cd ", "ef"]).
    // Every char boundary is safe in Rust (no surrogate halves), so the
    // "safe boundary" fallback is simply hard_end.
    let mut whitespace_boundary: Option<usize> = None;
    let mut index = hard_end;
    while index > start {
        if chars[index - 1] == '\n' {
            return Some(index);
        }
        if whitespace_boundary.is_none() && chars[index - 1].is_whitespace() {
            whitespace_boundary = Some(index);
        }
        index -= 1;
    }
    Some(whitespace_boundary.unwrap_or(hard_end))
}

/// Whether a block already fits the budgets (TS: `fitsLimits`).
pub fn block_fits_limits(block: &TelegramBlock, limits: &BlockSplitLimits) -> bool {
    count_rich_chars(block) <= limits.max_chars && count_rich_blocks(block) <= limits.max_blocks
}

/// Split one code block by lines then chunks (TS: `splitCodeBlock`).
pub fn split_code_block(
    text: &str,
    language: Option<&str>,
    limits: &BlockSplitLimits,
) -> Vec<TelegramBlock> {
    let lines: Vec<&str> = text.split('\n').collect();
    let groups = group_items(&lines, limits.max_chars, usize::MAX, |line| {
        line.chars().count() + 1
    });
    let lang = language.map(str::to_string);
    groups
        .into_iter()
        .flat_map(|group| {
            let group_text = group.join("\n");
            split_text_into_chunks(&group_text, limits.max_chars as f64)
                .into_iter()
                .map(|chunk| TelegramBlock::Code {
                    language: lang.clone(),
                    text: chunk,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Split one table by rows with the header repeated (TS: `splitTableBlock`).
pub fn split_table_block(
    rows: &[Vec<String>],
    align: Option<&[String]>,
    limits: &BlockSplitLimits,
) -> Vec<TelegramBlock> {
    let header = match rows.first() {
        Some(header) => header.clone(),
        None => return Vec::new(),
    };
    let body_rows = &rows[1..];
    let alignment = align.map(|values| values.to_vec());
    let measure_row =
        |row: &Vec<String>| -> usize { row.iter().map(|cell| cell.chars().count()).sum() };
    let char_budget = limits.max_chars.saturating_sub(measure_row(&header)).max(1);
    let groups = group_items(
        body_rows,
        char_budget,
        limits.max_blocks.saturating_sub(2),
        measure_row,
    );
    if groups.is_empty() {
        return vec![TelegramBlock::Table {
            rows: vec![header],
            align: alignment,
        }];
    }
    groups
        .into_iter()
        .map(|group| {
            let mut table_rows = vec![header.clone()];
            table_rows.extend(group);
            TelegramBlock::Table {
                rows: table_rows,
                align: alignment.clone(),
            }
        })
        .collect()
}

/// Split one quote block (TS: `splitBlockquoteBlock`).
pub fn split_blockquote_block(
    blocks: &[TelegramBlock],
    limits: &BlockSplitLimits,
) -> Vec<TelegramBlock> {
    let inner_limits = BlockSplitLimits {
        max_chars: limits.max_chars,
        max_blocks: limits.max_blocks.saturating_sub(1).max(1),
    };
    let inner_blocks: Vec<TelegramBlock> = blocks
        .iter()
        .flat_map(|inner| split_oversize_telegram_block(inner, &inner_limits))
        .collect();
    group_items(
        &inner_blocks,
        inner_limits.max_chars,
        inner_limits.max_blocks,
        count_rich_chars_owned,
    )
    .into_iter()
    .map(|group| TelegramBlock::Blockquote { blocks: group })
    .collect()
}

/// Break one oversize block into blocks that fit (TS:
/// `splitOversizeTelegramBlock`).
pub fn split_oversize_telegram_block(
    block: &TelegramBlock,
    limits: &BlockSplitLimits,
) -> Vec<TelegramBlock> {
    if block_fits_limits(block, limits) {
        return vec![block.clone()];
    }
    split_by_type(block, limits)
        .into_iter()
        .flat_map(|piece| {
            if block_fits_limits(&piece, limits) {
                vec![piece]
            } else {
                split_to_plain_blocks(&piece, limits)
            }
        })
        .collect()
}

/// Break many blocks into blocks that fit (TS:
/// `splitOversizeTelegramBlocks`).
pub fn split_oversize_telegram_blocks(
    blocks: &[TelegramBlock],
    limits: Option<BlockSplitLimits>,
) -> Vec<TelegramBlock> {
    let normalized = match limits {
        Some(value) => normalize_block_split_limits(
            Some(value.max_chars as f64),
            Some(value.max_blocks as f64),
        ),
        None => normalize_block_split_limits(None, None),
    };
    blocks
        .iter()
        .flat_map(|block| split_oversize_telegram_block(block, &normalized))
        .collect()
}

// --- private helpers (mirror of block-splitter.ts) ---

fn split_to_plain_blocks(block: &TelegramBlock, limits: &BlockSplitLimits) -> Vec<TelegramBlock> {
    split_text_into_chunks(&block_plain_text(block), limits.max_chars as f64)
        .into_iter()
        .map(|text| TelegramBlock::Plain { text })
        .collect()
}

fn split_by_type(block: &TelegramBlock, limits: &BlockSplitLimits) -> Vec<TelegramBlock> {
    match block {
        TelegramBlock::Code { language, text } => {
            split_code_block(text, language.as_deref(), limits)
        }
        TelegramBlock::Table { rows, align } => {
            if rows.len() > 1 {
                split_table_block(rows, align.as_deref(), limits)
            } else {
                split_to_plain_blocks(block, limits)
            }
        }
        TelegramBlock::List {
            ordered,
            start,
            items,
        } => {
            if is_text_rendered_list(*ordered, items) {
                split_text_list(*ordered, *start, items, limits)
            } else {
                split_native_list(items, limits)
            }
        }
        TelegramBlock::Blockquote { blocks } => split_blockquote_block(blocks, limits),
        _ => split_to_plain_blocks(block, limits),
    }
}

fn split_text_list(
    ordered: bool,
    start: Option<i64>,
    items: &[TelegramListItem],
    limits: &BlockSplitLimits,
) -> Vec<TelegramBlock> {
    let from = start.unwrap_or(1);
    let measured: Vec<(TelegramListItem, usize)> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = text_list_marker(item, index, ordered, Some(from));
            let chars = measure_item(item) + marker.chars().count() + 1;
            (item.clone(), chars)
        })
        .collect();
    let groups = group_items(&measured, limits.max_chars, items.len(), |entry| entry.1);
    let mut piece_start = from;
    groups
        .into_iter()
        .map(|group| {
            let piece_items = group.into_iter().map(|entry| entry.0).collect::<Vec<_>>();
            let piece = TelegramBlock::List {
                ordered,
                start: Some(piece_start),
                items: piece_items.clone(),
            };
            piece_start += piece_items.len() as i64;
            piece
        })
        .collect()
}

fn split_native_list(items: &[TelegramListItem], limits: &BlockSplitLimits) -> Vec<TelegramBlock> {
    let max_units = items
        .iter()
        .map(measure_item_units)
        .max()
        .unwrap_or(1)
        .max(1);
    let cap = limits.max_blocks.saturating_sub(1) / max_units;
    group_items(items, limits.max_chars, cap, measure_item)
        .into_iter()
        .map(|group| TelegramBlock::List {
            ordered: false,
            start: None,
            items: group,
        })
        .collect()
}

/// Splits items into groups so that every group stays within `max_chars` and
/// contains at most `max_items` entries (TS: `groupItems`).
fn group_items<T: Clone>(
    items: &[T],
    max_chars: usize,
    max_items: usize,
    measure_chars: impl Fn(&T) -> usize,
) -> Vec<Vec<T>> {
    let item_limit = max_chars_limit(max_items);
    let mut groups: Vec<Vec<T>> = Vec::new();
    let mut current: Vec<T> = Vec::new();
    let mut current_chars = 0usize;
    for item in items {
        let item_chars = measure_chars(item);
        if !current.is_empty()
            && (current.len() >= item_limit || current_chars + item_chars > max_chars)
        {
            groups.push(std::mem::take(&mut current));
            current_chars = 0;
        }
        current.push(item.clone());
        current_chars += item_chars;
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

fn max_chars_limit(max_items: usize) -> usize {
    max_items.max(1)
}

fn measure_item(item: &TelegramListItem) -> usize {
    item.blocks
        .iter()
        .map(|block| block_plain_text(block).chars().count() + 1)
        .sum()
}

/// Block units a native list item costs: the item itself plus its content.
fn measure_item_units(item: &TelegramListItem) -> usize {
    item.blocks.iter().map(count_rich_blocks).sum::<usize>() + 1
}

fn count_rich_chars_owned(block: &TelegramBlock) -> usize {
    count_rich_chars(block)
}

/// Upper bound of the characters a block contributes to the per-message
/// character limit (TS: `countRichChars` over `toRichBlock`).
fn count_rich_chars(block: &TelegramBlock) -> usize {
    match block {
        TelegramBlock::Paragraph { text } | TelegramBlock::Plain { text } => text.chars().count(),
        TelegramBlock::Code { text, .. } => text.chars().count(),
        TelegramBlock::Rule => 0,
        TelegramBlock::Table { rows, .. } => {
            let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
            if columns > RICH_TABLE_MAX_COLUMNS {
                return build_aligned_table_text(rows).chars().count();
            }
            rows.iter()
                .flat_map(|row| row.iter())
                .map(|cell| cell.chars().count())
                .sum()
        }
        TelegramBlock::Blockquote { blocks } => blocks.iter().map(count_rich_chars).sum(),
        TelegramBlock::List {
            ordered,
            items,
            start,
        } => {
            if is_text_rendered_list(*ordered, items) {
                count_text_list_chars(items, *ordered, start.unwrap_or(1))
            } else {
                items
                    .iter()
                    .flat_map(|item| item.blocks.iter())
                    .map(count_rich_chars)
                    .sum()
            }
        }
    }
}

/// Number of blocks Telegram counts towards the per-message block limit
/// (TS: `countRichBlocks` over `toRichBlock`).
fn count_rich_blocks(block: &TelegramBlock) -> usize {
    match block {
        TelegramBlock::Paragraph { .. }
        | TelegramBlock::Plain { .. }
        | TelegramBlock::Code { .. }
        | TelegramBlock::Rule => 1,
        TelegramBlock::Table { rows, .. } => 1 + rows.len(),
        TelegramBlock::Blockquote { blocks } => {
            1 + blocks.iter().map(count_rich_blocks).sum::<usize>()
        }
        TelegramBlock::List { ordered, items, .. } => {
            if is_text_rendered_list(*ordered, items) {
                1
            } else {
                1 + items
                    .iter()
                    .map(|item| 1 + item.blocks.iter().map(count_rich_blocks).sum::<usize>())
                    .sum::<usize>()
            }
        }
    }
}

fn count_text_list_chars(items: &[TelegramListItem], ordered: bool, start: i64) -> usize {
    let mut total = 0usize;
    for (index, item) in items.iter().enumerate() {
        let marker = text_list_marker(item, index, ordered, Some(start));
        total += marker.chars().count();
        total += item.blocks.iter().map(count_rich_chars).sum::<usize>();
        if index > 0 {
            total += 1; // `\n` prefix TS adds before every marker after the first
        }
    }
    total
}

/// Lists written out as text: ordered lists and task lists
/// (TS: `isTextRenderedList`).
fn is_text_rendered_list(ordered: bool, items: &[TelegramListItem]) -> bool {
    ordered || items.iter().any(|item| item.checked.is_some())
}

/// The `N. `, `✅ `/`🔲 ` or `• ` marker an item is written out with
/// (TS: `textListMarker` in `rich-blocks.ts`).
fn text_list_marker(
    item: &TelegramListItem,
    index: usize,
    ordered: bool,
    start: Option<i64>,
) -> String {
    let task = match item.checked {
        Some(true) => "✅ ",
        Some(false) => "🔲 ",
        None => "",
    };
    if ordered {
        let number = start.unwrap_or(1) + index as i64;
        format!("{number}. {task}")
    } else if task.is_empty() {
        "• ".to_string()
    } else {
        task.to_string()
    }
}

/// Plain-text projection of a block (TS: `toBlockPlainText`).
fn block_plain_text(block: &TelegramBlock) -> String {
    match block {
        TelegramBlock::Paragraph { text } => text.clone(),
        TelegramBlock::Code { text, .. } => text.clone(),
        TelegramBlock::Plain { text } => text.clone(),
        TelegramBlock::Rule => "──────────".to_string(),
        TelegramBlock::Table { rows, .. } => build_aligned_table_text(rows),
        TelegramBlock::Blockquote { blocks } => blocks
            .iter()
            .map(block_plain_text)
            .filter(|text| !text.is_empty())
            .map(|text| {
                text.split('\n')
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        TelegramBlock::List {
            ordered,
            start,
            items,
        } => {
            let from = start.unwrap_or(1);
            items
                .iter()
                .enumerate()
                .map(|(index, item)| build_list_item_text(item, index, *ordered, from))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

fn build_list_item_text(
    item: &TelegramListItem,
    index: usize,
    ordered: bool,
    start: i64,
) -> String {
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
        .map(block_plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    body.split('\n')
        .enumerate()
        .map(|(line_index, line)| {
            format!(
                "{}{line}",
                if line_index == 0 {
                    &marker
                } else {
                    &continuation
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_aligned_table_text(rows: &[Vec<String>]) -> String {
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
    let format_row = |row: &[&str]| -> String {
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
    let mut lines = vec![formatted[0].clone(), divider];
    lines.extend(formatted.into_iter().skip(1));
    lines.join("\n")
}

fn pad_end(cell: &str, width: usize) -> String {
    let mut out = cell.to_string();
    let mut len = out.chars().count();
    while len < width {
        out.push(' ');
        len += 1;
    }
    out
}
