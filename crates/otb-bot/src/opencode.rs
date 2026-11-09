//! OpenCode V1/V2 HTTP client + SSE subscribe with reconnect backoff (T4).
//!
//! Node parity (`src/opencode/client.ts`, `src/opencode/events.ts`):
//! - Health probe: V1 `GET global/health`, V2 `GET api/info`, 5s timeout,
//!   with Basic auth from `OPENCODE_SERVER_USERNAME`/`OPENCODE_SERVER_PASSWORD`
//!   when a password is set (Node: `getAuth`/`probeOpencodeServer`).
//! - Reconnect backoff `1000 * 2^(attempt-1)` capped at 15000ms with a
//!   `warn` log on every drop (the pure math lives in
//!   `otb_core::event_transport_gates::get_reconnect_delay_ms`).
//! - SSE idle timeout 30s (`sseIdleTimeoutMs`): a quiet stream is dropped
//!   and reconnected like any other disconnect.
//! - `server.connected` carries the reconnect story; the `restarted`
//!   property becomes `ReconnectInfo.server_restarted` (else `None`).
//!
//! ASSUMPTIONS (explicit):
//! 1. Base URL: `OPENCODE_URL` wins (T4), else `OPENCODE_API_URL` (Node
//!    parity), else `http://127.0.0.1:4096`. Never starts/stops the server.
//! 2. Only the probe routes are known exactly (`global/health`,
//!    `api/info`). V1 session/event paths (`session`, `event`) follow the
//!    SDK method names; V2 paths (`api/session`, `api/event`) additionally
//!    assume the `api/` prefix the probe route uses. If the live server
//!    answers elsewhere, only these path constants need to change.
//! 3. Legacy/V1 subscribe passes `?directory=`; V2 subscribes globally and
//!    filters client-side (mirrors `normalizeGlobalEvent` directory drop).
//! 4. `create_session` / `send_prompt` exist for API parity only. Nothing
//!    in the scaffold calls them: the default path is read-only (health,
//!    session list, event stream). No Telegram polling, no writes.
//!
//! Pure helpers (`resolve_base_url`, path builders, SSE frame splitter,
//! auth header) are unit-tested below. Network I/O is plain functions with
//! no auto-call: constructing the client performs zero requests.

use std::time::Duration;

use serde_json::Value;

/// Env var carrying the OpenCode base URL (T4).
pub const OPENCODE_URL_ENV: &str = "OPENCODE_URL";

/// Env var carrying the OpenCode base URL (Node parity fallback).
pub const OPENCODE_API_URL_ENV: &str = "OPENCODE_API_URL";

/// Default base URL when neither env var is set (T4).
pub const DEFAULT_OPENCODE_URL: &str = "http://127.0.0.1:4096";

/// Health-probe timeout (Node: `PROBE_TIMEOUT_MS`).
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(5_000);

/// SSE idle timeout: a quiet stream is dropped and reconnected
/// (Node: `sseIdleTimeoutMs = 30_000`).
pub const SSE_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Resolve the base URL from an explicit value (pure; testable).
/// Empty/whitespace counts as missing.
pub fn resolve_base_url(explicit: Option<&str>) -> String {
    match explicit {
        Some(raw) if !raw.trim().is_empty() => raw.trim().trim_end_matches('/').to_string(),
        _ => DEFAULT_OPENCODE_URL.to_string(),
    }
}

/// Base URL from the environment: `OPENCODE_URL`, else `OPENCODE_API_URL`,
/// else the default. Reads env only; performs no I/O.
pub fn base_url_from_env() -> String {
    let explicit = std::env::var(OPENCODE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var(OPENCODE_API_URL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
    resolve_base_url(explicit.as_deref())
}

/// Server version from the environment (`v2` else `v1`, Node parity).
pub fn server_version_from_env() -> &'static str {
    let raw = std::env::var("OPENCODE_SERVER_VERSION").ok();
    otb_core::types_config::resolve_server_version(raw.as_deref())
}

/// Health-probe path per server version (Node: `PROBE_ROUTES`).
pub fn health_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/info"
    } else {
        "global/health"
    }
}

/// Project-list path per server version (Node SDK `project.list` parity;
/// the V2 `api/` prefix is assumed like the probe route, see header).
pub fn project_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/project"
    } else {
        "project"
    }
}

/// Session list/create path per server version (V2 path assumed, see header).
pub fn session_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/session"
    } else {
        "session"
    }
}

/// Event-stream path per server version (V2 path assumed, see header).
pub fn event_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/event"
    } else {
        "event"
    }
}

/// Custom-command list path per server version (JS `command.list` parity:
/// V1 SDK `command.list` also serves skills, filtered client-side by
/// `source`; V2 splits `command.list` / `skill.list`. V2 `api/` prefix
/// assumed, see header).
pub fn command_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/command"
    } else {
        "command"
    }
}

/// Skill list path per server version (JS V2 `skill.list` parity; V1
/// reuses the `command.list` endpoint with a `source == "skill"` filter.
/// V2 `api/` prefix assumed, see header).
pub fn skill_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/skill"
    } else {
        "command"
    }
}

/// MCP status path per server version (JS `mcp.status` parity; the V2
/// shim reads `mcp.list`. V2 `api/` prefix assumed, see header).
pub fn mcp_status_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/mcp"
    } else {
        "mcp"
    }
}

