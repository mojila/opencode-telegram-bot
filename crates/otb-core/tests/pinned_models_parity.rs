//! Parity port of the pure pinned-dashboard + model-catalog subset (slice 23b, RED).
//!
//! Mirrors representative `it`s from:
//! - `tests/bot/pinned/pinned-message-manager.test.ts` (100 its)
//! - `tests/app/services/model-selection-service.test.ts` (79 its)
//!
//! Only the pure state/formatting/gating subset is ported here. Deferred I/O
//! (Telegram send/edit/pin/unpin, debounce timers, SSE subscription,
//! `opencodeClient.session.*` reads, `model.json` file IO, catalog fetch +
//! TTL/warmup/wait loops, settings-store writes) stays in TS and is NOT
//! asserted here.
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::pinned_models::*;
use std::collections::{HashMap, HashSet};

// --- pinned format: formatTokenCount ---

#[test]
fn slice23b_01_token_count_small() {
    assert_eq!(format_token_count(999), "999");
}

#[test]
fn slice23b_02_token_count_kilo() {
    assert_eq!(format_token_count(5000), "5K");
}

#[test]
fn slice23b_03_token_count_mega() {
    assert_eq!(format_token_count(2_500_000), "2.5M");
}

// --- pinned format: formatModelDisplayName ---

#[test]
fn slice23b_04_model_display_with_variant() {
    assert_eq!(
        format_model_display_name(Some("openai"), Some("gpt-5"), Some("low")),
        "openai/gpt-5 (low)"
    );
}

#[test]
fn slice23b_05_model_display_without_variant() {
    assert_eq!(
        format_model_display_name(Some("openai"), Some("gpt-5"), None),
        "openai/gpt-5"
    );
}

#[test]
fn slice23b_06_model_display_fallback_unknown() {
    assert_eq!(format_model_display_name(None, None, None), "Unknown");
    assert_eq!(
        format_model_display_name(Some("openai"), None, None),
        "Unknown"
    );
}

// --- pinned format: context + cost lines ---

#[test]
fn slice23b_07_context_line_with_limit() {
    assert_eq!(
        format_context_line(0, Some(1_000_000)),
        "Context: 0 / 1M (0%)"
    );
}

#[test]
fn slice23b_08_context_line_unknown_limit() {
    assert_eq!(
        format_context_line(3500, Some(0)),
        "Context: 3K / Unknown (0%)"
    );
    assert_eq!(
        format_context_line(3500, None),
        "Context: 3K / Unknown (0%)"
    );
}

#[test]
fn slice23b_09_cost_line_two_decimals() {
    assert_eq!(format_cost_line(1.0), "Cost: $1.00 spent");
    assert_eq!(format_cost_line(12.5), "Cost: $12.50 spent");
}

// --- tokens / context accounting ---

#[test]
fn slice23b_10_tokens_used_input_plus_cache() {
    assert_eq!(tokens_used(5000, 1000), 6000);
    assert_eq!(tokens_used(100, 50), 150);
}

#[test]
fn slice23b_11_context_info_none_until_limit() {
    assert_eq!(context_info(0, None), None);
    assert_eq!(context_info(0, Some(0)), None);
}

#[test]
fn slice23b_12_context_info_pair_when_known() {
    assert_eq!(context_info(150, Some(204_800)), Some((150, 204_800)));
}

#[test]
fn slice23b_13_history_prefers_latest_non_summary_nonzero() {
    let messages = vec![
        HistoryMessage {
            summary: false,
            input_tokens: 300,
            cache_read_tokens: 100,
            cost: 0.5,
        },
        HistoryMessage {
            summary: false,
            input_tokens: 900,
            cache_read_tokens: 100,
            cost: 0.25,
        },
        HistoryMessage {
            summary: false,
            input_tokens: 0,
            cache_read_tokens: 0,
            cost: 0.1,
        },
        HistoryMessage {
            summary: true,
            input_tokens: 1500,
            cache_read_tokens: 0,
            cost: 4.0,
        },
    ];
    let (tokens, cost) = summarize_history_context(&messages);
    assert_eq!(tokens, 400);
    assert!((cost - 0.85).abs() < 1e-9);
}

#[test]
fn slice23b_14_session_cumulative_cost_wins() {
    assert!((prefer_session_cost(Some(12.5), 0.5) - 12.5).abs() < 1e-9);
    assert!((prefer_session_cost(None, 0.5) - 0.5).abs() < 1e-9);
}

