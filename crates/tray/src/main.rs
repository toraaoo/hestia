//! tray — the Hestia system-tray helper.
//!
//! Spawned by the daemon whenever it serves, but with its own lifetime: when
//! the daemon stops the tray stays, showing a stopped state with a start
//! action. One tray runs per user session — a duplicate exits at startup, so
//! the daemon can spawn unconditionally on every start.

mod desktop;
mod icon;
mod lock;
mod menu;
mod worker;

use std::process::ExitCode;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::menu::TrayMenu;
use crate::worker::DaemonState;

pub enum UserEvent {
    State(DaemonState),
    Menu(MenuEvent),
    OpenDesktop,
    Exit,
}

fn main() -> ExitCode {
    let level = common::LogLevel::default();
    let file = common::FileLog::appending(common::paths::log_dir(None), "tray", level);
    let _guard = common::init_logging(level, Some(file));

    let Some(_lock) = lock::acquire() else {
        tracing::info!("another tray is already running; exiting");
        return ExitCode::SUCCESS;
    };

    let icon = match icon::load() {
        Ok(icon) => icon,
        Err(e) => {
            tracing::error!("cannot load the tray icon: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(version = common::app::VERSION, "tray starting");

    let event_loop = build_event_loop();

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

    let menu = TrayMenu::new();
    let actions = worker::spawn(event_loop.create_proxy());

    let mut tray: Option<TrayIcon> = None;
    let mut state = DaemonState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // Platform constraint: the tray icon must be created once the event
            // loop is actually running (gtk on Linux, the run loop on macOS).
            Event::NewEvents(StartCause::Init) => match build_tray(&menu, icon.clone()) {
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
                menu.apply(&new);
                state = new;
            }
            Event::UserEvent(UserEvent::Menu(event)) => {
                if let Some(action) = menu.action_for(&event, &state) {
                    let _ = actions.send(action);
                }
            }
            Event::UserEvent(UserEvent::OpenDesktop) => desktop::launch(),
            Event::UserEvent(UserEvent::Exit) => {
                // Drop the icon before exiting so it never lingers in the tray.
                tray.take();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    })
}

fn build_tray(menu: &TrayMenu, icon: tray_icon::Icon) -> tray_icon::Result<TrayIcon> {
    // Left-click opens the desktop app (handled via TrayIconEvent), so the menu
    // stays on right-click only.
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu.menu().clone()))
        .with_menu_on_left_click(false)
        .with_title(common::app::NAME)
        .with_tooltip(common::app::NAME)
        .build()
}

#[cfg(target_os = "linux")]
fn build_event_loop() -> tao::event_loop::EventLoop<UserEvent> {
    use tao::platform::unix::EventLoopBuilderExtUnix;

    glib::set_application_name(common::app::NAME);

    let mut builder = EventLoopBuilder::<UserEvent>::with_user_event();
    // Distinct from the desktop shell's id so the two GApplications don't
    // collide on the D-Bus name (see common::app::TRAY_ID).
    builder.with_app_id(common::app::TRAY_ID);
    builder.build()
}

#[cfg(not(target_os = "linux"))]
fn build_event_loop() -> tao::event_loop::EventLoop<UserEvent> {
    EventLoopBuilder::<UserEvent>::with_user_event().build()
}
