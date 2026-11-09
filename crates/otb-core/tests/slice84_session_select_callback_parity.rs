//! Slice 84 parity: session-select callback-handler pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/session-callback-handler.ts` (the select half:
//! `handleSessionSelect` `session:` dispatch guard, page-branch vs
//! id-missing gates, foreground-busy block; `shouldBlockBackgroundSessionOpen`
//! active-non-inline gate; `handleBackgroundSessionOpen` data/payload
//! dispatch plus the assistant-response-vs-other post-select map;
//! `selectSessionById` missing-project gate, stale search-state gate,
//! `isSessionOnDashboard` session-id plus message-id gate, preview-plus-chat
//! pick-load gate; `extractTextParts` text-part filter/join/trim/blank
//! gates; the `loadLatestAssistantResponse` assistant-role plus
//! non-summary plus `trim: false` plus max-`created`-with-`>=` reducer;
//! `sendSessionPickTranscript` empty-preview and pending-quote gates;
//! `sessionPickSendGapMs` vitest-vs-1000 gate).
//! `SESSION_CALLBACK_PREFIX`, `SESSION_PAGE_CALLBACK_PREFIX`,
//! `BACKGROUND_SESSION_CALLBACK_PREFIX`, `parseSessionPageCallback`,
//! `parseSessionIdCallback`, and `parseBackgroundSessionCallback` already
//! live in `command_session_gates` and are NOT re-tested here; the
//! `session:search*` dispatch and search-state gates live in
//! `session_search_callback_gates` and are NOT re-tested here; the
//! pick/quote builders live in `session_pick_catalog_gates` and arrive
//! here as bools.
//!
//! Every behavior test MUST FAIL with `unimplemented!()` panics until
//! `session_select_callback_gates` is ported (only the constant-value
//! tests pass).

use otb_core::session_select_callback_gates::*;

fn part(kind: &str, text: Option<&str>) -> SessionTextPart {
    SessionTextPart {
        part_type: kind.to_string(),
        text: text.map(str::to_string),
    }
}

fn candidate(role: &str, summary: bool, created: i64, texts: &[&str]) -> AssistantCandidate {
    AssistantCandidate {
        role: role.to_string(),
        summary,
        created,
        parts: texts.iter().map(|t| part("text", Some(t))).collect(),
    }
}

// ---- constants ----

#[test]
fn latest_response_limit_value_parity() {
    assert_eq!(LATEST_ASSISTANT_RESPONSE_MESSAGES_LIMIT, 20);
}

#[test]
fn pick_page_size_value_parity() {
    assert_eq!(SESSION_PICK_PAGE_SIZE, 20);
}

#[test]
fn pick_send_gap_value_parity() {
    assert_eq!(SESSION_PICK_SEND_GAP_MS, 1000);
}

// ---- select dispatch ----

#[test]
fn handles_select_none_parity() {
    assert!(!handles_session_select_callback(None));
}

#[test]
fn handles_select_empty_parity() {
    assert!(!handles_session_select_callback(Some("")));
}

#[test]
fn handles_select_page_parity() {
    assert!(handles_session_select_callback(Some("session:page:0")));
}

#[test]
fn handles_select_rejects_other_prefixes_parity() {
    assert!(!handles_session_select_callback(Some("project:1")));
    assert!(!handles_session_select_callback(Some("session")));
}

#[test]
fn handles_select_id_parity() {
    assert!(handles_session_select_callback(Some("session:abc123")));
}

#[test]
fn handles_select_search_prefix_still_dispatches_parity() {
    // `session:search` also starts with `session:`; search ordering lives
    // with the caller, the select dispatch itself still handles it.
    assert!(handles_session_select_callback(Some("session:search")));
}

// ---- page / id branches ----

#[test]
fn select_page_branch_parity() {
    assert!(is_session_select_page_branch(true));
    assert!(!is_session_select_page_branch(false));
}

#[test]
fn select_id_missing_parity() {
    assert!(is_session_select_id_missing(false));
    assert!(!is_session_select_id_missing(true));
}

// ---- busy block ----

#[test]
fn select_busy_blocks_parity() {
    assert!(should_block_session_select_on_busy(true));
    assert!(!should_block_session_select_on_busy(false));
}

// ---- background dispatch ----

#[test]
fn handles_background_open_parity() {
    assert!(handles_background_session_open(true, true));
}

#[test]
fn handles_background_rejects_no_data_parity() {
    assert!(!handles_background_session_open(false, true));
    assert!(!handles_background_session_open(false, false));
}

#[test]
fn handles_background_rejects_no_payload_parity() {
    assert!(!handles_background_session_open(true, false));
}

// ---- background block ----

#[test]
fn background_block_custom_parity() {
    assert!(should_block_background_session_open(true, false));
}

#[test]
fn background_allows_inline_parity() {
    assert!(!should_block_background_session_open(true, true));
}

#[test]
fn background_allows_no_interaction_parity() {
    assert!(!should_block_background_session_open(false, false));
    assert!(!should_block_background_session_open(false, true));
}

// ---- post-select action ----

#[test]
fn background_post_action_assistant_parity() {
    assert_eq!(
        background_post_select_action(true),
        BackgroundPostAction::LatestAssistantResponse
    );
}

#[test]
fn background_post_action_other_parity() {
    assert_eq!(
        background_post_select_action(false),
        BackgroundPostAction::None
    );
}

// ---- stale search ----

#[test]
fn stale_pick_when_moved_on_parity() {
    assert!(is_stale_search_pick(true, false));
}

