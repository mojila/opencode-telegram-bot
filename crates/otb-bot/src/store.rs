//! Session/project directory cache backed by rusqlite (T5).
//!
//! Node parity (`src/app/services/session-cache-service.ts`,
//! `src/app/stores/settings-store.ts`, `src/app/types/session.ts`):
//! - The bot's own cache shape is `SessionDirectoryCacheInfo`
//!   (`version: 1`, `lastSyncedUpdatedAt`, `directories: [{ worktree,
//!   lastUpdated }]`), capped at 10 entries, deduped, newest-first.
//! - `better-sqlite3` is only ever a READ-ONLY fallback over the external
//!   OpenCode storage DB (`<share>/opencode/opencode.db`):
//!   `SELECT directory, MAX(time_updated) FROM session GROUP BY directory
//!   ORDER BY updated DESC LIMIT 200`.
//!
//! ASSUMPTIONS (explicit):
//! 1. This store is the Rust-side equivalent of the settings.json cache,
//!    not a copy of `opencode.db`. It never opens a live path for writing:
//!    `resolve_db_path` uses `OTB_DB_PATH` when set (non-blank), else
//!    `<app_home>/.otb-bot-test.db`. Live `settings.json` / `opencode.db`
//!    paths are never written and never migrated.
//! 2. Project rows mirror Node's virtual projects
//!    (`id = "dir_" + sha1(worktree)[0..16]`). To avoid a new hash crate the
//!    Rust id uses FNV-1a 64-bit hex (same `dir_<16 hex>` shape, different
//!    digest). Only the id format's shape is parity; values differ from Node.
//! 3. Windows case-insensitivity (`worktree.toLowerCase()` key in Node) is
//!    mirrored with `cfg!(windows)` lowercasing in `worktree_key`.
//! 4. `rusqlite` is `bundled` (see `crates/otb-bot/Cargo.toml`), so no
//!    system SQLite is required on any OS.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

/// Env override for the isolated cache DB path (T5).
pub const DB_PATH_ENV: &str = "OTB_DB_PATH";

/// Isolated cache file name under the App Home (never a live path).
pub const ISOLATED_DB_FILE: &str = ".otb-bot-test.db";

/// Cache schema version (Node: `CACHE_VERSION = 1`).
pub const CACHE_VERSION: i64 = 1;

/// Max cached directories (Node: `MAX_CACHED_DIRECTORIES = 10`).
pub const MAX_CACHED_DIRECTORIES: usize = 10;

/// SQLite fallback row cap (Node: `SQLITE_FALLBACK_QUERY_LIMIT = 200`).
pub const FALLBACK_QUERY_LIMIT: i64 = 200;

/// Meta key holding `lastSyncedUpdatedAt`.
const META_LAST_SYNCED_KEY: &str = "last_synced_updated_at";

/// One cached worktree (Node: `CachedSessionDirectory`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedSessionDirectory {
    pub worktree: String,
    pub last_updated: i64,
}

/// One cached project (Node: `SessionDirectoryProject`, virtual id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDirectoryProject {
    pub id: String,
    pub worktree: String,
    pub name: String,
    pub last_updated: i64,
}

/// Resolve the isolated DB path: `OTB_DB_PATH` wins, else
/// `<app_home>/.otb-bot-test.db`. Reads env only; no I/O.
pub fn resolve_db_path(app_home: &Path) -> PathBuf {
    match std::env::var(DB_PATH_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(raw) => PathBuf::from(raw.trim()),
        None => app_home.join(ISOLATED_DB_FILE),
    }
}

/// Comparison key for a worktree (Node: lowercase on win32 only).
fn worktree_key(worktree: &str) -> String {
    if cfg!(windows) {
        worktree.to_lowercase()
    } else {
        worktree.to_string()
    }
}

/// Validity gate (Node: `isValidWorktree`).
fn is_valid_worktree(worktree: &str) -> bool {
    let trimmed = worktree.trim();
    !trimmed.is_empty() && trimmed != "/"
}

/// Virtual project id for a worktree (`dir_<16 hex>`, FNV-1a; see header).
pub fn virtual_project_id(worktree: &str) -> String {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in worktree.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("dir_{hash:016x}")
}

/// rusqlite-backed session/project directory cache.
pub struct SessionCacheStore {
    conn: Connection,
}

