//! Agent / model / variant selection pure gates (slice 62, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/agent-selection-service.ts` (`DEFAULT_AGENT`,
//!   `pickFallbackAgent` build-preferring fallback, `configuredField`
//!   trim/empty gate, `getStoredAgent`/`resolveProjectAgent` default gates,
//!   stored-beats-stale-session preference, available-name membership)
//! - `src/app/services/variant-selection-service.ts`
//!   (`formatVariantForButton` `💡`-prefix, `formatVariantForDisplay`
//!   capitalization, `variant:<id>` build/parse, `getCurrentVariant`
//!   `|| "default"` gate, model-selected gate, default-only fallback label,
//!   `< 2` drawn-variants skip gate, default-first variant-list build with
//!   duplicate-`default` skip and `disabled` passthrough,
//!   `validateVariantForModel` non-disabled match)
//! - `src/app/services/model-selection-service.ts` (`SEARCH_RESULTS_LIMIT`,
//!   `getModelKey` `provider/model` join, `dedupeModels` first-wins,
//!   `filterModelsByCatalog` null-catalog passthrough,
//!   `searchModels` trim/lowercase needle, substring match on the
//!   lowercased key, `localeCompare` order, 10-result cap,
//!   `getModelAvailability` error/empty/provider/model decision table,
//!   `isModelAdoptable` offered/unknown/warmup gates, favorites + recent
//!   list assembly with env-default append and favorite-key pruning)
//! - `src/bot/menus/agent-selection-menu.ts` (`agent:<name>` build, `✅`
//!   active label)
//! - `src/bot/menus/variant-selection-menu.ts` (default-variant `✅`
//!   fallback row)
//! - `src/bot/menus/model-selection-menu.ts` (`MODEL_*` prefixes,
//!   `buildModelListCallback`, digits-only `parseIndex`,
//!   `parseProvidersPageCallback`, the two-segment `parseProviderCallback`,
//!   `parseProviderModelCallback`, `calculateModelsPaginationRange`,
//!   provider-row / provider-model / list button labels, provider-browser
//!   callback builders)
//! - `src/bot/callbacks/agent-selection-callback-handler.ts` (`agent:`
//!   prefix guard)
//! - `src/bot/callbacks/variant-selection-callback-handler.ts` (`variant:`
//!   prefix guard, model-not-selected gate)
//! - `src/bot/callbacks/model-selection-callback-handler.ts`
//!   (`MODEL_SEARCH_RESULT_CALLBACK_PREFIX`, `parseModelItems` shape gates
//!   with `"default"` variant fallback, `parseNonNegativeIndex`,
//!   `parseCallbackIndex`, `resolveModelListCallback` kind/index/bounds
//!   gates with variant reset, `parseLegacyModelCallback` colon tail join,
//!   `isShortModelCallback`, `isProviderBrowserCallback`, `parseProviderItems`
//!   with `0` count fallback, `handleModelSelect` search/provider-browser
//!   skip gates, the provider-browser dispatch order, the search-results
//!   stage/message/cancel/again/result/legacy dispatch)
//!
//! No grammy/Telegram I/O, no SDK fetches, no store/`config`/env reads live
//! here — only callback prefixes, label composition, list math, and routing
//! predicates. Callers resolve `t(...)` locales, catalog rows, and keyboards.
//!
//! DEVIATIONS from TS:
//! 1. `String.length` / `slice` / `charAt` count UTF-16 code units in TS. The
//!    port works on `char`s; parity fixtures use ASCII-only inputs where the
//!    two coincide (`✅`/`⭐`/`🕘`/`💡` emoji are compared as whole strings).
//! 2. `data.replace("agent:", "")` / `.replace("variant:", "")` remove the
//!    first occurrence anywhere in TS. The port uses `strip_prefix`, which
//!    coincides once the handler's `startsWith` guard has passed; unprefixed
//!    input reads as `None` here instead of the unchanged string.
//! 3. `Number.parseInt` on a huge digit run yields `Infinity`, which fails
//!    the `isInteger` gate and reads as `None`. The port parses `u64` and
//!    likewise returns `None` on overflow; fixtures stay in range.
//! 4. `parseModelItems` reads `unknown` JSON in TS. The port takes already
//!    extracted `Option<&str>` fields: `None` models a missing/non-string
//!    field, `Some("")` stays a (falsy but passing) empty string, and a
//!    missing/non-string variant falls back to `"default"`.
//! 5. `searchModels` sorts with `localeCompare` in TS. The port sorts by the
//!    lowercased `provider/model` key bytes; ASCII fixtures order the same.
//! 6. `charAt(0).toUpperCase()` can expand a character (e.g. `ß` -> `SS`).
//!    The port uppercases the first `char` via `char::to_uppercase` and
//!    keeps the remainder untouched; ASCII fixtures coincide.
//! 7. `messageId` / catalog-row plumbing (`InteractionManager` snapshots,
//!    `ProviderInfo` records) is flattened to explicit scalar arguments so
//!    the routing gates stay pure; callers own the managers.

