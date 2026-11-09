//! Pure streaming / guard state machines.
//!
//! Rust port of the pure gates in:
//! - `src/bot/streaming/tool-call-streamer.ts` (stream keys, part building,
//!   long-text splitting, retry-after parsing, gate/throttle/break rules)
//! - `src/bot/streaming/response-streamer.ts` (signature skip, entity-aware
//!   edits, plain fallback, re-chunking, draft mode, flushPending rules)
//! - `src/bot/streaming/running-tool-tracker.ts` (threshold/bucket ticks,
//!   newest ordering, background/detach, display ticks)
//! - `src/bot/streaming/compact-progress-streamer.ts` (unique counts,
//!   hold/release, parked background cards)
//! - `src/bot/middleware/interaction-guard-decision.ts` (command
//!   normalization, input classification, block reasons, busy allowlists)
//! - `src/bot/streaming/stream-throttle.ts` (progressive throttle tiers)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/streaming_parity.rs` assert the TS
//! contract and MUST FAIL until the Rust port lands (step 2).

// --- Shared budgets (mirrors tool-call-streamer.ts / chunker limits) ---

/// Safe text length for one Telegram tool-stream message.
pub const TELEGRAM_MESSAGE_SAFE_LENGTH: usize = 4000;
/// Hard Telegram text message limit.
pub const TELEGRAM_TEXT_MESSAGE_LIMIT: usize = 4096;
/// Base progressive throttle (stream-throttle.ts).
pub const STREAM_THROTTLE_BASE_MS: u64 = 1000;
/// Max progressive throttle (stream-throttle.ts).
pub const STREAM_THROTTLE_MAX_MS: u64 = 10_000;

// --- Stream keys (tool-call-streamer.ts: ToolStreamKey) ---

/// Which per-session message stream a tool line belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamKey {
    Default,
    Todo,
    Subagent(String),
}

/// Classify a stream key string (`"default"` / `"todo"` / `"subagent:<id>"`).
pub fn classify_stream_key(key: &str) -> StreamKey {
    if key == "default" {
        StreamKey::Default
    } else if key == "todo" {
        StreamKey::Todo
    } else if let Some(id) = key.strip_prefix("subagent:") {
        StreamKey::Subagent(id.to_string())
    } else {
        StreamKey::Default
    }
}

/// Render a stream key back to its canonical string form.
pub fn stream_key_name(key: &StreamKey) -> String {
    match key {
        StreamKey::Default => "default".to_string(),
        StreamKey::Todo => "todo".to_string(),
        StreamKey::Subagent(id) => format!("subagent:{}", id),
    }
}

// --- Part building / splitting (tool-call-streamer.ts) ---

/// Join trimmed non-empty entries with `"\n\n"` (TS `buildParts`).
pub fn build_stream_parts(entries: &[String]) -> Vec<String> {
    let text = entries
        .iter()
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.is_empty() {
        return Vec::new();
    }

    split_long_text(&text, TELEGRAM_MESSAGE_SAFE_LENGTH)
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect()
}

