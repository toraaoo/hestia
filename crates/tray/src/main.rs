//! tray — the Hestia system-tray helper.
//!
//! Spawned by the daemon whenever it serves, but with its own lifetime: when
//! the daemon stops the tray stays, showing a stopped state with a start
//! action. One tray runs per user session — a duplicate exits at startup, so
//! the daemon can spawn unconditionally on every start.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod desktop;
mod icon;
mod lock;
mod menu;
mod platform;
mod worker;

use std::process::ExitCode;

fn main() -> ExitCode {
    let level = common::LogLevel::default();
    let file = common::FileLog::for_binary("tray", None, level);
    let _guard = common::init_logging(level, Some(file));

    let Some(_lock) = lock::acquire() else {
        tracing::info!("another tray is already running; exiting");
        return ExitCode::SUCCESS;
    };

    let raster = match icon::load() {
        Ok(raster) => raster,
        Err(e) => {
            tracing::error!("cannot load the tray icon: {e:#}");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(version = common::app::VERSION, "tray starting");
    platform::run(raster)
}
