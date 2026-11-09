//! Slice 79 parity: catalog callback + text-args pure gates (RED).
//!
//! Mirrors the pure TS contract in
//! `src/bot/callbacks/command-catalog-callback-handler.ts`
//! (`parseCommandItems`, `parseCommandsMetadata`, `commands:` prefix,
//! exact cancel/execute, message-id mismatch, `confirm` execute gate,
//! page-empty gate, select-index bounds, trimmed `/name args`
//! invocation, `type === "busy"`),
//! `src/bot/callbacks/skills-catalog-callback-handler.ts` (same shape
//! over `skills:`) and `src/bot/handlers/text-message-handler.ts`
//! (slash guard, `confirm`-stage gate, trimmed empty-arguments gate).
//!
//! Every test MUST FAIL with `unimplemented!()` panics until
//! `catalog_callback_gates` is ported.

use otb_core::catalog_callback_gates::*;

fn raw(name: Option<&str>, description: Option<&str>) -> RawCatalogItem {
    RawCatalogItem {
        name: name.map(str::to_string),
        description: description.map(str::to_string),
    }
}

fn snapshot(
    kind: &str,
    flow: Option<&str>,
    stage: Option<&str>,
    message_id: Option<i64>,
    project: Option<&str>,
    page: Option<f64>,
) -> InteractionSnapshot {
    InteractionSnapshot {
        kind: kind.to_string(),
        flow: flow.map(str::to_string),
        stage: stage.map(str::to_string),
        message_id,
        project_directory: project.map(str::to_string),
        commands: None,
        skills: None,
        command_name: None,
        skill_name: None,
        page,
    }
}

// --- parse_command_items ---

#[test]
fn slice79_command_items_valid_rows_parity() {
    let items = vec![
        raw(Some("deploy"), Some("ship it")),
        raw(Some("lint"), None),
    ];
    assert_eq!(
        parse_command_items(&items),
        Some(vec![
            CatalogItem {
                name: "deploy".to_string(),
                description: Some("ship it".to_string())
            },
            CatalogItem {
                name: "lint".to_string(),
                description: None
            },
        ])
    );
}

#[test]
fn slice79_command_items_empty_is_some_empty_parity() {
    assert_eq!(parse_command_items(&[]), Some(vec![]));
}

#[test]
fn slice79_command_items_blank_name_is_none_parity() {
    assert_eq!(parse_command_items(&[raw(Some("  "), None)]), None);
    assert_eq!(parse_command_items(&[raw(None, Some("x"))]), None);
}

#[test]
fn slice79_command_items_one_bad_row_poisons_all_parity() {
    let items = vec![raw(Some("ok"), None), raw(Some(""), None)];
    assert_eq!(parse_command_items(&items), None);
}

#[test]
fn slice79_command_items_whitespace_name_kept_verbatim_parity() {
    let items = vec![raw(Some(" deploy "), Some("d"))];
    assert_eq!(
        parse_command_items(&items),
        Some(vec![CatalogItem {
            name: " deploy ".to_string(),
            description: Some("d".to_string()),
        }])
    );
}

// --- parse_skill_items ---

#[test]
fn slice79_skill_items_valid_rows_parity() {
    let items = vec![raw(Some("pdf"), Some("read pdfs"))];
    assert_eq!(
        parse_skill_items(&items),
        Some(vec![CatalogItem {
            name: "pdf".to_string(),
            description: Some("read pdfs".to_string()),
        }])
    );
}

#[test]
fn slice79_skill_items_blank_name_is_none_parity() {
    assert_eq!(parse_skill_items(&[raw(Some(""), None)]), None);
}

#[test]
fn slice79_skill_items_empty_is_some_empty_parity() {
    assert_eq!(parse_skill_items(&[]), Some(vec![]));
}

// --- parse_commands_metadata ---

#[test]
fn slice79_commands_metadata_none_state_is_none_parity() {
    assert_eq!(parse_commands_metadata(None), None);
}

