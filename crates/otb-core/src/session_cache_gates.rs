//! Session-directory cache pure gates (slice 61).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/session-cache-service.ts`
//!   (`CACHE_VERSION`, `INITIAL_WARMUP_LIMIT`, `INCREMENTAL_SYNC_LIMIT`,
//!   `MAX_CACHED_DIRECTORIES`, `SYNC_SAFETY_WINDOW_MS`, `SYNC_COOLDOWN_MS`,
//!   `STORAGE_FALLBACK_SCAN_LIMIT`, `SQLITE_FALLBACK_QUERY_LIMIT`,
//!   `EMPTY_CACHE` / `createEmptyCacheData`, `worktreeKey` platform fold,
//!   `isValidWorktree` blank/root gate, `normalizeCacheData` record/number/
//!   trim filter, `dedupeAndTrimDirectories` newest-wins + recency sort +
//!   cap, `upsertDirectory` stale-timestamp gate + spelling preservation,
//!   `buildListParams` warmup/incremental branch with safety-window start,
//!   `createVirtualProjectId` `dir_` + sha1-hex-16 id,
//!   `getCachedSessionProjects` id/name mapping,
//!   `getStorageRootCandidates` home/state derivation, `shouldPrune` /
//!   `responseIsTruncated` warmup gates, the sync-cooldown gate,
//!   `getStorageRootsFromApi` missing-`get` gate, sqlite row
//!   `updated` finite-number mapping, `upsertSessionDirectory` last-synced
//!   bump gate, `ingestSessionInfoForCache` missing-directory gate)
//! - `src/app/types/session.ts` (`CachedSessionDirectory`,
//!   `SessionDirectoryProject`, `SessionDirectoryCacheInfo` shapes)
//!
//! No sqlite, no filesystem, no SDK calls, no timers live here — only cache
//! shapes, key/validity gates, dedupe/trim windows, list-param branches,
//! id/root derivation, and sync-gate predicates. Callers own the storage.
//!
//! DEVIATIONS from TS:
//! 1. `process.platform === "win32"` is implicit in TS (`worktreeKey`, the
//!    dedupe/upsert key). The port takes an explicit `is_windows: bool` so
//!    the fold stays pure and testable on every host.
//! 2. `normalizeCacheData` takes `unknown` plus `isRecord` in TS. The port
//!    takes `Option<&RawCacheData>`; `None` models a non-record payload and
//!    yields an empty cache.
//! 3. `path.join` follows the host separator in TS. The port joins with `/`;
//!    parity fixtures use posix-style paths where the two coincide.
//!    Backslashes in inputs are normalized to `/` before joining so Windows
//!    style roots still derive the same posix-style candidate.
//! 4. Timestamps are TS `number`s (floats; `mtimeMs` is fractional). The
//!    port uses `f64` with the same comparisons; integer-valued fixtures
//!    match exactly.
//! 5. `dedupeAndTrimDirectories` mutates its input in TS. The port takes and
//!    returns the vector; observable contents match.
//! 6. `upsertDirectory` mutates module-global `cacheData` in TS. The port
//!    takes `&mut Vec<CachedSessionDirectory>`; the stale-timestamp gate,
//!    spelling preservation, and trim/dedupe rules match.
//! 7. `createVirtualProjectId` hashes with sha1 in TS (`node:crypto`). The
//!    port ships a small pure-Rust SHA-1 (no new dependency) and keeps the
//!    `dir_` + first-16-lowercase-hex-chars contract.
//! 8. `has_session_directory` models TS `if (!directory)` on
//!    `string | undefined`: `None` and `Some("")` are absent, any other
//!    `Some` (including whitespace-only) is present; the later
//!    `is_valid_worktree` gate still rejects blanks.

// ---------------------------------------------------------------------------
// Budgets / markers (mirrors session-cache-service.ts)
// ---------------------------------------------------------------------------

/// Cache schema version (`CACHE_VERSION`, `SessionDirectoryCacheInfo`).
pub const CACHE_VERSION: u32 = 1;

/// Session-list page size for the first (warmup) sync
/// (`INITIAL_WARMUP_LIMIT`).
pub const INITIAL_WARMUP_LIMIT: usize = 1000;

/// Session-list page size for follow-up syncs (`INCREMENTAL_SYNC_LIMIT`).
pub const INCREMENTAL_SYNC_LIMIT: usize = 1000;

/// Max directories kept after dedupe/trim (`MAX_CACHED_DIRECTORIES`).
pub const MAX_CACHED_DIRECTORIES: usize = 10;

