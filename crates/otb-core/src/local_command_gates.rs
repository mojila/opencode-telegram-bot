//! Local-command-registry pure gates (slice 60, RED).
//!
//! Rust port of the pure remainder in:
//! - `src/app/services/local-command-registry.ts`
//!   (`COMMAND_NAME_PATTERN` /^[a-z0-9_]{1,32}$/, `COMMAND_LIMIT` 100,
//!   `EXECUTION_TIMEOUT_MS` 30000, `TELEGRAM_TEXT_LIMIT` 4096,
//!   `TRUNCATION_SUFFIX` "\n… [truncated]", `STDERR_TAIL_LIMIT` 1024,
//!   `isCommandFile` description/exec/`allowWhenBusy` validation,
//!   `.json` file filter + name derivation + sort, built-in collision gate,
//!   command-limit gate, `has` / `allowsWhenBusy` leading-slash lookup,
//!   `definitions` exec-stripping map, `appendBounded` head cap,
//!   `appendTail` tail cap, `truncate` over-limit cut, empty-stdout `empty`
//!   classification, unknown-command `failed` result, close-code success
//!   gate, working-directory `dirname` derivation)
//!
//! Step 1 (RED): every function is an `unimplemented!()` stub. The parity
//! tests in `crates/otb-core/tests/slice60_local_command_parity.rs` assert
//! the TS contract and MUST FAIL with `unimplemented!()` panics until the
//! Rust port lands (step 2).
//!
//! No child processes, no fs reads, no timers live here — only name
//! validation, limit gates, string-window transforms, and result
//! classification. Callers do the spawning/loading.
//!
//! DEVIATIONS from TS:
//! 1. `String.length` / `slice` count UTF-16 code units in TS. The port
//!    works on `char`s; parity fixtures use ASCII-only inputs where the
//!    two coincide.
//! 2. `appendTail` / `truncate` nudge the cut by one when it would split a
//!    UTF-16 surrogate pair. Rust `char`s never split a scalar value, so
//!    the port cuts exactly at the limit; ASCII fixtures match exactly.
//! 3. File-list sorting uses TS `localeCompare` in TS. The port sorts by
//!    byte order; ASCII file names coincide.
//! 4. `path.dirname` follows the bot host style in TS. The port derives
//!    the parent over `/` with a trailing-separator-tolerant fallback;
//!    parity fixtures use posix-style paths.
//! 5. `process.cwd()` fallback for `empty()` has no pure counterpart; the
//!    port models emptiness as zero loaded commands and leaves cwd out.
//! 6. (GREEN) `has` / `allowsWhenBusy` in TS do an unconditional
//!    `command.slice(1)`; the port strips one leading `/` only when
//!    present (passthrough otherwise), per the parity `strip_command_prefix`
//!    contract. Slash-prefixed lookups coincide.
//! 7. (GREEN) `classify_stdout_result` returns the stdout verbatim in
//!    `Success`; the TS `close` branch wraps it with `truncate(stdout)`.
//!    Callers apply `truncate_output` separately; short fixtures coincide.

// ---------------------------------------------------------------------------
// Budgets / markers (mirrors local-command-registry.ts)
// ---------------------------------------------------------------------------

/// Max local + built-in commands before Telegram refuses more.
pub const COMMAND_LIMIT: usize = 100;

/// Max command-name length (`{1,32}` in `COMMAND_NAME_PATTERN`).
pub const COMMAND_NAME_MAX_LEN: usize = 32;

/// Max command description length (`isCommandFile`).
pub const COMMAND_DESCRIPTION_MAX_LEN: usize = 256;

/// Command execution timeout (`EXECUTION_TIMEOUT_MS`).
pub const EXECUTION_TIMEOUT_MS: u64 = 30_000;

/// Telegram text budget for local-command output (`TELEGRAM_TEXT_LIMIT`).
pub const TELEGRAM_TEXT_LIMIT: usize = 4096;

/// Suffix marking truncated command output (`TRUNCATION_SUFFIX`).
pub const TRUNCATION_SUFFIX: &str = "\n… [truncated]";