#[test]
fn slice79_commands_metadata_wrong_kind_is_none_parity() {
    let state = snapshot(
        "other",
        Some("commands"),
        Some("list"),
        Some(1),
        Some("/p"),
        Some(0.0),
    );
    assert_eq!(parse_commands_metadata(Some(&state)), None);
}

#[test]
fn slice79_commands_metadata_wrong_flow_is_none_parity() {
    let state = snapshot(
        "custom",
        Some("skills"),
        Some("list"),
        Some(1),
        Some("/p"),
        Some(0.0),
    );
    assert_eq!(parse_commands_metadata(Some(&state)), None);
}

#[test]
fn slice79_commands_metadata_missing_ids_is_none_parity() {
    let state = snapshot(
        "custom",
        Some("commands"),
        Some("list"),
        None,
        Some("/p"),
        Some(0.0),
    );
    assert_eq!(parse_commands_metadata(Some(&state)), None);
    let state = snapshot(
        "custom",
        Some("commands"),
        Some("list"),
        Some(7),
        None,
        Some(0.0),
    );
    assert_eq!(parse_commands_metadata(Some(&state)), None);
}

#[test]
fn slice79_commands_metadata_list_ok_parity() {
    let mut state = snapshot(
        "custom",
        Some("commands"),
        Some("list"),
        Some(11),
        Some("/proj"),
        Some(2.0),
    );
    state.commands = Some(vec![raw(Some("a"), None)]);
    assert_eq!(
        parse_commands_metadata(Some(&state)),
        Some(CommandsMetadata::List(CommandsListMetadata {
            message_id: 11,
            project_directory: "/proj".to_string(),
            commands: vec![CatalogItem {
                name: "a".to_string(),
                description: None
            }],
            page: 2,
        }))
    );
}

#[test]
fn slice79_commands_metadata_list_page_clamped_parity() {
    let mut state = snapshot(
        "custom",
        Some("commands"),
        Some("list"),
        Some(11),
        Some("/proj"),
        Some(-3.0),
    );
    state.commands = Some(vec![]);
    assert_eq!(
        parse_commands_metadata(Some(&state)),
        Some(CommandsMetadata::List(CommandsListMetadata {
            message_id: 11,
            project_directory: "/proj".to_string(),
            commands: vec![],
            page: 0,
        }))
    );
}

#[test]
fn slice79_commands_metadata_list_bad_items_is_none_parity() {
    let mut state = snapshot(
        "custom",
        Some("commands"),
        Some("list"),
        Some(11),
        Some("/proj"),
        Some(0.0),
    );
    state.commands = Some(vec![raw(Some(""), None)]);
    assert_eq!(parse_commands_metadata(Some(&state)), None);
}

#[test]
fn slice79_commands_metadata_confirm_ok_parity() {
    let mut state = snapshot(
        "custom",
        Some("commands"),
        Some("confirm"),
        Some(9),
        Some("/p"),
        None,
    );
    state.command_name = Some("deploy".to_string());
    assert_eq!(
        parse_commands_metadata(Some(&state)),
        Some(CommandsMetadata::Confirm(CommandsConfirmMetadata {
            message_id: 9,
            project_directory: "/p".to_string(),
            command_name: "deploy".to_string(),
        }))
    );
}

#[test]
fn slice79_commands_metadata_confirm_blank_name_is_none_parity() {
    let mut state = snapshot(
        "custom",
        Some("commands"),
        Some("confirm"),
        Some(9),
        Some("/p"),
        None,
    );
    state.command_name = Some("   ".to_string());
    assert_eq!(parse_commands_metadata(Some(&state)), None);
}

#[test]
fn slice79_commands_metadata_unknown_stage_is_none_parity() {
    let state = snapshot(
        "custom",
        Some("commands"),
        Some("detail"),
        Some(9),
        Some("/p"),
        None,
    );
    assert_eq!(parse_commands_metadata(Some(&state)), None);
}

// --- parse_skills_metadata ---

