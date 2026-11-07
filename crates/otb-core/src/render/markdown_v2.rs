//! Markdown-to-Telegram-MarkdownV2 conversion.
//!
//! Rust port of `src/bot/render/markdown-to-telegram-v2.ts`, including the
//! pieces it builds on from `block-parser.ts` (`TelegramBlock` / `InlineNode`
//! model), `markdown-normalizer.ts`, and `types.ts`.
//!
//! TS -> Rust name mapping:
//! - `convertToTelegramMarkdownV2` -> [`convert_to_telegram_markdown_v2`]
//! - `escapeMarkdownV2` -> [`escape_markdown_v2`]
//! - `escapeMarkdownV2Code` -> [`escape_markdown_v2_code`]
//! - `escapeMarkdownV2LinkUrl` -> [`escape_markdown_v2_link_url`]
//!
//! Fidelity notes (all outside the tested contract):
//! - Inline HTML (`<b>`, `<u>`, ...) is kept verbatim and escaped instead of
//!   being mapped to formatting nodes.
//! - Nested lists are flattened to one level. Ordered lists always start at 1,
//!   matching the TS renderer (which ignores the mdast `start`).
//! - Table cells are flattened to plain text, as in TS
//!   (`extractTableCellPlainText`).
//! - The parser is total: malformed input degrades to escaped literal text.
//!   All slicing happens at ASCII delimiter boundaries, so malformed or
//!   non-ASCII input can never panic.

