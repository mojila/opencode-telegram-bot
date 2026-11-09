//! Read-only view over `settings.json` `current*` fields (status parity).
//!
//! Node parity (`src/app/stores/settings-store.ts`, `src/app/types/settings.ts`):
//! - `currentAgent: string`, `currentModel: { providerID, modelID, variant? }`,
//!   `currentProject: { id, worktree, name? }`,
//!   `currentSession: { id, title, directory }`.
//! - File location: `<app_home>/settings.json`
//!   (`getRuntimePaths().settingsFilePath` parity).
//! - Tolerant reads: a missing, unreadable, or invalid file yields an empty
//!   view (all `None`) so `/status` falls back to env/placeholders.
//!
//! SAFETY (explicit): reads one local JSON file only; never writes. Only the
//! file path, error kinds, and byte lengths are ever logged — never values.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// File name of the persistent settings store inside the App Home.
pub const SETTINGS_FILE_NAME: &str = "settings.json";

/// The `current*` slice of `settings.json` the `/status` snapshot needs.
/// Every field is `None` when absent or blank (never an empty string).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsCurrent {
    /// `currentAgent` (Node `getCurrentAgent`).
    pub agent: Option<String>,
    /// `currentModel.providerID` (Node `getCurrentModel`).
    pub model_provider: Option<String>,
    /// `currentModel.modelID`.
    pub model_id: Option<String>,
    /// `currentModel.variant` (e.g. `"default"`).
    pub model_variant: Option<String>,
    /// `currentProject.worktree` (Node `getCurrentProject`).
    pub project_worktree: Option<String>,
    /// `currentProject.id`.
    pub project_id: Option<String>,
    /// `currentProject.name`.
    pub project_name: Option<String>,
    /// `currentSession.id` (Node `getCurrentSession` via session-service).
    pub session_id: Option<String>,
    /// `currentSession.title` (remembered title; the live title is a
    /// `fetchSessionTitle` follow-up).
    pub session_title: Option<String>,
    /// `currentSession.directory`.
    pub session_directory: Option<String>,
}

/// `<app_home>/settings.json` path (resolved only, never written).
pub fn settings_file_path(app_home: &Path) -> PathBuf {
    app_home.join(SETTINGS_FILE_NAME)
}

/// One trimmed non-blank string field, else `None` (pure; testable).
fn non_empty_string(value: Option<&Value>) -> Option<String> {
    value?
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// Pure parse of decoded `settings.json` content (no I/O; testable).
pub fn parse_settings_current(value: &Value) -> SettingsCurrent {
    let model = value.get("currentModel");
    let project = value.get("currentProject");
    let session = value.get("currentSession");
    SettingsCurrent {
        agent: non_empty_string(value.get("currentAgent")),
        model_provider: non_empty_string(model.and_then(|item| item.get("providerID"))),
        model_id: non_empty_string(model.and_then(|item| item.get("modelID"))),
        model_variant: non_empty_string(model.and_then(|item| item.get("variant"))),
        project_worktree: non_empty_string(project.and_then(|item| item.get("worktree"))),
        project_id: non_empty_string(project.and_then(|item| item.get("id"))),
        project_name: non_empty_string(project.and_then(|item| item.get("name"))),
        session_id: non_empty_string(session.and_then(|item| item.get("id"))),
        session_title: non_empty_string(session.and_then(|item| item.get("title"))),
        session_directory: non_empty_string(session.and_then(|item| item.get("directory"))),
    }
}

/// Tolerant read of `<app_home>/settings.json`: missing, unreadable, or
/// invalid content yields [`SettingsCurrent::default`]. Logs only the path,
/// error kind, or byte length — never file values.
pub fn read_settings_current(app_home: &Path) -> SettingsCurrent {
    let path = settings_file_path(app_home);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            tracing::debug!(
                "[otb-bot] settings: no settings file at {} ({})",
                path.display(),
                err.kind()
            );
            return SettingsCurrent::default();
        }
    };
    match serde_json::from_str::<Value>(&content) {
        Ok(value) => parse_settings_current(&value),
        Err(err) => {
            tracing::debug!(
                "[otb-bot] settings: ignoring unparseable settings file ({} bytes): {err}",
                content.len()
            );
            SettingsCurrent::default()
        }
    }
}