#[test]
fn slice79_skills_metadata_list_ok_parity() {
    let mut state = snapshot(
        "custom",
        Some("skills"),
        Some("list"),
        Some(5),
        Some("/p"),
        Some(1.0),
    );
    state.skills = Some(vec![raw(Some("s"), None)]);
    assert_eq!(
        parse_skills_metadata(Some(&state)),
        Some(SkillsMetadata::List(SkillsListMetadata {
            message_id: 5,
            project_directory: "/p".to_string(),
            skills: vec![CatalogItem {
                name: "s".to_string(),
                description: None
            }],
            page: 1,
        }))
    );
}

#[test]
fn slice79_skills_metadata_fractional_page_reads_zero_parity() {
    let mut state = snapshot(
        "custom",
        Some("skills"),
        Some("list"),
        Some(5),
        Some("/p"),
        Some(1.5),
    );
    state.skills = Some(vec![]);
    assert_eq!(
        parse_skills_metadata(Some(&state)),
        Some(SkillsMetadata::List(SkillsListMetadata {
            message_id: 5,
            project_directory: "/p".to_string(),
            skills: vec![],
            page: 0,
        }))
    );
}

#[test]
fn slice79_skills_metadata_confirm_ok_parity() {
    let mut state = snapshot(
        "custom",
        Some("skills"),
        Some("confirm"),
        Some(6),
        Some("/p"),
        None,
    );
    state.skill_name = Some("pdf".to_string());
    assert_eq!(
        parse_skills_metadata(Some(&state)),
        Some(SkillsMetadata::Confirm(SkillsConfirmMetadata {
            message_id: 6,
            project_directory: "/p".to_string(),
            skill_name: "pdf".to_string(),
        }))
    );
}

#[test]
fn slice79_skills_metadata_wrong_flow_is_none_parity() {
    let state = snapshot(
        "custom",
        Some("commands"),
        Some("list"),
        Some(1),
        Some("/p"),
        Some(0.0),
    );
    assert_eq!(parse_skills_metadata(Some(&state)), None);
}

#[test]
fn slice79_skills_metadata_confirm_missing_name_is_none_parity() {
    let state = snapshot(
        "custom",
        Some("skills"),
        Some("confirm"),
        Some(6),
        Some("/p"),
        None,
    );
    assert_eq!(parse_skills_metadata(Some(&state)), None);
}

// --- prefix guards ---

#[test]
fn slice79_handles_commands_prefix_parity() {
    assert!(handles_commands_callback(Some("commands:select:3")));
    assert!(handles_commands_callback(Some("commands:cancel")));
}

#[test]
fn slice79_handles_commands_prefix_negative_parity() {
    assert!(!handles_commands_callback(Some("skills:select:3")));
    assert!(!handles_commands_callback(None));
    assert!(!handles_commands_callback(Some("")));
}

#[test]
fn slice79_handles_skills_prefix_parity() {
    assert!(handles_skills_callback(Some("skills:page:1")));
    assert!(!handles_skills_callback(Some("commands:page:1")));
    assert!(!handles_skills_callback(None));
}

// --- exact cancel/execute ---

#[test]
fn slice79_commands_cancel_exact_parity() {
    assert!(is_commands_cancel("commands:cancel"));
    assert!(!is_commands_cancel("commands:cancel:x"));
}

#[test]
fn slice79_commands_execute_exact_parity() {
    assert!(is_commands_execute("commands:execute"));
    assert!(!is_commands_execute("commands:execute:1"));
}

#[test]
fn slice79_skills_cancel_execute_exact_parity() {
    assert!(is_skills_cancel("skills:cancel"));
    assert!(!is_skills_cancel("skills:cancelled"));
    assert!(is_skills_execute("skills:execute"));
    assert!(!is_skills_execute("skills:execute-now"));
}

// --- mismatch / stage / page / select ---

#[test]
fn slice79_callback_mismatch_guards_parity() {
    assert!(is_callback_message_mismatch(None, Some(1)));
    assert!(is_callback_message_mismatch(Some(1), None));
    assert!(is_callback_message_mismatch(Some(1), Some(2)));
}

#[test]
fn slice79_callback_match_is_not_mismatch_parity() {
    assert!(!is_callback_message_mismatch(Some(7), Some(7)));
}