/// Provider catalog path per server version (JS `config.providers` parity:
/// V1 SDK `config.providers`, V2 fan-out over `model.list` /
/// `provider.list` in `src/opencode/v2/client.ts`. The scaffold reads a
/// single versioned route; V2 `api/` prefix assumed, see header).
pub fn providers_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/config/providers"
    } else {
        "config/providers"
    }
}

/// Agent list path per server version (JS `app.agents` parity: V1 SDK
/// `app.agents`, V2 `agent.list` in `src/opencode/v2/client.ts`. V2 `api/`
/// prefix assumed, see header).
pub fn agents_path(server_version: &str) -> &'static str {
    if server_version == "v2" {
        "api/agent"
    } else {
        "app/agents"
    }
}

/// MCP connect path per server version (JS `mcp.connect` parity:
/// `POST <mcp>/connect` with a `{"name":...}` body; the exact V1 route is
/// an assumption, see header item 2).
pub fn mcp_connect_path(server_version: &str) -> String {
    format!("{}/connect", mcp_status_path(server_version))
}

/// MCP disconnect path per server version (JS `mcp.disconnect` parity:
/// `POST <mcp>/disconnect` with a `{"name":...}` body; same assumption as
/// [`mcp_connect_path`]).
pub fn mcp_disconnect_path(server_version: &str) -> String {
    format!("{}/disconnect", mcp_status_path(server_version))
}

/// Session abort path per server version (Node SDK parity: V1
/// `session.abort` is `POST /session/{id}/abort`; V2 `session.interrupt`
/// is `POST /api/session/{id}/interrupt`). Pure; testable.
pub fn abort_path(server_version: &str, session_id: &str) -> String {
    if server_version == "v2" {
        format!("{}/{session_id}/interrupt", session_path(server_version))
    } else {
        format!("{}/{session_id}/abort", session_path(server_version))
    }
}

/// Revert path per server version (Node `session.revert` parity: V1
/// `POST session/{id}/revert`, V2 `POST api/session/{id}/revert`; the V2
/// `api/` prefix is assumed like the probe route, see header).
/// Pure; testable.
pub fn revert_path(server_version: &str, session_id: &str) -> String {
    format!("{}/{session_id}/revert", session_path(server_version))
}

/// Fork path per server version (Node `session.fork` parity: V1
/// `POST session/{id}/fork`, V2 `POST api/session/{id}/fork`; the V2
/// `api/` prefix is assumed like the probe route, see header).
/// Pure; testable.
pub fn fork_path(server_version: &str, session_id: &str) -> String {
    format!("{}/{session_id}/fork", session_path(server_version))
}

/// Prompt path per server version: `<session_path>/<session_id>`
/// (V1 `session/{id}`, V2 `api/session/{id}`; V2 prefix assumed, see header).
pub fn prompt_path(server_version: &str, session_id: &str) -> String {
    format!("{}/{}", session_path(server_version), session_id)
}

/// Session message-list path per server version:
/// `<session_path>/<session_id>/message` (V1 SDK `session.messages`
/// parity; V2 prefix assumed, see header).
pub fn message_path(server_version: &str, session_id: &str) -> String {
    format!("{}/{}/message", session_path(server_version), session_id)
}

/// Prompt body per server version (pure; testable).
/// V1 (SDK parity): `{"parts":[{"type":"text","text":...}]}`;
/// V2 (Node `admitPrompt` parity): `{"text":...}`.
pub fn prompt_payload(server_version: &str, text: &str) -> Value {
    if server_version == "v2" {
        serde_json::json!({ "text": text })
    } else {
        serde_json::json!({ "parts": [{ "type": "text", "text": text }] })
    }
}

/// Join a base URL and a relative path (pure; avoids double slashes).
pub fn join_url(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

/// Basic-auth header value for a username/password pair (Node: `getAuth`).
/// Empty password still sends `username:` credentials, like the TS client.
pub fn basic_auth_header(username: &str, password: &str) -> String {
    // Base64 without pulling in a new crate: reqwest has no btoa helper, so
    // encode the `username:password` bytes by hand (standard alphabet).
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = format!("{username}:{password}");
    let bytes = input.as_bytes();
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    format!("Basic {out}")
}

/// Env var carrying the OpenCode server username (Node: `OPENCODE_SERVER_USERNAME`).
pub const OPENCODE_SERVER_USERNAME_ENV: &str = "OPENCODE_SERVER_USERNAME";

/// Env var carrying the OpenCode server password (Node: `OPENCODE_SERVER_PASSWORD`).
pub const OPENCODE_SERVER_PASSWORD_ENV: &str = "OPENCODE_SERVER_PASSWORD";

/// Server username from the environment (Node: `config.opencode.username`;
/// defaults to `opencode` when missing or empty).
pub fn server_username_from_env() -> String {
    std::env::var(OPENCODE_SERVER_USERNAME_ENV)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| otb_core::runtime_bootstrap_gates::DEFAULT_SERVER_USERNAME.to_string())
}

/// Server password from the environment (Node: `config.opencode.password`);
/// `None` when missing or empty, in which case no auth header is sent
/// (Node: `getAuth` returns `undefined`).
pub fn server_password_from_env() -> Option<String> {
    std::env::var(OPENCODE_SERVER_PASSWORD_ENV)
        .ok()
        .filter(|value| !value.is_empty())
}

/// Basic-auth header from the environment (Node: `getAuth`/`authHeaders`);
/// `None` when no password is configured. Only the header presence (never
/// credential values) may be logged by callers.
pub fn auth_header_from_env() -> Option<String> {
    server_password_from_env()
        .map(|password| basic_auth_header(&server_username_from_env(), &password))
}

