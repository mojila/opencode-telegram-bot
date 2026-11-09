//! Project / model / agent persist gates (slice 173, GREEN).
//!
//! Rust port of the pure remainder behind the persist slice:
//! - `src/app/stores/settings-store.ts` (`currentProject` write + session
//!   reset, `currentModel` / `currentAgent` writes are I/O in
//!   `otb-bot::settings_current` and stay there)
//! - `src/app/services/model-selection-service.ts`
//!   (`getOpenCodeModelStatePath`, favorite/recent blank gates,
//!   `getStoredModel` fallback, `reconcileStoredModelSelection` replace gate,
//!   `resolveModelToAdopt` candidate/env choice)
//! - `src/app/services/agent-selection-service.ts`
//!   (`applyAgentConfiguredSettings` configured-field + agent-model variant
//!   choice)
//! - `src/app/services/variant-selection-service.ts` (`setCurrentVariant`
//!   no-model gate is I/O-side; the pure choice lives here)
//!
//! No filesystem reads, no OpenCode clients, no `logger` calls live here —
//! only path derivation, entry filtering, and selection-choice predicates.
//! Callers resolve `t(...)` locales, catalog rows, and keyboards.
//!
//! DEVIATIONS from TS:
//! 1. `path.join` follows the host separator in TS. The port joins with
//!    `std::path::PathBuf::join` (same observable shape on posix fixtures,
//!    correct separators on Windows).
//! 2. TS `getStoredModel` returns an empty-provider/model record when
//!    neither settings nor env names a model. The port returns `None`
//!    instead so callers cannot persist a blank identity; the empty record
//!    is a display-only fallback owned by `telegram.rs`.
//! 3. `readProvidersWhenListed` waiting, TTL caching, and warmup timers are
//!    impure and stay with the caller; availability arrives here as an
//!    already-computed [`crate::agent_model_variant_gates::ModelAvailability`].

use std::path::PathBuf;

use crate::agent_model_variant_gates::{
    configured_field, current_variant_or_default, is_model_adoptable, ModelAvailability,
};

/// One model identity without a variant (TS: `FavoriteModel`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentity {
    pub provider_id: String,
    pub model_id: String,
}

/// Effective model with a concrete variant string (TS: `ModelInfo` with the
/// `"default"` fallback from `getStoredModel`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveModel {
    pub provider_id: String,
    pub model_id: String,
    pub variant: String,
}

/// What `resolveModelToAdopt` settled on (TS: the model or `null`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdoptChoice {
    /// The candidate itself is adoptable.
    Candidate,
    /// The candidate is gone; the configured default is adoptable.
    EnvDefault,
    /// Neither is adoptable; leave the selection as it is.
    Keep,
}

/// OpenCode local model-state path (TS: `getOpenCodeModelStatePath`).
///
/// `XDG_STATE_HOME` (non-blank) wins as `<xdg>/opencode/model.json`; else
/// `<home>/.local/state/opencode/model.json`. `home` is `HOME` or
/// `USERPROFILE` (already resolved by the caller); a missing/blank home
/// falls back to a relative `.local/state/...` join. Pure; no env reads.
pub fn model_state_file_path(home: Option<&str>, xdg_state_home: Option<&str>) -> PathBuf {
    if let Some(xdg) = xdg_state_home
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(xdg).join("opencode").join("model.json");
    }
    match home.map(str::trim).filter(|value| !value.is_empty()) {
        Some(home) => PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("opencode")
            .join("model.json"),
        None => PathBuf::from(".local")
            .join("state")
            .join("opencode")
            .join("model.json"),
    }
}

/// One favorite/recent entry gate (TS: the `filter` in
/// `normalizeFavoriteModels` / `normalizeRecentModels`): both ids must be
/// present strings with length > 0. Whitespace-only strings pass here (TS
/// `length > 0`) and are pruned later by the catalog key match.
pub fn normalize_model_state_entry(
    provider_id: Option<&str>,
    model_id: Option<&str>,
) -> Option<ModelIdentity> {
    let provider_id = provider_id?;
    let model_id = model_id?;
    if provider_id.is_empty() || model_id.is_empty() {
        return None;
    }
    Some(ModelIdentity {
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
    })
}

/// Filter a raw favorite/recent list to valid identities, order preserved
/// (TS: `normalizeFavoriteModels` / `normalizeRecentModels` minus I/O).
pub fn normalize_model_state_entries(raw: &[(Option<&str>, Option<&str>)]) -> Vec<ModelIdentity> {
    raw.iter()
        .filter_map(|(provider_id, model_id)| normalize_model_state_entry(*provider_id, *model_id))
        .collect()
}

/// Effective model from settings or env (TS: `getStoredModel` minus the
/// empty-record fallback): stored wins with `variant || "default"`, else
/// the env default with `"default"`, else `None` (leave the selection).
pub fn stored_model_or_env_default(
    stored: Option<(&str, &str, Option<&str>)>,
    env_default: Option<(&str, &str)>,
) -> Option<EffectiveModel> {
    if let Some((provider_id, model_id, variant)) = stored {
        if !provider_id.is_empty() && !model_id.is_empty() {
            return Some(EffectiveModel {
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                variant: current_variant_or_default(variant),
            });
        }
    }
    if let Some((provider_id, model_id)) = env_default {
        if !provider_id.is_empty() && !model_id.is_empty() {
            return Some(EffectiveModel {
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                variant: "default".to_string(),
            });
        }
    }
    None
}

