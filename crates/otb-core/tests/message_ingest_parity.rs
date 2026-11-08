//! Parity port of document/message ingest pure logic (slice 29b, RED).
//!
//! Mirrors representative `it`s from:
//! - `tests/bot/handlers/document.test.ts` (octet-stream image detection,
//!   PDF-by-MIME routing, unsupported MIME, text prompt format, size gates)
//! - `tests/bot/handlers/media-group.test.ts` + `tests/bot/handlers/photo-handler.test.ts`
//!   (largest-photo pick, album batching/display/filenames, media-byte sums)
//! - `tests/bot/handlers/message-merger.test.ts` (merge-window threshold,
//!   text/file/photo merge join, per-chat buffering rule)
//! - `tests/bot/handlers/unsupported-message-handler.test.ts`
//!   (unsupported field filter)
//! - `tests/bot/handlers/prompt-queue-dispatch.test.ts` +
//!   `tests/bot/handlers/prompt-queue-dispatch-v2.test.ts`
//!   (queueable-prompt gates, queue-off suggestion, media size rejection)
//! - `tests/app/services/model-capabilities-service.test.ts`
//!   (`supportsInput` / `supportsAttachment` gates used by the
//!   model-no-image / model-no-pdf branches)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::message_ingest::*;

// --- document-handler.ts: resolveImageMimeType (8 its) ---

#[test]
fn slice29b_01_image_mime_passes_through() {
    assert_eq!(
        resolve_image_mime_type("image/png", "photo.png"),
        Some("image/png".to_string())
    );
}

#[test]
fn slice29b_02_octet_jpg_extension_maps_to_jpeg() {
    assert_eq!(
        resolve_image_mime_type("application/octet-stream", "photo.jpg"),
        Some("image/jpeg".to_string())
    );
}

#[test]
fn slice29b_03_octet_uppercase_jpeg_extension_maps_to_jpeg() {
    assert_eq!(
        resolve_image_mime_type("application/octet-stream", "photo.JPEG"),
        Some("image/jpeg".to_string())
    );
}

#[test]
fn slice29b_04_octet_png_extension_maps_to_png() {
    assert_eq!(
        resolve_image_mime_type("application/octet-stream", "photo.png"),
        Some("image/png".to_string())
    );
}

#[test]
fn slice29b_05_octet_webp_extension_maps_to_webp() {
    assert_eq!(
        resolve_image_mime_type("application/octet-stream", "photo.webp"),
        Some("image/webp".to_string())
    );
}

#[test]
fn slice29b_06_octet_gif_extension_maps_to_gif() {
    assert_eq!(
        resolve_image_mime_type("application/octet-stream", "photo.gif"),
        Some("image/gif".to_string())
    );
}

#[test]
fn slice29b_07_octet_without_image_extension_is_not_an_image() {
    assert_eq!(
        resolve_image_mime_type("application/octet-stream", "firmware.bin"),
        None
    );
}

#[test]
fn slice29b_08_pdf_mime_routes_by_mime_despite_image_extension() {
    assert_eq!(
        resolve_image_mime_type("application/pdf", "report.png"),
        None
    );
}

// --- document-handler.ts: DOCUMENT_MIME_TYPES (4 its) ---

#[test]
fn slice29b_09_pdf_is_a_document_mime() {
    assert!(is_document_mime_type("application/pdf"));
}

#[test]
fn slice29b_10_office_mimes_are_documents() {
    assert!(is_document_mime_type(
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ));
    assert!(is_document_mime_type(
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    ));
    assert!(is_document_mime_type(
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    ));
    assert!(is_document_mime_type("application/msword"));
}

#[test]
fn slice29b_11_opendocument_and_rtf_are_documents() {
    assert!(is_document_mime_type("application/vnd.oasis.opendocument.text"));
    assert!(is_document_mime_type("text/rtf"));
}

#[test]
fn slice29b_12_archives_images_text_and_empty_are_not_documents() {
    assert!(!is_document_mime_type("application/zip"));
    assert!(!is_document_mime_type("image/png"));
    assert!(!is_document_mime_type("text/plain"));
    assert!(!is_document_mime_type(""));
}

// --- document-handler.ts: text-file prompt format (3 its) ---

#[test]
fn slice29b_13_text_prompt_wraps_content_with_filename_markers() {
    assert_eq!(
        build_text_file_prompt("test.txt", "file content here", ""),
        "--- Content of test.txt ---\nfile content here\n--- End of file ---\n\n"
    );
}

