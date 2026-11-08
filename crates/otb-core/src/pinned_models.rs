//! Pure pinned-dashboard + model-catalog logic (slice 23b, RED).
//!
//! Rust port of the pure subset behind:
//! - `src/bot/pinned/pinned-message-format.ts` (`formatTokenCount`,
//!   `formatModelDisplayName`, `formatContextLine`, `formatCostLine`)
//! - `src/bot/pinned/pinned-message-manager.ts` pure state/formatting/gating:
//!   token accumulation (`input + cacheRead`), latest non-summary non-zero
//!   history context, cumulative-cost preference, diff filtering/equality,
//!   file-section rendering (max 10 + `... and N more`, outside-project
//!   shortening, `(+a -d)` suffix), title/cost update gating, attach
//!   detach rule, Telegram error classification (`message is not modified`,
//!   `message to edit not found`, `message to unpin not found`,
//!   transient `too many requests`), dashboard-enabled gating, project /
//!   branch / worktree line shaping
//! - `src/app/services/model-selection-service.ts` pure subset:
//!   `getModelKey`, `dedupeModels`, `filterModelsByCatalog`, favorite/recent
//!   normalization + validation, recent-minus-favorites, `searchModels`
//!   matching/sorting/limit, provider list/model list shaping, availability
//!   classification (`offered` / `model-missing` / `provider-missing` /
//!   `unknown`), stored-model reconcile fallback decision
//!
//! Deferred I/O (NOT ported here; covered by the remaining TS its):
//! Telegram `Api` send/edit/pin/unpin calls, debounce timers, SSE /
//! `opencodeClient.session.messages|diff|get` reads, `model.json` file IO,
//! provider-catalog fetch + TTL/warmup/wait loops, settings-store writes.
//!
//! Step 1 (RED): every function below is `unimplemented!()` so each test in
//! `tests/pinned_models_parity.rs` FAILS until the Rust port lands (step 2).

use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Shared model refs
// ---------------------------------------------------------------------------

/// Minimal model reference (TS: `FavoriteModel` / `ModelInfo` subset).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelRef {
    pub provider_id: String,
    pub model_id: String,
}

impl ModelRef {
    pub fn new(provider_id: &str, model_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
        }
    }
}

/// Raw favorite/recent entry as read from `model.json` (fields optional).
#[derive(Debug, Clone, Default)]
pub struct RawModelEntry {
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
}

impl RawModelEntry {
    pub fn valid(provider_id: &str, model_id: &str) -> Self {
        Self {
            provider_id: Some(provider_id.to_string()),
            model_id: Some(model_id.to_string()),
        }
    }

    pub fn missing_provider() -> Self {
        Self {
            provider_id: None,
            model_id: Some("m".to_string()),
        }
    }

    pub fn empty_provider() -> Self {
        Self {
            provider_id: Some(String::new()),
            model_id: Some("m".to_string()),
        }
    }

    pub fn missing_model() -> Self {
        Self {
            provider_id: Some("p".to_string()),
            model_id: None,
        }
    }

    pub fn empty_model() -> Self {
        Self {
            provider_id: Some("p".to_string()),
            model_id: Some(String::new()),
        }
    }
}

/// One assistant history message relevant to context accounting.
#[derive(Debug, Clone)]
pub struct HistoryMessage {
    pub summary: bool,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost: f64,
}

/// One changed file (TS: `FileChange`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub file: String,
    pub additions: u64,
    pub deletions: u64,
}

impl FileChange {
    pub fn new(file: &str, additions: u64, deletions: u64) -> Self {
        Self {
            file: file.to_string(),
            additions,
            deletions,
        }
    }
}

/// Raw diff entry where the file field may be absent (TS: entries without
/// `file` are ignored when loading diffs on session change).
#[derive(Debug, Clone)]
pub struct RawDiffEntry {
    pub file: Option<String>,
    pub additions: u64,
    pub deletions: u64,
}

/// Provider catalog entry used for pure list shaping.
#[derive(Debug, Clone)]
pub struct CatalogProvider {
    pub id: String,
    pub name: Option<String>,
    pub models: Vec<String>,
}

/// Shaped provider row (TS: `ProviderInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub model_count: usize,
}

