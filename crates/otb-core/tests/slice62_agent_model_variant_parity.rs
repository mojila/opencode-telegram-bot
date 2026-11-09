//! Parity port of the agent/model/variant selection pure gates (slice 62, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/app/services/agent-selection-service.ts` (default agent, fallback
//!   pick, configured-field trim, stored/requested resolution, stale-session
//!   preference, availability membership)
//! - `src/app/services/variant-selection-service.ts` (display/button
//!   formats, `variant:` build/parse, model-selected gate, default fallback
//!   label, `< 2` skip gate, variant-list build, usability, current default)
//! - `src/app/services/model-selection-service.ts` (search limit, model key,
//!   dedupe, catalog filter, search match/sort/cap, availability table,
//!   adoptability, favorites/recent assembly)
//! - `src/bot/menus/agent-selection-menu.ts` (`agent:` build, active label)
//! - `src/bot/menus/variant-selection-menu.ts` (default fallback row)
//! - `src/bot/menus/model-selection-menu.ts` (prefixes, list-callback build,
//!   page/provider/pick parses, pagination range, row labels, callback
//!   builders)
//! - `src/bot/callbacks/agent-selection-callback-handler.ts` (`agent:`
//!   prefix guard)
//! - `src/bot/callbacks/variant-selection-callback-handler.ts` (`variant:`
//!   prefix guard, model-not-selected gate)
//! - `src/bot/callbacks/model-selection-callback-handler.ts` (model/provider
//!   item shapes, result-index parse, list resolve, legacy parse, short and
//!   provider-browser classification, select/provider/search-results
//!   dispatch)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::agent_model_variant_gates::*;
use std::collections::HashSet;

fn m(provider_id: &str, model_id: &str) -> ModelRef {
    ModelRef {
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
    }
}

fn key_set(keys: &[&str]) -> HashSet<String> {
    keys.iter().map(|k| k.to_string()).collect()
}

// --- constants (10) ---

#[test]
fn default_agent_is_build() {
    assert_eq!(DEFAULT_AGENT, "build");
}

#[test]
fn search_results_limit_is_10() {
    assert_eq!(SEARCH_RESULTS_LIMIT, 10);
}

#[test]
fn agent_and_variant_prefixes() {
    assert_eq!(AGENT_CALLBACK_PREFIX, "agent:");
    assert_eq!(VARIANT_CALLBACK_PREFIX, "variant:");
}

#[test]
fn model_search_callbacks() {
    assert_eq!(MODEL_SEARCH_CALLBACK, "model:search");
    assert_eq!(MODEL_SEARCH_AGAIN_CALLBACK, "model:search:again");
    assert_eq!(MODEL_SEARCH_CANCEL_CALLBACK, "model:search:cancel");
}

#[test]
fn model_list_prefix() {
    assert_eq!(MODEL_LIST_CALLBACK_PREFIX, "model:list:");
}

#[test]
fn model_root_callback() {
    assert_eq!(MODEL_ROOT_CALLBACK, "model:root");
}

#[test]
fn model_providers_prefix() {
    assert_eq!(MODEL_PROVIDERS_CALLBACK_PREFIX, "model:providers:");
}

#[test]
fn model_provider_prefix() {
    assert_eq!(MODEL_PROVIDER_CALLBACK_PREFIX, "model:provider:");
}

#[test]
fn model_pick_prefix() {
    assert_eq!(MODEL_PROVIDER_MODEL_CALLBACK_PREFIX, "model:pick:");
}

#[test]
fn model_search_result_prefix() {
    assert_eq!(MODEL_SEARCH_RESULT_CALLBACK_PREFIX, "model:result:");
}

// --- agent gates (8) ---

#[test]
fn parse_agent_callback_ok() {
    assert_eq!(
        parse_agent_callback("agent:build"),
        Some("build".to_string())
    );
}

#[test]
fn parse_agent_callback_rejects_without_prefix() {
    assert_eq!(parse_agent_callback("model:search"), None);
    assert_eq!(parse_agent_callback(""), None);
    assert_eq!(parse_agent_callback("xagent:build"), None);
}

#[test]
fn build_agent_callback_roundtrip() {
    assert_eq!(build_agent_callback("plan"), "agent:plan");
    assert_eq!(
        parse_agent_callback(&build_agent_callback("plan")),
        Some("plan".to_string())
    );
}

