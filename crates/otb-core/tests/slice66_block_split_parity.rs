//! Slice 66 parity: block-split gates (RED).
//!
//! Mirrors the pure TS contracts in `src/bot/render/block-splitter.ts`
//! (`normalizeBlockSplitLimits`, `splitOversizeTelegramBlock`,
//! `splitOversizeTelegramBlocks`, per-type code/table/quote splits),
//! `src/bot/render/text-splitter.ts` (`splitTextIntoChunks`), and
//! `src/bot/render/limits.ts` (budgets).
//!
//! Every test MUST FAIL until `block_split_gates` is ported (step 2).

use otb_core::block_split_gates::*;

// --- budgets ---

#[test]
fn slice66_default_max_part_chars_parity() {
    assert_eq!(DEFAULT_MAX_PART_CHARS, 32_000);
}

#[test]
fn slice66_default_max_part_blocks_parity() {
    assert_eq!(DEFAULT_MAX_PART_BLOCKS, 480);
}

#[test]
fn slice66_plain_max_part_chars_parity() {
    assert_eq!(PLAIN_MAX_PART_CHARS, 3_800);
}

#[test]
fn slice66_telegram_text_message_limit_parity() {
    assert_eq!(TELEGRAM_TEXT_MESSAGE_LIMIT, 4_096);
}

// --- normalize_block_split_limits ---

#[test]
fn slice66_normalize_defaults_parity() {
    assert_eq!(
        normalize_block_split_limits(None, None),
        BlockSplitLimits {
            max_chars: 32_000,
            max_blocks: 480
        }
    );
}

#[test]
fn slice66_normalize_explicit_parity() {
    assert_eq!(
        normalize_block_split_limits(Some(100.0), Some(10.0)),
        BlockSplitLimits {
            max_chars: 100,
            max_blocks: 10
        }
    );
}

#[test]
fn slice66_normalize_floors_fraction_parity() {
    assert_eq!(
        normalize_block_split_limits(Some(100.9), Some(10.9)),
        BlockSplitLimits {
            max_chars: 100,
            max_blocks: 10
        }
    );
}

#[test]
fn slice66_normalize_clamps_zero_to_one_parity() {
    assert_eq!(
        normalize_block_split_limits(Some(0.0), Some(0.0)),
        BlockSplitLimits {
            max_chars: 1,
            max_blocks: 1
        }
    );
}

#[test]
fn slice66_normalize_clamps_negative_to_one_parity() {
    assert_eq!(
        normalize_block_split_limits(Some(-5.0), Some(-2.0)),
        BlockSplitLimits {
            max_chars: 1,
            max_blocks: 1
        }
    );
}

#[test]
fn slice66_normalize_partial_chars_only_parity() {
    assert_eq!(
        normalize_block_split_limits(Some(50.0), None),
        BlockSplitLimits {
            max_chars: 50,
            max_blocks: 480
        }
    );
}

#[test]
fn slice66_normalize_partial_blocks_only_parity() {
    assert_eq!(
        normalize_block_split_limits(None, Some(7.0)),
        BlockSplitLimits {
            max_chars: 32_000,
            max_blocks: 7
        }
    );
}

#[test]
fn slice66_normalize_fraction_below_one_clamps_parity() {
    assert_eq!(
        normalize_block_split_limits(Some(0.5), Some(0.9)),
        BlockSplitLimits {
            max_chars: 1,
            max_blocks: 1
        }
    );
}

// --- split_text_into_chunks ---

#[test]
fn slice66_chunks_empty_returns_none_parity() {
    assert_eq!(split_text_into_chunks("", 10.0), Vec::<String>::new());
}

#[test]
fn slice66_chunks_short_returns_single_parity() {
    assert_eq!(
        split_text_into_chunks("hello", 10.0),
        vec!["hello".to_string()]
    );
}

#[test]
fn slice66_chunks_exact_limit_single_parity() {
    assert_eq!(
        split_text_into_chunks("abcde", 5.0),
        vec!["abcde".to_string()]
    );
}

