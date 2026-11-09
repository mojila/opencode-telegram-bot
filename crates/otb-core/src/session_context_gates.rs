//! Session bootstrap / context-limit / missing-folder / inbox / attachment gates (slice 98, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/session-settings-service.ts` (`applySessionSettings`
//!   agent-truthiness adopt gate, `model?.providerID && model.id` attempt
//!   gate, `isSessionModel ? model.variant || "default" : "default"`
//!   pulled-variant rule, `!session.agent && !model` no-settings early
//!   `false`, `!isSameModel(before, after)` changed flag)
//! - `src/app/services/model-context-limit-service.ts`
//!   (`DEFAULT_CONTEXT_LIMIT` 200000, `PROVIDER_CACHE_TTL_MS` 600000,
//!   `getModelKey` slash join, blank-id default gate, every-empty
//!   no-cache guard, missing-providers no-cache guard, truthy-cache hit)
//! - `src/app/services/missing-folder-notice-service.ts`
//!   (`checkFolderPresence(...) !== "missing"` null gate, worktree-owner
//!   key choice)
//! - `src/app/services/folder-presence-service.ts` decision subset
//!   (`FolderPresence` `"missing"` equality used by the notice gate)
//! - `src/app/services/prompt-inbox-service.ts` (`withdrawInboxPrompt`
//!   no-inbox / list-error / not-included / cancel-error / removed
//!   precedence, `reconcileInboxPrompts` empty-mirror skip and
//!   stale-mirror drop gate)
//! - `src/app/services/prompt-attachment-service.ts`
//!   (`OPENCODE_TEXT_MIME`, worktree-mismatch drop, stat-error drop,
//!   not-a-file drop, `!isFileSizeAllowed` drop, outside-root drop)
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - `selectAgent` / `resolveModelToAdopt` / `selectModel` /
//!   `getStoredModel` reads, `isSameModel` model comparison (arrives as an
//!   explicit `before_eq_after` bool), `Date.now()` clocks, provider-list
//!   fetches, `getMissingExpectedProviders` / `readProvidersWhenListed`
//!   waits, `checkFolderPresence` / `findWorktreeOwner` network lookups,
//!   `opencodeV2Client.session.inbox` list/cancel calls,
//!   `promptQueue` mutation, `fs.stat` / `isFileSizeAllowed` /
//!   `isWithinProjectRootSafe` / `pathToFileURL` attachment build.
//!
//! Deviations: none intended. JS truthiness is mirrored explicitly:
//! `None` and `""` count as missing for agent / provider / model /
//! variant (whitespace-only strings count as present, like TS); a cached
//! `0` context limit counts as a miss (JS falsy); `[].every(...)` is
//! `true` for the empty provider list.
//!
//! Step 2 (GREEN): pure ports of the TS gates below.

/// Default context window when the provider list has no limit
/// (TS: `DEFAULT_CONTEXT_LIMIT`).
pub const DEFAULT_CONTEXT_LIMIT: u32 = 200_000;

/// Provider-list cache lifetime (TS: `PROVIDER_CACHE_TTL_MS`).
pub const PROVIDER_CACHE_TTL_MS: u64 = 600_000;

/// MIME sent for an `/ls` attachment file part
/// (TS: `OPENCODE_TEXT_MIME`).
pub const OPENCODE_TEXT_MIME: &str = "text/plain";

/// i18n key for a missing project folder
/// (TS: `t("bot.project_folder_missing", ...)`).
pub const MISSING_FOLDER_NOTICE_KEY: &str = "bot.project_folder_missing";

/// i18n key for a missing folder owned by a listed worktree parent
/// (TS: `t("bot.project_folder_missing_worktree", ...)`).
pub const MISSING_FOLDER_WORKTREE_NOTICE_KEY: &str = "bot.project_folder_missing_worktree";

/// Withdraw outcome for a mirrored V2 inbox prompt
/// (TS: `InboxWithdrawResult`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxWithdraw {
    Removed,
    Gone,
    Failed,
}

fn is_present(value: Option<&str>) -> bool {
    matches!(value, Some(s) if !s.is_empty())
}

/// `providerID/modelID` cache key (TS: `getModelKey`).
pub fn model_key(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}/{model_id}")
}

/// `true` when the limit call falls back without reading the cache
/// (TS: `if (!providerID || !modelID) return DEFAULT_CONTEXT_LIMIT`).
pub fn should_use_default_limit(provider_id: Option<&str>, model_id: Option<&str>) -> bool {
    !is_present(provider_id) || !is_present(model_id)
}

/// `true` when every provider lists zero models — the list is not cached
/// (TS: `providers.every((p) => Object.keys(p.models).length === 0)`).
pub fn is_providers_list_empty(model_counts: &[usize]) -> bool {
    model_counts.iter().all(|&count| count == 0)
}

