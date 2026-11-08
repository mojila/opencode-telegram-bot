//! Parity port of the session-directory cache pure gates (slice 61, RED).
//!
//! Mirrors the pure remainder in:
//! - `src/app/services/session-cache-service.ts` (cache budgets, empty
//!   payload, worktree key/validity gates, cache normalization,
//!   dedupe/trim windows, upsert gates, list params, virtual project ids,
//!   storage-root candidates, prune/truncation/cooldown gates, sqlite row
//!   mapping, last-synced bump and ingest gates)
//! - `src/app/types/session.ts` (`CachedSessionDirectory`,
//!   `SessionDirectoryProject` shapes)
//!
//! Step 1: these tests assert the TS contract and MUST FAIL with
//! `unimplemented!()` panics until the Rust port lands (step 2).

use otb_core::session_cache_gates::*;

// --- budgets / markers (8) ---

#[test]
fn cache_version_is_1() {
    assert_eq!(CACHE_VERSION, 1);
}

#[test]
fn initial_warmup_limit_is_1000() {
    assert_eq!(INITIAL_WARMUP_LIMIT, 1000);
}

#[test]
fn incremental_sync_limit_is_1000() {
    assert_eq!(INCREMENTAL_SYNC_LIMIT, 1000);
}

#[test]
fn max_cached_directories_is_10() {
    assert_eq!(MAX_CACHED_DIRECTORIES, 10);
}

#[test]
fn sync_safety_window_ms_is_60s() {
    assert_eq!(SYNC_SAFETY_WINDOW_MS, 60_000.0);
}

#[test]
fn sync_cooldown_ms_is_60s() {
    assert_eq!(SYNC_COOLDOWN_MS, 60_000.0);
}

#[test]
fn storage_fallback_scan_limit_is_200() {
    assert_eq!(STORAGE_FALLBACK_SCAN_LIMIT, 200);
}

#[test]
fn sqlite_fallback_query_limit_is_200() {
    assert_eq!(SQLITE_FALLBACK_QUERY_LIMIT, 200);
}

// --- empty payload (2) ---

#[test]
fn empty_cache_data_shape() {
    assert_eq!(
        empty_cache_data(),
        SessionDirectoryCacheData {
            version: 1,
            last_synced_updated_at: 0.0,
            directories: vec![],
        }
    );
}

#[test]
fn create_empty_cache_data_matches_empty() {
    assert_eq!(create_empty_cache_data(), empty_cache_data());
}

// --- worktreeKey (2) ---

#[test]
fn worktree_key_identity_on_posix() {
    assert_eq!(worktree_key("/App/Work", false), "/App/Work");
}

#[test]
fn worktree_key_lowercases_on_windows() {
    assert_eq!(worktree_key("/App/Work", true), "/app/work");
}

// --- isValidWorktree (4) ---

#[test]
fn valid_worktree_accepted() {
    assert!(is_valid_worktree("/app/work"));
}

#[test]
fn worktree_rejects_empty_and_blank() {
    assert!(!is_valid_worktree(""));
    assert!(!is_valid_worktree("   "));
}

#[test]
fn worktree_rejects_root_slash() {
    assert!(!is_valid_worktree("/"));
    assert!(!is_valid_worktree("  /  "));
}

#[test]
fn worktree_trims_before_validating() {
    assert!(is_valid_worktree("  /app/work  "));
}

// --- normalizeCacheData (7) ---

fn raw_dir(worktree: Option<&str>, last_updated: Option<f64>) -> RawCacheDirectory {
    RawCacheDirectory {
        worktree: worktree.map(str::to_string),
        last_updated,
    }
}

#[test]
fn normalize_none_gives_empty() {
    assert_eq!(normalize_cache_data(None, false), empty_cache_data());
}

#[test]
fn normalize_preserves_entries_and_trims() {
    let raw = RawCacheData {
        last_synced_updated_at: Some(42.0),
        directories: Some(vec![raw_dir(Some("  /app/a  "), Some(7.0))]),
    };
    assert_eq!(
        normalize_cache_data(Some(&raw), false),
        SessionDirectoryCacheData {
            version: 1,
            last_synced_updated_at: 42.0,
            directories: vec![CachedSessionDirectory {
                worktree: "/app/a".to_string(),
                last_updated: 7.0,
            }],
        }
    );
}

#[test]
fn normalize_non_finite_last_synced_becomes_zero() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let raw = RawCacheData {
            last_synced_updated_at: Some(bad),
            directories: Some(vec![]),
        };
        assert_eq!(
            normalize_cache_data(Some(&raw), false).last_synced_updated_at,
            0.0
        );
    }
}