/// Escapes characters reserved in Telegram MarkdownV2.
/// (TS: `escapeMarkdownV2`.)
pub fn escape_markdown_v2(text: &str) -> String {
    const SPECIAL: [char; 19] = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}',
        '.', '!', '\\',
    ];
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if SPECIAL.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Escapes code-span / code-block content for MarkdownV2.
/// (TS: `escapeMarkdownV2Code`.)
pub fn escape_markdown_v2_code(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c == '`' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Escapes a link URL for MarkdownV2.
/// (TS: `escapeMarkdownV2LinkUrl`.)
pub fn escape_markdown_v2_link_url(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c == ')' || c == '\\' || c == ']' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Converts standard Markdown into Telegram MarkdownV2.
/// (TS: `convertToTelegramMarkdownV2`.)
pub fn convert_to_telegram_markdown_v2(markdown: &str) -> String {
    let normalized = normalize_markdown(markdown, false);
    let blocks = parse_telegram_blocks(&normalized);
    if blocks.is_empty() {
        return String::new();
    }
    blocks
        .iter()
        .map(render_block)
        .collect::<Vec<_>>()
        .join("\n\n")
}

// --- Normalizer (port of `markdown-normalizer.ts`) ---

fn is_code_fence_line(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

fn is_horizontal_rule_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let marker = trimmed.as_bytes()[0];
    if marker != b'-' && marker != b'*' && marker != b'_' {
        return false;
    }
    let mut count = 0;
    for b in trimmed.as_bytes() {
        if *b == marker {
            count += 1;
        } else if *b != b' ' && *b != b'\t' {
            return false;
        }
    }
    count >= 3
}

fn is_indent_char(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\r'
}

/// Splits a `#` heading line into `(level, content)`.
/// (TS: `isHeadingLine` + `normalizeHeadingLine`.)
fn heading_parts(line: &str) -> Option<(usize, &str)> {
    let bytes = line.as_bytes();
    let mut k = 0;
    while k < bytes.len() && is_indent_char(bytes[k]) {
        k += 1;
    }
    if k > 3 {
        return None;
    }
    let mut hashes = 0;
    while k < bytes.len() && bytes[k] == b'#' {
        k += 1;
        hashes += 1;
    }
    if hashes == 0 || hashes > 6 {
        return None;
    }
    if k >= bytes.len() || (bytes[k] != b' ' && bytes[k] != b'\t') {
        return None;
    }
    while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
        k += 1;
    }
    let content = line[k..].trim();
    if content.is_empty() {
        return None;
    }
    Some((hashes, content))
}

/// Rewrites a `- [x] task` line to its `✅` / `🔲` form.
/// (TS: `normalizeChecklistLine`.)
fn normalize_checklist_line(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut k = 0;
    while k < bytes.len() && is_indent_char(bytes[k]) {
        k += 1;
    }
    let indent = &line[..k];
    let rest = &line[k..];
    let after_marker: &str;
    if let Some(stripped) = rest
        .strip_prefix('-')
        .or_else(|| rest.strip_prefix('+'))
        .or_else(|| rest.strip_prefix('*'))
    {
        after_marker = stripped;
    } else {
        let mut d = 0;
        while d < rest.len() && rest.as_bytes()[d].is_ascii_digit() {
            d += 1;
        }
        if d == 0 || !rest[d..].starts_with('.') {
            return None;
        }
        after_marker = &rest[d + 1..];
    }
    let mut s = 0;
    while s < after_marker.len() && is_indent_char(after_marker.as_bytes()[s]) {
        s += 1;
    }
    if s == 0 {
        return None;
    }
    let after_space = &after_marker[s..];
    let checked: u8;
    if after_space.starts_with("[ ]") {
        checked = b' ';
    } else if after_space.starts_with("[x]") || after_space.starts_with("[X]") {
        checked = b'x';
    } else {
        return None;
    }
    let after_box = &after_space[3..];
    let mut s2 = 0;
    while s2 < after_box.len() && is_indent_char(after_box.as_bytes()[s2]) {
        s2 += 1;
    }
    if s2 == 0 {
        return None;
    }
    let marker = if checked == b'x' { "✅" } else { "🔲" };
    Some(format!("{indent}{marker} {}", &after_box[s2..]))
}

fn normalize_markdown(text: &str, preserve_block_markup: bool) -> String {
    let mut output: Vec<String> = Vec::new();
    let mut in_code_fence = false;
    let mut in_quote = false;

    for line in text.split('\n') {
        if is_code_fence_line(line) {
            in_code_fence = !in_code_fence;
            in_quote = false;
            output.push(line.to_string());
            continue;
        }

        if in_code_fence {
            output.push(line.to_string());
            continue;
        }

        if line.trim().is_empty() {
            in_quote = false;
            output.push(line.to_string());
            continue;
        }

        if let Some((_, content)) = heading_parts(line) {
            output.push(if preserve_block_markup {
                line.to_string()
            } else {
                format!("**{content}**")
            });
            in_quote = false;
            continue;
        }

        if is_horizontal_rule_line(line) {
            output.push(if preserve_block_markup {
                line.to_string()
            } else {
                "──────────".to_string()
            });
            in_quote = false;
            continue;
        }

        let trimmed_left = line.trim_start();
        if trimmed_left.starts_with('>') {
            in_quote = true;
            let mut content = &trimmed_left[1..];
            if content.starts_with(' ') || content.starts_with('\t') {
                content = &content[1..];
            }
            let content_trimmed = content.trim_start();
            if !preserve_block_markup {
                if let Some(checklist) = normalize_checklist_line(content_trimmed) {
                    output.push(format!("> {}", checklist.trim_start()));
                    continue;
                }
            }
            output.push(format!("> {content_trimmed}"));
            continue;
        }

        if let Some(normalized_checklist) = normalize_checklist_line(line) {
            if preserve_block_markup {
                output.push(if in_quote {
                    format!("> {trimmed_left}")
                } else {
                    line.to_string()
                });
            } else {
                output.push(if in_quote {
                    format!("> {}", normalized_checklist.trim_start())
                } else {
                    normalized_checklist
                });
            }
            continue;
        }

        if in_quote {
            output.push(format!("> {trimmed_left}"));
            continue;
        }

        output.push(line.to_string());
    }

    output.join("\n")
}

// --- Block / inline model (port of `types.ts`) ---

/// `allow(dead_code)`: the HTML-derived nodes (`underline`, `spoiler`, ...)
/// and `Plain` blocks mirror the TS model and its renderer, but this parser
/// keeps inline HTML verbatim instead of producing them.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum InlineNode {
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

#[derive(Debug, Clone, PartialEq)]
struct TelegramListItem {
    blocks: Vec<TelegramBlock>,
    checked: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum TelegramBlock {
    Paragraph { inlines: Vec<InlineNode> },
    Heading { level: usize, inlines: Vec<InlineNode> },
    Blockquote { blocks: Vec<TelegramBlock> },
    List { ordered: bool, items: Vec<TelegramListItem> },
    Code { language: Option<String>, text: String },
    Table { rows: Vec<Vec<String>> },
    Rule,
    Plain { text: String },
}

// --- Inline parser (simplified port of `parseInlineNodes`) ---

fn push_text_node(nodes: &mut Vec<InlineNode>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(InlineNode::Text(existing)) = nodes.last_mut() {
        existing.push_str(text);
    } else {
        nodes.push(InlineNode::Text(text.to_string()));
    }
}

fn find_byte(bytes: &[u8], from: usize, target: u8) -> Option<usize> {
    let mut j = from;
    while j < bytes.len() {
        if bytes[j] == target {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn find_double_mark_close(bytes: &[u8], from: usize, mark: u8) -> Option<usize> {
    let mut j = from;
    while j + 1 < bytes.len() {
        if bytes[j] == mark && bytes[j + 1] == mark {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Finds a lone `mark`, skipping runs (`**` never closes a lone `*`).
fn find_single_mark_close(bytes: &[u8], from: usize, mark: u8) -> Option<usize> {
    let mut j = from;
    while j < bytes.len() {
        if bytes[j] == mark {
            let mut run = 1;
            while j + run < bytes.len() && bytes[j + run] == mark {
                run += 1;
            }
            if run == 1 {
                return Some(j);
            }
            j += run;
        } else {
            j += 1;
        }
    }
    None
}

fn find_marked_close(bytes: &[u8], from: usize) -> Option<usize> {
    let mut j = from;
    while j + 1 < bytes.len() {
        if bytes[j] == b'=' && bytes[j + 1] == b'=' && j > 0 && !bytes[j - 1].is_ascii_whitespace()
        {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Tries to parse `[text](url)` at `open` (where `bytes[open] == b'['`),
/// balancing nested parens in the URL. Returns
/// `(text_start, text_end, url_start, url_end, next)`.
fn try_parse_link(bytes: &[u8], open: usize) -> Option<(usize, usize, usize, usize, usize)> {
    let close = find_byte(bytes, open + 1, b']')?;
    if bytes.get(close + 1) != Some(&b'(') {
        return None;
    }
    let mut depth: usize = 0;
    let mut j = close + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let url_start = close + 2;
                    if bytes[url_start..j].iter().any(|b| b.is_ascii_whitespace()) {
                        return None;
                    }
                    return Some((open + 1, close, url_start, j, j + 1));
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn parse_inline_nodes(text: &str) -> Vec<InlineNode> {
    let mut nodes: Vec<InlineNode> = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut lit_start = 0;

    // NOTE: `i` only ever slices at ASCII delimiter positions, so stepping
    // over multi-byte UTF-8 one byte at a time can never panic.
    while i < len {
        let c = bytes[i];

        if c == b'`' {
            if let Some(end) = find_byte(bytes, i + 1, b'`') {
                push_text_node(&mut nodes, &text[lit_start..i]);
                nodes.push(InlineNode::Code(text[i + 1..end].to_string()));
                i = end + 1;
                lit_start = i;
                continue;
            }
            i += 1;
            continue;
        }

        if c == b'[' {
            if let Some((ts, te, us, ue, next)) = try_parse_link(bytes, i) {
                push_text_node(&mut nodes, &text[lit_start..i]);
                nodes.push(InlineNode::Link {
                    text: parse_inline_nodes(&text[ts..te]),
                    url: text[us..ue].to_string(),
                });
                i = next;
                lit_start = i;
                continue;
            }
            i += 1;
            continue;
        }

        if c == b'*' && i + 1 < len && bytes[i + 1] == b'*' {
            if let Some(end) = find_double_mark_close(bytes, i + 2, b'*') {
                push_text_node(&mut nodes, &text[lit_start..i]);
                nodes.push(InlineNode::Bold(parse_inline_nodes(&text[i + 2..end])));
                i = end + 2;
                lit_start = i;
                continue;
            }
            i += 1;
            continue;
        }

        if c == b'~' && i + 1 < len && bytes[i + 1] == b'~' {
            if let Some(end) = find_double_mark_close(bytes, i + 2, b'~') {
                push_text_node(&mut nodes, &text[lit_start..i]);
                nodes.push(InlineNode::Strike(parse_inline_nodes(&text[i + 2..end])));
                i = end + 2;
                lit_start = i;
                continue;
            }
            i += 1;
            continue;
        }

        if c == b'*' {
            if let Some(end) = find_single_mark_close(bytes, i + 1, b'*') {
                push_text_node(&mut nodes, &text[lit_start..i]);
                nodes.push(InlineNode::Italic(parse_inline_nodes(&text[i + 1..end])));
                i = end + 1;
                lit_start = i;
                continue;
            }
            i += 1;
            continue;
        }

        // `_` / `__` never open inside a word (GFM rule), so `raw_html`
        // stays literal.
        if c == b'_' {
            let prev_word = i > 0 && is_word_byte(bytes[i - 1]);
            if i + 1 < len && bytes[i + 1] == b'_' {
                if !prev_word {
                    if let Some(end) = find_double_mark_close(bytes, i + 2, b'_') {
                        push_text_node(&mut nodes, &text[lit_start..i]);
                        nodes.push(InlineNode::Bold(parse_inline_nodes(&text[i + 2..end])));
                        i = end + 2;
                        lit_start = i;
                        continue;
                    }
                }
                i += 1;
                continue;
            }
            if !prev_word {
                if let Some(end) = find_single_mark_close(bytes, i + 1, b'_') {
                    push_text_node(&mut nodes, &text[lit_start..i]);
                    nodes.push(InlineNode::Italic(parse_inline_nodes(&text[i + 1..end])));
                    i = end + 1;
                    lit_start = i;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        // Telegram `==marked==` (remark leaves it in the text in TS).
        if c == b'=' && i + 1 < len && bytes[i + 1] == b'=' {
            let after = i + 2;
            if after < len && !bytes[after].is_ascii_whitespace() {
                if let Some(end) = find_marked_close(bytes, after) {
                    push_text_node(&mut nodes, &text[lit_start..i]);
                    nodes.push(InlineNode::Marked(parse_inline_nodes(&text[after..end])));
                    i = end + 2;
                    lit_start = i;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    push_text_node(&mut nodes, &text[lit_start..]);
    nodes
}

/// Plain-text projection of inline nodes (TS: `extractInlinePlainText`),
/// used for table cells.
fn inline_to_plain(nodes: &[InlineNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            InlineNode::Text(text) | InlineNode::Code(text) => out.push_str(text),
            InlineNode::Link { text, .. } => out.push_str(&inline_to_plain(text)),
            InlineNode::Bold(children)
            | InlineNode::Italic(children)
            | InlineNode::Strike(children)
            | InlineNode::Underline(children)
            | InlineNode::Spoiler(children)
            | InlineNode::Subscript(children)
            | InlineNode::Superscript(children)
            | InlineNode::Marked(children) => out.push_str(&inline_to_plain(children)),
        }
    }
    out
}

// --- Block parser (simplified port of `parseTelegramBlocks`) ---

struct ListItemLine {
    ordered: bool,
    checked: Option<bool>,
    body: String,
}

fn strip_task_marker(body: &str) -> (Option<bool>, &str) {
    for (marker, checked) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
        if let Some(rest) = body.strip_prefix(marker) {
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
                return (Some(checked), rest.trim_start());
            }
        }
    }
    (None, body)
}

fn match_list_item(line: &str) -> Option<ListItemLine> {
    let content = line.trim_start();
    for marker in ['-', '+', '*'] {
        if let Some(rest) = content.strip_prefix(marker) {
            if rest.is_empty() {
                return Some(ListItemLine {
                    ordered: false,
                    checked: None,
                    body: String::new(),
                });
            }
            if rest.starts_with(' ') || rest.starts_with('\t') {
                let body = rest.trim_start();
                let (checked, body) = strip_task_marker(body);
                return Some(ListItemLine {
                    ordered: false,
                    checked,
                    body: body.to_string(),
                });
            }
            return None;
        }
    }
    let bytes = content.as_bytes();
    let mut d = 0;
    while d < bytes.len() && bytes[d].is_ascii_digit() {
        d += 1;
    }
    if d > 0 && d < bytes.len() && (bytes[d] == b'.' || bytes[d] == b')') {
        let rest = &content[d + 1..];
        if rest.is_empty() {
            return Some(ListItemLine {
                ordered: true,
                checked: None,
                body: String::new(),
            });
        }
        if rest.starts_with(' ') || rest.starts_with('\t') {
            let body = rest.trim_start();
            let (checked, body) = strip_task_marker(body);
            return Some(ListItemLine {
                ordered: true,
                checked,
                body: body.to_string(),
            });
        }
    }
    None
}

fn is_table_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t[1..].contains('|')
}

fn parse_table_row_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let inner = t.strip_prefix('|').unwrap_or(t);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner
        .split('|')
        .map(|cell| inline_to_plain(&parse_inline_nodes(cell.trim())))
        .collect()
}

fn is_delimiter_row(line: &str) -> bool {
    let cells = parse_table_row_cells(line);
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|cell| {
        let inner = cell.strip_prefix(':').unwrap_or(cell);
        let inner = inner.strip_suffix(':').unwrap_or(inner);
        !inner.is_empty() && inner.bytes().all(|b| b == b'-')
    })
}

fn parse_chunk(lines: &[&str]) -> Vec<TelegramBlock> {
    if lines.is_empty() {
        return Vec::new();
    }

    if lines.len() >= 2
        && lines.iter().all(|l| is_table_row(l))
        && is_delimiter_row(lines[1])
    {
        let mut rows = vec![parse_table_row_cells(lines[0])];
        for line in &lines[2..] {
            rows.push(parse_table_row_cells(line));
        }
        return vec![TelegramBlock::Table { rows }];
    }

    if lines.iter().all(|l| match_list_item(l).is_some()) {
        let mut items = Vec::with_capacity(lines.len());
        let mut ordered = false;
        for (index, line) in lines.iter().enumerate() {
            if let Some(item) = match_list_item(line) {
                if index == 0 {
                    ordered = item.ordered;
                }
                items.push(TelegramListItem {
                    blocks: vec![TelegramBlock::Paragraph {
                        inlines: parse_inline_nodes(&item.body),
                    }],
                    checked: item.checked,
                });
            }
        }
        return vec![TelegramBlock::List { ordered, items }];
    }

    if lines
        .iter()
        .all(|l| l.trim_start().starts_with('>'))
    {
        let inner: Vec<String> = lines
            .iter()
            .map(|l| {
                let t = l.trim_start();
                let mut content = &t[1..];
                if content.starts_with(' ') || content.starts_with('\t') {
                    content = &content[1..];
                }
                content.to_string()
            })
            .collect();
        let blocks = parse_block_list(&inner.join("\n"));
        if blocks.is_empty() {
            return Vec::new();
        }
        return vec![TelegramBlock::Blockquote { blocks }];
    }

    if lines.len() == 1 {
        let line = lines[0];
        if let Some((level, content)) = heading_parts(line) {
            return vec![TelegramBlock::Heading {
                level,
                inlines: parse_inline_nodes(content),
            }];
        }
        if is_horizontal_rule_line(line) {
            return vec![TelegramBlock::Rule];
        }
    }

    vec![TelegramBlock::Paragraph {
        inlines: parse_inline_nodes(&lines.join("\n")),
    }]
}

fn parse_block_list(text: &str) -> Vec<TelegramBlock> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut blocks: Vec<TelegramBlock> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim().is_empty() {
            i += 1;
            continue;
        }
        if is_code_fence_line(lines[i]) {
            let lang = lines[i].trim_start()[3..].trim();
            let lang = lang.split_whitespace().next().unwrap_or("");
            let language = if lang.is_empty() {
                None
            } else {
                Some(lang.to_string())
            };
            let mut body: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() && !is_code_fence_line(lines[i]) {
                body.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            blocks.push(TelegramBlock::Code {
                language,
                text: body.join("\n"),
            });
            continue;
        }
        let mut j = i;
        while j < lines.len() && !lines[j].trim().is_empty() {
            j += 1;
        }
        blocks.extend(parse_chunk(&lines[i..j]));
        i = j;
    }
    blocks
}

fn parse_telegram_blocks(markdown: &str) -> Vec<TelegramBlock> {
    let normalized = normalize_markdown(markdown, true);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    parse_block_list(trimmed)
}

// --- Renderer (port of `renderInlineNodes` / `renderBlock`) ---

fn render_inline_nodes(nodes: &[InlineNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            InlineNode::Text(text) => out.push_str(&escape_markdown_v2(text)),
            InlineNode::Bold(children) => {
                out.push('*');
                out.push_str(&render_inline_nodes(children));
                out.push('*');
            }
            InlineNode::Italic(children) => {
                out.push('_');
                out.push_str(&render_inline_nodes(children));
                out.push('_');
            }
            InlineNode::Strike(children) => {
                out.push('~');
                out.push_str(&render_inline_nodes(children));
                out.push('~');
            }
            InlineNode::Code(text) => {
                out.push('`');
                out.push_str(&escape_markdown_v2_code(text));
                out.push('`');
            }
            InlineNode::Link { text, url } => {
                out.push('[');
                out.push_str(&render_inline_nodes(text));
                out.push_str("](");
                out.push_str(&escape_markdown_v2_link_url(url));
                out.push(')');
            }
            InlineNode::Underline(children) => {
                out.push_str("__");
                out.push_str(&render_inline_nodes(children));
                out.push_str("__");
            }
            InlineNode::Spoiler(children) => {
                out.push_str("||");
                out.push_str(&render_inline_nodes(children));
                out.push_str("||");
            }
            // MarkdownV2 has no counterpart for these, so only the content survives.
            InlineNode::Subscript(children)
            | InlineNode::Superscript(children)
            | InlineNode::Marked(children) => {
                out.push_str(&render_inline_nodes(children));
            }
        }
    }
    out
}

fn render_block(block: &TelegramBlock) -> String {
    match block {
        TelegramBlock::Paragraph { inlines } => render_inline_nodes(inlines),
        TelegramBlock::Heading { inlines, .. } => {
            format!("*{}*", render_inline_nodes(inlines))
        }
        TelegramBlock::Blockquote { blocks } => blocks
            .iter()
            .map(render_block)
            .filter(|s| !s.is_empty())
            .map(|rendered| {
                rendered
                    .split('\n')
                    .map(|line| format!("> {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        TelegramBlock::List { ordered, items } => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let marker = if *ordered {
                    format!("{}\\. ", index + 1)
                } else {
                    "\\- ".to_string()
                };
                let task_prefix = match item.checked {
                    Some(true) => "✅ ",
                    Some(false) => "🔲 ",
                    None => "",
                };
                let body = item
                    .blocks
                    .iter()
                    .map(render_block)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                let continuation = " ".repeat(marker.len());
                format!("{task_prefix}{body}")
                    .split('\n')
                    .enumerate()
                    .map(|(line_index, line)| {
                        if line_index == 0 {
                            format!("{marker}{line}")
                        } else {
                            format!("{continuation}{line}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        TelegramBlock::Code { language, text } => format!(
            "```{}\n{}\n```",
            escape_markdown_v2_code(language.as_deref().unwrap_or("")),
            escape_markdown_v2_code(text)
        ),
        TelegramBlock::Table { rows } => {
            let Some(header_row) = rows.first() else {
                return String::new();
            };
            let mut out = vec![
                format!(
                    "\\| {} \\|",
                    header_row
                        .iter()
                        .map(|cell| escape_markdown_v2(cell))
                        .collect::<Vec<_>>()
                        .join(" \\| ")
                ),
                format!(
                    "\\| {} \\|",
                    header_row
                        .iter()
                        .map(|_| "\\-\\-\\-")
                        .collect::<Vec<_>>()
                        .join(" \\| ")
                ),
            ];
            for row in rows.iter().skip(1) {
                out.push(format!(
                    "\\| {} \\|",
                    row.iter()
                        .map(|cell| escape_markdown_v2(cell))
                        .collect::<Vec<_>>()
                        .join(" \\| ")
                ));
            }
            out.join("\n")
        }
        TelegramBlock::Rule => "---".to_string(),
        TelegramBlock::Plain { text } => escape_markdown_v2(text),
    }
}
