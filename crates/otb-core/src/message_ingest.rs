//! Document/message ingest pure logic (slice 29b, RED).
//!
//! Rust port of the pure classification/formatting helpers behind:
//! - `src/bot/handlers/document-handler.ts` (`GENERIC_MIME_TYPE`,
//!   `IMAGE_MIME_TYPES_BY_EXTENSION` lookup behind `resolveImageMimeType`,
//!   the `DOCUMENT_MIME_TYPES` office-document list, and the
//!   `--- Content of <file> --- ... --- End of file ---\n\n<caption>`
//!   text-file prompt format)
//! - `src/bot/handlers/photo-handler.ts` (largest-photo-is-last pick,
//!   `"photo.jpg"` standalone filename, `caption.trim() || "[Photo]"`
//!   display text)
//! - `src/bot/handlers/media-group-handler.ts`
//!   (`DEFAULT_MEDIA_GROUP_DEBOUNCE_MS`, `${chatId}:${mediaGroupId}`
//!   batch key, `photo-<messageId>.jpg` album filenames, `image/jpeg`
//!   album photo MIME, `captions.join(" / ") || "[Album: <n> files]"`
//!   display text, `[...textSections, ...captions].join("\n\n")` album
//!   prompt, unknown-size-poisons-total media-byte sum)
//! - `src/bot/handlers/message-merger.ts`
//!   (`TELEGRAM_SPLIT_CHUNK_MIN_LENGTH`, the
//!   `mergeWindowMs <= 0 || (!existing && short)` immediate rule,
//!   `mergeIncomingPrompts` text/file/photo join)
//! - `src/bot/handlers/unsupported-message-handler.ts`
//!   (`UNSUPPORTED_CONTENT_FIELDS` filter)
//! - `src/bot/handlers/prompt-queue-dispatch.ts` (`isQueueablePrompt`
//!   content/command/button gates, `shouldSuggestPromptQueue`,
//!   `formatQueuedMediaLimit`, the safe-integer/non-negative/cap
//!   queued-media size rejection)
//! - `src/app/services/model-capabilities-service.ts` (`supportsInput`,
//!   `supportsAttachment`)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/message_ingest_parity.rs` assert the TS
//! contract and MUST FAIL with `unimplemented!()` panics until the Rust port
//! lands (step 2).

// ---------------------------------------------------------------------------
// Document handler (`src/bot/handlers/document-handler.ts`)
// ---------------------------------------------------------------------------

/// MIME type Telegram reports when it cannot tell (TS: `GENERIC_MIME_TYPE`).
pub const GENERIC_MIME_TYPE: &str = "application/octet-stream";

/// Resolve the real image MIME for a document: `image/*` passes through,
/// `application/octet-stream` falls back to the filename extension, anything
/// else is not an image (TS: `resolveImageMimeType`).
pub fn resolve_image_mime_type(mime_type: &str, filename: &str) -> Option<String> {
    if mime_type.starts_with("image/") {
        return Some(mime_type.to_string());
    }
    if mime_type != GENERIC_MIME_TYPE {
        return None;
    }
    // TS: `path.extname(filename).slice(1).toLowerCase()`.
    let after_slash = filename.rsplit('/').next().unwrap_or(filename);
    let basename = after_slash.rsplit('\\').next().unwrap_or(after_slash);
    let ext = match basename.rfind('.') {
        Some(0) | None => "",
        Some(i) => &basename[i + 1..],
    };
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "png" => Some("image/png".to_string()),
        "webp" => Some("image/webp".to_string()),
        "gif" => Some("image/gif".to_string()),
        _ => None,
    }
}

/// Whether the MIME type is one of the office documents the bot sends
/// through the PDF pipeline (TS: `DOCUMENT_MIME_TYPES.includes`).
pub fn is_document_mime_type(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "application/pdf"
            | "application/msword"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.ms-powerpoint"
            | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            | "application/vnd.ms-excel"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.oasis.opendocument.text"
            | "application/vnd.oasis.opendocument.presentation"
            | "application/vnd.oasis.opendocument.spreadsheet"
            | "text/rtf"
    )
}

/// Wrap extracted text-file content with the filename markers and caption
/// (TS: `` `--- Content of ${filename} ---\n${text}\n--- End of file ---\n\n${caption}` ``).
pub fn build_text_file_prompt(filename: &str, content: &str, caption: &str) -> String {
    format!("--- Content of {filename} ---\n{content}\n--- End of file ---\n\n{caption}")
}

// ---------------------------------------------------------------------------
// Photo handler (`src/bot/handlers/photo-handler.ts`)
// ---------------------------------------------------------------------------

/// Standalone-photo filename offered to OpenCode.
pub const PHOTO_DOCUMENT_FILENAME: &str = "photo.jpg";

