//! Bot command definitions active-filter gates (slice 164, GREEN).
//!
//! Rust port of the pure sync remainder in
//! `src/bot/commands/definitions.ts`:
//! - `COMMAND_DEFINITIONS` order (the canonical Telegram command list)
//! - the `serverVersion` filter (`ACTIVE_COMMAND_DEFINITIONS`: keep entries
//!   with no `serverVersion`, or with `serverVersion === current`)
//! - `BOT_COMMANDS` / `getLocalizedBotCommands` shape (same order as the
//!   active definitions; description localization stays with the caller)
//! - `BUILT_IN_COMMAND_NAMES` (`"start"` first, then the active commands)
//!
//! AUDIT (branch `rust-rewrite`):
//! - `src/bot/commands/definitions.ts` lines 26-50 (`COMMAND_DEFINITIONS`
//!   with `reload` / `goal` gated on `serverVersion: "v2"`), 52-54
//!   (`ACTIVE_COMMAND_DEFINITIONS` filter), 56-68
//!   (`getLocalizedBotCommands` / `BOT_COMMANDS` / `BUILT_IN_COMMAND_NAMES`).
//! - Contract asserted by `tests/bot/commands/definitions.test.ts`: on V2
//!   `/reload` sits right after `/opencode_stop`, the localized list matches
//!   the command order, built-ins contain `reload` and `goal`; on V1 both
//!   are absent everywhere.
//!
//! Pure sync logic only. Impure work stays with the caller:
//! - reading `config.opencode.serverVersion` (passed in here as `current`),
//! - resolving `t(descriptionKey)` descriptions, `setMyCommands` delivery.
//!
//! Already ported elsewhere and NOT re-tested here:
//! - per-command handlers / routers (their own gates owners)
//! - locale strings for `cmd.description.*` (i18n owners)
//!
//! Deviations: none. All comparisons are exact ASCII command-name equality;
//! no width measurement is involved, so TS UTF-16 units and Rust `chars()`
//! coincide trivially.
//!
//! Step 2 (GREEN): functions are ported from the TS contract asserted by
//! the parity tests in
//! `crates/otb-core/tests/slice164_command_definitions_parity.rs`.

/// One row of `COMMAND_DEFINITIONS` (TS: `BotCommandI18nDefinition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDefinition {
    pub command: &'static str,
    /// `None` means available on every server version (TS: no `serverVersion`).
    pub server_version: Option<&'static str>,
}

/// Canonical command list in Telegram display order
/// (TS: `COMMAND_DEFINITIONS`).
pub const COMMAND_DEFINITIONS: &[CommandDefinition] = &[
    CommandDefinition {
        command: "status",
        server_version: None,
    },
    CommandDefinition {
        command: "new",
        server_version: None,
    },
    CommandDefinition {
        command: "abort",
        server_version: None,
    },
    CommandDefinition {
        command: "detach",
        server_version: None,
    },
    CommandDefinition {
        command: "sessions",
        server_version: None,
    },
    CommandDefinition {
        command: "recent",
        server_version: None,
    },
    CommandDefinition {
        command: "messages",
        server_version: None,
    },
    CommandDefinition {
        command: "settings",
        server_version: None,
    },
    CommandDefinition {
        command: "projects",
        server_version: None,
    },
    CommandDefinition {
        command: "worktree",
        server_version: None,
    },
    CommandDefinition {
        command: "task",
        server_version: None,
    },
    CommandDefinition {
        command: "tasklist",
        server_version: None,
    },
    CommandDefinition {
        command: "rename",
        server_version: None,
    },
    CommandDefinition {
        command: "commands",
        server_version: None,
    },
    CommandDefinition {
        command: "skills",
        server_version: None,
    },
    CommandDefinition {
        command: "mcps",
        server_version: None,
    },
    CommandDefinition {
        command: "opencode_start",
        server_version: None,
    },
    CommandDefinition {
        command: "opencode_stop",
        server_version: None,
    },
    CommandDefinition {
        command: "reload",
        server_version: Some("v2"),
    },
    CommandDefinition {
        command: "goal",
        server_version: Some("v2"),
    },
    CommandDefinition {
        command: "open",
        server_version: None,
    },
    CommandDefinition {
        command: "ls",
        server_version: None,
    },
    CommandDefinition {
        command: "help",
        server_version: None,
    },
];

/// Active filter for one definition
/// (TS: `!serverVersion || serverVersion === config.opencode.serverVersion`).
pub fn is_command_active(server_version: Option<&str>, current: &str) -> bool {
    match server_version {
        None => true,
        Some(required) => required == current,
    }
}

/// Active command names in canonical order, for a given server version
/// (TS: `ACTIVE_COMMAND_DEFINITIONS.map(({ command }) => command)`).
pub fn active_command_names(current: &str) -> Vec<&'static str> {
    COMMAND_DEFINITIONS
        .iter()
        .filter(|definition| is_command_active(definition.server_version, current))
        .map(|definition| definition.command)
        .collect()
}

/// Built-in command names: `"start"` first, then the active commands
/// (TS: `BUILT_IN_COMMAND_NAMES`).
pub fn built_in_command_names(current: &str) -> Vec<&'static str> {
    let mut names = vec!["start"];
    names.extend(active_command_names(current));
    names
}

/// Position of a command in the active list, if present.
pub fn active_command_index(command: &str, current: &str) -> Option<usize> {
    active_command_names(current)
        .iter()
        .position(|name| *name == command)
}

/// True when `reload` sits immediately after `opencode_stop`
/// (TS: `commands[commands.indexOf("opencode_stop") + 1] === "reload"`).
pub fn reload_follows_opencode_stop(current: &str) -> bool {
    let names = active_command_names(current);
    match names.iter().position(|name| *name == "opencode_stop") {
        None => false,
        Some(index) => names.get(index + 1) == Some(&"reload"),
    }
}