use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Fallback agent name (TS: `DEFAULT_AGENT`).
pub const DEFAULT_AGENT: &str = "build";

/// Maximum search-result rows (TS: `SEARCH_RESULTS_LIMIT`).
pub const SEARCH_RESULTS_LIMIT: usize = 10;

/// Agent-pick callback prefix (TS: `"agent:"`).
pub const AGENT_CALLBACK_PREFIX: &str = "agent:";

/// Variant-pick callback prefix (TS: `"variant:"`).
pub const VARIANT_CALLBACK_PREFIX: &str = "variant:";

/// Model search entry callback (TS: `MODEL_SEARCH_CALLBACK`).
pub const MODEL_SEARCH_CALLBACK: &str = "model:search";

/// Model search-again callback (TS: `MODEL_SEARCH_AGAIN_CALLBACK`).
pub const MODEL_SEARCH_AGAIN_CALLBACK: &str = "model:search:again";

/// Model search-cancel callback (TS: `MODEL_SEARCH_CANCEL_CALLBACK`).
pub const MODEL_SEARCH_CANCEL_CALLBACK: &str = "model:search:cancel";

/// Favorites/recent list-pick prefix (TS: `MODEL_LIST_CALLBACK_PREFIX`).
pub const MODEL_LIST_CALLBACK_PREFIX: &str = "model:list:";

/// Providers-browser root callback (TS: `MODEL_ROOT_CALLBACK`).
pub const MODEL_ROOT_CALLBACK: &str = "model:root";

/// Providers-page prefix (TS: `MODEL_PROVIDERS_CALLBACK_PREFIX`).
pub const MODEL_PROVIDERS_CALLBACK_PREFIX: &str = "model:providers:";

/// Single-provider page prefix (TS: `MODEL_PROVIDER_CALLBACK_PREFIX`).
pub const MODEL_PROVIDER_CALLBACK_PREFIX: &str = "model:provider:";

/// Provider-model pick prefix (TS: `MODEL_PROVIDER_MODEL_CALLBACK_PREFIX`).
pub const MODEL_PROVIDER_MODEL_CALLBACK_PREFIX: &str = "model:pick:";

/// Search-result pick prefix (TS: `MODEL_SEARCH_RESULT_CALLBACK_PREFIX`).
pub const MODEL_SEARCH_RESULT_CALLBACK_PREFIX: &str = "model:result:";

// ---------------------------------------------------------------------------
// Shared shapes
// ---------------------------------------------------------------------------

/// Minimal provider/model reference (TS: `FavoriteModel`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub provider_id: String,
    pub model_id: String,
}

/// Provider row (TS: `ProviderInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRow {
    pub id: String,
    pub name: String,
    pub model_count: u32,
}

/// One selectable variant (TS: `VariantInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantEntry {
    pub id: String,
    pub disabled: bool,
}

/// A parsed model item with a concrete variant string
/// (TS: `ModelInfo` with the `parseModelItems` `"default"` fallback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModelItem {
    pub provider_id: String,
    pub model_id: String,
    pub variant: String,
}

/// A list/legacy pick: the variant always resets to `"default"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    pub provider_id: String,
    pub model_id: String,
    pub variant: String,
}

/// Clamped pagination window (TS: `calculateModelsPaginationRange`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelsPaginationRange {
    pub page: i64,
    pub total_pages: i64,
    pub start_index: i64,
    pub end_index: i64,
}

