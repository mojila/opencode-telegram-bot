//! Parity port of the V1/V2 OpenCode mapper pure gates (slice 42, RED).
//!
//! Mirrors representative cases from:
//! - `tests/opencode/v2/mappers.test.ts` (tool names, tool input/metadata,
//!   patch-part metadata, form questions/answers, prompt parts, assistant
//!   messages, unknown-kind drops, goal-hidden user, providers catalog)
//! - `src/opencode/v2/mappers.ts` (every exported pure mapper: tool name
//!   table, `path`/`subagent_type`/`patchText` backfills, edit/apply_patch
//!   file-change metadata, text joining, error payload, session/global
//!   session/project rows, user/file message parts, assistant info/parts,
//!   tool-state branches, file diffs, model/provider/agent/command/MCP/
//!   permission rows, visible form fields, question mapping, form answers,
//!   prompt-part splits)
//! - `src/opencode/v2/goal-message.ts` (`isGoalInternalMetadata`)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::v2_mappers::*;
use std::collections::BTreeMap;

fn str_map(pairs: &[(&str, &str)]) -> JsonMap {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), JsonVal::Text(v.to_string())))
        .collect()
}

fn obj(pairs: &[(&str, JsonVal)]) -> JsonVal {
    JsonVal::Obj(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    )
}

fn tokens() -> TokenUsage {
    TokenUsage {
        input: 10,
        output: 2,
        reasoning: 0,
        cache_read: 0,
        cache_write: 0,
    }
}

fn session() -> V2SessionInfo {
    V2SessionInfo {
        id: "ses-1".to_string(),
        project_id: "proj-1".to_string(),
        parent_id: None,
        agent: Some("build".to_string()),
        model: Some("openai/gpt".to_string()),
        cost: 0.5,
        tokens: tokens(),
        title: Some("My session".to_string()),
        directory: "D:/repo".to_string(),
        created: 1,
        updated: 2,
        archived: None,
    }
}

fn model() -> V2Model {
    V2Model {
        id: "gpt".to_string(),
        model_id: "gpt-upstream".to_string(),
        provider_id: "openai".to_string(),
        name: "GPT".to_string(),
        family: None,
        capabilities: V2ModelCapabilities {
            tools: true,
            input: vec!["text".to_string(), "image".to_string()],
            output: vec!["text".to_string()],
        },
        cost: vec![],
        limit: V2ModelLimit {
            context: 1000,
            output: 100,
        },
        status: "active".to_string(),
        package: None,
        released_ms: 0,
        variants: vec![
            V2Variant {
                id: "low".to_string(),
                settings: None,
            },
            V2Variant {
                id: "high".to_string(),
                settings: None,
            },
        ],
        enabled: true,
    }
}

// --- toV1ToolName (4) ---

#[test]
fn slice42_01_shell_maps_to_bash() {
    assert_eq!(to_v1_tool_name("shell"), "bash");
}

#[test]
fn slice42_02_subagent_maps_to_task() {
    assert_eq!(to_v1_tool_name("subagent"), "task");
}

#[test]
fn slice42_03_patch_maps_to_apply_patch() {
    assert_eq!(to_v1_tool_name("patch"), "apply_patch");
}

#[test]
fn slice42_04_unknown_tool_names_pass_through() {
    assert_eq!(to_v1_tool_name("read"), "read");
}

// --- toV1ToolInput (6) ---

#[test]
fn slice42_05_path_backfills_file_path() {
    let input = str_map(&[("path", "a.txt")]);
    let mapped = to_v1_tool_input("read", &input);
    assert_eq!(
        mapped.get("path"),
        Some(&JsonVal::Text("a.txt".to_string()))
    );
    assert_eq!(
        mapped.get("filePath"),
        Some(&JsonVal::Text("a.txt".to_string()))
    );
}

#[test]
fn slice42_06_existing_file_path_is_kept() {
    let input = str_map(&[("path", "a.txt"), ("filePath", "b.txt")]);
    let mapped = to_v1_tool_input("read", &input);
    assert_eq!(
        mapped.get("filePath"),
        Some(&JsonVal::Text("b.txt".to_string()))
    );
}

