//! Pure voice / transcribe / cron / title / compact gates (slice 34b, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/handlers/voice-handler.ts` (voice/audio `file_id` remainder,
//!   `file_path` basename + `.oga` -> `.ogg` + `audio.ogg` default,
//!   blank-transcript gate, `STT_NOTE_PROMPT` gating)
//! - `src/app/services/stt-service.ts` (`isSttConfigured`,
//!   `getAudioFormat` extension -> format mapping, transcriptions URL)
//! - `src/app/services/scheduled-task-schedule-parser-service.ts`
//!   (`extractJsonPayload` fenced/direct/brace candidates,
//!   `isValidIsoDatetime`, schedule JSON validation)
//! - `src/bot/commands/task-command.ts` (`truncateTaskPrompt` 100-char
//!   whitespace-collapsed preview, `expandCronMinuteField` family,
//!   `validateCronMinutesFrequency` 5-minute floor)
//! - `src/app/formatters/session-title-formatter.ts`
//!   (`formatSessionTitle` empty -> dashboard name, locale default)
//! - `src/app/formatters/compact-summary-formatter.ts`
//!   (`emptyToolCallCounts`, empty check, merge for the finished card)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice34b_voice_cron_parity.rs` assert the
//! TS contract and MUST FAIL with `unimplemented!()` panics until the Rust
//! port lands (step 2).

// ---------------------------------------------------------------------------
// STT / transcribe helpers (`stt-service.ts`)
// ---------------------------------------------------------------------------

/// `Boolean(apiUrl && apiKey)` (TS: `isSttConfigured`).
pub fn is_stt_configured(api_url: &str, api_key: &str) -> bool {
    !api_url.is_empty() && !api_key.is_empty()
}

/// Lower-cased audio format for a filename, defaulting to `"ogg"`
/// (TS: `getAudioFormat`).
pub fn audio_format_for_filename(filename: &str) -> String {
    let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "oga" | "ogg" => "ogg",
        "mp3" => "mp3",
        "wav" => "wav",
        "m4a" => "m4a",
        "flac" => "flac",
        "aac" => "aac",
        "webm" => "webm",
        _ => "ogg",
    }
    .to_string()
}

/// `{apiUrl}/audio/transcriptions` (TS: `transcribeAudio` URL build).
pub fn stt_transcriptions_url(api_url: &str) -> String {
    format!("{api_url}/audio/transcriptions")
}

// ---------------------------------------------------------------------------
// Voice download / transcript gates (`voice-handler.ts`)
// ---------------------------------------------------------------------------

/// Basename of a Telegram `file_path`, defaulting to `audio.ogg`, with a
/// trailing `.oga` renamed to `.ogg` (TS: `downloadTelegramFile`).
pub fn normalize_voice_download_filename(file_path: &str) -> String {
    let basename = file_path.split('/').next_back().unwrap_or("");
    let mut filename = if basename.is_empty() {
        "audio.ogg".to_string()
    } else {
        basename.to_string()
    };
    if filename.ends_with(".oga") {
        filename.truncate(filename.len() - 4);
        filename.push_str(".ogg");
    }
    filename
}

/// Whether the trimmed transcript is empty (TS: `recognizedText` empty gate
/// answering `stt.empty_result`).
pub fn is_empty_transcript(text: &str) -> bool {
    text.trim().is_empty()
}

/// Whether `STT_NOTE_PROMPT` survives the blank / `"false"` (any case) /
/// `"0"` gate (TS: `notePrompt` gate in `handleVoiceMessage`).
pub fn should_include_stt_note(note_prompt: &str) -> bool {
    let trimmed = note_prompt.trim();
    !trimmed.is_empty() && trimmed.to_lowercase() != "false" && trimmed != "0"
}

/// `[Note: ...]` prefix, or `None` when the note is gated out
/// (TS: `llmNote` build).
pub fn build_stt_note_prefix(note_prompt: &str) -> Option<String> {
    if should_include_stt_note(note_prompt) {
        Some(format!("[Note: {}]", note_prompt.trim()))
    } else {
        None
    }
}