/// Availability of one model (TS: `ModelAvailability`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelAvailability {
    Offered,
    ModelMissing,
    ProviderMissing,
    Unknown,
}

impl ModelAvailability {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelAvailability::Offered => "offered",
            ModelAvailability::ModelMissing => "model-missing",
            ModelAvailability::ProviderMissing => "provider-missing",
            ModelAvailability::Unknown => "unknown",
        }
    }
}

/// Default context limit (TS: `DEFAULT_CONTEXT_LIMIT`).
pub const DEFAULT_CONTEXT_LIMIT: u64 = 204_800;

/// Max files listed before collapsing the rest into `... and N more`.
pub const MAX_LISTED_FILES: usize = 10;

/// Max search results (TS: `SEARCH_RESULTS_LIMIT`).
pub const SEARCH_RESULTS_LIMIT: usize = 10;

// ---------------------------------------------------------------------------
// Pinned format helpers
// ---------------------------------------------------------------------------

/// TS: `formatTokenCount`.
pub fn format_token_count(count: u64) -> String {
    if count >= 1_000_000 {
        let formatted = format!("{:.1}M", count as f64 / 1_000_000.0);
        formatted.replace(".0M", "M")
    } else if count >= 1000 {
        format!("{}K", count / 1000)
    } else {
        count.to_string()
    }
}

/// TS: `formatModelDisplayName` (`None`/empty falls back to `"Unknown"`).
pub fn format_model_display_name(
    provider_id: Option<&str>,
    model_id: Option<&str>,
    variant: Option<&str>,
) -> String {
    let provider = provider_id.filter(|s| !s.is_empty());
    let model = model_id.filter(|s| !s.is_empty());
    match (provider, model) {
        (Some(p), Some(m)) => {
            let name = format!("{p}/{m}");
            match variant.filter(|s| !s.is_empty()) {
                Some(v) => format!("{name} ({v})"),
                None => name,
            }
        }
        _ => "Unknown".to_string(),
    }
}

/// TS: `formatContextLine` (en: `Context: {used} / {limit} ({percent}%)`,
/// unknown limit renders as `Unknown` with 0%).
pub fn format_context_line(tokens_used: u64, tokens_limit: Option<u64>) -> String {
    let safe_limit = tokens_limit.filter(|l| *l > 0);
    let percent = match safe_limit {
        Some(limit) => ((tokens_used as f64 / limit as f64) * 100.0).round() as u64,
        None => 0,
    };
    let used = format_token_count(tokens_used);
    let limit = safe_limit.map(format_token_count).unwrap_or_else(|| "Unknown".to_string());
    format!("Context: {used} / {limit} ({percent}%)")
}

/// TS: `formatCostLine` (en: `Cost: {cost} spent`, two decimals).
pub fn format_cost_line(cost: f64) -> String {
    format!("Cost: ${cost:.2} spent")
}

/// Token accumulation rule: `tokensUsed = input + cacheRead`.
pub fn tokens_used(input: u64, cache_read: u64) -> u64 {
    input.saturating_add(cache_read)
}

/// Context percentage, 0 when the limit is missing/non-positive.
pub fn context_percent(tokens_used: u64, tokens_limit: Option<u64>) -> u64 {
    match tokens_limit {
        Some(limit) if limit > 0 => ((tokens_used as f64 / limit as f64) * 100.0).round() as u64,
        _ => 0,
    }
}

/// Context info pair; `None` until a limit is known (TS: `getContextInfo`).
pub fn context_info(
    tokens_used: u64,
    tokens_limit: Option<u64>,
) -> Option<(u64, u64)> {
    match tokens_limit {
        Some(limit) if limit > 0 => Some((tokens_used, limit)),
        _ => None,
    }
}

/// Fall back to [`DEFAULT_CONTEXT_LIMIT`] when the lookup yields no limit.
pub fn fallback_context_limit(lookup: Option<u64>) -> u64 {
    lookup.unwrap_or(DEFAULT_CONTEXT_LIMIT)
}

