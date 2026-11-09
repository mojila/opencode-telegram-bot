//! Parity port of `tests/bot/render/chunker.test.ts`.
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).
//!
//! String-unit mapping: TS `string.length` counts UTF-16 code units while Rust
//! `String::len()` counts bytes and `chars().count()` counts Unicode scalar
//! values. Budget assertions therefore use `chars().count()`; the emoji test
//! asserts whole-scalar parts that rejoin exactly (Rust `String` cannot hold
//! a lone surrogate, so "never splits inside a surrogate pair" becomes "never
//! splits inside a scalar value / split parts are valid and lossless").

use otb_core::render::chunker::{
    chunk_plain_text, chunk_telegram_rendered_blocks, ListItem, PartSource, PlainChunkerOptions,
    RichBlock, TelegramChunkerOptions, TelegramRenderedBlock, PLAIN_MAX_PART_CHARS,
};

fn paragraph(text: &str) -> TelegramRenderedBlock {
    TelegramRenderedBlock {
        block: RichBlock::Paragraph {
            text: text.to_string(),
        },
        plain_text: text.to_string(),
    }
}

fn list_block(item_count: usize) -> TelegramRenderedBlock {
    let items = (0..item_count)
        .map(|index| ListItem {
            blocks: vec![RichBlock::Paragraph {
                text: format!("i{index}"),
            }],
        })
        .collect::<Vec<_>>();
    let plain_text = (0..item_count)
        .map(|index| format!("- i{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    TelegramRenderedBlock {
        block: RichBlock::List { items },
        plain_text,
    }
}

fn divider() -> TelegramRenderedBlock {
    TelegramRenderedBlock {
        block: RichBlock::Divider,
        plain_text: String::new(),
    }
}

// --- chunk_plain_text (TS: chunkPlainText) ---

#[test]
fn plain_keeps_short_text_as_single_part() {
    let parts = chunk_plain_text("short text", None);

    assert_eq!(parts.len(), 1);
    assert!(parts[0].blocks.is_empty());
    assert_eq!(parts[0].fallback_text, "short text");
    assert_eq!(parts[0].source, PartSource::Plain);
}

#[test]
fn plain_returns_no_parts_for_empty_text() {
    assert!(chunk_plain_text("", None).is_empty());
}

#[test]
fn plain_cuts_on_line_boundaries_and_stays_within_budget() {
    let line = format!("{}\n", "a".repeat(99));
    let text = line.repeat(60);

    let parts = chunk_plain_text(
        &text,
        Some(PlainChunkerOptions {
            max_chars: Some(1000),
        }),
    );

    assert!(parts.len() > 1);
    for part in &parts {
        assert!(part.fallback_text.chars().count() <= 1000);
        assert_eq!(part.source, PartSource::Plain);
        assert!(part.blocks.is_empty());
    }
    let joined = parts
        .iter()
        .map(|part| part.fallback_text.as_str())
        .collect::<String>();
    assert_eq!(joined, text);
    assert!(parts[0].fallback_text.ends_with('\n'));
}

#[test]
fn plain_uses_the_plain_budget_by_default() {
    let parts = chunk_plain_text(&"x".repeat(PLAIN_MAX_PART_CHARS * 2 + 5), None);

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].fallback_text.chars().count(), PLAIN_MAX_PART_CHARS);
}

#[test]
fn plain_never_splits_inside_emoji() {
    // TS asserts `fallbackText.length % 2 === 0` (UTF-16 units: U+1F600 is 2
    // units). Rust mapping: U+1F600 is 1 char / 4 bytes, and the budget unit
    // is chars, so exact part counts may differ from TS. Assert the property:
    // every part holds whole scalar values (byte length is a multiple of the
    // 4-byte emoji) and the parts rejoin losslessly.
    let text = "😀".repeat(10);
    let parts = chunk_plain_text(&text, Some(PlainChunkerOptions { max_chars: Some(5) }));

    assert!(!parts.is_empty());
    for part in &parts {
        assert_eq!(part.fallback_text.len() % 4, 0);
        assert!(part.fallback_text.chars().count() <= 5);
    }
    let joined = parts
        .iter()
        .map(|part| part.fallback_text.as_str())
        .collect::<String>();
    assert_eq!(joined, text);
}

// --- chunk_telegram_rendered_blocks (TS: chunkTelegramRenderedBlocks) ---

#[test]
fn blocks_pack_into_one_part_and_join_fallback_with_blank_lines() {
    let parts = chunk_telegram_rendered_blocks(vec![paragraph("one"), paragraph("two")], None);

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].source, PartSource::Blocks);
    assert_eq!(parts[0].blocks.len(), 2);
    assert_eq!(parts[0].fallback_text, "one\n\ntwo");
}

#[test]
fn blocks_return_no_parts_for_no_blocks() {
    assert!(chunk_telegram_rendered_blocks(vec![], None).is_empty());
}

#[test]
fn blocks_start_new_part_when_char_budget_would_overflow() {
    let parts = chunk_telegram_rendered_blocks(
        vec![paragraph(&"a".repeat(60)), paragraph(&"b".repeat(60))],
        Some(TelegramChunkerOptions {
            max_chars: Some(100),
            max_blocks: None,
        }),
    );

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].fallback_text, "a".repeat(60));
    assert_eq!(parts[1].fallback_text, "b".repeat(60));
}

#[test]
fn blocks_start_new_part_when_block_budget_would_overflow() {
    // Each list is 1 block + 2 units per item = 7 units for three items.
    let parts = chunk_telegram_rendered_blocks(
        vec![list_block(3), list_block(3)],
        Some(TelegramChunkerOptions {
            max_chars: Some(100_000),
            max_blocks: Some(10),
        }),
    );

    assert_eq!(parts.len(), 2);
}

#[test]
fn blocks_count_nested_units_rather_than_top_level_blocks() {
    let parts = chunk_telegram_rendered_blocks(
        vec![list_block(3), paragraph("tail")],
        Some(TelegramChunkerOptions {
            max_chars: Some(100_000),
            max_blocks: Some(7),
        }),
    );

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[1].fallback_text, "tail");
}

#[test]
fn blocks_keep_oversized_block_in_part_of_its_own() {
    let parts = chunk_telegram_rendered_blocks(
        vec![paragraph(&"a".repeat(500)), paragraph("b")],
        Some(TelegramChunkerOptions {
            max_chars: Some(100),
            max_blocks: None,
        }),
    );

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].fallback_text, "a".repeat(500));
    assert_eq!(parts[1].fallback_text, "b");
}

#[test]
fn blocks_skip_empty_plain_projections_when_joining_fallback() {
    let parts =
        chunk_telegram_rendered_blocks(vec![paragraph("one"), divider(), paragraph("two")], None);

    assert_eq!(parts[0].blocks.len(), 3);
    assert_eq!(parts[0].fallback_text, "one\n\ntwo");
}