/// Persist `currentSession` into `<app_home>/settings.json` (JS
/// `setCurrentSession` + `writeSettingsFileAtomically` parity in
/// `session-service.ts` / `settings-store.ts`): other keys are preserved,
/// the write goes through `<file>.tmp` + rename with a `.bak` rotation, and
/// only paths/error kinds (never values) are logged.
pub fn write_current_session(
    app_home: &Path,
    id: &str,
    title: &str,
    directory: &str,
) -> Result<(), String> {
    let path = settings_file_path(app_home);
    let mut value: Value = match std::fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or(Value::Object(serde_json::Map::new()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Value::Object(serde_json::Map::new())
        }
        Err(err) => {
            tracing::warn!(
                "[otb-bot] settings: cannot read settings file at {} ({})",
                path.display(),
                err.kind()
            );
            return Err(format!("cannot read settings file: {}", err.kind()));
        }
    };
    if !value.is_object() {
        value = Value::Object(serde_json::Map::new());
    }
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "currentSession".to_string(),
            serde_json::json!({ "id": id, "title": title, "directory": directory }),
        );
    }
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "[otb-bot] settings: cannot create settings dir at {} ({})",
                parent.display(),
                err.kind()
            );
            return Err(format!("cannot create settings dir: {}", err.kind()));
        }
    }
    persist_settings_value(&path, &value)
}

/// Persist a decoded `settings.json` value atomically through `<file>.tmp`
/// + rename with a `.bak` rotation (Node `writeSettingsFileAtomically`
/// parity). Only paths/error kinds (never values) are logged.
fn persist_settings_value(path: &std::path::PathBuf, value: &Value) -> Result<(), String> {
    let tmp_path = path.with_extension("json.tmp");
    let bak_path = path.with_extension("json.bak");
    let serialized = serde_json::to_string_pretty(value)
        .map_err(|err| format!("cannot serialize settings: {err}"))?;
    if let Err(err) = std::fs::write(&tmp_path, serialized) {
        let _ = std::fs::remove_file(&tmp_path);
        tracing::warn!(
            "[otb-bot] settings: cannot write temp file ({})",
            err.kind()
        );
        return Err(format!("cannot write settings: {}", err.kind()));
    }
    // Nothing to back up on the very first write (Node parity).
    if path.exists() {
        // Best effort: a stale backup must not block the live write.
        let _ = std::fs::remove_file(&bak_path);
        if let Err(err) = std::fs::rename(path, &bak_path) {
            let _ = std::fs::remove_file(&tmp_path);
            tracing::warn!("[otb-bot] settings: cannot rotate backup ({})", err.kind());
            return Err(format!("cannot rotate settings backup: {}", err.kind()));
        }
    }
    if let Err(err) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        tracing::warn!(
            "[otb-bot] settings: cannot replace settings file ({})",
            err.kind()
        );
        return Err(format!("cannot write settings: {}", err.kind()));
    }
    Ok(())
}

/// Persist `currentProject` into `<app_home>/settings.json` and drop
/// `currentSession` (JS `setCurrentProject` session-reset parity):
/// other keys are preserved and the write is atomic like
/// [`write_current_session`].
pub fn write_current_project(
    app_home: &Path,
    id: &str,
    worktree: &str,
    name: &str,
) -> Result<(), String> {
    let path = settings_file_path(app_home);
    let mut value: Value = match std::fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or(Value::Object(serde_json::Map::new()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Value::Object(serde_json::Map::new())
        }
        Err(err) => {
            tracing::warn!(
                "[otb-bot] settings: cannot read settings file at {} ({})",
                path.display(),
                err.kind()
            );
            return Err(format!("cannot read settings file: {}", err.kind()));
        }
    };
    if !value.is_object() {
        value = Value::Object(serde_json::Map::new());
    }
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "currentProject".to_string(),
            serde_json::json!({ "id": id, "worktree": worktree, "name": name }),
        );
        object.remove("currentSession");
    }
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "[otb-bot] settings: cannot create settings dir at {} ({})",
                parent.display(),
                err.kind()
            );
            return Err(format!("cannot create settings dir: {}", err.kind()));
        }
    }
    persist_settings_value(&path, &value)
}

