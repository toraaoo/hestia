//! hestia — the Hestia command-line interface. A thin client over the daemon.

mod commands;
mod ui;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use common::LogLevel;

#[derive(Parser)]
#[command(name = "hestia", version, about = "Hestia command-line interface")]
struct Cli {
    #[arg(
        short,
        long,
        global = true,
        action = clap::ArgAction::Count,
        help = "Increase log verbosity (-v debug, -vv trace)"
    )]
    verbose: u8,
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
    /// Launch an instance (picks interactively when omitted)
    Play {
        /// Instance name or id; the sole instance launches directly
        instance: Option<String>,
        #[arg(long, help = "Account name or uuid (default: the switched-to account)")]
        account: Option<String>,
    },
    /// Minecraft accounts (Microsoft sign-in, switching)
    #[command(visible_alias = "auth")]
    Account {
        #[command(subcommand)]
        cmd: commands::account::AccountCmd,
    },
    /// Java runtimes (Eclipse Temurin via the Adoptium API)
    Java {
        #[command(subcommand)]
        cmd: commands::java::JavaCmd,
    },
    /// Minecraft servers (create, start/stop, logs)
    Server {
        #[command(subcommand)]
        cmd: commands::server::ServerCmd,
    },
    /// Minecraft instances (create, launch)
    Instance {
        #[command(subcommand)]
        cmd: commands::instance::InstanceCmd,
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
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let level = if cli.quiet {
        LogLevel::Warn
    } else {
        match cli.verbose {
            0 => LogLevel::default(),
            1 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    };
    let _guard = common::init_logging(level, None);
    tracing::debug!(version = common::app::VERSION, "hestia cli starting");

    // In the daemon model the data directory is daemon-global, so --home is
    // exported as $HESTIA_HOME and only takes effect when this invocation
    // auto-spawns the daemon; a daemon already running keeps its own directory.
    if let Some(home) = &cli.home {
        if !home.is_empty() {
            tracing::debug!(home, "exporting HESTIA_HOME for an auto-spawned daemon");
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
    ui::teardown();

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
        Command::Play { instance, account } => commands::play::run(instance, account).await,
        Command::Account { cmd } => commands::account::run(cmd).await,
        Command::Java { cmd } => commands::java::run(cmd).await,
        Command::Server { cmd } => commands::server::run(cmd).await,
        Command::Instance { cmd } => commands::instance::run(cmd).await,
        Command::Cache { cmd } => commands::cache::run(cmd).await,
        Command::Config { cmd } => commands::config::run(cmd).await,
        Command::Daemon { cmd } => commands::daemon::run(cmd).await,
    }
}
