//! Block-parser + keyboard-debounce + interaction-guard + message-router gates (slice 99, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/render/block-parser.ts` (`MAX_NESTING_DEPTH` 6, heading
//!   level clamp `Math.min(6, Math.max(1, floor(depth)))`, `prefixLines`,
//!   `createPlainBlock` trim/empty gate, `parseColumnAlign` all-null gate
//!   with `null -> "left"`, ordered `-` / `N. ` markers, `checked`
//!   `✅ ` / `🔲 ` prefixes, list-item `trimEnd`, table-row `" | "`
//!   join, `lang ?? undefined` code language, `depth >= 6` degrade gate,
//!   empty-table / empty-list gates, `thematicBreak` rule text)
//! - `src/bot/render/pipeline.ts` (thin glue only: `toRenderedBlocks` is
//!   split-then-map over `liftTextListContent`; no new pure predicate, so
//!   the pipeline contributes no gate of its own here)
//! - `src/bot/keyboards/keyboard-manager.ts` (`UPDATE_DEBOUNCE_MS` 2000,
//!   `now - last < 2000` debounce, `state !== null` initialized gate,
//!   `model.variant || "default"` blank-variant gate, fresh-context
//!   `tokensUsed === 0` gate, `"build"` fallback agent,
//!   `keyboard.updated` notice key)
//! - `src/bot/middleware/interaction-guard.ts`
//!   (`getInteractionBlockedMessage` kind x reason key matrix,
//!   `getQueuedPhotoMediaBytes` empty-`0` / invalid-`undefined` /
//!   checked-sum gate, `isQueueableInput`
//!   `busy && !state && text && prompt` gate)
//! - `src/bot/routers/message-router.ts` (`withdrawInboxPrompt`
//!   `removed` / `gone` / error reply-key choice,
//!   `blockMenuWhileInteractionActive` active gate, `/`-prefix command
//!   gate)
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `formatVariantForButton` rendering (`variant_selection` owner; this
//!   slice owns only the `|| "default"` input gate).
//! - `resolveInteractionGuardDecision` itself (`interaction_guard_edge_gates`).
//! - `liftTextListContent` / `splitOversizeTelegramBlocks` / `toRichBlock`
//!   (`rich_blocks` / `block_split_gates` owners).
//! - `t(...)` lookups: reply keys travel as literal key strings.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - mdast parsing (`unified`/`remarkParse` trees arrive as pre-shaped
//!   scalars), grammy `Api`/`ctx` I/O, `Date.now()` clocks (arrive as
//!   explicit `now_ms` args), `promptQueue` / `keyboardManager` /
//!   `pinnedMessageManager` state, network `fetch`.
//!
//! Deviations: none intended. `String.length` parity fixtures use
//! ASCII-only inputs where JS UTF-16 code units and Rust `char`s agree.
//! `Math.floor` on whole-number heading depths is exact.
//!
//! Step 2 (GREEN): pure ports of the TS predicates above.

/// Rich messages allow this many nesting levels before quotes and lists
/// degrade to plain text (TS: `MAX_NESTING_DEPTH`).
pub const MAX_NESTING_DEPTH: u32 = 6;

/// Minimum gap between keyboard pushes (TS: `UPDATE_DEBOUNCE_MS`).
pub const KEYBOARD_UPDATE_DEBOUNCE_MS: u64 = 2000;

/// Rule text for a thematic break (TS: `"──────────"`).
pub const THEMATIC_BREAK_TEXT: &str = "──────────";

/// Agent used for the fallback keyboard (TS: `"build"`).
pub const FALLBACK_AGENT: &str = "build";

/// Notice sent with a keyboard push (TS: `t("keyboard.updated")`).
pub const KEYBOARD_UPDATED_KEY: &str = "keyboard.updated";

/// Heading level clamped to 1..=6 (TS: `Math.min(6, Math.max(1, floor(depth)))`).
pub fn clamp_heading_level(depth: i64) -> u8 {
    depth.clamp(1, 6) as u8
}

/// Prefix every line (TS: `prefixLines`).
pub fn prefix_lines(text: &str, prefix: &str) -> String {
    text.split('\n')
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Trimmed plain text, or `None` when blank (TS: `createPlainBlock` gate).
pub fn create_plain_text(text: &str) -> Option<String> {
    let normalized = text.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

/// Column alignment, or `None` when unset (TS: `parseColumnAlign`:
/// `!align || every(null)` -> `undefined`, else `null -> "left"`).
pub fn parse_column_align(align: &[Option<&str>]) -> Option<Vec<String>> {
    if align.is_empty() || align.iter().all(|value| value.is_none()) {
        return None;
    }
    Some(
        align
            .iter()
            .map(|value| value.unwrap_or("left").to_string())
            .collect(),
    )
}

/// List marker (TS: ordered `` `${index + 1}. ` `` else `"- "`).
pub fn list_marker(ordered: bool, index: usize) -> String {
    if ordered {
        format!("{}. ", index + 1)
    } else {
        "- ".to_string()
    }
}

/// Checkbox prefix (TS: `checked === true` -> `"✅ "`,
/// `=== false` -> `"🔲 "`, else `""`).
pub fn checkbox_prefix(checked: Option<bool>) -> &'static str {
    match checked {
        Some(true) => "✅ ",
        Some(false) => "🔲 ",
        None => "",
    }
}

/// List-item line with trailing whitespace removed
/// (TS: `` `${marker}${prefix}${body}`.trimEnd() ``).
pub fn render_list_item(marker: &str, prefix: &str, body: &str) -> String {
    format!("{marker}{prefix}{body}").trim_end().to_string()
}

/// Table row joined for the plain projection (TS: `join(" | ")`).
pub fn join_table_row(cells: &[&str]) -> String {
    cells.join(" | ")
}

/// Code fence language (TS: `node.lang ?? undefined`).
pub fn code_language(lang: Option<&str>) -> Option<String> {
    lang.map(str::to_string)
}

/// `true` when nesting degrades to plain text (TS: `depth >= MAX_NESTING_DEPTH`).
pub fn should_degrade_to_plain(depth: u32) -> bool {
    depth >= MAX_NESTING_DEPTH
}

/// `true` when a table has no rows (TS: `rows.length === 0`).
pub fn is_table_empty(row_count: usize) -> bool {
    row_count == 0
}

/// `true` when a list has no items (TS: `items.length === 0`).
pub fn is_list_empty(item_count: usize) -> bool {
    item_count == 0
}

/// `true` when the push is inside the debounce window
/// (TS: `now - lastUpdateTime < UPDATE_DEBOUNCE_MS`).
pub fn should_debounce_keyboard_update(now_ms: i64, last_ms: i64) -> bool {
    (now_ms as i128 - last_ms as i128) < KEYBOARD_UPDATE_DEBOUNCE_MS as i128
}

/// `true` once state exists (TS: `state !== null`).
pub fn is_keyboard_initialized(has_state: bool) -> bool {
    has_state
}

/// Variant input with the blank fallback applied
/// (TS: `model.variant || "default"`).
pub fn or_default_variant<'a>(variant: Option<&'a str>) -> &'a str {
    match variant {
        Some(value) if !value.is_empty() => value,
        _ => "default",
    }
}

