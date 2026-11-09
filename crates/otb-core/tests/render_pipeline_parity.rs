//! Parity port of `src/bot/render/pipeline.ts`
//! (`toRenderedBlocks` / `renderTelegramBlocks` / `renderTelegramParts`
//! with `src/bot/render/limits.ts` budgets).

use otb_core::render::chunker::{PartSource, TelegramChunkerOptions};
use otb_core::render::pipeline::{
    normalize_block_split_limits, parse_markdown_blocks, render_telegram_blocks,
    render_telegram_parts,
};

#[test]
fn pipeline_normalizes_split_limits_with_floor_and_defaults() {
    let limits = normalize_block_split_limits(None, None);
    assert_eq!(limits.max_chars, 32_000);
    assert_eq!(limits.max_blocks, 480);

    let floored = normalize_block_split_limits(Some(0), Some(0));
    assert_eq!(floored.max_chars, 1);
    assert_eq!(floored.max_blocks, 1);
}

#[test]
fn pipeline_parses_blank_line_paragraphs_and_skips_empty() {
    let blocks = parse_markdown_blocks("one\n\n\ntwo\n\n");
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].1, "one");
    assert_eq!(blocks[1].1, "two");
}

#[test]
fn pipeline_renders_blocks_pairing_rich_and_plain() {
    let rendered = render_telegram_blocks("hello", None, None);
    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].plain_text, "hello");
}

#[test]
fn pipeline_degrades_oversize_paragraph_to_plain_chunks() {
    let text = "a".repeat(120);
    let rendered = render_telegram_blocks(&text, Some(100), Some(480));
    assert!(rendered.len() >= 2);
    for block in &rendered {
        assert!(block.plain_text.chars().count() <= 100);
    }
    let joined: String = rendered.iter().map(|b| b.plain_text.as_str()).collect();
    assert_eq!(joined, text);
}

#[test]
fn pipeline_chunks_parts_within_char_budget() {
    let parts = render_telegram_parts(
        "one\n\ntwo",
        Some(TelegramChunkerOptions {
            max_chars: Some(3),
            max_blocks: None,
        }),
    );
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].fallback_text, "one");
    assert_eq!(parts[1].fallback_text, "two");
    assert_eq!(parts[0].source, PartSource::Blocks);
}

#[test]
fn pipeline_returns_no_parts_for_blank_markdown() {
    let parts = render_telegram_parts("   \n\n   ", None);
    assert!(parts.is_empty());
}