#[test]
fn slice42_07_task_agent_backfills_subagent_type() {
    let input = str_map(&[("agent", "explore"), ("prompt", "look")]);
    let mapped = to_v1_tool_input("task", &input);
    assert_eq!(
        mapped.get("subagent_type"),
        Some(&JsonVal::Text("explore".to_string()))
    );
    assert_eq!(
        mapped.get("agent"),
        Some(&JsonVal::Text("explore".to_string()))
    );
}

#[test]
fn slice42_08_task_falls_back_to_type_field() {
    let input = str_map(&[("type", "explore")]);
    let mapped = to_v1_tool_input("task", &input);
    assert_eq!(
        mapped.get("subagent_type"),
        Some(&JsonVal::Text("explore".to_string()))
    );
}

#[test]
fn slice42_09_apply_patch_patch_backfills_patch_text() {
    let input = str_map(&[("patch", "*** Begin Patch")]);
    let mapped = to_v1_tool_input("apply_patch", &input);
    assert_eq!(
        mapped.get("patchText"),
        Some(&JsonVal::Text("*** Begin Patch".to_string()))
    );
}

#[test]
fn slice42_10_unrelated_tool_input_is_untouched() {
    let input = str_map(&[("command", "echo hi")]);
    assert_eq!(to_v1_tool_input("bash", &input), input);
}

// --- toV1ToolMetadata (5) ---

#[test]
fn slice42_11_edit_single_file_gains_diff_and_filediff() {
    let file = obj(&[
        ("file", JsonVal::Text("src/a.ts".to_string())),
        ("patch", JsonVal::Text("Index: src/a.ts\n+one".to_string())),
        ("status", JsonVal::Text("modified".to_string())),
        ("additions", JsonVal::Num(1.0)),
        ("deletions", JsonVal::Num(0.0)),
    ]);
    let mut metadata = JsonMap::new();
    metadata.insert("files".to_string(), JsonVal::Arr(vec![file]));
    metadata.insert("truncated".to_string(), JsonVal::Bool(false));
    let mapped = to_v1_tool_metadata("edit", &metadata);
    assert_eq!(
        mapped.get("diff"),
        Some(&JsonVal::Text("Index: src/a.ts\n+one".to_string()))
    );
    let filediff = obj(&[
        ("file", JsonVal::Text("src/a.ts".to_string())),
        ("patch", JsonVal::Text("Index: src/a.ts\n+one".to_string())),
        ("additions", JsonVal::Num(1.0)),
        ("deletions", JsonVal::Num(0.0)),
    ]);
    assert_eq!(mapped.get("filediff"), Some(&filediff));
    assert_eq!(mapped.get("truncated"), Some(&JsonVal::Bool(false)));
}

#[test]
fn slice42_12_edit_with_several_files_is_unchanged() {
    let file = obj(&[("file", JsonVal::Text("a.ts".to_string()))]);
    let mut metadata = JsonMap::new();
    metadata.insert("files".to_string(), JsonVal::Arr(vec![file.clone(), file]));
    assert_eq!(to_v1_tool_metadata("edit", &metadata), metadata);
}

#[test]
fn slice42_13_edit_without_string_file_is_unchanged() {
    let file = obj(&[("patch", JsonVal::Text("+x".to_string()))]);
    let mut metadata = JsonMap::new();
    metadata.insert("files".to_string(), JsonVal::Arr(vec![file]));
    assert_eq!(to_v1_tool_metadata("edit", &metadata), metadata);
}

#[test]
fn slice42_14_apply_patch_files_gain_relative_paths() {
    let file = obj(&[
        ("file", JsonVal::Text("src/a.ts".to_string())),
        ("patch", JsonVal::Text("+x".to_string())),
    ]);
    let mut metadata = JsonMap::new();
    metadata.insert("files".to_string(), JsonVal::Arr(vec![file]));
    let mapped = to_v1_tool_metadata("apply_patch", &metadata);
    let files = match mapped.get("files") {
        Some(JsonVal::Arr(files)) => files.clone(),
        other => panic!("expected files array, got {other:?}"),
    };
    let first = match &files[0] {
        JsonVal::Obj(map) => map.clone(),
        other => panic!("expected file record, got {other:?}"),
    };
    assert_eq!(
        first.get("filePath"),
        Some(&JsonVal::Text("src/a.ts".to_string()))
    );
    assert_eq!(
        first.get("relativePath"),
        Some(&JsonVal::Text("src/a.ts".to_string()))
    );
}

