//! Parity port of voice / transcribe / cron / title / compact pure gates
//! (slice 34b, RED).
//!
//! Mirrors:
//! - `tests/app/services/stt-service.test.ts` (`isSttConfigured` 4 its,
//!   `getAudioFormat` extension mapping, transcriptions URL)
//! - `tests/bot/handlers/voice.test.ts` remainder (download filename
//!   `.oga` -> `.ogg` + `audio.ogg` default, blank-transcript gate,
//!   `STT_NOTE_PROMPT` gating incl. `"false"`/`"0"`/blank/whitespace)
//! - `tests/bot/commands/task.test.ts` (`truncateTaskPrompt` preview,
//!   `*/2` too-frequent rejection vs `*/5` acceptance)
//! - `tests/app/services/scheduled-task-schedule-parser-service.test.ts`
//!   remainder (fenced-JSON extract, ISO-datetime gate)
//! - `tests/app/services/scheduled-task-next-run-service.test.ts` remainder
//!   (5-field cron shape)
//! - `tests/app/formatters/session-title-formatter.test.ts` (empty en/ru,
//!   kept title, whitespace kept)
//! - `tests/app/formatters/compact-summary-formatter.test.ts` remainder
//!   (empty counts, empty gate, merge for the finished card)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::voice_cron_gates::*;

// --- isSttConfigured (4) ---

#[test]
fn slice34b_01_stt_not_configured_when_both_empty() {
    assert!(!is_stt_configured("", ""));
}

#[test]
fn slice34b_02_stt_not_configured_when_only_url() {
    assert!(!is_stt_configured("https://api.groq.com/openai/v1", ""));
}

#[test]
fn slice34b_03_stt_not_configured_when_only_key() {
    assert!(!is_stt_configured("", "sk-test-key"));
}

#[test]
fn slice34b_04_stt_configured_when_both_set() {
    assert!(is_stt_configured(
        "https://api.groq.com/openai/v1",
        "sk-test-key"
    ));
}

// --- getAudioFormat mapping (8) ---

#[test]
fn slice34b_05_audio_format_oga_is_ogg() {
    assert_eq!(audio_format_for_filename("voice.oga"), "ogg");
}

#[test]
fn slice34b_06_audio_format_ogg_stays_ogg() {
    assert_eq!(audio_format_for_filename("file_1.ogg"), "ogg");
}

#[test]
fn slice34b_07_audio_format_mp3_case_insensitive() {
    assert_eq!(audio_format_for_filename("voice.mp3"), "mp3");
    assert_eq!(audio_format_for_filename("VOICE.MP3"), "mp3");
}

#[test]
fn slice34b_08_audio_format_wav_and_m4a() {
    assert_eq!(audio_format_for_filename("clip.wav"), "wav");
    assert_eq!(audio_format_for_filename("clip.m4a"), "m4a");
}

#[test]
fn slice34b_09_audio_format_flac_aac_webm() {
    assert_eq!(audio_format_for_filename("clip.flac"), "flac");
    assert_eq!(audio_format_for_filename("clip.aac"), "aac");
    assert_eq!(audio_format_for_filename("clip.webm"), "webm");
}

#[test]
fn slice34b_10_audio_format_unknown_defaults_to_ogg() {
    assert_eq!(audio_format_for_filename("clip.xyz"), "ogg");
}

#[test]
fn slice34b_11_audio_format_no_extension_defaults_to_ogg() {
    assert_eq!(audio_format_for_filename("noextension"), "ogg");
}

#[test]
fn slice34b_12_stt_url_appends_transcriptions_path() {
    assert_eq!(
        stt_transcriptions_url("https://api.groq.com/openai/v1"),
        "https://api.groq.com/openai/v1/audio/transcriptions"
    );
}

// --- voice download filename + transcript gates (6) ---

#[test]
fn slice34b_13_oga_filename_becomes_ogg() {
    assert_eq!(
        normalize_voice_download_filename("voice/file_123.oga"),
        "file_123.ogg"
    );
}

#[test]
fn slice34b_14_filename_defaults_to_audio_ogg_when_no_basename() {
    assert_eq!(normalize_voice_download_filename(""), "audio.ogg");
    assert_eq!(normalize_voice_download_filename("voice/"), "audio.ogg");
}

