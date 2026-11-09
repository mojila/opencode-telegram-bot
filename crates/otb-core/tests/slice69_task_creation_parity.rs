//! Slice 69 parity: task-creation gates (RED).
//!
//! Mirrors the pure TS contract in:
//! - `src/app/managers/scheduled-task-creation-manager.ts`
//!   (`TaskCreationManager`)
//! - `src/app/types/scheduled-task.ts` (`createScheduledTaskModel`,
//!   `cloneParsedTaskSchedule`, `cloneScheduledTaskModel`,
//!   `cloneScheduledTask`, `DEFAULT_TASK_AGENT`)
//!
//! Every test MUST FAIL until `task_creation_gates` is ported (step 2).

use otb_core::task_creation_gates::*;

fn sample_model() -> ScheduledTaskModel {
    create_scheduled_task_model("anthropic", "claude-sonnet", Some("default"))
}

fn sample_cron_task() -> ScheduledTask {
    ScheduledTask::Cron(ScheduledCronTask {
        base: ScheduledTaskBase {
            id: "task-1".to_string(),
            project_id: "proj-1".to_string(),
            project_worktree: "/work/a".to_string(),
            agent: Some("build".to_string()),
            model: sample_model(),
            schedule_text: "every day".to_string(),
            schedule_summary: "daily 09:00".to_string(),
            timezone: "UTC".to_string(),
            prompt: "run tests".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            next_run_at: Some("2026-01-02T09:00:00Z".to_string()),
            last_run_at: None,
            run_count: 0,
            last_status: "idle".to_string(),
            last_error: None,
        },
        cron: "0 9 * * *".to_string(),
        enabled: Some(true),
        activation_pending: None,
    })
}

fn sample_parsed_cron() -> ParsedTaskSchedule {
    ParsedTaskSchedule::Cron {
        cron: "0 9 * * *".to_string(),
        timezone: "UTC".to_string(),
        summary: "daily 09:00".to_string(),
        next_run_at: "2026-01-02T09:00:00Z".to_string(),
    }
}

// --- constants + model helpers ---

#[test]
fn slice69_default_task_agent_value_parity() {
    assert_eq!(DEFAULT_TASK_AGENT, "build");
}

#[test]
fn slice69_create_model_keeps_variant_parity() {
    let model = create_scheduled_task_model("openai", "gpt-5", Some("fast"));
    assert_eq!(model.provider_id, "openai");
    assert_eq!(model.model_id, "gpt-5");
    assert_eq!(model.variant, Some("fast".to_string()));
}

#[test]
fn slice69_create_model_none_variant_parity() {
    let model = create_scheduled_task_model("openai", "gpt-5", None);
    assert_eq!(model.variant, None);
}

#[test]
fn slice69_clone_model_equal_parity() {
    let model = sample_model();
    assert_eq!(clone_scheduled_task_model(&model), model);
}

#[test]
fn slice69_clone_model_independent_parity() {
    let model = sample_model();
    let mut cloned = clone_scheduled_task_model(&model);
    cloned.model_id = "mutated".to_string();
    assert_eq!(model.model_id, "claude-sonnet");
}

#[test]
fn slice69_clone_parsed_cron_roundtrip_parity() {
    let parsed = sample_parsed_cron();
    assert_eq!(clone_parsed_task_schedule(&parsed), parsed);
}

#[test]
fn slice69_clone_parsed_once_roundtrip_parity() {
    let parsed = ParsedTaskSchedule::Once {
        run_at: "2026-02-01T10:00:00Z".to_string(),
        timezone: "UTC".to_string(),
        summary: "once".to_string(),
        next_run_at: "2026-02-01T10:00:00Z".to_string(),
    };
    assert_eq!(clone_parsed_task_schedule(&parsed), parsed);
}

#[test]
fn slice69_clone_parsed_cron_independent_parity() {
    let parsed = sample_parsed_cron();
    let mut cloned = clone_parsed_task_schedule(&parsed);
    if let ParsedTaskSchedule::Cron { cron, .. } = &mut cloned {
        *cron = "mutated".to_string();
    }
    assert_eq!(clone_parsed_task_schedule(&parsed), sample_parsed_cron());
}

