//! App config: `.env` keys, defaults, and App Home paths (T2).
//!
//! Node parity (`src/config.ts`, `src/utils/logger.ts`, `src/runtime/paths.ts`):
//! - `.env` keys: `LOG_LEVEL` (default `info`), `LOG_RETENTION` (default 10),
//!   `OPENCODE_SERVER_VERSION` (default `v1`), `OPENCODE_SERVER_MODE`,
//!   `OPENCODE_API_URL`, `BOT_LOCALE` (default `en`),
//!   `BOT_HEALTH_PORT` (default off; 3100 in a container).
//! - `LOG_LEVEL` mapping: `debug|info|warn|error`, anything else -> `info`.
//! - App Home: `OPENCODE_TELEGRAM_HOME` override wins; else the installed App
//!   Home (`~/Library/Application Support/opencode-telegram-bot` on macOS,
//!   `%APPDATA%` on Windows, `$XDG_CONFIG_HOME` elsewhere) when the binary
//!   looks installed or that home already holds a `.env` file; otherwise the
//!   current directory (sources-mode parity). No files are created or written
//!   here.

use std::path::{Path, PathBuf};

/// Default page size for the `/projects` list
/// (Node: `config.bot.projectsListLimit`, default 10).
pub const PROJECTS_LIST_LIMIT: usize = 10;

/// Default page size for the `/messages` list
/// (Node: `config.bot.messagesListLimit`, default 10).
pub const MESSAGES_LIST_LIMIT: usize = 10;

/// Default page size for the `/commands` and `/skills` lists
/// (Node: `config.bot.commandsListLimit`, default 10).
pub const CATALOG_LIST_LIMIT: usize = 10;

/// Default page size for the `/model` provider/model browsers
/// (Node: `config.bot.modelsListLimit`, default 10).
pub const MODELS_LIST_LIMIT: usize = 10;

/// Default page size for the `/sessions` and `/recent` lists
/// (Node: `config.bot.sessionsListLimit`, default 10).
pub const SESSIONS_LIST_LIMIT: usize = 10;

/// Supported log levels (Node: `LogLevel` in `src/utils/logger.ts`).
pub const LOG_LEVELS: &[&str] = &["debug", "info", "warn", "error"];

/// Default log level (Node: `DEFAULT_LOG_LEVEL = "info"`).
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// Default log retention (Node: `DEFAULT_LOG_RETENTION = 10`).
pub const DEFAULT_LOG_RETENTION: i64 = 10;

/// Normalize a raw `LOG_LEVEL` value; unknown values fall back to `info`.
pub fn normalize_log_level(raw: Option<&str>) -> String {
    match raw {
        Some(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            if LOG_LEVELS.contains(&normalized.as_str()) {
                return normalized;
            }
            DEFAULT_LOG_LEVEL.to_string()
        }
        None => DEFAULT_LOG_LEVEL.to_string(),
    }
}

/// Minimal app config for the T2 scaffold.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Normalized `debug|info|warn|error` (default `info`).
    pub log_level: String,
    /// `LOG_RETENTION` (default 10).
    pub log_retention: i64,
    /// Resolved App Home (override or current directory).
    pub app_home: PathBuf,
    /// `<app_home>/.env` path (resolved only, never written).
    pub env_file: PathBuf,
    /// `v1` or `v2` (default `v1`).
    pub opencode_server_version: String,
    /// `OPENCODE_SERVER_USERNAME` (default `opencode`, Node: `config.opencode.username`).
    pub opencode_server_username: String,
    /// `OPENCODE_SERVER_PASSWORD` (Node: `config.opencode.password`); `None`
    /// when unset or empty. The value itself is never logged.
    pub opencode_server_password: Option<String>,
    /// `service` or `standalone`.
    pub opencode_server_mode: String,
    /// Resolved API URL with version defaults.
    pub opencode_api_url: String,
    /// `BOT_LOCALE` (default `en`).
    pub bot_locale: String,
    /// `BOT_HEALTH_PORT` (default off; 3100 in a container; 0 disables).
    pub health_port: u16,
}

/// App directory name (Node: `APP_DIR_NAME` in `src/runtime/paths.ts`).
pub const APP_DIR_NAME: &str = "opencode-telegram-bot";

/// Installed App Home for a given home directory
/// (Node: `getInstalledAppHome` in `src/runtime/paths.ts`).
/// Pure: no env reads, no I/O.
pub fn installed_app_home_from_parts(
    home_dir: &str,
    xdg_config_home: Option<&str>,
    app_data: Option<&str>,
) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = app_data
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(home_dir).join("AppData").join("Roaming"));
        base.join(APP_DIR_NAME)
    }
    #[cfg(target_os = "macos")]
    {
        let _ = (xdg_config_home, app_data);
        Path::new(home_dir)
            .join("Library")
            .join("Application Support")
            .join(APP_DIR_NAME)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = app_data;
        let base = xdg_config_home
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(home_dir).join(".config"));
        base.join(APP_DIR_NAME)
    }
}