#[test]
fn slice34b_15_plain_ogg_basename_passes_through() {
    assert_eq!(
        normalize_voice_download_filename("voice/file_1.ogg"),
        "file_1.ogg"
    );
}

#[test]
fn slice34b_16_blank_transcript_is_empty() {
    assert!(is_empty_transcript("   "));
    assert!(is_empty_transcript(""));
    assert!(!is_empty_transcript("run tests"));
}

#[test]
fn slice34b_17_stt_note_gate_rejects_blank_false_zero() {
    assert!(!should_include_stt_note(""));
    assert!(!should_include_stt_note("   "));
    assert!(!should_include_stt_note("false"));
    assert!(!should_include_stt_note("FALSE"));
    assert!(!should_include_stt_note("0"));
}

#[test]
fn slice34b_18_stt_note_gate_accepts_real_note() {
    assert!(should_include_stt_note("Handle with care"));
    assert_eq!(
        build_stt_note_prefix("Handle with care"),
        Some("[Note: Handle with care]".to_string())
    );
    assert_eq!(build_stt_note_prefix("false"), None);
    assert_eq!(
        voice_llm_text("Handle with care", "run tests"),
        "[Note: Handle with care]\nrun tests"
    );
    assert_eq!(voice_llm_text("", "run tests"), "run tests");
}

// --- cron shape + minute expansion (10) ---

#[test]
fn slice34b_19_cron_needs_five_fields() {
    assert!(cron_field_count_is_valid("0 17 * * *"));
    assert!(cron_field_count_is_valid("*/5 * * * *"));
}

#[test]
fn slice34b_20_cron_rejects_wrong_field_count() {
    assert!(!cron_field_count_is_valid("0 17 * *"));
    assert!(!cron_field_count_is_valid("0 17 * * * *"));
    assert!(!cron_field_count_is_valid(""));
}

#[test]
fn slice34b_21_minute_star_expands_to_sixty_values() {
    assert_eq!(expand_cron_minute_field("*").unwrap().len(), 60);
}

#[test]
fn slice34b_22_minute_single_value() {
    assert_eq!(expand_cron_minute_field("5").unwrap(), vec![5]);
}

#[test]
fn slice34b_23_minute_range_expands() {
    assert_eq!(expand_cron_minute_field("1-3").unwrap(), vec![1, 2, 3]);
}

#[test]
fn slice34b_24_minute_step_expands() {
    assert_eq!(
        expand_cron_minute_field("*/15").unwrap(),
        vec![0, 15, 30, 45]
    );
}

#[test]
fn slice34b_25_minute_list_expands_sorted() {
    assert_eq!(expand_cron_minute_field("30,10").unwrap(), vec![10, 30]);
}

#[test]
fn slice34b_26_minute_invalid_rejects() {
    assert!(expand_cron_minute_field("").is_err());
    assert!(expand_cron_minute_field("70").is_err());
    assert!(expand_cron_minute_field("5-2").is_err());
}

#[test]
fn slice34b_27_hourly_and_daily_pass_frequency() {
    assert!(validate_cron_minutes_frequency("0 * * * *").is_ok());
    assert!(validate_cron_minutes_frequency("0 17 * * *").is_ok());
}

#[test]
fn slice34b_28_every_two_minutes_is_too_frequent() {
    assert!(validate_cron_minutes_frequency("*/2 * * * *").is_err());
}

// --- schedule parser remainder (6) ---

#[test]
fn slice34b_29_every_five_minutes_passes_frequency() {
    assert!(validate_cron_minutes_frequency("*/5 * * * *").is_ok());
}

#[test]
fn slice34b_30_comma_minutes_one_apart_are_too_frequent() {
    assert!(validate_cron_minutes_frequency("1,2 * * * *").is_err());
}

#[test]
fn slice34b_31_short_cron_rejects_frequency_check() {
    assert!(validate_cron_minutes_frequency("0 17 * *").is_err());
}

#[test]
fn slice34b_32_fenced_json_extracts_inner_payload() {
    let raw = "```json\n{\"kind\":\"once\"}\n```";
    assert_eq!(
        extract_fenced_json(raw),
        Some("{\"kind\":\"once\"}".to_string())
    );
}

