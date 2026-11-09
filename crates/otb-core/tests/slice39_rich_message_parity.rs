//! Parity port of rich-message normalize pure logic (slice 39, RED).
//!
//! Mirrors representative cases from:
//! - `tests/bot/handlers/rich-message.test.ts` (`convertRichMessage`
//!   structured/markdown, nested photos + skipped count, empty-blocks
//!   routing, slash-command entity exposure)
//! - `tests/bot/rich-message-routing.test.ts` (plain sentence routing,
//!   slash command routing, photo-only prompt, goal-input photo refusal)
//! - `src/bot/handlers/rich-message-handler.ts` (every `renderBlock` /
//!   `renderRichText` / `plainRichText` branch, caption/table/quote/indent
//!   helpers, escape helpers, token + skip-notice gates)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::rich_message::*;

fn plain(s: &str) -> RichText {
    RichText::Plain(s.to_string())
}

fn photo_sizes() -> Vec<PhotoSize> {
    vec![
        PhotoSize {
            file_id: "p-small".to_string(),
            file_size: None,
        },
        PhotoSize {
            file_id: "p-large".to_string(),
            file_size: Some(2048),
        },
    ]
}

// --- convert core (6) ---

#[test]
fn slice39_01_plain_paragraph_passes_through() {
    let out = convert_rich_message(
        &[RichBlock::Paragraph(plain("How are you feeling today?"))],
        0,
    );
    assert_eq!(out.text, "How are you feeling today?");
    assert!(out.photos.is_empty());
    assert_eq!(out.skipped_media_count, 0);
}

#[test]
fn slice39_02_empty_blocks_convert_to_empty() {
    let out = convert_rich_message(&[], 7);
    assert_eq!(out.text, "");
    assert!(out.photos.is_empty());
    assert_eq!(out.skipped_media_count, 0);
}

#[test]
fn slice39_03_blocks_join_with_blank_line() {
    let out = convert_rich_message(
        &[
            RichBlock::Paragraph(plain("a")),
            RichBlock::Paragraph(plain("b")),
        ],
        0,
    );
    assert_eq!(out.text, "a\n\nb");
}

#[test]
fn slice39_04_output_is_trimmed() {
    let out = convert_rich_message(&[RichBlock::Paragraph(plain("  hi  "))], 0);
    assert_eq!(out.text, "hi");
}

#[test]
fn slice39_05_photo_only_yields_empty_text_plus_photo() {
    let out = convert_rich_message(
        &[RichBlock::Photo {
            sizes: photo_sizes(),
            caption: None,
        }],
        42,
    );
    assert_eq!(out.text, "");
    assert_eq!(out.photos.len(), 1);
    assert_eq!(out.skipped_media_count, 0);
}

#[test]
fn slice39_06_skipped_only_yields_empty_text_plus_count() {
    let out = convert_rich_message(
        &[
            RichBlock::SkippedMedia { caption: None },
            RichBlock::SkippedMedia { caption: None },
        ],
        0,
    );
    assert_eq!(out.text, "");
    assert_eq!(out.skipped_media_count, 2);
}

// --- heading / pre / divider / math / anchor (8) ---

#[test]
fn slice39_07_heading_prefixes_hashes_by_size() {
    assert_eq!(
        convert_rich_message(
            &[RichBlock::Heading {
                size: 2,
                text: plain("Report")
            }],
            0
        )
        .text,
        "## Report"
    );
}

#[test]
fn slice39_08_heading_level_one_and_three() {
    assert_eq!(
        convert_rich_message(
            &[RichBlock::Heading {
                size: 1,
                text: plain("T")
            }],
            0
        )
        .text,
        "# T"
    );
    assert_eq!(
        convert_rich_message(
            &[RichBlock::Heading {
                size: 3,
                text: plain("T")
            }],
            0
        )
        .text,
        "### T"
    );
}

#[test]
fn slice39_09_pre_wraps_fence_with_language() {
    let out = convert_rich_message(
        &[RichBlock::Pre {
            language: Some("ts".to_string()),
            text: plain("const value = 1;"),
        }],
        0,
    );
    assert_eq!(out.text, "```ts\nconst value = 1;\n```");
}

