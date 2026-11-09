//! Telegram message chunking.
//!
//! Rust port of `src/bot/render/chunker.ts` (+ `limits.ts`, `text-splitter.ts`
//! splitting semantics and `rich-blocks.ts` budget counting for the block
//! variants modelled here).
//!
//! String-unit mapping (important for parity):
//! TS `string.length` and `slice` count UTF-16 code units, so the TS splitter
//! reasons about surrogate pairs (`text-splitter.ts`). Rust `String` is UTF-8
//! and `chars()` yields Unicode scalar values, so there are no surrogates to
//! split: the port cuts on `char` boundaries instead, and the budget unit
//! becomes scalar values rather than UTF-16 units. Exact part counts for
//! non-BMP text (e.g. emoji) may therefore differ from TS; the ported tests
//! assert structural properties (valid parts that rejoin exactly) there.
//!
//! `char::is_whitespace` (Unicode `White_Space`) stands in for the TS
//! `/\s/` test when looking for a whitespace split point; both agree on
//! ASCII space / `\t` / `\n` / `\r`, which is what the contract exercises.

// --- Budgets (mirror of limits.ts) ---

/// Budget for a part that carries native rich blocks (chars).
pub const DEFAULT_MAX_PART_CHARS: usize = 32_000;
/// Budget for a part that carries native rich blocks (block units).
pub const DEFAULT_MAX_PART_BLOCKS: usize = 480;
/// Budget for text delivered as a plain Telegram message.
pub const PLAIN_MAX_PART_CHARS: usize = 3_800;
/// Hard Telegram limit for send/edit text messages.
pub const TELEGRAM_TEXT_MESSAGE_LIMIT: usize = 4_096;

/// Separator used to join block plain-text projections into `fallback_text`.
const DEFAULT_BLOCK_SEPARATOR: &str = "\n\n";

// --- Minimal block model (enough for the chunker contract) ---
//
// The full TS `TelegramRichBlock` is the grammy rich-block union; this port
// models the variants the parity tests construct: paragraph, divider, and
// list-of-paragraphs.

/// One item of a list block.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub blocks: Vec<RichBlock>,
}

/// Minimal native rich block for the chunker contract.
#[derive(Debug, Clone, PartialEq)]
pub enum RichBlock {
    Paragraph { text: String },
    Divider,
    List { items: Vec<ListItem> },
}

/// Which representation a rendered part carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSource {
    Blocks,
    Plain,
}

/// A single parsed block paired with its plain-text projection.
/// (TS: `TelegramRenderedBlock`.)
#[derive(Debug, Clone, PartialEq)]
pub struct TelegramRenderedBlock {
    pub block: RichBlock,
    pub plain_text: String,
}

/// A message ready to be delivered to Telegram.
/// (TS: `TelegramRenderedPart`.)
#[derive(Debug, Clone, PartialEq)]
pub struct TelegramRenderedPart {
    pub blocks: Vec<RichBlock>,
    pub fallback_text: String,
    pub source: PartSource,
}

/// Options for [`chunk_plain_text`]. (TS: `PlainChunkerOptions`.)
#[derive(Debug, Clone, Default)]
pub struct PlainChunkerOptions {
    pub max_chars: Option<usize>,
}

/// Options for [`chunk_telegram_rendered_blocks`].
/// (TS: `TelegramChunkerOptions`.)
#[derive(Debug, Clone, Default)]
pub struct TelegramChunkerOptions {
    pub max_chars: Option<usize>,
    pub max_blocks: Option<usize>,
}

/// Number of blocks Telegram counts towards the per-message block limit.
/// (TS: `countRichBlocks` in `rich-blocks.ts`, restricted to the block
/// variants modelled here: nested list-item paragraphs count.)
fn count_rich_blocks(block: &RichBlock) -> usize {
    match block {
        RichBlock::Paragraph { .. } | RichBlock::Divider => 1,
        RichBlock::List { items } => {
            1 + items
                .iter()
                .map(|item| 1 + item.blocks.iter().map(count_rich_blocks).sum::<usize>())
                .sum::<usize>()
        }
    }
}

/// Upper bound of the characters a block contributes to the per-message
/// character limit, counted in Unicode scalar values (`chars()`).
/// (TS: `countRichChars` in `rich-blocks.ts`, restricted to the block
/// variants modelled here.)
fn count_rich_chars(block: &RichBlock) -> usize {
    match block {
        RichBlock::Paragraph { text } => text.chars().count(),
        RichBlock::Divider => 0,
        RichBlock::List { items } => items
            .iter()
            .map(|item| item.blocks.iter().map(count_rich_chars).sum::<usize>())
            .sum(),
    }
}