impl SessionCacheStore {
    /// Open (or create) the isolated cache DB and ensure the schema.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Open an in-memory DB (tests).
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS session_directories (
               worktree TEXT PRIMARY KEY,
               last_updated INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS session_projects (
               id TEXT PRIMARY KEY,
               worktree TEXT NOT NULL,
               name TEXT NOT NULL,
               last_updated INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS cache_meta (
               key TEXT PRIMARY KEY,
               value INTEGER NOT NULL
             );",
        )
    }

    /// Upsert a directory; keeps the newer timestamp, trims to the newest
    /// 10, and mirrors the row into `session_projects`. Returns true when
    /// the cache changed (Node: `upsertDirectory` boolean).
    pub fn upsert_directory(
        &mut self,
        worktree: &str,
        last_updated: i64,
    ) -> Result<bool, rusqlite::Error> {
        if !is_valid_worktree(worktree) {
            return Ok(false);
        }
        let normalized = worktree.trim().to_string();
        let key = worktree_key(&normalized);

        let rows: Vec<(String, i64)> = self
            .conn
            .prepare("SELECT worktree, last_updated FROM session_directories")?
            .query_map([], |row| {
                let stored: String = row.get(0)?;
                let updated: i64 = row.get(1)?;
                Ok((stored, updated))
            })?
            .filter_map(Result::ok)
            .collect();
        let existing = rows
            .into_iter()
            .find(|(stored, _)| worktree_key(stored) == key);

        if let Some((stored, updated)) = existing {
            if updated >= last_updated {
                return Ok(false);
            }
            let tx = self.conn.transaction()?;
            tx.execute(
                "DELETE FROM session_directories WHERE worktree = ?1",
                params![stored],
            )?;
            tx.execute(
                "INSERT INTO session_directories (worktree, last_updated) VALUES (?1, ?2)",
                params![stored, last_updated],
            )?;
            let project_id = virtual_project_id(&stored);
            tx.execute(
                "INSERT INTO session_projects (id, worktree, name, last_updated)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET last_updated = excluded.last_updated",
                params![project_id, stored, stored, last_updated],
            )?;
            tx.commit()?;
        } else {
            let tx = self.conn.transaction()?;
            tx.execute(
                "INSERT INTO session_directories (worktree, last_updated) VALUES (?1, ?2)",
                params![normalized, last_updated],
            )?;
            let project_id = virtual_project_id(&normalized);
            tx.execute(
                "INSERT INTO session_projects (id, worktree, name, last_updated)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET last_updated = excluded.last_updated",
                params![project_id, normalized, normalized, last_updated],
            )?;
            tx.commit()?;
        }

        // Trim to the newest MAX_CACHED_DIRECTORIES entries.
        self.conn.execute(
            "DELETE FROM session_directories WHERE worktree NOT IN (
               SELECT worktree FROM session_directories
               ORDER BY last_updated DESC LIMIT ?1
             )",
            params![MAX_CACHED_DIRECTORIES as i64],
        )?;
        self.conn.execute(
            "DELETE FROM session_projects WHERE worktree NOT IN (
               SELECT worktree FROM session_directories
             )",
            [],
        )?;

        // Bump last-synced watermark like Node's `maxUpdated` tracking.
        let current = self.get_last_synced()?;
        if last_updated > current {
            self.set_last_synced(last_updated)?;
        }
        Ok(true)
    }

    /// Cached directories, newest-first (Node: sorted `directories`).
    pub fn get_directories(&self) -> Result<Vec<CachedSessionDirectory>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT worktree, last_updated FROM session_directories ORDER BY last_updated DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let worktree: String = row.get(0)?;
            let last_updated: i64 = row.get(1)?;
            Ok(CachedSessionDirectory {
                worktree,
                last_updated,
            })
        })?;
        rows.collect()
    }

    /// Cached projects, newest-first (Node: `getCachedSessionProjects`).
    pub fn get_projects(&self) -> Result<Vec<SessionDirectoryProject>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, worktree, name, last_updated FROM session_projects ORDER BY last_updated DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let worktree: String = row.get(1)?;
            let name: String = row.get(2)?;
            let last_updated: i64 = row.get(3)?;
            Ok(SessionDirectoryProject {
                id,
                worktree,
                name,
                last_updated,
            })
        })?;
        rows.collect()
    }

    /// Last-synced watermark (Node: `lastSyncedUpdatedAt`, default 0).
    pub fn get_last_synced(&self) -> Result<i64, rusqlite::Error> {
        let value: Option<i64> = self
            .conn
            .query_row(
                "SELECT value FROM cache_meta WHERE key = ?1",
                params![META_LAST_SYNCED_KEY],
                |row| row.get(0),
            )
            .ok();
        Ok(value.unwrap_or(0))
    }

    /// Overwrite the last-synced watermark.
    pub fn set_last_synced(&self, value: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO cache_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![META_LAST_SYNCED_KEY, value],
        )?;
        Ok(())
    }
}

