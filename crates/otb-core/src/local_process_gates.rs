//! Local OpenCode process-target pure gates (slice 49, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/opencode/process.ts`
//!   (`resolveLocalOpencodeTarget`: URL parse, local-host allowlist,
//!   default/explicit port, integer `> 0` gate;
//!   `parseOpencodeVersionOutput`: first `\d+\.\d+\.\d+` match;
//!   `getOpencodeApiVersion`: `Number.parseInt` major `>= 2` is V2;
//!   `findWindowsListeningPidInNetstat` / `findUnixListeningPidInSs`:
//!   line/column scan, socket-port gate, pid `> 0` gate, first hit wins;
//!   `parseUnixPidList`: first valid `lsof -t` pid wins)
//!
//! No child processes, no filesystem, no `process.env` reads live here —
//! only the string/URL math the spawn/lookup flow drives.
//!
//! DEVIATIONS from TS:
//! 1. `resolveLocalOpencodeTarget` returns `Option<LocalOpencodeTarget>`;
//!    the WHATWG URL parse is a `scheme://authority` subset: userinfo is
//!    stripped, the hostname is lowercased, and a scheme-default port
//!    (`http`/`ws` 80, `https`/`wss` 443, `ftp` 21) normalizes to the 4096
//!    default — all matching `new URL(...).hostname` / `.port`. A port of
//!    `0`, a non-numeric port, or a port above 65535 is `None` (WHATWG
//!    throws on the latter shapes; the TS `<= 0` gate covers `0`).
//! 2. `getOpencodeApiVersion` returns `"v1"` / `"v2"` as `&'static str`,
//!    matching the `resolve_server_version` convention in
//!    [`crate::types_config`].
//! 3. `findUnixListeningPidInSs` port-gates on every whitespace-separated
//!    column instead of only `columns[3]`: `ss` layouts vary (a leading
//!    Netid token shifts the local address from index 3 to 4), and LISTEN
//!    rows only expose the target port in the local-address column, so
//!    both layouts resolve while header and `:*` peer columns still miss.
//!    A literal `columns[3]` port fails the parity fixtures; this keeps the
//!    TS "socket-port gate, pid `> 0` gate, first hit wins" contract.
//! 4. The private TS helpers `parsePid` / `parseSocketPort` are exercised
//!    through the public finders; only `parseUnixPidList` (pure over
//!    `lsof -t` output) is exposed directly.
//! 5. Line/whitespace splitting uses Rust `str` semantics (`split('\n')` +
//!    `trim` + `split_whitespace`), matching the TS `/\r?\n/` + `trim()` +
//!    `/\s+/` splits on every tested shape.

/// Default local OpenCode port (TS: `DEFAULT_OPENCODE_PORT`).
const DEFAULT_OPENCODE_PORT: u16 = 4096;

/// WHATWG URL ports are capped at 65535; larger values throw, hence `None`.
const MAX_URL_PORT: i64 = 65535;

/// Local server address the bot may spawn or probe (TS:
/// `LocalOpencodeTarget`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalOpencodeTarget {
    pub host: String,
    pub port: u16,
}