#[test]
fn slice42_15_metadata_without_files_is_unchanged() {
    let metadata = str_map(&[("truncated", "false")]);
    assert_eq!(to_v1_tool_metadata("read", &metadata), metadata);
}

// --- toolContentText (2) ---

#[test]
fn slice42_16_missing_content_is_empty_text() {
    assert_eq!(tool_content_text(None), "");
}

#[test]
fn slice42_17_text_items_join_with_newlines() {
    let content = vec![
        V2ContentItem {
            kind: "text".to_string(),
            text: Some("hi".to_string()),
        },
        V2ContentItem {
            kind: "image".to_string(),
            text: None,
        },
        V2ContentItem {
            kind: "text".to_string(),
            text: Some("there".to_string()),
        },
    ];
    assert_eq!(tool_content_text(Some(&content)), "hi\nthere");
}

// --- toV1ErrorPayload (2) ---

#[test]
fn slice42_18_missing_error_has_unknown_defaults() {
    assert_eq!(
        to_v1_error_payload(None),
        V1ErrorPayload {
            name: "UnknownError".to_string(),
            data_message: "Unknown error".to_string(),
        }
    );
}

#[test]
fn slice42_19_error_message_wins_over_type() {
    let error = V2StructuredError {
        message: Some("boom".to_string()),
        err_type: Some("SomeType".to_string()),
    };
    assert_eq!(
        to_v1_error_payload(Some(&error)),
        V1ErrorPayload {
            name: "UnknownError".to_string(),
            data_message: "boom".to_string(),
        }
    );
}

// --- toV1Session (3) ---

#[test]
fn slice42_20_session_core_fields_map_with_v2_version() {
    let mapped = to_v1_session(&session());
    assert_eq!(mapped.id, "ses-1");
    assert_eq!(mapped.slug, "ses-1");
    assert_eq!(mapped.project_id, "proj-1");
    assert_eq!(mapped.directory, "D:/repo");
    assert_eq!(mapped.version, "v2");
    assert_eq!(mapped.title, "My session");
}

#[test]
fn slice42_21_session_parent_omitted_when_none() {
    assert_eq!(to_v1_session(&session()).parent_id, None);
}

#[test]
fn slice42_22_session_title_defaults_to_empty() {
    let mut without_title = session();
    without_title.title = None;
    assert_eq!(to_v1_session(&without_title).title, "");
}

// --- toV1GlobalSession (1) ---

#[test]
fn slice42_23_global_session_has_null_project() {
    let mapped = to_v1_global_session(&session());
    assert_eq!(mapped.inner.id, "ses-1");
    assert_eq!(mapped.project, None);
}

// --- toV1Project (1) ---

#[test]
fn slice42_24_project_canonical_becomes_worktree() {
    let project = V2Project {
        id: "proj-1".to_string(),
        canonical: "D:/repo".to_string(),
        name: Some("Repo".to_string()),
        created: 1,
        updated: 2,
        sandboxes: vec![],
    };
    let mapped = to_v1_project(&project);
    assert_eq!(mapped.worktree, "D:/repo");
    assert_eq!(mapped.name, Some("Repo".to_string()));
}

// --- toV1Message user (2) + goal filter (1) ---

#[test]
fn slice42_25_user_text_part_shape() {
    let message = V2Message::User(V2UserMessage {
        id: "msg-user".to_string(),
        created: 2,
        text: "please rename the module".to_string(),
        files: vec![],
        metadata: None,
    });
    let mapped = to_v1_message(&message, "ses-1", "D:/repo").expect("user message maps");
    match mapped.info {
        V1MessageInfo::User { id, session_id, .. } => {
            assert_eq!(id, "msg-user");
            assert_eq!(session_id, "ses-1");
        }
        other => panic!("expected user info, got {other:?}"),
    }
    assert!(mapped.parts.iter().any(|part| matches!(
        part,
        V1Part::Text { text, .. } if text == "please rename the module"
    )));
}