#[test]
fn slice79_confirm_stage_gate_parity() {
    assert!(is_confirm_stage(Some("confirm")));
    assert!(!is_confirm_stage(Some("list")));
    assert!(!is_confirm_stage(None));
}

#[test]
fn slice79_catalog_page_empty_gate_parity() {
    assert!(is_catalog_page_empty(3, 3));
    assert!(is_catalog_page_empty(4, 3));
    assert!(is_catalog_page_empty(-1, 3));
    assert!(!is_catalog_page_empty(0, 3));
    assert!(!is_catalog_page_empty(2, 3));
}

#[test]
fn slice79_catalog_select_valid_parity() {
    assert!(is_catalog_select_valid(0, 2, true));
    assert!(is_catalog_select_valid(1, 2, true));
}

#[test]
fn slice79_catalog_select_invalid_parity() {
    assert!(!is_catalog_select_valid(2, 2, true));
    assert!(!is_catalog_select_valid(-1, 2, true));
    assert!(!is_catalog_select_valid(0, 2, false));
    assert!(!is_catalog_select_valid(0, 0, true));
}

#[test]
fn slice79_resolve_catalog_page_parity() {
    assert_eq!(resolve_catalog_page(Some(2.0)), 2);
    assert_eq!(resolve_catalog_page(Some(-2.0)), 0);
    assert_eq!(resolve_catalog_page(Some(1.5)), 0);
    assert_eq!(resolve_catalog_page(None), 0);
}

// --- text-args guards ---

#[test]
fn slice79_catalog_text_eligible_parity() {
    assert!(is_catalog_text_eligible(Some("some args")));
    assert!(is_catalog_text_eligible(Some("")));
}

#[test]
fn slice79_catalog_text_ineligible_parity() {
    assert!(!is_catalog_text_eligible(None));
    assert!(!is_catalog_text_eligible(Some("/deploy")));
    assert!(!is_catalog_text_eligible(Some("/")));
}

#[test]
fn slice79_arguments_empty_gate_parity() {
    assert!(is_arguments_empty("   "));
    assert!(is_arguments_empty(""));
    assert!(!is_arguments_empty(" hi "));
}

#[test]
fn slice79_trim_arguments_parity() {
    assert_eq!(trim_arguments("  hi  "), "hi");
    assert_eq!(trim_arguments("   "), "");
}

// --- invocations ---

#[test]
fn slice79_command_invocation_with_args_parity() {
    assert_eq!(
        build_command_invocation("deploy", "prod now"),
        "/deploy prod now"
    );
}

#[test]
fn slice79_command_invocation_without_args_parity() {
    assert_eq!(build_command_invocation("deploy", ""), "/deploy");
}

#[test]
fn slice79_skill_invocation_shapes_parity() {
    assert_eq!(build_skill_invocation("pdf", "page 2"), "/pdf page 2");
    assert_eq!(build_skill_invocation("pdf", ""), "/pdf");
}

// --- busy + clear ---

#[test]
fn slice79_session_busy_predicate_parity() {
    assert!(is_session_busy(Some("busy")));
    assert!(!is_session_busy(Some("idle")));
    assert!(!is_session_busy(None));
    assert!(!is_session_busy(Some("BUSY")));
}

#[test]
fn slice79_clear_runs_only_with_metadata_parity() {
    assert!(should_clear_catalog_interaction(true));
    assert!(!should_clear_catalog_interaction(false));
}

#[test]
fn slice79_callback_prefix_constants_parity() {
    assert_eq!(COMMANDS_CALLBACK_PREFIX, "commands:");
    assert_eq!(COMMANDS_CALLBACK_CANCEL, "commands:cancel");
    assert_eq!(COMMANDS_CALLBACK_EXECUTE, "commands:execute");
    assert_eq!(SKILLS_CALLBACK_PREFIX, "skills:");
    assert_eq!(SKILLS_CALLBACK_CANCEL, "skills:cancel");
    assert_eq!(SKILLS_CALLBACK_EXECUTE, "skills:execute");
}
