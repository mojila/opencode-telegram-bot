//! Server-target / local-start URL gates (slice 50, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/opencode/server-target.ts`
//!   (`describeOpencodeServerUrl`: `protocol//host` + pathname unless
//!   `"/"`, query/hash dropped, invalid URL falls back to
//!   `"the configured OPENCODE_API_URL"`; `switchOpencodeServer` gate:
//!   `server && server.url !== configuredUrl ? server : null`;
//!   `getActiveOpencodeServer` switched ?? configured;
//!   `isOnConfiguredOpencodeServer` switched-is-null check)
//! - `src/opencode/server-health.ts` (`reportFailedHealthCause` key/message
//!   builders: `"unauthorized"` key + auth-failed line, `` `mismatch:${actual}` ``
//!   key + version-mismatch line; unknown causes log nothing. The
//!   `reportOpencodeProblem` once/always dedupe and the
//!   `classifyFailedHealthCheck` cause mapping already live in
//!   [`crate::callback_health_gates`])
//! - `src/opencode/local-start.ts` (`getUrlPort`: `Number.parseInt(new URL(url).port)`
//!   integer gate; the registered-server `` `registered:${url}` `` key +
//!   not-connecting message with the rejected-credentials / not-answering
//!   reason; the `` `executable:${version}` `` key + executable-mismatch
//!   message. Probing, switching, and starting servers need network/child
//!   processes and stay out)
//!
//! No network/fetch, no child processes, no `config` reads live here —
//! only the URL math and report-string building the flows above drive.
//!
//! DEVIATIONS from TS:
//! 1. URL parsing is a `scheme://authority` subset (same shape as
//!    [`crate::local_process_gates`]: scheme syntax-checked, userinfo
//!    stripped at the last `@`, hostname lowercased, scheme-default ports
//!    `http`/`ws` 80, `https`/`wss` 443, `ftp` 21 elided, ports validated
//!    as ASCII digits `<= 65535`). Garbage/empty/whitespace-host/non-numeric
//!    or out-of-range ports make `describe` fall back to the configured
//!    label and `get_url_port` return `None`, matching `new URL` throwing.
//!    No percent-encoding or Unicode normalization beyond lowercasing;
//!    enough for the parity fixtures.
//! 2. The `null` switched server is `None`: `active_server_url` takes the
//!    switched URL as `Option`, and `select_switched_server` compares raw
//!    URL strings exactly like the TS `!==` gate (no normalization).

// ---------------------------------------------------------------------------
// Server target (`src/opencode/server-target.ts`)
// ---------------------------------------------------------------------------

/// Label used when a server URL cannot be parsed (TS: the
/// `"the configured OPENCODE_API_URL"` fallback in
/// `describeOpencodeServerUrl`).
pub const CONFIGURED_SERVER_FALLBACK_LABEL: &str = "the configured OPENCODE_API_URL";

/// An OpenCode server address with the basic-auth credentials it expects,
/// if any (TS: `OpencodeServerTarget`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpencodeServerTarget {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Scheme-default ports WHATWG drops before TS reads `.host` / `.port`.
fn scheme_default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

struct ParsedServerUrl {
    scheme: String,
    host: String,
    is_ipv6: bool,
    /// Explicit non-default port; `None` when absent or scheme-default
    /// (matching WHATWG `.port === ""`).
    port: Option<u16>,
    /// `""` for `"/"` (TS drops it), else the raw path with query/hash cut.
    path: String,
}

