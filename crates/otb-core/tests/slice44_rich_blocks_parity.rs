//! Parity port of the rich-block conversion pure gates (slice 44, RED).
//!
//! Mirrors representative cases from:
//! - `src/bot/render/rich-blocks.ts` (every `toRichText` / `toRichBlock`
//!   branch, `isTextRenderedList`, `textListMarker`, `liftTextListContent`,
//!   wide-table `pre` fallback, empty-table paragraph, heading clamp)
//! - `src/bot/render/part-signature.ts`
//!   (`getTelegramRenderedPartSignature`: plain vs blocks source, empty
//!   blocks, entity encoding)
//! - `src/bot/render/limits.ts` (budget constants)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::rich_blocks::*;

// --- helpers ---

fn text(s: &str) -> InlineNode {
    InlineNode::Text(s.to_string())
}

fn para(s: &str) -> TelegramBlock {
    TelegramBlock::Paragraph {
        inlines: vec![text(s)],
    }
}

fn item(blocks: Vec<TelegramBlock>, checked: Option<bool>) -> TelegramListItem {
    TelegramListItem { blocks, checked }
}

fn plain_item(s: &str) -> TelegramListItem {
    item(vec![para(s)], None)
}

// --- is_text_rendered_list (01-06) ---

#[test]
fn slice44_01_unordered_plain_list_is_native() {
    let items = vec![plain_item("a"), plain_item("b")];
    assert!(!is_text_rendered_list(false, &items));
}

#[test]
fn slice44_02_ordered_list_is_text_rendered() {
    let items = vec![plain_item("a")];
    assert!(is_text_rendered_list(true, &items));
}

#[test]
fn slice44_03_checked_task_list_is_text_rendered() {
    let items = vec![item(vec![para("done")], Some(true))];
    assert!(is_text_rendered_list(false, &items));
}

#[test]
fn slice44_04_unchecked_task_list_is_text_rendered() {
    let items = vec![item(vec![para("todo")], Some(false))];
    assert!(is_text_rendered_list(false, &items));
}

#[test]
fn slice44_05_mixed_list_with_one_task_is_text_rendered() {
    let items = vec![plain_item("a"), item(vec![para("b")], Some(true))];
    assert!(is_text_rendered_list(false, &items));
}

#[test]
fn slice44_06_empty_unordered_list_is_native() {
    assert!(!is_text_rendered_list(false, &[]));
}

// --- text_list_marker (07-14) ---

#[test]
fn slice44_07_unordered_plain_marker_is_bullet() {
    assert_eq!(text_list_marker(&plain_item("a"), 0, false, None), "• ");
}

#[test]
fn slice44_08_unordered_checked_marker_is_check() {
    let it = item(vec![para("a")], Some(true));
    assert_eq!(text_list_marker(&it, 0, false, None), "✅ ");
}

#[test]
fn slice44_09_unordered_unchecked_marker_is_box() {
    let it = item(vec![para("a")], Some(false));
    assert_eq!(text_list_marker(&it, 0, false, None), "🔲 ");
}

#[test]
fn slice44_10_ordered_marker_counts_from_one() {
    assert_eq!(text_list_marker(&plain_item("a"), 0, true, None), "1. ");
}

#[test]
fn slice44_11_ordered_marker_uses_index_and_start() {
    assert_eq!(text_list_marker(&plain_item("a"), 2, true, Some(5)), "7. ");
}

#[test]
fn slice44_12_ordered_task_marker_combines_count_and_state() {
    let it = item(vec![para("a")], Some(true));
    assert_eq!(text_list_marker(&it, 1, true, None), "2. ✅ ");
}

#[test]
fn slice44_13_ordered_untasked_marker_combines_count_and_box() {
    let it = item(vec![para("a")], Some(false));
    assert_eq!(text_list_marker(&it, 0, true, Some(1)), "1. 🔲 ");
}

#[test]
fn slice44_14_marker_index_ignores_item_content() {
    assert_eq!(
        text_list_marker(&plain_item("zzz"), 4, true, Some(10)),
        "14. "
    );
}

// --- to_rich_text (15-24) ---

#[test]
fn slice44_15_empty_nodes_become_empty_string() {
    assert_eq!(to_rich_text(&[]), RichText::Str(String::new()));
}

#[test]
fn slice44_16_single_text_node_collapses() {
    assert_eq!(to_rich_text(&[text("hi")]), RichText::Str("hi".to_string()));
}

#[test]
fn slice44_17_empty_text_nodes_are_dropped() {
    assert_eq!(
        to_rich_text(&[text(""), text("hi")]),
        RichText::Str("hi".to_string())
    );
}

#[test]
fn slice44_18_all_empty_nodes_become_empty_string() {
    assert_eq!(to_rich_text(&[text("")]), RichText::Str(String::new()));
}

#[test]
fn slice44_19_two_text_nodes_become_array() {
    assert_eq!(
        to_rich_text(&[text("a"), text("b")]),
        RichText::Arr(vec![
            RichText::Str("a".to_string()),
            RichText::Str("b".to_string())
        ])
    );
}