/// `Number.parseInt(value, 10)` semantics: skip leading whitespace, optional
/// sign, then a run of ASCII digits; `None` when there are no digits.
fn js_parse_int(raw: &str) -> Option<i64> {
    let trimmed = raw.trim_start();
    let (negative, rest) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits: Vec<u8> = rest.bytes().take_while(u8::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let mut value: i64 = 0;
    for digit in digits {
        value = value
            .saturating_mul(10)
            .saturating_add((digit - b'0') as i64);
    }
    Some(if negative {
        value.saturating_neg()
    } else {
        value
    })
}

/// TS `parsePid`: integer `> 0` that fits the return type, else `None`.
/// `Number.parseInt` takes a leading digit run, so `"1234,fd=3"` parses.
fn parse_pid(value: &str) -> Option<u32> {
    match js_parse_int(value) {
        Some(pid) if pid > 0 && pid <= u32::MAX as i64 => Some(pid as u32),
        _ => None,
    }
}

/// TS `parseSocketPort`: trailing `:digits` suffix, integer `> 0`.
fn parse_socket_port(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    let colon = trimmed.rfind(':')?;
    let digits = &trimmed[colon + 1..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    match js_parse_int(digits) {
        Some(port) if port > 0 => Some(port),
        _ => None,
    }
}

fn is_local_hostname(hostname: &str) -> bool {
    matches!(
        hostname,
        "localhost" | "127.0.0.1" | "::1" | "0.0.0.0"
    )
}

/// Scheme-default ports WHATWG drops before TS reads `.port` (TS then falls
/// back to 4096).
fn scheme_default_port(scheme: &str) -> Option<i64> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

/// Null when the URL is invalid, non-local, or has no usable port (TS:
/// `resolveLocalOpencodeTarget`).
pub fn resolve_local_opencode_target(api_url: &str) -> Option<LocalOpencodeTarget> {
    let sep = api_url.find("://")?;
    let raw_scheme = &api_url[..sep];
    if raw_scheme.is_empty()
        || !raw_scheme
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic())
        || !raw_scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
    {
        return None;
    }
    let scheme = raw_scheme.to_ascii_lowercase();
    let after = &api_url[sep + 3..];
    let auth_end = after
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(after.len());
    let authority = &after[..auth_end];
    let hostport = match authority.rfind('@') {
        Some(idx) => &authority[idx + 1..],
        None => authority,
    };
    if hostport.is_empty() {
        return None;
    }
    let (host_raw, port_text): (&str, Option<&str>) = if hostport.starts_with('[') {
        let close = hostport.find(']')?;
        let rest = &hostport[close + 1..];
        let port = match rest {
            "" => None,
            _ if rest.starts_with(':') => {
                let text = &rest[1..];
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
            _ => return None,
        };
        (&hostport[1..close], port)
    } else if let Some(idx) = hostport.rfind(':') {
        let host = &hostport[..idx];
        let text = &hostport[idx + 1..];
        if host.is_empty() {
            return None;
        }
        if text.is_empty() {
            (host, None)
        } else {
            if host.contains(':') {
                // Unbracketed IPv6 is rejected by WHATWG (`new URL` throws).
                return None;
            }
            (host, Some(text))
        }
    } else {
        (hostport, None)
    };
    if host_raw.is_empty() || host_raw.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }
    let host = host_raw.to_ascii_lowercase();
    if !is_local_hostname(&host) {
        return None;
    }
    let port = match port_text {
        None => DEFAULT_OPENCODE_PORT,
        Some(text) => {
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let parsed: i64 = text.parse().ok()?;
            if parsed <= 0 || parsed > MAX_URL_PORT {
                return None;
            }
            let parsed = parsed as u16;
            match scheme_default_port(scheme.as_str()) {
                Some(default) if parsed as i64 == default => DEFAULT_OPENCODE_PORT,
                _ => parsed,
            }
        }
    };
    Some(LocalOpencodeTarget { host, port })
}

/// First `X.Y.Z` triple in the stdout, else `None` (TS:
/// `parseOpencodeVersionOutput`).
pub fn parse_opencode_version_output(stdout: &str) -> Option<String> {
    let bytes = stdout.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if bytes.get(j) == Some(&b'.') {
                let mut k = j + 1;
                if bytes.get(k).is_some_and(u8::is_ascii_digit) {
                    while k < bytes.len() && bytes[k].is_ascii_digit() {
                        k += 1;
                    }
                    if bytes.get(k) == Some(&b'.') {
                        let mut m = k + 1;
                        if bytes.get(m).is_some_and(u8::is_ascii_digit) {
                            while m < bytes.len() && bytes[m].is_ascii_digit() {
                                m += 1;
                            }
                            return Some(stdout[i..m].to_string());
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// `"v2"` when the JS-`parseInt` major is `>= 2`, else `"v1"` (TS:
/// `getOpencodeApiVersion`).
pub fn get_opencode_api_version(version: &str) -> &'static str {
    match js_parse_int(version) {
        Some(major) if major >= 2 => "v2",
        _ => "v1",
    }
}

/// Pid owning `port` in `netstat -ano` output, else `None` (TS:
/// `findWindowsListeningPidInNetstat`).
pub fn find_windows_listening_pid_in_netstat(stdout: &str, port: u16) -> Option<u32> {
    let target = port as i64;
    for line in stdout.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let columns: Vec<&str> = trimmed.split_whitespace().collect();
        let local_address = columns.get(1).copied().unwrap_or("");
        if parse_socket_port(local_address) != Some(target) {
            continue;
        }
        if let Some(pid) = parse_pid(columns.last().copied().unwrap_or("")) {
            return Some(pid);
        }
    }
    None
}

/// First `pid=<digits>` hit on the line, mirroring TS
/// `trimmedLine.match(/pid=(\d+)/)` + `parsePid` (immediate digits required).
fn first_pid_in_ss_line(trimmed_line: &str) -> Option<u32> {
    let mut rest = trimmed_line;
    while let Some(idx) = rest.find("pid=") {
        let candidate = &rest[idx + "pid=".len()..];
        if candidate.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
            if let Some(pid) = parse_pid(candidate) {
                return Some(pid);
            }
        }
        rest = candidate;
    }
    None
}

/// Pid owning `port` in `ss -ltnp` output, else `None` (TS:
/// `findUnixListeningPidInSs`; see deviation 3 for the column scan).
pub fn find_unix_listening_pid_in_ss(stdout: &str, port: u16) -> Option<u32> {
    let target = port as i64;
    for line in stdout.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let port_hit = trimmed
            .split_whitespace()
            .any(|column| parse_socket_port(column) == Some(target));
        if !port_hit {
            continue;
        }
        if let Some(pid) = first_pid_in_ss_line(trimmed) {
            return Some(pid);
        }
    }
    None
}

/// First valid pid in `lsof -t` output, else `None` (TS:
/// `parseUnixPidList`).
pub fn parse_unix_pid_list(stdout: &str) -> Option<u32> {
    for line in stdout.split('\n') {
        if let Some(pid) = parse_pid(line) {
            return Some(pid);
        }
    }
    None
}