/// `true` for a freshly switched context (TS: `tokensUsed === 0`).
pub fn is_fresh_context(tokens_used: u64) -> bool {
    tokens_used == 0
}

/// Blocked-input i18n key by interaction kind and reason
/// (TS: `getInteractionBlockedMessage`).
pub fn interaction_blocked_message_key(
    kind: Option<&str>,
    reason: Option<&str>,
) -> &'static str {
    match kind {
        Some("permission") => match reason {
            Some("command_not_allowed") => "permission.blocked.command_not_allowed",
            _ => "permission.blocked.expected_reply",
        },
        Some("inline") => match reason {
            Some("command_not_allowed") => "inline.blocked.command_not_allowed",
            _ => "inline.blocked.expected_choice",
        },
        Some("question") => match reason {
            Some("command_not_allowed") => "question.blocked.command_not_allowed",
            _ => "question.blocked.expected_answer",
        },
        Some("rename") => match reason {
            Some("command_not_allowed") => "rename.blocked.command_not_allowed",
            _ => "rename.blocked.expected_name",
        },
        Some("task") => match reason {
            Some("command_not_allowed") => "task.blocked.command_not_allowed",
            _ => "task.blocked.expected_input",
        },
        _ => match reason {
            Some("expired") => "interaction.blocked.expired",
            Some("expected_callback") => "interaction.blocked.expected_callback",
            Some("expected_command") => "interaction.blocked.expected_command",
            Some("command_not_allowed") => "interaction.blocked.command_not_allowed",
            _ => "interaction.blocked.expected_text",
        },
    }
}

/// Maximum integer exactly representable as an f64 (TS: `Number.isSafeInteger`
/// upper bound). Inputs or sums outside `0..=MAX_SAFE_INTEGER` read as
/// invalid, mirroring the TS `getQueuedPhotoMediaBytes` gate.
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Summed photo bytes, or `None` when a size is missing/invalid or the
/// sum overflows (TS: `getQueuedPhotoMediaBytes`; empty reads as `0`).
pub fn sum_photo_media_bytes(sizes: &[Option<i64>]) -> Option<i64> {
    if sizes.is_empty() {
        return Some(0);
    }
    let mut total: i64 = 0;
    for size in sizes {
        let value = match size {
            Some(value) => *value,
            None => return None,
        };
        if value < 0 || value > MAX_SAFE_INTEGER {
            return None;
        }
        total = total.checked_add(value)?;
        if total > MAX_SAFE_INTEGER {
            return None;
        }
    }
    Some(total)
}

/// `true` when a text prompt queues behind a busy session
/// (TS: `busy && !state && inputType === "text" && incomingPrompt`).
pub fn should_queue_text_prompt(
    busy: bool,
    has_state: bool,
    input_is_text: bool,
    has_prompt: bool,
) -> bool {
    busy && !has_state && input_is_text && has_prompt
}

/// Reply key for a queued-prompt withdraw
/// (TS: `removed -> "queue.removed"`, `gone -> "queue.not_found"`,
/// else `"bot.prompt_send_error"`).
pub fn withdraw_reply_key(result: &str) -> &'static str {
    match result {
        "removed" => "queue.removed",
        "gone" => "queue.not_found",
        _ => "bot.prompt_send_error",
    }
}

/// `true` when a menu open is blocked by an active interaction
/// (TS: `getSnapshot()` non-null gate).
pub fn should_block_menu_while_active(has_active: bool) -> bool {
    has_active
}

/// `true` for `/command` text (TS: `text.startsWith("/")`).
pub fn is_command_text(text: &str) -> bool {
    text.starts_with('/')
}
