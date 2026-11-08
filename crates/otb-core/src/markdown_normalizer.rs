//! Markdown-normalizer pure gates (slice 43).
//!
//! Rust port of the pure line transforms in:
//! - `src/bot/render/markdown-normalizer.ts`
//!   (`normalizeMarkdownForTelegramRendering`,
//!   `normalizeMarkdownForTelegramBlockParsing`)
//!
//! Pure sync string logic only: no Telegram SDK, no timers, no I/O.
//!
//! Parity notes / known deviations from the TS original:
//! - JS `\s` / `\S` / `trim()` / `trimStart()` are approximated with
//!   `char::is_whitespace` (`str::trim`, `str::trim_start`). Both agree on
//!   ASCII space / `\t` / `\r` / form-feed, which is what the contract
//!   exercises; exotic cases (e.g. U+FEFF trimming) may differ.
//! - JS `\d` is ASCII-only (`[0-9]`); the port uses `is_ascii_digit`, same.
//! - JS `(.*)$` excludes bare `\r` (`.` skips line terminators) while the
//!   port keeps a mid-line `\r` in the checklist tail. Lines from
//!   `\n`-splitting never contain `\n`, so this only matters for stray
//!   `\r` bytes, which the parity tests do not cover.

/// Replacement line for horizontal rules in rendering mode
/// (TS: `"──────────"` — ten U+2500 box-drawing strokes).
const RULE_LINE: &str = "──────────";

fn is_code_fence_line(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

fn is_horizontal_rule_line(line: &str) -> bool {
    // TS: /^([-*_])(?:\s*\1){2,}$/ tested against `line.trim()`.
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let mut chars = trimmed.chars();
    let mark = chars.next().unwrap_or('\0');
    if mark != '-' && mark != '*' && mark != '_' {
        return false;
    }
    let mut count = 1usize;
    for c in chars {
        if c.is_whitespace() {
            continue;
        }
        if c == mark {
            count += 1;
        } else {
            return false;
        }
    }
    count >= 3
}

fn is_heading_line(line: &str) -> bool {
    // TS: /^\s{0,3}#{1,6}\s+\S/
    let mut chars = line.chars().peekable();
    let mut leading = 0usize;
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
        leading += 1;
    }
    if leading > 3 {
        return false;
    }
    let mut hashes = 0usize;
    while chars.peek() == Some(&'#') {
        chars.next();
        hashes += 1;
    }
    if hashes < 1 || hashes > 6 {
        return false;
    }
    if !matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        return false;
    }
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
    matches!(chars.peek(), Some(c) if !c.is_whitespace())
}

fn normalize_heading_line(line: &str) -> String {
    // TS: line.match(/^\s{0,3}#{1,6}\s+(.+?)\s*$/) -> `**$1**`, else `line`.
    if !is_heading_line(line) {
        return line.to_string();
    }
    let mut rest = line;
    // Strip up to 3 leading whitespace chars.
    for _ in 0..3 {
        match rest.chars().next() {
            Some(c) if c.is_whitespace() => {
                rest = &rest[c.len_utf8()..];
            }
            _ => break,
        }
    }
    // Strip 1-6 hashes.
    let mut stripped = 0usize;
    while rest.starts_with('#') && stripped < 6 {
        rest = &rest[1..];
        stripped += 1;
    }
    // Strip the whitespace run after the hashes.
    let mut cut = 0usize;
    let mut seen = false;
    for (i, c) in rest.char_indices() {
        if c.is_whitespace() {
            cut = i + c.len_utf8();
            seen = true;
        } else {
            break;
        }
    }
    if !seen {
        return line.to_string();
    }
    let heading = rest[cut..].trim_end();
    if heading.is_empty() {
        return line.to_string();
    }
    format!("**{heading}**")
}