#[test]
fn normalize_drops_entries_with_bad_shapes() {
    let raw = RawCacheData {
        last_synced_updated_at: Some(0.0),
        directories: Some(vec![
            raw_dir(None, Some(1.0)),
            raw_dir(Some("/ok"), None),
            raw_dir(Some(""), Some(2.0)),
        ]),
    };
    assert!(normalize_cache_data(Some(&raw), false).directories.is_empty());
}

#[test]
fn normalize_drops_root_worktree() {
    let raw = RawCacheData {
        last_synced_updated_at: Some(0.0),
        directories: Some(vec![raw_dir(Some("/"), Some(1.0))]),
    };
    assert!(normalize_cache_data(Some(&raw), false).directories.is_empty());
}

#[test]
fn normalize_forces_version_one() {
    let raw = RawCacheData {
        last_synced_updated_at: Some(9.0),
        directories: Some(vec![]),
    };
    assert_eq!(normalize_cache_data(Some(&raw), false).version, 1);
}

#[test]
fn normalize_dedupes_keeping_newest() {
    let raw = RawCacheData {
        last_synced_updated_at: Some(0.0),
        directories: Some(vec![
            raw_dir(Some("/app/a"), Some(1.0)),
            raw_dir(Some("/app/a"), Some(5.0)),
            raw_dir(Some("/app/b"), Some(3.0)),
        ]),
    };
    assert_eq!(
        normalize_cache_data(Some(&raw), false).directories,
        vec![
            CachedSessionDirectory {
                worktree: "/app/a".to_string(),
                last_updated: 5.0,
            },
            CachedSessionDirectory {
                worktree: "/app/b".to_string(),
                last_updated: 3.0,
            },
        ]
    );
}

// --- dedupeAndTrimDirectories (3) ---

fn dir(worktree: &str, last_updated: f64) -> CachedSessionDirectory {
    CachedSessionDirectory {
        worktree: worktree.to_string(),
        last_updated,
    }
}

#[test]
fn dedupe_sorts_newest_first() {
    let out = dedupe_and_trim_directories(vec![dir("/a", 1.0), dir("/b", 9.0)], false);
    assert_eq!(out, vec![dir("/b", 9.0), dir("/a", 1.0)]);
}

#[test]
fn dedupe_windows_keys_fold_case() {
    let out = dedupe_and_trim_directories(vec![dir("/App", 1.0), dir("/app", 4.0)], true);
    assert_eq!(out, vec![dir("/app", 4.0)]);
}

#[test]
fn dedupe_caps_at_ten_newest() {
    let input: Vec<CachedSessionDirectory> = (0..12)
        .map(|n| dir(&format!("/d{n}"), n as f64))
        .collect();
    let out = dedupe_and_trim_directories(input, false);
    assert_eq!(out.len(), 10);
    assert_eq!(out[0], dir("/d11", 11.0));
    assert_eq!(out[9], dir("/d2", 2.0));
}

// --- upsertDirectory (5) ---

#[test]
fn upsert_rejects_invalid_worktree() {
    let mut dirs = vec![];
    assert!(!upsert_directory(&mut dirs, "   ", 5.0, false));
    assert!(!upsert_directory(&mut dirs, "/", 5.0, false));
    assert!(dirs.is_empty());
}

#[test]
fn upsert_inserts_new_directory_trimmed() {
    let mut dirs = vec![];
    assert!(upsert_directory(&mut dirs, "  /app/a  ", 5.0, false));
    assert_eq!(dirs, vec![dir("/app/a", 5.0)]);
}

#[test]
fn upsert_ignores_stale_and_equal_timestamps() {
    let mut dirs = vec![dir("/app/a", 5.0)];
    assert!(!upsert_directory(&mut dirs, "/app/a", 4.0, false));
    assert!(!upsert_directory(&mut dirs, "/app/a", 5.0, false));
    assert_eq!(dirs, vec![dir("/app/a", 5.0)]);
}

#[test]
fn upsert_updates_newer_timestamp_keeping_spelling() {
    let mut dirs = vec![dir("/App/A", 5.0)];
    assert!(upsert_directory(&mut dirs, "/app/a", 9.0, true));
    assert_eq!(dirs, vec![dir("/App/A", 9.0)]);
}

#[test]
fn last_synced_bumps_only_when_greater() {
    assert!(should_bump_last_synced(10.0, 9.0));
    assert!(!should_bump_last_synced(9.0, 9.0));
    assert!(!should_bump_last_synced(8.0, 9.0));
}