/// Split long text at newlines when possible, else hard-cut (TS `splitLongText`).
pub fn split_long_text(text: &str, limit: usize) -> Vec<String> {
    if limit == 0 {
        return if text.is_empty() {
            Vec::new()
        } else {
            vec![text.to_string()]
        };
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= limit {
        return vec![text.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut start = 0;
    while chars.len() - start > limit {
        let window_end = start + limit;
        // Mirror `lastIndexOf("\n", limit)`: newest newline at or before the cut.
        let mut newline: Option<usize> = None;
        for offset in (0..=limit).rev() {
            if chars[start + offset] == '\n' {
                newline = Some(start + offset);
                break;
            }
        }
        let half = limit / 2;
        let split = match newline {
            Some(pos) if pos > start && pos - start >= half => pos,
            _ => window_end,
        };

        chunks.push(chars[start..split].iter().collect());
        start = split;
        while start < chars.len() && chars[start] == '\n' {
            start += 1;
        }
        if start >= chars.len() {
            break;
        }
    }

    if start < chars.len() {
        chunks.push(chars[start..].iter().collect());
    }
    chunks
}

/// Parse `429 … retry after N` into milliseconds, else `None` (TS `getRetryAfterMs`).
pub fn parse_retry_after_ms(message: &str) -> Option<u64> {
    fn is_word_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    let bytes = message.as_bytes();
    let mut found_429 = false;
    let mut index = 0;
    while index + 3 <= bytes.len() {
        if &bytes[index..index + 3] == b"429" {
            let before_ok = index == 0 || !is_word_byte(bytes[index - 1]);
            let after_ok = index + 3 >= bytes.len() || !is_word_byte(bytes[index + 3]);
            if before_ok && after_ok {
                found_429 = true;
                break;
            }
        }
        index += 1;
    }
    if !found_429 {
        return None;
    }

    let lower = message.to_lowercase();
    let marker = "retry after";
    let pos = lower.find(marker)?;
    let after = &lower[pos + marker.len()..];
    let trimmed = after.trim_start();
    // TS requires at least one `\s+` between the marker and the digits.
    if trimmed.len() == after.len() {
        return None;
    }
    let digits_end = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let digits = &trimmed[..digits_end];
    if digits.is_empty() {
        return None;
    }
    let seconds: u64 = digits.parse().ok()?;
    if seconds == 0 {
        return None;
    }
    Some(seconds.saturating_mul(1000))
}

// --- Throttle tiers (stream-throttle.ts) ---

/// Progressive throttle tier for elapsed stream time (1s / 2s / 5s / 10s).
pub fn resolve_progressive_throttle_ms(elapsed_ms: u64) -> u64 {
    const MINUTE_MS: u64 = 60_000;
    if elapsed_ms < MINUTE_MS {
        STREAM_THROTTLE_BASE_MS
    } else if elapsed_ms < 5 * MINUTE_MS {
        2000
    } else if elapsed_ms < 15 * MINUTE_MS {
        5000
    } else {
        STREAM_THROTTLE_MAX_MS
    }
}

/// Floor a (possibly dynamic) throttle value at zero (TS `resolveStreamThrottleMs`).
pub fn resolve_stream_throttle_ms(value: i64) -> u64 {
    value.max(0) as u64
}

// --- Response-streamer pure rules ---

/// True when the new payload signature differs from what is on screen
/// (entity-aware: identical text with different entities still edits).
pub fn should_sync_payload(last_sent_signatures: &[String], new_signatures: &[String]) -> bool {
    last_sent_signatures != new_signatures
}

/// Split over-limit plain fallback text so every part fits `limit` chars.
pub fn chunk_plain_fallback(text: &str, limit: usize) -> Vec<String> {
    // Mirrors `splitTextIntoChunks`: prefer a line break, then any whitespace,
    // then a hard cut. Unlike `splitLongText`, boundaries are kept verbatim so
    // the chunks rejoin to the original text.
    let limit = limit.max(1);
    if text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= limit {
        return vec![text.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let hard_end = (start + limit).min(chars.len());
        if hard_end >= chars.len() {
            chunks.push(chars[start..].iter().collect());
            break;
        }

        let mut boundary: Option<usize> = None;
        let mut whitespace: Option<usize> = None;
        let mut fallback: Option<usize> = None;
        let mut index = hard_end;
        while index > start {
            if chars[index - 1] == '\n' {
                boundary = Some(index);
                break;
            }
            if whitespace.is_none() && chars[index - 1].is_whitespace() {
                whitespace = Some(index);
            }
            if fallback.is_none() {
                fallback = Some(index);
            }
            index -= 1;
        }
        let end = boundary.or(whitespace).or(fallback).unwrap_or(hard_end);
        chunks.push(chars[start..end].iter().collect());
        start = end;
    }
    chunks
}

/// How many trailing on-screen messages to delete when the payload shrinks.
pub fn extra_message_delete_count(visible: usize, needed: usize) -> usize {
    visible.saturating_sub(needed)
}

/// True when `flushPending` may send immediately (nothing on screen yet and
/// the stream is not broken).
pub fn should_flush_pending_immediately(visible_count: usize, is_broken: bool) -> bool {
    visible_count == 0 && !is_broken
}

// --- Running-tool-tracker pure rules ---

/// Round elapsed time down to the display step (TS `bucketElapsedMs`).
pub fn bucket_elapsed_ms(elapsed_ms: i64) -> u64 {
    if elapsed_ms <= 0 {
        return 0;
    }
    let elapsed = elapsed_ms as u64;
    const MINUTE_MS: u64 = 60_000;
    const TEN_MINUTES_MS: u64 = 10 * MINUTE_MS;
    const HOUR_MS: u64 = 60 * MINUTE_MS;
    if elapsed < MINUTE_MS {
        elapsed / 10_000 * 10_000
    } else if elapsed < TEN_MINUTES_MS {
        elapsed / MINUTE_MS * MINUTE_MS
    } else if elapsed < HOUR_MS {
        elapsed / (5 * MINUTE_MS) * (5 * MINUTE_MS)
    } else {
        elapsed / (15 * MINUTE_MS) * (15 * MINUTE_MS)
    }
}

/// True when a tick should be emitted (past threshold and bucket changed).
pub fn should_emit_tick(elapsed_ms: u64, threshold_ms: u64, last_bucket_ms: Option<u64>) -> bool {
    if elapsed_ms < threshold_ms {
        return false;
    }
    let bucket = bucket_elapsed_ms(i64::try_from(elapsed_ms).unwrap_or(i64::MAX));
    match last_bucket_ms {
        None => true,
        Some(last) => bucket != last,
    }
}

/// True when tracking hit the cap and the tick is final.
pub fn is_final_tick(elapsed_ms: u64, max_tracking_ms: u64) -> bool {
    elapsed_ms >= max_tracking_ms
}

/// Pick the newest live call id for a session (later start wins, released and
/// detached calls excluded). `calls` are `(call_id, started_at_ms, released,
/// detached)`.
pub fn newest_call_id(calls: &[(&str, u64, bool, bool)]) -> Option<String> {
    let mut best: Option<(&str, u64)> = None;
    for call in calls {
        let (call_id, started_at_ms, released, detached) = *call;
        if released || detached {
            continue;
        }
        match best {
            // Later starts win; a later entry also wins ties (insertion order).
            Some((_, best_started_at)) if started_at_ms < best_started_at => {}
            _ => best = Some((call_id, started_at_ms)),
        }
    }
    best.map(|(call_id, _)| call_id.to_string())
}

// --- Compact-progress pure rules ---

/// What one finished call adds to a compact card's summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolCallCounts {
    pub commands: u32,
    pub edits: u32,
    pub reads: u32,
    pub tools: u32,
}

/// Zero counts (TS `emptyToolCallCounts`).
pub fn empty_tool_call_counts() -> ToolCallCounts {
    ToolCallCounts::default()
}

/// Merge one call's counts into the running totals.
pub fn add_tool_call_counts(total: ToolCallCounts, delta: ToolCallCounts) -> ToolCallCounts {
    ToolCallCounts {
        commands: total.commands.saturating_add(delta.commands),
        edits: total.edits.saturating_add(delta.edits),
        reads: total.reads.saturating_add(delta.reads),
        tools: total.tools.saturating_add(delta.tools),
    }
}

/// Render the `✅ …` summary line, or `None` when there is nothing to count.
pub fn format_compact_summary(counts: ToolCallCounts, changed_files: usize) -> Option<String> {
    let mut entries: Vec<String> = Vec::new();
    if counts.commands > 0 {
        entries.push(format!("commands: {}", counts.commands));
    }
    if counts.edits > 0 {
        entries.push(format!("edits: {}", counts.edits));
    }
    if counts.reads > 0 {
        entries.push(format!("reads: {}", counts.reads));
    }
    if counts.tools > 0 {
        entries.push(format!("tools: {}", counts.tools));
    }
    if changed_files > 0 {
        entries.push(format!("changed files: {}", changed_files));
    }
    if entries.is_empty() {
        None
    } else {
        Some(format!("✅ {}", entries.join(", ")))
    }
}

/// Render the `⏳ Working\n…` progress line.
pub fn format_progress_text(activity: &str) -> String {
    format!("⏳ Working\n{}", activity)
}

// --- Interaction-guard decision pure rules ---

/// Live interaction kinds (TS `InteractionKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionKind {
    Inline,
    Permission,
    Question,
    Rename,
    Task,
    Custom,
}

/// What the active interaction accepts (TS `ExpectedInput`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedInput {
    Callback,
    Text,
    Command,
    Mixed,
}

/// Classified incoming update (TS `IncomingInputType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncomingInputType {
    Callback,
    Command,
    Text,
    Other,
}

