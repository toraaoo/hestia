//! Windows: the notification area through Tauri's native tray crates.
//!
//! Shell_NotifyIcon needs a window with a message pump, so the icon lives on a
//! `tao` event loop and the menu is built once from persistent widgets that
//! state changes mutate in place.

use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::worker::{self, Action, DaemonState, Sink};
use crate::{icon, menu};

enum UserEvent {
    State(DaemonState),
    Menu(MenuEvent),
    OpenDesktop,
    Exit,
}

pub fn run(raster: icon::Raster) -> ExitCode {
    let icon = match tray_icon::Icon::from_rgba(raster.rgba, raster.width, raster.height) {
        Ok(icon) => icon,
        Err(e) => {
            tracing::error!("cannot build the tray icon: {e}");
            return ExitCode::FAILURE;
        }
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let menu_proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UserEvent::Menu(event));
    }));

    let click_proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        // Left-click (on release) opens the desktop app; right-click still
        // shows the menu, so only a completed left-click is forwarded.
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let _ = click_proxy.send_event(UserEvent::OpenDesktop);
        }
    }));

    let widgets = Widgets::new();
    let (actions, requests) = mpsc::channel();
    worker::spawn(requests, event_loop.create_proxy());

    drive(event_loop, widgets, icon, actions)
}

fn drive(
    event_loop: EventLoop<UserEvent>,
    widgets: Widgets,
    icon: tray_icon::Icon,
    actions: Sender<Action>,
) -> ExitCode {
    let mut tray: Option<TrayIcon> = None;
    let mut state = DaemonState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // Platform constraint: the icon must be created once the event loop
            // is actually running, so it has a window to hang its messages on.
            Event::NewEvents(StartCause::Init) => match build_tray(&widgets, icon.clone()) {
                Ok(t) => tray = Some(t),
                Err(e) => {
                    tracing::error!("cannot create the tray icon: {e}");
                    *control_flow = ControlFlow::ExitWithCode(1);
                }
            },
            Event::UserEvent(UserEvent::State(new)) => {
                if let Some(tray) = &tray {
                    let _ = tray.set_tooltip(Some(menu::tooltip(&new)));
                }
                widgets.apply(&new);
                state = new;
            }
            Event::UserEvent(UserEvent::Menu(event)) => {
                if let Some(action) = widgets.action_for(&event, &state) {
                    let _ = actions.send(action);
                }
            }
            Event::UserEvent(UserEvent::OpenDesktop) => crate::desktop::launch(),
            Event::UserEvent(UserEvent::Exit) => {
                // Drop the icon before exiting so it never lingers in the tray.
                tray.take();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    })
}

impl Sink for EventLoopProxy<UserEvent> {
    fn state(&self, state: DaemonState) {
        let _ = self.send_event(UserEvent::State(state));
    }

    fn exit(&self) {
        let _ = self.send_event(UserEvent::Exit);
    }
}

fn build_tray(widgets: &Widgets, icon: tray_icon::Icon) -> tray_icon::Result<TrayIcon> {
    // Left-click opens the desktop app (handled via TrayIconEvent), so the menu
    // stays on right-click only.
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(widgets.menu.clone()))
        .with_menu_on_left_click(false)
        .with_title(common::app::NAME)
        .with_tooltip(common::app::NAME)
        .build()
}

struct Widgets {
    menu: Menu,
    open: MenuItem,
    status: MenuItem,
    daemon: MenuItem,
    autostart: CheckMenuItem,
    quit: MenuItem,
}

impl Widgets {
    fn new() -> Self {
        let initial = DaemonState::default();
        let open = MenuItem::new(menu::open_label(), true, None);
        let status = MenuItem::new(menu::status_text(&initial), false, None);
        let daemon = MenuItem::new(menu::daemon_label(&initial), true, None);
        let autostart = CheckMenuItem::new(menu::AUTOSTART_LABEL, false, false, None);
        let quit = MenuItem::new(menu::quit_label(), true, None);

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

        Widgets {
            menu,
            open,
            status,
            daemon,
            autostart,
            quit,
        }
    }

    fn apply(&self, state: &DaemonState) {
        self.status.set_text(menu::status_text(state));
        self.daemon.set_text(menu::daemon_label(state));
        self.autostart.set_enabled(menu::autostart_enabled(state));
        self.autostart.set_checked(state.autostart);
    }

    fn action_for(&self, event: &MenuEvent, state: &DaemonState) -> Option<Action> {
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
