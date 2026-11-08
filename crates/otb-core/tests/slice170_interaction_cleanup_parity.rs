//! Slice 170 parity: interaction-cleanup gates.
//!
//! Mirrors the pure TS contract in
//! `tests/app/managers/interaction-cleanup.test.ts` (the `reset` flow over
//! `src/app/managers/interaction-manager.ts`) and
//! `tests/app/managers/interaction-error-scope.test.ts` (the
//! `clearErrorScope` flow over the same manager):
//! - the `reset` effect (slot and waiting queue cleared together, the
//!   generation grows by exactly one, the waiting-request listener stays
//!   silent — including for a rename flow)
//! - the scope-to-kind table (`question` / `permission` / `rename` /
//!   `taskCreation` map to a slot kind, with `taskCreation` reading the
//!   `"task"` slot; `interaction` and `none` map to none)
//! - the error-scope decision (`none` never clears, `interaction` clears
//!   unconditionally, every other scope clears only the matching slot
//!   kind)
//! - the permission-first bump (only the `permission` scope bumps the
//!   generation before clearing, so a request it releases carries
//!   `generation + 1`)
//!
//! Already covered elsewhere and NOT re-tested here:
//! - `interaction_state::InteractionSlot` / `WaitingQueue` (the slot and
//!   queue primitives this flow composes)
//! - the `WaitingAgentRequestListener` delivery timing (`setImmediate`
//!   handoff), the `dropAndEndPoll` / `onQuestionDropped` /
//!   `onPermissionPromptsDropped` / `onReset` listener emissions, and the
//!   `logger` lines (callers)
//! - per-manager state (`QuestionManager` / `PermissionManager` /
//!   `RenameManager` / `TaskCreationManager` clear their own state when
//!   their slot kind is dropped)
//! - starting a new interaction after a cleanup (`start` after `reset`)

use otb_core::interaction_cleanup_gates::*;

// ---- reset effect (4) ----

#[test]
fn reset_generation_delta_is_one_parity() {
    // TS: `reset` calls `bumpGeneration` once, so `getGeneration()` grows
    // by exactly one.
    assert_eq!(RESET_GENERATION_DELTA, 1);
}

#[test]
fn reset_clears_slot_and_waiting_together_parity() {
    // TS: `reset` empties the slot and the waiting queue together, so
    // `getSnapshot()` and `getWaitingKind()` both read null.
    let effect = reset_effect();
    assert!(effect.clears_slot);
    assert!(effect.clears_waiting);
    assert_eq!(effect.generation_delta, RESET_GENERATION_DELTA);
}

#[test]
fn reset_suppresses_waiting_release_parity() {
    // TS: the queue is emptied before any release runs, so
    // `onWaitingRequestReady` never fires for a reset.
    assert!(reset_effect().suppresses_waiting_release);
}

#[test]
fn reset_clears_rename_flow_like_any_slot_parity() {
    // TS: a rename flow is cleared the same way — `reset` is
    // unconditional over the slot kind.
    let effect = reset_effect();
    assert!(effect.clears_slot);
    assert!(effect.clears_waiting);
}

// ---- scope-to-kind table (2) ----

#[test]
fn scope_slot_kind_table_parity() {
    // TS: `SCOPE_TO_INTERACTION_KIND` maps each scoped kind, while
    // `interaction` and `none` map to nothing.
    assert_eq!(
        scope_slot_kind(InteractionErrorScope::Question),
        Some(StatefulSlotKind::Question)
    );
    assert_eq!(
        scope_slot_kind(InteractionErrorScope::Permission),
        Some(StatefulSlotKind::Permission)
    );
    assert_eq!(
        scope_slot_kind(InteractionErrorScope::Rename),
        Some(StatefulSlotKind::Rename)
    );
    assert_eq!(scope_slot_kind(InteractionErrorScope::Interaction), None);
    assert_eq!(scope_slot_kind(InteractionErrorScope::None), None);
}

#[test]
fn scope_task_creation_reads_task_slot_parity() {
    // TS: the scope is named `taskCreation` but the slot kind is `"task"`.
    assert_eq!(
        scope_slot_kind(InteractionErrorScope::TaskCreation),
        Some(StatefulSlotKind::Task)
    );
}

// ---- none scope (1) ----