#[test]
fn agent_menu_button_label_marks_active() {
    assert_eq!(agent_menu_button_label("🛠️ Build", false), "🛠️ Build");
    assert_eq!(agent_menu_button_label("🛠️ Build", true), "✅ 🛠️ Build");
}

#[test]
fn pick_fallback_agent_parity() {
    assert_eq!(pick_fallback_agent(&["plan", "build", "general"]), "build");
    assert_eq!(pick_fallback_agent(&["plan", "general"]), "plan");
    assert_eq!(pick_fallback_agent(&[]), "build");
}

#[test]
fn configured_field_parity() {
    assert_eq!(
        configured_field(Some("  openai  ")),
        Some("openai".to_string())
    );
    assert_eq!(configured_field(Some("   ")), None);
    assert_eq!(configured_field(None), None);
}

#[test]
fn stored_and_requested_agent_resolution() {
    assert_eq!(stored_agent_or_default(Some("plan")), "plan");
    assert_eq!(stored_agent_or_default(None), "build");
    assert_eq!(requested_agent(Some("explore"), Some("plan")), "explore");
    assert_eq!(requested_agent(None, Some("plan")), "plan");
    assert_eq!(requested_agent(None, None), "build");
}

#[test]
fn stored_agent_preference_and_listing() {
    assert!(should_prefer_stored_agent(Some("plan"), Some("build")));
    assert!(!should_prefer_stored_agent(Some("plan"), Some("plan")));
    assert!(!should_prefer_stored_agent(None, Some("build")));
    assert!(is_agent_listed(&["plan", "build"], "build"));
    assert!(!is_agent_listed(&["plan"], "build"));
}

// --- variant gates (8) ---

#[test]
fn format_variant_for_display_parity() {
    assert_eq!(format_variant_for_display("default"), "Default");
    assert_eq!(format_variant_for_display("high"), "High");
    assert_eq!(format_variant_for_display(""), "");
}

#[test]
fn format_variant_for_button_parity() {
    assert_eq!(format_variant_for_button("default"), "💡 Default");
    assert_eq!(format_variant_for_button("low"), "💡 Low");
}

#[test]
fn parse_variant_callback_roundtrip() {
    assert_eq!(build_variant_callback("high"), "variant:high");
    assert_eq!(
        parse_variant_callback("variant:high"),
        Some("high".to_string())
    );
    assert_eq!(parse_variant_callback("variant:"), Some(String::new()));
    assert_eq!(parse_variant_callback("model:search"), None);
}

#[test]
fn has_model_selected_parity() {
    assert!(has_model_selected(Some("openai"), Some("gpt")));
    assert!(!has_model_selected(None, Some("gpt")));
    assert!(!has_model_selected(Some("openai"), None));
    assert!(!has_model_selected(Some(""), Some("gpt")));
}

#[test]
fn default_variant_button_label_parity() {
    assert_eq!(default_variant_button_label("Default"), "✅ Default");
}

#[test]
fn should_skip_variant_menu_parity() {
    assert!(should_skip_variant_menu(0));
    assert!(should_skip_variant_menu(1));
    assert!(!should_skip_variant_menu(2));
}

#[test]
fn collect_variants_parity() {
    let built = collect_variants(&[("low", Some(false)), ("high", Some(true))]);
    assert_eq!(
        built,
        vec![
            VariantEntry {
                id: "default".to_string(),
                disabled: false
            },
            VariantEntry {
                id: "low".to_string(),
                disabled: false
            },
            VariantEntry {
                id: "high".to_string(),
                disabled: true
            },
        ]
    );
    // A duplicate `default` from the API is skipped.
    let dup = collect_variants(&[("default", Some(false)), ("low", None)]);
    assert_eq!(
        dup,
        vec![
            VariantEntry {
                id: "default".to_string(),
                disabled: false
            },
            VariantEntry {
                id: "low".to_string(),
                disabled: false
            },
        ]
    );
}

#[test]
fn variant_usable_and_current_default() {
    let variants = collect_variants(&[("low", Some(false)), ("high", Some(true))]);
    assert!(is_variant_usable(&variants, "low"));
    assert!(!is_variant_usable(&variants, "high"));
    assert!(!is_variant_usable(&variants, "missing"));
    assert_eq!(current_variant_or_default(Some("high")), "high");
    assert_eq!(current_variant_or_default(None), "default");
    assert_eq!(current_variant_or_default(Some("")), "default");
}

