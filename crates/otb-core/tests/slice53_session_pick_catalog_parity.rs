//! Parity port of the session-pick / external-input / catalog pure gates
//! (slice 53, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/app/services/session-pick-reply.ts` (`findEligibleReply`,
//!   `findLatestUserPrompt`, `findMessageById`, `findPendingUserPrompt`,
//!   `buildSessionPickQuote`)
//! - `src/app/services/external-user-input-service.ts`
//!   (`truncateExternalUserInputText`, `buildExternalUserInputNotification`)
//! - `src/app/services/command-catalog-service.ts` (directory fold, command
//!   filter without `"goal"`)
//! - `src/app/services/skills-catalog-service.ts` (directory fold, skill
//!   filter)
//! - `src/app/services/mcp-catalog-service.ts` (`MCP_STATUS_NAMES`,
//!   failed-state error normalization, `parseMcpCatalogServers`)
//! - `src/app/services/recent-sessions-service.ts` (question > permission >
//!   running > idle status precedence)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::session_pick_catalog_gates::*;

// --- helpers ---

fn text_part(text: &str) -> SessionPickPart {
    SessionPickPart {
        kind: "text".to_string(),
        text: Some(text.to_string()),
        filename: None,
    }
}

fn image_part() -> SessionPickPart {
    SessionPickPart {
        kind: "image".to_string(),
        text: None,
        filename: None,
    }
}

fn file_part(name: Option<&str>) -> SessionPickPart {
    SessionPickPart {
        kind: "file".to_string(),
        text: None,
        filename: name.map(str::to_string),
    }
}

fn msg(
    id: Option<&str>,
    role: &str,
    summary: bool,
    has_error: bool,
    parent_id: Option<&str>,
    created: Option<i64>,
    completed: Option<i64>,
    parts: Vec<SessionPickPart>,
) -> SessionPickMessage {
    SessionPickMessage {
        info: SessionPickInfo {
            id: id.map(str::to_string),
            role: Some(role.to_string()),
            summary,
            parent_id: parent_id.map(str::to_string),
            has_error,
            created,
            completed,
        },
        parts,
    }
}

fn assistant(text: &str, created: i64) -> SessionPickMessage {
    msg(
        Some("a"),
        "assistant",
        false,
        false,
        Some("p"),
        Some(created),
        Some(created),
        vec![text_part(text)],
    )
}

fn user_msg(text: &str, created: i64) -> SessionPickMessage {
    msg(
        Some("u"),
        "user",
        false,
        false,
        None,
        Some(created),
        None,
        vec![text_part(text)],
    )
}

fn raw_item(name: &str, description: Option<&str>, source: &str) -> RawCatalogItem {
    RawCatalogItem {
        name: name.to_string(),
        description: description.map(str::to_string),
        source: source.to_string(),
    }
}

fn status_record(status: &str, error: Option<CatalogJson>) -> CatalogJson {
    let mut fields = vec![("status".to_string(), CatalogJson::Str(status.to_string()))];
    if let Some(err) = error {
        fields.push(("error".to_string(), err));
    }
    CatalogJson::Obj(fields)
}

// --- eligible reply (01-09) ---

#[test]
fn slice53_01_eligible_picks_newest_non_summary_assistant_text() {
    let messages = vec![
        assistant("old", 5),
        msg(
            Some("s"),
            "assistant",
            true,
            false,
            None,
            Some(9),
            Some(9),
            vec![text_part("summary")],
        ),
        assistant("new", 7),
    ];
    assert_eq!(
        find_eligible_reply(&messages, false),
        Some(EligibleReply {
            text: "new".to_string(),
            parent_id: Some("p".to_string()),
            created: 7,
        })
    );
}

#[test]
fn slice53_02_eligible_skips_errored_messages() {
    let mut bad = assistant("bad", 9);
    bad.info.has_error = true;
    let messages = vec![bad, assistant("good", 5)];
    assert_eq!(
        find_eligible_reply(&messages, false),
        Some(EligibleReply {
            text: "good".to_string(),
            parent_id: Some("p".to_string()),
            created: 5,
        })
    );
}

#[test]
fn slice53_03_eligible_busy_skips_incomplete_idle_accepts() {
    let mut running = assistant("live", 9);
    running.info.completed = None;
    let idle = vec![running.clone(), assistant("done", 5)];
    assert_eq!(
        find_eligible_reply(&idle, true),
        Some(EligibleReply {
            text: "done".to_string(),
            parent_id: Some("p".to_string()),
            created: 5,
        })
    );
    assert_eq!(
        find_eligible_reply(&[running], false),
        Some(EligibleReply {
            text: "live".to_string(),
            parent_id: Some("p".to_string()),
            created: 9,
        })
    );
}

