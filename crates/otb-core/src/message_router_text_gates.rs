//! Message-router text dispatch gates (slice 115, GREEN).
//!
//! Rust port of the pure remainder in
//! `src/bot/routers/message-router.ts` (`bot.on("message:text")`
//! dispatch chain):
//! - `getIncomingPrompt` miss (`None` input never routes)
//! - `"/"`-prefixed text miss (command path, never a prompt)
//! - active-question short-circuit (`questionManager.isActive()` answers
//!   via `handleQuestionTextAnswer` and stops)
//! - ordered handler precedence: goal -> task -> model-search ->
//!   project-search -> session-search -> rename -> catalog, each stopping
//!   the chain when it handles the text
//! - busy-session fallback: `tryEnqueuePromptIfBusy` queues behind a
//!   running turn, otherwise `queuePromptForMerging` merges the text.
//!
//! Step 2 (GREEN): pure port of the TS contract. The parity
//! tests in `crates/otb-core/tests/slice115_message_router_text_parity.rs`
//! assert the TS contract.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx` / `Api` I/O, `container.setTelegramContext`,
//!   `getIncomingPrompt` media assembly, `questionManager` /
//!   `interactionManager` snapshot reads, `handleGoalTextInput` /
//!   `handleTaskTextInput` / `handleModelSearchTextInput` /
//!   `handleProjectSearchTextInput` / `handleSessionSearchTextInput` /
//!   `handleRenameTextAnswer` / `handleCatalogTextArguments` /
//!   `tryEnqueuePromptIfBusy` / `queuePromptForMerging` execution,
//!   `config.bot.messageMergeWindowMs` reads, `t(...)` locale lookups
//!   (reply keys live in the leaf-gate owners).
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `"/"` text detection (`block_interaction_gates::is_command_text`
//!   owner; slash-ness arrives here as a precomputed `bool`)
//! - goal / task / model-search text eligibility and kind dispatch
//!   (`goal_text_input_gates` / `task_model_text_input_gates` owners)
//! - command vs skill catalog argument routing
//!   (`catalog_text_routing_gates` owner)
//! - rename / project-search / session-search text guards
//!   (`rename_callback_gates` / `project_search_callback_gates` /
//!   `session_search_callback_gates` owners)
//! - withdraw-inbox reply keys and menu-block gates
//!   (`block_interaction_gates` owner).
//!
//! Deviations: none intended. `None` input counts as a miss (TS
//! `if (!input) return`); slash-ness is a precomputed `bool` so this
//! slice owns only the ordering, exactly like the TS chain.

/// Slash prefix marking command text (TS: `text.startsWith("/")` in
/// `message-router.ts`; detection itself lives in
/// `block_interaction_gates::is_command_text`).
pub const MESSAGE_ROUTER_SLASH_PREFIX: &str = "/";

/// Where one `message:text` update ends up (TS: the early returns and
/// the final `queuePromptForMerging` tail of the `message:text` handler).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRouterTextRoute {
    MissNoInput,
    MissSlash,
    Question,
    Goal,
    Task,
    ModelSearch,
    ProjectSearch,
    SessionSearch,
    Rename,
    Catalog,
    Queue,
    Merge,
}

/// `true` when there is no routable text at all
/// (TS: `if (!input) return` after `getIncomingPrompt(ctx)`).
pub fn should_miss_without_input(has_input: bool) -> bool {
    !has_input
}

/// `true` when slash-prefixed text leaves the prompt path
/// (TS: `if (text.startsWith("/")) return` before the question check).
pub fn should_miss_for_slash(has_input: bool, is_slash: bool) -> bool {
    has_input && is_slash
}

/// `true` when the active question consumes the text and stops the chain
/// (TS: `if (container.questionManager.isActive()) {
/// await handleQuestionTextAnswer(...); return; }`).
pub fn should_route_to_question(has_input: bool, is_slash: bool, question_active: bool) -> bool {
    has_input && !is_slash && question_active
}

/// `true` when the goal stage is tried at all
/// (TS: reached only when the text survived the input/slash/question
/// gates; otherwise `handleGoalTextInput` never runs).
pub fn should_attempt_goal(has_input: bool, is_slash: bool, question_active: bool) -> bool {
    has_input && !is_slash && !question_active
}

/// `true` when a handled stage stops the chain
/// (TS: every `if (await handle*...) return` / `if (handled*) return`).
pub fn should_stop_after_handled(handled: bool) -> bool {
    handled
}

/// `true` when the text queues behind a busy turn instead of merging
/// (TS: `if (await tryEnqueuePromptIfBusy(ctx, input)) return` after all
/// search/rename/catalog stages missed).
pub fn should_queue_behind_busy(all_missed: bool, queued: bool) -> bool {
    all_missed && queued
}

/// Full precedence for one text update (TS: the ordered early returns in
/// the `message:text` handler: no-input miss, slash miss, question,
/// goal, task, model-search, project-search, session-search, rename,
/// catalog, queue-if-busy, else merge).
#[allow(clippy::too_many_arguments)]
pub fn select_message_router_text_route(
    has_input: bool,
    is_slash: bool,
    question_active: bool,
    goal_handled: bool,
    task_handled: bool,
    model_search_handled: bool,
    project_handled: bool,
    session_handled: bool,
    rename_handled: bool,
    catalog_handled: bool,
    queued_busy: bool,
) -> MessageRouterTextRoute {
    if !has_input {
        return MessageRouterTextRoute::MissNoInput;
    }
    if is_slash {
        return MessageRouterTextRoute::MissSlash;
    }
    if question_active {
        return MessageRouterTextRoute::Question;
    }
    if goal_handled {
        return MessageRouterTextRoute::Goal;
    }
    if task_handled {
        return MessageRouterTextRoute::Task;
    }
    if model_search_handled {
        return MessageRouterTextRoute::ModelSearch;
    }
    if project_handled {
        return MessageRouterTextRoute::ProjectSearch;
    }
    if session_handled {
        return MessageRouterTextRoute::SessionSearch;
    }
    if rename_handled {
        return MessageRouterTextRoute::Rename;
    }
    if catalog_handled {
        return MessageRouterTextRoute::Catalog;
    }
    if queued_busy {
        return MessageRouterTextRoute::Queue;
    }
    MessageRouterTextRoute::Merge
}