/// Minimal WHATWG `new URL` subset: `scheme://authority` + path, query/hash
/// dropped. Returns `None` wherever `new URL` throws for the tested shapes.
fn parse_server_url(raw: &str) -> Option<ParsedServerUrl> {
    let sep = raw.find("://")?;
    let raw_scheme = &raw[..sep];
    if raw_scheme.is_empty() {
        return None;
    }
    let mut scheme_bytes = raw_scheme.bytes();
    if !scheme_bytes.next().is_some_and(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    if !raw_scheme
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
    {
        return None;
    }
    let scheme = raw_scheme.to_ascii_lowercase();
    let after = &raw[sep + 3..];
    let auth_end = after
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(after.len());
    let authority = &after[..auth_end];
    let rest = &after[auth_end..];
    if authority.is_empty() {
        return None;
    }
    if authority.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }
    let hostport = match authority.rfind('@') {
        Some(idx) => &authority[idx + 1..],
        None => authority,
    };
    if hostport.is_empty() {
        return None;
    }
    let (host_raw, is_ipv6, port_text): (&str, bool, Option<&str>) = if hostport.starts_with('[') {
        let close = hostport.find(']')?;
        let after_bracket = &hostport[close + 1..];
        let port = match after_bracket {
            "" => None,
            _ if after_bracket.starts_with(':') => {
                let text = &after_bracket[1..];
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
            _ => return None,
        };
        (&hostport[1..close], true, port)
    } else if let Some(idx) = hostport.rfind(':') {
        let host = &hostport[..idx];
        let text = &hostport[idx + 1..];
        if host.is_empty() {
            return None;
        }
        if text.is_empty() {
            (host, false, None)
        } else {
            if host.contains(':') {
                // Unbracketed IPv6 is rejected by WHATWG (`new URL` throws).
                return None;
            }
            (host, false, Some(text))
        }
    } else {
        (hostport, false, None)
    };
    if host_raw.is_empty() || host_raw.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }
    let port: Option<u16> = match port_text {
        None => None,
        Some(text) => {
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let value: u32 = text.parse().ok()?;
            if value > 65535 {
                return None;
            }
            let value = value as u16;
            match scheme_default_port(scheme.as_str()) {
                Some(default) if value == default => None,
                _ => Some(value),
            }
        }
    };
    let host = host_raw.to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    let path_end = rest.find(|c| c == '?' || c == '#').unwrap_or(rest.len());
    let pathname = &rest[..path_end];
    let path = if pathname.is_empty() || pathname == "/" {
        String::new()
    } else {
        pathname.to_string()
    };
    Some(ParsedServerUrl {
        scheme,
        host,
        is_ipv6,
        port,
        path,
    })
}

/// A server address for the log, without any credentials written into it
/// (TS: `describeOpencodeServerUrl`).
pub fn describe_opencode_server_url(server_url: &str) -> String {
    let parsed = match parse_server_url(server_url) {
        Some(parsed) => parsed,
        None => return CONFIGURED_SERVER_FALLBACK_LABEL.to_string(),
    };
    let host = if parsed.is_ipv6 {
        format!("[{}]", parsed.host)
    } else {
        parsed.host
    };
    let authority = match parsed.port {
        Some(port) => format!("{host}:{port}"),
        None => host,
    };
    format!("{}://{}{}", parsed.scheme, authority, parsed.path)
}

/// The server every OpenCode request goes to: the switched one, if any,
/// else the configured one (TS: `getActiveOpencodeServer`).
pub fn active_server_url<'a>(configured_url: &'a str, switched_url: Option<&'a str>) -> &'a str {
    switched_url.unwrap_or(configured_url)
}

/// Whether the bot talks to the configured server (TS:
/// `isOnConfiguredOpencodeServer`: `switchedServer === null`).
pub fn is_on_configured_server(switched_url: Option<&str>) -> bool {
    switched_url.is_none()
}

/// Points later requests at the given server, or back at the configured one
/// for `None` / same-as-configured (TS: the `switchOpencodeServer`
/// assignment gate).
pub fn select_switched_server(
    configured_url: &str,
    server: Option<OpencodeServerTarget>,
) -> Option<OpencodeServerTarget> {
    match server {
        Some(server) if server.url != configured_url => Some(server),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Failed-health-cause reports (`src/opencode/server-health.ts`)
// ---------------------------------------------------------------------------

/// Dedupe key for a wrong-credentials cause (TS: `"unauthorized"` in
/// `reportFailedHealthCause`).
pub fn unauthorized_health_report_key() -> &'static str {
    "unauthorized"
}

/// Log line for a wrong-credentials cause (TS: the `unauthorized` branch
/// of `reportFailedHealthCause`).
pub fn format_unauthorized_health_message(server_description: &str) -> String {
    format!(
        "[OpenCode] Authentication failed at {server_description}: \
         check OPENCODE_SERVER_USERNAME and OPENCODE_SERVER_PASSWORD"
    )
}

/// Dedupe key for a wrong-version cause (TS: `` `mismatch:${actual}` `` in
/// `reportFailedHealthCause`).
pub fn mismatch_health_report_key(actual: &str) -> String {
    format!("mismatch:{actual}")
}

/// Log line for a wrong-version cause (TS: the `mismatch` branch of
/// `reportFailedHealthCause`; `Unknown` causes log nothing).
pub fn format_mismatch_health_message(
    configured_version: &str,
    server_description: &str,
    actual: &str,
    other_version: &str,
) -> String {
    format!(
        "[OpenCode] Server version mismatch: \
         OPENCODE_SERVER_VERSION={configured_version}, but the server at \
         {server_description} is {actual} (API {other_version}). Set \
         OPENCODE_SERVER_VERSION={other_version} and restart the bot, or run \
         an OpenCode {configured_version} server at this address."
    )
}

// ---------------------------------------------------------------------------
// Local-start URL gates (`src/opencode/local-start.ts`)
// ---------------------------------------------------------------------------

/// Port of a URL, or `None` when absent, default, or invalid (TS:
/// `getUrlPort`: `Number.parseInt(new URL(url).port)` integer gate).
pub fn get_url_port(url: &str) -> Option<u16> {
    parse_server_url(url)?.port
}

/// Why the registered V2 background server cannot be adopted (TS: the
/// `problem` string in `findRegisteredServerOnAnotherPort`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisteredServerProblem {
    RejectedCredentials,
    NotAnswering,
}

