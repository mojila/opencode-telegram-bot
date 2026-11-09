//! Render pipeline (parity library only).
//!
//! Rust port of `src/bot/render/pipeline.ts`:
//! - `toRenderedBlocks` (lift text lists, split oversize blocks, pair rich +
//!   plain projections)
//! - `renderTelegramBlocks` (parse markdown, then `toRenderedBlocks`)
//! - `renderTelegramParts` (render, then chunk into Telegram parts)
//!
//! Chunker limits mirror `src/bot/render/limits.ts` via
//! `super::chunker` (`DEFAULT_MAX_PART_CHARS`, `DEFAULT_MAX_PART_BLOCKS`,
//! `PLAIN_MAX_PART_CHARS`). The full TS markdown parser
//! (`block-parser.ts`), rich conversion (`rich-blocks.ts`) and oversize
//! splitter (`block-splitter.ts`) are markdown/grammy specific; this parity
//! port keeps the same stage order with a minimal block model: blank-line
//! paragraphs, oversize paragraphs degraded to plain-text chunks. Live bot
//! runtime still uses the TS pipeline; wire this module in only when the
//! Rust runtime owns rendering.
//!
//! Wiring point: replace the TS `renderTelegramParts` call with
//! `pipeline::render_telegram_parts` once `otb-core` owns delivery.

use super::chunker::{
    chunk_telegram_rendered_blocks, RichBlock, TelegramChunkerOptions, TelegramRenderedBlock,
    TelegramRenderedPart, DEFAULT_MAX_PART_BLOCKS, DEFAULT_MAX_PART_CHARS,
};

/// Normalized split limits (TS: `normalizeBlockSplitLimits`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockSplitLimits {
    pub max_chars: usize,
    pub max_blocks: usize,
}

/// Floor partial limits at 1, defaulting to the Telegram part budgets
/// (TS: `normalizeBlockSplitLimits`).
pub fn normalize_block_split_limits(
    max_chars: Option<usize>,
    max_blocks: Option<usize>,
) -> BlockSplitLimits {
    BlockSplitLimits {
        max_chars: max_chars.unwrap_or(DEFAULT_MAX_PART_CHARS).max(1),
        max_blocks: max_blocks.unwrap_or(DEFAULT_MAX_PART_BLOCKS).max(1),
    }
}

/// Pair each block with its plain-text projection, degrading oversize
/// paragraphs to plain-text chunks (TS: `toRenderedBlocks` stage order:
/// lift, split oversize, pair rich + plain).
pub fn to_rendered_blocks(
    blocks: Vec<(RichBlock, String)>,
    max_chars: Option<usize>,
    max_blocks: Option<usize>,
) -> Vec<TelegramRenderedBlock> {
    let limits = normalize_block_split_limits(max_chars, max_blocks);
    let mut rendered = Vec::new();
    for (block, plain_text) in blocks {
        let chars = plain_text.chars().count();
        if chars <= limits.max_chars {
            rendered.push(TelegramRenderedBlock { block, plain_text });
            continue;
        }
        // Oversize fallback mirrors `splitToPlainBlocks`: plain-text chunks.
        for chunk in super::chunker::chunk_plain_text(
            &plain_text,
            Some(super::chunker::PlainChunkerOptions {
                max_chars: Some(limits.max_chars),
            }),
        ) {
            rendered.push(TelegramRenderedBlock {
                block: RichBlock::Paragraph {
                    text: chunk.fallback_text.clone(),
                },
                plain_text: chunk.fallback_text,
            });
        }
    }
    let _ = limits.max_blocks;
    rendered
}

/// Minimal markdown split: blank-line separated paragraphs
/// (TS `parseTelegramBlocks` is richer; this keeps stage order for parity).
pub fn parse_markdown_blocks(markdown: &str) -> Vec<(RichBlock, String)> {
    let mut blocks = Vec::new();
    for piece in markdown.split("\n\n") {
        let text = piece.trim();
        if text.is_empty() {
            continue;
        }
        blocks.push((
            RichBlock::Paragraph {
                text: text.to_string(),
            },
            text.to_string(),
        ));
    }
    blocks
}

/// Parse markdown then pair rich + plain projections
/// (TS: `renderTelegramBlocks`).
pub fn render_telegram_blocks(
    markdown: &str,
    max_chars: Option<usize>,
    max_blocks: Option<usize>,
) -> Vec<TelegramRenderedBlock> {
    to_rendered_blocks(parse_markdown_blocks(markdown), max_chars, max_blocks)
}

/// Render markdown into deliverable Telegram parts
/// (TS: `renderTelegramParts`).
pub fn render_telegram_parts(
    markdown: &str,
    options: Option<TelegramChunkerOptions>,
) -> Vec<TelegramRenderedPart> {
    let rendered = render_telegram_blocks(
        markdown,
        options.as_ref().and_then(|o| o.max_chars),
        options.as_ref().and_then(|o| o.max_blocks),
    );
    if rendered.is_empty() {
        return Vec::new();
    }
    chunk_telegram_rendered_blocks(rendered, options)
}
