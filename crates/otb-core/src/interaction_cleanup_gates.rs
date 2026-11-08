//! Interaction cleanup gates (slice 170, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `tests/app/managers/interaction-cleanup.test.ts` (the `reset` contract
//! over `src/app/managers/interaction-manager.ts`) and
//! `tests/app/managers/interaction-error-scope.test.ts` (the
//! `clearErrorScope` contract over the same manager):
//! - the `reset` effect (slot and waiting queue cleared together, the
//!   generation grows by exactly one, no waiting request is handed to the
//!   listener)
//! - the scope-to-kind table (`SCOPE_TO_INTERACTION_KIND`: `question`,
//!   `permission`, `rename`, `taskCreation` map to a slot kind;
//!   `interaction` and `none` map to none, with `taskCreation` reading the
//!   `"task"` slot)
//! - the error-scope decision (`none` never clears, `interaction` clears
//!   unconditionally, every other scope clears only the matching slot kind)
//! - the permission-first bump (only the `permission` scope bumps the
//!   generation before clearing, so a request it releases carries
//!   `generation + 1`; every other scope releases at `generation`)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/app/managers/interaction-manager.ts` lines 74-90 (the
//!   `InteractionErrorScope` union and the `SCOPE_TO_INTERACTION_KIND`
//!   table), 242-265 (`reset`: snapshot the slot, count the waiting
//!   queue, empty the queue, bump the generation, drop the slot), 270-291
//!   (`clearErrorScope`: `none` returns, `interaction` drops everything,
//!   `permission` bumps first, the rest clear only their mapped kind).
//! - `tests/app/managers/interaction-cleanup.test.ts` lines 48-65 (slot
//!   and waiting cleared together, generation + 1, listener silent),
//!   67-74 (a rename flow is cleared the same way), 76-88 (a new
//!   interaction starts after cleanup).
//! - `tests/app/managers/interaction-error-scope.test.ts` lines 51-58
//!   (question scope clears), 60-67 (it keeps an inline slot), 69-83 (it
//!   releases waiting permissions at the same generation), 85-101 (the
//!   permission scope bumps first and releases at generation + 1),
//!   103-110 (permission scope clears), 112-119 (rename scope clears),
//!   121-128 (it keeps a question slot), 130-137 (taskCreation scope
//!   clears), 139-145 (interaction scope clears unconditionally), 147-154
//!   (none scope is a no-op).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - the `WaitingAgentRequestListener` delivery (a released request is
//!   handed over with `setImmediate`; test lines 69-83 and 85-101 assert
//!   the delivery, which is timing, not a gate — the gates below own only
//!   the cleared/bumped inputs to that delivery).
//! - `dropAndEndPoll` / `onQuestionDropped` / `onPermissionPromptsDropped`
//!   / `onReset` listener emissions and the `logger` lines.
//! - `QuestionManager` / `PermissionManager` / `RenameManager` /
//!   `TaskCreationManager` state (each manager clears its own state when
//!   its slot kind is dropped; this slice owns only the slot-side
//!   decision).
//! - starting a new interaction after a cleanup (test lines 76-88 is
//!   `start` after `reset`, owned by `interaction_state::InteractionSlot`).
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `interaction_state::InteractionSlot` / `WaitingQueue` (the slot and
//!   queue primitives this flow composes; slice 27b owner).
//! - `normalize_allowed_commands` and the `start` / `transition` /
//!   `waitQuestion` / `waitPermission` ordering (same owner).
//!
//! Deviations: none intended. A slot holding `inline` / `custom` (or no
//! slot at all) reads as `None` here: neither ever equals a mapped kind,
//! so the observable decision is identical. Clearing an empty slot is a
//! harmless TS no-op (`drop` returns `null`), so the `interaction` scope
//! reports `ClearSlot` even with no slot. Generations are `u64`, matching
//! `interaction_state`.
//!
//! Step 2 (GREEN): pure sync logic ported from the TS contract asserted
//! by the parity tests in
//! `crates/otb-core/tests/slice170_interaction_cleanup_parity.rs`.

/// Cleanup generation bump applied by `reset` (TS: `bumpGeneration` adds
/// exactly one, so `getGeneration()` grows by one per reset).
pub const RESET_GENERATION_DELTA: u64 = 1;

/// Error-cleanup scope (TS: `InteractionErrorScope`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionErrorScope {
    Question,
    Permission,
    Rename,
    TaskCreation,
    Interaction,
    None,
}