#[test]
fn slice42_26_user_file_builds_data_url() {
    let message = V2Message::User(V2UserMessage {
        id: "msg-file".to_string(),
        created: 2,
        text: "look".to_string(),
        files: vec![V2UserFile {
            mime: "image/png".to_string(),
            name: Some("shot.png".to_string()),
            source_uri: None,
            data: Some("AAA".to_string()),
        }],
        metadata: None,
    });
    let mapped = to_v1_message(&message, "ses-1", "D:/repo").expect("file message maps");
    assert!(mapped.parts.iter().any(|part| matches!(
        part,
        V1Part::File { url, .. } if url == "data:image/png;base64,AAA"
    )));
}

#[test]
fn slice42_27_goal_internal_user_message_is_dropped() {
    let mut goal_meta = BTreeMap::new();
    goal_meta.insert("source".to_string(), "goal".to_string());
    let kickoff = V2Message::User(V2UserMessage {
        id: "msg-goal".to_string(),
        created: 1,
        text: "keep going".to_string(),
        files: vec![],
        metadata: Some(MetaVal::Record(goal_meta)),
    });
    assert!(to_v1_message(&kickoff, "ses-1", "D:/repo").is_none());
}

// --- toV1Message assistant (4) ---

fn assistant_message() -> V2Message {
    V2Message::Assistant(V2AssistantMessage {
        id: "msg-1".to_string(),
        agent: "build".to_string(),
        model: V2ModelRef {
            id: "m".to_string(),
            provider_id: "p".to_string(),
            variant: None,
        },
        time: V2Time {
            created: 1,
            completed: Some(5),
            streamed: None,
        },
        tokens: tokens(),
        cost: Some(0.5),
        error: None,
        finish: None,
        content: vec![V2Content::Text {
            text: "DONE".to_string(),
        }],
    })
}

#[test]
fn slice42_28_assistant_info_fields_map() {
    let mapped = to_v1_message(&assistant_message(), "ses-1", "D:/repo").expect("maps");
    match mapped.info {
        V1MessageInfo::Assistant {
            id,
            agent,
            model_id,
            provider_id,
            cost,
            ..
        } => {
            assert_eq!(id, "msg-1");
            assert_eq!(agent, "build");
            assert_eq!(model_id, "m");
            assert_eq!(provider_id, "p");
            assert_eq!(cost, 0.5);
        }
        other => panic!("expected assistant info, got {other:?}"),
    }
    assert!(mapped.parts.iter().any(|part| matches!(
        part,
        V1Part::Text { text, .. } if text == "DONE"
    )));
}

#[test]
fn slice42_29_assistant_streamed_time_is_carried() {
    let mut streamed = assistant_message();
    if let V2Message::Assistant(inner) = &mut streamed {
        inner.time.streamed = Some(4);
    }
    let mapped = to_v1_message(&streamed, "ses-1", "D:/repo").expect("maps");
    match mapped.info {
        V1MessageInfo::Assistant { streamed, .. } => assert_eq!(streamed, Some(4)),
        other => panic!("expected assistant info, got {other:?}"),
    }
}

#[test]
fn slice42_30_shell_tool_renames_to_bash_with_output() {
    let mut input = JsonMap::new();
    input.insert("command".to_string(), JsonVal::Text("echo hi".to_string()));
    let message = V2Message::Assistant(V2AssistantMessage {
        id: "msg-1".to_string(),
        agent: "build".to_string(),
        model: V2ModelRef {
            id: "m".to_string(),
            provider_id: "p".to_string(),
            variant: None,
        },
        time: V2Time {
            created: 1,
            completed: Some(5),
            streamed: None,
        },
        tokens: tokens(),
        cost: Some(0.0),
        error: None,
        finish: None,
        content: vec![V2Content::Tool {
            id: "call-1".to_string(),
            name: "shell".to_string(),
            state: V2ToolState::Completed {
                input,
                content: vec![V2ContentItem {
                    kind: "text".to_string(),
                    text: Some("hi".to_string()),
                }],
                metadata: None,
            },
            created: 2,
            ran: Some(3),
            completed: Some(4),
        }],
    });
    let mapped = to_v1_message(&message, "ses-1", "D:/repo").expect("maps");
    assert!(mapped.parts.iter().any(|part| matches!(
        part,
        V1Part::Tool { tool, call_id, state: V1ToolState::Completed { output, .. }, .. }
            if tool == "bash" && call_id == "call-1" && output == "hi"
    )));
}

