//! Pure bot-bootstrap / CLI / runtime-mode gates (slice 38, GREEN).
//!
//! Rust port of the pure remainder in:
//! - `src/cli/args.ts` (`parseCliArgs`, `isCliCommand`, mode
//!   normalization, unknown-command / invalid-mode / unknown-option /
//!   mode-requires-value / start-only error messages)
//! - `src/runtime/mode.ts` (`normalizeMode`, `parseModeFromArgv`,
//!   `hasInvalidModeSyntax`, `resolveRuntimeMode`, `getRuntimeMode`
//!   env-fallback normalization)
//! - `src/bot/index.ts` (`TRANSIENT_RETRY_SAFE_TELEGRAM_METHODS` +
//!   `shouldRetryTelegramServerError`, `STARTUP_MANAGED_TELEGRAM_METHODS`,
//!   `CHAT_DELIVERY_TELEGRAM_METHODS`, `withBotLinkPreviewDefault`
//!   text/default gates)
//! - `src/bot/telegram-client-options.ts` (client-wiring gates: custom
//!   `apiRoot`/`proxySecret`, forward-proxy vs force-IPv4 precedence,
//!   SOCKS scheme pick, proxy-credential redaction)
//!
//! Pure sync logic only: no grammy/Telegram I/O, no fetch, no timers, no
//! `process.env` reads (env input is passed in as `Option<&str>` where the
//! TS source reads it).

/// CLI subcommands (`src/cli/args.ts`: `CliCommand`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliCommand {
    Start,
    Status,
    Stop,
    Config,
}

/// Runtime modes (`src/runtime/mode.ts`: `RuntimeMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Sources,
    Installed,
}

/// Parsed CLI result (TS: `ParsedCliArgs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCliArgs {
    pub command: CliCommand,
    pub mode: Option<RuntimeMode>,
    pub daemon: bool,
    pub show_help: bool,
    pub error: Option<String>,
}

/// `resolveRuntimeMode` result (TS: `ResolveRuntimeModeResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntimeMode {
    pub mode: RuntimeMode,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// CLI args (`src/cli/args.ts`)
// ---------------------------------------------------------------------------

/// `start` / `status` / `stop` / `config` (TS: `isCliCommand`).
pub fn is_cli_command(value: &str) -> bool {
    matches!(value, "start" | "status" | "stop" | "config")
}

/// `"installed"` / `"sources"`, else `None` (TS: `normalizeMode`).
pub fn normalize_cli_mode(value: &str) -> Option<RuntimeMode> {
    match value {
        "installed" => Some(RuntimeMode::Installed),
        "sources" => Some(RuntimeMode::Sources),
        _ => None,
    }
}

const MODE_REQUIRES_VALUE: &str = "Option --mode requires a value: sources|installed";
const MODE_ONLY_START: &str = "Option --mode is supported only for the start command";
const DAEMON_ONLY_START: &str = "Option --daemon is supported only for the start command";

/// Full argv parse with start defaults and start-only guards
/// (TS: `parseCliArgs`).
pub fn parse_cli_args(argv: &[&str]) -> ParsedCliArgs {
    let mut command = CliCommand::Start;
    let mut mode: Option<RuntimeMode> = None;
    let mut daemon = false;
    let mut show_help = false;
    let mut current_index = 0;

    if let Some(first_arg) = argv.first() {
        if !first_arg.is_empty() && !first_arg.starts_with('-') {
            if !is_cli_command(first_arg) {
                return ParsedCliArgs {
                    command,
                    mode: None,
                    daemon: false,
                    show_help: true,
                    error: Some(cli_unknown_command_message(first_arg)),
                };
            }
            command = match *first_arg {
                "start" => CliCommand::Start,
                "status" => CliCommand::Status,
                "stop" => CliCommand::Stop,
                _ => CliCommand::Config,
            };
            current_index = 1;
        }
    }

    while current_index < argv.len() {
        let token = argv[current_index];
        if token.is_empty() {
            break;
        }

        if token == "--help" || token == "-h" {
            show_help = true;
            current_index += 1;
            continue;
        }

        if token == "--daemon" {
            daemon = true;
            current_index += 1;
            continue;
        }

        if token == "--mode" {
            let mode_value = argv.get(current_index + 1).copied();
            match mode_value {
                Some(value) if !value.is_empty() && !value.starts_with('-') => {
                    match normalize_cli_mode(value) {
                        Some(parsed) => {
                            mode = Some(parsed);
                            current_index += 2;
                        }
                        None => {
                            return ParsedCliArgs {
                                command,
                                mode,
                                daemon,
                                show_help: true,
                                error: Some(cli_invalid_mode_message(value)),
                            };
                        }
                    }
                }
                _ => {
                    return ParsedCliArgs {
                        command,
                        mode,
                        daemon,
                        show_help: true,
                        error: Some(MODE_REQUIRES_VALUE.to_string()),
                    };
                }
            }
            continue;
        }

        if let Some(mode_value) = token.strip_prefix("--mode=") {
            match normalize_cli_mode(mode_value) {
                Some(parsed) => {
                    mode = Some(parsed);
                    current_index += 1;
                }
                None => {
                    return ParsedCliArgs {
                        command,
                        mode,
                        daemon,
                        show_help: true,
                        error: Some(cli_invalid_mode_message(mode_value)),
                    };
                }
            }
            continue;
        }

        return ParsedCliArgs {
            command,
            mode,
            daemon,
            show_help: true,
            error: Some(cli_unknown_option_message(token)),
        };
    }

    if command != CliCommand::Start && mode.is_some() {
        return ParsedCliArgs {
            command,
            mode,
            daemon,
            show_help: true,
            error: Some(MODE_ONLY_START.to_string()),
        };
    }

    if command != CliCommand::Start && daemon {
        return ParsedCliArgs {
            command,
            mode,
            daemon,
            show_help: true,
            error: Some(DAEMON_ONLY_START.to_string()),
        };
    }

    ParsedCliArgs {
        command,
        mode,
        daemon,
        show_help,
        error: None,
    }
}