#[test]
fn slice39_10_pre_without_language_uses_bare_fence() {
    let out = convert_rich_message(
        &[RichBlock::Pre {
            language: None,
            text: plain("x"),
        }],
        0,
    );
    assert_eq!(out.text, "```\nx\n```");
}

#[test]
fn slice39_11_pre_with_triple_backticks_uses_quad_fence() {
    let out = convert_rich_message(
        &[RichBlock::Pre {
            language: Some("ts".to_string()),
            text: plain("a ``` b"),
        }],
        0,
    );
    assert!(out.text.starts_with("````ts\n"));
    assert!(out.text.ends_with("\n````"));
}

#[test]
fn slice39_12_divider_renders_rule() {
    assert_eq!(convert_rich_message(&[RichBlock::Divider], 0).text, "---");
}

#[test]
fn slice39_13_math_block_wraps_dollars() {
    assert_eq!(
        convert_rich_message(
            &[RichBlock::Math {
                expression: "E = mc^2".to_string()
            }],
            0
        )
        .text,
        "$$\nE = mc^2\n$$"
    );
}

#[test]
fn slice39_14_anchor_renders_named_tag() {
    assert_eq!(
        convert_rich_message(
            &[RichBlock::Anchor {
                name: "sec".to_string()
            }],
            0
        )
        .text,
        "<a name=\"sec\"></a>"
    );
}

// --- lists / quotes / collage / table / details (10) ---

#[test]
fn slice39_15_list_plain_items_use_dash() {
    let out = convert_rich_message(
        &[RichBlock::List {
            items: vec![RichListItem {
                has_checkbox: false,
                is_checked: false,
                label: "-".to_string(),
                blocks: vec![RichBlock::Paragraph(plain("first"))],
            }],
        }],
        0,
    );
    assert_eq!(out.text, "- first");
}

#[test]
fn slice39_16_list_checked_box_marker() {
    assert_eq!(list_item_marker(true, true, "-"), "- [x]");
}

#[test]
fn slice39_17_list_unchecked_box_marker() {
    assert_eq!(list_item_marker(true, false, "-"), "- [ ]");
}

#[test]
fn slice39_18_list_custom_label_marker() {
    assert_eq!(list_item_marker(false, false, "*"), "*");
    assert_eq!(list_item_marker(false, false, ""), "-");
}

#[test]
fn slice39_19_blockquote_prefixes_lines() {
    let out = convert_rich_message(
        &[RichBlock::Blockquote {
            blocks: vec![RichBlock::Paragraph(plain("quoted"))],
            credit: None,
        }],
        0,
    );
    assert_eq!(out.text, "> quoted");
}

#[test]
fn slice39_20_blockquote_appends_credit() {
    let out = convert_rich_message(
        &[RichBlock::Blockquote {
            blocks: vec![RichBlock::Paragraph(plain("quoted"))],
            credit: Some(plain("src")),
        }],
        0,
    );
    assert!(out.text.contains("> quoted"));
    assert!(out.text.contains("— src"));
}

#[test]
fn slice39_21_pullquote_renders_with_credit() {
    let out = convert_rich_message(
        &[RichBlock::Pullquote {
            text: plain("wise"),
            credit: Some(plain("sage")),
        }],
        0,
    );
    assert!(out.text.contains("> wise"));
    assert!(out.text.contains("— sage"));
}

#[test]
fn slice39_22_collage_joins_blocks_and_caption() {
    let out = convert_rich_message(
        &[RichBlock::Collage {
            blocks: vec![RichBlock::Paragraph(plain("inner"))],
            caption: Some(plain("cap")),
        }],
        0,
    );
    assert_eq!(out.text, "inner\n\ncap");
}

