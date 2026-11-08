//! Browse + catalog command dispatch gates (slice 105, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/bot/commands/ls-command.ts` (busy gate, container gate,
//!   `getProjectRoot` null gate, `ctx.match` trim + `args || remembered`
//!   target resolution, no-target gate, `isWithinProjectRoot` access-denied
//!   gate, `renderLsBrowseView` error gate, `!view.hasActions` plain-send
//!   gate)
//! - `src/bot/commands/open-command.ts` (busy gate, container gate,
//!   `roots.length === 1 && singleRoot` single-root gate,
//!   `renderOpenBrowseView` error gate)
//! - `src/bot/commands/command-catalog-command.ts`
//!   (`!currentProject` gate, `commands.length === 0` empty gate)
//! - `src/bot/commands/skills-catalog-command.ts` (same shape over
//!   `skills.length === 0`)
//! - `src/bot/commands/mcp-catalog-command.ts` (same shape over
//!   `servers.length === 0`)
//! - `src/bot/commands/sessions-command.ts` (`!currentProject` gate,
//!   `firstPage.sessions.length === 0` empty gate)
//! - `src/bot/commands/projects-command.ts`
//!   (`projects.length === 0` empty gate)
//! - `src/bot/commands/recent-command.ts` (`rows.length === 0` empty gate)
//! - `src/bot/commands/messages-command.ts` (`!currentProject` gate,
//!   `!currentSession` gate, `session.directory !== project.worktree`
//!   mismatch gate, `messages.length === 0` empty gate)
//! - `src/bot/commands/models.ts` (legacy providers command:
//!   `error || !providersData` fetch-error gate,
//!   `!providers || providers.length === 0` empty gate,
//!   `models.length === 0` per-provider no-models gate)
//!
//! Already ported elsewhere and NOT re-tested here:
//! - `isForegroundBusy` / `isContainerRuntime` themselves
//!   (`attach_foreground_gates` / `stores_runtime` owners; this slice owns
//!   only the `if (busy) return` / `if (container) return` dispatch above
//!   them, taking their outcomes as explicit bools).
//! - `getProjectRoot` / `isWithinProjectRoot` / `joinPath` / `getBaseName`
//!   path math (`file_browser_service_gates` owner; directory snapshots
//!   arrive as explicit `has_project` / `within_root` bools).
//! - `resolveInitialLsDirectory` / `rememberLsDirectory` user-scoped memory,
//!   `renderLsBrowseView` / `renderOpenBrowseView` /
//!   `buildOpenRootsKeyboard` menu building, catalog loaders
//!   (`loadCommandCatalog` / `loadSkillsCatalog` / `loadMcpCatalog` /
//!   `loadSessionPage` / `getListedProjects` / `loadRecentSessions` /
//!   `loadUserMessages`), page-size config (`commandsListLimit` etc.),
//!   `t(...)` reply keys (the caller maps the bool gates below to the
//!   `bot.project_not_selected` / `ls.access_denied` / `commands.empty` /
//!   `skills.empty` / `mcps.empty` / `sessions.empty` / `projects.empty` /
//!   `recent.empty` / `messages.*` / `legacy.models.*` texts).
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - grammy `ctx`/`Api` I/O (`reply`, `replyWithInlineMenu`,
//!   `interactionManager.start`), `getCurrentProject` / `getCurrentSession`
//!   reads, `syncSessionDirectoryCache`, `scanDirectory` / `scanLsDirectory`
//!   / `getFileDetails` scans, `opencodeClient.config.providers` fetch,
//!   `config.bot.*` limits, `logger`.
//!
//! Deviations: none intended. `ctx.match` trimming mirrors
//! `typeof ctx.match === "string" ? ctx.match.trim() : undefined`: callers
//! pass the already-trimmed `Option<&str>` (empty treated as missing, like
//! the TS `args || remembered` fallback). `roots[0]` presence is folded
//! into the `root_count === 1` gate (a one-element list always carries its
//! single root here).

/// `true` when a browse/list command stops early because the foreground is busy
/// (TS: `if (isForegroundBusy(deps)) { await replyBusyBlocked(ctx); return; }`).
pub fn should_block_command_when_busy(busy: bool) -> bool {
    busy
}

/// `true` when an `/ls` or `/open` browser stops early inside a container
/// (TS: `if (isContainerRuntime()) { await ctx.reply(...); return; }`).
pub fn should_report_container_unavailable(in_container: bool) -> bool {
    in_container
}

/// `true` when a browse/catalog command stops early without a project
/// (TS: `if (!currentProject) { await ctx.reply(...); return; }`).
pub fn should_report_no_project(has_project: bool) -> bool {
    !has_project
}

/// `true` when a catalog/list command reports its empty state
/// (TS: `if (list.length === 0) { await ctx.reply(...); return; }` for
/// commands, skills, MCP servers, sessions pages, projects, recent rows,
/// and user messages).
pub fn should_report_empty_list(count: usize) -> bool {
    count == 0
}

/// `true` when `/messages` stops early without a current session
/// (TS: `if (!currentSession) { await ctx.reply(...); return; }`).
pub fn should_report_no_session(has_session: bool) -> bool {
    !has_session
}

/// `true` when `/messages` stops early because the session belongs to another
/// project root (TS: `if (currentSession.directory !== currentProject.worktree)`).
pub fn should_report_session_project_mismatch(session_dir: &str, project_dir: &str) -> bool {
    session_dir != project_dir
}

/// Resolve the `/ls` target directory: the explicit `/ls <dir>` argument wins
/// when non-empty after trimming, otherwise the remembered directory
/// (TS: `const targetDir = args || resolveInitialLsDirectory(...)`).
pub fn resolve_ls_target<'a>(
    explicit_arg: Option<&'a str>,
    remembered: Option<&'a str>,
) -> Option<&'a str> {
    match explicit_arg {
        Some(arg) if !arg.trim().is_empty() => Some(arg.trim()),
        _ => remembered,
    }
}