/// Assembled favorites + recent lists (TS: `getModelSelectionLists`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionLists {
    pub favorites: Vec<ModelRef>,
    pub recent: Vec<ModelRef>,
}

/// Model-list picker tab (TS: `ModelListKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelListKind {
    Favorites,
    Recent,
}

/// Catalog availability (TS: `ModelAvailability`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelAvailability {
    Offered,
    Unknown,
    ProviderMissing,
    ModelMissing,
}

/// `handleModelSelect` routing (TS: the search/provider-browser guards).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSelectRoute {
    Ignored,
    SearchFlow,
    ProviderBrowser,
    ListOrLegacy,
}

/// `handleModelProvidersCallback` dispatch (TS: root/page/provider/pick).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderBrowserRoute {
    Root,
    ProvidersPage(u64),
    ProviderModels { provider_index: u64, page: u64 },
    PickModel(u64),
    Unmatched,
}

/// `handleModelSearchResults` dispatch (TS: stage/message/cancel/again).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchResultsRoute {
    Ignored,
    StaleMessage,
    Cancel,
    SearchAgain,
    ResultIndex(u64),
    ShortInvalid,
    LegacyCandidate,
}

// ---------------------------------------------------------------------------
// Agent gates (`agent-selection-service.ts`, agent menu + callback handler)
// ---------------------------------------------------------------------------

/// Name after `agent:` (TS: the `startsWith("agent:")` guard + strip).
pub fn parse_agent_callback(data: &str) -> Option<String> {
    data.strip_prefix(AGENT_CALLBACK_PREFIX)
        .map(|rest| rest.to_string())
}

/// `agent:<name>` callback build.
pub fn build_agent_callback(name: &str) -> String {
    format!("{AGENT_CALLBACK_PREFIX}{name}")
}

/// Agent row label with `✅` mark for the current agent
/// (TS: `buildAgentSelectionMenu`; `display_name` is the caller-resolved
/// `getAgentDisplayName`).
pub fn agent_menu_button_label(display_name: &str, is_active: bool) -> String {
    if is_active {
        format!("✅ {display_name}")
    } else {
        display_name.to_string()
    }
}

/// Build-preferring fallback, else the first agent, else the default
/// (TS: `pickFallbackAgent`).
pub fn pick_fallback_agent(names: &[&str]) -> String {
    if names.contains(&DEFAULT_AGENT) {
        return DEFAULT_AGENT.to_string();
    }
    names
        .first()
        .map(|name| name.to_string())
        .unwrap_or_else(|| DEFAULT_AGENT.to_string())
}

/// Trimmed field or `None` when blank/missing (TS: `configuredField`).
pub fn configured_field(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Stored agent or the default (TS: `getStoredAgent`).
pub fn stored_agent_or_default(stored: Option<&str>) -> String {
    stored.unwrap_or(DEFAULT_AGENT).to_string()
}

/// `preferred ?? stored ?? default` (TS: `resolveProjectAgent` request).
pub fn requested_agent(preferred: Option<&str>, stored: Option<&str>) -> String {
    preferred
        .or(stored)
        .unwrap_or(DEFAULT_AGENT)
        .to_string()
}

/// A stored pick wins over a stale session agent
/// (TS: `fetchCurrentAgent`: `storedAgent && lastAgent !== storedAgent`).
pub fn should_prefer_stored_agent(stored: Option<&str>, last_agent: Option<&str>) -> bool {
    match stored {
        Some(stored_name) => Some(stored_name) != last_agent,
        None => false,
    }
}

/// Whether the requested agent is in the available list
/// (TS: `agents.some((agent) => agent.name === requestedAgent)`).
pub fn is_agent_listed(names: &[&str], requested: &str) -> bool {
    names.contains(&requested)
}

// ---------------------------------------------------------------------------
// Variant gates (`variant-selection-service.ts`, variant menu + handler)
// ---------------------------------------------------------------------------

fn capitalize_first(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = String::new();
            for upper in first.to_uppercase() {
                out.push(upper);
            }
            out.push_str(chars.as_str());
            out
        }
    }
}

/// Capitalized variant id (TS: `formatVariantForDisplay`).
pub fn format_variant_for_display(variant_id: &str) -> String {
    capitalize_first(variant_id)
}