/// Whether the stored pick is replaced by the configured default (TS:
/// `reconcileStoredModelSelection` tail): only when a fresh, complete
/// catalog was read, the selected key is absent, and the env default is
/// present and offered. Every other combination keeps the selection.
pub fn should_replace_stored_model(
    catalog_available: bool,
    catalog_complete: bool,
    selected_listed: bool,
    env_default_offered: bool,
) -> bool {
    catalog_available && catalog_complete && !selected_listed && env_default_offered
}

/// Candidate-vs-env choice (TS: `resolveModelToAdopt`): an adoptable
/// candidate wins outright; otherwise the env default wins when it names a
/// different key and is adoptable; otherwise the selection stays.
pub fn resolve_model_to_adopt_choice(
    candidate: ModelAvailability,
    env: Option<ModelAvailability>,
    warmup_active: bool,
    same_key: bool,
) -> AdoptChoice {
    if is_model_adoptable(candidate, warmup_active) {
        return AdoptChoice::Candidate;
    }
    if !same_key {
        if let Some(env_availability) = env {
            if is_model_adoptable(env_availability, warmup_active) {
                return AdoptChoice::EnvDefault;
            }
        }
    }
    AdoptChoice::Keep
}

/// Variant to store when an agent brings its own model (TS:
/// `applyAgentConfiguredSettings`): the agent's configured variant when the
/// adopted model is the agent's own, else `"default"`. Blank configured and
/// stored variants fall back to `"default"` (TS `?? getStoredModel().variant
/// ?? "default"` with `configuredField` trim semantics).
pub fn resolve_agent_applied_variant(
    is_agent_model: bool,
    configured_variant: Option<&str>,
    stored_variant: Option<&str>,
) -> String {
    if !is_agent_model {
        return "default".to_string();
    }
    configured_field(configured_variant)
        .or_else(|| configured_field(stored_variant))
        .unwrap_or_else(|| "default".to_string())
}

/// Whether an agent tap with no model still applies its variant (TS: the
/// `if (variant)` tail of `applyAgentConfiguredSettings`).
pub fn should_apply_agent_variant_only(has_model: bool, variant_present: bool) -> bool {
    !has_model && variant_present
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_state_path_prefers_xdg_state_home() {
        let path = model_state_file_path(Some("/home/u"), Some("  /state  "));
        assert_eq!(
            path,
            PathBuf::from("/state").join("opencode").join("model.json")
        );
    }

    #[test]
    fn model_state_path_falls_back_to_home_local_state() {
        let path = model_state_file_path(Some("/home/u"), None);
        assert_eq!(
            path,
            PathBuf::from("/home/u")
                .join(".local")
                .join("state")
                .join("opencode")
                .join("model.json")
        );
    }

    #[test]
    fn model_state_entries_drop_blanks_but_keep_whitespace() {
        let raw = [
            (Some("openai"), Some("gpt")),
            (Some(""), Some("gpt")),
            (Some("openai"), None),
            (Some("  "), Some("gpt")),
        ];
        let entries = normalize_model_state_entries(&raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].provider_id, "openai");
        assert_eq!(entries[1].provider_id, "  ");
    }

    #[test]
    fn stored_model_prefers_settings_then_env_then_none() {
        let stored =
            stored_model_or_env_default(Some(("p", "m", Some("low"))), Some(("env-p", "env-m")));
        assert_eq!(
            stored,
            Some(EffectiveModel {
                provider_id: "p".to_string(),
                model_id: "m".to_string(),
                variant: "low".to_string(),
            })
        );
        let fallback = stored_model_or_env_default(None, Some(("env-p", "env-m")));
        assert_eq!(
            fallback,
            Some(EffectiveModel {
                provider_id: "env-p".to_string(),
                model_id: "env-m".to_string(),
                variant: "default".to_string(),
            })
        );
        assert_eq!(stored_model_or_env_default(None, None), None);
    }

    #[test]
    fn replace_gate_needs_complete_catalog_and_offered_default() {
        assert!(should_replace_stored_model(true, true, false, true));
        assert!(!should_replace_stored_model(false, true, false, true));
        assert!(!should_replace_stored_model(true, false, false, true));
        assert!(!should_replace_stored_model(true, true, true, true));
        assert!(!should_replace_stored_model(true, true, false, false));
    }

    #[test]
    fn adopt_choice_prefers_candidate_then_env_then_keep() {
        assert_eq!(
            resolve_model_to_adopt_choice(ModelAvailability::Offered, None, false, false),
            AdoptChoice::Candidate
        );
        assert_eq!(
            resolve_model_to_adopt_choice(
                ModelAvailability::ModelMissing,
                Some(ModelAvailability::Offered),
                false,
                false
            ),
            AdoptChoice::EnvDefault
        );
        assert_eq!(
            resolve_model_to_adopt_choice(
                ModelAvailability::ModelMissing,
                Some(ModelAvailability::Offered),
                false,
                true
            ),
            AdoptChoice::Keep
        );
        assert_eq!(
            resolve_model_to_adopt_choice(ModelAvailability::ModelMissing, None, false, false),
            AdoptChoice::Keep
        );
    }

    #[test]
    fn agent_variant_follows_agent_model_else_default() {
        assert_eq!(
            resolve_agent_applied_variant(true, Some("high"), Some("low")),
            "high"
        );
        assert_eq!(
            resolve_agent_applied_variant(true, None, Some("low")),
            "low"
        );
        assert_eq!(resolve_agent_applied_variant(true, None, None), "default");
        assert_eq!(
            resolve_agent_applied_variant(false, Some("high"), Some("low")),
            "default"
        );
    }

    #[test]
    fn variant_only_applies_without_model() {
        assert!(should_apply_agent_variant_only(false, true));
        assert!(!should_apply_agent_variant_only(true, true));
        assert!(!should_apply_agent_variant_only(false, false));
    }
}