/// LLM prompt text: note prefix plus transcript (TS: `textForLLM`).
pub fn voice_llm_text(note_prompt: &str, recognized_text: &str) -> String {
    match build_stt_note_prefix(note_prompt) {
        Some(prefix) => format!("{prefix}\n{recognized_text}"),
        None => recognized_text.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Cron parse / validation (`task-command.ts`, `scheduled-task-next-run-service.ts`)
// ---------------------------------------------------------------------------

/// Whether a cron expression has exactly 5 whitespace-separated fields
/// (TS: `cron.trim().split(/\s+/)` length check).
pub fn cron_field_count_is_valid(cron: &str) -> bool {
    cron.split_whitespace().count() == 5
}

fn parse_cron_minute_number(value: &str) -> Result<u32, String> {
    match value.parse::<i64>() {
        Ok(n) if n >= 0 && n <= 59 => Ok(n as u32),
        _ => Err("Invalid cron minute value returned by parser".to_string()),
    }
}

fn expand_cron_minute_base(base: &str) -> Result<Vec<u32>, String> {
    if base == "*" {
        return Ok((0..60).collect());
    }
    if base.contains('-') {
        // TS destructures `[rawStart, rawEnd]` and ignores extra `-` segments.
        let mut iter = base.split('-');
        let raw_start = iter.next().unwrap_or("");
        let raw_end = iter.next().unwrap_or("");
        let start = parse_cron_minute_number(raw_start)?;
        let end = parse_cron_minute_number(raw_end)?;
        if start > end {
            return Err("Invalid cron minute range returned by parser".to_string());
        }
        return Ok((start..=end).collect());
    }
    Ok(vec![parse_cron_minute_number(base)?])
}

fn expand_cron_minute_token(token: &str) -> Result<Vec<u32>, String> {
    let mut parts = token.split('/');
    let raw_base = parts
        .next()
        .ok_or_else(|| "Invalid cron minute field returned by parser".to_string())?;
    let raw_step = parts.next();
    // TS destructures `[rawBase, rawStep]` and ignores any extra `/` segments.
    let base_values = expand_cron_minute_base(raw_base)?;
    match raw_step {
        None => Ok(base_values),
        Some(raw) => {
            let step: usize = match raw.parse() {
                Ok(n) if n > 0 => n,
                _ => {
                    return Err("Invalid cron minute step returned by parser".to_string());
                }
            };
            Ok(base_values
                .into_iter()
                .enumerate()
                .filter(|(index, _)| index % step == 0)
                .map(|(_, value)| value)
                .collect())
        }
    }
}

/// Expanded sorted minute values for one cron minute field
/// (TS: `expandCronMinuteField`).
pub fn expand_cron_minute_field(field: &str) -> Result<Vec<u32>, String> {
    let mut values: Vec<u32> = Vec::new();
    for token in field.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err("Invalid cron minute field returned by parser".to_string());
        }
        for value in expand_cron_minute_token(trimmed)? {
            if !values.contains(&value) {
                values.push(value);
            }
        }
    }
    if values.is_empty() {
        return Err("Invalid cron minute field returned by parser".to_string());
    }
    values.sort_unstable();
    Ok(values)
}

/// Rejects schedules more frequent than every 5 minutes
/// (TS: `validateCronMinutesFrequency`).
pub fn validate_cron_minutes_frequency(cron: &str) -> Result<(), String> {
    let parts: Vec<&str> = cron.split_whitespace().collect();
    if parts.len() < 5 {
        return Err("Invalid cron expression returned by parser".to_string());
    }
    let minute_field = parts[0];
    if minute_field.is_empty() {
        return Err("Invalid cron expression returned by parser".to_string());
    }
    let minute_values = expand_cron_minute_field(minute_field).map_err(|e| e.to_string())?;
    if minute_values.len() <= 1 {
        return Ok(());
    }
    let mut min_gap: i64 = 60;
    for (index, &current) in minute_values.iter().enumerate() {
        let wrap_around = index == minute_values.len() - 1;
        let next_raw = if wrap_around {
            minute_values[0]
        } else {
            minute_values[index + 1]
        };
        let next = if wrap_around {
            next_raw as i64 + 60
        } else {
            next_raw as i64
        };
        min_gap = min_gap.min(next - current as i64);
    }
    if min_gap < 5 {
        return Err("task.schedule_too_frequent".to_string());
    }
    Ok(())
}

/// The `code-fence` JSON candidate of a schedule-parser response, when the
/// raw text wraps JSON in ``` / ```json fences
/// (TS: `extractJsonPayload` fenced candidate).
pub fn extract_fenced_json(raw_text: &str) -> Option<String> {
    let trimmed = raw_text.trim();
    let open = trimmed.find("```")?;
    let mut rest = &trimmed[open + 3..];
    if rest
        .get(..4)
        .map(|tag| tag.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        rest = &rest[4..];
    }
    let close = rest.find("```")?;
    let inner = rest[..close].trim();
    if inner.is_empty() {
        return None;
    }
    Some(inner.to_string())
}