#[test]
fn slice42_31_streaming_tool_state_becomes_pending() {
    let mut raw = JsonMap::new();
    raw.insert("command".to_string(), JsonVal::Text("echo hi".to_string()));
    let message = V2Message::Assistant(V2AssistantMessage {
        id: "msg-1".to_string(),
        agent: "build".to_string(),
        model: V2ModelRef {
            id: "m".to_string(),
            provider_id: "p".to_string(),
            variant: None,
        },
        time: V2Time {
            created: 1,
            completed: None,
            streamed: None,
        },
        tokens: tokens(),
        cost: None,
        error: None,
        finish: None,
        content: vec![V2Content::Tool {
            id: "call-1".to_string(),
            name: "read".to_string(),
            state: V2ToolState::Streaming { raw_input: raw },
            created: 2,
            ran: None,
            completed: None,
        }],
    });
    let mapped = to_v1_message(&message, "ses-1", "D:/repo").expect("maps");
    assert!(mapped.parts.iter().any(|part| matches!(
        part,
        V1Part::Tool {
            state: V1ToolState::Pending { .. },
            ..
        }
    )));
}

// --- unknown kinds + file diffs (2) ---

#[test]
fn slice42_32_message_kinds_without_v1_form_are_dropped() {
    assert!(to_v1_message(&V2Message::Other("idle".to_string()), "ses-1", "D:/repo").is_none());
}

#[test]
fn slice42_33_file_diff_copies_fields() {
    let diff = V2FileDiff {
        file: "src/a.ts".to_string(),
        status: "modified".to_string(),
        additions: 1,
        deletions: 0,
        patch: Some("+one".to_string()),
    };
    assert_eq!(
        to_v1_file_diff(&diff),
        V1FileDiff {
            file: "src/a.ts".to_string(),
            status: "modified".to_string(),
            additions: 1,
            deletions: 0,
            patch: Some("+one".to_string()),
        }
    );
}

// --- toV1Model (2) ---

#[test]
fn slice42_34_model_capabilities_derive_attachment_and_reasoning() {
    let mapped = to_v1_model(&model());
    assert!(mapped.capabilities.input.image);
    assert!(!mapped.capabilities.input.pdf);
    assert!(mapped.capabilities.reasoning);
    assert!(mapped.capabilities.toolcall);
    assert!(mapped.capabilities.temperature);
}

#[test]
fn slice42_35_model_missing_cost_defaults_to_zero() {
    let mapped = to_v1_model(&model());
    assert_eq!(mapped.cost.input, 0.0);
    assert_eq!(mapped.release_date, "1970-01-01");
    assert_eq!(mapped.limit_context, 1000);
}

// --- toV1Providers / agents / commands / MCP / permissions (6) ---

#[test]
fn slice42_36_providers_group_models_and_skip_disabled() {
    let providers = vec![V2ProviderRef {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
    }];
    let mut models = vec![model()];
    let mut disabled = model();
    disabled.id = "old".to_string();
    disabled.enabled = false;
    models.push(disabled);
    let mut foreign = model();
    foreign.id = "other".to_string();
    foreign.provider_id = "unknown-p".to_string();
    models.push(foreign);
    let mapped = to_v1_providers(&providers, &models);
    assert_eq!(mapped.len(), 2);
    let openai = mapped
        .iter()
        .find(|p| p.id == "openai")
        .expect("openai provider");
    assert_eq!(openai.name, "OpenAI");
    assert_eq!(openai.models.len(), 1);
    assert!(openai.models.contains_key("gpt"));
    let unknown = mapped
        .iter()
        .find(|p| p.id == "unknown-p")
        .expect("fallback provider");
    assert_eq!(unknown.name, "unknown-p");
}