#[test]
fn slice69_clone_task_cron_backfills_enabled_parity() {
    let mut task = sample_cron_task();
    if let ScheduledTask::Cron(inner) = &mut task {
        inner.enabled = None;
    }
    let cloned = clone_scheduled_task(&task);
    match cloned {
        ScheduledTask::Cron(inner) => assert_eq!(inner.enabled, Some(true)),
        ScheduledTask::Once(_) => panic!("expected cron"),
    }
}

#[test]
fn slice69_clone_task_cron_keeps_explicit_disabled_parity() {
    let mut task = sample_cron_task();
    if let ScheduledTask::Cron(inner) = &mut task {
        inner.enabled = Some(false);
    }
    let cloned = clone_scheduled_task(&task);
    match cloned {
        ScheduledTask::Cron(inner) => assert_eq!(inner.enabled, Some(false)),
        ScheduledTask::Once(_) => panic!("expected cron"),
    }
}

#[test]
fn slice69_clone_task_backfills_agent_parity() {
    let mut task = sample_cron_task();
    if let ScheduledTask::Cron(inner) = &mut task {
        inner.base.agent = None;
    }
    let cloned = clone_scheduled_task(&task);
    match cloned {
        ScheduledTask::Cron(inner) => {
            assert_eq!(inner.base.agent, Some(DEFAULT_TASK_AGENT.to_string()))
        }
        ScheduledTask::Once(_) => panic!("expected cron"),
    }
}

#[test]
fn slice69_clone_task_keeps_explicit_agent_parity() {
    let cloned = clone_scheduled_task(&sample_cron_task());
    match cloned {
        ScheduledTask::Cron(inner) => {
            assert_eq!(inner.base.agent, Some("build".to_string()))
        }
        ScheduledTask::Once(_) => panic!("expected cron"),
    }
}

#[test]
fn slice69_clone_task_once_kind_preserved_parity() {
    let mut task = sample_cron_task();
    let base = match task {
        ScheduledTask::Cron(inner) => inner.base.clone(),
        ScheduledTask::Once(_) => unreachable!(),
    };
    task = ScheduledTask::Once(ScheduledOnceTask {
        base,
        run_at: "2026-03-01T10:00:00Z".to_string(),
    });
    let cloned = clone_scheduled_task(&task);
    match cloned {
        ScheduledTask::Once(inner) => {
            assert_eq!(inner.run_at, "2026-03-01T10:00:00Z")
        }
        ScheduledTask::Cron(_) => panic!("expected once"),
    }
}

#[test]
fn slice69_clone_task_model_deep_copy_parity() {
    let task = sample_cron_task();
    let mut cloned = clone_scheduled_task(&task);
    if let ScheduledTask::Cron(inner) = &mut cloned {
        inner.base.model.model_id = "mutated".to_string();
    }
    let fresh = clone_scheduled_task(&task);
    match fresh {
        ScheduledTask::Cron(inner) => assert_eq!(inner.base.model.model_id, "claude-sonnet"),
        ScheduledTask::Once(_) => panic!("expected cron"),
    }
}

// --- TaskCreationManager lifecycle ---

#[test]
fn slice69_start_returns_awaiting_schedule_parity() {
    let mut manager = TaskCreationManager::new();
    let state = manager.start("proj-1", "/work/a", sample_model(), "build");
    assert_eq!(state.stage, TaskCreationStage::AwaitingSchedule);
    assert_eq!(state.project_id, "proj-1");
    assert_eq!(state.project_worktree, "/work/a");
    assert_eq!(state.agent, "build");
}

#[test]
fn slice69_start_clears_schedule_fields_parity() {
    let mut manager = TaskCreationManager::new();
    let state = manager.start("proj-1", "/work/a", sample_model(), "build");
    assert_eq!(state.schedule_text, None);
    assert_eq!(state.parsed_schedule, None);
    assert_eq!(state.schedule_request_message_id, None);
    assert_eq!(state.preview_message_id, None);
    assert_eq!(state.prompt_request_message_id, None);
}

#[test]
fn slice69_start_clones_model_parity() {
    let mut manager = TaskCreationManager::new();
    let state = manager.start("proj-1", "/work/a", sample_model(), "build");
    assert_eq!(state.model, sample_model());
}