#[test]
fn slice34b_33_unfenced_text_has_no_fenced_json() {
    assert_eq!(extract_fenced_json("not json"), None);
    assert_eq!(extract_fenced_json("{\"kind\":\"cron\"}"), None);
}

#[test]
fn slice34b_34_iso_datetime_gate() {
    assert!(is_valid_iso_datetime("2026-03-15T10:05:00.000Z"));
    assert!(!is_valid_iso_datetime("not-a-date"));
    assert!(!is_valid_iso_datetime(""));
}

// --- task prompt preview (3) ---

#[test]
fn slice34b_35_short_prompt_passes_through_collapsed() {
    assert_eq!(truncate_task_prompt("Send me a daily summary"), "Send me a daily summary");
}

#[test]
fn slice34b_36_prompt_collapses_whitespace() {
    assert_eq!(truncate_task_prompt("a  b\nc"), "a b c");
}

#[test]
fn slice34b_37_long_prompt_truncates_to_100_with_ellipsis() {
    let long = "x".repeat(140);
    let out = truncate_task_prompt(&long);
    assert_eq!(out.len(), 100);
    assert!(out.ends_with("..."));
}

// --- session title remainder (5) ---

#[test]
fn slice34b_38_default_title_en() {
    assert_eq!(default_session_title("en"), "new session");
}

#[test]
fn slice34b_39_default_title_ru() {
    assert_eq!(default_session_title("ru"), "новая сессия");
}

#[test]
fn slice34b_40_gated_title_empty_uses_default() {
    assert_eq!(format_session_title_gated("", "en"), "new session");
    assert_eq!(format_session_title_gated("", "ru"), "новая сессия");
}

#[test]
fn slice34b_41_gated_title_keeps_set_title() {
    assert_eq!(
        format_session_title_gated("Greeting message", "en"),
        "Greeting message"
    );
    assert_eq!(format_session_title_gated("   ", "en"), "   ");
}

#[test]
fn slice34b_42_untitled_gate_only_for_empty() {
    assert!(is_untitled_session_title(""));
    assert!(!is_untitled_session_title("Greeting message"));
    assert!(!is_untitled_session_title("   "));
}

// --- compact summary remainder (6) ---

#[test]
fn slice34b_43_empty_counts_are_zero() {
    assert_eq!(
        empty_compact_counts(),
        CompactCounts {
            commands: 0,
            edits: 0,
            reads: 0,
            tools: 0
        }
    );
}

#[test]
fn slice34b_44_empty_gate_for_zero_counts() {
    assert!(is_empty_compact_counts(&CompactCounts {
        commands: 0,
        edits: 0,
        reads: 0,
        tools: 0
    }));
}

#[test]
fn slice34b_45_nonempty_counts_are_not_empty() {
    assert!(!is_empty_compact_counts(&CompactCounts {
        commands: 0,
        edits: 0,
        reads: 6,
        tools: 0
    }));
    assert!(!is_empty_compact_counts(&CompactCounts {
        commands: 3,
        edits: 1,
        reads: 4,
        tools: 2
    }));
}

#[test]
fn slice34b_46_merge_adds_elementwise() {
    let a = CompactCounts {
        commands: 1,
        edits: 2,
        reads: 0,
        tools: 1,
    };
    let b = CompactCounts {
        commands: 2,
        edits: 0,
        reads: 4,
        tools: 1,
    };
    assert_eq!(
        merge_compact_counts(&a, &b),
        CompactCounts {
            commands: 3,
            edits: 2,
            reads: 4,
            tools: 2
        }
    );
}

#[test]
fn slice34b_47_merge_with_empty_is_identity() {
    let counts = CompactCounts {
        commands: 3,
        edits: 1,
        reads: 4,
        tools: 2,
    };
    assert_eq!(merge_compact_counts(&counts, &empty_compact_counts()), counts);
}

#[test]
fn slice34b_48_merge_empty_with_empty_stays_empty() {
    let merged = merge_compact_counts(&empty_compact_counts(), &empty_compact_counts());
    assert!(is_empty_compact_counts(&merged));
    assert_eq!(
        merged,
        CompactCounts {
            commands: 0,
            edits: 0,
            reads: 0,
            tools: 0
        }
    );
}