// --- model catalog gates (8) ---

#[test]
fn model_key_parity() {
    assert_eq!(model_key("openai", "gpt"), "openai/gpt");
}

#[test]
fn dedupe_models_first_wins() {
    let out = dedupe_models(vec![m("a", "x"), m("a", "x"), m("b", "y")]);
    assert_eq!(out, vec![m("a", "x"), m("b", "y")]);
}

#[test]
fn filter_models_by_catalog_parity() {
    let models = vec![m("a", "x"), m("b", "y")];
    assert_eq!(filter_models_by_catalog(models.clone(), None), models);
    let valid = key_set(&["a/x"]);
    assert_eq!(
        filter_models_by_catalog(models, Some(&valid)),
        vec![m("a", "x")]
    );
}

#[test]
fn search_models_match_sort_limit() {
    let models = vec![m("b", "zebra"), m("a", "apple"), m("a", "apricot")];
    let out = search_models(&models, "  AP ");
    assert_eq!(out, vec![m("a", "apple"), m("a", "apricot")]);
    let many: Vec<ModelRef> = (0..20).map(|i| m("p", &format!("m{i:02}"))).collect();
    assert_eq!(search_models(&many, "p/m").len(), SEARCH_RESULTS_LIMIT);
}

#[test]
fn search_models_blank_query() {
    let models = vec![m("a", "x")];
    assert!(search_models(&models, "   ").is_empty());
}

#[test]
fn classify_model_availability_parity() {
    assert_eq!(
        classify_model_availability(true, true, true, true),
        ModelAvailability::Unknown
    );
    assert_eq!(
        classify_model_availability(false, false, false, false),
        ModelAvailability::Unknown
    );
    assert_eq!(
        classify_model_availability(false, true, false, false),
        ModelAvailability::ProviderMissing
    );
    assert_eq!(
        classify_model_availability(false, true, true, false),
        ModelAvailability::ModelMissing
    );
    assert_eq!(
        classify_model_availability(false, true, true, true),
        ModelAvailability::Offered
    );
}

#[test]
fn is_model_adoptable_parity() {
    assert!(is_model_adoptable(ModelAvailability::Offered, false));
    assert!(is_model_adoptable(ModelAvailability::Unknown, false));
    assert!(!is_model_adoptable(
        ModelAvailability::ProviderMissing,
        false
    ));
    assert!(is_model_adoptable(ModelAvailability::ProviderMissing, true));
    assert!(!is_model_adoptable(ModelAvailability::ModelMissing, true));
}

#[test]
fn assemble_selection_lists_parity() {
    let valid = key_set(&["a/x", "b/y", "env/z"]);
    let lists = assemble_selection_lists(
        vec![m("a", "x")],
        vec![m("a", "x"), m("b", "y")],
        Some(m("env", "z")),
        Some(&valid),
    );
    assert_eq!(lists.favorites, vec![m("a", "x"), m("env", "z")]);
    // Recent drops models already in favorites.
    assert_eq!(lists.recent, vec![m("b", "y")]);
}

// --- model menu gates (7) ---

#[test]
fn build_model_list_callback_parity() {
    assert_eq!(
        build_model_list_callback(ModelListKind::Favorites, 2),
        "model:list:favorites:2"
    );
    assert_eq!(
        build_model_list_callback(ModelListKind::Recent, 0),
        "model:list:recent:0"
    );
}

#[test]
fn parse_non_negative_index_parity() {
    assert_eq!(parse_non_negative_index("12"), Some(12));
    assert_eq!(parse_non_negative_index("0"), Some(0));
    assert_eq!(parse_non_negative_index(""), None);
    assert_eq!(parse_non_negative_index("1a"), None);
    assert_eq!(parse_non_negative_index("-1"), None);
    assert_eq!(parse_non_negative_index(" 3"), None);
}