#[test]
fn none_scope_never_clears_parity() {
    // TS: `clearErrorScope("none", …)` returns early with the slot
    // untouched, whether or not a slot is live.
    use StatefulSlotKind::*;
    assert_eq!(
        decide_error_scope_clear(InteractionErrorScope::None, Some(Question)),
        ErrorScopeEffect::Noop
    );
    assert_eq!(
        decide_error_scope_clear(InteractionErrorScope::None, None),
        ErrorScopeEffect::Noop
    );
}

// ---- interaction scope (1) ----

#[test]
fn interaction_scope_clears_unconditionally_parity() {
    // TS: `clearErrorScope("interaction", …)` drops the slot whatever it
    // holds (clearing an empty slot is a harmless no-op downstream).
    use StatefulSlotKind::*;
    assert_eq!(
        decide_error_scope_clear(InteractionErrorScope::Interaction, Some(Question)),
        ErrorScopeEffect::ClearSlot
    );
    assert_eq!(
        decide_error_scope_clear(InteractionErrorScope::Interaction, None),
        ErrorScopeEffect::ClearSlot
    );
}

// ---- matching-kind scopes (4) ----

#[test]
fn question_scope_clears_matching_slot_parity() {
    // TS: the question scope clears a live question slot.
    assert_eq!(
        decide_error_scope_clear(
            InteractionErrorScope::Question,
            Some(StatefulSlotKind::Question)
        ),
        ErrorScopeEffect::ClearSlot
    );
}

#[test]
fn question_scope_keeps_unrelated_slot_parity() {
    // TS: the question scope leaves an inline slot (or an empty slot)
    // alone.
    assert_eq!(
        decide_error_scope_clear(InteractionErrorScope::Question, None),
        ErrorScopeEffect::Noop
    );
    assert_eq!(
        decide_error_scope_clear(
            InteractionErrorScope::Question,
            Some(StatefulSlotKind::Permission)
        ),
        ErrorScopeEffect::Noop
    );
}

#[test]
fn permission_scope_clears_matching_slot_parity() {
    // TS: the permission scope clears a live permission slot.
    assert_eq!(
        decide_error_scope_clear(
            InteractionErrorScope::Permission,
            Some(StatefulSlotKind::Permission)
        ),
        ErrorScopeEffect::ClearSlot
    );
}

#[test]
fn rename_and_task_creation_scopes_clear_matching_slots_parity() {
    // TS: the rename scope clears a rename slot but keeps a question slot;
    // the taskCreation scope clears a task slot.
    assert_eq!(
        decide_error_scope_clear(
            InteractionErrorScope::Rename,
            Some(StatefulSlotKind::Rename)
        ),
        ErrorScopeEffect::ClearSlot
    );
    assert_eq!(
        decide_error_scope_clear(
            InteractionErrorScope::Rename,
            Some(StatefulSlotKind::Question)
        ),
        ErrorScopeEffect::Noop
    );
    assert_eq!(
        decide_error_scope_clear(
            InteractionErrorScope::TaskCreation,
            Some(StatefulSlotKind::Task)
        ),
        ErrorScopeEffect::ClearSlot
    );
}

// ---- generation bump (3) ----

#[test]
fn only_permission_scope_bumps_generation_first_parity() {
    // TS: `clearErrorScope` bumps only for `scope === "permission"`.
    use InteractionErrorScope::*;
    assert!(scope_bumps_generation_first(Permission));
    assert!(!scope_bumps_generation_first(Question));
    assert!(!scope_bumps_generation_first(Rename));
    assert!(!scope_bumps_generation_first(TaskCreation));
    assert!(!scope_bumps_generation_first(Interaction));
    assert!(!scope_bumps_generation_first(None));
}

#[test]
fn permission_scope_releases_at_bumped_generation_parity() {
    // TS: a request released by the permission scope carries
    // `generation + 1`.
    assert_eq!(
        released_generation_for_scope(InteractionErrorScope::Permission, 7),
        8
    );
}

#[test]
fn other_scopes_release_at_current_generation_parity() {
    // TS: a request released by the question scope carries the same
    // generation it was queued under.
    assert_eq!(
        released_generation_for_scope(InteractionErrorScope::Question, 7),
        7
    );
    assert_eq!(
        released_generation_for_scope(InteractionErrorScope::Interaction, 7),
        7
    );
}