/// Read-only scan of an EXTERNAL OpenCode storage DB (never the cache DB).
///
/// Mirrors the Node `better-sqlite3` fallback query over the `session` table.
/// Opens read-only; returns `None` when the file/table is unusable.
pub fn query_external_session_directories(
    db_path: &Path,
    limit: i64,
) -> Option<Vec<CachedSessionDirectory>> {
    let conn = Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT directory, MAX(time_updated) AS updated
             FROM session
             GROUP BY directory
             ORDER BY updated DESC
             LIMIT ?1",
        )
        .ok()?;
    let rows = stmt
        .query_map(params![limit], |row| {
            let directory: String = row.get(0)?;
            let updated: Option<i64> = row.get(1)?;
            Ok(CachedSessionDirectory {
                worktree: directory,
                last_updated: updated.unwrap_or(0),
            })
        })
        .ok()?
        .filter_map(Result::ok)
        .filter(|item| is_valid_worktree(&item.worktree))
        .collect::<Vec<_>>();
    Some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_keeps_newer_and_lists_newest_first() {
        let mut store = SessionCacheStore::open_in_memory().expect("memory db");
        assert!(store.upsert_directory("/work/a", 100).expect("upsert"));
        assert!(store.upsert_directory("/work/b", 300).expect("upsert"));
        // Stale write is ignored.
        assert!(!store.upsert_directory("/work/a", 50).expect("upsert"));
        // Invalid worktrees are ignored.
        assert!(!store.upsert_directory("   ", 400).expect("upsert"));
        assert!(!store.upsert_directory("/", 400).expect("upsert"));

        let dirs = store.get_directories().expect("dirs");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].worktree, "/work/b");
        assert_eq!(dirs[1].worktree, "/work/a");
        assert_eq!(store.get_last_synced().expect("watermark"), 300);

        let projects = store.get_projects().expect("projects");
        assert_eq!(projects.len(), 2);
        assert!(projects[0].id.starts_with("dir_"));
        assert_eq!(projects[0].worktree, "/work/b");
    }

    #[test]
    fn cache_trims_to_ten_newest_entries() {
        let mut store = SessionCacheStore::open_in_memory().expect("memory db");
        for i in 0..12 {
            store
                .upsert_directory(&format!("/work/{i:02}"), 100 + i)
                .expect("upsert");
        }
        let dirs = store.get_directories().expect("dirs");
        assert_eq!(dirs.len(), MAX_CACHED_DIRECTORIES);
        assert_eq!(dirs[0].worktree, "/work/11");
        assert!(!dirs.iter().any(|d| d.worktree == "/work/00"));
        assert_eq!(store.get_projects().expect("projects").len(), 10);
    }

    #[test]
    fn external_query_matches_node_fallback_sql() {
        let file = std::env::temp_dir().join(format!(
            "otb-store-test-{}.db",
            std::process::id()
        ));
        // Seed a temp-file DB so the read-only helper has a real file to
        // open (isolated temp path, never a live DB).
        {
            let conn = Connection::open(&file).expect("temp db");
            conn.execute_batch(
                "CREATE TABLE session (directory TEXT, time_updated INTEGER);
                 INSERT INTO session VALUES ('/work/a', 10), ('/work/a', 30), ('/work/b', 20);",
            )
            .expect("seed");
        }
        let rows =
            query_external_session_directories(&file, FALLBACK_QUERY_LIMIT).expect("rows");
        std::fs::remove_file(&file).ok();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].worktree, "/work/a");
        assert_eq!(rows[0].last_updated, 30);
        assert_eq!(rows[1].worktree, "/work/b");
    }
}
