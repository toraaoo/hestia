//! The daemon-facing side of the tray: a background thread that polls the
//! daemon over the client SDK, executes menu actions, and reports state
//! changes to the event loop through its proxy.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use client::Client;
use tao::event_loop::EventLoopProxy;
use tokio::runtime::Runtime;

use crate::UserEvent;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const RESTART_GRACE: Duration = Duration::from_millis(500);
const AUTOSTART_KEY: &str = "autostart";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DaemonState {
    pub running: bool,
    pub version: Option<String>,
    pub autostart: bool,
}

pub enum Action {
    Start,
    Restart,
    SetAutostart(bool),
    Quit,
}

pub fn spawn(proxy: EventLoopProxy<UserEvent>) -> Sender<Action> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || run(rx, proxy));
    tx
}

fn run(rx: Receiver<Action>, proxy: EventLoopProxy<UserEvent>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let mut client: Option<Client> = None;
    let mut last: Option<DaemonState> = None;

    push_state(&rt, &mut client, &mut last, &proxy);
    loop {
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(Action::Quit) => {
                if let Some(c) = ensure_client(&rt, &mut client) {
                    tracing::info!("quit: stopping the daemon");
                    if let Err(e) = rt.block_on(c.daemon().stop(false)) {
                        tracing::warn!("daemon stop failed: {e}");
                    }
                }
                let _ = proxy.send_event(UserEvent::Exit);
                return;
            }
            Ok(action) => {
                perform(&rt, &mut client, action);
                push_state(&rt, &mut client, &mut last, &proxy);
            }
            Err(RecvTimeoutError::Timeout) => {
                push_state(&rt, &mut client, &mut last, &proxy);
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn perform(rt: &Runtime, client: &mut Option<Client>, action: Action) {
    match action {
        Action::Start => {
            tracing::info!("starting the daemon");
            connect_spawning(rt, client);
        }
        Action::Restart => {
            tracing::info!("restarting the daemon");
            if let Some(c) = client.take() {
                let _ = rt.block_on(c.daemon().stop(false));
            }
            // Give the old daemon a moment to release the endpoint.
            std::thread::sleep(RESTART_GRACE);
            connect_spawning(rt, client);
        }
        Action::SetAutostart(enabled) => {
            if let Some(c) = ensure_client(rt, client) {
                if let Err(e) =
                    rt.block_on(c.config().set(AUTOSTART_KEY, serde_json::json!(enabled)))
                {
                    tracing::warn!(enabled, "cannot set autostart: {e}");
                }
            }
        }
        Action::Quit => unreachable!("quit is handled by the worker loop"),
    }
}

fn connect_spawning(rt: &Runtime, client: &mut Option<Client>) {
    match rt.block_on(Client::connect(true)) {
        Ok(c) => *client = Some(c),
        Err(e) => tracing::warn!("cannot start the daemon: {e}"),
    }
}

fn ensure_client<'a>(rt: &Runtime, client: &'a mut Option<Client>) -> Option<&'a Client> {
    if client.is_none() {
        *client = rt.block_on(Client::connect(false)).ok();
    }
    client.as_ref()
}

fn poll(rt: &Runtime, client: &mut Option<Client>) -> DaemonState {
    if client.is_none() {
        *client = rt.block_on(Client::connect(false)).ok();
    }
    let (status, autostart) = match client.as_ref() {
        None => return DaemonState::default(),
        Some(c) => (
            rt.block_on(c.daemon().status()),
            rt.block_on(c.config().get(AUTOSTART_KEY)),
        ),
    };
    match status {
        Ok(s) => DaemonState {
            running: true,
            version: Some(s.version),
            autostart: autostart
                .ok()
                .flatten()
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        Err(_) => {
            *client = None;
            DaemonState::default()
        }
    }
}

fn push_state(
    rt: &Runtime,
    client: &mut Option<Client>,
    last: &mut Option<DaemonState>,
    proxy: &EventLoopProxy<UserEvent>,
) {
    let state = poll(rt, client);
    if last.as_ref() != Some(&state) {
        *last = Some(state.clone());
        let _ = proxy.send_event(UserEvent::State(state));
    }
}