#[test]
fn slice53_04_eligible_skips_empty_text() {
    let messages = vec![
        assistant("   ", 9),
        msg(
            Some("a"),
            "assistant",
            false,
            false,
            None,
            Some(8),
            Some(8),
            vec![image_part()],
        ),
        assistant("real", 5),
    ];
    assert_eq!(find_eligible_reply(&messages, false).map(|pick| pick.text), Some("real".to_string()));
}

#[test]
fn slice53_05_eligible_tie_prefers_later_in_list() {
    let messages = vec![assistant("first", 7), assistant("second", 7)];
    assert_eq!(
        find_eligible_reply(&messages, false).map(|pick| pick.text),
        Some("second".to_string())
    );
}

#[test]
fn slice53_06_eligible_missing_created_defaults_to_zero() {
    let mut undated = assistant("undated", 0);
    undated.info.created = None;
    let messages = vec![undated, assistant("dated", 0)];
    assert_eq!(
        find_eligible_reply(&messages, false).map(|pick| pick.text),
        Some("dated".to_string())
    );
}

#[test]
fn slice53_07_eligible_joins_text_parts_ignores_other_parts() {
    let messages = vec![msg(
        Some("a"),
        "assistant",
        false,
        false,
        None,
        Some(3),
        Some(3),
        vec![text_part("Hello "), image_part(), text_part("world")],
    )];
    assert_eq!(
        find_eligible_reply(&messages, false).map(|pick| pick.text),
        Some("Hello world".to_string())
    );
}

#[test]
fn slice53_08_eligible_parent_id_missing_is_none() {
    let mut orphan = assistant("hi", 3);
    orphan.info.parent_id = None;
    assert_eq!(
        find_eligible_reply(&[orphan], false),
        Some(EligibleReply {
            text: "hi".to_string(),
            parent_id: None,
            created: 3,
        })
    );
}

#[test]
fn slice53_09_eligible_none_without_usable_assistant() {
    let messages = vec![user_msg("hi", 3)];
    assert_eq!(find_eligible_reply(&messages, false), None);
    assert_eq!(find_eligible_reply(&[], true), None);
}

// --- latest user prompt (10-14) ---

#[test]
fn slice53_10_latest_user_picks_newest_user_message() {
    let messages = vec![user_msg("old", 2), assistant("reply", 5), user_msg("new", 9)];
    assert_eq!(
        find_latest_user_prompt(&messages, "attachment").map(|found| found.info.created),
        Some(Some(9))
    );
}

#[test]
fn slice53_11_latest_user_ignores_assistant_messages() {
    let messages = vec![assistant("reply", 5)];
    assert_eq!(find_latest_user_prompt(&messages, "attachment"), None);
}

#[test]
fn slice53_12_latest_user_file_only_message_counts() {
    let messages = vec![
        user_msg("text", 4),
        msg(
            Some("f"),
            "user",
            false,
            false,
            None,
            Some(6),
            None,
            vec![file_part(Some("plan.pdf"))],
        ),
    ];
    let latest = find_latest_user_prompt(&messages, "attachment").expect("file message counts");
    assert_eq!(latest.info.created, Some(6));
}

#[test]
fn slice53_13_latest_user_skips_empty_text_without_files() {
    let messages = vec![
        user_msg("   ", 6),
        msg(
            Some("u"),
            "user",
            false,
            false,
            None,
            Some(4),
            None,
            vec![image_part()],
        ),
        user_msg("real", 2),
    ];
    let latest = find_latest_user_prompt(&messages, "attachment").expect("falls back to text");
    assert_eq!(latest.info.created, Some(2));
}

#[test]
fn slice53_14_latest_user_tie_prefers_later_in_list() {
    let first = user_msg("first", 7);
    let second = user_msg("second", 7);
    let messages = vec![first, second];
    let latest = find_latest_user_prompt(&messages, "attachment").expect("tie resolves");
    assert_eq!(latest.parts, vec![text_part("second")]);
}

// --- find by id + pending (15-19) ---