/// `true` when the providers list may be cached: it names models and no
/// expected provider is still missing (TS: the two early `return`s in
/// `applyProvidersList`).
pub fn should_cache_providers_list(has_models: bool, missing_count: usize) -> bool {
    has_models && missing_count == 0
}

/// `true` when a cached limit counts as a hit (TS: `if (cachedLimit)`
/// truthiness — `None` and `0` miss).
pub fn is_context_cache_hit(cached: Option<u32>) -> bool {
    matches!(cached, Some(limit) if limit != 0)
}

/// `true` when the session agent is adopted (TS: `if (session.agent)`).
pub fn should_adopt_agent(agent: Option<&str>) -> bool {
    is_present(agent)
}

/// `true` when a model adopt is attempted
/// (TS: `if (model?.providerID && model.id)`).
pub fn should_attempt_model_adopt(provider_id: Option<&str>, model_id: Option<&str>) -> bool {
    is_present(provider_id) && is_present(model_id)
}

/// Variant stored for the pulled model
/// (TS: `isSessionModel ? model.variant || "default" : "default"`).
pub fn resolve_pulled_variant(is_session_model: bool, session_variant: Option<&str>) -> String {
    if is_session_model {
        if let Some(variant) = session_variant {
            if !variant.is_empty() {
                return variant.to_string();
            }
        }
    }
    "default".to_string()
}

/// `true` when the session carries neither agent nor model and the call
/// reports `false` early (TS: `if (!session.agent && !model)`).
pub fn should_report_no_settings(agent: Option<&str>, has_model: bool) -> bool {
    !is_present(agent) && !has_model
}

/// `true` when the stored model changed
/// (TS: `!isSameModel(modelBefore, getStoredModel())`).
pub fn has_session_model_changed(before_eq_after: bool) -> bool {
    !before_eq_after
}

/// `true` when the presence answer counts as gone
/// (TS: `checkFolderPresence(...) === "missing"`).
pub fn is_missing_presence(presence: &str) -> bool {
    presence == "missing"
}

/// Notice key for a folder, or `None` when it is not missing
/// (TS: `!== "missing"` null gate, then worktree-owner key choice).
pub fn select_missing_folder_notice(
    is_missing: bool,
    has_worktree_owner: bool,
) -> Option<&'static str> {
    if !is_missing {
        return None;
    }
    if has_worktree_owner {
        Some(MISSING_FOLDER_WORKTREE_NOTICE_KEY)
    } else {
        Some(MISSING_FOLDER_NOTICE_KEY)
    }
}

/// Withdraw precedence for one mirrored inbox prompt (TS:
/// `withdrawInboxPrompt`: no inbox / list failure / not included / cancel
/// failure / removed).
pub fn resolve_withdraw_outcome(
    has_inbox: bool,
    list_ok: bool,
    includes: bool,
    cancel_ok: bool,
) -> InboxWithdraw {
    if !has_inbox {
        return InboxWithdraw::Failed;
    }
    if !list_ok {
        return InboxWithdraw::Failed;
    }
    if !includes {
        return InboxWithdraw::Gone;
    }
    if !cancel_ok {
        return InboxWithdraw::Failed;
    }
    InboxWithdraw::Removed
}

/// `true` when there is nothing to reconcile
/// (TS: `if (mirrored.length === 0) return;`).
pub fn should_skip_inbox_reconcile(mirrored_count: usize) -> bool {
    mirrored_count == 0
}

/// `true` when a mirrored prompt no longer waits and is dropped
/// (TS: `item.inbox && !waitingIds.includes(item.inbox.inboxId)`).
pub fn should_drop_stale_mirror(list_ok: bool, includes: bool) -> bool {
    list_ok && !includes
}

/// `true` when the pending attachment belongs to another project
/// (TS: `if (pending.worktree !== worktree)`).
pub fn should_drop_attachment_on_project_change(attached: &str, current: &str) -> bool {
    attached != current
}

/// `true` when the attachment stat failed (TS: `catch` around `fs.stat`).
pub fn should_drop_attachment_on_stat_error(stat_ok: bool) -> bool {
    !stat_ok
}

/// `true` when the attachment path is not a file (TS: `!stat.isFile()`).
pub fn should_drop_attachment_when_not_file(is_file: bool) -> bool {
    !is_file
}

/// `true` when the attachment exceeds the size budget
/// (TS: `if (!isFileSizeAllowed(...))`).
pub fn should_drop_attachment_when_too_large(size_allowed: bool) -> bool {
    !size_allowed
}

/// `true` when the attachment left the project root
/// (TS: `if (!(await isWithinProjectRootSafe(...)))`).
pub fn should_drop_attachment_when_outside_root(inside_root: bool) -> bool {
    !inside_root
}