fn normalize_checklist_line(line: &str) -> Option<String> {
    // TS: /^(\s*)(?:[-+*]|\d+\.)\s+\[( |x|X)\]\s+(.*)$/
    let mut indent_end = 0usize;
    for (i, c) in line.char_indices() {
        if c.is_whitespace() {
            indent_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    let (indent, mut rest) = line.split_at(indent_end);

    match rest.chars().next()? {
        '-' | '+' | '*' => {
            let len = rest.chars().next().unwrap_or('\0').len_utf8();
            rest = &rest[len..];
        }
        c if c.is_ascii_digit() => {
            let mut end = 0usize;
            for ch in rest.chars() {
                if ch.is_ascii_digit() {
                    end += ch.len_utf8();
                } else {
                    break;
                }
            }
            let after_digits = &rest[end..];
            if !after_digits.starts_with('.') {
                return None;
            }
            rest = &after_digits[1..];
        }
        _ => return None,
    }

    // `\s+` after the bullet.
    let mut cut = 0usize;
    let mut seen = false;
    for (i, c) in rest.char_indices() {
        if c.is_whitespace() {
            cut = i + c.len_utf8();
            seen = true;
        } else {
            break;
        }
    }
    if !seen {
        return None;
    }
    rest = &rest[cut..];

    // `\[( |x|X)\]`
    let mut chars = rest.chars();
    if chars.next()? != '[' {
        return None;
    }
    let state = chars.next()?;
    if state != ' ' && state != 'x' && state != 'X' {
        return None;
    }
    if chars.next()? != ']' {
        return None;
    }
    rest = &rest[3..];

    // `\s+` after the bracket, then `(.*)$` (may be empty).
    let mut tail_cut = 0usize;
    let mut tail_seen = false;
    for (i, c) in rest.char_indices() {
        if c.is_whitespace() {
            tail_cut = i + c.len_utf8();
            tail_seen = true;
        } else {
            break;
        }
    }
    if !tail_seen {
        return None;
    }
    let tail = &rest[tail_cut..];

    let marker = if state == 'x' || state == 'X' {
        "✅"
    } else {
        "🔲"
    };
    Some(format!("{indent}{marker} {tail}"))
}

/// Strip a leading `>` plus one optional whitespace char
/// (TS: `trimmedLeft.replace(/^>\s?/, "")`). Caller must have checked the
/// `>` prefix.
fn strip_quote_prefix(trimmed_left: &str) -> &str {
    let after = &trimmed_left[1..];
    match after.chars().next() {
        Some(c) if c.is_whitespace() => &after[c.len_utf8()..],
        _ => after,
    }
}

fn normalize_markdown(text: &str, preserve_block_markup: bool) -> String {
    // TS: `text.split("\n")` ... `output.join("\n")`.
    let lines: Vec<&str> = text.split('\n').collect();
    let mut output: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_code_fence = false;
    let mut in_quote = false;

    for line in lines {
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

        if is_heading_line(line) {
            if preserve_block_markup {
                output.push(line.to_string());
            } else {
                output.push(normalize_heading_line(line));
            }
            in_quote = false;
            continue;
        }

        if is_horizontal_rule_line(line) {
            if preserve_block_markup {
                output.push(line.to_string());
            } else {
                output.push(RULE_LINE.to_string());
            }
            in_quote = false;
            continue;
        }

        let trimmed_left = line.trim_start();
        if trimmed_left.starts_with('>') {
            in_quote = true;
            let quote_content = strip_quote_prefix(trimmed_left);
            if let Some(normalized) = normalize_checklist_line(quote_content) {
                if !preserve_block_markup {
                    output.push(format!("> {}", normalized.trim_start()));
                    continue;
                }
            }
            output.push(format!("> {}", quote_content.trim_start()));
            continue;
        }

        if let Some(normalized) = normalize_checklist_line(line) {
            if preserve_block_markup {
                if in_quote {
                    output.push(format!("> {trimmed_left}"));
                } else {
                    output.push(line.to_string());
                }
            } else if in_quote {
                output.push(format!("> {}", normalized.trim_start()));
            } else {
                output.push(normalized);
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

/// Normalize markdown for final Telegram rendering: headings become bold,
/// horizontal rules become a box-drawing line, checklists become emoji
/// markers, and quote continuations gain an explicit `>` prefix
/// (TS: `normalizeMarkdownForTelegramRendering`).
pub fn normalize_markdown_for_telegram_rendering(text: &str) -> String {
    normalize_markdown(text, false)
}

/// Normalize markdown for block parsing: structural markup (headings, rules,
/// checklists) is preserved, but quote continuation lines still gain an
/// explicit `>` prefix (TS: `normalizeMarkdownForTelegramBlockParsing`).
pub fn normalize_markdown_for_telegram_block_parsing(text: &str) -> String {
    normalize_markdown(text, true)
}
