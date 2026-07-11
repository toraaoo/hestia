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
    let mut worker = Worker {
        rt,
        client: None,
        autostart: false,
    };
    let mut last: Option<DaemonState> = None;

    worker.push_state(&mut last, &proxy);
    loop {
        match rx.recv_timeout(POLL_INTERVAL) {
            Ok(Action::Quit) => {
                worker.stop_daemon();
                let _ = proxy.send_event(UserEvent::Exit);
                return;
            }
            Ok(action) => {
                worker.perform(action);
                worker.push_state(&mut last, &proxy);
            }
            Err(RecvTimeoutError::Timeout) => {
                worker.push_state(&mut last, &proxy);
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

struct Worker {
    rt: Runtime,
    client: Option<Client>,
    // Reading autostart runs a subprocess on some platforms (schtasks on
    // Windows), so it is fetched per connection and after a toggle — never on
    // the poll tick.
    autostart: bool,
}

impl Worker {
    fn perform(&mut self, action: Action) {
        match action {
            Action::Start => {
                tracing::info!("starting the daemon");
                self.connect_spawning();
            }
            Action::Restart => {
                tracing::info!("restarting the daemon");
                if let Some(c) = self.client.take() {
                    let _ = self.rt.block_on(c.daemon().stop(false));
                }
                // Give the old daemon a moment to release the endpoint.
                std::thread::sleep(RESTART_GRACE);
                self.connect_spawning();
            }
            Action::SetAutostart(enabled) => {
                if self.connect_if_needed() {
                    let c = self.client.as_ref().expect("connected");
                    if let Err(e) = self
                        .rt
                        .block_on(c.config().set(AUTOSTART_KEY, serde_json::json!(enabled)))
                    {
                        tracing::warn!(enabled, "cannot set autostart: {e}");
                    }
                    self.autostart = fetch_autostart(&self.rt, c);
                }
            }
            Action::Quit => unreachable!("quit is handled by the worker loop"),
        }
    }

    fn stop_daemon(&mut self) {
        if !self.connect_if_needed() {
            return;
        }
        let c = self.client.as_ref().expect("connected");
        tracing::info!("quit: stopping the daemon");
        if let Err(e) = self.rt.block_on(c.daemon().stop(false)) {
            tracing::warn!("daemon stop failed: {e}");
        }
    }

    fn connect_spawning(&mut self) {
        match self.rt.block_on(Client::connect(true)) {
            Ok(c) => {
                self.autostart = fetch_autostart(&self.rt, &c);
                self.client = Some(c);
            }
            Err(e) => tracing::warn!("cannot start the daemon: {e}"),
        }
    }

    fn connect_if_needed(&mut self) -> bool {
        if self.client.is_none() {
            if let Ok(c) = self.rt.block_on(Client::connect(false)) {
                self.autostart = fetch_autostart(&self.rt, &c);
                self.client = Some(c);
            }
        }
        self.client.is_some()
    }

    fn poll(&mut self) -> DaemonState {
        if !self.connect_if_needed() {
            return DaemonState::default();
        }
        let status = match self.client.as_ref() {
            None => return DaemonState::default(),
            Some(c) => self.rt.block_on(c.daemon().status()),
        };
        match status {
            Ok(s) => DaemonState {
                running: true,
                version: Some(s.version),
                autostart: self.autostart,
            },
            Err(_) => {
                self.client = None;
                DaemonState::default()
            }
        }
    }

    fn push_state(&mut self, last: &mut Option<DaemonState>, proxy: &EventLoopProxy<UserEvent>) {
        let state = self.poll();
        if last.as_ref() != Some(&state) {
            *last = Some(state.clone());
            let _ = proxy.send_event(UserEvent::State(state));
        }
    }
}

fn fetch_autostart(rt: &Runtime, client: &Client) -> bool {
    rt.block_on(client.config().get(AUTOSTART_KEY))
        .ok()
        .flatten()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}