#[test]
fn slice42_37_agent_id_becomes_name_with_model() {
    let agent = V2Agent {
        id: "build".to_string(),
        description: Some("Build agent".to_string()),
        mode: "primary".to_string(),
        hidden: false,
        color: None,
        model: Some(V2ModelRef {
            id: "m".to_string(),
            provider_id: "p".to_string(),
            variant: None,
        }),
        steps: None,
    };
    let mapped = to_v1_agent(&agent);
    assert_eq!(mapped.name, "build");
    assert_eq!(mapped.model_id, Some("m".to_string()));
    assert_eq!(mapped.model_provider_id, Some("p".to_string()));
}

#[test]
fn slice42_38_commands_then_skills_with_source_tags() {
    let mapped = to_v1_commands(
        &[V2CommandDef {
            name: "commit".to_string(),
            description: Some("d".to_string()),
        }],
        &[V2SkillDef {
            id: "review".to_string(),
            description: None,
        }],
    );
    assert_eq!(mapped.len(), 2);
    assert_eq!(mapped[0].source, "command");
    assert_eq!(mapped[0].description, Some("d".to_string()));
    assert_eq!(mapped[1].name, "review");
    assert_eq!(mapped[1].source, "skill");
}

#[test]
fn slice42_39_mcp_connected_and_auth_statuses_pass_through() {
    let mapped = to_v1_mcp_status(&[
        V2McpServer {
            name: "a".to_string(),
            status: "connected".to_string(),
            error: None,
        },
        V2McpServer {
            name: "b".to_string(),
            status: "needs_auth".to_string(),
            error: None,
        },
    ]);
    assert_eq!(
        mapped.get("a"),
        Some(&V1McpStatus {
            status: "connected".to_string(),
            error: None
        })
    );
    assert_eq!(
        mapped.get("b"),
        Some(&V1McpStatus {
            status: "needs_auth".to_string(),
            error: None
        })
    );
}

#[test]
fn slice42_40_mcp_failed_and_unknown_statuses_become_failed() {
    let mapped = to_v1_mcp_status(&[
        V2McpServer {
            name: "c".to_string(),
            status: "failed".to_string(),
            error: Some("refused".to_string()),
        },
        V2McpServer {
            name: "d".to_string(),
            status: "weird".to_string(),
            error: None,
        },
    ]);
    assert_eq!(
        mapped.get("c"),
        Some(&V1McpStatus {
            status: "failed".to_string(),
            error: Some("refused".to_string())
        })
    );
    assert_eq!(
        mapped.get("d"),
        Some(&V1McpStatus {
            status: "failed".to_string(),
            error: Some("weird".to_string())
        })
    );
}

#[test]
fn slice42_41_permission_fields_rename_with_defaults() {
    let request = V2PermissionRequest {
        id: "perm-1".to_string(),
        session_id: "ses-1".to_string(),
        action: "read".to_string(),
        resources: vec!["a.txt".to_string()],
        metadata: None,
        save: None,
    };
    let mapped = to_v1_permission(&request);
    assert_eq!(mapped.permission, "read");
    assert_eq!(mapped.patterns, vec!["a.txt".to_string()]);
    assert!(mapped.metadata.is_empty());
    assert!(mapped.always.is_empty());
}

// --- forms / questions / answers (5) ---

fn form_field(key: &str, field_type: &str) -> V2FormField {
    V2FormField {
        key: key.to_string(),
        field_type: field_type.to_string(),
        title: Some(key.to_string()),
        description: None,
        url: None,
        hidden: false,
        custom: None,
        options: vec![],
        when: vec![],
        default: FormDefault::Null,
    }
}

#[test]
fn slice42_42_visible_fields_drop_hidden_and_unmet_conditions() {
    let mut secret = form_field("secret", "string");
    secret.hidden = true;
    let mut extra = form_field("extra", "string");
    extra.when = vec![V2Condition {
        key: "ok".to_string(),
        op: "eq".to_string(),
        value: FormDefault::Bool(true),
    }];
    let form = V2Form {
        id: "form-1".to_string(),
        session_id: "ses-1".to_string(),
        title: "Pick one".to_string(),
        fields: vec![form_field("color", "string"), secret, extra],
    };
    let visible = visible_form_fields(&form);
    assert_eq!(
        visible.iter().map(|f| f.key.as_str()).collect::<Vec<_>>(),
        vec!["color"]
    );
}