#[test]
fn slice44_20_bold_wraps_children() {
    assert_eq!(
        to_rich_text(&[InlineNode::Bold(vec![text("b")])]),
        RichText::Styled {
            kind: "bold".to_string(),
            text: Box::new(RichText::Str("b".to_string()))
        }
    );
}

#[test]
fn slice44_21_strike_maps_to_strikethrough() {
    assert_eq!(
        to_rich_text(&[InlineNode::Strike(vec![text("s")])]),
        RichText::Styled {
            kind: "strikethrough".to_string(),
            text: Box::new(RichText::Str("s".to_string()))
        }
    );
}

#[test]
fn slice44_22_code_node_keeps_text() {
    assert_eq!(
        to_rich_text(&[InlineNode::Code("x+1".to_string())]),
        RichText::Styled {
            kind: "code".to_string(),
            text: Box::new(RichText::Str("x+1".to_string()))
        }
    );
}

#[test]
fn slice44_23_link_keeps_url() {
    assert_eq!(
        to_rich_text(&[InlineNode::Link {
            text: vec![text("t")],
            url: "https://x.test".to_string()
        }]),
        RichText::Url {
            text: Box::new(RichText::Str("t".to_string())),
            url: "https://x.test".to_string()
        }
    );
}

#[test]
fn slice44_24_mixed_nodes_become_array_with_styles() {
    let out = to_rich_text(&[text("a"), InlineNode::Italic(vec![text("b")])]);
    assert_eq!(
        out,
        RichText::Arr(vec![
            RichText::Str("a".to_string()),
            RichText::Styled {
                kind: "italic".to_string(),
                text: Box::new(RichText::Str("b".to_string()))
            },
        ])
    );
}

// --- to_rich_block (25-36) ---

#[test]
fn slice44_25_paragraph_converts_text() {
    assert_eq!(
        to_rich_block(&para("hello")),
        RichBlock::Paragraph {
            text: RichText::Str("hello".to_string())
        }
    );
}

#[test]
fn slice44_26_heading_keeps_level_in_range() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Heading {
            level: 3,
            inlines: vec![text("t")]
        }),
        RichBlock::Heading {
            size: 3,
            text: RichText::Str("t".to_string())
        }
    );
}

#[test]
fn slice44_27_heading_level_clamps_to_six() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Heading {
            level: 9,
            inlines: vec![text("t")]
        }),
        RichBlock::Heading {
            size: 6,
            text: RichText::Str("t".to_string())
        }
    );
}

#[test]
fn slice44_28_heading_level_clamps_to_one() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Heading {
            level: 0,
            inlines: vec![text("t")]
        }),
        RichBlock::Heading {
            size: 1,
            text: RichText::Str("t".to_string())
        }
    );
}

#[test]
fn slice44_29_code_with_language_stays_pre() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Code {
            language: Some("ts".to_string()),
            text: "x".to_string()
        }),
        RichBlock::Pre {
            language: Some("ts".to_string()),
            text: RichText::Str("x".to_string())
        }
    );
}

#[test]
fn slice44_30_code_without_language_has_no_language() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Code {
            language: None,
            text: "x".to_string()
        }),
        RichBlock::Pre {
            language: None,
            text: RichText::Str("x".to_string())
        }
    );
}

#[test]
fn slice44_31_rule_becomes_divider() {
    assert_eq!(to_rich_block(&TelegramBlock::Rule), RichBlock::Divider);
}

#[test]
fn slice44_32_plain_becomes_paragraph() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Plain {
            text: "p".to_string()
        }),
        RichBlock::Paragraph {
            text: RichText::Str("p".to_string())
        }
    );
}

#[test]
fn slice44_33_table_marks_header_and_defaults_align() {
    let out = to_rich_block(&TelegramBlock::Table {
        rows: vec![vec!["h".to_string()], vec!["c".to_string()]],
        align: None,
    });
    assert_eq!(
        out,
        RichBlock::Table {
            cells: vec![
                vec![RichTableCell {
                    text: "h".to_string(),
                    is_header: true,
                    align: TableAlign::Left
                }],
                vec![RichTableCell {
                    text: "c".to_string(),
                    is_header: false,
                    align: TableAlign::Left
                }],
            ],
            is_bordered: true,
        }
    );
}

#[test]
fn slice44_34_wide_table_falls_back_to_pre() {
    let wide: Vec<String> = (0..21).map(|i| format!("c{i}")).collect();
    let out = to_rich_block(&TelegramBlock::Table {
        rows: vec![wide],
        align: None,
    });
    assert!(
        matches!(out, RichBlock::Pre { .. }),
        "21-column table must degrade to pre, got {out:?}"
    );
}

#[test]
fn slice44_35_empty_table_becomes_empty_paragraph() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Table {
            rows: vec![],
            align: None
        }),
        RichBlock::Paragraph {
            text: RichText::Str(String::new())
        }
    );
}

