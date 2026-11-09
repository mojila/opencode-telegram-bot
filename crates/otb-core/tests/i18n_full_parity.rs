//! Expanded i18n parity: representative tables + formatting-logic stubs.
//!
//! Step 3 (RED, cycle 33b): mirrors the TS `tests/i18n/*` surface for the
//! expanded key set in `otb_core::i18n_full` — keyboard labels, queued
//! prompts, goal, status, errors, prompts — plus the callback-answer budget.
//!
//! Table-lookup tests (001-010) PASS against the verbatim en/ru tables.
//! Logic tests (011-048) call `unimplemented!()` formatting stubs and MUST
//! FAIL until GREEN implements interpolation behind them.
//!
//! String-unit mapping: TS `text.length` counts UTF-16 code units, so budget
//! assertions use `text.encode_utf16().count()`.

use otb_core::i18n::extract_placeholders;
use otb_core::i18n_full::{
    t_full, FULL_CALLBACK_KEYS, FULL_KEYS, GOAL_KEYS, PROMPT_KEYS, QUEUE_KEYS, STATUS_KEYS,
};
use std::collections::BTreeSet;

fn placeholder_set(template: &str) -> BTreeSet<String> {
    extract_placeholders(template).into_iter().collect()
}

fn placeholders_en_ru(key: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    (
        placeholder_set(&t_full("en", key)),
        placeholder_set(&t_full("ru", key)),
    )
}

// --- table lookup + category shape (PASS in RED) ---

#[test]
fn slice33b_001_keyboard_keys_cover_agent_variant_and_queue_rows() {
    assert!(otb_core::i18n_full::KEYBOARD_KEYS.contains(&"keyboard.agent_mode"));
    assert!(otb_core::i18n_full::KEYBOARD_KEYS.contains(&"keyboard.variant"));
    assert!(otb_core::i18n_full::KEYBOARD_KEYS.contains(&"keyboard.queued_prompt"));
    assert!(otb_core::i18n_full::KEYBOARD_KEYS.contains(&"keyboard.context"));
}

#[test]
fn slice33b_002_agent_mode_template_keeps_both_placeholders() {
    assert_eq!(t_full("en", "keyboard.agent_mode"), "{emoji} {name} Agent");
    assert_eq!(t_full("ru", "keyboard.agent_mode"), "{emoji} {name} Agent");
}

#[test]
fn slice33b_003_ru_queue_added_keeps_count_and_max() {
    let (en, ru) = placeholders_en_ru("queue.added");
    assert!(en.contains("{count}"));
    assert!(en.contains("{max}"));
    assert_eq!(en, ru);
}

#[test]
fn slice33b_004_keyboard_context_placeholders_match_en_ru() {
    let (en, ru) = placeholders_en_ru("keyboard.context");
    assert_eq!(en, placeholder_set("📊 {used} / {limit} ({percent}%)"));
    assert_eq!(en, ru);
}

#[test]
fn slice33b_005_queued_prompt_placeholders_match_en_ru() {
    let (en, ru) = placeholders_en_ru("keyboard.queued_prompt");
    assert_eq!(en, placeholder_set("❌ {index}. {text}"));
    assert_eq!(en, ru);
}

#[test]
fn slice33b_006_goal_too_long_placeholders_match_en_ru() {
    let (en, ru) = placeholders_en_ru("goal.too_long");
    assert!(en.contains("{limit}"));
    assert!(en.contains("{count}"));
    assert_eq!(en, ru);
}

#[test]
fn slice33b_007_status_uptime_placeholders_match_en_ru() {
    let (en, ru) = placeholders_en_ru("status.line.uptime_sec");
    assert_eq!(en, placeholder_set("Uptime: {seconds} sec"));
    assert_eq!(en, ru);
}

#[test]
fn slice33b_008_callback_answers_stay_within_telegram_limit() {
    assert!(!FULL_CALLBACK_KEYS.is_empty());
    for key in FULL_CALLBACK_KEYS {
        for locale in ["en", "ru"] {
            let text = t_full(locale, key);
            assert!(!text.is_empty(), "{key} ({locale}) must resolve");
            assert!(
                text.encode_utf16().count() <= otb_core::i18n::CALLBACK_ANSWER_MAX_LENGTH,
                "{key} ({locale}) exceeds Telegram callback budget",
            );
        }
    }
}

#[test]
fn slice33b_009_full_keys_cover_every_category() {
    for key in QUEUE_KEYS
        .iter()
        .chain(GOAL_KEYS)
        .chain(STATUS_KEYS)
        .chain(PROMPT_KEYS)
        .chain(FULL_CALLBACK_KEYS)
    {
        assert!(FULL_KEYS.contains(key), "{key} must be in FULL_KEYS");
        assert!(!t_full("en", key).is_empty());
        assert!(!t_full("ru", key).is_empty());
    }
}

