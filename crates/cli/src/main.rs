//! hestia — the Hestia command-line interface. A thin client over the daemon.

mod commands;
mod output;
mod ui;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use common::LogLevel;

#[derive(Parser)]
#[command(name = "hestia", version, about = "Hestia command-line interface")]
struct Cli {
    #[arg(short, long, global = true, help = "Enable verbose (debug) logging")]
    verbose: bool,
    #[arg(
        short,
        long,
        global = true,
        conflicts_with = "verbose",
        help = "Only show warnings and errors"
    )]
    quiet: bool,
    #[arg(
        long,
        global = true,
        help = "Override Hestia's data directory (else $HESTIA_HOME, else the platform default)"
    )]
    home: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Minecraft accounts (Microsoft sign-in)
    Auth {
        #[command(subcommand)]
        cmd: commands::auth::AuthCmd,
    },
    /// Java runtimes (Eclipse Temurin via the Adoptium API)
    Java {
        #[command(subcommand)]
        cmd: commands::java::JavaCmd,
    },
    /// Download cache
    Cache {
        #[command(subcommand)]
        cmd: commands::cache::CacheCmd,
    },
    /// Configuration (typed settings, stored as JSON)
    Config {
        #[command(subcommand)]
        cmd: commands::config::ConfigCmd,
    },
    /// Daemon lifecycle
    Daemon {
        #[command(subcommand)]
        cmd: commands::daemon::DaemonCmd,
    },
    /// Launch and supervise processes through the daemon
    Process {
        #[command(subcommand)]
        cmd: commands::process::ProcessCmd,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let level = if cli.verbose {
        LogLevel::Debug
    } else if cli.quiet {
        LogLevel::Warn
    } else {
        LogLevel::Info
    };
    let _guard = common::init_logging(level, None);

    // In the daemon model the data directory is daemon-global, so --home is
    // exported as $HESTIA_HOME and only takes effect when this invocation
    // auto-spawns the daemon; a daemon already running keeps its own directory.
    if let Some(home) = &cli.home {
        if !home.is_empty() {
            std::env::set_var("HESTIA_HOME", home);
        }
    }

    let Some(command) = cli.command else {
        // No subcommand given: show usage.
        let _ = <Cli as clap::CommandFactory>::command().print_help();
        println!();
        return ExitCode::SUCCESS;
    };

    let rt = tokio::runtime::Runtime::new().expect("build tokio runtime");
    let result = rt.block_on(dispatch(command));

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn dispatch(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Auth { cmd } => commands::auth::run(cmd).await,
        Command::Java { cmd } => commands::java::run(cmd).await,
        Command::Cache { cmd } => commands::cache::run(cmd).await,
        Command::Config { cmd } => commands::config::run(cmd).await,
        Command::Daemon { cmd } => commands::daemon::run(cmd).await,
        Command::Process { cmd } => commands::process::run(cmd).await,
    }
}