/// Latest non-summary non-zero non-zero context message wins over the historical peak
/// (TS: `loadContextFromHistory` restores input+cache of that message and
/// sums window cost). Returns `(tokens_used, window_cost)`.
///
/// The history is read newest-first, so the first non-summary message with a
/// non-zero `input + cacheRead` size wins; the cost sums every non-summary
/// message (summary messages are technical, not real agent responses).
pub fn summarize_history_context(messages: &[HistoryMessage]) -> (u64, f64) {
    let mut tokens_used = 0u64;
    let mut found = false;
    let mut total_cost = 0.0f64;
    for message in messages {
        if message.summary {
            continue;
        }
        total_cost += message.cost;
        if !found {
            let size = message.input_tokens.saturating_add(message.cache_read_tokens);
            if size > 0 {
                tokens_used = size;
                found = true;
            }
        }
    }
    (tokens_used, total_cost)
}

/// Prefer the session cumulative cost over the loaded window when present.
pub fn prefer_session_cost(session_cost: Option<f64>, window_cost: f64) -> f64 {
    session_cost.unwrap_or(window_cost)
}

/// Keep the previous cost when a same-session refresh read fails.
pub fn keep_cost_on_failed_refresh(
    previous_cost: f64,
    refresh_cost: Option<f64>,
) -> f64 {
    refresh_cost.unwrap_or(previous_cost)
}

// ---------------------------------------------------------------------------
// Changed-files helpers
// ---------------------------------------------------------------------------

/// Drop diff entries without a file (TS: session-change diff load).
pub fn filter_diff_entries(entries: &[RawDiffEntry]) -> Vec<FileChange> {
    entries
        .iter()
        .filter_map(|entry| {
            entry.file.as_ref().map(|file| FileChange {
                file: file.clone(),
                additions: entry.additions,
                deletions: entry.deletions,
            })
        })
        .collect()
}

/// Equality used to ignore a diff identical to the current one.
pub fn diffs_equal(a: &[FileChange], b: &[FileChange]) -> bool {
    a == b
}

