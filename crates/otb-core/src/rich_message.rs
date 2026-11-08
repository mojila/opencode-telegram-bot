//! Rich-message normalize pure logic (slice 39).
//!
//! Rust port of the pure conversion behind
//! `src/bot/handlers/rich-message-handler.ts`:
//! - `convertRichMessage` (blocks + messageId -> text/photos/skipped)
//! - block rendering (paragraph/footer/thinking, heading, pre fence,
//!   divider, math, anchor, list markers, blockquote/pullquote quoting,
//!   collage/slideshow join, table markdown, details, photo largest-pick,
//!   skipped-media counting)
//! - inline rendering (`renderRichText`) and `plainRichText`
//! - caption/table/quote/indent/code helpers and markdown/link/reference/
//!   HTML escapes
//! - slash-command entity gate (`exposeSlashCommandEntity` token rule)
//! - skip-notice gate (`skippedMediaCount > 0`)

/// Photo source tag for rich-message photos (TS: `source: "rich"`).
pub const RICH_PHOTO_SOURCE: &str = "rich";

/// Telegram photo size entry (TS: `block.photo[]` element).
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoSize {
    pub file_id: String,
    pub file_size: Option<u64>,
}

/// Table cell horizontal alignment (TS: `cell.align`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAlign {
    Left,
    Center,
    Right,
}

/// Inline rich text (TS: `RichText`).
#[derive(Debug, Clone, PartialEq)]
pub enum RichText {
    Plain(String),
    Bold(Box<RichText>),
    Italic(Box<RichText>),
    Underline(Box<RichText>),
    Strikethrough(Box<RichText>),
    Spoiler(Box<RichText>),
    Subscript(Box<RichText>),
    Superscript(Box<RichText>),
    Marked(Box<RichText>),
    Code(String),
    CustomEmoji { alternative_text: String },
    Math { expression: String },
    Url { text: Box<RichText>, url: String },
    Email { text: Box<RichText>, email: String },
    Phone { text: Box<RichText>, phone: String },
    Anchor { name: String },
    AnchorLink { text: Box<RichText>, anchor_name: String },
    Reference { name: String, text: Box<RichText> },
    ReferenceLink { text: Box<RichText>, reference_name: String },
    /// `date_time` / `text_mention` / `bank_card_number` / `mention` /
    /// `hashtag` / `cashtag` / `bot_command`: renders inner text as-is.
    Passthrough(Box<RichText>),
    Concat(Vec<RichText>),
}

/// One list item (TS: `block.items[]`).
#[derive(Debug, Clone, PartialEq)]
pub struct RichListItem {
    pub has_checkbox: bool,
    pub is_checked: bool,
    pub label: String,
    pub blocks: Vec<RichBlock>,
}

/// A rich-message block (TS: `RichBlock`).
#[derive(Debug, Clone, PartialEq)]
pub enum RichBlock {
    Paragraph(RichText),
    Footer(RichText),
    Thinking(RichText),
    Heading { size: u8, text: RichText },
    Pre { language: Option<String>, text: RichText },
    Divider,
    Math { expression: String },
    Anchor { name: String },
    List { items: Vec<RichListItem> },
    Blockquote { blocks: Vec<RichBlock>, credit: Option<RichText> },
    Pullquote { text: RichText, credit: Option<RichText> },
    Collage { blocks: Vec<RichBlock>, caption: Option<RichText> },
    Slideshow { blocks: Vec<RichBlock>, caption: Option<RichText> },
    Table { cells: Vec<Vec<TableCell>>, caption: Option<RichText> },
    Details { summary: RichText, blocks: Vec<RichBlock>, is_open: bool },
    Photo { sizes: Vec<PhotoSize>, caption: Option<RichText> },
    /// `animation` / `audio` / `video` / `voice_note` / `map`: counts one
    /// skipped-media part, renders caption only.
    SkippedMedia { caption: Option<RichText> },
}

/// One table cell (TS: `RichBlockTableCell` text + align).
#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub text: RichText,
    pub align: CellAlign,
}

/// Converted photo offered to OpenCode (TS: `TelegramPhotoInput` from photo).
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertedPhoto {
    pub file_id: String,
    pub filename: String,
    pub source: String,
    pub file_size: Option<u64>,
}

/// `convertRichMessage` result (TS: `RichMessageConversion`).
#[derive(Debug, Clone, PartialEq)]
pub struct RichMessageConversion {
    pub text: String,
    pub photos: Vec<ConvertedPhoto>,
    pub skipped_media_count: usize,
}