/// True when a `content-type` header value carries JSON
/// (Node: `contentType.includes("json")` in `probeOpencodeServer`).
/// Anything else (notably `text/html`) is not a live server answer.
pub fn is_json_content_type(content_type: Option<&str>) -> bool {
    content_type.map_or(false, |value| value.contains("json"))
}
/// Unwrap a `{"data": ...}` envelope when present (pure; testable).
/// Live probe: `POST api/session` answers
/// `{"data":{"id":"ses_...","projectID":"...","location":{...}}}` while the
/// SDK shapes used elsewhere are bare (`{"id":...}`). Returns the inner
/// value when `data` is an object/array, else the body itself.
pub fn unwrap_data_envelope<'a>(body: &'a Value) -> &'a Value {
    match body.get("data") {
        Some(inner) if inner.is_object() || inner.is_array() => inner,
        _ => body,
    }
}

/// Session rows from a list body (pure; testable). Accepts a bare array
/// (`[...]`) or an enveloped one (`{"data":[...]}`); anything else yields
/// an empty list.
pub fn session_list_from_body(body: &Value) -> Vec<Value> {
    unwrap_data_envelope(body)
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct EventEnvelope {
    pub directory: String,
    pub event_type: String,
    pub payload: Value,
}

/// What a reconnect tells about the server (Node: `ReconnectInfo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectInfo {
    pub server_restarted: Option<bool>,
}

/// Health-probe answer (Node: `OpencodeServerProbe` + version read).
#[derive(Debug, Clone)]
pub struct HealthProbe {
    /// HTTP 2xx with a JSON body.
    pub ok: bool,
    /// Server version string when the body proves a live server
    /// (V1 additionally requires `healthy: true`, V2 a numeric `pid`).
    pub version: Option<String>,
}

fn read_probe_version(server_version: &str, body: &Value) -> Option<String> {
    let version = body.get("version")?.as_str()?;
    if server_version == "v2" {
        return body.get("pid")?.as_u64().map(|_| version.to_string());
    }
    if body.get("healthy")?.as_bool() == Some(true) {
        Some(version.to_string())
    } else {
        None
    }
}

/// Thin OpenCode HTTP client. Constructing it performs zero requests and
/// never starts or stops the server.
#[derive(Debug, Clone)]
pub struct OpencodeClient {
    base_url: String,
    server_version: String,
    authorization: Option<String>,
    http: reqwest::Client,
}

impl OpencodeClient {
    /// Build from explicit values (pure constructor; no I/O).
    pub fn new(base_url: &str, server_version: &str) -> Self {
        Self {
            base_url: resolve_base_url(Some(base_url)),
            server_version: server_version.to_string(),
            authorization: None,
            http: reqwest::Client::new(),
        }
    }

    /// Build from the environment (`OPENCODE_URL` / `OPENCODE_API_URL`,
    /// `OPENCODE_SERVER_VERSION`, plus `OPENCODE_SERVER_USERNAME` /
    /// `OPENCODE_SERVER_PASSWORD` for Basic auth). Reads env only;
    /// performs no I/O.
    pub fn from_env() -> Self {
        Self {
            base_url: base_url_from_env(),
            server_version: server_version_from_env().to_string(),
            authorization: auth_header_from_env(),
            http: reqwest::Client::new(),
        }
    }

    /// Attach Basic credentials (Node: per-server `getAuth`).
    pub fn with_basic_auth(mut self, username: &str, password: &str) -> Self {
        self.authorization = Some(basic_auth_header(username, password));
        self
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        let request = self.http.get(join_url(&self.base_url, path));
        match &self.authorization {
            Some(value) => request.header(reqwest::header::AUTHORIZATION, value.clone()),
            None => request,
        }
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let request = self.http.post(join_url(&self.base_url, path));
        match &self.authorization {
            Some(value) => request.header(reqwest::header::AUTHORIZATION, value.clone()),
            None => request,
        }
    }