#[test]
fn slice53_15_find_by_id_returns_matching_message() {
    let messages = vec![user_msg("hi", 3), assistant("yo", 4)];
    let found = find_message_by_id(&messages, "u").expect("id matches");
    assert_eq!(found.info.role.as_deref(), Some("user"));
}

#[test]
fn slice53_16_find_by_id_missing_is_none() {
    let messages = vec![user_msg("hi", 3)];
    assert_eq!(find_message_by_id(&messages, "gone"), None);
}

#[test]
fn slice53_17_pending_returns_user_prompt_newer_than_reply() {
    let messages = vec![assistant("reply", 5), user_msg("follow-up", 8)];
    let pending = find_pending_user_prompt(&messages, 5, "attachment").expect("newer prompt");
    assert_eq!(pending.info.created, Some(8));
}

#[test]
fn slice53_18_pending_none_when_latest_not_newer() {
    let messages = vec![user_msg("before", 4), assistant("reply", 5)];
    assert_eq!(find_pending_user_prompt(&messages, 5, "attachment"), None);
    assert_eq!(find_pending_user_prompt(&messages, 9, "attachment"), None);
}

#[test]
fn slice53_19_pending_none_without_user_prompt() {
    let messages = vec![assistant("reply", 5)];
    assert_eq!(find_pending_user_prompt(&messages, 1, "attachment"), None);
}

// --- session pick quote (20-23) ---

#[test]
fn slice53_20_quote_uses_text_with_person_title() {
    let quote = build_session_pick_quote(&user_msg("Review README", 3), "Last input", "attachment")
        .expect("text quotes");
    assert_eq!(quote.title, "👤 Last input");
    assert_eq!(quote.body, "Review README");
}

#[test]
fn slice53_21_quote_file_only_uses_clip_filenames() {
    let message = msg(
        Some("u"),
        "user",
        false,
        false,
        None,
        Some(3),
        None,
        vec![file_part(Some("plan.pdf"))],
    );
    let quote =
        build_session_pick_quote(&message, "Last input", "attachment").expect("file lines quote");
    assert_eq!(quote.body, "📎 plan.pdf");
}

#[test]
fn slice53_22_quote_missing_filename_falls_back_to_label() {
    let message = msg(
        Some("u"),
        "user",
        false,
        false,
        None,
        Some(3),
        None,
        vec![file_part(None)],
    );
    let quote =
        build_session_pick_quote(&message, "Last input", "file").expect("label fallback quotes");
    assert_eq!(quote.body, "📎 file");
}

#[test]
fn slice53_23_quote_empty_text_and_no_files_is_none() {
    assert_eq!(build_session_pick_quote(&user_msg("   ", 3), "Last input", "attachment"), None);
    let image_only = msg(
        Some("u"),
        "user",
        false,
        false,
        None,
        Some(3),
        None,
        vec![image_part()],
    );
    assert_eq!(build_session_pick_quote(&image_only, "Last input", "attachment"), None);
}

// --- external user input (24-28) ---

#[test]
fn slice53_24_truncate_short_text_unchanged() {
    assert_eq!(truncate_external_user_input_text("hello"), "hello");
    assert_eq!(
        truncate_external_user_input_text(&"x".repeat(EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH)),
        "x".repeat(EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH)
    );
}

#[test]
fn slice53_25_truncate_over_budget_appends_ellipsis() {
    let long = "y".repeat(EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH + 1);
    let truncated = truncate_external_user_input_text(&long);
    assert_eq!(truncated.len(), EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH);
    assert!(truncated.ends_with("..."));
}

#[test]
fn slice53_26_notification_normalizes_crlf_and_trims() {
    let notice =
        build_external_user_input_notification("Review\r\nREADME  ", "External input").expect("notice");
    assert_eq!(notice.title, "👤 External input");
    assert_eq!(notice.body, "Review\nREADME");
}

#[test]
fn slice53_27_notification_blank_is_none() {
    assert_eq!(build_external_user_input_notification("   \r\n  ", "External input"), None);
}

#[test]
fn slice53_28_notification_body_truncated_at_budget() {
    let long = "z".repeat(EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH + 10);
    let notice =
        build_external_user_input_notification(&long, "External input").expect("long notice");
    assert_eq!(notice.body.len(), EXTERNAL_USER_INPUT_MAX_DISPLAY_LENGTH);
    assert!(notice.body.ends_with("..."));
}

// --- catalog directories (29-30) ---