#[test]
fn slice23b_15_failed_refresh_keeps_previous_cost() {
    assert!((keep_cost_on_failed_refresh(12.5, None) - 12.5).abs() < 1e-9);
}

#[test]
fn slice23b_16_fallback_default_limit() {
    assert_eq!(fallback_context_limit(None), DEFAULT_CONTEXT_LIMIT);
    assert_eq!(fallback_context_limit(Some(1_000_000)), 1_000_000);
}

// --- changed files ---

#[test]
fn slice23b_17_diff_ignores_entries_without_file() {
    let entries = vec![
        RawDiffEntry {
            file: Some("D:/repo/src/a.ts".into()),
            additions: 3,
            deletions: 1,
        },
        RawDiffEntry {
            file: None,
            additions: 9,
            deletions: 9,
        },
        RawDiffEntry {
            file: Some("D:/repo/src/b.ts".into()),
            additions: 0,
            deletions: 2,
        },
    ];
    assert_eq!(
        filter_diff_entries(&entries),
        vec![
            FileChange::new("D:/repo/src/a.ts", 3, 1),
            FileChange::new("D:/repo/src/b.ts", 0, 2),
        ]
    );
}

#[test]
fn slice23b_18_identical_diff_is_equal() {
    let a = vec![FileChange::new("D:/repo/src/a.ts", 1, 0)];
    let b = vec![FileChange::new("D:/repo/src/a.ts", 1, 0)];
    let c = vec![FileChange::new("D:/repo/src/a.ts", 2, 0)];
    assert!(diffs_equal(&a, &b));
    assert!(!diffs_equal(&a, &c));
}

#[test]
fn slice23b_19_file_diff_suffix_shapes() {
    assert_eq!(format_file_diff_suffix(7, 0), " (+7)");
    assert_eq!(format_file_diff_suffix(1, 2), " (+1 -2)");
}

#[test]
fn slice23b_20_shortens_paths_outside_project() {
    assert_eq!(
        display_path_for_project("C:/other/deep/nested/path/file.ts", "D:/repo"),
        ".../nested/path/file.ts"
    );
    assert_eq!(
        display_path_for_project("D:/repo/src/a.ts", "D:/repo"),
        "src/a.ts"
    );
}

#[test]
fn slice23b_21_files_section_caps_at_ten_plus_overflow() {
    let changes: Vec<FileChange> = (0..12)
        .map(|i| FileChange::new(&format!("D:/repo/src/file-{i}.ts"), 1, 0))
        .collect();
    let text = format_files_section(&changes, "D:/repo");
    assert!(text.contains("Files (12):"));
    assert!(text.contains("src/file-9.ts"));
    assert!(!text.contains("src/file-10.ts"));
    assert!(text.contains("... and 2 more"));
}

#[test]
fn slice23b_22_files_section_empty_when_no_changes() {
    assert_eq!(format_files_section(&[], "D:/repo"), "");
}

#[test]
fn slice23b_23_file_item_line_shape() {
    assert_eq!(
        format_file_item(&FileChange::new("D:/repo/src/a.ts", 3, 1), "D:/repo"),
        "  src/a.ts (+3 -1)"
    );
}

// --- incremental update gating ---

#[test]
fn slice23b_24_title_updates_only_for_new_nonempty() {
    assert!(!should_update_title("Test Session", "Test Session"));
    assert!(!should_update_title("Test Session", ""));
    assert!(should_update_title("Test Session", "Renamed session"));
}

#[test]
fn slice23b_25_cost_updates_ignoring_no_change() {
    assert!(!should_apply_cost_update(5.0, 0.0));
    assert!(!should_apply_cost_update(5.0, f64::NAN));
    assert!(should_apply_cost_update(5.0, 1.0));
}

#[test]
fn slice23b_26_detach_drops_busy_flag() {
    assert_eq!(detach_attach_state(true, true, true), (false, false));
    assert_eq!(detach_attach_state(false, false, false), (false, false));
}

#[test]
fn slice23b_27_dashboard_gating() {
    assert!(!should_render_dashboard(false, true, true));
    assert!(!should_render_dashboard(true, false, false));
    assert!(should_render_dashboard(true, true, true));
}

// --- Telegram error classification ---

#[test]
fn slice23b_28_not_modified_is_delivered() {
    assert!(is_not_modified_error(
        "Bad Request: message is not modified"
    ));
    assert!(!is_not_modified_error("Bad Request: chat not found"));
}

