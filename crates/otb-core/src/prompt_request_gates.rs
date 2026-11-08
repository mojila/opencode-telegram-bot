//! Prompt-request pure gates (slice 78, RED).
//!
//! Rust port of the pure remainder of `src/bot/handlers/prompt.ts`:
//! `buildPromptOptions` (text-first part ordering, `/ls` attachment last,
//! `"See attached file(s)"` placeholder, agent/model/variant inclusion),
//! the `prepareTelegramPhotos` branching (no-photo passthrough, image-
//! capability gate, album-vs-standalone reply keys, strip-photos-instead-
//! of-blocking rule, download notice/error keys), the empty-prompt guard,
//! the session/project mismatch predicate, `resolveResponseMode`
//! (explicit option vs `"all"` TTS mode), the `promptResponseModes`
//! registry (`set` / `get` / `clear`), the handed-over-prompt parts check
//! and the inbox-admit early guard.
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice78_prompt_request_parity.rs`
//! assert the TS contract and MUST FAIL until the Rust port lands (step 2).
//!
//! Out of scope (impure, stays with the caller):
//! - `processUserPrompt` / `preparePromptRequest` / `admitPromptToInbox`
//!   orchestration (grammy `ctx`, OpenCode clients, settings stores).
//! - `downloadTelegramFile` / `toDataUri` bytes and `getModelCapabilities`
//!   fetching (`supportsInput` itself is covered by the capabilities
//!   service consumer; only its `false` branch outcome is decided here).
//! - `t(...)` lookups: reply keys travel as the literal `t(...)` key
//!   strings (`"bot.media_group_not_processed"`, ...), resolved by the
//!   caller.
//! - `IncomingPrompt` file/photo payloads (`FilePartInput`,
//!   `TelegramPhotoInput`) are counts / source tags here: part bodies
//!   are carried opaquely, exactly like `prompt_handover_gates`.
//!
//! Deviations: none intended; `""` counts as missing for agent /
//! provider / model / variant (TS truthiness), and `[].every(...)`
//! is `true` for the empty photo list, exactly like TS.

use std::collections::HashMap;

/// How the assistant answers a prompt (TS: `PromptResponseMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptResponseMode {
    /// Text reply only.
    TextOnly,
    /// Text reply plus TTS audio.
    TextAndTts,
}

/// TS `PromptResponseMode` wire value (`"text_only" | "text_and_tts"`).
pub fn response_mode_as_str(mode: PromptResponseMode) -> &'static str {
    match mode {
        PromptResponseMode::TextOnly => "text_only",
        PromptResponseMode::TextAndTts => "text_and_tts",
    }
}

/// Explicit per-call option wins; otherwise `"all"` TTS mode means
/// text-and-TTS (TS: `options.responseMode ?? (getTtsMode() === "all"
/// ? "text_and_tts" : "text_only")`).
pub fn resolve_response_mode(
    explicit: Option<PromptResponseMode>,
    tts_all: bool,
) -> PromptResponseMode {
    if let Some(mode) = explicit {
        return mode;
    }
    if tts_all {
        PromptResponseMode::TextAndTts
    } else {
        PromptResponseMode::TextOnly
    }
}

/// One OpenCode prompt part (TS: `TextPartInput | FilePartInput`;
/// file bodies stay opaque, only their kind and order matter here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptPart {
    /// `{ type: "text", text }`.
    Text(String),
    /// A file part (`input.fileParts` entry or `/ls` attachment).
    File,
}

/// Placeholder text when files ride without user text (TS:
/// `input.fileParts.length === 1 ? "See attached file"
/// : "See attached files"`).
pub fn attachment_placeholder_text(file_count: usize) -> &'static str {
    if file_count == 1 {
        "See attached file"
    } else {
        "See attached files"
    }
}

/// Text first, then `file_count` files, then the `/ls` attachment; files
/// without text gain the placeholder up front (TS: `buildPromptOptions`
/// parts assembly; the placeholder is keyed on `input.fileParts.length`,
/// so an attachment on its own gets none).
pub fn build_prompt_parts(
    text: &str,
    file_count: usize,
    has_attachment: bool,
) -> Vec<PromptPart> {
    let mut parts: Vec<PromptPart> = Vec::new();
    if !text.trim().is_empty() {
        parts.push(PromptPart::Text(text.to_string()));
    }
    for _ in 0..file_count {
        parts.push(PromptPart::File);
    }
    if has_attachment {
        parts.push(PromptPart::File);
    }
    // TS: `if (!parts.some((part) => part.type === "text") && input.fileParts.length > 0)`
    let has_text = parts.iter().any(|part| matches!(part, PromptPart::Text(_)));
    if !has_text && file_count > 0 {
        parts.insert(
            0,
            PromptPart::Text(attachment_placeholder_text(file_count).to_string()),
        );
    }
    parts
}

/// File parts among assembled parts (TS: `promptOptions.parts.filter(
/// (part) => part.type === "file").length`, which counts the `/ls`
/// attachment too).
pub fn count_file_parts(parts: &[PromptPart]) -> usize {
    parts
        .iter()
        .filter(|part| matches!(part, PromptPart::File))
        .count()
}

/// Agent, model and variant a prompt is sent with (TS: `PromptSelection`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptSelection {
    pub agent: Option<String>,
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

/// Model reference of an assembled prompt (TS: `promptOptions.model`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptModelRef {
    pub provider_id: String,
    pub model_id: String,
}