/// Tail window kept for stderr output (`STDERR_TAIL_LIMIT`).
pub const STDERR_TAIL_LIMIT: usize = 1024;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

/// A command listing without its shell line (TS: `LocalCommandDefinition`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCommandDefinition {
    pub command: String,
    pub description: String,
    pub allow_when_busy: bool,
}

/// A loaded command including its shell line (TS: `LoadedCommand`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCommand {
    pub command: String,
    pub description: String,
    pub exec: String,
    pub allow_when_busy: bool,
}

/// Raw parsed `*.json` command file before validation (TS: `isCommandFile`
/// input via `JSON.parse` + `Reflect.get`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCommandFile {
    pub description: Option<String>,
    pub exec: Option<String>,
    pub allow_when_busy: Option<bool>,
    pub allow_when_busy_is_bool: bool,
}

/// Execution outcome (TS: `LocalCommandResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalCommandResult {
    Success {
        text: String,
    },
    Empty,
    Failed {
        exit_code: Option<i32>,
        stderr: String,
    },
    Timeout,
}

// ---------------------------------------------------------------------------
// Name / file validation (`COMMAND_NAME_PATTERN`, `isCommandFile`)
// ---------------------------------------------------------------------------

/// Lowercase alnum/underscore, 1–32 chars (TS: `COMMAND_NAME_PATTERN`).
pub fn is_valid_command_name(name: &str) -> bool {
    let len = name.chars().count();
    if len == 0 || len > COMMAND_NAME_MAX_LEN {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Non-empty, ≤256 chars, no CR/LF (TS: description branch of
/// `isCommandFile`).
pub fn is_valid_command_description(description: &str) -> bool {
    let len = description.chars().count();
    len > 0
        && len <= COMMAND_DESCRIPTION_MAX_LEN
        && !description.contains('\r')
        && !description.contains('\n')
}

/// Non-blank after trim (TS: exec branch of `isCommandFile`).
pub fn is_valid_command_exec(exec: &str) -> bool {
    !exec.trim().is_empty()
}

/// Full file validation (TS: `isCommandFile`).
pub fn is_command_file(raw: &RawCommandFile) -> bool {
    if !raw.allow_when_busy_is_bool {
        return false;
    }
    match raw.description.as_deref() {
        Some(d) if is_valid_command_description(d) => {}
        _ => return false,
    }
    match raw.exec.as_deref() {
        Some(e) if is_valid_command_exec(e) => {}
        _ => return false,
    }
    true
}

/// `true` for regular files ending in `.json` (TS: the `readdir` filter).
pub fn is_command_json_file(is_file: bool, file_name: &str) -> bool {
    is_file && file_name.ends_with(".json")
}

/// Strip the trailing `.json` suffix, or `None` (TS: `fileName.slice`).
pub fn command_name_from_file_name(file_name: &str) -> Option<String> {
    file_name.strip_suffix(".json").map(|s| s.to_string())
}

/// Byte-ordered file-name sort (TS: `localeCompare` sort over `.json`
/// names; ASCII coincides).
pub fn sort_command_file_names(names: &[String]) -> Vec<String> {
    let mut out = names.to_vec();
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// Registry gates (collision, limit, lookup, definitions)
// ---------------------------------------------------------------------------

/// `true` when the name collides with a built-in command (TS: `names.has`).
pub fn is_builtin_collision(builtin_commands: &[String], command: &str) -> bool {
    builtin_commands.iter().any(|b| b == command)
}

/// `true` when no more commands fit under the Telegram cap (TS:
/// `builtInCommands.length + commands.size >= COMMAND_LIMIT`).
pub fn command_limit_reached(builtin_count: usize, loaded_count: usize) -> bool {
    builtin_count.saturating_add(loaded_count) >= COMMAND_LIMIT
}

/// Strip one leading `/` for registry lookup (TS: `command.slice(1)`).
pub fn strip_command_prefix(command: &str) -> &str {
    command.strip_prefix('/').unwrap_or(command)
}

/// `true` when the registry holds the command (TS: `has`).
pub fn has_command(loaded: &[LoadedCommand], command: Option<&str>) -> bool {
    match command {
        None => false,
        Some(c) if c.is_empty() => false,
        Some(c) => {
            let key = strip_command_prefix(c);
            loaded.iter().any(|cmd| cmd.command == key)
        }
    }
}

/// `true` when the command may run while busy (TS: `allowsWhenBusy`).
pub fn allows_when_busy(loaded: &[LoadedCommand], command: Option<&str>) -> bool {
    match command {
        None => false,
        Some(c) if c.is_empty() => false,
        Some(c) => {
            let key = strip_command_prefix(c);
            loaded
                .iter()
                .any(|cmd| cmd.command == key && cmd.allow_when_busy)
        }
    }
}

/// Drop `exec` lines for listings (TS: `definitions`).
pub fn definitions_from_loaded(loaded: &[LoadedCommand]) -> Vec<LocalCommandDefinition> {
    loaded
        .iter()
        .map(|cmd| LocalCommandDefinition {
            command: cmd.command.clone(),
            description: cmd.description.clone(),
            allow_when_busy: cmd.allow_when_busy,
        })
        .collect()
}

/// Parent directory of the commands directory (TS: `path.dirname` for the
/// spawn `cwd`; posix-style inputs).
pub fn resolve_working_directory(directory_path: &str) -> String {
    if directory_path.is_empty() {
        return ".".to_string();
    }
    // Tolerate trailing separators, but keep a lone root intact.
    let mut end = directory_path.len();
    while end > 1 && directory_path.as_bytes()[end - 1] == b'/' {
        end -= 1;
    }
    let trimmed = &directory_path[..end];
    match trimmed.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(idx) => trimmed[..idx].to_string(),
    }
}

// ---------------------------------------------------------------------------
// Output windows (`appendBounded`, `appendTail`, `truncate`)
// ---------------------------------------------------------------------------

/// Append output keeping at most `limit` chars (TS: `appendBounded`).
pub fn append_bounded(current: &str, next: &str, limit: usize) -> String {
    if current.chars().count() >= limit {
        return current.to_string();
    }
    let combined = format!("{current}{next}");
    combined.chars().take(limit).collect()
}

/// Append stderr keeping only the last `limit` chars (TS: `appendTail`).
pub fn append_tail(current: &str, next: &str, limit: usize) -> String {
    let combined = format!("{current}{next}");
    let len = combined.chars().count();
    if len <= limit {
        return combined;
    }
    combined.chars().skip(len - limit).collect()
}

/// Cut over-limit output to `TELEGRAM_TEXT_LIMIT - suffix` + suffix
/// (TS: `truncate`).
pub fn truncate_output(text: &str) -> String {
    if text.chars().count() <= TELEGRAM_TEXT_LIMIT {
        return text.to_string();
    }
    let suffix_len = TRUNCATION_SUFFIX.chars().count();
    let end = TELEGRAM_TEXT_LIMIT.saturating_sub(suffix_len);
    let head: String = text.chars().take(end).collect();
    format!("{head}{TRUNCATION_SUFFIX}")
}

// ---------------------------------------------------------------------------
// Result classification (`execute` close/error/timeout branches)
// ---------------------------------------------------------------------------

/// Result for a command the registry does not hold (TS: the `!definition`
/// early return).
pub fn result_for_unknown_command() -> LocalCommandResult {
    LocalCommandResult::Failed {
        exit_code: None,
        stderr: String::new(),
    }
}

/// `true` for a zero close code (TS: `code !== 0` failure branch).
pub fn is_success_close_code(code: i32) -> bool {
    code == 0
}

/// Success vs `empty` by stdout length (TS: `stdout.length === 0` gate).
pub fn classify_stdout_result(stdout: &str) -> LocalCommandResult {
    if stdout.is_empty() {
        LocalCommandResult::Empty
    } else {
        LocalCommandResult::Success {
            text: stdout.to_string(),
        }
    }
}
