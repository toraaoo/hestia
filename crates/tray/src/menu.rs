//! The tray menu: a status header, the daemon quick actions, and quit.

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};

use crate::worker::{Action, DaemonState};

pub struct TrayMenu {
    menu: Menu,
    open: MenuItem,
    status: MenuItem,
    daemon: MenuItem,
    autostart: CheckMenuItem,
    quit: MenuItem,
}

impl TrayMenu {
    pub fn new() -> Self {
        let initial = DaemonState::default();
        let open = MenuItem::new(format!("Open {}", common::app::NAME), true, None);
        let status = MenuItem::new(status_text(&initial), false, None);
        let daemon = MenuItem::new("Start daemon", false, None);
        let autostart = CheckMenuItem::new("Start at login", false, false, None);
        let quit = MenuItem::new(format!("Quit {}", common::app::NAME), true, None);

        let menu = Menu::new();
        let _ = menu.append_items(&[
            &open,
            &PredefinedMenuItem::separator(),
            &status,
            &PredefinedMenuItem::separator(),
            &daemon,
            &autostart,
            &PredefinedMenuItem::separator(),
            &quit,
        ]);

        TrayMenu {
            menu,
            open,
            status,
            daemon,
            autostart,
            quit,
        }
    }

    pub fn menu(&self) -> &Menu {
        &self.menu
    }

    pub fn apply(&self, state: &DaemonState) {
        self.status.set_text(status_text(state));
        self.daemon.set_text(if state.running {
            "Restart daemon"
        } else {
            "Start daemon"
        });
        self.daemon.set_enabled(true);
        self.autostart.set_enabled(state.running);
        self.autostart.set_checked(state.autostart);
    }

    pub fn action_for(&self, event: &MenuEvent, state: &DaemonState) -> Option<Action> {
        let id = event.id();
        if id == self.open.id() {
            Some(Action::OpenDesktop)
        } else if id == self.daemon.id() {
            Some(if state.running {
                Action::Restart
            } else {
                Action::Start
            })
        } else if id == self.autostart.id() {
            Some(Action::SetAutostart(self.autostart.is_checked()))
        } else if id == self.quit.id() {
            Some(Action::Quit)
        } else {
            None
        }
    }
}

fn status_text(state: &DaemonState) -> String {
    match &state.version {
        Some(version) => format!("{} {version} — running", common::app::NAME),
        None => format!("{} — stopped", common::app::NAME),
    }
}

pub fn tooltip(state: &DaemonState) -> String {
    let condition = if state.running { "running" } else { "stopped" };
    format!("{} — {condition}", common::app::NAME)
}