/// Overlap window re-fetched on incremental syncs
/// (`SYNC_SAFETY_WINDOW_MS`).
pub const SYNC_SAFETY_WINDOW_MS: f64 = 60_000.0;

/// Minimum gap between sync attempts (`SYNC_COOLDOWN_MS`).
pub const SYNC_COOLDOWN_MS: f64 = 60_000.0;

/// Max session files scanned per storage root
/// (`STORAGE_FALLBACK_SCAN_LIMIT`).
pub const STORAGE_FALLBACK_SCAN_LIMIT: usize = 200;

/// Max rows read per sqlite fallback database
/// (`SQLITE_FALLBACK_QUERY_LIMIT`, the `LIMIT ?` parameter).
pub const SQLITE_FALLBACK_QUERY_LIMIT: usize = 200;

// ---------------------------------------------------------------------------
// Shared types (mirrors types/session.ts + the cache shape)
// ---------------------------------------------------------------------------

/// One cached directory (TS: `CachedSessionDirectory`).
#[derive(Debug, Clone, PartialEq)]
pub struct CachedSessionDirectory {
    pub worktree: String,
    pub last_updated: f64,
}

/// The persisted cache payload (TS: `SessionDirectoryCacheData`).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionDirectoryCacheData {
    pub version: u32,
    pub last_synced_updated_at: f64,
    pub directories: Vec<CachedSessionDirectory>,
}

/// A virtual project row derived from a cached directory
/// (TS: `SessionDirectoryProject`).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionDirectoryProject {
    pub id: String,
    pub worktree: String,
    pub name: String,
    pub last_updated: f64,
}

/// Unvalidated cache payload shape (TS: the `unknown` input to
/// `normalizeCacheData` after the `isRecord` gate).
#[derive(Debug, Clone, PartialEq)]
pub struct RawCacheDirectory {
    pub worktree: Option<String>,
    pub last_updated: Option<f64>,
}

/// Unvalidated cache root shape (TS: the `unknown` input to
/// `normalizeCacheData`).
#[derive(Debug, Clone, PartialEq)]
pub struct RawCacheData {
    pub last_synced_updated_at: Option<f64>,
    pub directories: Option<Vec<RawCacheDirectory>>,
}

/// Session-list paging for one sync pass (TS: `buildListParams` result).
#[derive(Debug, Clone, PartialEq)]
pub struct ListParams {
    pub limit: usize,
    pub start: Option<f64>,
}

// ---------------------------------------------------------------------------
// Empty cache (EMPTY_CACHE / createEmptyCacheData)
// ---------------------------------------------------------------------------

/// The canonical empty payload (TS: `EMPTY_CACHE`).
pub fn empty_cache_data() -> SessionDirectoryCacheData {
    SessionDirectoryCacheData {
        version: CACHE_VERSION,
        last_synced_updated_at: 0.0,
        directories: Vec::new(),
    }
}