/// `` `Unknown command: ${value}` `` (TS: `CLI_MESSAGES.unknownCommand`).
pub fn cli_unknown_command_message(value: &str) -> String {
    format!("Unknown command: {value}")
}

/// `` `Invalid mode value: ${value}. Expected sources|installed` ``
/// (TS: `CLI_MESSAGES.invalidMode`).
pub fn cli_invalid_mode_message(value: &str) -> String {
    format!("Invalid mode value: {value}. Expected sources|installed")
}

/// `"Unknown option: ..."` (TS: `CLI_MESSAGES.unknownOption`).
pub fn cli_unknown_option_message(value: &str) -> String {
    format!("Unknown option: {value}")
}

// ---------------------------------------------------------------------------
// Runtime mode (`src/runtime/mode.ts`)
// ---------------------------------------------------------------------------

/// `"installed"` / `"sources"`, else `None` (TS: `normalizeMode`).
pub fn normalize_runtime_mode(value: &str) -> Option<RuntimeMode> {
    match value {
        "installed" => Some(RuntimeMode::Installed),
        "sources" => Some(RuntimeMode::Sources),
        _ => None,
    }
}

/// Last `--mode` / `--mode=` value wins; `None` when absent or when a
/// `--mode` flag lacks a usable value (TS: `parseModeFromArgv`).
///
/// Deviation: the TS loop `return null`s immediately on a bare `--mode`
/// flag; the Rust port returns `None` immediately at that point too,
/// discarding any earlier valid value — same observable result.
pub fn parse_mode_from_argv(argv: &[&str]) -> Option<RuntimeMode> {
    let mut mode: Option<RuntimeMode> = None;
    let mut index = 0;
    while index < argv.len() {
        let token = argv[index];
        if token.is_empty() {
            index += 1;
            continue;
        }
        if token == "--mode" {
            let value = argv.get(index + 1).copied();
            match value {
                Some(v) if !v.is_empty() && !v.starts_with('-') => {
                    mode = normalize_runtime_mode(v);
                    index += 2;
                }
                _ => return None,
            }
            continue;
        }
        if let Some(value) = token.strip_prefix("--mode=") {
            mode = normalize_runtime_mode(value);
        }
        index += 1;
    }
    mode
}

/// True when any `--mode` flag is missing its value or names an unknown
/// mode (TS: `hasInvalidModeSyntax`).
pub fn has_invalid_mode_syntax(argv: &[&str]) -> bool {
    let mut index = 0;
    while index < argv.len() {
        let token = argv[index];
        if token.is_empty() {
            index += 1;
            continue;
        }
        if token == "--mode" {
            let value = argv.get(index + 1).copied();
            match value {
                Some(v) if !v.is_empty() && !v.starts_with('-') => {
                    if normalize_runtime_mode(v).is_none() {
                        return true;
                    }
                    index += 1;
                }
                _ => return true,
            }
            index += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--mode=") {
            if normalize_runtime_mode(value).is_none() {
                return true;
            }
        }
        index += 1;
    }
    false
}