#[test]
fn fresh_pick_when_same_parity() {
    assert!(!is_stale_search_pick(true, true));
}

#[test]
fn no_search_state_never_stale_parity() {
    assert!(!is_stale_search_pick(false, true));
    assert!(!is_stale_search_pick(false, false));
}

// ---- dashboard ----

#[test]
fn dashboard_match_with_message_parity() {
    assert!(is_session_on_dashboard(Some("s1"), Some(9), "s1"));
}

#[test]
fn dashboard_match_without_message_is_off_parity() {
    assert!(!is_session_on_dashboard(Some("s1"), None, "s1"));
}

#[test]
fn dashboard_mismatch_parity() {
    assert!(!is_session_on_dashboard(Some("s2"), Some(9), "s1"));
}

#[test]
fn dashboard_unpinned_parity() {
    assert!(!is_session_on_dashboard(None, Some(9), "s1"));
    assert!(!is_session_on_dashboard(None, None, "s1"));
}

// ---- pick load ----

#[test]
fn pick_loads_on_preview_plus_chat_parity() {
    assert!(should_load_pick_transcript(true, true));
}

#[test]
fn pick_skips_without_chat_parity() {
    assert!(!should_load_pick_transcript(true, false));
}

#[test]
fn pick_skips_without_preview_parity() {
    assert!(!should_load_pick_transcript(false, true));
    assert!(!should_load_pick_transcript(false, false));
}

// ---- extract text parts ----

#[test]
fn extract_none_when_no_text_parts_parity() {
    assert_eq!(extract_session_text_parts(&[], true), None);
    assert_eq!(
        extract_session_text_parts(&[part("image", None)], true),
        None
    );
}

#[test]
fn extract_none_when_blank_parity() {
    assert_eq!(
        extract_session_text_parts(&[part("text", Some("   "))], true),
        None
    );
    assert_eq!(
        extract_session_text_parts(&[part("text", Some("   "))], false),
        None
    );
}

#[test]
fn extract_trims_by_default_parity() {
    assert_eq!(
        extract_session_text_parts(&[part("text", Some("  hi  "))], true),
        Some("hi".to_string())
    );
}

#[test]
fn extract_keeps_padding_when_untrimmed_parity() {
    assert_eq!(
        extract_session_text_parts(&[part("text", Some("  hi  "))], false),
        Some("  hi  ".to_string())
    );
}

#[test]
fn extract_joins_text_parts_parity() {
    let parts = [part("text", Some("he")), part("text", Some("llo"))];
    assert_eq!(
        extract_session_text_parts(&parts, true),
        Some("hello".to_string())
    );
}

#[test]
fn extract_ignores_non_text_parts_parity() {
    let parts = [part("image", Some("hi")), part("text", Some("ok"))];
    assert_eq!(
        extract_session_text_parts(&parts, true),
        Some("ok".to_string())
    );
}

// ---- latest assistant reducer ----

#[test]
fn latest_none_when_empty_parity() {
    assert_eq!(pick_latest_assistant_text(&[]), None);
}

#[test]
fn latest_skips_non_assistant_parity() {
    let msgs = [candidate("user", false, 5, &["hello"])];
    assert_eq!(pick_latest_assistant_text(&msgs), None);
}

#[test]
fn latest_skips_summary_rows_parity() {
    let msgs = [candidate("assistant", true, 5, &["hello"])];
    assert_eq!(pick_latest_assistant_text(&msgs), None);
}

#[test]
fn latest_picks_newest_parity() {
    let msgs = [
        candidate("assistant", false, 5, &["old"]),
        candidate("assistant", false, 9, &["new"]),
    ];
    assert_eq!(
        pick_latest_assistant_text(&msgs),
        Some(LatestPick {
            text: "new".to_string(),
            created: 9
        })
    );
}

#[test]
fn latest_tie_later_wins_parity() {
    let msgs = [
        candidate("assistant", false, 7, &["first"]),
        candidate("assistant", false, 7, &["second"]),
    ];
    assert_eq!(
        pick_latest_assistant_text(&msgs),
        Some(LatestPick {
            text: "second".to_string(),
            created: 7
        })
    );
}

#[test]
fn latest_skips_blank_text_parity() {
    let msgs = [candidate("assistant", false, 7, &["   "])];
    assert_eq!(pick_latest_assistant_text(&msgs), None);
}

// ---- transcript gates ----

#[test]
fn empty_preview_only_without_reply_or_quote_parity() {
    assert!(should_send_empty_session_preview(false, false));
}

#[test]
fn empty_preview_skipped_with_content_parity() {
    assert!(!should_send_empty_session_preview(true, false));
    assert!(!should_send_empty_session_preview(false, true));
    assert!(!should_send_empty_session_preview(true, true));
}

#[test]
fn pending_quote_needs_reply_parity() {
    assert!(should_show_pending_prompt(true, true));
}

#[test]
fn pending_quote_skipped_otherwise_parity() {
    assert!(!should_show_pending_prompt(true, false));
    assert!(!should_show_pending_prompt(false, true));
    assert!(!should_show_pending_prompt(false, false));
}

// ---- send gap ----

#[test]
fn pick_gap_zero_under_vitest_parity() {
    assert_eq!(session_pick_send_gap_ms(true), 0);
}

#[test]
fn pick_gap_thousand_otherwise_parity() {
    assert_eq!(session_pick_send_gap_ms(false), 1000);
}