/// Why the guard blocked an update (TS `BlockReason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    Expired,
    ExpectedCallback,
    ExpectedText,
    ExpectedCommand,
    CommandNotAllowed,
}

/// Allow / block verdict (TS `GuardDecision`, transport-free subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDecision {
    pub allow: bool,
    pub input_type: IncomingInputType,
    pub reason: Option<BlockReason>,
    pub command: Option<String>,
    pub busy: bool,
}

/// Lowercase `/command` token without `@mention` or args, else `None`.
pub fn normalize_incoming_command(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let token = trimmed.split_whitespace().next()?;
    let without_mention = token.split('@').next().unwrap_or("").to_lowercase();
    if without_mention.len() <= 1 {
        return None;
    }

    Some(without_mention)
}

/// Classify an update from its callback data / message text / media flags.
pub fn classify_incoming_input(
    callback_data: Option<&str>,
    message_text: Option<&str>,
    has_media: bool,
) -> IncomingInputType {
    let _ = has_media;
    if callback_data.is_some_and(|data| !data.is_empty()) {
        return IncomingInputType::Callback;
    }
    if let Some(text) = message_text {
        if normalize_incoming_command(text).is_some() {
            return IncomingInputType::Command;
        }
        return IncomingInputType::Text;
    }
    IncomingInputType::Other
}