/// Installed App Home from the live environment (no I/O).
pub fn installed_app_home() -> PathBuf {
    let home = std::env::var("HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| ".".to_string());
    let xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let app_data = std::env::var("APPDATA").ok();
    installed_app_home_from_parts(&home, xdg.as_deref(), app_data.as_deref())
}

/// True when an executable path looks like an installed binary (Node
/// `installed`-mode heuristic): under `/opt/homebrew` or `/usr/local`, or
/// anywhere outside a `Projects` checkout (a `Projects` segment means a
/// sources checkout). Bare names (PATH-resolved, e.g. launchd) count as
/// installed; an empty path does not.
pub fn exe_looks_installed_with_path(exe_path: &str) -> bool {
    if exe_path.trim().is_empty() {
        return false;
    }
    let normalized = exe_path.replace('\\', "/");
    if normalized.starts_with("/opt/homebrew/") || normalized.starts_with("/usr/local/") {
        return true;
    }
    if normalized.contains("/Projects/") {
        return false;
    }
    true
}

/// Pure App Home decision (no env reads, no I/O):
/// `OPENCODE_TELEGRAM_HOME` (non-blank) wins — absolute as-is, relative
/// resolved against `cwd` — else the installed home when the binary looks
/// installed or that home already holds a `.env` file, else `cwd`.
pub fn resolve_app_home_from(
    home_override: Option<&str>,
    cwd: &Path,
    exe_path: &str,
    installed_home: &Path,
    installed_env_exists: bool,
) -> PathBuf {
    if let Some(raw) = home_override {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let path = Path::new(trimmed);
            if path.is_absolute() {
                return path.to_path_buf();
            }
            return cwd.join(path);
        }
    }
    if exe_looks_installed_with_path(exe_path) || installed_env_exists {
        return installed_home.to_path_buf();
    }
    cwd.to_path_buf()
}

/// Resolve the App Home without writing anything.
///
/// `OPENCODE_TELEGRAM_HOME` (non-blank) wins; otherwise the installed App Home
/// when the current binary looks installed (`/opt/homebrew`, `/usr/local`, or
/// outside a `Projects` checkout) or that home already contains a `.env`
/// file; otherwise the current directory (sources-mode parity).
pub fn resolve_app_home() -> PathBuf {
    let home_override = std::env::var("OPENCODE_TELEGRAM_HOME").ok();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let exe_path = std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| std::env::args().next().unwrap_or_default());
    let installed_home = installed_app_home();
    let installed_env_exists = installed_home.join(".env").is_file();
    resolve_app_home_from(
        home_override.as_deref(),
        &cwd,
        &exe_path,
        &installed_home,
        installed_env_exists,
    )
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

impl AppConfig {
    /// Load config from the environment.
    ///
    /// Loads `<app_home>/.env` first (when the file exists) and then the
    /// current-directory `.env` as a fallback; both optional and read-only.
    /// First load wins in `dotenvy`, so App Home takes precedence over a
    /// CWD-local file; existing process env always wins over both.
    /// Never writes files and never logs secret values.
    pub fn load() -> Result<Self, String> {
        let app_home = resolve_app_home();
        let env_file = app_home.join(".env");
        // Installed parity: launchd runs with `CWD=/`, so the App Home file
        // must load regardless of CWD. Missing files are fine.
        if env_file.is_file() {
            let _ = dotenvy::from_path(&env_file).ok();
        }
        let _ = dotenvy::dotenv().ok();

        let log_level = normalize_log_level(env_opt("LOG_LEVEL").as_deref());
        let log_retention = otb_core::types_config::parse_positive_int_env(
            env_opt("LOG_RETENTION").as_deref(),
            DEFAULT_LOG_RETENTION,
        );
        let server_version = otb_core::types_config::resolve_server_version(
            env_opt("OPENCODE_SERVER_VERSION").as_deref(),
        )
        .to_string();
        let server_mode = otb_core::types_config::resolve_server_mode(
            &server_version,
            env_opt("OPENCODE_SERVER_MODE").as_deref(),
        )
        .map_err(|err| err.to_string())?
        .to_string();
        let api_url = otb_core::types_config::resolve_api_url(
            &server_version,
            &server_mode,
            env_opt("OPENCODE_API_URL").as_deref(),
        )
        .map_err(|err| err.to_string())?;
        let bot_locale =
            otb_core::types_config::normalize_locale_tag(env_opt("BOT_LOCALE").as_deref(), "en");
        let health_port = crate::health::resolve_health_port_from_env();

        Ok(Self {
            log_level,
            log_retention,
            app_home,
            env_file,
            opencode_server_version: server_version,
            opencode_server_username: crate::opencode::server_username_from_env(),
            opencode_server_password: crate::opencode::server_password_from_env(),
            opencode_server_mode: server_mode,
            opencode_api_url: api_url,
            bot_locale,
            health_port,
        })
    }