#[test]
fn slice66_chunks_prefers_newline_parity() {
    assert_eq!(
        split_text_into_chunks("ab\ncdefgh", 5.0),
        vec!["ab\n".to_string(), "cdefg".to_string(), "h".to_string()]
    );
}

#[test]
fn slice66_chunks_prefers_whitespace_parity() {
    assert_eq!(
        split_text_into_chunks("ab cd ef", 5.0),
        vec!["ab ".to_string(), "cd ".to_string(), "ef".to_string()]
    );
}

#[test]
fn slice66_chunks_hard_cut_without_spaces_parity() {
    assert_eq!(
        split_text_into_chunks("abcdefgh", 3.0),
        vec!["abc".to_string(), "def".to_string(), "gh".to_string()]
    );
}

#[test]
fn slice66_chunks_limit_floors_parity() {
    assert_eq!(
        split_text_into_chunks("abcdef", 3.9),
        vec!["abc".to_string(), "def".to_string()]
    );
}

#[test]
fn slice66_chunks_limit_zero_clamps_to_one_parity() {
    assert_eq!(
        split_text_into_chunks("ab", 0.0),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[test]
fn slice66_chunks_rejoin_exactly_parity() {
    let text = "line one\nline two is longer than limit here";
    let chunks = split_text_into_chunks(text, 10.0);
    assert_eq!(chunks.concat(), text);
}

#[test]
fn slice66_chunks_each_within_limit_parity() {
    let chunks = split_text_into_chunks("aa bb cc dd ee ff", 5.0);
    assert!(chunks.iter().all(|c| c.chars().count() <= 5));
}

#[test]
fn slice66_chunks_single_long_word_splits_parity() {
    let chunks = split_text_into_chunks("abcdefghij", 4.0);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks.concat(), "abcdefghij");
}

#[test]
fn slice66_chunks_trailing_newline_kept_parity() {
    let chunks = split_text_into_chunks("ab\ncd\n", 3.0);
    assert_eq!(chunks.concat(), "ab\ncd\n");
}

// --- block_fits_limits ---

#[test]
fn slice66_fits_small_paragraph_parity() {
    let block = TelegramBlock::Paragraph {
        text: "hi".to_string(),
    };
    let limits = BlockSplitLimits {
        max_chars: 32_000,
        max_blocks: 480,
    };
    assert!(block_fits_limits(&block, &limits));
}

#[test]
fn slice66_fits_oversize_code_rejected_parity() {
    let block = TelegramBlock::Code {
        language: None,
        text: "x".repeat(100),
    };
    let limits = BlockSplitLimits {
        max_chars: 10,
        max_blocks: 480,
    };
    assert!(!block_fits_limits(&block, &limits));
}

#[test]
fn slice66_fits_rule_parity() {
    let block = TelegramBlock::Rule;
    let limits = BlockSplitLimits {
        max_chars: 32_000,
        max_blocks: 480,
    };
    assert!(block_fits_limits(&block, &limits));
}

// --- split_code_block ---

#[test]
fn slice66_code_short_single_block_parity() {
    let limits = BlockSplitLimits {
        max_chars: 100,
        max_blocks: 480,
    };
    assert_eq!(
        split_code_block("a\nb", Some("rust"), &limits),
        vec![TelegramBlock::Code {
            language: Some("rust".to_string()),
            text: "a\nb".to_string()
        }]
    );
}

#[test]
fn slice66_code_keeps_language_per_piece_parity() {
    let limits = BlockSplitLimits {
        max_chars: 4,
        max_blocks: 480,
    };
    let pieces = split_code_block("ab\ncdef\ngh", Some("js"), &limits);
    assert!(pieces.len() >= 2);
    for piece in &pieces {
        match piece {
            TelegramBlock::Code { language, .. } => {
                assert_eq!(language.as_deref(), Some("js"));
            }
            other => panic!("expected code piece, got {other:?}"),
        }
    }
}

#[test]
fn slice66_code_no_language_stays_none_parity() {
    let limits = BlockSplitLimits {
        max_chars: 100,
        max_blocks: 480,
    };
    assert_eq!(
        split_code_block("hi", None, &limits),
        vec![TelegramBlock::Code {
            language: None,
            text: "hi".to_string()
        }]
    );
}

#[test]
fn slice66_code_long_line_chunked_parity() {
    let limits = BlockSplitLimits {
        max_chars: 4,
        max_blocks: 480,
    };
    let pieces = split_code_block(&"x".repeat(10), None, &limits);
    assert!(pieces.len() >= 2);
    assert_eq!(
        pieces
            .iter()
            .map(|b| match b {
                TelegramBlock::Code { text, .. } => text.clone(),
                other => panic!("expected code, got {other:?}"),
            })
            .collect::<String>(),
        "x".repeat(10)
    );
}

// --- split_table_block ---

#[test]
fn slice66_table_single_header_only_parity() {
    let limits = BlockSplitLimits {
        max_chars: 100,
        max_blocks: 480,
    };
    let rows = vec![vec!["h1".to_string(), "h2".to_string()]];
    assert_eq!(
        split_table_block(&rows, None, &limits),
        vec![TelegramBlock::Table { rows, align: None }]
    );
}

#[test]
fn slice66_table_empty_rows_empty_parity() {
    let limits = BlockSplitLimits {
        max_chars: 100,
        max_blocks: 480,
    };
    let rows: Vec<Vec<String>> = vec![];
    assert_eq!(split_table_block(&rows, None, &limits), vec![]);
}

#[test]
fn slice66_table_header_repeated_per_group_parity() {
    let limits = BlockSplitLimits {
        max_chars: 6,
        max_blocks: 3,
    };
    let rows = vec![
        vec!["h".to_string()],
        vec!["a".to_string()],
        vec!["b".to_string()],
        vec!["c".to_string()],
    ];
    let pieces = split_table_block(&rows, None, &limits);
    assert!(pieces.len() >= 2);
    for piece in &pieces {
        match piece {
            TelegramBlock::Table { rows, .. } => {
                assert_eq!(rows[0], vec!["h".to_string()]);
            }
            other => panic!("expected table, got {other:?}"),
        }
    }
}

#[test]
fn slice66_table_keeps_align_parity() {
    let limits = BlockSplitLimits {
        max_chars: 6,
        max_blocks: 3,
    };
    let rows = vec![
        vec!["h".to_string()],
        vec!["a".to_string()],
        vec!["b".to_string()],
    ];
    let align = vec!["center".to_string()];
    let pieces = split_table_block(&rows, Some(align.as_slice()), &limits);
    for piece in &pieces {
        match piece {
            TelegramBlock::Table { align, .. } => {
                assert_eq!(align.as_ref(), Some(&vec!["center".to_string()]));
            }
            other => panic!("expected table, got {other:?}"),
        }
    }
}

// --- split_blockquote_block ---

#[test]
fn slice66_quote_short_single_quote_parity() {
    let limits = BlockSplitLimits {
        max_chars: 100,
        max_blocks: 480,
    };
    let inner = vec![TelegramBlock::Paragraph {
        text: "hi".to_string(),
    }];
    assert_eq!(
        split_blockquote_block(&inner, &limits),
        vec![TelegramBlock::Blockquote { blocks: inner }]
    );
}

#[test]
fn slice66_quote_inner_blocks_preserved_parity() {
    let limits = BlockSplitLimits {
        max_chars: 100,
        max_blocks: 480,
    };
    let inner = vec![
        TelegramBlock::Paragraph {
            text: "a".to_string(),
        },
        TelegramBlock::Paragraph {
            text: "b".to_string(),
        },
    ];
    let pieces = split_blockquote_block(&inner, &limits);
    assert_eq!(pieces.len(), 1);
    match &pieces[0] {
        TelegramBlock::Blockquote { blocks } => assert_eq!(blocks, &inner),
        other => panic!("expected quote, got {other:?}"),
    }
}

// --- split_oversize_telegram_block ---

#[test]
fn slice66_oversize_fitting_returns_same_parity() {
    let limits = BlockSplitLimits {
        max_chars: 100,
        max_blocks: 480,
    };
    let block = TelegramBlock::Paragraph {
        text: "hi".to_string(),
    };
    assert_eq!(split_oversize_telegram_block(&block, &limits), vec![block]);
}

#[test]
fn slice66_oversize_code_splits_parity() {
    let limits = BlockSplitLimits {
        max_chars: 4,
        max_blocks: 480,
    };
    let block = TelegramBlock::Code {
        language: None,
        text: "ab\ncdef\nghij".to_string(),
    };
    let pieces = split_oversize_telegram_block(&block, &limits);
    assert!(pieces.len() >= 2);
}

#[test]
fn slice66_oversize_single_row_table_degrades_plain_parity() {
    let limits = BlockSplitLimits {
        max_chars: 3,
        max_blocks: 480,
    };
    let block = TelegramBlock::Table {
        rows: vec![vec!["abcdef".to_string()]],
        align: None,
    };
    let pieces = split_oversize_telegram_block(&block, &limits);
    assert!(pieces
        .iter()
        .all(|b| matches!(b, TelegramBlock::Plain { .. })));
}

#[test]
fn slice66_oversize_plain_splits_to_plain_parity() {
    let limits = BlockSplitLimits {
        max_chars: 4,
        max_blocks: 480,
    };
    let block = TelegramBlock::Plain {
        text: "abcdefgh".to_string(),
    };
    let pieces = split_oversize_telegram_block(&block, &limits);
    assert!(pieces.len() >= 2);
    assert!(pieces
        .iter()
        .all(|b| matches!(b, TelegramBlock::Plain { .. })));
}

#[test]
fn slice66_oversize_list_splits_by_items_parity() {
    let limits = BlockSplitLimits {
        max_chars: 4,
        max_blocks: 480,
    };
    let item = |t: &str| TelegramListItem {
        blocks: vec![TelegramBlock::Paragraph {
            text: t.to_string(),
        }],
        checked: None,
    };
    let block = TelegramBlock::List {
        ordered: false,
        start: None,
        items: vec![item("aaaa"), item("bbbb"), item("cccc")],
    };
    let pieces = split_oversize_telegram_block(&block, &limits);
    assert!(pieces.len() >= 2);
}

#[test]
fn slice66_oversize_quote_splits_parity() {
    let limits = BlockSplitLimits {
        max_chars: 4,
        max_blocks: 2,
    };
    let block = TelegramBlock::Blockquote {
        blocks: vec![
            TelegramBlock::Paragraph {
                text: "aaaaaaaa".to_string(),
            },
            TelegramBlock::Paragraph {
                text: "bbbbbbbb".to_string(),
            },
        ],
    };
    let pieces = split_oversize_telegram_block(&block, &limits);
    assert!(pieces
        .iter()
        .all(|b| matches!(b, TelegramBlock::Blockquote { .. })));
}

// --- split_oversize_telegram_blocks ---

#[test]
fn slice66_blocks_empty_returns_empty_parity() {
    assert_eq!(split_oversize_telegram_blocks(&[], None), vec![]);
}

#[test]
fn slice66_blocks_default_limits_used_parity() {
    let blocks = vec![TelegramBlock::Paragraph {
        text: "hi".to_string(),
    }];
    assert_eq!(split_oversize_telegram_blocks(&blocks, None), blocks);
}

#[test]
fn slice66_blocks_custom_limits_split_parity() {
    let blocks = vec![TelegramBlock::Code {
        language: None,
        text: "ab\ncdef\nghij".to_string(),
    }];
    let limits = Some(BlockSplitLimits {
        max_chars: 4,
        max_blocks: 480,
    });
    let pieces = split_oversize_telegram_blocks(&blocks, limits);
    assert!(pieces.len() >= 2);
}

#[test]
fn slice66_blocks_order_preserved_parity() {
    let blocks = vec![
        TelegramBlock::Paragraph {
            text: "first".to_string(),
        },
        TelegramBlock::Paragraph {
            text: "second".to_string(),
        },
    ];
    let out = split_oversize_telegram_blocks(&blocks, None);
    assert_eq!(out, blocks);
}