#[test]
fn slice29b_14_text_prompt_appends_caption_after_markers() {
    let prompt = build_text_file_prompt("test.txt", "file content here", "Please review this file");
    assert!(prompt.starts_with("--- Content of test.txt ---\n"));
    assert!(prompt.contains("Please review this file"));
}

#[test]
fn slice29b_15_text_prompt_contains_extracted_document_text() {
    let prompt = build_text_file_prompt("document.pdf", "Extracted PDF content", "");
    assert!(prompt.contains("Extracted PDF content"));
    assert!(prompt.contains("document.pdf"));
}

// --- photo-handler.ts (4 its) ---

#[test]
fn slice29b_16_photo_picks_the_largest_last_entry() {
    assert_eq!(pick_largest_photo(&["small-id", "large-id"]), Some("large-id"));
}

#[test]
fn slice29b_17_photo_without_sizes_picks_nothing() {
    assert_eq!(pick_largest_photo(&[] as &[&str]), None);
}

#[test]
fn slice29b_18_photo_display_text_uses_caption() {
    assert_eq!(photo_display_text("Describe this image"), "Describe this image");
}

#[test]
fn slice29b_19_photo_display_text_falls_back_without_caption() {
    assert_eq!(photo_display_text(""), "[Photo]");
    assert_eq!(photo_display_text("   "), "[Photo]");
    assert_eq!(PHOTO_DOCUMENT_FILENAME, "photo.jpg");
}

// --- media-group-handler.ts (10 its) ---

#[test]
fn slice29b_20_media_group_batch_key_combines_chat_and_album() {
    assert_eq!(media_group_batch_key(777, "album-1"), "777:album-1");
}

#[test]
fn slice29b_21_album_display_text_falls_back_to_file_count() {
    assert_eq!(album_display_text(&[], 3), "[Album: 3 files]");
}

#[test]
fn slice29b_22_album_display_text_joins_captions() {
    assert_eq!(
        album_display_text(
            &["first".to_string(), "second".to_string()],
            2
        ),
        "first / second"
    );
}

#[test]
fn slice29b_23_album_display_text_trims_and_skips_blank_captions() {
    assert_eq!(
        album_display_text(&["  hi  ".to_string(), "   ".to_string()], 2),
        "hi"
    );
}

#[test]
fn slice29b_24_album_photo_filename_derives_from_message_id() {
    assert_eq!(album_photo_filename(42), "photo-42.jpg");
}

#[test]
fn slice29b_25_album_photos_use_jpeg_mime() {
    assert_eq!(ALBUM_PHOTO_MIME, "image/jpeg");
}

#[test]
fn slice29b_26_media_group_bytes_sum_known_sizes() {
    assert_eq!(sum_media_group_bytes(&[Some(1024), Some(2048)]), Some(3072));
}

#[test]
fn slice29b_27_media_group_bytes_unknown_size_poisons_total() {
    assert_eq!(sum_media_group_bytes(&[Some(1024), None]), None);
}

#[test]
fn slice29b_28_album_text_section_wraps_content_with_markers() {
    assert_eq!(
        build_album_text_section("notes.txt", "hello"),
        "--- Content of notes.txt ---\nhello\n--- End of file ---"
    );
}

#[test]
fn slice29b_29_album_prompt_text_joins_sections_then_captions() {
    assert_eq!(
        build_album_prompt_text(
            &["--- Content of a.txt ---\nhi\n--- End of file ---".to_string()],
            &["look".to_string()]
        ),
        "--- Content of a.txt ---\nhi\n--- End of file ---\n\nlook"
    );
}

// --- message-merger.ts (6 its) ---

#[test]
fn slice29b_30_short_message_processes_immediately_without_pending() {
    assert!(should_process_immediately(false, "hello".len(), 1500));
}

#[test]
fn slice29b_31_disabled_window_processes_immediately() {
    assert!(should_process_immediately(false, 4000, 0));
    assert!(should_process_immediately(false, 4000, -1));
}

#[test]
fn slice29b_32_long_message_buffers_when_merging_enabled() {
    assert!(!should_process_immediately(false, 4000, 1500));
}

#[test]
fn slice29b_33_short_message_buffers_behind_pending_chunk() {
    assert!(!should_process_immediately(true, "part 2".len(), 1500));
}

#[test]
fn slice29b_34_merged_prompt_joins_texts_with_blank_line() {
    let merged = merge_incoming_prompts(&[
        MergeInput { text: "a".repeat(4000), file_parts: vec![], photos: vec![] },
        MergeInput { text: "part 2".to_string(), file_parts: vec![], photos: vec![] },
    ]);
    assert_eq!(merged.text, format!("{}\n\npart 2", "a".repeat(4000)));
}

