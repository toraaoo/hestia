//! The server console over RCON: `attach` (interactive) and the one-shot
//! `command`'s reply rendering.

use anyhow::{bail, Context, Result};
use client::{Client, ProcessEvent};

use super::entry;
use crate::ui::{self, View};

/// Attach an interactive console to a running server: its live output above
/// an input line; Esc detaches without touching the server.
pub(super) async fn attach(client: Client, server: &str) -> Result<()> {
    if !ui::is_interactive() {
        bail!("attach needs an interactive terminal (use `server logs -f` and `server command`)");
    }
    let info = client.server().status(server).await?;
    let process = entry::running_process(&info)
        .with_context(|| format!("server '{}' is not running", info.name))?;
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

pub(super) fn show_reply(reply: &str) -> Result<()> {
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
