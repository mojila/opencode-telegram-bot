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
//! - `src/app/services/folder-presence-service.ts` full decision core
//!   (`FolderPresence` verdicts, listing classification, path math,
//!   ancestor-walk decision, refill gate, ask-once dedupe; slice 174)
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
//! Slice 174 adds the folder-gone detector (`folder-presence-service.ts`
//! full decision core: listing classification, path math, walk verdict,
//! refill gate, ask-once dedupe); its own deviations are documented on the
//! section below.
//!
//! Step 2 (GREEN): pure ports of the TS gates below.

use std::collections::HashSet;

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

// ---------------------------------------------------------------------------
// Folder-gone detector (slice 174, GREEN).
//
// Rust port of the pure remainder in
// `src/app/services/folder-presence-service.ts`:
// - `GLOBAL_PROJECT_FOLDER` (the global project never vanishes);
// - the V2-only probe gate (any other version answers `"unknown"` before
//   listing);
// - `listFolder` answer classification (entries count as listed, a 500 as
//   the server failing to resolve the folder, anything else as another
//   failure);
// - `pathStyleOf` / `namesMatch` / `basename` / `dirname` path math, with
//   the ancestor-walk climbing while `parent !== child`;
// - the `checkFolderPresence` walk decision (a listed folder is present; a
//   listed ancestor still naming the child is unknown; a listed ancestor
//   missing the child is missing; any other non-500 answer is unknown);
// - the `collectFromPresentFolders` refill gate and the
//   `findMissingFolders` ask-once dedupe.
//
// Impure work stays with the caller: `file.list` network reads (one per
// probed folder, via the read-only `list_folder_entries` client), the walk
// loop itself, and logging.
//
// Deviations:
// 1. The TS `ClientError` name / `UnexpectedStatus` reason taxonomy
//    collapses to the HTTP status: only a 500 counts as `ServerError`.
// 2. `namesMatch` lowercases with Unicode `to_lowercase`, mirroring TS
//    `toLowerCase`.
// 3. `path_parent` expects the absolute server folders the detector probes
//    (relative inputs without a separator yield `"."` like Node posix);
//    Windows roots map to themselves so the walk terminates.
// ---------------------------------------------------------------------------

/// The global project of non-git folders; it is never a folder that can
/// vanish (TS: `GLOBAL_PROJECT_FOLDER`).
pub const GLOBAL_PROJECT_FOLDER: &str = "/";

/// `true` for the global project, which is always present (TS:
/// `if (folder === GLOBAL_PROJECT_FOLDER) return "present"`).
pub fn is_global_project_folder(folder: &str) -> bool {
    folder == GLOBAL_PROJECT_FOLDER
}

/// `true` when the server is asked about the folder at all: presence
/// probing is V2-only (TS: any other `opencodeServerVersion` answers
/// `"unknown"` before listing).
pub fn is_presence_checkable(server_version: &str) -> bool {
    server_version == "v2"
}

/// One folder listing as the detector reads it (TS: `FolderListing`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderListing {
    /// The folder lists (even empty): it exists.
    Listed,
    /// The listing failed with a 500: the server cannot resolve it.
    ServerError,
    /// Any other answer, including a transport failure.
    OtherFailure,
}

/// Classify one `file.list` answer (TS: `listFolder`). `listed_ok` is
/// `true` when the request answered 2xx with entries; otherwise the HTTP
/// status decides between a 500 the server failed on and anything else.
pub fn classify_listing_result(listed_ok: bool, http_status: Option<u16>) -> FolderListing {
    if listed_ok {
        return FolderListing::Listed;
    }
    if http_status == Some(500) {
        return FolderListing::ServerError;
    }
    FolderListing::OtherFailure
}

/// One ancestor answer on the walk to the filesystem root (TS: the
/// `listFolder(parent)` branch inside `checkFolderPresence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AncestorListing {
    /// The parent lists and still names the child on the way down.
    ContainsChild,
    /// The parent lists but no longer names the child: the folder is gone.
    WithoutChild,
    /// The parent listing failed with a 500; the walk climbs on.
    ServerError,
    /// Any other answer; the walk stops without a verdict.
    OtherFailure,
}

/// Presence verdict for a folder (TS: `FolderPresence`; `"unknown"`
/// covers every answer that is not a confirmed absence, and callers treat
/// it as present).
pub fn classify_folder_presence(own: FolderListing, ancestors: &[AncestorListing]) -> &'static str {
    match own {
        FolderListing::Listed => "present",
        FolderListing::OtherFailure => "unknown",
        FolderListing::ServerError => {
            for step in ancestors {
                match step {
                    AncestorListing::ContainsChild | AncestorListing::OtherFailure => {
                        return "unknown";
                    }
                    AncestorListing::WithoutChild => return "missing",
                    AncestorListing::ServerError => {}
                }
            }
            "unknown"
        }
    }
}