#[test]
fn slice39_23_table_renders_header_and_alignment() {
    let cells = vec![
        vec![
            TableCell {
                text: plain("Name"),
                align: CellAlign::Left,
            },
            TableCell {
                text: plain("Value"),
                align: CellAlign::Right,
            },
        ],
        vec![
            TableCell {
                text: plain("A"),
                align: CellAlign::Left,
            },
            TableCell {
                text: plain("1"),
                align: CellAlign::Right,
            },
        ],
    ];
    assert_eq!(
        render_table(&cells, None),
        "| Name | Value |\n| :--- | ---: |\n| A | 1 |"
    );
}

#[test]
fn slice39_24_details_open_and_closed() {
    let open = convert_rich_message(
        &[RichBlock::Details {
            summary: plain("More"),
            blocks: vec![RichBlock::Paragraph(plain("body"))],
            is_open: true,
        }],
        0,
    );
    assert!(open
        .text
        .starts_with("<details open><summary>More</summary>"));
    let closed = convert_rich_message(
        &[RichBlock::Details {
            summary: plain("More"),
            blocks: vec![RichBlock::Paragraph(plain("body"))],
            is_open: false,
        }],
        0,
    );
    assert!(closed.text.starts_with("<details><summary>More</summary>"));
}

// --- photos / skipped (6) ---

#[test]
fn slice39_25_largest_photo_is_last_size() {
    assert_eq!(
        largest_photo_file_id(&photo_sizes()),
        Some("p-large".to_string())
    );
}

#[test]
fn slice39_26_rich_photo_filename_pattern() {
    assert_eq!(rich_photo_filename(42, 1), "rich-photo-42-1.jpg");
}

#[test]
fn slice39_27_photo_conversion_keeps_source_and_size() {
    let out = convert_rich_message(
        &[RichBlock::Photo {
            sizes: photo_sizes(),
            caption: None,
        }],
        42,
    );
    assert_eq!(
        out.photos,
        vec![ConvertedPhoto {
            file_id: "p-large".to_string(),
            filename: "rich-photo-42-1.jpg".to_string(),
            source: RICH_PHOTO_SOURCE.to_string(),
            file_size: Some(2048),
        }]
    );
}

#[test]
fn slice39_28_photo_caption_renders_when_no_other_text() {
    let out = convert_rich_message(
        &[RichBlock::Photo {
            sizes: photo_sizes(),
            caption: Some(plain("look")),
        }],
        1,
    );
    assert_eq!(out.text, "look");
}

#[test]
fn slice39_29_skipped_media_keeps_caption() {
    let out = convert_rich_message(
        &[RichBlock::SkippedMedia {
            caption: Some(plain("Video caption")),
        }],
        0,
    );
    assert_eq!(out.text, "Video caption");
    assert_eq!(out.skipped_media_count, 1);
}

#[test]
fn slice39_30_nested_collage_counts_each_skipped_block() {
    let out = convert_rich_message(
        &[RichBlock::Collage {
            blocks: vec![
                RichBlock::Photo {
                    sizes: photo_sizes(),
                    caption: None,
                },
                RichBlock::SkippedMedia { caption: None },
                RichBlock::SkippedMedia { caption: None },
            ],
            caption: Some(plain("Collage caption")),
        }],
        42,
    );
    assert_eq!(out.photos.len(), 1);
    assert_eq!(out.skipped_media_count, 2);
    assert!(out.text.contains("Collage caption"));
}

// --- inline rich text (8) ---

#[test]
fn slice39_31_bold_and_italic_render() {
    assert_eq!(
        render_rich_text(&RichText::Bold(Box::new(plain("Bold")))),
        "**Bold**"
    );
    assert_eq!(
        render_rich_text(&RichText::Italic(Box::new(plain("italic")))),
        "_italic_"
    );
}

#[test]
fn slice39_32_url_escapes_parens() {
    assert_eq!(
        render_rich_text(&RichText::Url {
            text: Box::new(plain("link")),
            url: "https://example.com/a_(b)".to_string()
        }),
        "[link](https://example.com/a_\\(b\\))"
    );
}

#[test]
fn slice39_33_inline_code_single_and_double_fence() {
    assert_eq!(render_inline_code("code"), "`code`");
    assert_eq!(render_inline_code("a`b"), "``a`b``");
}

