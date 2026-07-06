//! hestiad — the Hestia daemon.
//!
//!   hestiad [serve]   run the daemon: bind the endpoint, serve until signalled
//!   hestiad ping      connect to a running daemon, report its identity
//!
//! main() only bootstraps: CLI parsing, logging init, and dispatch. The serve
//! loop lives in server.rs; every channel lives in services.rs.

mod autostart;
mod runtime;
mod server;
mod services;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use common::LogLevel;

#[derive(Parser)]
#[command(name = "hestiad", version, about = "hestiad — the Hestia daemon")]
struct Cli {
    #[arg(
        short,
        long,
        action = clap::ArgAction::Count,
        help = "Increase log verbosity (-v debug, -vv trace)"
    )]
    verbose: u8,
    #[arg(
        short,
        long,
        help = "Warnings and errors only",
        conflicts_with = "verbose"
    )]
    quiet: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the daemon (default)
    Serve,
    /// Check that a running daemon is reachable
    Ping,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let level = if cli.quiet {
        LogLevel::Warn
    } else {
        match cli.verbose {
            0 => LogLevel::Info,
            1 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    };

    let rt = tokio::runtime::Runtime::new().expect("build tokio runtime");

    let code = match cli.command {
        Some(Command::Ping) => {
            // ping is a one-shot foreground tool — stderr only.
            let _guard = common::init_logging(level, None);
            rt.block_on(run_ping())
        }
        _ => {
            // The long-lived daemon also logs to a rotating, compressed file, since
            // clients detach its stderr.
            let file = common::FileLog::new(common::paths::log_dir(None), "hestiad");
            let log_path = file.active_path();
            let _guard = common::init_logging(level, Some(file));
            rt.block_on(server::run_daemon(log_path))
        }
    };
    ExitCode::from(code as u8)
}

async fn run_ping() -> i32 {
    match client::Client::connect(false).await {
        Ok(client) => match client.app().info().await {
            Ok(info) => {
                println!("{} {} — alive", info.name, info.version);
                0
            }
            Err(e) => {
                eprintln!("hestiad ping: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("hestiad ping: {e}");
            1
        }
    }
}