/// `true` for server paths in the Windows style (TS: `pathStyleOf`:
/// a drive-letter root or a UNC prefix; server paths arrive in the
/// server's style, which may differ from the bot's).
pub fn is_windows_style_path(folder: &str) -> bool {
    let bytes = folder.as_bytes();
    let drive_root = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    drive_root || folder.starts_with("\\\\")
}

/// Trailing `/`/`\` stripped (TS: `entry.replace(/[\\/]+$/, "")`).
pub fn strip_trailing_separators(entry: &str) -> &str {
    entry.trim_end_matches(['/', '\\'])
}

/// Entry-name match (TS: `namesMatch`: case-insensitive on win32, exact
/// otherwise).
pub fn folder_names_equal(windows_style: bool, left: &str, right: &str) -> bool {
    if windows_style {
        left.to_lowercase() == right.to_lowercase()
    } else {
        left == right
    }
}

/// Last segment with trailing separators stripped (TS: `basename` per path
/// style; win32 splits on both separators, posix on `/` only).
pub fn path_basename(path: &str, windows_style: bool) -> &str {
    let stripped = strip_trailing_separators(path);
    if windows_style {
        if stripped.len() == 2
            && stripped.as_bytes()[0].is_ascii_alphabetic()
            && stripped.as_bytes()[1] == b':'
        {
            return "";
        }
        match stripped.rfind(['/', '\\']) {
            Some(idx) => &stripped[idx + 1..],
            None => stripped,
        }
    } else {
        match stripped.rfind('/') {
            Some(idx) => &stripped[idx + 1..],
            None => stripped,
        }
    }
}

/// Parent folder (TS: `dirname` per path style). Roots map to themselves
/// so the ancestor walk terminates (`parent !== child` ends the loop).
pub fn path_parent(path: &str, windows_style: bool) -> String {
    if !windows_style {
        if path == "/" {
            return "/".to_string();
        }
        match strip_trailing_separators(path).rfind('/') {
            None => ".".to_string(),
            Some(0) => "/".to_string(),
            Some(idx) => strip_trailing_separators(path)[..idx].to_string(),
        }
    } else {
        windows_path_parent(path)
    }
}

/// Win32 `dirname` for the absolute folders the detector probes.
fn windows_path_parent(path: &str) -> String {
    let bytes = path.as_bytes();
    if bytes.len() == 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return format!("{}:\\", &path[..1]);
    }
    let stripped = strip_trailing_separators(path);
    if let Some(rest) = stripped.strip_prefix("\\\\") {
        let parts: Vec<&str> = rest
            .split(['/', '\\'])
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() <= 2 {
            return stripped.to_string();
        }
    }
    match stripped.rfind(['/', '\\']) {
        None => ".".to_string(),
        Some(idx) => {
            let parent = &stripped[..idx];
            if parent.len() == 2
                && parent.as_bytes()[0].is_ascii_alphabetic()
                && parent.as_bytes()[1] == b':'
            {
                format!("{parent}\\")
            } else if parent.is_empty() {
                "\\".to_string()
            } else {
                parent.to_string()
            }
        }
    }
}

/// `true` when a parent listing still names the child on the way down
/// (TS: the `entries.some(...)` in `checkFolderPresence`: trailing
/// separators stripped, basename compared, case-insensitive on win32).
/// `child_name` is the basename of the child folder.
pub fn ancestor_lists_child(windows_style: bool, entries: &[&str], child_name: &str) -> bool {
    entries.iter().any(|entry| {
        folder_names_equal(
            windows_style,
            path_basename(entry, windows_style),
            child_name,
        )
    })
}

/// `true` while the present-folder collector reads another page (TS:
/// `collectFromPresentFolders`: stops once `wanted` items remain or the
/// source is exhausted; otherwise the limit doubles and it reads again).
/// The surviving rows keep source order; the missing-folder cache stays
/// with the caller.
pub fn should_refetch_present_items(
    visible_count: usize,
    wanted: usize,
    fetched_count: usize,
    limit: usize,
) -> bool {
    visible_count < wanted && fetched_count >= limit
}

/// Folders in first-seen order, each once (TS: `[...new Set(folders)]` in
/// `findMissingFolders`: every folder is asked about once).
pub fn dedupe_folders<'a>(folders: &[&'a str]) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    folders
        .iter()
        .filter(|folder| seen.insert(**folder))
        .copied()
        .collect()
}
