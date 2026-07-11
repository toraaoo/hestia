//! hestia — the Hestia command-line interface. A thin client over the daemon.

mod commands;
mod ui;

use std::path::Path;
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
        help = "Only show errors on the console"
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
        #[arg(short, long, help = "Return immediately instead of following the logs")]
        detach: bool,
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
    /// Start a server or launch an instance by name
    Start {
        /// Server or instance name or id
        target: String,
        #[arg(
            long,
            help = "Account name or uuid for an instance (default: the switched-to account)"
        )]
        account: Option<String>,
        #[arg(short, long, help = "Return immediately instead of attaching")]
        detach: bool,
    },
    /// Stop a running server or instance by name
    Stop {
        /// Server or instance name or id
        target: String,
    },
    /// Restart a server or instance by name
    Restart {
        /// Server or instance name or id
        target: String,
        #[arg(
            long,
            help = "Account name or uuid for an instance (default: the switched-to account)"
        )]
        account: Option<String>,
        #[arg(short, long, help = "Return immediately instead of attaching")]
        detach: bool,
    },
    /// Tail a server or instance's captured output by name
    Logs {
        /// Server or instance name or id
        target: String,
        #[arg(short = 'n', long = "tail", help = "Only the last N lines")]
        tail: Option<usize>,
        #[arg(short, long, help = "Keep streaming new output until Ctrl-C")]
        follow: bool,
    },
    /// Browse mods on a content source
    Mod {
        #[command(subcommand)]
        cmd: commands::content::BrowseCmd,
    },
    /// Browse modpacks on a content source
    Modpack {
        #[command(subcommand)]
        cmd: commands::content::BrowseCmd,
    },
    /// Browse resource packs on a content source
    Resourcepack {
        #[command(subcommand)]
        cmd: commands::content::BrowseCmd,
    },
    /// Browse shaders on a content source
    Shader {
        #[command(subcommand)]
        cmd: commands::content::BrowseCmd,
    },
    /// Browse data packs on a content source
    Datapack {
        #[command(subcommand)]
        cmd: commands::content::BrowseCmd,
    },
    /// Search mods on a content source (alias for `mod search`)
    Search {
        /// Search terms
        query: Option<String>,
        #[arg(short, long, help = "Filter by loader (e.g. fabric)")]
        loader: Option<String>,
        #[arg(short = 'g', long = "game-version", help = "Filter by game version")]
        game_version: Option<String>,
        #[arg(short = 'S', long, help = "Content source (default: modrinth)")]
        source: Option<String>,
    },
    /// The available content sources
    Sources,
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
    /// Update Hestia itself from the release feed
    #[command(name = "self-update")]
    SelfUpdate {
        #[arg(short = 'y', long, help = "Apply without the confirmation prompt")]
        yes: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Diagnostics go to an appended logs/hestia.log so they never garble command
    // output; the console only sees warnings and errors unless -v/-vv raise it.
    let (console_level, file_level) = if cli.quiet {
        (LogLevel::Error, LogLevel::Warn)
    } else {
        match cli.verbose {
            0 => (LogLevel::Warn, LogLevel::default()),
            1 => (LogLevel::Debug, LogLevel::Debug),
            _ => (LogLevel::Trace, LogLevel::Trace),
        }
    };
    let home = cli.home.as_deref().filter(|h| !h.is_empty()).map(Path::new);
    let file = common::FileLog::appending(common::paths::log_dir(home), "hestia", file_level);
    let _guard = common::init_logging(console_level, Some(file));
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
        Command::Play {
            instance,
            account,
            detach,
        } => commands::play::run(instance, account, detach).await,
        Command::Account { cmd } => commands::account::run(cmd).await,
        Command::Java { cmd } => commands::java::run(cmd).await,
        Command::Server { cmd } => commands::server::run(cmd).await,
        Command::Instance { cmd } => commands::instance::run(cmd).await,
        Command::Start {
            target,
            account,
            detach,
        } => commands::lifecycle::start(target, account, detach).await,
        Command::Stop { target } => commands::lifecycle::stop(target).await,
        Command::Restart {
            target,
            account,
            detach,
        } => commands::lifecycle::restart(target, account, detach).await,
        Command::Logs {
            target,
            tail,
            follow,
        } => commands::lifecycle::logs(target, tail, follow).await,
        Command::Mod { cmd } => {
            commands::content::run_browse(client::proto::content::ContentKind::Mod, cmd).await
        }
        Command::Modpack { cmd } => {
            commands::content::run_browse(client::proto::content::ContentKind::Modpack, cmd).await
        }
        Command::Resourcepack { cmd } => {
            commands::content::run_browse(client::proto::content::ContentKind::ResourcePack, cmd)
                .await
        }
        Command::Shader { cmd } => {
            commands::content::run_browse(client::proto::content::ContentKind::Shader, cmd).await
        }
        Command::Datapack { cmd } => {
            commands::content::run_browse(client::proto::content::ContentKind::DataPack, cmd).await
        }
        Command::Search {
            query,
            loader,
            game_version,
            source,
        } => {
            commands::content::run_browse(
                client::proto::content::ContentKind::Mod,
                commands::content::BrowseCmd::Search {
                    query,
                    loader,
                    game_version,
                    category: Vec::new(),
                    sort: commands::content::SortArg::Relevance,
                    source,
                    limit: 20,
                    offset: 0,
                },
            )
            .await
        }
        Command::Sources => commands::content::run_sources().await,
        Command::Cache { cmd } => commands::cache::run(cmd).await,
        Command::Config { cmd } => commands::config::run(cmd).await,
        Command::Daemon { cmd } => commands::daemon::run(cmd).await,
        Command::SelfUpdate { yes } => commands::update::run(yes).await,
    }
}
