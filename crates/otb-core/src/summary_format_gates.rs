//! Summary-message pure gates (slice 46, GREEN).
//!
//! Rust port of the pure splitting/formatting logic behind
//! `src/bot/messages/summary-message-formatter.ts`:
//! - `endsWithOddTrailingBackslashes` (odd-backslash guard for split points)
//! - `resolveSplitEndIndex` (newline-preferring split with escape guard)
//! - `splitText` (newline-aware chunking with single-char fallback)
//! - `escapeMarkdownV2PipesOutsideCode` (bare `|` escape outside code)
//! - `formatSummaryWithMode` (`raw` fence-wrap vs `markdown` convert+split)
//!
//! Transport (`config.bot.messageFormatMode` default in `formatSummary`,
//! logging) stays out; this module only decides *how* to chunk and format
//! summary text.
//!
//! Index contract (documented): the TS source counts `String.length` /
//! `slice` / `lastIndexOf` in UTF-16 code units. The Rust port counts
//! `usize` indices in `char`s (Unicode scalar values). Parity tests use
//! ASCII-only inputs where UTF-16 units, bytes, and chars coincide, so the
//! port is exact there. For non-BMP input (e.g. emoji, which is 2 UTF-16
//! units but 1 Rust `char` / 4 bytes) split positions and chunk lengths
//! may differ from TS by design; see DEVIATIONS below.
//!
//! DEVIATIONS from TS:
//! 1. UTF-16 vs chars: indices/lengths are `char`-based, not UTF-16
//!    code units. ASCII parity is exact; supplementary-plane text can
//!    split at slightly different offsets.
//! 2. Markdown path: TS `formatMarkdownForTelegram` runs
//!    `normalizeMarkdownForTelegramRendering` +
//!    `convertToTelegramMarkdownV2` (full Markdown→Telegram MarkdownV2
//!    conversion) and then `escapeMarkdownV2PipesOutsideCode`. That
//!    converter lives outside this pure-gate slice, so the Rust port
//!    applies only `escape_markdown_v2_pipes_outside_code` to the trimmed
//!    part before re-splitting. Content of converted markdown parts may
//!    differ from TS; chunking/limit guarantees are preserved.
//! 3. `trim()`: TS `String.trim()` vs Rust `str::trim()` agree on ASCII
//!    whitespace; exotic Unicode whitespace sets differ slightly.
//! 4. `maxLength` is `usize` (already an integer >= 0), so TS
//!    `Math.floor` is a no-op; `Math.max(1, …)` normalization is kept via
//!    `.max(1)` / `saturating_sub`.

/// Default Telegram text-message ceiling (`TELEGRAM_MESSAGE_LIMIT`).
pub const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

/// Overhead the raw formatter reserves per chunk (TS `"```\n\n```".length`).
pub const RAW_FENCE_OVERHEAD: usize = 8;

/// Message format mode (TS `MessageFormatMode` in `src/config.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageFormatMode {
    Raw,
    Markdown,
}

/// Odd-backslash guard for a `[start, end)` window (TS
/// `endsWithOddTrailingBackslashes`).
pub fn ends_with_odd_trailing_backslashes(text: &str, start: usize, end: usize) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let start = start.min(len);
    let end = end.min(len);
    if end <= start {
        return false;
    }
    let mut backslash_count = 0usize;
    let mut index = end;
    while index > start {
        index -= 1;
        if chars[index] != '\\' {
            break;
        }
        backslash_count += 1;
    }
    backslash_count % 2 == 1
}