/// `💡 Capitalized` button text (TS: `formatVariantForButton`).
pub fn format_variant_for_button(variant_id: &str) -> String {
    format!("💡 {}", capitalize_first(variant_id))
}

/// `variant:<id>` callback build.
pub fn build_variant_callback(variant_id: &str) -> String {
    format!("{VARIANT_CALLBACK_PREFIX}{variant_id}")
}

/// Id after `variant:` (TS: `.replace("variant:", "")`; empty stays empty).
pub fn parse_variant_callback(data: &str) -> Option<String> {
    data.strip_prefix(VARIANT_CALLBACK_PREFIX)
        .map(|rest| rest.to_string())
}

/// Both provider and model ids are set
/// (TS: the `!currentModel.providerID || !currentModel.modelID` notify gate).
pub fn has_model_selected(provider_id: Option<&str>, model_id: Option<&str>) -> bool {
    matches!(provider_id, Some(id) if !id.is_empty())
        && matches!(model_id, Some(id) if !id.is_empty())
}

/// The `✅ <default>` fallback row when no variant is active
/// (TS: `buildVariantSelectionMenu`; `display_default` is the
/// caller-resolved `formatVariantForDisplay("default")`).
pub fn default_variant_button_label(display_default: &str) -> String {
    format!("✅ {display_default}")
}

/// The follow-up menu opens only for a real choice
/// (TS: `drawnVariants < 2` skip in `showVariantSelectionMenuAfterModelChange`).
pub fn should_skip_variant_menu(drawn_variants: usize) -> bool {
    drawn_variants < 2
}

/// `default` first, API ids after it, duplicate `default` skipped, `disabled`
/// passed through (TS: `getAvailableVariants` list build; each entry is
/// `(id, disabled)`).
pub fn collect_variants(api: &[(&str, Option<bool>)]) -> Vec<VariantEntry> {
    let mut variants = vec![VariantEntry {
        id: "default".to_string(),
        disabled: false,
    }];
    for (id, disabled) in api {
        if *id == "default" {
            continue;
        }
        variants.push(VariantEntry {
            id: id.to_string(),
            disabled: disabled.unwrap_or(false),
        });
    }
    variants
}

/// A non-disabled entry carries the id (TS: `validateVariantForModel` find).
pub fn is_variant_usable(variants: &[VariantEntry], id: &str) -> bool {
    variants
        .iter()
        .any(|variant| variant.id == id && !variant.disabled)
}