fn is_valid_iso_date_part(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        if index == 4 || index == 7 {
            continue;
        }
        if !byte.is_ascii_digit() {
            return false;
        }
    }
    let month: u32 = date[5..7].parse().unwrap_or(99);
    let day: u32 = date[8..10].parse().unwrap_or(99);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn is_valid_iso_time_part(time: &str) -> bool {
    // HH:MM[:SS[.frac]][Z|±HH:MM|±HHMM|±HH]
    let bytes = time.as_bytes();
    if bytes.len() < 5
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || bytes[2] != b':'
        || !bytes[3].is_ascii_digit()
        || !bytes[4].is_ascii_digit()
    {
        return false;
    }
    let hour: u32 = time[0..2].parse().unwrap_or(99);
    let minute: u32 = time[3..5].parse().unwrap_or(99);
    if hour > 23 || minute > 59 {
        return false;
    }
    let mut rest = &time[5..];
    if let Some(after_colon) = rest.strip_prefix(':') {
        let after_bytes = after_colon.as_bytes();
        if after_bytes.len() < 2
            || !after_bytes[0].is_ascii_digit()
            || !after_bytes[1].is_ascii_digit()
        {
            return false;
        }
        if after_colon[0..2].parse::<u32>().unwrap_or(99) > 59 {
            return false;
        }
        rest = &after_colon[2..];
    }
    if let Some(after_dot) = rest.strip_prefix('.') {
        let frac_len = after_dot.bytes().take_while(|b| b.is_ascii_digit()).count();
        if frac_len == 0 {
            return false;
        }
        rest = &after_dot[frac_len..];
    }
    if rest.is_empty() {
        // `Date.parse` accepts a bare time without zone; keep strict-ish
        // but accept it so plain ISO local datetimes pass.
        return true;
    }
    if rest.eq_ignore_ascii_case("z") {
        return true;
    }
    let (sign, offset) = match rest.strip_prefix(|c| c == '+' || c == '-') {
        Some(o) => (true, o),
        None => (false, rest),
    };
    if !sign {
        return false;
    }
    // Accept HH:MM, HHMM, or HH.
    let digits: String = offset.chars().filter(|c| *c != ':').collect();
    if digits.len() != 2 && digits.len() != 4 {
        return false;
    }
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let oh: u32 = digits[0..2].parse().unwrap_or(99);
    let om: u32 = if digits.len() == 4 {
        digits[2..4].parse().unwrap_or(99)
    } else {
        0
    };
    oh <= 23 && om <= 59
}

/// Whether `value` parses as a date (TS: `isValidIsoDatetime`).
pub fn is_valid_iso_datetime(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let sep = trimmed
        .find(|c| c == 'T' || c == 't')
        .or_else(|| trimmed.find(' '));
    let sep = match sep {
        Some(i) => i,
        None => return false,
    };
    let (date_part, time_part) = (&trimmed[..sep], &trimmed[sep + 1..]);
    if time_part.is_empty() {
        return false;
    }
    is_valid_iso_date_part(date_part) && is_valid_iso_time_part(time_part)
}

/// Whitespace-collapsed 100-char prompt preview with `...` cut
/// (TS: `truncateTaskPrompt`, `TASK_PROMPT_PREVIEW_LENGTH = 100`).
pub fn truncate_task_prompt(prompt: &str) -> String {
    const PREVIEW_LENGTH: usize = 100;
    let normalized: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= PREVIEW_LENGTH {
        return normalized;
    }
    let head: String = normalized.chars().take(PREVIEW_LENGTH - 3).collect();
    format!("{head}...")
}

// ---------------------------------------------------------------------------
// Session title (`session-title-formatter.ts`)
// ---------------------------------------------------------------------------

/// Dashboard name for an untitled session, by locale
/// (TS: `t("pinned.default_session_title")`).
pub fn default_session_title(locale: &str) -> String {
    if locale == "ru" {
        "новая сессия".to_string()
    } else {
        "new session".to_string()
    }
}

/// `title || default` (TS: `formatSessionTitle`).
pub fn format_session_title_gated(title: &str, locale: &str) -> String {
    if title.is_empty() {
        default_session_title(locale)
    } else {
        title.to_string()
    }
}

/// Whether the title still needs the dashboard placeholder.
pub fn is_untitled_session_title(title: &str) -> bool {
    title.is_empty()
}

// ---------------------------------------------------------------------------
// Compact summary remainder (`compact-summary-formatter.ts`)
// ---------------------------------------------------------------------------

/// What one finished call adds to a compact card, by low-verbosity category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactCounts {
    pub commands: u32,
    pub edits: u32,
    pub reads: u32,
    pub tools: u32,
}

/// All-zero counts (TS: `emptyToolCallCounts`).
pub fn empty_compact_counts() -> CompactCounts {
    CompactCounts {
        commands: 0,
        edits: 0,
        reads: 0,
        tools: 0,
    }
}

/// Whether every count is zero (TS: the `""` empty-summary gate in
/// `formatCompactSummary`).
pub fn is_empty_compact_counts(counts: &CompactCounts) -> bool {
    counts.commands == 0 && counts.edits == 0 && counts.reads == 0 && counts.tools == 0
}

/// Element-wise sum of two count snapshots.
pub fn merge_compact_counts(a: &CompactCounts, b: &CompactCounts) -> CompactCounts {
    CompactCounts {
        commands: a.commands + b.commands,
        edits: a.edits + b.edits,
        reads: a.reads + b.reads,
        tools: a.tools + b.tools,
    }
}