/// Map an interaction's expected input to its block reason.
pub fn expected_input_block_reason(expected: ExpectedInput) -> BlockReason {
    match expected {
        ExpectedInput::Callback => BlockReason::ExpectedCallback,
        ExpectedInput::Command => BlockReason::ExpectedCommand,
        ExpectedInput::Text | ExpectedInput::Mixed => BlockReason::ExpectedText,
    }
}

/// True for commands that pass the busy gate (`/abort`, `/detach`, `/status`,
/// `/help`, `/opencode_stop`).
pub fn is_busy_allowed_command(command: Option<&str>) -> bool {
    matches!(
        command,
        Some("/abort") | Some("/detach") | Some("/status") | Some("/help") | Some("/opencode_stop")
    )
}

/// True for question/permission kinds, which may interact while busy.
pub fn allows_busy_interaction(kind: Option<InteractionKind>) -> bool {
    matches!(
        kind,
        Some(InteractionKind::Question) | Some(InteractionKind::Permission)
    )
}

fn input_matches_expected(expected: ExpectedInput, input: IncomingInputType) -> bool {
    match expected {
        ExpectedInput::Callback => input == IncomingInputType::Callback,
        ExpectedInput::Text => input == IncomingInputType::Text,
        ExpectedInput::Command => input == IncomingInputType::Command,
        ExpectedInput::Mixed => {
            input == IncomingInputType::Callback || input == IncomingInputType::Text
        }
    }
}