/// Newline-preferring split end for a chunk starting at `current_index`
/// (TS `resolveSplitEndIndex`).
pub fn resolve_split_end_index(
    text: &str,
    current_index: usize,
    max_length: usize,
    avoid_trailing_markdown_escape: bool,
) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let current_index = current_index.min(len);
    let hard_limit = len.min(current_index.saturating_add(max_length));
    if hard_limit >= len {
        return len;
    }

    let mut end_index = hard_limit;
    // TS: text.lastIndexOf("\n", endIndex) — backwards search inclusive of
    // endIndex. Find the greatest i <= endIndex with chars[i] == '\n'.
    let search_end = end_index.min(len.saturating_sub(1));
    let mut i = search_end + 1;
    while i > 0 {
        i -= 1;
        if chars[i] == '\n' {
            if i > current_index {
                end_index = i + 1;
            }
            break;
        }
        if i == 0 {
            break;
        }
    }

    if !avoid_trailing_markdown_escape {
        return end_index;
    }

    // Rebuild the string once for the guard (ASCII parity; char-based).
    let text_owned: String = chars.iter().collect();
    while end_index > current_index
        && ends_with_odd_trailing_backslashes(&text_owned, current_index, end_index)
    {
        end_index -= 1;
    }

    if end_index > current_index {
        end_index
    } else {
        hard_limit
    }
}

/// Newline-aware chunking of `text` into `max_length` pieces (TS
/// `splitText`).
pub fn split_text(
    text: &str,
    max_length: usize,
    avoid_trailing_markdown_escape: bool,
) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut parts: Vec<String> = Vec::new();
    let mut current_index = 0usize;

    while current_index < len {
        let owned: String = chars.iter().collect();
        let end_index = resolve_split_end_index(
            &owned,
            current_index,
            max_length,
            avoid_trailing_markdown_escape,
        );

        if end_index <= current_index {
            let fallback_end = len.min(current_index.saturating_add(1));
            parts.push(chars[current_index..fallback_end].iter().collect());
            current_index = fallback_end;
            continue;
        }

        parts.push(chars[current_index..end_index].iter().collect());
        current_index = end_index;
    }

    parts
}

/// Escape bare `|` outside inline code / fences (TS
/// `escapeMarkdownV2PipesOutsideCode`).
pub fn escape_markdown_v2_pipes_outside_code(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(text.len() + 8);
    let mut index = 0usize;
    let mut in_inline_code = false;
    let mut in_code_fence = false;

    while index < len {
        if index + 3 <= len
            && chars[index] == '`'
            && chars[index + 1] == '`'
            && chars[index + 2] == '`'
        {
            result.push_str("```");
            index += 3;
            in_code_fence = !in_code_fence;
            continue;
        }

        let ch = chars[index];

        if !in_code_fence && ch == '`' {
            in_inline_code = !in_inline_code;
            result.push(ch);
            index += 1;
            continue;
        }

        if !in_code_fence
            && !in_inline_code
            && ch == '|'
            && (index == 0 || chars[index - 1] != '\\')
        {
            result.push_str("\\|");
            index += 1;
            continue;
        }

        result.push(ch);
        index += 1;
    }

    result
}

/// Chunk + format a summary for Telegram (TS `formatSummaryWithMode`).
pub fn format_summary_with_mode(
    text: &str,
    mode: MessageFormatMode,
    max_length: usize,
) -> Vec<String> {
    if text.is_empty() || text.trim().is_empty() {
        return Vec::new();
    }

    let normalized_max_length = max_length.max(1);
    let raw_text_limit = if mode == MessageFormatMode::Raw {
        normalized_max_length
            .saturating_sub(RAW_FENCE_OVERHEAD)
            .max(1)
    } else {
        normalized_max_length
    };
    let parts = split_text(text, raw_text_limit, false);
    let mut formatted_parts: Vec<String> = Vec::new();

    for part in &parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        if mode == MessageFormatMode::Markdown {
            // TS runs normalizeMarkdownForTelegramRendering +
            // convertToTelegramMarkdownV2 here; this gate approximates with
            // the pipe-escape only (see DEVIATIONS).
            let converted = escape_markdown_v2_pipes_outside_code(trimmed);
            let converted_parts = split_text(&converted, normalized_max_length, true);
            for converted_part in &converted_parts {
                let normalized_part = converted_part.trim();
                if !normalized_part.is_empty() {
                    formatted_parts.push(normalized_part.to_string());
                }
            }
            continue;
        }

        if parts.len() > 1 {
            formatted_parts.push(format!("```\n{trimmed}\n```"));
        } else {
            formatted_parts.push(trimmed.to_string());
        }
    }

    formatted_parts
}