/// Display text when a photo arrives without a caption.
pub const PHOTO_FALLBACK_DISPLAY_TEXT: &str = "[Photo]";

/// Telegram sizes photos ascending, so the largest is last
/// (TS: `photos[photos.length - 1]`).
pub fn pick_largest_photo<'a>(file_ids: &[&'a str]) -> Option<&'a str> {
    file_ids.last().copied()
}

/// `caption.trim() || "[Photo]"` (TS: photo `displayText`).
pub fn photo_display_text(caption: &str) -> String {
    let trimmed = caption.trim();
    if trimmed.is_empty() {
        PHOTO_FALLBACK_DISPLAY_TEXT.to_string()
    } else {
        trimmed.to_string()
    }
}

// ---------------------------------------------------------------------------
// Media-group handler (`src/bot/handlers/media-group-handler.ts`)
// ---------------------------------------------------------------------------

/// Album debounce window (TS: `DEFAULT_MEDIA_GROUP_DEBOUNCE_MS`).
pub const DEFAULT_MEDIA_GROUP_DEBOUNCE_MS: u64 = 1_000;

/// Album items the bot renders as files carry this MIME type.
pub const ALBUM_PHOTO_MIME: &str = "image/jpeg";

/// `${chatId}:${mediaGroupId}` batch key (TS: `getBatchKey`).
pub fn media_group_batch_key(chat_id: i64, media_group_id: &str) -> String {
    format!("{chat_id}:{media_group_id}")
}

/// `captions.join(" / ") || "[Album: <n> files]"` (TS: album `displayText`).
pub fn album_display_text(captions: &[String], item_count: usize) -> String {
    let kept: Vec<&str> = captions
        .iter()
        .map(|caption| caption.trim())
        .filter(|caption| !caption.is_empty())
        .collect();
    if kept.is_empty() {
        format!("[Album: {item_count} files]")
    } else {
        kept.join(" / ")
    }
}

/// `photo-<messageId>.jpg` album filename (TS: media-group photo mapping).
pub fn album_photo_filename(message_id: i64) -> String {
    format!("photo-{message_id}.jpg")
}

/// Sum of known per-item bytes; a single unknown size poisons the total to
/// `None` (TS: the `mediaBytes` reduce over album items).
pub fn sum_media_group_bytes(sizes: &[Option<u64>]) -> Option<u64> {
    let mut total: u64 = 0;
    for size in sizes {
        match size {
            Some(bytes) => total = total.saturating_add(*bytes),
            None => return None,
        }
    }
    Some(total)
}

/// One downloaded text section of the album prompt
/// (TS: `` `--- Content of ${filename} ---\n${text}\n--- End of file ---` ``).
pub fn build_album_text_section(filename: &str, content: &str) -> String {
    format!("--- Content of {filename} ---\n{content}\n--- End of file ---")
}

/// `[...textSections, ...captions].join("\n\n")` (TS: `preparePrompt` return).
pub fn build_album_prompt_text(text_sections: &[String], captions: &[String]) -> String {
    let mut all: Vec<&str> = Vec::with_capacity(text_sections.len() + captions.len());
    all.extend(text_sections.iter().map(String::as_str));
    all.extend(captions.iter().map(String::as_str));
    all.join("\n\n")
}

// ---------------------------------------------------------------------------
// Message merger (`src/bot/handlers/message-merger.ts`)
// ---------------------------------------------------------------------------

/// Texts shorter than this are sent immediately instead of being buffered
/// (TS: `TELEGRAM_SPLIT_CHUNK_MIN_LENGTH`).
pub const TELEGRAM_SPLIT_CHUNK_MIN_LENGTH: usize = 4_000;

/// `mergeWindowMs <= 0 || (!existing && text.length < MIN)` immediate rule
/// (TS: `queuePromptForMerging` fast path).
pub fn should_process_immediately(
    has_pending: bool,
    text_len: usize,
    merge_window_ms: i64,
) -> bool {
    merge_window_ms <= 0 || (!has_pending && text_len < TELEGRAM_SPLIT_CHUNK_MIN_LENGTH)
}

/// One buffered prompt chunk (TS: `IncomingPrompt`).
#[derive(Debug, Clone, PartialEq)]
pub struct MergeInput {
    pub text: String,
    pub file_parts: Vec<String>,
    pub photos: Vec<String>,
}

/// Merged prompt: texts joined with `"\n\n"`, files and photos concatenated
/// in arrival order (TS: `mergeIncomingPrompts`).
#[derive(Debug, Clone, PartialEq)]
pub struct MergedPrompt {
    pub text: String,
    pub file_parts: Vec<String>,
    pub photos: Vec<String>,
}