/// Pure allow/block verdict for an interaction + busy snapshot.
/// `interaction`: `(kind, expected_input, allowed_commands)` when a slot is live.
/// `expired`: the slot lapsed and must report `Expired`.
/// `busy`: a foreground run holds the session.
#[allow(clippy::too_many_arguments)]
pub fn decide_guard(
    interaction: Option<(InteractionKind, ExpectedInput, &[String])>,
    expired: bool,
    busy: bool,
    input: IncomingInputType,
    command: Option<&str>,
) -> GuardDecision {
    let command_owned = command.map(|value| value.to_string());

    if expired && interaction.is_some() {
        return GuardDecision {
            allow: false,
            input_type: input,
            reason: Some(BlockReason::Expired),
            command: command_owned,
            busy,
        };
    }

    if busy {
        if input == IncomingInputType::Command {
            if is_busy_allowed_command(command) {
                return GuardDecision {
                    allow: true,
                    input_type: input,
                    reason: None,
                    command: command_owned,
                    busy: true,
                };
            }
            return GuardDecision {
                allow: false,
                input_type: input,
                reason: Some(BlockReason::CommandNotAllowed),
                command: command_owned,
                busy: true,
            };
        }
        if let Some((kind, expected, _allowed_commands)) = interaction {
            if allows_busy_interaction(Some(kind)) {
                if input_matches_expected(expected, input) {
                    return GuardDecision {
                        allow: true,
                        input_type: input,
                        reason: None,
                        command: command_owned,
                        busy: true,
                    };
                }
                return GuardDecision {
                    allow: false,
                    input_type: input,
                    reason: Some(expected_input_block_reason(expected)),
                    command: command_owned,
                    busy: true,
                };
            }
        }
        return GuardDecision {
            allow: false,
            input_type: input,
            reason: Some(BlockReason::ExpectedText),
            command: command_owned,
            busy: true,
        };
    }

    let Some((_, expected, allowed_commands)) = interaction else {
        return GuardDecision {
            allow: true,
            input_type: input,
            reason: None,
            command: command_owned,
            busy: false,
        };
    };

    if input == IncomingInputType::Command {
        if command == Some("/start") {
            return GuardDecision {
                allow: true,
                input_type: input,
                reason: None,
                command: command_owned,
                busy: false,
            };
        }
        let listed = command.is_some_and(|value| allowed_commands.iter().any(|item| item == value));
        if listed {
            return GuardDecision {
                allow: true,
                input_type: input,
                reason: None,
                command: command_owned,
                busy: false,
            };
        }
        return GuardDecision {
            allow: false,
            input_type: input,
            reason: Some(BlockReason::CommandNotAllowed),
            command: command_owned,
            busy: false,
        };
    }

    if input_matches_expected(expected, input) {
        return GuardDecision {
            allow: true,
            input_type: input,
            reason: None,
            command: command_owned,
            busy: false,
        };
    }
    GuardDecision {
        allow: false,
        input_type: input,
        reason: Some(expected_input_block_reason(expected)),
        command: command_owned,
        busy: false,
    }
}

// --- Slice 20 remainder (GREEN): response / tool-call / compact-progress ---

///
/// Rust port of the remaining pure gates in:
/// - `src/bot/streaming/response-streamer.ts` (draft vs edit routing,
///   broken-stream flush, payload shrink deletes)
/// - `src/bot/streaming/tool-call-streamer.ts` (tool-line flush gating)
/// - `src/bot/streaming/compact-progress-streamer.ts` (hold/release,
///   parked background cards)
/// - `src/bot/streaming/stream-throttle.ts` (gap between pick sends)
///
/// No Telegram I/O lives here — only pure predicates for parity tests.

/// True when the response streamer must use draft mode
/// (TS: `MESSAGE_FORMAT_MODE === "raw"` or explicit draft; edit mode is the
/// default for markdown).
pub fn should_use_draft_response_mode(raw_format: bool, draft_requested: bool) -> bool {
    raw_format || draft_requested
}

/// True when a broken stream flushes pending output immediately instead of
/// waiting for the next edit tick (TS: `flushPending` when broken).
pub fn should_flush_broken_stream_pending(is_broken: bool, pending_count: usize) -> bool {
    is_broken && pending_count > 0
}

/// True when a tool-call line needs a send (TS: new call id or status
/// transition out of pending).
pub fn should_flush_tool_call_line(is_new_call: bool, status_changed: bool) -> bool {
    is_new_call || status_changed
}

/// True when compact progress holds its card instead of emitting
/// (TS: hold is on and no terminal counts are ready yet).
pub fn should_hold_compact_progress(hold_on: bool, has_terminal_counts: bool) -> bool {
    hold_on && !has_terminal_counts
}

/// Send gap between session-pick transcript messages
/// (TS: `SESSION_PICK_SEND_GAP_MS`, 0 under `VITEST`).
pub fn session_pick_send_gap_ms(is_test: bool, configured_ms: u64) -> u64 {
    if is_test {
        0
    } else {
        configured_ms
    }
}