/// Reason text for an unusable registered server (TS: `"rejected the
/// credentials from the OpenCode registry"` /
/// `"did not answer as an OpenCode v2 server"`).
pub fn registered_server_problem_text(problem: RegisteredServerProblem) -> &'static str {
    match problem {
        RegisteredServerProblem::RejectedCredentials => {
            "rejected the credentials from the OpenCode registry"
        }
        RegisteredServerProblem::NotAnswering => "did not answer as an OpenCode v2 server",
    }
}

/// Dedupe key for an unusable registered server (TS:
/// `` `registered:${registered.url}` ``).
pub fn registered_server_report_key(registered_url: &str) -> String {
    format!("registered:{registered_url}")
}

/// Log line for an unusable registered server (TS: the
/// `findRegisteredServerOnAnotherPort` error message).
pub fn format_registered_server_message(
    server_description: &str,
    problem: RegisteredServerProblem,
) -> String {
    let problem = registered_server_problem_text(problem);
    format!(
        "[OpenCode] Not connecting to the registered OpenCode v2 background \
         server at {server_description}: it {problem}. Not starting another \
         local server either, since that would replace it; stop that server \
         (opencode service stop) or fix it."
    )
}

/// Dedupe key for an executable-version refusal (TS:
/// `` `executable:${executableVersion}` `` in
/// `isBlockedByExecutableVersion`).
pub fn executable_mismatch_report_key(executable_version: &str) -> String {
    format!("executable:{executable_version}")
}

/// Log line for an executable-version refusal (TS: the
/// `isBlockedByExecutableVersion` error message).
pub fn format_executable_mismatch_message(
    configured_version: &str,
    executable_version: &str,
    executable_api_version: &str,
) -> String {
    format!(
        "[OpenCode] Server version mismatch: \
         OPENCODE_SERVER_VERSION={configured_version}, but the local opencode \
         executable is OpenCode {executable_version} (API \
         {executable_api_version}). Set \
         OPENCODE_SERVER_VERSION={executable_api_version} and restart the \
         bot, or make an OpenCode {configured_version} opencode executable \
         the first one on PATH."
    )
}

// ---------------------------------------------------------------------------
// Unavailable-server resolution guards (slice 174, GREEN).
//
// Rust port of the pure remainder in `src/opencode/local-start.ts` beyond
// slice 50: the registered-server lookup skips (non-V2 or standalone never
// adopts; a registered server on the target port is not a switch
// candidate) and the executable-version block (a missing version read
// passes; a matching API version passes; anything else refuses).
//
// Probing, switching, and starting servers need network/child processes
// and stay with the caller; outcomes arrive as explicit snapshots (the
// registered URL port via [`get_url_port`], the executable API version via
// `local_process_gates::get_opencode_api_version`).
//
// Deviations: none intended. `getUrlPort(...) === target.port` never
// matches a missing/default/unparseable port (`None`), mirroring
// `null === port` being false.
// ---------------------------------------------------------------------------

/// `true` when the registered-server lookup is skipped: only a V2
/// non-standalone setup adopts the background server (TS:
/// `findRegisteredServerOnAnotherPort` early `none`: `version !== "v2"`
/// or `serverMode === "standalone"`, which must not discover, adopt, or
/// refuse because of a registered server).
pub fn should_skip_registered_lookup(server_version: &str, server_mode: &str) -> bool {
    server_version != "v2" || server_mode == "standalone"
}

/// `true` when the registered server listens on the target port and is not
/// a switch candidate (TS: `getUrlPort(registered.url) === target.port`
/// yields `none`).
pub fn is_registered_on_target_port(registered_port: Option<u16>, target_port: u16) -> bool {
    registered_port == Some(target_port)
}

/// `true` when the local `opencode` executable is a release of the other
/// API version and the resolution refuses instead of spawning (TS:
/// `isBlockedByExecutableVersion`). The executable version arrives as its
/// API version (`"v1"` / `"v2"`); an unreadable executable passes.
pub fn is_blocked_by_executable_version(
    executable_api_version: Option<&str>,
    configured_version: &str,
) -> bool {
    match executable_api_version {
        Some(api) => api != configured_version,
        None => false,
    }
}