#[test]
fn parse_providers_and_provider_callbacks() {
    assert_eq!(parse_providers_page_callback("model:providers:2"), Some(2));
    assert_eq!(parse_providers_page_callback("model:provider:0:1"), None);
    assert_eq!(parse_providers_page_callback("model:providers:x"), None);
    assert_eq!(parse_provider_callback("model:provider:3:1"), Some((3, 1)));
    assert_eq!(parse_provider_callback("model:provider:3"), None);
    assert_eq!(parse_provider_callback("model:provider:3:1:9"), None);
    assert_eq!(parse_provider_callback("model:provider:a:1"), None);
    assert_eq!(parse_provider_model_callback("model:pick:4"), Some(4));
    assert_eq!(parse_provider_model_callback("model:pick:"), None);
    // `providers:` is not shadowed by the `provider:` prefix.
    assert!(!"model:providers:0".starts_with(MODEL_PROVIDER_CALLBACK_PREFIX));
}

#[test]
fn calculate_models_pagination_range_parity() {
    let first = calculate_models_pagination_range(25, 0, 10);
    assert_eq!(
        first,
        ModelsPaginationRange {
            page: 0,
            total_pages: 3,
            start_index: 0,
            end_index: 10
        }
    );
    let clamped = calculate_models_pagination_range(25, 99, 10);
    assert_eq!(clamped.page, 2);
    assert_eq!((clamped.start_index, clamped.end_index), (20, 25));
    let empty = calculate_models_pagination_range(0, 0, 10);
    assert_eq!(
        empty,
        ModelsPaginationRange {
            page: 0,
            total_pages: 1,
            start_index: 0,
            end_index: 0
        }
    );
}

#[test]
fn provider_row_label_and_callback_builders() {
    assert_eq!(provider_row_label("OpenAI", 12), "OpenAI (12)");
    assert_eq!(provider_callback(3), "model:provider:3:0");
    assert_eq!(providers_page_callback(2), "model:providers:2");
    assert_eq!(provider_models_page_callback(3, 1), "model:provider:3:1");
}

#[test]
fn provider_model_button_label_parity() {
    assert_eq!(provider_model_button_label("gpt", false), "gpt");
    assert_eq!(provider_model_button_label("gpt", true), "✅ gpt");
}

#[test]
fn model_list_button_label_parity() {
    assert_eq!(model_list_button_label("a", "x", "⭐", false), "⭐ a/x");
    assert_eq!(model_list_button_label("a", "x", "🕘", true), "✅ 🕘 a/x");
}

// --- model callback-handler gates (7) ---

#[test]
fn parse_model_item_parity() {
    assert_eq!(
        parse_model_item(Some("a"), Some("x"), Some("high")),
        Some(ParsedModelItem {
            provider_id: "a".to_string(),
            model_id: "x".to_string(),
            variant: "high".to_string(),
        })
    );
    // A missing variant falls back to `default`; an empty one stays empty.
    assert_eq!(
        parse_model_item(Some("a"), Some("x"), None).map(|i| i.variant),
        Some("default".to_string())
    );
    assert_eq!(
        parse_model_item(Some("a"), Some("x"), Some("")).map(|i| i.variant),
        Some(String::new())
    );
    assert_eq!(parse_model_item(None, Some("x"), None), None);
    assert_eq!(parse_model_item(Some("a"), None, None), None);
}

#[test]
fn parse_provider_item_parity() {
    assert_eq!(
        parse_provider_item(Some("o"), Some("OpenAI"), Some(12)),
        Some(ProviderRow {
            id: "o".to_string(),
            name: "OpenAI".to_string(),
            model_count: 12
        })
    );
    // A non-number count reads as zero.
    assert_eq!(
        parse_provider_item(Some("o"), Some("OpenAI"), None).map(|p| p.model_count),
        Some(0)
    );
    assert_eq!(parse_provider_item(None, Some("OpenAI"), None), None);
    assert_eq!(parse_provider_item(Some("o"), None, None), None);
}

#[test]
fn parse_callback_index_parity() {
    assert_eq!(
        parse_callback_index("model:result:3", "model:result:"),
        Some(3)
    );
    assert_eq!(
        parse_callback_index("model:result:x", "model:result:"),
        None
    );
    assert_eq!(
        parse_callback_index("model:list:favorites:1", "model:result:"),
        None
    );
}