/// A fresh empty payload with its own directory vector
/// (TS: `createEmptyCacheData`).
pub fn create_empty_cache_data() -> SessionDirectoryCacheData {
    SessionDirectoryCacheData {
        version: CACHE_VERSION,
        last_synced_updated_at: 0.0,
        directories: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Key / validity gates (worktreeKey, isValidWorktree)
// ---------------------------------------------------------------------------

/// Platform-folded dedupe key: lowercased on Windows, identity elsewhere
/// (TS: `worktreeKey`).
pub fn worktree_key(worktree: &str, is_windows: bool) -> String {
    if is_windows {
        worktree.to_lowercase()
    } else {
        worktree.to_string()
    }
}

/// Non-blank after trim and not the filesystem root `/`
/// (TS: `isValidWorktree`).
pub fn is_valid_worktree(worktree: &str) -> bool {
    let trimmed = worktree.trim();
    !trimmed.is_empty() && trimmed != "/"
}

// ---------------------------------------------------------------------------
// Normalization (normalizeCacheData, dedupeAndTrimDirectories)
// ---------------------------------------------------------------------------

/// Validated cache payload: non-records become empty, a non-finite
/// `lastSyncedUpdatedAt` becomes `0`, directory entries need a string
/// worktree plus a numeric `lastUpdated`, worktrees are trimmed and
/// root/blank entries dropped, the version is forced to `CACHE_VERSION`,
/// and the remainder is deduped/trimmed (TS: `normalizeCacheData`).
pub fn normalize_cache_data(
    raw: Option<&RawCacheData>,
    is_windows: bool,
) -> SessionDirectoryCacheData {
    let Some(raw) = raw else {
        return create_empty_cache_data();
    };
    let last_synced_updated_at = match raw.last_synced_updated_at {
        Some(v) if v.is_finite() => v,
        _ => 0.0,
    };
    let mut directories: Vec<CachedSessionDirectory> = Vec::new();
    if let Some(items) = raw.directories.as_ref() {
        for item in items {
            let (Some(worktree), Some(last_updated)) =
                (item.worktree.as_ref(), item.last_updated)
            else {
                continue;
            };
            let trimmed = worktree.trim();
            if !is_valid_worktree(trimmed) {
                continue;
            }
            directories.push(CachedSessionDirectory {
                worktree: trimmed.to_string(),
                last_updated,
            });
        }
    }
    let directories = dedupe_and_trim_directories(directories, is_windows);
    SessionDirectoryCacheData {
        version: CACHE_VERSION,
        last_synced_updated_at,
        directories,
    }
}

/// Newest entry wins per platform key, then recency-descending sort capped
/// at `MAX_CACHED_DIRECTORIES` (TS: `dedupeAndTrimDirectories`).
pub fn dedupe_and_trim_directories(
    directories: Vec<CachedSessionDirectory>,
    is_windows: bool,
) -> Vec<CachedSessionDirectory> {
    use std::collections::HashMap;
    let mut unique: HashMap<String, CachedSessionDirectory> = HashMap::new();
    for item in directories {
        let key = worktree_key(&item.worktree, is_windows);
        match unique.get(&key) {
            Some(existing) if existing.last_updated >= item.last_updated => {}
            _ => {
                unique.insert(key, item);
            }
        }
    }
    let mut out: Vec<CachedSessionDirectory> = unique.into_values().collect();
    // Stable recency-descending sort mirroring `sort((a, b) => b - a)`.
    out.sort_by(|a, b| {
        b.last_updated
            .partial_cmp(&a.last_updated)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(MAX_CACHED_DIRECTORIES);
    out
}

// ---------------------------------------------------------------------------
// Upsert (upsertDirectory, upsertSessionDirectory last-synced bump)
// ---------------------------------------------------------------------------

/// Insert or refresh one directory. Returns `false` for invalid worktrees
/// and for stale (older-or-equal) timestamps, preserving the stored
/// spelling on update; otherwise inserts/updates, dedupes/trims, and
/// returns `true` (TS: `upsertDirectory`).
pub fn upsert_directory(
    directories: &mut Vec<CachedSessionDirectory>,
    worktree: &str,
    last_updated: f64,
    is_windows: bool,
) -> bool {
    if !is_valid_worktree(worktree) {
        return false;
    }
    let normalized = worktree.trim().to_string();
    let key = worktree_key(&normalized, is_windows);
    if let Some(existing) = directories
        .iter_mut()
        .find(|item| worktree_key(&item.worktree, is_windows) == key)
    {
        if existing.last_updated >= last_updated {
            return false;
        }
        // Preserve the stored spelling, only bump the timestamp.
        existing.last_updated = last_updated;
    } else {
        directories.push(CachedSessionDirectory {
            worktree: normalized,
            last_updated,
        });
    }
    let trimmed = dedupe_and_trim_directories(std::mem::take(directories), is_windows);
    *directories = trimmed;
    true
}

/// Whether `lastUpdated` advances the high-water mark
/// (TS: `upsertSessionDirectory` bump gate).
pub fn should_bump_last_synced(last_updated: f64, last_synced_updated_at: f64) -> bool {
    last_updated > last_synced_updated_at
}

/// Whether a session info object carries a directory to ingest
/// (TS: `ingestSessionInfoForCache` missing-directory gate).
pub fn has_session_directory(directory: Option<&str>) -> bool {
    matches!(directory, Some(s) if !s.is_empty())
}

// ---------------------------------------------------------------------------
// Sync paging + prune gates (buildListParams, runSync branches)
// ---------------------------------------------------------------------------

/// Warmup page (no `start`) when forced or never synced, otherwise the
/// incremental page starting `SYNC_SAFETY_WINDOW_MS` back, clamped at `0`
/// (TS: `buildListParams`).
pub fn build_list_params(force: bool, last_synced_updated_at: f64) -> ListParams {
    if force || last_synced_updated_at == 0.0 {
        return ListParams {
            limit: INITIAL_WARMUP_LIMIT,
            start: None,
        };
    }
    let start = last_synced_updated_at - SYNC_SAFETY_WINDOW_MS;
    // Mirror JS `Math.max(0, start)`: NaN stays NaN (Rust `f64::max` would
    // return 0.0 for NaN, so branch explicitly).
    let clamped = if start.is_nan() {
        f64::NAN
    } else {
        start.max(0.0)
    };
    ListParams {
        limit: INCREMENTAL_SYNC_LIMIT,
        start: Some(clamped),
    }
}

/// Whether a sync pass prunes directories unseen in the response
/// (TS: `runSync` `shouldPrune`: forced or never synced).
pub fn should_prune(force: bool, last_synced_updated_at: f64) -> bool {
    force || last_synced_updated_at == 0.0
}

/// Whether the response hit the warmup page size and may hide older
/// sessions (TS: `runSync` `responseIsTruncated`).
pub fn is_response_truncated(fetched_count: usize) -> bool {
    fetched_count >= INITIAL_WARMUP_LIMIT
}

/// Whether a non-forced sync attempt falls inside the cooldown window
/// (TS: `syncSessionDirectoryCache` early return).
pub fn is_sync_cooldown_active(now_ms: f64, last_sync_attempt_ms: f64, force: bool) -> bool {
    !force && now_ms - last_sync_attempt_ms < SYNC_COOLDOWN_MS
}

// ---------------------------------------------------------------------------
// Virtual projects + storage roots
// ---------------------------------------------------------------------------

fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x6745_2301;
    let mut h1: u32 = 0xEFCD_AB89;
    let mut h2: u32 = 0x98BA_DCFE;
    let mut h3: u32 = 0x1032_5476;
    let mut h4: u32 = 0xC3D2_E1F0;

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        for i in 0..80 {
            let (f, k) = if i < 20 {
                ((b & c) | ((!b) & d), 0x5A82_7999)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ED9_EBA1)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC)
            } else {
                (b ^ c ^ d, 0xCA62_C1D6)
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

/// `dir_` + first 16 sha1 hex chars of the worktree
/// (TS: `createVirtualProjectId`).
pub fn create_virtual_project_id(worktree: &str) -> String {
    let digest = sha1_digest(worktree.as_bytes());
    let mut hex = String::with_capacity(40);
    for byte in digest {
        hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        hex.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap_or('0'));
    }
    format!("dir_{}", &hex[..16])
}

/// Project row for one cached directory: virtual id, worktree as both
/// worktree and name (TS: `getCachedSessionProjects` mapping).
pub fn session_directory_project(worktree: &str, last_updated: f64) -> SessionDirectoryProject {
    SessionDirectoryProject {
        id: create_virtual_project_id(worktree),
        worktree: worktree.to_string(),
        name: worktree.to_string(),
        last_updated,
    }
}

fn push_candidate_unique(candidates: &mut Vec<String>, candidate: String) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

/// Candidate storage roots from `~/.local/share/opencode` plus a
/// `.../state/opencode`-suffixed state dir remapped onto
/// `.../share/opencode`, home first, deduped
/// (TS: `getStorageRootCandidates`).
pub fn get_storage_root_candidates(home: Option<&str>, state: Option<&str>) -> Vec<String> {
    let mut candidates: Vec<String> = Vec::new();

    if let Some(h) = home {
        if !h.is_empty() {
            let normalized = h.replace('\\', "/");
            let base = normalized.trim_end_matches('/');
            let candidate = if base.is_empty() {
                "/.local/share/opencode".to_string()
            } else {
                format!("{base}/.local/share/opencode")
            };
            push_candidate_unique(&mut candidates, candidate);
        }
    }

    if let Some(s) = state {
        if !s.is_empty() {
            let normalized = s.replace('\\', "/");
            let norm = normalized.trim_end_matches('/').to_string();
            if !norm.is_empty() {
                let lower = norm.to_lowercase();
                if lower.ends_with("/state/opencode") {
                    let prefix_len = norm.len().saturating_sub("/state/opencode".len());
                    let prefix = norm[..prefix_len].to_string();
                    let trimmed = prefix.trim_end_matches('/').to_string();
                    let candidate = if trimmed.is_empty() {
                        if prefix.is_empty() {
                            "share/opencode".to_string()
                        } else {
                            "/share/opencode".to_string()
                        }
                    } else {
                        format!("{trimmed}/share/opencode")
                    };
                    push_candidate_unique(&mut candidates, candidate);
                }
            }
        }
    }

    candidates
}

/// Whether the path API exposes the getter the roots depend on
/// (TS: `getStorageRootsFromApi` missing-`get` gate).
pub fn path_api_available(has_get: bool) -> bool {
    has_get
}

/// Sqlite `updated` mapping: finite numbers pass through, anything else
/// becomes `0` (TS: `querySessionDirectoriesFromSqlite` row mapping).
pub fn normalize_sqlite_updated(updated: Option<f64>) -> f64 {
    match updated {
        Some(v) if v.is_finite() => v,
        _ => 0.0,
    }
}