pub fn merge_incoming_prompts(inputs: &[MergeInput]) -> MergedPrompt {
    MergedPrompt {
        text: inputs
            .iter()
            .map(|input| input.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n"),
        file_parts: inputs
            .iter()
            .flat_map(|input| input.file_parts.iter().cloned())
            .collect(),
        photos: inputs
            .iter()
            .flat_map(|input| input.photos.iter().cloned())
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Unsupported messages (`src/bot/handlers/unsupported-message-handler.ts`)
// ---------------------------------------------------------------------------

/// Message fields the bot has no handler for (TS: `UNSUPPORTED_CONTENT_FIELDS`).
pub const UNSUPPORTED_CONTENT_FIELDS: &[&str] = &[
    "video",
    "sticker",
    "animation",
    "video_note",
    "location",
    "contact",
    "poll",
];

/// Keep only the fields the bot cannot handle (TS: `getUnsupportedFields`).
pub fn unsupported_fields_present(message_fields: &[String]) -> Vec<String> {
    UNSUPPORTED_CONTENT_FIELDS
        .iter()
        .filter(|field| message_fields.iter().any(|m| m == *field))
        .map(|field| field.to_string())
        .collect()
}

/// Whether the message carries anything the bot cannot handle.
pub fn has_unsupported_content(message_fields: &[String]) -> bool {
    !unsupported_fields_present(message_fields).is_empty()
}

// ---------------------------------------------------------------------------
// Prompt-queue dispatch (`src/bot/handlers/prompt-queue-dispatch.ts`)
// ---------------------------------------------------------------------------

/// Bot-side queue depth cap (TS: `MAX_QUEUED_PROMPTS`).
pub const MAX_QUEUED_PROMPTS: usize = 5;

/// Bot-side queued-media byte cap (TS: `MAX_QUEUED_MEDIA_BYTES`).
pub const MAX_QUEUED_MEDIA_BYTES: u64 = 20 * 1024 * 1024;

/// Whether the text is prompt content rather than a command or a reply-keyboard
/// button press (TS: `isQueueablePrompt`).
pub fn is_queueable_prompt(text: &str, file_part_count: usize, photo_count: usize) -> bool {
    let normalized = text.trim();
    let has_content = !normalized.is_empty() || file_part_count > 0 || photo_count > 0;
    has_content
        && !normalized.starts_with('/')
        && !crate::keyboards::is_reply_keyboard_button_text(text)
}

/// True only when the queue is off and the text would otherwise have been
/// queued (TS: `shouldSuggestPromptQueue`).
pub fn should_suggest_prompt_queue(
    queue_enabled: bool,
    text: &str,
    file_part_count: usize,
    photo_count: usize,
) -> bool {
    !queue_enabled && is_queueable_prompt(text, file_part_count, photo_count)
}

/// Whole megabytes of the queued-media cap for the limit reply
/// (TS: `formatQueuedMediaLimit`).
pub fn format_queued_media_limit_mb() -> String {
    (MAX_QUEUED_MEDIA_BYTES / (1024 * 1024)).to_string()
}

/// Reject a busy queued-media candidate: unknown, non-safe-integer,
/// negative, or over-cap sizes (TS: `rejectQueuedMediaBeforePreparation`
/// size gate).
pub fn is_queued_media_size_rejected(media_bytes: Option<f64>, queued_bytes: u64) -> bool {
    /// Largest integer exactly representable as `f64` (TS: `Number.MAX_SAFE_INTEGER`).
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    let Some(media) = media_bytes else {
        return true;
    };
    if !media.is_finite() || media.fract() != 0.0 || media < 0.0 || media > MAX_SAFE_INTEGER {
        return true;
    }
    // TS: `promptQueue.canAcceptMedia`: `mediaBytes >= 0 && queued + media <= MAX`.
    let media_u64 = media as u64;
    match queued_bytes.checked_add(media_u64) {
        Some(sum) => sum > MAX_QUEUED_MEDIA_BYTES,
        None => true,
    }
}

// ---------------------------------------------------------------------------
// Model capabilities (`src/app/services/model-capabilities-service.ts`)
// ---------------------------------------------------------------------------

/// The input modalities a model lists (TS: `Model["capabilities"]["input"]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelInputSupport {
    pub text: bool,
    pub audio: bool,
    pub image: bool,
    pub video: bool,
    pub pdf: bool,
}

/// `capabilities.input[type] === true`, `false` when unknown
/// (TS: `supportsInput`).
pub fn supports_input(caps: Option<&ModelInputSupport>, input: &str) -> bool {
    match caps {
        None => false,
        Some(caps) => match input {
            "text" => caps.text,
            "audio" => caps.audio,
            "image" => caps.image,
            "video" => caps.video,
            "pdf" => caps.pdf,
            _ => false,
        },
    }
}

/// `capabilities.attachment === true`, `false` when unknown
/// (TS: `supportsAttachment`).
pub fn supports_attachment(attachment: Option<bool>) -> bool {
    attachment == Some(true)
}