#[test]
fn slice23b_29_deleted_message_recreates_dashboard() {
    assert!(is_message_deleted_error(
        "Bad Request: message to edit not found"
    ));
    assert!(!is_message_deleted_error(
        "Bad Request: message is not modified"
    ));
}

#[test]
fn slice23b_30_unpin_not_found_forgets_leftover() {
    assert!(is_unpin_not_found_error(
        "Bad Request: message to unpin not found"
    ));
    assert!(!is_unpin_not_found_error("Bad Request: too many requests"));
}

#[test]
fn slice23b_31_transient_errors_keep_ownership() {
    assert!(is_transient_telegram_error(
        "Bad Request: too many requests"
    ));
    assert!(!is_transient_telegram_error(
        "Bad Request: message to edit not found"
    ));
}

// --- project / branch / worktree lines ---

#[test]
fn slice23b_32_project_line_with_branch() {
    assert_eq!(
        project_line("D:/repo", Some("main")),
        "Project: D:/repo: main"
    );
}

#[test]
fn slice23b_33_project_line_without_branch() {
    assert_eq!(project_line("D:/repo", None), "Project: D:/repo");
}

#[test]
fn slice23b_34_worktree_line_only_for_linked() {
    assert_eq!(
        worktree_line(Some("D:/repo-feature"), true),
        Some("Worktree: D:/repo-feature".to_string())
    );
    assert_eq!(worktree_line(Some("D:/repo"), false), None);
    assert_eq!(worktree_line(None, true), None);
}

// --- model selection pure helpers ---

#[test]
fn slice23b_35_model_key_shape() {
    assert_eq!(model_key("openai", "gpt-4o"), "openai/gpt-4o");
}

#[test]
fn slice23b_36_dedupe_first_wins() {
    let models = vec![
        ModelRef::new("openai", "gpt-4o"),
        ModelRef::new("openai", "gpt-4o"),
        ModelRef::new("anthropic", "claude-sonnet"),
    ];
    assert_eq!(
        dedupe_models(&models),
        vec![
            ModelRef::new("openai", "gpt-4o"),
            ModelRef::new("anthropic", "claude-sonnet"),
        ]
    );
}

#[test]
fn slice23b_37_filter_by_catalog() {
    let models = vec![
        ModelRef::new("openai", "gpt-4o"),
        ModelRef::new("openai", "missing-favorite"),
    ];
    let mut keys = HashSet::new();
    keys.insert("openai/gpt-4o".to_string());
    assert_eq!(
        filter_models_by_catalog(&models, Some(&keys)),
        vec![ModelRef::new("openai", "gpt-4o")]
    );
    assert_eq!(filter_models_by_catalog(&models, None), models);
}

#[test]
fn slice23b_38_normalize_drops_invalid_entries() {
    let entries = vec![
        RawModelEntry::valid("openai", "gpt-4o"),
        RawModelEntry::empty_provider(),
        RawModelEntry::missing_provider(),
        RawModelEntry::missing_model(),
        RawModelEntry::empty_model(),
    ];
    assert_eq!(
        normalize_model_entries(&entries),
        vec![ModelRef::new("openai", "gpt-4o")]
    );
}

#[test]
fn slice23b_39_recent_excludes_favorites() {
    let favorites = vec![ModelRef::new("openai", "gpt-4o")];
    let recent = vec![
        ModelRef::new("openai", "gpt-4o"),
        ModelRef::new("google", "gemini-pro"),
    ];
    assert_eq!(
        exclude_favorites_from_recent(&recent, &favorites),
        vec![ModelRef::new("google", "gemini-pro")]
    );
}

#[test]
fn slice23b_40_search_case_insensitive_sorted_capped() {
    let catalog = vec![
        ModelRef::new("openai", "gpt-4o"),
        ModelRef::new("openai", "gpt-3.5"),
        ModelRef::new("anthropic", "claude-sonnet"),
    ];
    let results = search_models(&catalog, "GPT");
    assert_eq!(
        results,
        vec![
            ModelRef::new("openai", "gpt-3.5"),
            ModelRef::new("openai", "gpt-4o"),
        ]
    );
    assert!(search_models(&catalog, "   ").is_empty());
    assert!(search_models(&catalog, "nonexistent-model-xyz").is_empty());
}