#[test]
fn slice44_36_blockquote_converts_children() {
    assert_eq!(
        to_rich_block(&TelegramBlock::Blockquote {
            blocks: vec![para("q")]
        }),
        RichBlock::Blockquote {
            blocks: vec![RichBlock::Paragraph {
                text: RichText::Str("q".to_string())
            }]
        }
    );
}

// --- lift_text_list_content (37-42) ---

#[test]
fn slice44_37_non_list_blocks_pass_through() {
    let blocks = vec![para("a"), TelegramBlock::Rule];
    assert_eq!(lift_text_list_content(&blocks), blocks);
}

#[test]
fn slice44_38_text_only_ordered_list_stays_one_piece() {
    let block = TelegramBlock::List {
        ordered: true,
        items: vec![plain_item("a"), plain_item("b")],
        start: Some(1),
    };
    let out = lift_text_list_content(std::slice::from_ref(&block));
    assert_eq!(out, vec![block]);
}

#[test]
fn slice44_39_native_list_passes_through_untouched() {
    let block = TelegramBlock::List {
        ordered: false,
        items: vec![plain_item("a")],
        start: None,
    };
    let out = lift_text_list_content(std::slice::from_ref(&block));
    assert_eq!(out, vec![block]);
}

#[test]
fn slice44_40_trailing_code_block_is_lifted_out() {
    let block = TelegramBlock::List {
        ordered: true,
        items: vec![item(
            vec![
                para("lead"),
                TelegramBlock::Code {
                    language: None,
                    text: "x".to_string(),
                },
            ],
            None,
        )],
        start: Some(1),
    };
    let out = lift_text_list_content(std::slice::from_ref(&block));
    assert_eq!(out.len(), 2);
    assert!(matches!(out[0], TelegramBlock::List { .. }));
    assert!(matches!(out[1], TelegramBlock::Code { .. }));
}

#[test]
fn slice44_41_numbering_continues_across_lift() {
    let block = TelegramBlock::List {
        ordered: true,
        items: vec![
            item(
                vec![
                    para("one"),
                    TelegramBlock::Code {
                        language: None,
                        text: "x".to_string(),
                    },
                ],
                None,
            ),
            plain_item("two"),
        ],
        start: Some(1),
    };
    let out = lift_text_list_content(std::slice::from_ref(&block));
    let starts: Vec<u64> = out
        .iter()
        .filter_map(|b| match b {
            TelegramBlock::List { start, .. } => Some(start.unwrap_or(1)),
            _ => None,
        })
        .collect();
    assert_eq!(starts, vec![1, 2]);
}

#[test]
fn slice44_42_empty_input_stays_empty() {
    assert!(lift_text_list_content(&[]).is_empty());
}

// --- part signature (43-48) ---

fn plain_part(fallback: &str) -> RenderedPart {
    RenderedPart {
        blocks: vec![],
        fallback_text: fallback.to_string(),
        source: PartSource::Plain,
        entities: vec![],
    }
}

#[test]
fn slice44_43_plain_source_signature_without_entities() {
    let sig = get_telegram_rendered_part_signature(&plain_part("hello"));
    assert_eq!(sig, "plain\n\nhello");
}

#[test]
fn slice44_44_plain_signature_encodes_entities() {
    let part = RenderedPart {
        entities: vec![PartEntity {
            kind: "bold".to_string(),
            offset: 0,
            length: 5,
        }],
        ..plain_part("hello")
    };
    let sig = get_telegram_rendered_part_signature(&part);
    assert_eq!(
        sig,
        "plain\n[{\"type\":\"bold\",\"offset\":0,\"length\":5}]\nhello"
    );
}

#[test]
fn slice44_45_empty_blocks_use_plain_branch() {
    let part = RenderedPart {
        source: PartSource::Blocks,
        ..plain_part("fb")
    };
    assert_eq!(get_telegram_rendered_part_signature(&part), "plain\n\nfb");
}

#[test]
fn slice44_46_blocks_source_prefixes_blocks_json() {
    let part = RenderedPart {
        blocks: vec![RichBlock::Paragraph {
            text: RichText::Str("hi".to_string()),
        }],
        fallback_text: "hi".to_string(),
        source: PartSource::Blocks,
        entities: vec![],
    };
    let sig = get_telegram_rendered_part_signature(&part);
    assert_eq!(sig, "blocks\n[{\"type\":\"paragraph\",\"text\":\"hi\"}]");
}

#[test]
fn slice44_47_blocks_signature_ignores_entities() {
    let part = RenderedPart {
        blocks: vec![RichBlock::Divider],
        fallback_text: "fb".to_string(),
        source: PartSource::Blocks,
        entities: vec![PartEntity {
            kind: "bold".to_string(),
            offset: 0,
            length: 1,
        }],
    };
    assert_eq!(
        get_telegram_rendered_part_signature(&part),
        "blocks\n[{\"type\":\"divider\"}]"
    );
}

#[test]
fn slice44_48_create_paragraph_block_round_trips() {
    assert_eq!(
        create_paragraph_block("t"),
        RichBlock::Paragraph {
            text: RichText::Str("t".to_string())
        }
    );
}