/// Explicit wins; invalid argv syntax keeps the default with an error;
/// otherwise argv wins over the default (TS: `resolveRuntimeMode`).
pub fn resolve_runtime_mode(
    default: RuntimeMode,
    argv: &[&str],
    explicit: Option<RuntimeMode>,
) -> ResolvedRuntimeMode {
    if let Some(mode) = explicit {
        return ResolvedRuntimeMode { mode, error: None };
    }
    if has_invalid_mode_syntax(argv) {
        return ResolvedRuntimeMode {
            mode: default,
            error: Some("Invalid value for --mode. Expected sources|installed".to_string()),
        };
    }
    ResolvedRuntimeMode {
        mode: parse_mode_from_argv(argv).unwrap_or(default),
        error: None,
    }
}

/// Env normalization behind `getRuntimeMode`: unknown/missing falls back
/// to `sources` (TS: `normalizeMode(raw) ?? "sources"`).
pub fn normalize_env_mode(raw: Option<&str>) -> RuntimeMode {
    match raw {
        Some(value) => normalize_runtime_mode(value).unwrap_or(RuntimeMode::Sources),
        None => RuntimeMode::Sources,
    }
}

// ---------------------------------------------------------------------------
// Bot API method sets (`src/bot/index.ts`)
// ---------------------------------------------------------------------------

/// The five idempotent methods safe to retry after a transient 5xx
/// (TS: `TRANSIENT_RETRY_SAFE_TELEGRAM_METHODS`).
pub fn should_retry_telegram_server_error(method: &str) -> bool {
    matches!(
        method,
        "editMessageReplyMarkup"
            | "editMessageText"
            | "sendChatAction"
            | "sendMessageDraft"
            | "sendRichMessageDraft"
    )
}

/// `deleteWebhook` / `getMe` / `getWebhookInfo` bypass the API-transformer
/// retry (TS: `STARTUP_MANAGED_TELEGRAM_METHODS`).
pub fn is_startup_managed_method(method: &str) -> bool {
    matches!(method, "deleteWebhook" | "getMe" | "getWebhookInfo")
}

/// Chat-delivery methods tracked for outage notices (TS:
/// `CHAT_DELIVERY_TELEGRAM_METHODS`: `sendMessage`, `sendRichMessage`,
/// `editMessageText`, `sendDocument`, `sendAudio`, `sendPhoto`).
pub fn is_chat_delivery_method(method: &str) -> bool {
    matches!(
        method,
        "sendMessage"
            | "sendRichMessage"
            | "editMessageText"
            | "sendDocument"
            | "sendAudio"
            | "sendPhoto"
    )
}

/// True when the bot should stamp `link_preview_options: { is_disabled:
/// true }`: only `sendMessage`/`editMessageText` carrying caller text and
/// no caller-set preview choice (TS: `withBotLinkPreviewDefault`).
pub fn should_apply_link_preview_default(
    method: &str,
    has_text: bool,
    has_existing_option: bool,
) -> bool {
    (method == "sendMessage" || method == "editMessageText") && has_text && !has_existing_option
}

// ---------------------------------------------------------------------------
// Telegram client wiring (`src/bot/telegram-client-options.ts`)
// ---------------------------------------------------------------------------

/// `"socks"` when the proxy URL starts with `socks`, `"https"` for any
/// other non-empty proxy URL, `"none"` when empty (TS: the
/// `proxyUrl.startsWith("socks")` branch).
pub fn proxy_agent_kind(proxy_url: &str) -> &'static str {
    if proxy_url.is_empty() {
        "none"
    } else if proxy_url.starts_with("socks") {
        "socks"
    } else {
        "https"
    }
}

/// A custom grammy client object is needed when `apiRoot` or `proxySecret`
/// is set (TS: the `apiRoot || proxySecret` gate).
pub fn should_configure_telegram_client(api_root: &str, proxy_secret: &str) -> bool {
    !api_root.is_empty() || !proxy_secret.is_empty()
}

/// A forward proxy agent is wired whenever `proxyUrl` is non-empty, taking
/// precedence over `forceIpv4` (TS: the `if (telegram.proxyUrl)` branch).
pub fn should_use_forward_proxy(proxy_url: &str) -> bool {
    !proxy_url.is_empty()
}

/// `proxyUrl.replace(/\/\/.*@/, "//***@")` for safe logging (TS: proxy log
/// lines in `createTelegramBotOptions`).
///
/// Deviation: the JS regex is greedy (`.*` to the last `@`); the Rust port
/// mirrors that by cutting from the first `//` to the last `@`.
pub fn redact_proxy_credentials(url: &str) -> String {
    let Some(slashes) = url.find("//") else {
        return url.to_string();
    };
    let prefix_end = slashes + 2;
    let Some(at) = url.rfind('@') else {
        return url.to_string();
    };
    if at < prefix_end {
        return url.to_string();
    }
    format!("{}***@{}", &url[..prefix_end], &url[at + 1..])
}