#[test]
fn slice39_34_custom_emoji_uses_alt_text() {
    assert_eq!(
        render_rich_text(&RichText::CustomEmoji {
            alternative_text: "🔥".to_string()
        }),
        "🔥"
    );
}

#[test]
fn slice39_35_inline_math_wraps_dollars() {
    assert_eq!(
        render_rich_text(&RichText::Math {
            expression: "x^2".to_string()
        }),
        "$x^2$"
    );
}

#[test]
fn slice39_36_email_and_phone_schemes() {
    assert_eq!(
        render_rich_text(&RichText::Email {
            text: Box::new(plain("me")),
            email: "a@b.c".to_string()
        }),
        "[me](mailto:a@b.c)"
    );
    assert_eq!(
        render_rich_text(&RichText::Phone {
            text: Box::new(plain("call")),
            phone: "+1".to_string()
        }),
        "[call](tel:+1)"
    );
}

#[test]
fn slice39_37_reference_and_reference_link() {
    assert_eq!(
        render_rich_text(&RichText::Reference {
            name: "1".to_string(),
            text: Box::new(plain("note")),
        }),
        "[^1]: note"
    );
    assert_eq!(
        render_rich_text(&RichText::ReferenceLink {
            text: Box::new(plain("see")),
            reference_name: "1".to_string(),
        }),
        "[see][^1]"
    );
}

#[test]
fn slice39_38_passthrough_renders_inner() {
    assert_eq!(
        render_rich_text(&RichText::Passthrough(Box::new(plain("/status")))),
        "/status"
    );
}

// --- plain / caption / quote / indent (4) ---

#[test]
fn slice39_39_plain_strips_bold_but_keeps_text() {
    assert_eq!(
        plain_rich_text(&RichText::Bold(Box::new(plain("Bold")))),
        "Bold"
    );
}

#[test]
fn slice39_40_plain_keeps_code_verbatim() {
    assert_eq!(
        plain_rich_text(&RichText::Code("a ` b".to_string())),
        "a ` b"
    );
}

#[test]
fn slice39_41_quote_prefixes_each_line() {
    assert_eq!(quote_markdown("a\nb"), "> a\n> b");
}

#[test]
fn slice39_42_indent_rest_after_marker() {
    assert_eq!(prefix_first_and_indent_rest("- ", "a\nb"), "- a\n  b");
}

// --- escapes (3) ---

#[test]
fn slice39_43_escape_markdown_specials() {
    assert_eq!(escape_markdown_text("a*b_c"), "a\\*b\\_c");
}

#[test]
fn slice39_44_escape_table_cell_pipe_and_newline() {
    assert_eq!(escape_table_cell("a|b\nc"), "a\\|b<br>c");
}

#[test]
fn slice39_45_escape_link_reference_html() {
    assert_eq!(escape_link_target("a(b)"), "a\\(b\\)");
    assert_eq!(escape_reference_name("a^b]"), "ab");
    assert_eq!(escape_html_attribute("a\"<>&"), "a&quot;&lt;&gt;&amp;");
}

// --- slash-command + skip-notice gates (3) ---

#[test]
fn slice39_46_slash_token_extracted_from_leading_word() {
    assert_eq!(
        extract_slash_command_token("/status extra"),
        Some("/status".to_string())
    );
    assert!(should_expose_bot_command("/status extra"));
    assert_eq!(
        slash_command_entity_length("/status extra"),
        Some("/status".len())
    );
}

#[test]
fn slice39_47_bare_slash_and_plain_text_expose_nothing() {
    assert_eq!(extract_slash_command_token("/"), None);
    assert_eq!(extract_slash_command_token("hello"), None);
    assert!(!should_expose_bot_command("hello"));
    assert!(!should_expose_bot_command("/"));
    assert_eq!(slash_command_entity_length("hello"), None);
}

#[test]
fn slice39_48_skip_notice_only_when_media_skipped() {
    assert!(should_send_skip_notice(2));
    assert!(!should_send_skip_notice(0));
}