/// Assembled OpenCode prompt request (TS: `PromptRequestOptions`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptOptions {
    pub session_id: String,
    pub directory: String,
    pub parts: Vec<PromptPart>,
    pub agent: Option<String>,
    pub model: Option<PromptModelRef>,
    pub variant: Option<String>,
}

fn non_empty(value: &str) -> bool {
    !value.is_empty()
}

/// Full request assembly: parts plus conditional agent / model / variant
/// (TS: `buildPromptOptions`; `agent` only when set, `model` only when
/// both ids are set, `variant` only with a model).
pub fn build_prompt_options(
    session_id: &str,
    directory: &str,
    text: &str,
    file_count: usize,
    has_attachment: bool,
    selection: PromptSelection,
) -> PromptOptions {
    let parts = build_prompt_parts(text, file_count, has_attachment);
    let agent = selection
        .agent
        .filter(|agent| non_empty(agent));
    let model = if non_empty(&selection.provider_id) && non_empty(&selection.model_id) {
        Some(PromptModelRef {
            provider_id: selection.provider_id.clone(),
            model_id: selection.model_id.clone(),
        })
    } else {
        None
    };
    let variant = if model.is_some() {
        selection.variant.filter(|variant| non_empty(variant))
    } else {
        None
    };
    PromptOptions {
        session_id: session_id.to_string(),
        directory: directory.to_string(),
        parts,
        agent,
        model,
        variant,
    }
}

/// `false` when there is nothing to send: blank text, no files, no
/// photos and no pending `/ls` attachment (TS: the early `return false`
/// in `processUserPrompt`).
pub fn should_dispatch_prompt(
    text: &str,
    file_count: usize,
    photo_count: usize,
    has_pending_attachment: bool,
) -> bool {
    !text.trim().is_empty() || file_count > 0 || photo_count > 0 || has_pending_attachment
}

/// The current session belongs to another project folder (TS:
/// `currentSession.directory !== currentProject.worktree`, which resets
/// the session context).
pub fn is_session_project_mismatch(session_dir: Option<&str>, project_dir: &str) -> bool {
    match session_dir {
        Some(dir) => dir != project_dir,
        None => false,
    }
}

/// A handed-over `/detach` prompt survives only with parts left (TS:
/// `promptOptions.parts.length > 0 ? promptOptions : null`).
pub fn handover_has_parts(part_count: usize) -> bool {
    part_count > 0
}

/// An inbox prompt is refused without a current session or when the
/// project folder is gone (TS: the early `null` returns in
/// `admitPromptToInbox`).
pub fn inbox_admit_blocked(has_session: bool, folder_missing: bool) -> bool {
    !has_session || folder_missing
}

/// Every photo came standalone (TS: `input.photos.every((photo) =>
/// photo.source === "standalone")`; `true` for zero photos).
pub fn only_standalone_photos(sources: &[&str]) -> bool {
    sources.iter().all(|source| *source == "standalone")
}

/// At least one photo came from an album (TS:
/// `input.photos.some((photo) => photo.source === "album")`).
pub fn has_album_photo(sources: &[&str]) -> bool {
    sources.iter().any(|source| *source == "album")
}

/// Reply key when the model cannot read images (TS: album input reports
/// `t("bot.media_group_not_processed")`, otherwise
/// `t("bot.photo_model_no_image")`).
pub fn unsupported_photo_reply_key(has_album: bool) -> &'static str {
    if has_album {
        "bot.media_group_not_processed"
    } else {
        "bot.photo_model_no_image"
    }
}

/// Download notice once images are accepted (TS: an album, or more than
/// one photo, reports `t("bot.files_downloading")`, otherwise
/// `t("bot.photo_downloading")`).
pub fn photo_download_notice_key(all_album: bool, photo_count: usize) -> &'static str {
    if all_album || photo_count > 1 {
        "bot.files_downloading"
    } else {
        "bot.photo_downloading"
    }
}

/// Download-failure reply (TS: `t("bot.media_group_download_error")`
/// for an album, else `t("bot.photo_download_error")`).
pub fn photo_download_error_key(all_album: bool) -> &'static str {
    if all_album {
        "bot.media_group_download_error"
    } else {
        "bot.photo_download_error"
    }
}

/// Standalone photos the model cannot read are dropped when the text
/// still stands on its own (TS: `onlyStandalone &&
/// input.text.trim().length > 0` keeps `{ ...input, photos: [] }`,
/// otherwise the prompt is refused).
pub fn should_strip_unsupported_photos(only_standalone: bool, text: &str) -> bool {
    only_standalone && !text.trim().is_empty()
}

/// In-memory response-mode overrides per session (TS:
/// `promptResponseModes`: `setPromptResponseMode` /
/// `getPromptResponseMode` / `clearPromptResponseMode`, where `get`
/// returns `null` when unset).
#[derive(Debug, Default)]
pub struct ResponseModeRegistry {
    modes: HashMap<String, PromptResponseMode>,
}

impl ResponseModeRegistry {
    pub fn new() -> Self {
        Self {
            modes: HashMap::new(),
        }
    }

    pub fn set(&mut self, session_id: &str, mode: PromptResponseMode) {
        self.modes.insert(session_id.to_string(), mode);
    }

    pub fn get(&self, session_id: &str) -> Option<PromptResponseMode> {
        self.modes.get(session_id).copied()
    }

    pub fn clear(&mut self, session_id: &str) {
        self.modes.remove(session_id);
    }
}