/// Slot kinds that carry their own state (TS:
/// `StatefulInteractionKind`: the `question` / `permission` / `rename` /
/// `task` slot kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatefulSlotKind {
    Question,
    Permission,
    Rename,
    Task,
}

/// What an error-scope cleanup does to the slot (TS: `clearErrorScope`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorScopeEffect {
    /// Nothing is cleared (TS: the `none` scope, or a scoped clear whose
    /// kind does not match the slot).
    Noop,
    /// The slot is cleared (TS: the `interaction` scope, or a scoped clear
    /// whose kind matches the slot).
    ClearSlot,
}

/// What `reset` does to the slot, the waiting queue, and the generation
/// (TS: `InteractionManager.reset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetEffect {
    /// The slot is emptied (`getSnapshot()` reads `null`, whatever the
    /// slot held — including a rename flow).
    pub clears_slot: bool,
    /// The waiting queue is emptied (`getWaitingKind()` reads `null`).
    pub clears_waiting: bool,
    /// Added to the generation (`getGeneration()` grows by exactly one).
    pub generation_delta: u64,
    /// No waiting request is handed to the listener (the queue is emptied
    /// before any release runs, so `onWaitingRequestReady` never fires).
    pub suppresses_waiting_release: bool,
}

/// The `reset` effect (TS: `reset` always clears the slot and the queue
/// together, bumps the generation once, and releases nothing).
pub fn reset_effect() -> ResetEffect {
    ResetEffect {
        clears_slot: true,
        clears_waiting: true,
        generation_delta: RESET_GENERATION_DELTA,
        suppresses_waiting_release: true,
    }
}

/// Scope-to-kind table (TS: `SCOPE_TO_INTERACTION_KIND`).
/// `Interaction` and `None` map to `None`; note `TaskCreation` reads the
/// `"task"` slot.
pub fn scope_slot_kind(scope: InteractionErrorScope) -> Option<StatefulSlotKind> {
    match scope {
        InteractionErrorScope::Question => Some(StatefulSlotKind::Question),
        InteractionErrorScope::Permission => Some(StatefulSlotKind::Permission),
        InteractionErrorScope::Rename => Some(StatefulSlotKind::Rename),
        InteractionErrorScope::TaskCreation => Some(StatefulSlotKind::Task),
        InteractionErrorScope::Interaction | InteractionErrorScope::None => None,
    }
}

/// `true` when the scope bumps the generation before clearing (TS: the
/// `if (scope === "permission") bumpGeneration()` branch in
/// `clearErrorScope` — only the permission scope bumps).
pub fn scope_bumps_generation_first(scope: InteractionErrorScope) -> bool {
    scope == InteractionErrorScope::Permission
}

/// Error-scope cleanup decision (TS: `clearErrorScope` — `none` never
/// clears, `interaction` clears unconditionally, every other scope clears
/// only when the slot holds its mapped kind). `slot_kind` is `None` for an
/// empty slot or an `inline` / `custom` slot, neither of which ever
/// matches.
pub fn decide_error_scope_clear(
    scope: InteractionErrorScope,
    slot_kind: Option<StatefulSlotKind>,
) -> ErrorScopeEffect {
    match scope {
        InteractionErrorScope::None => ErrorScopeEffect::Noop,
        InteractionErrorScope::Interaction => ErrorScopeEffect::ClearSlot,
        _ => {
            if scope_slot_kind(scope) == slot_kind {
                ErrorScopeEffect::ClearSlot
            } else {
                ErrorScopeEffect::Noop
            }
        }
    }
}

/// Generation observed by a request released from an error-scope clear
/// (TS: only the `permission` scope bumps first, so a request it releases
/// carries `generation + 1`; every other scope releases at `generation`).
pub fn released_generation_for_scope(scope: InteractionErrorScope, generation: u64) -> u64 {
    if scope_bumps_generation_first(scope) {
        generation + 1
    } else {
        generation
    }
}