    /// OpenCode client for the configured server, with Basic auth attached
    /// when a password is configured (Node: per-server `getAuth`).
    /// Pure constructor; performs no I/O.
    pub fn opencode_client(&self) -> crate::opencode::OpencodeClient {
        let client = crate::opencode::OpencodeClient::new(
            &self.opencode_api_url,
            &self.opencode_server_version,
        );
        match &self.opencode_server_password {
            Some(password) => client.with_basic_auth(&self.opencode_server_username, password),
            None => client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_home(name: &str) -> PathBuf {
        // Temp-dir-based fake HOME: no live paths, no FS writes, no env mutation.
        std::env::temp_dir().join(format!("otb-config-test-{name}"))
    }

    fn installed_for(name: &str) -> PathBuf {
        let home = test_home(name);
        installed_app_home_from_parts(&home.to_string_lossy(), None, None)
    }

    #[test]
    fn app_home_override_wins_absolute() {
        let cwd = PathBuf::from("/cwd");
        let installed = installed_for("override-abs");
        let out = resolve_app_home_from(
            Some("  /tmp/custom-home  "),
            &cwd,
            "/opt/homebrew/bin/otb-bot",
            &installed,
            false,
        );
        assert_eq!(out, PathBuf::from("/tmp/custom-home"));
    }

    #[test]
    fn app_home_override_wins_relative_to_cwd() {
        let cwd = PathBuf::from("/some/cwd");
        let installed = installed_for("override-rel");
        let out = resolve_app_home_from(
            Some("custom-home"),
            &cwd,
            "/opt/homebrew/bin/otb-bot",
            &installed,
            false,
        );
        assert_eq!(out, PathBuf::from("/some/cwd/custom-home"));
    }

    #[test]
    fn app_home_blank_override_falls_through_to_installed() {
        let cwd = PathBuf::from("/");
        let installed = installed_for("override-blank");
        let out = resolve_app_home_from(
            Some("   "),
            &cwd,
            "/opt/homebrew/bin/otb-bot",
            &installed,
            false,
        );
        assert_eq!(out, installed);
    }

    #[test]
    fn app_home_installed_default_for_homebrew_binary() {
        // launchd shape: CWD=/, Homebrew binary, no override.
        let cwd = PathBuf::from("/");
        let installed = installed_for("homebrew");
        let out = resolve_app_home_from(None, &cwd, "/opt/homebrew/bin/otb-bot", &installed, false);
        assert_eq!(out, installed);
    }

    #[test]
    fn app_home_installed_env_fallback_for_sources_binary() {
        // Sources-looking binary, but the HOME-based App Support dir already
        // holds a `.env` file: prefer the installed home.
        let cwd = PathBuf::from("/");
        let installed = installed_for("env-fallback");
        let out = resolve_app_home_from(
            None,
            &cwd,
            "/Users/dev/Projects/demo/target/debug/otb-bot",
            &installed,
            true,
        );
        assert_eq!(out, installed);
    }

    #[test]
    fn app_home_sources_default_for_checkout_binary() {
        let cwd = PathBuf::from("/Users/dev/Projects/demo");
        let installed = installed_for("sources");
        let out = resolve_app_home_from(
            None,
            &cwd,
            "/Users/dev/Projects/demo/target/debug/otb-bot",
            &installed,
            false,
        );
        assert_eq!(out, cwd);
    }

    #[test]
    fn exe_looks_installed_classification() {
        assert!(exe_looks_installed_with_path("/opt/homebrew/bin/otb-bot"));
        assert!(exe_looks_installed_with_path("/usr/local/bin/otb-bot"));
        assert!(exe_looks_installed_with_path("otb-bot"));
        assert!(!exe_looks_installed_with_path(""));
        assert!(!exe_looks_installed_with_path(
            "/Users/dev/Projects/demo/target/debug/otb-bot"
        ));
    }

    #[test]
    fn installed_home_keeps_app_dir_name() {
        let installed = installed_for("dirname");
        assert_eq!(
            installed.file_name().and_then(|name| name.to_str()),
            Some(APP_DIR_NAME)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn installed_home_macos_layout() {
        let installed = installed_for("macos-layout");
        let expected = test_home("macos-layout")
            .join("Library")
            .join("Application Support")
            .join(APP_DIR_NAME);
        assert_eq!(installed, expected);
    }
}