#[test]
fn slice23b_41_search_matches_provider_id() {
    let catalog = vec![
        ModelRef::new("openai", "gpt-4o"),
        ModelRef::new("anthropic", "claude-sonnet"),
    ];
    assert_eq!(
        search_models(&catalog, "anthrop"),
        vec![ModelRef::new("anthropic", "claude-sonnet")]
    );
}

#[test]
fn slice23b_42_providers_shaped_sorted_skip_empty() {
    let catalog = vec![
        CatalogProvider {
            id: "openai".into(),
            name: Some("OpenAI".into()),
            models: vec!["gpt-4o".into(), "gpt-3.5".into()],
        },
        CatalogProvider {
            id: "anthropic".into(),
            name: None,
            models: vec![],
        },
        CatalogProvider {
            id: "google".into(),
            name: None,
            models: vec!["gemini-pro".into()],
        },
    ];
    assert_eq!(
        shape_providers(&catalog),
        vec![
            ProviderInfo {
                id: "google".into(),
                name: "google".into(),
                model_count: 1
            },
            ProviderInfo {
                id: "openai".into(),
                name: "OpenAI".into(),
                model_count: 2
            },
        ]
    );
}

#[test]
fn slice23b_43_provider_models_sorted() {
    let catalog = vec![CatalogProvider {
        id: "openai".into(),
        name: None,
        models: vec!["gpt-4o".into(), "gpt-3.5".into()],
    }];
    assert_eq!(
        provider_models(&catalog, "openai"),
        vec![
            ModelRef::new("openai", "gpt-3.5"),
            ModelRef::new("openai", "gpt-4o"),
        ]
    );
    assert!(provider_models(&catalog, "unknown").is_empty());
}

#[test]
fn slice23b_44_availability_classification() {
    let mut catalog = HashMap::new();
    catalog.insert("openai".to_string(), vec!["gpt-4o".to_string()]);
    assert_eq!(
        classify_model_availability(&catalog, "openai", "gpt-4o"),
        ModelAvailability::Offered
    );
    assert_eq!(
        classify_model_availability(&catalog, "openai", "retired"),
        ModelAvailability::ModelMissing
    );
    assert_eq!(
        classify_model_availability(&catalog, "missing-provider", "gpt-4o"),
        ModelAvailability::ProviderMissing
    );
    assert_eq!(
        classify_model_availability(&HashMap::new(), "openai", "gpt-4o"),
        ModelAvailability::Unknown
    );
}

#[test]
fn slice23b_45_reconcile_replace_decision() {
    assert!(should_replace_stored_model(true, false, true));
    assert!(!should_replace_stored_model(true, true, true));
    assert!(!should_replace_stored_model(false, false, true));
    assert!(!should_replace_stored_model(true, false, false));
}

#[test]
fn slice23b_46_adoptability_with_warmup() {
    assert!(is_model_adoptable(ModelAvailability::Offered, false));
    assert!(is_model_adoptable(ModelAvailability::Unknown, false));
    assert!(is_model_adoptable(ModelAvailability::ProviderMissing, true));
    assert!(!is_model_adoptable(
        ModelAvailability::ProviderMissing,
        false
    ));
    assert!(!is_model_adoptable(ModelAvailability::ModelMissing, true));
}

#[test]
fn slice23b_47_build_favorites_adds_offered_default_once() {
    let file_favorites = vec![ModelRef::new("openai", "gpt-4o")];
    let default = ModelRef::new("opencode", "big-pickle");
    let mut keys = HashSet::new();
    keys.insert("openai/gpt-4o".to_string());
    keys.insert("opencode/big-pickle".to_string());
    let favorites = build_favorites(&file_favorites, Some(&default), Some(&keys));
    assert!(favorites.contains(&ModelRef::new("openai", "gpt-4o")));
    assert!(favorites.contains(&default));
    assert_eq!(favorites.iter().filter(|m| **m == default).count(), 1);
}

#[test]
fn slice23b_48_build_recent_dedupes_and_excludes_favorites() {
    let favorites = vec![ModelRef::new("openai", "gpt-4o")];
    let file_recent = vec![
        ModelRef::new("openai", "gpt-4o"),
        ModelRef::new("google", "gemini-pro"),
        ModelRef::new("google", "gemini-pro"),
    ];
    let recent = build_recent(&file_recent, &favorites, None);
    assert_eq!(recent, vec![ModelRef::new("google", "gemini-pro")]);
}