/// Finds the largest cut point within `limit` chars of `start`, preferring a
/// line break, then any whitespace, then the hard (char-boundary) limit.
/// (TS: `findSplitBoundary` in `text-splitter.ts`; every `char` boundary is
/// safe in Rust, so there is no surrogate-pair scan.)
fn find_split_boundary(chars: &[char], start: usize, limit: usize) -> Option<usize> {
    let hard_end = (start + limit).min(chars.len());
    if hard_end >= chars.len() {
        return Some(chars.len());
    }

    let mut whitespace_boundary: Option<usize> = None;

    for index in (start + 1..=hard_end).rev() {
        if chars[index - 1] == '\n' {
            return Some(index);
        }
        if whitespace_boundary.is_none() && chars[index - 1].is_whitespace() {
            whitespace_boundary = Some(index);
        }
    }

    Some(whitespace_boundary.unwrap_or(hard_end))
}

/// Splits text into chunks of at most `max_length` chars, cutting on line
/// boundaries, then whitespace, then char boundaries (never inside a scalar
/// value, so emoji stay whole). Chunks rejoin losslessly.
/// (TS: `splitTextIntoChunks` in `text-splitter.ts`, budget in chars.)
fn split_text_into_chunks(text: &str, max_length: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let limit = max_length.max(1);
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= limit {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = find_split_boundary(&chars, start, limit).unwrap_or(chars.len());
        let end = end.max(start + 1);
        chunks.push(chars[start..end].iter().collect());
        start = end;
    }

    chunks
}

/// Groups rendered blocks so that every group stays within both Telegram
/// budgets. Blocks are expected to fit individually already (see
/// `block-splitter.ts`); an oversize one lands in a group of its own.
/// (TS: `groupRenderedBlocks` in `chunker.ts`.)
fn group_rendered_blocks(
    blocks: Vec<TelegramRenderedBlock>,
    options: Option<TelegramChunkerOptions>,
) -> Vec<Vec<TelegramRenderedBlock>> {
    let max_chars = options
        .as_ref()
        .and_then(|o| o.max_chars)
        .unwrap_or(DEFAULT_MAX_PART_CHARS)
        .max(1);
    let max_blocks = options
        .as_ref()
        .and_then(|o| o.max_blocks)
        .unwrap_or(DEFAULT_MAX_PART_BLOCKS)
        .max(1);

    let mut groups: Vec<Vec<TelegramRenderedBlock>> = Vec::new();
    let mut current: Vec<TelegramRenderedBlock> = Vec::new();
    let mut current_chars = 0usize;
    let mut current_units = 0usize;

    for rendered in blocks {
        let chars = count_rich_chars(&rendered.block);
        let units = count_rich_blocks(&rendered.block);

        if !current.is_empty()
            && (current_chars + chars > max_chars || current_units + units > max_blocks)
        {
            groups.push(current);
            current = Vec::new();
            current_chars = 0;
            current_units = 0;
        }

        current.push(rendered);
        current_chars += chars;
        current_units += units;
    }

    if !current.is_empty() {
        groups.push(current);
    }

    groups
}

/// Cuts text into parts that fit a plain Telegram text message. Used by raw
/// format mode and by every degradation from native blocks to plain text.
/// (TS: `chunkPlainText`.)
pub fn chunk_plain_text(
    text: &str,
    options: Option<PlainChunkerOptions>,
) -> Vec<TelegramRenderedPart> {
    let max_chars = options
        .as_ref()
        .and_then(|o| o.max_chars)
        .unwrap_or(PLAIN_MAX_PART_CHARS)
        .max(1);

    split_text_into_chunks(text, max_chars)
        .into_iter()
        .map(|chunk| TelegramRenderedPart {
            blocks: Vec::new(),
            fallback_text: chunk,
            source: PartSource::Plain,
        })
        .collect()
}

/// Packs rendered blocks into messages, respecting both Telegram budgets:
/// characters and block count.
/// (TS: `chunkTelegramRenderedBlocks`.)
pub fn chunk_telegram_rendered_blocks(
    blocks: Vec<TelegramRenderedBlock>,
    options: Option<TelegramChunkerOptions>,
) -> Vec<TelegramRenderedPart> {
    group_rendered_blocks(blocks, options)
        .into_iter()
        .map(|group| {
            let fallback_text = group
                .iter()
                .map(|rendered| rendered.plain_text.as_str())
                .filter(|plain| !plain.is_empty())
                .collect::<Vec<_>>()
                .join(DEFAULT_BLOCK_SEPARATOR);
            let rich_blocks = group.into_iter().map(|rendered| rendered.block).collect();
            TelegramRenderedPart {
                blocks: rich_blocks,
                fallback_text,
                source: PartSource::Blocks,
            }
        })
        .collect()
}