/// Stored variant or `"default"` (TS: `getCurrentVariant`).
pub fn current_variant_or_default(variant: Option<&str>) -> String {
    match variant {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => "default".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Model catalog gates (`model-selection-service.ts`)
// ---------------------------------------------------------------------------

/// `provider/model` key (TS: `getModelKey`).
pub fn model_key(provider_id: &str, model_id: &str) -> String {
    format!("{provider_id}/{model_id}")
}

/// First occurrence wins (TS: `dedupeModels`).
pub fn dedupe_models(models: Vec<ModelRef>) -> Vec<ModelRef> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut unique = Vec::new();
    for model in models {
        let key = model_key(&model.provider_id, &model.model_id);
        if seen.insert(key) {
            unique.push(model);
        }
    }
    unique
}

/// Keep catalog-listed models; a missing catalog passes everything through
/// (TS: `filterModelsByCatalog`).
pub fn filter_models_by_catalog(
    models: Vec<ModelRef>,
    valid_keys: Option<&HashSet<String>>,
) -> Vec<ModelRef> {
    match valid_keys {
        None => models,
        Some(valid) => models
            .into_iter()
            .filter(|model| valid.contains(&model_key(&model.provider_id, &model.model_id)))
            .collect(),
    }
}

/// Case-insensitive substring match on the lowercased key, ordered, capped
/// (TS: `searchModels`).
pub fn search_models(models: &[ModelRef], query: &str) -> Vec<ModelRef> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut results: Vec<ModelRef> = models
        .iter()
        .filter(|model| {
            model_key(&model.provider_id, &model.model_id)
                .to_lowercase()
                .contains(&needle)
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

/// Error/empty/provider/model decision table (TS: `getModelAvailability`).
pub fn classify_model_availability(
    had_error: bool,
    listed_with_models: bool,
    provider_found: bool,
    model_offered: bool,
) -> ModelAvailability {
    if had_error {
        return ModelAvailability::Unknown;
    }
    if !listed_with_models {
        return ModelAvailability::Unknown;
    }
    if !provider_found {
        return ModelAvailability::ProviderMissing;
    }
    if !model_offered {
        return ModelAvailability::ModelMissing;
    }
    ModelAvailability::Offered
}

/// Offered or unknown adopts; a missing provider adopts only in warmup
/// (TS: `isModelAdoptable`).
pub fn is_model_adoptable(availability: ModelAvailability, warmup_active: bool) -> bool {
    match availability {
        ModelAvailability::Offered | ModelAvailability::Unknown => true,
        ModelAvailability::ProviderMissing => warmup_active,
        ModelAvailability::ModelMissing => false,
    }
}

/// Favorites get validated raws plus the validated env default; recent drops
/// favorite keys (TS: `getModelSelectionLists` assembly).
pub fn assemble_selection_lists(
    raw_favorites: Vec<ModelRef>,
    raw_recent: Vec<ModelRef>,
    env_default: Option<ModelRef>,
    valid_keys: Option<&HashSet<String>>,
) -> SelectionLists {
    let validated_favorites = filter_models_by_catalog(raw_favorites, valid_keys);
    let validated_recent = filter_models_by_catalog(raw_recent, valid_keys);
    let offered_default =
        filter_models_by_catalog(env_default.into_iter().collect(), valid_keys);

    let favorites =
        dedupe_models([validated_favorites, offered_default].concat());
    let favorite_keys: HashSet<String> = favorites
        .iter()
        .map(|model| model_key(&model.provider_id, &model.model_id))
        .collect();
    let recent = dedupe_models(validated_recent)
        .into_iter()
        .filter(|model| !favorite_keys.contains(&model_key(&model.provider_id, &model.model_id)))
        .collect();

    SelectionLists { favorites, recent }
}

// ---------------------------------------------------------------------------
// Model menu gates (`model-selection-menu.ts`)
// ---------------------------------------------------------------------------

/// `model:list:<kind>:<index>` build (TS: `buildModelListCallback`).
pub fn build_model_list_callback(kind: ModelListKind, index: usize) -> String {
    let kind_text = match kind {
        ModelListKind::Favorites => "favorites",
        ModelListKind::Recent => "recent",
    };
    format!("{MODEL_LIST_CALLBACK_PREFIX}{kind_text}:{index}")
}

/// Digits-only non-negative parse (TS: the private `parseIndex`).
pub fn parse_non_negative_index(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok()
}

/// Providers-page parse (TS: `parseProvidersPageCallback`).
pub fn parse_providers_page_callback(data: &str) -> Option<u64> {
    let rest = data.strip_prefix(MODEL_PROVIDERS_CALLBACK_PREFIX)?;
    parse_non_negative_index(rest)
}

/// Exactly-two-segment provider/page parse (TS: `parseProviderCallback`).
pub fn parse_provider_callback(data: &str) -> Option<(u64, u64)> {
    let rest = data.strip_prefix(MODEL_PROVIDER_CALLBACK_PREFIX)?;
    let mut parts = rest.split(':');
    let provider_text = parts.next()?;
    let page_text = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some((
        parse_non_negative_index(provider_text)?,
        parse_non_negative_index(page_text)?,
    ))
}

/// Provider-model index parse (TS: `parseProviderModelCallback`).
pub fn parse_provider_model_callback(data: &str) -> Option<u64> {
    let rest = data.strip_prefix(MODEL_PROVIDER_MODEL_CALLBACK_PREFIX)?;
    parse_non_negative_index(rest)
}

/// First-page bounds, last-page clamp, page-size floor of 1, and the
/// always-at-least-one page for an empty catalog
/// (TS: `calculateModelsPaginationRange`).
pub fn calculate_models_pagination_range(
    total_items: i64,
    page: i64,
    page_size: i64,
) -> ModelsPaginationRange {
    let safe_page_size = page_size.max(1);
    // Stable manual ceil-div: `i64::div_ceil` needs nightly `int_roundings`
    // on this toolchain, so guard non-positive totals (empty catalog maps
    // to zero pages, then to exactly one page below).
    let pages_by_items = if total_items <= 0 {
        0
    } else {
        (total_items + safe_page_size - 1) / safe_page_size
    };
    let total_pages = pages_by_items.max(1);
    let normalized_page = page.clamp(0, total_pages - 1);
    let start_index = normalized_page * safe_page_size;
    let end_index = (start_index + safe_page_size).min(total_items).max(0);
    ModelsPaginationRange {
        page: normalized_page,
        total_pages,
        start_index,
        end_index,
    }
}

/// `"name (count)"` provider row (TS: `buildProvidersMenuView` label).
pub fn provider_row_label(name: &str, model_count: u32) -> String {
    format!("{name} ({model_count})")
}

/// `model:provider:<index>:0` callback build.
pub fn provider_callback(provider_index: usize) -> String {
    format!("{MODEL_PROVIDER_CALLBACK_PREFIX}{provider_index}:0")
}

/// `model:providers:<page>` callback build.
pub fn providers_page_callback(page: usize) -> String {
    format!("{MODEL_PROVIDERS_CALLBACK_PREFIX}{page}")
}

/// `model:provider:<providerIndex>:<page>` callback build.
pub fn provider_models_page_callback(provider_index: usize, page: usize) -> String {
    format!("{MODEL_PROVIDER_CALLBACK_PREFIX}{provider_index}:{page}")
}

/// Plain model id, `✅`-marked when current
/// (TS: `buildProviderModelsMenuView` label).
pub fn provider_model_button_label(model_id: &str, is_active: bool) -> String {
    if is_active {
        format!("✅ {model_id}")
    } else {
        model_id.to_string()
    }
}

/// `"⭐ provider/model"` / `"🕘 provider/model"` with `✅` mark when current
/// (TS: `buildModelSelectionMenu` `addButton`).
pub fn model_list_button_label(
    provider_id: &str,
    model_id: &str,
    prefix: &str,
    is_active: bool,
) -> String {
    let label = format!("{prefix} {provider_id}/{model_id}");
    if is_active {
        format!("✅ {label}")
    } else {
        label
    }
}

// ---------------------------------------------------------------------------
// Model callback-handler gates (`model-selection-callback-handler.ts`,
// agent/variant handler guards)
// ---------------------------------------------------------------------------

/// Object-with-string-ids shape with `"default"` variant fallback
/// (TS: `parseModelItems` entry gate).
pub fn parse_model_item(
    provider_id: Option<&str>,
    model_id: Option<&str>,
    variant: Option<&str>,
) -> Option<ParsedModelItem> {
    let provider_id = provider_id?;
    let model_id = model_id?;
    Some(ParsedModelItem {
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
        variant: variant.unwrap_or("default").to_string(),
    })
}

/// Object-with-string-id/name shape with `0` count fallback
/// (TS: `parseProviderItems` entry gate).
pub fn parse_provider_item(
    id: Option<&str>,
    name: Option<&str>,
    model_count: Option<u32>,
) -> Option<ProviderRow> {
    let id = id?;
    let name = name?;
    Some(ProviderRow {
        id: id.to_string(),
        name: name.to_string(),
        model_count: model_count.unwrap_or(0),
    })
}

/// Non-negative index after a result prefix
/// (TS: `parseCallbackIndex`).
pub fn parse_callback_index(data: &str, prefix: &str) -> Option<u64> {
    let rest = data.strip_prefix(prefix)?;
    parse_non_negative_index(rest)
}

/// `model:list:<kind>:<index>` pick with kind/index/bounds gates; the
/// variant resets to `"default"` (TS: `resolveModelListCallback`).
pub fn resolve_model_list_callback(
    data: &str,
    favorites: &[ModelRef],
    recent: &[ModelRef],
) -> Option<ResolvedModel> {
    let rest = data.strip_prefix(MODEL_LIST_CALLBACK_PREFIX)?;
    let mut parts = rest.split(':');
    let kind = parts.next()?;
    let index_text = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if kind.is_empty() || index_text.is_empty() {
        return None;
    }
    let index = parse_non_negative_index(index_text)?;
    let model = match kind {
        "favorites" => favorites.get(index as usize)?,
        "recent" => recent.get(index as usize)?,
        _ => return None,
    };
    Some(ResolvedModel {
        provider_id: model.provider_id.clone(),
        model_id: model.model_id.clone(),
        variant: "default".to_string(),
    })
}

/// `provider:model...` colon tail join with non-empty gates
/// (TS: `parseLegacyModelCallback`).
pub fn parse_legacy_model_callback(data: &str) -> Option<ResolvedModel> {
    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let provider_id = parts[1];
    let model_id = parts[2..].join(":");
    if provider_id.is_empty() || model_id.is_empty() {
        return None;
    }
    Some(ResolvedModel {
        provider_id: provider_id.to_string(),
        model_id,
        variant: "default".to_string(),
    })
}

/// Search-result, list, or provider-browser callback
/// (TS: `isShortModelCallback`).
pub fn is_short_model_callback(data: &str) -> bool {
    data.starts_with(MODEL_SEARCH_RESULT_CALLBACK_PREFIX)
        || data.starts_with(MODEL_LIST_CALLBACK_PREFIX)
        || is_provider_browser_callback(data)
}

/// Root, providers, provider, or pick callback
/// (TS: `isProviderBrowserCallback`).
pub fn is_provider_browser_callback(data: &str) -> bool {
    data == MODEL_ROOT_CALLBACK
        || data.starts_with(MODEL_PROVIDERS_CALLBACK_PREFIX)
        || data.starts_with(MODEL_PROVIDER_CALLBACK_PREFIX)
        || data.starts_with(MODEL_PROVIDER_MODEL_CALLBACK_PREFIX)
}

/// `handleModelSelect` skip routing: search callbacks and provider-browser
/// callbacks belong to other handlers (TS: the two guard blocks).
pub fn route_model_select_callback(data: &str) -> ModelSelectRoute {
    if !data.starts_with("model:") {
        return ModelSelectRoute::Ignored;
    }
    if data == MODEL_SEARCH_CALLBACK
        || data == MODEL_SEARCH_AGAIN_CALLBACK
        || data == MODEL_SEARCH_CANCEL_CALLBACK
    {
        return ModelSelectRoute::SearchFlow;
    }
    if is_provider_browser_callback(data) {
        return ModelSelectRoute::ProviderBrowser;
    }
    ModelSelectRoute::ListOrLegacy
}

/// `handleModelProvidersCallback` dispatch order: root, providers page,
/// provider page, model pick (TS: the `if` chain).
pub fn route_provider_browser_callback(data: &str) -> ProviderBrowserRoute {
    if data == MODEL_ROOT_CALLBACK {
        return ProviderBrowserRoute::Root;
    }
    if let Some(page) = parse_providers_page_callback(data) {
        return ProviderBrowserRoute::ProvidersPage(page);
    }
    if let Some((provider_index, page)) = parse_provider_callback(data) {
        return ProviderBrowserRoute::ProviderModels {
            provider_index,
            page,
        };
    }
    if let Some(index) = parse_provider_model_callback(data) {
        return ProviderBrowserRoute::PickModel(index);
    }
    ProviderBrowserRoute::Unmatched
}

/// `handleModelSearchResults` dispatch: results stage, message match,
/// cancel, again, result index, legacy tail (TS: the guard chain).
pub fn route_search_results_callback(
    data: &str,
    is_results_stage: bool,
    message_id_matches: bool,
) -> SearchResultsRoute {
    if !is_results_stage {
        return SearchResultsRoute::Ignored;
    }
    if !message_id_matches {
        return SearchResultsRoute::StaleMessage;
    }
    if data == MODEL_SEARCH_CANCEL_CALLBACK {
        return SearchResultsRoute::Cancel;
    }
    if data == MODEL_SEARCH_AGAIN_CALLBACK {
        return SearchResultsRoute::SearchAgain;
    }
    if let Some(index) = parse_callback_index(data, MODEL_SEARCH_RESULT_CALLBACK_PREFIX) {
        return SearchResultsRoute::ResultIndex(index);
    }
    if data.starts_with("model:") {
        if is_short_model_callback(data) {
            return SearchResultsRoute::ShortInvalid;
        }
        return SearchResultsRoute::LegacyCandidate;
    }
    SearchResultsRoute::Ignored
}
