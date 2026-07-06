//! `hestia server …` — create fully provisioned servers and drive them through
//! the daemon. Creation walks through flavor/version pickers (and the EULA
//! confirm) when arguments are omitted.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use client::proto::minecraft::ConfigEntry;
use client::proto::process::{ProcessInfo, ProcessState};
use client::proto::server::{ServerCreateParams, ServerInfo};
use client::{Client, ProcessEvent};

use crate::commands::mc;
use crate::ui::{self, ProvisionReporter, Spinner, View};

const EULA_URL: &str = "https://aka.ms/MinecraftEULA";

/// Everything `server create` accepts; folded into the create params and the
/// create-time settings list.
#[derive(clap::Args)]
pub struct CreateArgs {
    /// Flavor id (e.g. vanilla, fabric)
    flavor: Option<String>,
    /// Game version (e.g. 1.21.1)
    version: Option<String>,
    #[arg(
        short,
        long,
        help = "Pin a loader version (modloaders only; default latest)"
    )]
    loader: Option<String>,
    #[arg(short, long, help = "Display name (defaults to <flavor>-<version>)")]
    name: Option<String>,
    #[arg(
        long,
        help = "Accept the Minecraft EULA (https://aka.ms/MinecraftEULA)"
    )]
    eula: bool,
    #[arg(short, long, help = "Pin the game port (default: lowest free)")]
    port: Option<u16>,
    #[arg(long, help = "Set -Xms and -Xmx together (e.g. 4G, 2048M)")]
    memory: Option<String>,
    #[arg(long, help = "The message of the day shown in the server list")]
    motd: Option<String>,
    #[arg(long, help = "Maximum number of players")]
    max_players: Option<u32>,
    #[arg(long, value_parser = ["peaceful", "easy", "normal", "hard"])]
    difficulty: Option<String>,
    #[arg(long, value_parser = ["survival", "creative", "adventure", "spectator"])]
    gamemode: Option<String>,
    #[arg(long, help = "World seed (level-seed)")]
    seed: Option<String>,
    #[arg(
        long = "prop",
        value_name = "KEY=VALUE",
        help = "Set any other server.properties entry (repeatable; wins over the dedicated flags)"
    )]
    prop: Vec<String>,
}

