//! otb-bot binary (T2 config/logging/CLI + T3 health parity).
//!
//! Minimal wiring only: load config (read-only `.env`), init logging, parse
//! `start|status`. `start` also spawns the optional health endpoint as a
//! background task and, only with `OTB_ENABLE_POLLING=1`, runs the Telegram
//! long-poll loop. No DB writes, no daemon changes.

mod cli;
/// T7: 23-command registry (structure only; no Telegram API calls).
mod commands;
mod config;
/// T10: daemon/side-by-side guard (structure only; pure, no binds/writes).
#[allow(dead_code)]
mod guard;
mod health;
/// T9: locale list + en fallback (structure only; no Telegram calls).
#[allow(dead_code)]
mod locale;
mod logging;
/// T8: media/TTS/STT parity (structure only; no network/ffmpeg calls).
#[allow(dead_code)]
mod media;
/// T7: 19-menu registry (structure only; no Telegram API calls).
mod menus;
mod telegram;
// T4: client functions only (read-only; mutations never called). Wiring
// lands in a later milestone, so allow dead code until then.
#[allow(dead_code)]
mod opencode;
/// T5: rusqlite session/project directory cache (isolated path only).
/// Not wired into the scaffold yet, so allow dead code until then.
#[allow(dead_code)]
mod store;
/// T15: read-only `settings.json` `current*` view for `/status`.
mod settings_current;

use clap::Parser;

use crate::cli::{Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_config = config::AppConfig::load().map_err(|err| anyhow::anyhow!(err))?;

    logging::init_logging(&app_config.log_level);

    let cli = Cli::parse();
    // Node defaults to `start` when no command is given.
    match cli.effective_command() {
        Command::Start => {
            tracing::info!(
                "[otb-bot] start: api_url={} server={}/{} locale={} health_port={}",
                app_config.opencode_api_url,
                app_config.opencode_server_version,
                app_config.opencode_server_mode,
                app_config.bot_locale,
                app_config.health_port,
            );
            let _health =
                crate::health::spawn(app_config.health_port, env!("CARGO_PKG_VERSION").to_string());
            // T13: real polling only when explicitly opted in
            // (`OTB_ENABLE_POLLING=1`); default stays scaffold-only so the
            // live install keeps its current exit-0 behavior. Tests never
            // set the flag, so they never block here.
            if crate::telegram::polling_enabled() {
                crate::telegram::run_telegram_from_env().await;
            } else {
                // T6: thin Telegram wiring only (dummy token; polling off).
                // No network calls here.
                crate::telegram::maybe_start_telegram();
                println!("otb-bot start: scaffold only (no polling, no DB writes)");
            }
        }
        Command::Status => {
            tracing::info!("[otb-bot] status: scaffold only");
            println!("{}", otb_core::cli_entry_gates::CLI_STATUS_STOPPED);
            println!(
                "{}",
                otb_core::cli_entry_gates::line_app_home(
                    &app_config.app_home.to_string_lossy()
                )
            );
            println!("commands: {}", crate::commands::command_count());
            println!(
                "dispatch(status): {}",
                crate::commands::dispatch("status").unwrap_or("unknown".to_string())
            );
            println!("menus: {}", crate::menus::menu_count());
        }
    }

    Ok(())
}