struct ConversionState {
    photos: Vec<ConvertedPhoto>,
    skipped_media_count: usize,
    message_id: i64,
}

/// Convert blocks to prompt text + photos + skipped count
/// (TS: `convertRichMessage`: `renderBlocks(...).trim()`).
pub fn convert_rich_message(blocks: &[RichBlock], message_id: i64) -> RichMessageConversion {
    let mut state = ConversionState {
        photos: Vec::new(),
        skipped_media_count: 0,
        message_id,
    };
    let text = render_blocks(blocks, &mut state).trim().to_string();
    RichMessageConversion {
        text,
        photos: state.photos,
        skipped_media_count: state.skipped_media_count,
    }
}

fn render_blocks(blocks: &[RichBlock], state: &mut ConversionState) -> String {
    blocks
        .iter()
        .map(|block| render_block(block, state))
        .filter(|rendered| !rendered.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[allow(clippy::too_many_lines)]
fn render_block(block: &RichBlock, state: &mut ConversionState) -> String {
    match block {
        RichBlock::Paragraph(text)
        | RichBlock::Footer(text)
        | RichBlock::Thinking(text) => render_rich_text(text),
        RichBlock::Heading { size, text } => {
            format!("{} {}", "#".repeat(*size as usize), render_rich_text(text))
        }
        RichBlock::Pre { language, text } => {
            let content = plain_rich_text(text);
            let fence = if content.contains("```") { "````" } else { "```" };
            format!("{}{}\n{}\n{}", fence, language.as_deref().unwrap_or(""), content, fence)
        }
        RichBlock::Divider => "---".to_string(),
        RichBlock::Math { expression } => format!("$$\n{}\n$$", expression),
        RichBlock::Anchor { name } => {
            format!("<a name=\"{}\"></a>", escape_html_attribute(name))
        }
        RichBlock::List { items } => items
            .iter()
            .map(|item| {
                let marker = list_item_marker(item.has_checkbox, item.is_checked, &item.label);
                prefix_first_and_indent_rest(&marker, &render_blocks(&item.blocks, state))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        RichBlock::Blockquote { blocks, credit } => {
            let mut parts = vec![render_blocks(blocks, state)];
            if let Some(credit) = credit {
                parts.push(format!("— {}", render_rich_text(credit)));
            }
            quote_markdown(&parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join("\n\n"))
        }
        RichBlock::Pullquote { text, credit } => {
            let mut parts = vec![render_rich_text(text)];
            if let Some(credit) = credit {
                parts.push(format!("— {}", render_rich_text(credit)));
            }
            quote_markdown(&parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join("\n\n"))
        }
        RichBlock::Collage { blocks, caption } | RichBlock::Slideshow { blocks, caption } => {
            let mut parts = vec![render_blocks(blocks, state)];
            if let Some(caption) = caption {
                parts.push(render_caption(caption, None));
            }
            parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join("\n\n")
        }
        RichBlock::Table { cells, caption } => render_table(cells, caption.as_ref()),
        RichBlock::Details { summary, blocks, is_open } => {
            let open = if *is_open { " open" } else { "" };
            format!(
                "<details{}><summary>{}</summary>\n\n{}\n\n</details>",
                open,
                render_rich_text(summary),
                render_blocks(blocks, state)
            )
        }
        RichBlock::Photo { sizes, caption } => {
            if let Some(largest) = sizes.last() {
                state.photos.push(ConvertedPhoto {
                    file_id: largest.file_id.clone(),
                    filename: rich_photo_filename(state.message_id, state.photos.len() + 1),
                    source: RICH_PHOTO_SOURCE.to_string(),
                    file_size: largest.file_size,
                });
            }
            caption.as_ref().map(|c| render_caption(c, None)).unwrap_or_default()
        }
        RichBlock::SkippedMedia { caption } => {
            state.skipped_media_count += 1;
            caption.as_ref().map(|c| render_caption(c, None)).unwrap_or_default()
        }
    }
}

/// Render inline rich text to Markdown (TS: `renderRichText`).
pub fn render_rich_text(text: &RichText) -> String {
    match text {
        RichText::Plain(s) => escape_markdown_text(s),
        RichText::Concat(parts) => parts.iter().map(render_rich_text).collect(),
        RichText::Bold(inner) => format!("**{}**", render_rich_text(inner)),
        RichText::Italic(inner) => format!("_{}_", render_rich_text(inner)),
        RichText::Underline(inner) => format!("<u>{}</u>", render_rich_text(inner)),
        RichText::Strikethrough(inner) => format!("~~{}~~", render_rich_text(inner)),
        RichText::Spoiler(inner) => format!("||{}||", render_rich_text(inner)),
        RichText::Subscript(inner) => format!("<sub>{}</sub>", render_rich_text(inner)),
        RichText::Superscript(inner) => format!("<sup>{}</sup>", render_rich_text(inner)),
        RichText::Marked(inner) => format!("=={}==", render_rich_text(inner)),
        RichText::Code(content) => render_inline_code(content),
        RichText::CustomEmoji { alternative_text } => escape_markdown_text(alternative_text),
        RichText::Math { expression } => format!("${}$", expression),
        RichText::Url { text, url } => {
            format!("[{}]({})", render_rich_text(text), escape_link_target(url))
        }
        RichText::Email { text, email } => {
            format!("[{}](mailto:{})", render_rich_text(text), escape_link_target(email))
        }
        RichText::Phone { text, phone } => {
            format!("[{}](tel:{})", render_rich_text(text), escape_link_target(phone))
        }
        RichText::Anchor { name } => {
            format!("<a name=\"{}\"></a>", escape_html_attribute(name))
        }
        RichText::AnchorLink { text, anchor_name } => {
            format!("[{}](#{})", render_rich_text(text), escape_link_target(anchor_name))
        }
        RichText::Reference { name, text } => {
            format!("[^{}]: {}", escape_reference_name(name), render_rich_text(text))
        }
        RichText::ReferenceLink { text, reference_name } => {
            format!("[{}][^{}]", render_rich_text(text), escape_reference_name(reference_name))
        }
        RichText::Passthrough(inner) => render_rich_text(inner),
    }
}

/// Strip formatting to plain text (TS: `plainRichText`; used for `pre`
/// content and inline code).
pub fn plain_rich_text(text: &RichText) -> String {
    match text {
        RichText::Plain(s) => s.clone(),
        RichText::Concat(parts) => parts.iter().map(plain_rich_text).collect(),
        RichText::Code(content) => content.clone(),
        RichText::CustomEmoji { alternative_text } => alternative_text.clone(),
        RichText::Math { expression } => expression.clone(),
        RichText::Anchor { .. } => String::new(),
        RichText::Bold(inner)
        | RichText::Italic(inner)
        | RichText::Underline(inner)
        | RichText::Strikethrough(inner)
        | RichText::Spoiler(inner)
        | RichText::Subscript(inner)
        | RichText::Superscript(inner)
        | RichText::Marked(inner)
        | RichText::Url { text: inner, .. }
        | RichText::Email { text: inner, .. }
        | RichText::Phone { text: inner, .. }
        | RichText::AnchorLink { text: inner, .. }
        | RichText::Reference { text: inner, .. }
        | RichText::ReferenceLink { text: inner, .. }
        | RichText::Passthrough(inner) => plain_rich_text(inner),
    }
}

/// Render a caption with optional credit (TS: `renderCaption`).
pub fn render_caption(text: &RichText, credit: Option<&RichText>) -> String {
    let mut parts = vec![render_rich_text(text)];
    if let Some(credit) = credit {
        parts.push(format!("— {}", render_rich_text(credit)));
    }
    parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join("\n")
}

/// Render cells (+ optional caption) as a Markdown table
/// (TS: `renderTable`).
pub fn render_table(cells: &[Vec<TableCell>], caption: Option<&RichText>) -> String {
    if cells.is_empty() {
        return caption.map_or_else(String::new, render_rich_text);
    }
    let column_count = cells.iter().map(Vec::len).max().unwrap_or(0);
    let rows: Vec<Vec<String>> = cells
        .iter()
        .map(|row| {
            (0..column_count)
                .map(|index| {
                    row.get(index).map_or_else(String::new, |cell| {
                        escape_table_cell(&render_rich_text(&cell.text))
                    })
                })
                .collect()
        })
        .collect();
    let first_row = &cells[0];
    let alignment: Vec<&str> = (0..column_count)
        .map(|index| match first_row.get(index).map(|cell| cell.align) {
            Some(CellAlign::Center) => ":---:",
            Some(CellAlign::Right) => "---:",
            _ => ":---",
        })
        .collect();
    let header = rows.first().map_or_else(String::new, |row| row.join(" | "));
    let mut table = format!("| {} |\n| {} |", header, alignment.join(" | "));
    for row in rows.iter().skip(1) {
        table.push_str(&format!("\n| {} |", row.join(" | ")));
    }
    if let Some(caption) = caption {
        let rendered = render_rich_text(caption);
        if rendered.is_empty() {
            table
        } else {
            format!("{}\n\n{}", rendered, table)
        }
    } else {
        table
    }
}

/// Pick the largest photo size: Telegram orders ascending, last is largest
/// (TS: `block.photo[block.photo.length - 1]`).
pub fn largest_photo_file_id(sizes: &[PhotoSize]) -> Option<String> {
    sizes.last().map(|size| size.file_id.clone())
}

/// `rich-photo-<messageId>-<n>.jpg` album filename (TS: photo mapping).
pub fn rich_photo_filename(message_id: i64, index: usize) -> String {
    format!("rich-photo-{}-{}.jpg", message_id, index)
}

/// List-item marker: `- [x]` / `- [ ]` for checkboxes, else label or `-`
/// (TS: list `marker`).
pub fn list_item_marker(has_checkbox: bool, is_checked: bool, label: &str) -> String {
    if has_checkbox {
        if is_checked { "- [x]".to_string() } else { "- [ ]".to_string() }
    } else if label.is_empty() {
        "-".to_string()
    } else {
        label.to_string()
    }
}

/// Prefix the first line with the marker, indent the rest two spaces
/// (TS: `prefixFirstAndIndentRest`).
///
/// TS callers pass bare markers (`-`, `- [ ]`, labels) and the helper adds
/// the separating space; normalize trailing marker whitespace first so a
/// marker like `"- "` does not produce a double space (`"-  a"`).
pub fn prefix_first_and_indent_rest(marker: &str, text: &str) -> String {
    let marker = marker.trim_end();
    text.split('\n')
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                format!("{} {}", marker, line)
            } else {
                format!("  {}", line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Prefix every line with `> ` (TS: `quoteMarkdown`).
pub fn quote_markdown(text: &str) -> String {
    text.split('\n').map(|line| format!("> {}", line)).collect::<Vec<_>>().join("\n")
}

/// Wrap in backticks, doubling when the content holds one
/// (TS: `renderInlineCode`).
pub fn render_inline_code(text: &str) -> String {
    let fence = if text.contains('`') { "``" } else { "`" };
    format!("{fence}{text}{fence}")
}

/// Escape Markdown text chars `[\`*_[\]<>]` (TS: `escapeMarkdownText`).
pub fn escape_markdown_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Escape `|` and fold newlines to `<br>` (TS: `escapeTableCell`).
pub fn escape_table_cell(text: &str) -> String {
    text.replace('|', "\\|").replace("\r\n", "<br>").replace('\n', "<br>")
}

/// Escape `\`, `(`, `)` in link targets (TS: `escapeLinkTarget`).
pub fn escape_link_target(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '(' | ')') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Strip `]`, `^` from reference names (TS: `escapeReferenceName`).
pub fn escape_reference_name(text: &str) -> String {
    text.chars().filter(|ch| !matches!(ch, ']' | '^')).collect()
}

/// Escape `&`, `"`, `<`, `>` in HTML attributes (TS: `escapeHtmlAttribute`).
pub fn escape_html_attribute(text: &str) -> String {
    text.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

/// First whitespace-separated token when it is a `/command`
/// (`/x` or longer); otherwise `None`
/// (TS: `exposeSlashCommandEntity` token rule).
pub fn extract_slash_command_token(text: &str) -> Option<String> {
    let token = text.trim().split_whitespace().next().unwrap_or("");
    if token.starts_with('/') && token.len() >= 2 {
        Some(token.to_string())
    } else {
        None
    }
}

/// Whether the text exposes a `bot_command` entity at offset 0
/// (TS: `exposeSlashCommandEntity` gate).
pub fn should_expose_bot_command(text: &str) -> bool {
    extract_slash_command_token(text).is_some()
}

/// Length of the leading `bot_command` entity for slash text
/// (TS: `length: token.length`, offset 0).
pub fn slash_command_entity_length(text: &str) -> Option<usize> {
    extract_slash_command_token(text).map(|token| token.len())
}

/// Whether a skip notice reply is sent (TS: `skippedMediaCount > 0`).
pub fn should_send_skip_notice(skipped_media_count: usize) -> bool {
    skipped_media_count > 0
}