#[derive(Subcommand)]
pub enum ServerCmd {
    /// Create a fully provisioned server (prompts for anything omitted)
    Create {
        #[command(flatten)]
        args: CreateArgs,
    },
    /// Managed servers and their state
    #[command(visible_alias = "ls")]
    List,
    /// Get, set, or list this server's settings (memory, jvm-args, server.properties)
    Config {
        /// Server name or id
        server: String,
        #[command(subcommand)]
        cmd: mc::ConfigCmd,
    },
    /// Attach an interactive console: live logs, type to send commands
    #[command(visible_alias = "console")]
    Attach {
        /// Server name or id
        server: String,
    },
    /// Send one console command and print the reply
    #[command(visible_alias = "cmd")]
    Command {
        /// Server name or id
        server: String,
        /// The command, as it would be typed in the console
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Start a server under the daemon's supervisor
    Start {
        /// Server name or id
        server: String,
    },
    /// Stop a running server
    Stop {
        /// Server name or id
        server: String,
    },
    /// Stop a running server and start it again
    Restart {
        /// Server name or id
        server: String,
    },
    /// A server's record merged with its live process state
    Status {
        /// Server name or id
        server: String,
    },
    /// Captured server output
    Logs {
        /// Server name or id
        server: String,
        #[arg(short = 'n', long = "tail", help = "Only the last N lines")]
        tail: Option<usize>,
        #[arg(short, long, help = "Keep streaming new output until Ctrl-C")]
        follow: bool,
    },
    /// Delete a server (its jar, world and all)
    #[command(visible_alias = "rm")]
    Remove {
        /// Server name or id
        server: String,
    },
    /// Game versions a flavor offers (prompts for the flavor when omitted)
    Versions {
        /// Flavor id (e.g. vanilla, fabric)
        flavor: Option<String>,
        #[arg(long, help = "Include snapshots and old versions")]
        all: bool,
    },
    /// The available flavors
    Flavors,
}

pub async fn run(cmd: ServerCmd) -> Result<()> {
    let client = super::connect().await?;
    match cmd {
        ServerCmd::Create { args } => create(&client, args).await?,
        ServerCmd::List => list(&client).await?,
        ServerCmd::Config { server, cmd } => config(&client, &server, cmd).await?,
        ServerCmd::Attach { server } => return attach(client, &server).await,
        ServerCmd::Command { server, command } => {
            let reply = client.server().command(&server, &command.join(" ")).await?;
            show_reply(&reply)?;
        }
        ServerCmd::Start { server } => start(&client, &server).await?,
        ServerCmd::Stop { server } => {
            {
                let _spinner = Spinner::start(format!("stopping '{server}'"));
                client.server().stop(&server).await?;
            }
            ui::show(View::line(format!("server '{server}' stopped")))?;
        }
        ServerCmd::Restart { server } => {
            {
                let _spinner = Spinner::start(format!("stopping '{server}'"));
                client.server().stop(&server).await?;
                wait_until_stopped(&client, &server).await?;
            }
            start(&client, &server).await?;
        }
        ServerCmd::Status { server } => {
            let info = client.server().status(&server).await?;
            show_status(&info)?;
        }
        ServerCmd::Logs {
            server,
            tail,
            follow,
        } => {
            let lines = client.server().logs(&server, tail).await?;
            if lines.is_empty() && !follow {
                return ui::show(View::note("no output captured (has it been started?)"));
            }
            for line in lines {
                ui::show(View::line(line.line))?;
            }
            if follow {
                follow_logs(&client, &server).await?;
            }
        }
        ServerCmd::Remove { server } => {
            {
                let _spinner = Spinner::start(format!("removing '{server}'"));
                client.server().remove(&server).await?;
            }
            ui::show(View::line(format!("server '{server}' removed")))?;
        }
        ServerCmd::Versions { flavor, all } => {
            let flavors = {
                let _spinner = Spinner::start("fetching flavors");
                client.server().flavors().await?
            };
            let flavor = mc::pick_flavor(flavors, flavor)?;
            let versions = {
                let _spinner = Spinner::start("fetching versions");
                client.server().versions(&flavor).await?
            };
            mc::show_versions(&flavor, versions, all)?;
        }
        ServerCmd::Flavors => {
            let flavors = {
                let _spinner = Spinner::start("fetching flavors");
                client.server().flavors().await?
            };
            mc::show_flavors(&flavors)?;
        }
    }
    Ok(())
}

async fn create(client: &Client, args: CreateArgs) -> Result<()> {
    let flavors = {
        let _spinner = Spinner::start("fetching flavors");
        client.server().flavors().await?
    };
    let flavor = mc::pick_flavor(flavors, args.flavor)?;
    let versions = {
        let _spinner = Spinner::start("fetching versions");
        client.server().versions(&flavor).await?
    };
    let version = mc::pick_version(versions, args.version)?;
    let name = match args.name {
        Some(name) => name,
        None => ui::input("server name", &format!("{flavor}-{version}"))?,
    };
    if !args.eula {
        confirm_eula()?;
    }
    let config = build_config(
        args.memory,
        [
            ("motd", args.motd),
            ("max-players", args.max_players.map(|n| n.to_string())),
            ("difficulty", args.difficulty),
            ("gamemode", args.gamemode),
            ("level-seed", args.seed),
        ],
        args.prop,
    )?;

    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let params = ServerCreateParams {
        name,
        flavor,
        version,
        loader_version: args.loader,
        eula: true,
        port: args.port,
        config,
        id: String::new(),
    };
    let result = client
        .server()
        .create(params, move |p| progress.update(p))
        .await;
    reporter.finish();
    let server = result?;
    ui::show(View::line(format!("server '{}' created", server.name)))?;
    show_status(&server)
}

/// Fold the create-time settings into one entries list: `--memory`, then the
/// dedicated property flags, then `--prop KEY=VALUE` (split on the first `=`;
/// a missing `=` is an error). Entries apply in order, so a `--prop` naming
/// the same key as a dedicated flag wins.
fn build_config(
    memory: Option<String>,
    flags: impl IntoIterator<Item = (&'static str, Option<String>)>,
    prop: Vec<String>,
) -> Result<Vec<ConfigEntry>> {
    let mut config = Vec::new();
    if let Some(memory) = memory {
        config.push(ConfigEntry {
            key: "memory".into(),
            value: memory,
        });
    }
    for (key, value) in flags {
        if let Some(value) = value {
            config.push(ConfigEntry {
                key: key.into(),
                value,
            });
        }
    }
    for entry in prop {
        let (key, value) = entry
            .split_once('=')
            .with_context(|| format!("--prop '{entry}' must be KEY=VALUE"))?;
        config.push(ConfigEntry {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(config)
}

/// `hestia server config <server> get|set|list` — the per-server settings
/// surface. Changes apply on the next start.
async fn config(client: &Client, server: &str, cmd: mc::ConfigCmd) -> Result<()> {
    match cmd {
        mc::ConfigCmd::Get { key } => match client.server().config_get(server, &key).await? {
            Some(value) => ui::show(View::line(value))?,
            None => bail!("'{key}' is not set"),
        },
        mc::ConfigCmd::Set { key, value } => {
            client.server().config_set(server, &key, &value).await?;
            ui::show(View::note("applies from the next start"))?;
        }
        mc::ConfigCmd::List => {
            let entries = client.server().config_list(server).await?;
            mc::show_config_entries(format!("{server} config"), entries)?;
        }
    }
    Ok(())
}

/// Attach an interactive console to a running server: its live output above
/// an input line; Esc detaches without touching the server.
async fn attach(client: Client, server: &str) -> Result<()> {
    if !ui::is_interactive() {
        bail!("attach needs an interactive terminal (use `server logs -f` and `server command`)");
    }
    let info = client.server().status(server).await?;
    let process =
        running_process(&info).with_context(|| format!("server '{}' is not running", info.name))?;
    let backfill = client
        .server()
        .logs(&info.id, Some(100))
        .await?
        .into_iter()
        .map(|l| l.line)
        .collect();
    let mut process_events = client.process().subscribe(&process.id).await?;

    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let forward_tx = event_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = process_events.recv().await {
            let sent = match event {
                ProcessEvent::Output(line) => forward_tx.send(ui::ConsoleEvent::Output(line.line)),
                ProcessEvent::Exit(_) => {
                    let _ = forward_tx.send(ui::ConsoleEvent::Closed("server stopped".into()));
                    break;
                }
            };
            if sent.is_err() {
                break;
            }
        }
    });

    // The command task owns the client: the session (and with it the
    // subscription) lives exactly as long as the console runs.
    let server_id = info.id.clone();
    tokio::spawn(async move {
        while let Some(command) = command_rx.recv().await {
            let event = match client.server().command(&server_id, &command).await {
                Ok(reply) => ui::ConsoleEvent::Reply(strip_codes(&reply)),
                Err(e) => ui::ConsoleEvent::Notice(format!("{e:#}")),
            };
            if event_tx.send(event).is_err() {
                break;
            }
        }
    });

    let title = format!(" {} ", info.name);
    let closed =
        tokio::task::spawn_blocking(move || ui::console(&title, backfill, event_rx, command_tx))
            .await??;
    match closed {
        Some(message) => ui::show(View::note(message)),
        None => ui::show(View::note(format!("detached ('{server}' keeps running)"))),
    }
}

async fn follow_logs(client: &Client, server: &str) -> Result<()> {
    let info = client.server().status(server).await?;
    let process =
        running_process(&info).with_context(|| format!("server '{}' is not running", info.name))?;
    let mut events = client.process().subscribe(&process.id).await?;
    while let Some(event) = events.recv().await {
        match event {
            ProcessEvent::Output(line) => ui::show(View::line(line.line))?,
            ProcessEvent::Exit(_) => {
                return ui::show(View::note("server stopped"));
            }
        }
    }
    Ok(())
}

fn running_process(info: &ServerInfo) -> Option<ProcessInfo> {
    info.process
        .clone()
        .filter(|p| p.state == ProcessState::Running)
}

fn show_reply(reply: &str) -> Result<()> {
    let reply = strip_codes(reply);
    if reply.trim().is_empty() {
        return ui::show(View::note("(no reply)"));
    }
    for line in reply.lines() {
        ui::show(View::line(line))?;
    }
    Ok(())
}

/// Drop Minecraft's `§x` color codes — RCON replies carry them verbatim.
fn strip_codes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// Interactive fallback for a missing `--eula`; errors when stdin is not a
/// terminal so scripts must pass the flag explicitly.
fn confirm_eula() -> Result<()> {
    let choice = ui::select(
        &format!("running a Minecraft server requires accepting the EULA ({EULA_URL})"),
        &["accept".to_string(), "decline".to_string()],
    )
    .context("pass --eula to accept the Minecraft EULA")?;
    if choice != 0 {
        bail!("the Minecraft EULA was not accepted");
    }
    Ok(())
}

async fn start(client: &Client, server: &str) -> Result<()> {
    let started = {
        let _spinner = Spinner::start(format!("starting '{server}'"));
        client.server().start(server).await?
    };
    ui::show(View::line(format!(
        "server '{server}' started (pid {})",
        started.pid
    )))
}

/// Poll until the server's process has exited, so a restart's `start` does not
/// race the old child.
async fn wait_until_stopped(client: &Client, server: &str) -> Result<()> {
    for _ in 0..30 {
        let info = client.server().status(server).await?;
        let running = info
            .process
            .is_some_and(|p| p.state == ProcessState::Running);
        if !running {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("server '{server}' did not stop in time");
}

async fn list(client: &Client) -> Result<()> {
    let servers = client.server().list().await?;
    if servers.is_empty() {
        return ui::show(View::note("no servers yet (hestia server create)"));
    }
    let rows = servers
        .iter()
        .map(|s| {
            vec![
                s.name.clone(),
                s.flavor.clone(),
                s.game_version.clone(),
                s.loader_version.clone().unwrap_or_else(|| "-".into()),
                address_label(s),
                state_label(s),
            ]
        })
        .collect();
    ui::show(View::table(
        "servers",
        ["NAME", "FLAVOR", "VERSION", "LOADER", "ADDRESS", "STATE"],
        rows,
    ))
}

fn show_status(info: &ServerInfo) -> Result<()> {
    ui::show(View::detail([
        ("name", info.name.clone()),
        ("id", info.id.clone()),
        ("flavor", info.flavor.clone()),
        ("version", info.game_version.clone()),
        (
            "loader",
            info.loader_version.clone().unwrap_or_else(|| "-".into()),
        ),
        ("java", info.java_major.to_string()),
        ("address", address_label(info)),
        (
            "console",
            if info.console { "yes" } else { "on next start" }.into(),
        ),
        ("state", state_label(info)),
    ]))
}

fn address_label(info: &ServerInfo) -> String {
    match info.game_port {
        Some(port) => format!("localhost:{port}"),
        None => "-".into(),
    }
}

fn state_label(info: &ServerInfo) -> String {
    if !info.ready {
        return "provisioning".into();
    }
    mc::process_state_label(&info.process)
}