    /// Health probe (read-only). V1 requires `healthy: true`, V2 a numeric
    /// `pid` before a version is reported (Node: `readServerVersion`).
    /// Mirrors `probeOpencodeServer`: 401 (missing/wrong credentials),
    /// any other non-2xx, a non-JSON body (e.g. an HTML page), or a JSON
    /// body without the expected shape all mean "no live server there"
    /// (`ok: false`). Only the status code is observable; credential
    /// values are never logged.
    pub async fn health(&self) -> Result<HealthProbe, reqwest::Error> {
        let response = self
            .get(health_path(&self.server_version))
            .timeout(PROBE_TIMEOUT)
            .send()
            .await?;
        if !response.status().is_success() {
            return Ok(HealthProbe {
                ok: false,
                version: None,
            });
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        if !is_json_content_type(content_type.as_deref()) {
            return Ok(HealthProbe {
                ok: false,
                version: None,
            });
        }
        let body: Value = match response.json().await {
            Ok(body) => body,
            Err(_) => {
                return Ok(HealthProbe {
                    ok: false,
                    version: None,
                });
            }
        };
        let version = read_probe_version(&self.server_version, &body);
        Ok(HealthProbe { ok: true, version })
    }

    /// Session list (read-only). Returns raw session values; shaping stays
    /// with the caller. Accepts a bare array or a `{"data":[...]}` envelope
    /// (live V2 probe shape); anything else yields an empty list.
    pub async fn list_sessions(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<Value>, reqwest::Error> {
        let mut request = self.get(session_path(&self.server_version));
        if let Some(directory) = directory.filter(|dir| !dir.is_empty()) {
            request = request.query(&[("directory", directory)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(session_list_from_body(&body))
    }

    /// Project list (read-only). Returns raw project values (`id`,
    /// `worktree`, `name`, `time.updated`); shaping stays with the caller.
    /// Accepts a bare array or a `{"data":[...]}` envelope like the session
    /// list; anything else yields an empty list.
    pub async fn list_projects(&self) -> Result<Vec<Value>, reqwest::Error> {
        let response = self
            .get(project_path(&self.server_version))
            .send()
            .await?
            .error_for_status()?;
        let body: Value = response.json().await?;
        Ok(session_list_from_body(&body))
    }

    /// One session (read-only). Used for the `revert.messageID` cutoff in the
    /// `/messages` view (JS `session.get` parity in
    /// `message-history-service.ts`). Unwraps a `{"data":{...}}` envelope
    /// when present.
    pub async fn get_session(
        &self,
        session_id: &str,
        directory: &str,
    ) -> Result<Value, reqwest::Error> {
        let path = format!("{}/{}", session_path(&self.server_version), session_id);
        let mut request = self.get(&path);
        if !directory.is_empty() {
            request = request.query(&[("directory", directory)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(unwrap_data_envelope(&body).clone())
    }

    /// Session messages (read-only). Returns raw message values
    /// (`info`/`parts` shapes); shaping stays with the caller. Accepts a
    /// bare array or a `{"data":[...]}` envelope; anything else yields an
    /// empty list.
    pub async fn session_messages(
        &self,
        session_id: &str,
        directory: &str,
    ) -> Result<Vec<Value>, reqwest::Error> {
        let mut request = self.get(&message_path(&self.server_version, session_id));
        if !directory.is_empty() {
            request = request.query(&[("directory", directory)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(session_list_from_body(&body))
    }

    /// Custom-command list (read-only). Returns raw command values (`name`,
    /// `description`, `source`); the `source == "command"` + non-`"goal"`
    /// shaping stays with the caller. Accepts a bare array or a
    /// `{"data":[...]}` envelope; anything else yields an empty list.
    /// The directory is folded for the API (`\` -> `/`, Node
    /// `normalizeDirectoryForCommandApi` parity) and sent as `?directory=`.
    pub async fn list_commands(&self, directory: &str) -> Result<Vec<Value>, reqwest::Error> {
        let normalized =
            otb_core::session_pick_catalog_gates::normalize_catalog_api_directory(directory);
        let mut request = self.get(command_path(&self.server_version));
        if !normalized.is_empty() {
            request = request.query(&[("directory", normalized)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(session_list_from_body(&body))
    }

    /// Skill list (read-only). Same shape/contract as [`list_commands`];
    /// the `source == "skill"` shaping stays with the caller. V1 shares
    /// the `command.list` endpoint (the filter splits them); V2 reads its
    /// own `skill.list` path.
    pub async fn list_skills(&self, directory: &str) -> Result<Vec<Value>, reqwest::Error> {
        let normalized =
            otb_core::session_pick_catalog_gates::normalize_catalog_api_directory(directory);
        let mut request = self.get(skill_path(&self.server_version));
        if !normalized.is_empty() {
            request = request.query(&[("directory", normalized)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(session_list_from_body(&body))
    }

    /// MCP server statuses (read-only). Returns the raw status body (record
    /// or array form); parsing (`parseMcpCatalogServers` parity) stays with
    /// the caller. `{"data":{...}}` envelopes are unwrapped.
    pub async fn mcp_status(&self, directory: &str) -> Result<Value, reqwest::Error> {
        let normalized =
            otb_core::session_pick_catalog_gates::normalize_catalog_api_directory(directory);
        let mut request = self.get(mcp_status_path(&self.server_version));
        if !normalized.is_empty() {
            request = request.query(&[("directory", normalized)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(unwrap_data_envelope(&body).clone())
    }

    /// Enable or disable one MCP server (JS `toggleMcpCatalogServer`
    /// parity in `mcp-catalog-service.ts`: `mcp.connect` /
    /// `mcp.disconnect` with `{ name, directory }`). `?directory=` is sent
    /// when non-empty; the body carries the folded directory plus the server
    /// name. Any 2xx counts as accepted (`true`).
    pub async fn mcp_set_enabled(
        &self,
        directory: &str,
        name: &str,
        enable: bool,
    ) -> Result<bool, reqwest::Error> {
        let normalized =
            otb_core::session_pick_catalog_gates::normalize_catalog_api_directory(directory);
        let path = if enable {
            mcp_connect_path(&self.server_version)
        } else {
            mcp_disconnect_path(&self.server_version)
        };
        let mut request = self.post(&path);
        if !normalized.is_empty() {
            request = request.query(&[("directory", normalized.clone())]);
        }
        let response = request
            .json(&serde_json::json!({ "name": name, "directory": normalized }))
            .send()
            .await?
            .error_for_status()?;
        Ok(response.status().is_success())
    }

    /// Provider catalog (read-only). Returns the raw body
    /// (`{"providers":[...]}` V1 shape or a bare array); shaping
    /// (provider rows, model entries, variants) stays with the caller.
    /// `{"data":{...}}` envelopes are unwrapped.
    pub async fn list_providers(&self) -> Result<Value, reqwest::Error> {
        let response = self
            .get(providers_path(&self.server_version))
            .send()
            .await?
            .error_for_status()?;
        let body: Value = response.json().await?;
        Ok(unwrap_data_envelope(&body).clone())
    }

    /// Agent list (read-only). Returns raw agent values (`name`, `mode`,
    /// `hidden`, ...); filtering (`hidden`/`mode`) stays with the caller.
    /// Accepts a bare array or a `{"data":[...]}` envelope.
    pub async fn list_agents(&self, directory: &str) -> Result<Vec<Value>, reqwest::Error> {
        let normalized =
            otb_core::session_pick_catalog_gates::normalize_catalog_api_directory(directory);
        let mut request = self.get(agents_path(&self.server_version));
        if !normalized.is_empty() {
            request = request.query(&[("directory", normalized)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(session_list_from_body(&body))
    }

    /// Rename a session (JS `session.update` parity in
    /// `rename-callback-handler.ts`: `PATCH <session_path>/{id}` with
    /// `?directory=` and a `{"title":...}` body). Returns the raw updated
    /// session; `{"data":{...}}` envelopes are unwrapped.
    pub async fn update_session_title(
        &self,
        session_id: &str,
        directory: &str,
        title: &str,
    ) -> Result<Value, reqwest::Error> {
        let path = format!("{}/{}", session_path(&self.server_version), session_id);
        let mut request = self
            .http
            .patch(join_url(&self.base_url, &path))
            .json(&serde_json::json!({ "title": title }));
        match &self.authorization {
            Some(value) => {
                request = request.header(reqwest::header::AUTHORIZATION, value.clone());
            }
            None => {}
        }
        if !directory.is_empty() {
            request = request.query(&[("directory", directory)]);
        }
        let response = request.send().await?.error_for_status()?;
        let body: Value = response.json().await?;
        Ok(unwrap_data_envelope(&body).clone())
    }

    /// Session create (parity only; the scaffold never calls it).
    /// Unwraps a `{"data":{...}}` envelope when present so callers can
    /// read top-level `id`/`title` either way. Request shape (flat
    /// `{"directory":...}` + optional `title`) and auth/paths are unchanged.
    pub async fn create_session(
        &self,
        directory: &str,
        title: Option<&str>,
    ) -> Result<Value, reqwest::Error> {
        let mut payload = serde_json::json!({ "directory": directory });
        if let Some(title) = title {
            payload["title"] = Value::String(title.to_string());
        }
        let response = self
            .post(session_path(&self.server_version))
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;
        let body: Value = response.json().await?;
        Ok(unwrap_data_envelope(&body).clone())
    }

    /// Session prompt (parity only; the scaffold never calls it, keeping the
    /// default path read-only).
    pub async fn send_prompt(&self, session_id: &str, text: &str) -> Result<Value, reqwest::Error> {
        let path = format!("{}/{}", session_path(&self.server_version), session_id);
        let payload = serde_json::json!({ "text": text });
        let response = self
            .post(&path)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;
        response.json().await
    }

    /// Send a text prompt into a session (T13 write path; nothing calls it
    /// unless polling dispatches a command that needs it). Reuses the
    /// versioned base URL + auth header helpers via `post`.
    pub async fn prompt_session(
        &self,
        session_id: &str,
        text: &str,
    ) -> Result<Value, reqwest::Error> {
        let path = prompt_path(&self.server_version, session_id);
        let payload = prompt_payload(&self.server_version, text);
        let response = self
            .post(&path)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;
        response.json().await
    }

    /// Abort a running session turn (JS `session.abort` parity in
    /// `abort-command.ts`: V1 `abort`, V2 `interrupt`). `?directory=` is
    /// sent when non-empty. Any 2xx counts as accepted (`true`); anything
    /// else is a request error. Safe by construction: it only signals the
    /// server, never starts or stops it.
    pub async fn abort_session(
        &self,
        session_id: &str,
        directory: &str,
    ) -> Result<bool, reqwest::Error> {
        let mut request = self.post(&abort_path(&self.server_version, session_id));
        if !directory.is_empty() {
            request = request.query(&[("directory", directory)]);
        }
        let response = request.send().await?.error_for_status()?;
        Ok(response.status().is_success())
    }

    /// Revert a session to a user message (JS `session.revert` parity in
    /// `message-history-callback-handler.ts`: V1 `session.revert` is
    /// `POST session/{id}/revert`; V2 `session.revert.stage` is covered by
    /// the same versioned path with a `{"messageID":...}` body — the exact
    /// V2 field name is an assumption, see header item 2). `?directory=` is
    /// sent when non-empty. Any 2xx counts as accepted (`true`); anything
    /// else is a request error. A rejected call means the caller renders
    /// the local `messages.revert_error` notice.
    pub async fn revert_session(
        &self,
        session_id: &str,
        directory: &str,
        message_id: &str,
    ) -> Result<bool, reqwest::Error> {
        let mut request = self.post(&revert_path(&self.server_version, session_id));
        if !directory.is_empty() {
            request = request.query(&[("directory", directory)]);
        }
        let response = request
            .json(&serde_json::json!({ "messageID": message_id }))
            .send()
            .await?
            .error_for_status()?;
        Ok(response.status().is_success())
    }

    /// Fork a session from a user message (JS `session.fork` parity in
    /// `message-history-callback-handler.ts`: V1 `session.fork` is
    /// `POST session/{id}/fork`; V2 `session.fork({sessionID, before})` is
    /// covered by the same versioned path with a `{"messageID":...}` body —
    /// the exact V2 field name is an assumption, see header item 2).
    /// `?directory=` is sent when non-empty. Returns the raw forked
    /// session; `{"data":{...}}` envelopes are unwrapped.
    pub async fn fork_session(
        &self,
        session_id: &str,
        directory: &str,
        message_id: &str,
    ) -> Result<Value, reqwest::Error> {
        let mut request = self.post(&fork_path(&self.server_version, session_id));
        if !directory.is_empty() {
            request = request.query(&[("directory", directory)]);
        }
        let response = request
            .json(&serde_json::json!({ "messageID": message_id }))
            .send()
            .await?
            .error_for_status()?;
        let body: Value = response.json().await?;
        Ok(unwrap_data_envelope(&body).clone())
    }

    /// Subscribe to the SSE event stream with reconnect backoff (read-only).
    ///
    /// Mirrors `subscribeToEvents`: drops (errors, idle timeouts, clean
    /// ends) log a `warn` with the next delay, then reconnect; a `false`
    /// return from `on_event`... (callbacks are infallible; stop by
    /// aborting the caller's task). Runs until the future is dropped.
    pub async fn subscribe_events<F, R>(&self, directory: &str, on_event: F, on_reconnect: R)
    where
        F: Fn(EventEnvelope),
        R: Fn(ReconnectInfo),
    {
        let mut attempt: i64 = 0;
        let mut reconnect_pending = false;
        loop {
            match self
                .read_stream_once(directory, reconnect_pending, &on_event, &on_reconnect)
                .await
            {
                StreamOutcome::Abort => break,
                StreamOutcome::Dropped => {
                    reconnect_pending = true;
                    attempt += 1;
                    let delay = otb_core::event_transport_gates::get_reconnect_delay_ms(attempt);
                    tracing::warn!(
                        "[Opencode] Event stream ended for {directory}, reconnecting in {delay}ms (attempt={attempt})"
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    /// One subscribe attempt: returns when the stream drops. `Aborted` is
    /// reserved for a future stop signal; today every end is a `Dropped`
    /// that the caller backs off and retries (Node: same reconnect loop).
    async fn read_stream_once<F, R>(
        &self,
        directory: &str,
        reconnect_pending: bool,
        on_event: &F,
        on_reconnect: &R,
    ) -> StreamOutcome
    where
        F: Fn(EventEnvelope),
        R: Fn(ReconnectInfo),
    {
        let mut request = self
            .get(event_path(&self.server_version))
            .header(reqwest::header::ACCEPT, "text/event-stream");
        if self.server_version != "v2" {
            request = request.query(&[("directory", directory)]);
        }
        let response = match request.send().await {
            Ok(response) => match response.error_for_status() {
                Ok(response) => response,
                Err(err) => {
                    tracing::warn!("[Opencode] Event subscribe rejected for {directory}: {err}");
                    return StreamOutcome::Dropped;
                }
            },
            Err(err) => {
                tracing::warn!("[Opencode] Event subscribe failed for {directory}: {err}");
                return StreamOutcome::Dropped;
            }
        };

        let mut response = response;
        let mut buffer = String::new();
        let mut connected_seen = false;
        let mut reconnect_armed = reconnect_pending;
        loop {
            let chunk = match tokio::time::timeout(SSE_IDLE_TIMEOUT, response.chunk()).await {
                Ok(Ok(Some(chunk))) => chunk,
                Ok(Ok(None)) => return StreamOutcome::Dropped,
                Ok(Err(err)) => {
                    tracing::warn!("[Opencode] Event stream error for {directory}: {err}");
                    return StreamOutcome::Dropped;
                }
                Err(_) => {
                    tracing::warn!(
                        "[Opencode] {}",
                        otb_core::event_transport_gates::SSE_IDLE_TIMEOUT_ERROR
                    );
                    return StreamOutcome::Dropped;
                }
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            let (frames, rest) = split_sse_frames(&buffer);
            buffer = rest;
            for frame in frames {
                let Some(envelope) = normalize_frame(directory, &frame) else {
                    continue;
                };
                if envelope.event_type == "server.connected" {
                    if connected_seen || reconnect_armed {
                        reconnect_armed = false;
                        let restarted = envelope
                            .payload
                            .get("properties")
                            .and_then(|props| props.get("restarted"))
                            .and_then(Value::as_bool);
                        let info = ReconnectInfo {
                            server_restarted:
                                otb_core::event_transport_gates::resolve_reconnect_restarted(
                                    true, restarted,
                                ),
                        };
                        tracing::info!("[Opencode] Event stream reconnected for {directory}");
                        on_reconnect(info);
                    } else {
                        connected_seen = true;
                    }
                    continue;
                }
                on_event(envelope);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamOutcome {
    Abort,
    Dropped,
}

/// Split accumulated SSE text into complete `data:` frames.
/// Returns the frame payloads plus the unterminated remainder.
/// A frame ends at a blank line; `data:` lines join with `\n`
/// (comment/`event:`/`id:` lines are ignored).
pub fn split_sse_frames(buffer: &str) -> (Vec<String>, String) {
    let mut frames = Vec::new();
    let mut data: Vec<&str> = Vec::new();
    let mut consumed_through = 0usize;
    let mut line_start = 0usize;
    let mut pos = 0usize;
    while pos <= buffer.len() {
        let is_end = pos == buffer.len();
        let is_newline = !is_end && buffer.as_bytes()[pos] == b'\n';
        if is_end || is_newline {
            let boundary = pos + usize::from(!is_end);
            let mut line = &buffer[line_start..pos];
            line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() {
                if !data.is_empty() {
                    frames.push(data.join("\n"));
                    data.clear();
                }
                consumed_through = boundary;
            } else if let Some(payload) = line
                .strip_prefix("data:")
                .map(|rest| rest.strip_prefix(' ').unwrap_or(rest))
            {
                data.push(payload);
            }
            line_start = boundary;
            if is_end {
                break;
            }
        }
        pos += 1;
    }
    (frames, buffer[consumed_through..].to_string())
}

/// Normalize one SSE `data:` payload into an envelope (pure; testable).
/// Non-JSON or event-shaped misses return `None`; events from another
/// directory are dropped (Node: `normalizeGlobalEvent` directory gate).
fn normalize_frame(directory: &str, frame: &str) -> Option<EventEnvelope> {
    let payload: Value = serde_json::from_str(frame).ok()?;
    let event_type = payload.get("type")?.as_str()?.to_string();
    let event_directory = payload.get("directory").and_then(Value::as_str);
    if let Some(event_dir) = event_directory {
        if otb_core::event_transport_gates::should_drop_global_event_on_directory(
            Some(event_dir),
            otb_core::event_transport_gates::is_same_directory(event_dir, directory),
        ) {
            return None;
        }
    }
    let resolved =
        otb_core::event_transport_gates::resolve_global_event_directory(event_directory, directory);
    Some(EventEnvelope {
        directory: resolved,
        event_type,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_prefers_explicit_and_trims_slash() {
        assert_eq!(
            resolve_base_url(Some("http://127.0.0.1:4096/")),
            "http://127.0.0.1:4096"
        );
        assert_eq!(resolve_base_url(Some("")), DEFAULT_OPENCODE_URL);
        assert_eq!(resolve_base_url(None), DEFAULT_OPENCODE_URL);
    }

    #[test]
    fn versioned_paths_mirror_probe_convention() {
        assert_eq!(health_path("v1"), "global/health");
        assert_eq!(health_path("v2"), "api/info");
        assert_eq!(session_path("v1"), "session");
        assert_eq!(session_path("v2"), "api/session");
        assert_eq!(project_path("v1"), "project");
        assert_eq!(project_path("v2"), "api/project");
        assert_eq!(command_path("v1"), "command");
        assert_eq!(command_path("v2"), "api/command");
        assert_eq!(skill_path("v1"), "command");
        assert_eq!(skill_path("v2"), "api/skill");
        assert_eq!(mcp_status_path("v1"), "mcp");
        assert_eq!(mcp_status_path("v2"), "api/mcp");
        assert_eq!(providers_path("v1"), "config/providers");
        assert_eq!(providers_path("v2"), "api/config/providers");
        assert_eq!(agents_path("v1"), "app/agents");
        assert_eq!(agents_path("v2"), "api/agent");
        assert_eq!(mcp_connect_path("v1"), "mcp/connect");
        assert_eq!(mcp_disconnect_path("v2"), "api/mcp/disconnect");
        assert_eq!(message_path("v1", "ses_1"), "session/ses_1/message");
        assert_eq!(message_path("v2", "ses_1"), "api/session/ses_1/message");
        assert_eq!(event_path("v1"), "event");
        assert_eq!(event_path("v2"), "api/event");
        assert_eq!(
            join_url("http://127.0.0.1:4096/", "event"),
            "http://127.0.0.1:4096/event"
        );
    }

    #[test]
    fn basic_auth_header_matches_node_get_auth() {
        // Node: `Basic base64("username:password")`.
        assert_eq!(
            basic_auth_header("aladdin", "open sesame"),
            "Basic YWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
        assert_eq!(basic_auth_header("user", ""), "Basic dXNlcjo=");
    }

    #[test]
    fn auth_header_from_env_matches_node_get_auth() {
        // Node `getAuth`: no header without a password; otherwise
        // `Basic base64("username:password")` with username defaulting to
        // `opencode`. Dummy values only; live env is restored afterwards.
        let prev_user = std::env::var(OPENCODE_SERVER_USERNAME_ENV).ok();
        let prev_pass = std::env::var(OPENCODE_SERVER_PASSWORD_ENV).ok();

        std::env::remove_var(OPENCODE_SERVER_USERNAME_ENV);
        std::env::set_var(OPENCODE_SERVER_PASSWORD_ENV, "dummy-password");
        assert_eq!(server_username_from_env(), "opencode");
        assert_eq!(
            auth_header_from_env(),
            Some(basic_auth_header("opencode", "dummy-password"))
        );

        std::env::set_var(OPENCODE_SERVER_USERNAME_ENV, "dummy-user");
        assert_eq!(
            auth_header_from_env(),
            Some(basic_auth_header("dummy-user", "dummy-password"))
        );

        std::env::remove_var(OPENCODE_SERVER_PASSWORD_ENV);
        assert_eq!(server_password_from_env(), None);
        assert_eq!(auth_header_from_env(), None);

        match prev_user {
            Some(value) => std::env::set_var(OPENCODE_SERVER_USERNAME_ENV, value),
            None => std::env::remove_var(OPENCODE_SERVER_USERNAME_ENV),
        }
        match prev_pass {
            Some(value) => std::env::set_var(OPENCODE_SERVER_PASSWORD_ENV, value),
            None => std::env::remove_var(OPENCODE_SERVER_PASSWORD_ENV),
        }
    }

    #[test]
    fn non_json_content_type_is_not_a_live_server_answer() {
        // Node `probeOpencodeServer` requires `content-type` to include
        // `json`: a 200 HTML page (e.g. V1 `global/health` on a service-mode
        // server) is `none`, i.e. unavailable — never healthy.
        assert!(is_json_content_type(Some("application/json")));
        assert!(is_json_content_type(Some(
            "application/json; charset=utf-8"
        )));
        assert!(!is_json_content_type(Some("text/html; charset=utf-8")));
        assert!(!is_json_content_type(Some("text/plain")));
        assert!(!is_json_content_type(None));
    }

    #[test]
    fn prompt_url_and_payload_follow_version_prefix() {
        assert_eq!(prompt_path("v1", "abc"), "session/abc");
        assert_eq!(prompt_path("v2", "abc"), "api/session/abc");
        assert_eq!(
            prompt_payload("v1", "hi"),
            serde_json::json!({ "parts": [{ "type": "text", "text": "hi" }] })
        );
        assert_eq!(
            prompt_payload("v2", "hi"),
            serde_json::json!({ "text": "hi" })
        );
    }

    #[test]
    fn abort_path_follows_sdk_method_names_per_version() {
        // V1 `session.abort` is `POST /session/{id}/abort`; V2
        // `session.interrupt` is `POST /api/session/{id}/interrupt`
        // (generated SDK method paths).
        assert_eq!(abort_path("v1", "ses_1"), "session/ses_1/abort");
        assert_eq!(abort_path("v2", "ses_1"), "api/session/ses_1/interrupt");
    }

    #[test]
    fn revert_and_fork_paths_follow_session_prefix_per_version() {
        // `session.revert` / `session.fork` parity: the versioned session
        // prefix plus the verb (V2 `api/` prefix assumed, like the probe).
        assert_eq!(revert_path("v1", "ses_1"), "session/ses_1/revert");
        assert_eq!(revert_path("v2", "ses_1"), "api/session/ses_1/revert");
        assert_eq!(fork_path("v1", "ses_1"), "session/ses_1/fork");
        assert_eq!(fork_path("v2", "ses_1"), "api/session/ses_1/fork");
    }

    #[test]
    fn probe_version_gates_match_node_read_server_version() {
        let healthy = serde_json::json!({ "version": "1.2.3", "healthy": true });
        assert_eq!(read_probe_version("v1", &healthy).as_deref(), Some("1.2.3"));
        let sick = serde_json::json!({ "version": "1.2.3", "healthy": false });
        assert_eq!(read_probe_version("v1", &sick), None);
        let v2 = serde_json::json!({ "version": "2.0.0", "pid": 8971 });
        assert_eq!(read_probe_version("v2", &v2).as_deref(), Some("2.0.0"));
        let v2_no_pid = serde_json::json!({ "version": "2.0.0" });
        assert_eq!(read_probe_version("v2", &v2_no_pid), None);
    }

    #[test]
    fn sse_frames_split_on_blank_lines_and_keep_tail() {
        let (frames, rest) = split_sse_frames("data: {\"a\":1}\n\ndata: partial");
        assert_eq!(frames, vec!["{\"a\":1}"]);
        assert_eq!(rest, "data: partial");
        // Multi-line data joins; non-`data:` lines are ignored per SSE.
        let (frames, rest) = split_sse_frames("data: one\ndata: two\n:comment\n\n");
        assert_eq!(frames, vec!["one\ntwo"]);
        assert_eq!(rest, "");
    }

    #[test]
    fn data_envelope_unwrap_supports_bare_and_enveloped_sessions() {
        // Live probe shape: `POST api/session` answers `{"data":{...}}`.
        let enveloped = serde_json::json!({
            "data": { "id": "ses_abc", "title": "hello", "projectID": "pro_1" }
        });
        let inner = unwrap_data_envelope(&enveloped);
        assert_eq!(inner.get("id").and_then(Value::as_str), Some("ses_abc"));
        assert_eq!(inner.get("title").and_then(Value::as_str), Some("hello"));

        // Bare shape (SDK-style): returned as-is.
        let bare = serde_json::json!({ "id": "ses_xyz", "title": "hi" });
        let inner = unwrap_data_envelope(&bare);
        assert_eq!(inner.get("id").and_then(Value::as_str), Some("ses_xyz"));

        // Non-object `data` is not an envelope.
        let odd = serde_json::json!({ "data": "nope", "id": "ses_1" });
        assert_eq!(
            unwrap_data_envelope(&odd).get("id").and_then(Value::as_str),
            Some("ses_1")
        );
    }

    #[test]
    fn session_list_from_body_supports_bare_and_enveloped_arrays() {
        let bare = serde_json::json!([{ "id": "ses_1" }, { "id": "ses_2" }]);
        let rows = session_list_from_body(&bare);
        assert_eq!(rows.len(), 2);

        let enveloped = serde_json::json!({ "data": [{ "id": "ses_1" }] });
        let rows = session_list_from_body(&enveloped);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("id").and_then(Value::as_str), Some("ses_1"));

        let junk = serde_json::json!({ "data": { "id": "ses_1" } });
        assert!(session_list_from_body(&junk).is_empty());
        assert!(session_list_from_body(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn normalize_frame_drops_foreign_directories() {
        let frame = r#"{"type":"session.idle","directory":"/other","properties":{}}"#;
        assert!(normalize_frame("/work", frame).is_none());
        let frame = r#"{"type":"session.idle","directory":"/work","properties":{}}"#;
        let envelope = normalize_frame("/work", frame).expect("kept");
        assert_eq!(envelope.directory, "/work");
        assert_eq!(envelope.event_type, "session.idle");
        assert!(normalize_frame("/work", "not json").is_none());
        assert!(normalize_frame("/work", r#"{"no":"type"}"#).is_none());
    }
}