#[test]
fn slice53_29_catalog_directory_folds_backslashes() {
    assert_eq!(normalize_catalog_api_directory("C:\\proj\\app"), "C:/proj/app");
}

#[test]
fn slice53_30_catalog_directory_posix_unchanged() {
    assert_eq!(normalize_catalog_api_directory("/srv/app"), "/srv/app");
}

// --- command catalog filter (31-35) ---

#[test]
fn slice53_31_command_filter_keeps_command_source() {
    let items = vec![
        raw_item("deploy", Some("Ship it"), "command"),
        raw_item("lint", None, "command"),
    ];
    assert_eq!(
        filter_command_catalog_items(&items),
        vec![
            CatalogEntry {
                name: "deploy".to_string(),
                description: Some("Ship it".to_string()),
            },
            CatalogEntry {
                name: "lint".to_string(),
                description: None,
            },
        ]
    );
}

#[test]
fn slice53_32_command_filter_drops_other_sources() {
    let items = vec![
        raw_item("deploy", None, "skill"),
        raw_item("mcp-server", None, "mcp"),
        raw_item("local", None, "other"),
    ];
    assert!(filter_command_catalog_items(&items).is_empty());
}

#[test]
fn slice53_33_command_filter_drops_blank_names() {
    let items = vec![
        raw_item("   ", None, "command"),
        raw_item("", None, "command"),
        raw_item("ok", None, "command"),
    ];
    assert_eq!(
        filter_command_catalog_items(&items),
        vec![CatalogEntry {
            name: "ok".to_string(),
            description: None,
        }]
    );
}

#[test]
fn slice53_34_command_filter_drops_goal_case_insensitive_trimmed() {
    let items = vec![
        raw_item("goal", None, "command"),
        raw_item(" Goal ", None, "command"),
        raw_item("GOAL", None, "command"),
        raw_item("goals", None, "command"),
    ];
    assert_eq!(
        filter_command_catalog_items(&items),
        vec![CatalogEntry {
            name: "goals".to_string(),
            description: None,
        }]
    );
}

#[test]
fn slice53_35_command_filter_preserves_original_name() {
    let items = vec![raw_item(" Deploy ", Some("Ship"), "command")];
    assert_eq!(
        filter_command_catalog_items(&items),
        vec![CatalogEntry {
            name: " Deploy ".to_string(),
            description: Some("Ship".to_string()),
        }]
    );
}

// --- skills catalog filter (36-37) ---

#[test]
fn slice53_36_skills_filter_keeps_skill_source() {
    let items = vec![raw_item("review", Some("Review code"), "skill")];
    assert_eq!(
        filter_skills_catalog_items(&items),
        vec![CatalogEntry {
            name: "review".to_string(),
            description: Some("Review code".to_string()),
        }]
    );
}

#[test]
fn slice53_37_skills_filter_drops_command_source_and_blanks() {
    let items = vec![
        raw_item("deploy", None, "command"),
        raw_item("  ", None, "skill"),
        raw_item("goal", None, "skill"),
    ];
    // Unlike commands, "goal" is a valid skill name and stays.
    assert_eq!(
        filter_skills_catalog_items(&items),
        vec![CatalogEntry {
            name: "goal".to_string(),
            description: None,
        }]
    );
}

// --- MCP catalog (38-44) ---

#[test]
fn slice53_38_mcp_all_status_names_recognized_unknown_rejected() {
    assert_eq!(mcp_status_name_of("connected"), Some(McpStatusName::Connected));
    assert_eq!(mcp_status_name_of("disabled"), Some(McpStatusName::Disabled));
    assert_eq!(mcp_status_name_of("failed"), Some(McpStatusName::Failed));
    assert_eq!(mcp_status_name_of("needs_auth"), Some(McpStatusName::NeedsAuth));
    assert_eq!(
        mcp_status_name_of("needs_client_registration"),
        Some(McpStatusName::NeedsClientRegistration)
    );
    assert_eq!(mcp_status_name_of("offline"), None);
    assert_eq!(MCP_STATUS_NAMES.len(), 5);
}

#[test]
fn slice53_39_mcp_failed_without_error_normalizes_to_empty() {
    let servers = parse_mcp_catalog_servers(&CatalogJson::Obj(vec![(
        "gh".to_string(),
        status_record("failed", None),
    )]))
    .expect("record parses");
    assert_eq!(
        servers,
        vec![McpCatalogServerItem {
            name: "gh".to_string(),
            status: McpServerStatus {
                name: McpStatusName::Failed,
                error: Some(String::new()),
            },
        }]
    );
}