/// Persist `currentModel` into `<app_home>/settings.json` (JS
/// `setCurrentModel` parity in `settings-store.ts`): other keys are
/// preserved and the write is atomic like [`write_current_session`].
/// `variant` is stored verbatim (usually `"default"`); pass `None` to drop
/// it. Only paths/error kinds (never values) are logged.
pub fn write_current_model(
    app_home: &Path,
    provider_id: &str,
    model_id: &str,
    variant: Option<&str>,
) -> Result<(), String> {
    let path = settings_file_path(app_home);
    let mut value: Value = match std::fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or(Value::Object(serde_json::Map::new()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Value::Object(serde_json::Map::new())
        }
        Err(err) => {
            tracing::warn!(
                "[otb-bot] settings: cannot read settings file at {} ({})",
                path.display(),
                err.kind()
            );
            return Err(format!("cannot read settings file: {}", err.kind()));
        }
    };
    if !value.is_object() {
        value = Value::Object(serde_json::Map::new());
    }
    if let Some(object) = value.as_object_mut() {
        let mut model = serde_json::json!({ "providerID": provider_id, "modelID": model_id });
        if let Some(variant) = variant.map(str::trim).filter(|variant| !variant.is_empty()) {
            model["variant"] = Value::String(variant.to_string());
        }
        object.insert("currentModel".to_string(), model);
    }
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "[otb-bot] settings: cannot create settings dir at {} ({})",
                parent.display(),
                err.kind()
            );
            return Err(format!("cannot create settings dir: {}", err.kind()));
        }
    }
    persist_settings_value(&path, &value)
}

/// Persist `currentAgent` into `<app_home>/settings.json` (JS
/// `setCurrentAgent` parity in `settings-store.ts`): other keys are
/// preserved and the write is atomic like [`write_current_session`].
/// Only paths/error kinds (never values) are logged.
pub fn write_current_agent(app_home: &Path, name: &str) -> Result<(), String> {
    let path = settings_file_path(app_home);
    let mut value: Value = match std::fs::read_to_string(&path) {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or(Value::Object(serde_json::Map::new()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Value::Object(serde_json::Map::new())
        }
        Err(err) => {
            tracing::warn!(
                "[otb-bot] settings: cannot read settings file at {} ({})",
                path.display(),
                err.kind()
            );
            return Err(format!("cannot read settings file: {}", err.kind()));
        }
    };
    if !value.is_object() {
        value = Value::Object(serde_json::Map::new());
    }
    if let Some(object) = value.as_object_mut() {
        object.insert("currentAgent".to_string(), Value::String(name.to_string()));
    }
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "[otb-bot] settings: cannot create settings dir at {} ({})",
                parent.display(),
                err.kind()
            );
            return Err(format!("cannot create settings dir: {}", err.kind()));
        }
    }
    persist_settings_value(&path, &value)
}