#[test]
fn missing_session_directory_skipped() {
    assert!(!has_session_directory(None));
    assert!(!has_session_directory(Some("")));
    assert!(has_session_directory(Some("/app/a")));
}

// --- buildListParams (3) ---

#[test]
fn list_params_force_uses_warmup_without_start() {
    assert_eq!(
        build_list_params(true, 123.0),
        ListParams {
            limit: 1000,
            start: None,
        }
    );
}

#[test]
fn list_params_never_synced_uses_warmup_without_start() {
    assert_eq!(
        build_list_params(false, 0.0),
        ListParams {
            limit: 1000,
            start: None,
        }
    );
}

#[test]
fn list_params_incremental_uses_safety_window() {
    assert_eq!(
        build_list_params(false, 100_000.0),
        ListParams {
            limit: 1000,
            start: Some(40_000.0),
        }
    );
    assert_eq!(
        build_list_params(false, 1_000.0),
        ListParams {
            limit: 1000,
            start: Some(0.0),
        }
    );
}

// --- prune / truncation / cooldown (5) ---

#[test]
fn should_prune_when_forced_or_never_synced() {
    assert!(should_prune(true, 50.0));
    assert!(should_prune(false, 0.0));
    assert!(!should_prune(false, 50.0));
}

#[test]
fn response_truncation_at_warmup_limit() {
    assert!(!is_response_truncated(999));
    assert!(is_response_truncated(1000));
    assert!(is_response_truncated(1001));
}

#[test]
fn sync_cooldown_active_within_window() {
    assert!(is_sync_cooldown_active(70_000.0, 20_000.0, false));
    assert!(!is_sync_cooldown_active(80_000.0, 20_000.0, false));
    assert!(!is_sync_cooldown_active(70_000.0, 20_000.0, true));
}

// --- virtual ids + projects (4) ---

#[test]
fn virtual_project_id_shape() {
    let id = create_virtual_project_id("/app/work");
    assert!(id.starts_with("dir_"), "unexpected id: {id}");
    assert_eq!(id.len(), 20, "unexpected id: {id}");
    assert!(
        id[4..].chars().all(|c| c.is_ascii_hexdigit()),
        "unexpected id: {id}"
    );
}

#[test]
fn virtual_project_id_is_deterministic() {
    assert_eq!(
        create_virtual_project_id("/app/a"),
        create_virtual_project_id("/app/a")
    );
    assert_ne!(
        create_virtual_project_id("/app/a"),
        create_virtual_project_id("/app/b")
    );
}

#[test]
fn session_directory_project_maps_fields() {
    let project = session_directory_project("/app/a", 7.0);
    assert_eq!(project.worktree, "/app/a");
    assert_eq!(project.name, "/app/a");
    assert_eq!(project.last_updated, 7.0);
    assert_eq!(project.id, create_virtual_project_id("/app/a"));
}

// --- storage roots (5) ---

#[test]
fn storage_roots_empty_without_paths() {
    assert!(get_storage_root_candidates(None, None).is_empty());
    assert!(!path_api_available(false));
    assert!(path_api_available(true));
}

#[test]
fn storage_roots_home_derives_share_path() {
    assert_eq!(
        get_storage_root_candidates(Some("/home/u"), None),
        vec!["/home/u/.local/share/opencode".to_string()]
    );
}

#[test]
fn storage_roots_state_suffix_derives_share_path() {
    assert_eq!(
        get_storage_root_candidates(None, Some("/home/u/.local/state/opencode")),
        vec!["/home/u/.local/share/opencode".to_string()]
    );
}

#[test]
fn storage_roots_tolerates_trailing_separators() {
    assert_eq!(
        get_storage_root_candidates(None, Some("/home/u/.local/state/opencode///")),
        vec!["/home/u/.local/share/opencode".to_string()]
    );
}

#[test]
fn storage_roots_home_first_and_deduped() {
    assert_eq!(
        get_storage_root_candidates(
            Some("/home/u"),
            Some("/home/u/.local/state/opencode")
        ),
        vec!["/home/u/.local/share/opencode".to_string()]
    );
}

// --- sqlite row mapping (3) ---

#[test]
fn sqlite_finite_updated_preserved() {
    assert_eq!(normalize_sqlite_updated(Some(12.0)), 12.0);
}

#[test]
fn sqlite_missing_updated_maps_to_zero() {
    assert_eq!(normalize_sqlite_updated(None), 0.0);
}

#[test]
fn sqlite_non_finite_updated_maps_to_zero() {
    assert_eq!(normalize_sqlite_updated(Some(f64::NAN)), 0.0);
    assert_eq!(normalize_sqlite_updated(Some(f64::INFINITY)), 0.0);
}