#[test]
fn slice69_is_active_false_initially_parity() {
    let manager = TaskCreationManager::new();
    assert!(!manager.is_active());
}

#[test]
fn slice69_is_active_true_after_start_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    assert!(manager.is_active());
}

#[test]
fn slice69_is_waiting_for_schedule_true_after_start_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    assert!(manager.is_waiting_for_schedule());
    assert!(!manager.is_parsing_schedule());
    assert!(!manager.is_waiting_for_prompt());
}

#[test]
fn slice69_waiting_flags_all_false_initially_parity() {
    let manager = TaskCreationManager::new();
    assert!(!manager.is_waiting_for_schedule());
    assert!(!manager.is_parsing_schedule());
    assert!(!manager.is_waiting_for_prompt());
}

#[test]
fn slice69_is_parsing_schedule_flow_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.mark_schedule_parsing();
    assert!(manager.is_parsing_schedule());
    assert!(!manager.is_waiting_for_schedule());
    manager.set_parsed_schedule("daily", sample_parsed_cron(), 11);
    assert!(!manager.is_parsing_schedule());
}

#[test]
fn slice69_is_waiting_for_prompt_true_after_set_parsed_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.set_parsed_schedule("daily", sample_parsed_cron(), 11);
    assert!(manager.is_waiting_for_prompt());
    assert!(!manager.is_waiting_for_schedule());
    assert!(!manager.is_parsing_schedule());
}

#[test]
fn slice69_get_state_none_initially_parity() {
    let manager = TaskCreationManager::new();
    assert_eq!(manager.get_state(), None);
}

#[test]
fn slice69_get_state_returns_clone_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    let mut first = manager.get_state().expect("state");
    first.project_id = "mutated".to_string();
    assert_eq!(manager.get_state().expect("again").project_id, "proj-1");
}

#[test]
fn slice69_set_parsed_schedule_moves_to_awaiting_prompt_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    let state = manager
        .set_parsed_schedule("every day", sample_parsed_cron(), 21)
        .expect("state");
    assert_eq!(state.stage, TaskCreationStage::AwaitingPrompt);
}

#[test]
fn slice69_set_parsed_schedule_records_text_and_ids_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    let state = manager
        .set_parsed_schedule("every day", sample_parsed_cron(), 21)
        .expect("state");
    assert_eq!(state.schedule_text, Some("every day".to_string()));
    assert_eq!(state.parsed_schedule, Some(sample_parsed_cron()));
    assert_eq!(state.preview_message_id, Some(21));
}

#[test]
fn slice69_set_parsed_schedule_clears_request_ids_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.set_schedule_request_message_id(7);
    manager.set_prompt_request_message_id(8);
    let state = manager
        .set_parsed_schedule("every day", sample_parsed_cron(), 21)
        .expect("state");
    assert_eq!(state.schedule_request_message_id, None);
    assert_eq!(state.prompt_request_message_id, None);
}

#[test]
fn slice69_set_parsed_schedule_empty_noop_parity() {
    let mut manager = TaskCreationManager::new();
    assert_eq!(
        manager.set_parsed_schedule("every day", sample_parsed_cron(), 21),
        None
    );
}

#[test]
fn slice69_mark_schedule_parsing_stage_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    let state = manager.mark_schedule_parsing().expect("state");
    assert_eq!(state.stage, TaskCreationStage::ParsingSchedule);
}

#[test]
fn slice69_mark_schedule_parsing_empty_noop_parity() {
    let mut manager = TaskCreationManager::new();
    assert_eq!(manager.mark_schedule_parsing(), None);
}

#[test]
fn slice69_set_prompt_request_message_id_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    let state = manager.set_prompt_request_message_id(33).expect("state");
    assert_eq!(state.prompt_request_message_id, Some(33));
}

#[test]
fn slice69_set_prompt_request_message_id_empty_noop_parity() {
    let mut manager = TaskCreationManager::new();
    assert_eq!(manager.set_prompt_request_message_id(33), None);
}

#[test]
fn slice69_set_schedule_request_message_id_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    let state = manager.set_schedule_request_message_id(44).expect("state");
    assert_eq!(state.schedule_request_message_id, Some(44));
}