/// Drop `currentSession` from `<app_home>/settings.json` (JS `clearSession`
/// parity in `session-service.ts`, the local-only half of `/detach`):
/// other keys are preserved and the write is atomic like
/// [`write_current_session`]. A missing file is a no-op success; only paths,
/// error kinds, and byte lengths are ever logged — never values.
pub fn clear_current_session(app_home: &Path) -> Result<(), String> {
    let path = settings_file_path(app_home);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            tracing::warn!(
                "[otb-bot] settings: cannot read settings file at {} ({})",
                path.display(),
                err.kind()
            );
            return Err(format!("cannot read settings file: {}", err.kind()));
        }
    };
    let mut value: Value =
        serde_json::from_str(&content).unwrap_or(Value::Object(serde_json::Map::new()));
    if !value.is_object() {
        value = Value::Object(serde_json::Map::new());
    }
    if let Some(object) = value.as_object_mut() {
        object.remove("currentSession");
    }
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "[otb-bot] settings: cannot create settings dir at {} ({})",
                parent.display(),
                err.kind()
            );
            return Err(format!("cannot create settings dir: {}", err.kind()));
        }
    }
    persist_settings_value(&path, &value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_fields_and_drops_blanks() {
        let value = serde_json::json!({
            "currentAgent": "orchestrator",
            "currentModel": {
                "providerID": "meta",
                "modelID": "muse-spark-1.3-contributor",
                "variant": "default"
            },
            "currentProject": {
                "id": "dir_abc123",
                "worktree": "/work/opencode-telegram-bot",
                "name": "opencode-telegram-bot"
            },
            "currentSession": {
                "id": "ses_1",
                "title": "Fix status",
                "directory": "/work/opencode-telegram-bot"
            },
            "ttsMode": "off"
        });
        let current = parse_settings_current(&value);
        assert_eq!(current.agent.as_deref(), Some("orchestrator"));
        assert_eq!(current.model_provider.as_deref(), Some("meta"));
        assert_eq!(
            current.model_id.as_deref(),
            Some("muse-spark-1.3-contributor")
        );
        assert_eq!(current.model_variant.as_deref(), Some("default"));
        assert_eq!(
            current.project_worktree.as_deref(),
            Some("/work/opencode-telegram-bot")
        );
        assert_eq!(current.session_title.as_deref(), Some("Fix status"));

        // Blank strings and missing objects collapse to `None`.
        let sparse = parse_settings_current(&serde_json::json!({
            "currentAgent": "   ",
            "currentModel": { "providerID": "", "modelID": "  " },
        }));
        assert_eq!(sparse, SettingsCurrent::default());
    }

    #[test]
    fn missing_or_invalid_file_yields_empty_view() {
        let missing = PathBuf::from("/definitely/not/here/otb-test-home");
        assert_eq!(read_settings_current(&missing), SettingsCurrent::default());

        let dir = std::env::temp_dir().join(format!("otb-settings-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join(SETTINGS_FILE_NAME), "{not json").expect("seed invalid");
        assert_eq!(read_settings_current(&dir), SettingsCurrent::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_current_session_round_trips_atomically() {
        // Temp-dir App Home only; removed afterwards (never a live path).
        let dir =
            std::env::temp_dir().join(format!("otb-settings-write-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join(SETTINGS_FILE_NAME),
            serde_json::json!({ "currentProject": { "worktree": "/work/proj" } }).to_string(),
        )
        .expect("seed settings");

        write_current_session(&dir, "ses_1", "Fix bug", "/work/proj").expect("write");
        let current = read_settings_current(&dir);
        assert_eq!(current.session_id.as_deref(), Some("ses_1"));
        assert_eq!(current.session_title.as_deref(), Some("Fix bug"));
        assert_eq!(current.session_directory.as_deref(), Some("/work/proj"));
        // Unrelated keys are preserved (Node parity).
        assert_eq!(current.project_worktree.as_deref(), Some("/work/proj"));

        // Second write rotates the previous file to `.bak`.
        write_current_session(&dir, "ses_2", "Next", "/work/proj").expect("rewrite");
        let bak = std::fs::read_to_string(dir.join("settings.json.bak")).expect("backup");
        assert!(bak.contains("ses_1"));
        let current = read_settings_current(&dir);
        assert_eq!(current.session_id.as_deref(), Some("ses_2"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_current_project_round_trips_and_resets_session() {
        // Temp-dir App Home only; removed afterwards (never a live path).
        let dir =
            std::env::temp_dir().join(format!("otb-settings-project-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join(SETTINGS_FILE_NAME),
            serde_json::json!({
                "currentSession": { "id": "ses_1", "title": "Old", "directory": "/work/old" },
                "ttsMode": "off",
            })
            .to_string(),
        )
        .expect("seed settings");

        write_current_project(&dir, "dir_abc", "/work/new", "new").expect("write");
        let current = read_settings_current(&dir);
        assert_eq!(current.project_id.as_deref(), Some("dir_abc"));
        assert_eq!(current.project_worktree.as_deref(), Some("/work/new"));
        assert_eq!(current.project_name.as_deref(), Some("new"));
        // Selecting a project resets the session (Node `setCurrentProject` parity).
        assert_eq!(current.session_id, None);

        // Re-selecting another worktree keeps the atomic backup rotation.
        write_current_project(&dir, "dir_def", "/work/other", "other").expect("rewrite");
        let bak = std::fs::read_to_string(dir.join("settings.json.bak")).expect("backup");
        assert!(bak.contains("/work/new"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn clear_current_session_drops_only_the_session() {
        // Temp-dir App Home only; removed afterwards (never a live path).
        let dir =
            std::env::temp_dir().join(format!("otb-settings-clear-test-{}", std::process::id()));
        // A missing file is a no-op success (already detached).
        clear_current_session(&dir).expect("missing file clears");
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join(SETTINGS_FILE_NAME),
            serde_json::json!({
                "currentProject": { "worktree": "/work/proj" },
                "currentSession": { "id": "ses_1", "title": "Old", "directory": "/work/proj" },
            })
            .to_string(),
        )
        .expect("seed settings");

        clear_current_session(&dir).expect("clear");
        let current = read_settings_current(&dir);
        assert_eq!(current.session_id, None);
        // The project survives the detach (Node `clearSession` parity).
        assert_eq!(current.project_worktree.as_deref(), Some("/work/proj"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