/// `(+a)` when only additions, `(+a -d)` otherwise.
pub fn format_file_diff_suffix(additions: u64, deletions: u64) -> String {
    let mut parts = Vec::new();
    if additions > 0 {
        parts.push(format!("+{additions}"));
    }
    if deletions > 0 {
        parts.push(format!("-{deletions}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(" "))
    }
}

/// Shorten paths outside the project to `.../last-3-segments`.
pub fn display_path_for_project(file: &str, project_path: &str) -> String {
    let normalized = file.replace('\\', "/");
    let worktree = project_path.replace('\\', "/");
    if !worktree.is_empty() && normalized.starts_with(worktree.as_str()) {
        let mut relative = normalized[worktree.len()..].to_string();
        if let Some(stripped) = relative.strip_prefix('/') {
            relative = stripped.to_string();
        }
        if !relative.is_empty() {
            return relative;
        }
        return normalized;
    }
    let segments: Vec<&str> = normalized.split('/').collect();
    if segments.len() <= 3 {
        return normalized;
    }
    format!(".../{}", segments[segments.len() - 3..].join("/"))
}

/// Render one file item line (TS `pinned.files.item`: `  {path}{diff}`).
pub fn format_file_item(change: &FileChange, project_path: &str) -> String {
    format!(
        "  {}{}",
        display_path_for_project(&change.file, project_path),
        format_file_diff_suffix(change.additions, change.deletions)
    )
}

/// Render the files section: `Files (n):` + at most 10 items + overflow line
/// `  ... and N more`. Empty when there are no files.
pub fn format_files_section(changes: &[FileChange], project_path: &str) -> String {
    if changes.is_empty() {
        return String::new();
    }
    let mut lines = vec![format!("Files ({}):", changes.len())];
    for change in changes.iter().take(MAX_LISTED_FILES) {
        lines.push(format_file_item(change, project_path));
    }
    if changes.len() > MAX_LISTED_FILES {
        lines.push(format!("  ... and {} more", changes.len() - MAX_LISTED_FILES));
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Incremental-update gating
// ---------------------------------------------------------------------------

/// Only a new non-empty session title triggers a dashboard update.
pub fn should_update_title(current: &str, next: &str) -> bool {
    !next.is_empty() && current != next
}

/// Cost updates that cannot change the total (zero-delta / NaN) are ignored.
pub fn should_apply_cost_update(_current_total: f64, delta: f64) -> bool {
    delta.is_finite() && delta != 0.0
}

/// Detaching drops the busy flag along with the active flag.
pub fn detach_attach_state(active: bool, busy: bool, detach: bool) -> (bool, bool) {
    if detach {
        (false, false)
    } else {
        (active, busy)
    }
}

/// Dashboard renders only when enabled AND a session with state exists.
pub fn should_render_dashboard(
    dashboard_enabled: bool,
    has_session: bool,
    has_message_state: bool,
) -> bool {
    dashboard_enabled && has_session && has_message_state
}

// ---------------------------------------------------------------------------
// Telegram error classification
// ---------------------------------------------------------------------------

/// `Bad Request: message is not modified` → treated as delivered.
pub fn is_not_modified_error(message: &str) -> bool {
    message.to_lowercase().contains("message is not modified")
}

/// `Bad Request: message to edit not found` → recreate the dashboard.
pub fn is_message_deleted_error(message: &str) -> bool {
    message.to_lowercase().contains("message to edit not found")
}

/// `Bad Request: message to unpin not found` → forget the leftover id.
pub fn is_unpin_not_found_error(message: &str) -> bool {
    message.to_lowercase().contains("message to unpin not found")
}

/// Transient failures (e.g. `too many requests`) keep ownership for retry.
pub fn is_transient_telegram_error(message: &str) -> bool {
    message.to_lowercase().contains("too many requests")
}

// ---------------------------------------------------------------------------
// Project / branch / worktree lines
// ---------------------------------------------------------------------------

/// `Project: {path}` plus `: {branch}` when a branch is known.
pub fn project_line(project_path: &str, branch: Option<&str>) -> String {
    match branch.filter(|b| !b.is_empty()) {
        Some(b) => format!("Project: {project_path}: {b}"),
        None => format!("Project: {project_path}"),
    }
}

/// Separate `Worktree: {path}` line only for linked worktrees.
pub fn worktree_line(worktree_path: Option<&str>, is_linked: bool) -> Option<String> {
    if !is_linked {
        return None;
    }
    match worktree_path.filter(|p| !p.is_empty()) {
        Some(p) => Some(format!("Worktree: {p}")),
        None => None,
    }
}

// ---------------------------------------------------------------------------
// Model-selection pure helpers
// ---------------------------------------------------------------------------

/// TS: `getModelKey` → `"providerID/modelID"`.
pub fn model_key(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}/{model_id}")
}

/// TS: `dedupeModels` (first occurrence wins).
pub fn dedupe_models(models: &[ModelRef]) -> Vec<ModelRef> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for model in models {
        let key = model_key(&model.provider_id, &model.model_id);
        if seen.insert(key) {
            out.push(model.clone());
        }
    }
    out
}

/// TS: `filterModelsByCatalog` (`None` catalog keeps everything).
pub fn filter_models_by_catalog(
    models: &[ModelRef],
    valid_keys: Option<&HashSet<String>>,
) -> Vec<ModelRef> {
    match valid_keys {
        None => models.to_vec(),
        Some(keys) => models
            .iter()
            .filter(|m| keys.contains(&model_key(&m.provider_id, &m.model_id)))
            .cloned()
            .collect(),
    }
}

/// TS: `normalizeFavoriteModels` / `normalizeRecentModels` — drop entries
/// with missing/empty `providerID` or `modelID`.
pub fn normalize_model_entries(entries: &[RawModelEntry]) -> Vec<ModelRef> {
    entries
        .iter()
        .filter_map(|entry| match (&entry.provider_id, &entry.model_id) {
            (Some(p), Some(m)) if !p.is_empty() && !m.is_empty() => Some(ModelRef {
                provider_id: p.clone(),
                model_id: m.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// Drop recent models already present in favorites.
pub fn exclude_favorites_from_recent(
    recent: &[ModelRef],
    favorites: &[ModelRef],
) -> Vec<ModelRef> {
    let favorite_keys: HashSet<String> = favorites
        .iter()
        .map(|m| model_key(&m.provider_id, &m.model_id))
        .collect();
    recent
        .iter()
        .filter(|m| !favorite_keys.contains(&model_key(&m.provider_id, &m.model_id)))
        .cloned()
        .collect()
}

/// Build the favorites list: validated file favorites + offered config
/// model, deduped.
pub fn build_favorites(
    file_favorites: &[ModelRef],
    config_default: Option<&ModelRef>,
    valid_keys: Option<&HashSet<String>>,
) -> Vec<ModelRef> {
    let mut combined = filter_models_by_catalog(file_favorites, valid_keys);
    if let Some(default) = config_default {
        let offered = filter_models_by_catalog(std::slice::from_ref(default), valid_keys);
        combined.extend(offered);
    }
    dedupe_models(&combined)
}

/// Build the recent list: validated + deduped file recents minus favorites.
pub fn build_recent(
    file_recent: &[ModelRef],
    favorites: &[ModelRef],
    valid_keys: Option<&HashSet<String>>,
) -> Vec<ModelRef> {
    let validated = filter_models_by_catalog(file_recent, valid_keys);
    let deduped = dedupe_models(&validated);
    exclude_favorites_from_recent(&deduped, favorites)
}

/// TS: `searchModels` — blank query yields nothing; case-insensitive
/// substring over `providerID/modelID`; sorted; capped at 10.
pub fn search_models(catalog: &[ModelRef], query: &str) -> Vec<ModelRef> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }
    let mut results: Vec<ModelRef> = catalog
        .iter()
        .filter(|m| {
            model_key(&m.provider_id, &m.model_id)
                .to_lowercase()
                .contains(&normalized)
        })
        .cloned()
        .collect();
    results.sort_by(|a, b| {
        model_key(&a.provider_id, &a.model_id)
            .to_lowercase()
            .cmp(&model_key(&b.provider_id, &b.model_id).to_lowercase())
    });
    results.truncate(SEARCH_RESULTS_LIMIT);
    results
}

/// TS: `getProviders` shaping — skip providers with no models, use the
/// display name when present, sort by (name, id).
pub fn shape_providers(catalog: &[CatalogProvider]) -> Vec<ProviderInfo> {
    let mut rows: Vec<ProviderInfo> = catalog
        .iter()
        .filter(|p| !p.models.is_empty())
        .map(|p| {
            let name = match &p.name {
                Some(n) if !n.is_empty() => n.clone(),
                _ => p.id.clone(),
            };
            ProviderInfo {
                id: p.id.clone(),
                name,
                model_count: p.models.len(),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    rows
}

/// TS: `getProviderModels` — models of one provider sorted by model ID.
pub fn provider_models(catalog: &[CatalogProvider], provider_id: &str) -> Vec<ModelRef> {
    match catalog.iter().find(|p| p.id == provider_id) {
        None => Vec::new(),
        Some(provider) => {
            let mut models: Vec<ModelRef> = provider
                .models
                .iter()
                .map(|m| ModelRef {
                    provider_id: provider.id.clone(),
                    model_id: m.clone(),
                })
                .collect();
            models.sort_by(|a, b| a.model_id.cmp(&b.model_id));
            models
        }
    }
}

/// TS: `getModelAvailability` pure classification over a listed catalog.
/// Empty catalog → `Unknown`; provider absent → `ProviderMissing`; model
/// absent → `ModelMissing`; otherwise `Offered`.
pub fn classify_model_availability(
    catalog: &HashMap<String, Vec<String>>,
    provider_id: &str,
    model_id: &str,
) -> ModelAvailability {
    if catalog.is_empty() {
        return ModelAvailability::Unknown;
    }
    match catalog.get(provider_id) {
        None => ModelAvailability::ProviderMissing,
        Some(models) if models.iter().any(|m| m == model_id) => ModelAvailability::Offered,
        Some(_) => ModelAvailability::ModelMissing,
    }
}

/// Stored-model reconcile decision: `true` when the stored model is missing
/// from a complete catalog AND the config fallback is offered (so the
/// caller replaces the stored model with the config default).
pub fn should_replace_stored_model(
    catalog_complete: bool,
    stored_listed: bool,
    fallback_offered: bool,
) -> bool {
    catalog_complete && !stored_listed && fallback_offered
}

/// Pure adoptability used by `resolveModelToAdopt`: offered/unknown models
/// adopt directly; a missing provider still adopts inside the warm-up window.
pub fn is_model_adoptable(availability: ModelAvailability, warmup_active: bool) -> bool {
    match availability {
        ModelAvailability::Offered | ModelAvailability::Unknown => true,
        ModelAvailability::ProviderMissing => warmup_active,
        ModelAvailability::ModelMissing => false,
    }
}