#[test]
fn slice29b_35_merged_prompt_concatenates_files_and_orders_photos() {
    let merged = merge_incoming_prompts(&[
        MergeInput {
            text: "first".to_string(),
            file_parts: vec!["f1".to_string()],
            photos: vec!["photo-1".to_string()],
        },
        MergeInput {
            text: "second".to_string(),
            file_parts: vec!["f2".to_string()],
            photos: vec!["photo-2".to_string()],
        },
    ]);
    assert_eq!(merged.file_parts, vec!["f1".to_string(), "f2".to_string()]);
    assert_eq!(merged.photos, vec!["photo-1".to_string(), "photo-2".to_string()]);
}

// --- unsupported-message-handler.ts (3 its) ---

#[test]
fn slice29b_36_unsupported_fields_detect_video_and_sticker() {
    let fields: Vec<String> =
        ["video", "message_id", "sticker"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        unsupported_fields_present(&fields),
        vec!["video".to_string(), "sticker".to_string()]
    );
}

#[test]
fn slice29b_37_supported_only_message_has_no_unsupported_fields() {
    let fields: Vec<String> =
        ["text", "photo", "caption"].iter().map(|s| s.to_string()).collect();
    assert!(unsupported_fields_present(&fields).is_empty());
    assert!(!has_unsupported_content(&fields));
}

#[test]
fn slice29b_38_unsupported_content_gate_covers_all_seven_fields() {
    assert_eq!(UNSUPPORTED_CONTENT_FIELDS.len(), 7);
    for field in ["video_note", "location", "contact", "poll", "animation"] {
        assert!(has_unsupported_content(&[field.to_string()]));
    }
    assert!(!has_unsupported_content(&[]));
}

// --- prompt-queue-dispatch.ts: isQueueablePrompt (5 its) ---

#[test]
fn slice29b_39_plain_text_is_queueable() {
    assert!(is_queueable_prompt("hello", 0, 0));
}

#[test]
fn slice29b_40_empty_message_without_attachments_is_not_queueable() {
    assert!(!is_queueable_prompt("   ", 0, 0));
}

#[test]
fn slice29b_41_commands_are_not_queueable() {
    assert!(!is_queueable_prompt("/new hello", 0, 0));
}

#[test]
fn slice29b_42_reply_keyboard_buttons_are_not_queueable() {
    assert!(!is_queueable_prompt("🛠️ Build Agent", 0, 0));
    assert!(is_queueable_prompt("Create a migration plan", 0, 0));
}

#[test]
fn slice29b_43_attachments_make_empty_text_queueable() {
    assert!(is_queueable_prompt("", 1, 0));
    assert!(is_queueable_prompt("", 0, 1));
}

// --- prompt-queue-dispatch.ts: suggestion, limit, size gate (3 its) ---

#[test]
fn slice29b_44_queue_off_suggests_for_would_be_queued_prompts() {
    assert!(should_suggest_prompt_queue(false, "hello", 0, 0));
    assert!(!should_suggest_prompt_queue(true, "hello", 0, 0));
    assert!(!should_suggest_prompt_queue(false, "/new", 0, 0));
}

#[test]
fn slice29b_45_queued_media_limit_formats_whole_megabytes() {
    assert_eq!(MAX_QUEUED_MEDIA_BYTES, 20 * 1024 * 1024);
    assert_eq!(format_queued_media_limit_mb(), "20");
}

#[test]
fn slice29b_46_queued_media_size_rejects_unknown_and_over_cap() {
    assert!(is_queued_media_size_rejected(None, 0));
    assert!(is_queued_media_size_rejected(Some(1.5), 0));
    assert!(is_queued_media_size_rejected(Some(-1.0), 0));
    assert!(is_queued_media_size_rejected(Some((20 * 1024 * 1024 + 1) as f64), 0));
    assert!(!is_queued_media_size_rejected(Some(1024.0), 0));
    assert!(!is_queued_media_size_rejected(Some((20 * 1024 * 1024) as f64), 0));
}

// --- model-capabilities-service.ts (2 its) ---

#[test]
fn slice29b_47_supports_input_checks_named_modality() {
    let caps = ModelInputSupport { text: true, audio: false, image: true, video: false, pdf: true };
    assert!(supports_input(Some(&caps), "image"));
    assert!(supports_input(Some(&caps), "pdf"));
    assert!(!supports_input(Some(&caps), "video"));
    assert!(!supports_input(None, "image"));
}

#[test]
fn slice29b_48_supports_attachment_requires_explicit_true() {
    assert!(supports_attachment(Some(true)));
    assert!(!supports_attachment(Some(false)));
    assert!(!supports_attachment(None));
}