#[test]
fn slice33b_010_unknown_locale_falls_back_to_en_and_unknown_key_echoes() {
    assert_eq!(t_full("xx", "queue.removed"), t_full("en", "queue.removed"));
    assert_eq!(t_full("en", "no.such.key"), "no.such.key");
}

// --- keyboard labels (FAIL in RED) ---

#[test]
fn slice33b_011_keyboard_agent_label_en() {
    assert_eq!(
        otb_core::i18n_full::keyboard_agent_label("en", "🤖", "builder"),
        "🤖 builder Agent",
    );
}

#[test]
fn slice33b_012_keyboard_context_label_en() {
    assert_eq!(
        otb_core::i18n_full::keyboard_context_label("en", "12k", "200k", 6),
        "📊 12k / 200k (6%)",
    );
}

#[test]
fn slice33b_013_keyboard_variant_label_en() {
    assert_eq!(
        otb_core::i18n_full::keyboard_variant_label("en", "thinking"),
        "💭 thinking",
    );
}

#[test]
fn slice33b_014_queued_prompt_label_en() {
    assert_eq!(
        otb_core::i18n_full::queued_prompt_label("en", 1, "fix tests"),
        "❌ 1. fix tests",
    );
}

// --- queued prompts (FAIL in RED) ---

#[test]
fn slice33b_015_queue_added_en() {
    assert_eq!(
        otb_core::i18n_full::queue_added("en", 2, 5),
        "📥 Added to queue (2/5). It will be sent when the current task finishes.",
    );
}

#[test]
fn slice33b_016_queue_full_en() {
    assert_eq!(
        otb_core::i18n_full::queue_full("en", 5),
        "⚠️ Queue is full (5). Remove a message or wait for the current task to finish.",
    );
}

#[test]
fn slice33b_017_queue_media_limit_en() {
    assert_eq!(
        otb_core::i18n_full::queue_media_limit("en", 25),
        "⚠️ Queued media is limited to 25 MiB. Wait for an item to send, then try again.",
    );
}

#[test]
fn slice33b_018_queue_steer_added_en() {
    assert_eq!(
        otb_core::i18n_full::queue_steer_added("en", 1, 5),
        "📥 Added to the current task (1/5).",
    );
}

// --- goal (FAIL in RED) ---

#[test]
fn slice33b_019_goal_status_line_en() {
    assert_eq!(
        otb_core::i18n_full::goal_status_line("en", "active", "ship v2"),
        "◎ Goal: active — ship v2",
    );
}

#[test]
fn slice33b_020_goal_status_label_en() {
    assert_eq!(
        otb_core::i18n_full::goal_status_label("en", "active"),
        "active"
    );
}

#[test]
fn slice33b_021_goal_confirm_start_en() {
    assert_eq!(
        otb_core::i18n_full::goal_confirm_start("en", "ship v2"),
        "Start this goal?\n\nship v2",
    );
}

#[test]
fn slice33b_022_goal_confirm_resume_en() {
    assert_eq!(
        otb_core::i18n_full::goal_confirm_resume("en", "ship v2"),
        "Resume this goal?\n\nship v2",
    );
}

#[test]
fn slice33b_023_goal_too_long_en() {
    assert_eq!(
        otb_core::i18n_full::goal_too_long("en", 500, 512),
        "A goal is limited to 500 characters (got 512).",
    );
}

#[test]
fn slice33b_024_goal_notice_paused_en() {
    assert_eq!(
        otb_core::i18n_full::goal_notice("en", false, "paused", "Paused by the user"),
        "◎ Goal paused\nPaused by the user",
    );
}

#[test]
fn slice33b_025_goal_abort_paused_en() {
    assert_eq!(
        otb_core::i18n_full::goal_abort("en", true, "Paused by the user"),
        "Goal paused: Paused by the user",
    );
}

#[test]
fn slice33b_026_goal_reason_round_limit_en() {
    assert_eq!(
        otb_core::i18n_full::goal_reason_round_limit("en", 10),
        "Reached the limit of 10 rounds",
    );
}

#[test]
fn slice33b_027_goal_rounds_en() {
    assert_eq!(otb_core::i18n_full::goal_rounds("en", 3, 10), "3/10");
}

#[test]
fn slice33b_028_goal_button_details_en() {
    assert_eq!(
        otb_core::i18n_full::goal_button_details("en", "active", "ship v2"),
        "◎ active · ship v2",
    );
}

// --- status (FAIL in RED) ---

#[test]
fn slice33b_029_status_health_line_en() {
    assert_eq!(
        otb_core::i18n_full::status_health_line("en", "Healthy"),
        "Status: Healthy",
    );
}

#[test]
fn slice33b_030_status_version_line_en() {
    assert_eq!(
        otb_core::i18n_full::status_version_line("en", "1.2.3"),
        "OpenCode version: 1.2.3",
    );
}