#[test]
fn slice42_43_boolean_and_plain_questions_map() {
    let mut ok = form_field("ok", "boolean");
    ok.title = Some("Proceed?".to_string());
    let mut note = form_field("note", "string");
    note.title = Some("Note".to_string());
    let form = V2Form {
        id: "form-1".to_string(),
        session_id: "ses-1".to_string(),
        title: "Pick one".to_string(),
        fields: vec![ok, note],
    };
    let request = to_v1_question(&form);
    assert_eq!(request.questions.len(), 2);
    assert_eq!(
        request.questions[0]
            .options
            .iter()
            .map(|o| o.label.as_str())
            .collect::<Vec<_>>(),
        vec!["true", "false"]
    );
    assert_eq!(request.questions[0].custom, Some(false));
    assert!(request.questions[1].options.is_empty());
    assert_eq!(request.questions[1].custom, None);
}

#[test]
fn slice42_44_form_without_visible_fields_falls_back_to_title() {
    let mut secret = form_field("secret", "string");
    secret.hidden = true;
    let form = V2Form {
        id: "form-1".to_string(),
        session_id: "ses-1".to_string(),
        title: "Pick one".to_string(),
        fields: vec![secret],
    };
    let request = to_v1_question(&form);
    assert_eq!(request.questions.len(), 1);
    assert_eq!(request.questions[0].question, "Pick one");
}

#[test]
fn slice42_45_form_answers_resolve_labels_and_booleans() {
    let mut color = form_field("color", "string");
    color.options = vec![
        V2FormOption {
            value: "r".to_string(),
            label: "Red".to_string(),
            description: None,
        },
        V2FormOption {
            value: "g".to_string(),
            label: "Green".to_string(),
            description: None,
        },
    ];
    let ok = form_field("ok", "boolean");
    let form = V2Form {
        id: "form-1".to_string(),
        session_id: "ses-1".to_string(),
        title: "Pick one".to_string(),
        fields: vec![color, ok],
    };
    let answers = vec![vec!["Green".to_string()], vec!["false".to_string()]];
    let mapped = to_form_answer(&form, &answers).expect("valid answers");
    assert_eq!(
        mapped.get("color"),
        Some(&FormAnswerValue::Text("g".to_string()))
    );
    assert_eq!(mapped.get("ok"), Some(&FormAnswerValue::Bool(false)));
}

#[test]
fn slice42_46_invalid_integer_answer_is_an_error() {
    let count = form_field("count", "integer");
    let form = V2Form {
        id: "form-1".to_string(),
        session_id: "ses-1".to_string(),
        title: "Pick one".to_string(),
        fields: vec![count],
    };
    assert!(to_form_answer(&form, &[vec!["three".to_string()]]).is_err());
}

// --- prompt parts + goal metadata (2) ---

#[test]
fn slice42_47_prompt_parts_split_into_text_and_files() {
    let parts = vec![
        V1PromptPart {
            part_type: "text".to_string(),
            text: Some("Look at this".to_string()),
            mime: None,
            filename: None,
            url: None,
        },
        V1PromptPart {
            part_type: "file".to_string(),
            text: None,
            mime: Some("image/png".to_string()),
            filename: Some("shot.png".to_string()),
            url: Some("data:image/png;base64,AAA".to_string()),
        },
    ];
    let mapped = to_v2_prompt_input(&parts);
    assert_eq!(mapped.text, "Look at this");
    assert_eq!(
        mapped.files,
        vec![V2PromptFile {
            uri: "data:image/png;base64,AAA".to_string(),
            name: Some("shot.png".to_string()),
        }]
    );
}

#[test]
fn slice42_48_non_goal_metadata_is_not_internal() {
    let mut user_meta = BTreeMap::new();
    user_meta.insert("source".to_string(), "user".to_string());
    assert!(!is_goal_internal_metadata(&MetaVal::Record(user_meta)));
    assert!(!is_goal_internal_metadata(&MetaVal::Other));
}