#[test]
fn resolve_model_list_callback_parity() {
    let favorites = vec![m("a", "x")];
    let recent = vec![m("b", "y")];
    assert_eq!(
        resolve_model_list_callback("model:list:favorites:0", &favorites, &recent),
        Some(ResolvedModel {
            provider_id: "a".to_string(),
            model_id: "x".to_string(),
            variant: "default".to_string(),
        })
    );
    assert_eq!(
        resolve_model_list_callback("model:list:recent:0", &favorites, &recent).map(|r| r.model_id),
        Some("y".to_string())
    );
    assert_eq!(
        resolve_model_list_callback("model:list:other:0", &favorites, &recent),
        None
    );
    assert_eq!(
        resolve_model_list_callback("model:list:favorites:5", &favorites, &recent),
        None
    );
    assert_eq!(
        resolve_model_list_callback("model:list:favorites", &favorites, &recent),
        None
    );
}

#[test]
fn parse_legacy_model_callback_parity() {
    assert_eq!(
        parse_legacy_model_callback("model:a:x"),
        Some(ResolvedModel {
            provider_id: "a".to_string(),
            model_id: "x".to_string(),
            variant: "default".to_string(),
        })
    );
    // A model id may itself contain colons.
    assert_eq!(
        parse_legacy_model_callback("model:a:x:y").map(|r| r.model_id),
        Some("x:y".to_string())
    );
    assert_eq!(parse_legacy_model_callback("model:a"), None);
    assert_eq!(parse_legacy_model_callback("model::x"), None);
    assert_eq!(parse_legacy_model_callback("model:a:"), None);
}

#[test]
fn short_and_provider_browser_classification() {
    assert!(is_short_model_callback("model:result:0"));
    assert!(is_short_model_callback("model:list:favorites:0"));
    assert!(is_short_model_callback("model:providers:0"));
    assert!(!is_short_model_callback("model:search"));
    assert!(is_provider_browser_callback("model:root"));
    assert!(is_provider_browser_callback("model:providers:0"));
    assert!(is_provider_browser_callback("model:provider:0:1"));
    assert!(is_provider_browser_callback("model:pick:2"));
    assert!(!is_provider_browser_callback("model:search"));
    assert!(!is_provider_browser_callback("model:list:recent:0"));
}

#[test]
fn routes_dispatch() {
    assert_eq!(
        route_model_select_callback("other"),
        ModelSelectRoute::Ignored
    );
    assert_eq!(
        route_model_select_callback("model:search"),
        ModelSelectRoute::SearchFlow
    );
    assert_eq!(
        route_model_select_callback("model:providers:1"),
        ModelSelectRoute::ProviderBrowser
    );
    assert_eq!(
        route_model_select_callback("model:list:favorites:0"),
        ModelSelectRoute::ListOrLegacy
    );
    assert_eq!(
        route_provider_browser_callback("model:root"),
        ProviderBrowserRoute::Root
    );
    assert_eq!(
        route_provider_browser_callback("model:providers:2"),
        ProviderBrowserRoute::ProvidersPage(2)
    );
    assert_eq!(
        route_provider_browser_callback("model:provider:3:1"),
        ProviderBrowserRoute::ProviderModels {
            provider_index: 3,
            page: 1
        }
    );
    assert_eq!(
        route_provider_browser_callback("model:pick:4"),
        ProviderBrowserRoute::PickModel(4)
    );
    assert_eq!(
        route_provider_browser_callback("model:search"),
        ProviderBrowserRoute::Unmatched
    );
    assert_eq!(
        route_search_results_callback("model:search:cancel", false, true),
        SearchResultsRoute::Ignored
    );
    assert_eq!(
        route_search_results_callback("model:result:0", true, false),
        SearchResultsRoute::StaleMessage
    );
    assert_eq!(
        route_search_results_callback("model:search:cancel", true, true),
        SearchResultsRoute::Cancel
    );
    assert_eq!(
        route_search_results_callback("model:search:again", true, true),
        SearchResultsRoute::SearchAgain
    );
    assert_eq!(
        route_search_results_callback("model:result:2", true, true),
        SearchResultsRoute::ResultIndex(2)
    );
    assert_eq!(
        route_search_results_callback("model:pick:1", true, true),
        SearchResultsRoute::ShortInvalid
    );
    assert_eq!(
        route_search_results_callback("model:custom:tail", true, true),
        SearchResultsRoute::LegacyCandidate
    );
}