#[test]
fn slice33b_031_status_uptime_line_en() {
    assert_eq!(
        otb_core::i18n_full::status_uptime_line("en", 90),
        "Uptime: 90 sec",
    );
}

#[test]
fn slice33b_032_status_mode_line_en() {
    assert_eq!(
        otb_core::i18n_full::status_mode_line("en", "build"),
        "Agent: build",
    );
}

#[test]
fn slice33b_033_status_model_line_en() {
    assert_eq!(
        otb_core::i18n_full::status_model_line("en", "gpt-5"),
        "Model: gpt-5",
    );
}

#[test]
fn slice33b_034_status_project_line_en() {
    assert_eq!(
        otb_core::i18n_full::status_project_line("en", "bot"),
        "Project: bot",
    );
}

#[test]
fn slice33b_035_status_session_line_en() {
    assert_eq!(
        otb_core::i18n_full::status_session_line("en", "fix i18n"),
        "Current session: fix i18n",
    );
}

// --- errors (FAIL in RED) ---

#[test]
fn slice33b_036_error_session_en() {
    assert_eq!(
        otb_core::i18n_full::error_session("en", "boom"),
        "🔴 OpenCode returned an error: boom",
    );
}

#[test]
fn slice33b_037_error_unknown_command_en() {
    assert_eq!(
        otb_core::i18n_full::error_unknown_command("en", "/nope"),
        "⚠️ Unknown command: /nope. Use /help to see available commands.",
    );
}

#[test]
fn slice33b_038_error_photo_too_large_en() {
    assert_eq!(
        otb_core::i18n_full::error_photo_too_large("en", 20),
        "⚠️ Photo is too large (max 20MB)",
    );
}

#[test]
fn slice33b_039_error_file_too_large_en() {
    assert_eq!(
        otb_core::i18n_full::error_file_too_large("en", 20),
        "⚠️ File is too large (max 20MB)",
    );
}

#[test]
fn slice33b_040_error_project_folder_missing_en() {
    assert_eq!(
        otb_core::i18n_full::error_project_folder_missing("en", "/tmp/gone"),
        "🚫 The project folder no longer exists: /tmp/gone. Choose another project in /projects.",
    );
}

#[test]
fn slice33b_041_error_session_retry_en() {
    assert_eq!(
        otb_core::i18n_full::error_session_retry("en", "rate limited"),
        "🔁 rate limited\n\nProvider keeps returning the same error on repeated retries. Use /abort to abort.",
    );
}

// --- prompts (FAIL in RED) ---

#[test]
fn slice33b_042_prompt_rename_en() {
    assert_eq!(
        otb_core::i18n_full::prompt_rename("en", "fix i18n"),
        "📝 Enter new title for session:\n\nCurrent: fix i18n",
    );
}

#[test]
fn slice33b_043_prompt_task_schedule_preview_en() {
    assert_eq!(
        otb_core::i18n_full::prompt_task_schedule_preview(
            "en",
            "daily at 09:00",
            "Cron: 0 9 * * *\n",
            "UTC",
            "recurring",
            "tomorrow",
        ),
        "✅ Schedule parsed\n\nHow I understood it: daily at 09:00\nCron: 0 9 * * *\nTimezone: UTC\nType: recurring\nNext run: tomorrow",
    );
}

#[test]
fn slice33b_044_prompt_task_created_en() {
    assert_eq!(
        otb_core::i18n_full::prompt_task_created(
            "en",
            "ping",
            "bot",
            "build",
            "gpt-5",
            "daily",
            "",
            "tomorrow",
        ),
        "✅ Scheduled task created\n\nTask: ping\nProject: bot\nAgent: build\nModel: gpt-5\nSchedule: daily\nNext run: tomorrow",
    );
}

#[test]
fn slice33b_045_question_summary_en() {
    assert_eq!(
        otb_core::i18n_full::question_summary("en", 1, "Ship?", "Yes"),
        "Question 1:\nShip?\n\nAnswer:\nYes\n\n",
    );
}

#[test]
fn slice33b_046_permission_header_en() {
    assert_eq!(
        otb_core::i18n_full::permission_header("en", "🔐", "Bash"),
        "🔐 Permission request: Bash\n\n",
    );
}

#[test]
fn slice33b_047_progress_activity_en() {
    assert_eq!(
        otb_core::i18n_full::progress_activity("en", "⏳ Working", "tools: 3"),
        "⏳ Working\ntools: 3",
    );
}

#[test]
fn slice33b_048_context_confirm_text_en() {
    assert_eq!(
        otb_core::i18n_full::context_confirm_text("en", "fix i18n"),
        "📊 Context compaction for session \"fix i18n\"\n\nThis will reduce context usage by removing old messages from history.\n\nContinue?",
    );
}