/// `true` when `/ls` stops early without any target directory
/// (TS: `if (!targetDir) { await ctx.reply(...); return; }`).
pub fn should_report_no_ls_target(has_target: bool) -> bool {
    !has_target
}

/// `true` when `/ls` refuses a target outside the project root
/// (TS: `if (!isWithinProjectRoot(targetDir)) { await ctx.reply(...); return; }`).
pub fn should_report_ls_access_denied(within_root: bool) -> bool {
    !within_root
}

/// `true` when the `/ls` browse view is sent as plain text without actions
/// (TS: `if (!view.hasActions) { await ctx.reply(view.text, ...); return; }`).
pub fn should_send_ls_without_keyboard(has_actions: bool) -> bool {
    !has_actions
}

/// `true` when `/open` browses the single browser root directly instead of
/// listing roots (TS: `if (roots.length === 1 && singleRoot)`).
pub fn should_show_single_open_root(root_count: usize) -> bool {
    root_count == 1
}

/// `true` when an `/ls` or `/open` browse view reports its scan error
/// (TS: `if ("error" in view) { await ctx.reply(...); return; }`).
pub fn should_report_browse_scan_error(has_error: bool) -> bool {
    has_error
}

/// `true` when the legacy `/models` command reports a fetch error
/// (TS: `if (error || !providersData) { await ctx.reply(...); return; }`).
pub fn should_report_models_fetch_error(has_error: bool, has_data: bool) -> bool {
    has_error || !has_data
}

/// `true` when the legacy `/models` command reports no providers
/// (TS: `if (!providers || providers.length === 0)`).
pub fn should_report_no_providers(provider_count: usize) -> bool {
    provider_count == 0
}

/// `true` when a provider section shows its no-models line
/// (TS: `if (models.length === 0)` inside the per-provider loop).
pub fn should_show_no_models_for_provider(model_count: usize) -> bool {
    model_count == 0
}