#[test]
fn slice53_40_mcp_failed_keeps_string_error_connected_carries_none() {
    let value = CatalogJson::Obj(vec![
        (
            "gh".to_string(),
            status_record("failed", Some(CatalogJson::Str("boom".to_string()))),
        ),
        ("fs".to_string(), status_record("connected", None)),
    ]);
    let servers = parse_mcp_catalog_servers(&value).expect("record parses");
    assert_eq!(
        servers,
        vec![
            McpCatalogServerItem {
                name: "gh".to_string(),
                status: McpServerStatus {
                    name: McpStatusName::Failed,
                    error: Some("boom".to_string()),
                },
            },
            McpCatalogServerItem {
                name: "fs".to_string(),
                status: McpServerStatus {
                    name: McpStatusName::Connected,
                    error: None,
                },
            },
        ]
    );
}

#[test]
fn slice53_41_mcp_array_form_parses_servers() {
    let value = CatalogJson::Arr(vec![CatalogJson::Obj(vec![
        ("name".to_string(), CatalogJson::Str("fs".to_string())),
        ("status".to_string(), status_record("disabled", None)),
    ])]);
    assert_eq!(
        parse_mcp_catalog_servers(&value),
        Some(vec![McpCatalogServerItem {
            name: "fs".to_string(),
            status: McpServerStatus {
                name: McpStatusName::Disabled,
                error: None,
            },
        }])
    );
}

#[test]
fn slice53_42_mcp_unknown_status_string_skips_server() {
    let value = CatalogJson::Obj(vec![
        ("old".to_string(), status_record("offline", None)),
        ("fs".to_string(), status_record("connected", None)),
    ]);
    let servers = parse_mcp_catalog_servers(&value).expect("drift skips, not fails");
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "fs");
}

#[test]
fn slice53_43_mcp_non_string_status_is_invalid() {
    let value = CatalogJson::Obj(vec![(
        "fs".to_string(),
        CatalogJson::Obj(vec![("status".to_string(), CatalogJson::Num(3.0))]),
    )]);
    assert_eq!(parse_mcp_catalog_servers(&value), None);
}

#[test]
fn slice53_44_mcp_invalid_shapes_are_none() {
    assert_eq!(parse_mcp_catalog_servers(&CatalogJson::Null), None);
    assert_eq!(parse_mcp_catalog_servers(&CatalogJson::Str("nope".to_string())), None);
    // Array item that is not a record.
    assert_eq!(
        parse_mcp_catalog_servers(&CatalogJson::Arr(vec![CatalogJson::Num(1.0)])),
        None
    );
    // Record item without a string name.
    assert_eq!(
        parse_mcp_catalog_servers(&CatalogJson::Arr(vec![CatalogJson::Obj(vec![(
            "status".to_string(),
            status_record("connected", None)
        )])])),
        None
    );
    // Non-record status payload.
    assert_eq!(
        parse_mcp_catalog_servers(&CatalogJson::Obj(vec![(
            "fs".to_string(),
            CatalogJson::Bool(true)
        )])),
        None
    );
}

// --- recent status precedence (45-48) ---

#[test]
fn slice53_45_recent_question_outranks_everything() {
    assert_eq!(
        decide_recent_status(true, true, Some("busy")),
        RecentStatus::Question
    );
    assert_eq!(decide_recent_status(true, false, None), RecentStatus::Question);
}

#[test]
fn slice53_46_recent_permission_outranks_running() {
    assert_eq!(
        decide_recent_status(false, true, Some("busy")),
        RecentStatus::Permission
    );
    assert_eq!(
        decide_recent_status(false, true, Some("retry")),
        RecentStatus::Permission
    );
}

#[test]
fn slice53_47_recent_busy_and_retry_run_otherwise_idle() {
    assert_eq!(decide_recent_status(false, false, Some("busy")), RecentStatus::Running);
    assert_eq!(
        decide_recent_status(false, false, Some("retry")),
        RecentStatus::Running
    );
    assert_eq!(decide_recent_status(false, false, Some("idle")), RecentStatus::Idle);
    assert_eq!(decide_recent_status(false, false, None), RecentStatus::Idle);
}

#[test]
fn slice53_48_recent_idle_when_nothing_pending() {
    assert_eq!(
        decide_recent_status(false, false, Some("done")),
        RecentStatus::Idle
    );
}
