//! CLI parity: `start|status` with `--help`/`--version` (T2).
//!
//! Node parity (`src/cli.ts`, `src/cli/args.ts`): no command defaults to
//! `start`; `status` reports whether the service runs. `stop`/`config` and
//! `--daemon`/`--mode` stay Node-only for now.

use clap::{Parser, Subcommand};

/// CLI usage mirrors Node `src/cli.ts` (start/status subset for T2).
#[derive(Debug, Parser)]
#[command(
    name = "otb-bot",
    version,
    about = "OpenCode Telegram Bot (Rust port)",
    long_about = "OpenCode Telegram Bot (Rust port).\n\nUsage:\n  otb-bot [start]\n  otb-bot status"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Run the bot in foreground (mirrors `opencode-telegram [start]`).
    Start,
    /// Report service status (mirrors `opencode-telegram status`).
    Status,
}

impl Cli {
    /// Effective command; `None` defaults to `Start` like Node.
    pub fn effective_command(&self) -> Command {
        self.command.unwrap_or(Command::Start)
    }
}

/// Parse CLI args (used by `main` and tests).
pub fn parse_cli<I, T>(args: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::parse_from(args)
}