#[test]
fn slice69_set_schedule_request_message_id_empty_noop_parity() {
    let mut manager = TaskCreationManager::new();
    assert_eq!(manager.set_schedule_request_message_id(44), None);
}

#[test]
fn slice69_reset_schedule_back_to_awaiting_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.set_parsed_schedule("every day", sample_parsed_cron(), 21);
    let state = manager.reset_schedule().expect("state");
    assert_eq!(state.stage, TaskCreationStage::AwaitingSchedule);
    assert!(manager.is_waiting_for_schedule());
    assert!(!manager.is_waiting_for_prompt());
}

#[test]
fn slice69_reset_schedule_clears_parsed_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.set_schedule_request_message_id(7);
    manager.set_parsed_schedule("every day", sample_parsed_cron(), 21);
    let state = manager.reset_schedule().expect("state");
    assert_eq!(state.schedule_text, None);
    assert_eq!(state.parsed_schedule, None);
    assert_eq!(state.schedule_request_message_id, None);
    assert_eq!(state.preview_message_id, None);
    assert_eq!(state.prompt_request_message_id, None);
}

#[test]
fn slice69_reset_schedule_empty_noop_parity() {
    let mut manager = TaskCreationManager::new();
    assert_eq!(manager.reset_schedule(), None);
}

#[test]
fn slice69_reset_preserves_project_and_agent_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-9", "/work/z", sample_model(), "plan");
    manager.set_parsed_schedule("every day", sample_parsed_cron(), 21);
    let state = manager.reset_schedule().expect("state");
    assert_eq!(state.project_id, "proj-9");
    assert_eq!(state.project_worktree, "/work/z");
    assert_eq!(state.agent, "plan");
    assert_eq!(state.model, sample_model());
}

#[test]
fn slice69_clear_deactivates_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.clear();
    assert!(!manager.is_active());
    assert_eq!(manager.get_state(), None);
}

#[test]
fn slice69_clear_empty_noop_parity() {
    let mut manager = TaskCreationManager::new();
    manager.clear();
    assert!(!manager.is_active());
}

#[test]
fn slice69_start_overwrites_previous_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.set_parsed_schedule("every day", sample_parsed_cron(), 21);
    let state = manager.start("proj-2", "/work/b", sample_model(), "plan");
    assert_eq!(state.stage, TaskCreationStage::AwaitingSchedule);
    assert_eq!(state.project_id, "proj-2");
    assert_eq!(state.parsed_schedule, None);
}

#[test]
fn slice69_clear_then_restart_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.clear();
    let state = manager.start("proj-2", "/work/b", sample_model(), "plan");
    assert!(manager.is_active());
    assert_eq!(state.project_id, "proj-2");
}

#[test]
fn slice69_get_state_preserves_message_ids_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager.set_schedule_request_message_id(7);
    manager.set_prompt_request_message_id(8);
    let state = manager.get_state().expect("state");
    assert_eq!(state.schedule_request_message_id, Some(7));
    assert_eq!(state.prompt_request_message_id, Some(8));
}

#[test]
fn slice69_set_parsed_schedule_stores_parsed_clone_parity() {
    let mut manager = TaskCreationManager::new();
    manager.start("proj-1", "/work/a", sample_model(), "build");
    manager
        .set_parsed_schedule("every day", sample_parsed_cron(), 21)
        .expect("state");
    let mut first = manager.get_state().expect("state");
    first.parsed_schedule = None;
    assert_eq!(
        manager.get_state().expect("again").parsed_schedule,
        Some(sample_parsed_cron())
    );
}

#[test]
fn slice69_full_flow_start_parse_reset_parity() {
    let mut manager = TaskCreationManager::new();
    assert!(!manager.is_active());
    manager.start("proj-1", "/work/a", sample_model(), "build");
    assert!(manager.is_waiting_for_schedule());
    manager.mark_schedule_parsing();
    assert!(manager.is_parsing_schedule());
    manager.set_parsed_schedule("every day", sample_parsed_cron(), 21);
    assert!(manager.is_waiting_for_prompt());
    manager.reset_schedule();
    assert!(manager.is_waiting_for_schedule());
    manager.clear();
    assert!(!manager.is_active());
}
