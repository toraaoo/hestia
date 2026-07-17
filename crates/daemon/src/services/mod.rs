//! Every channel the daemon serves, wired onto the router. One `handle::<C>` per
//! channel, grouped into a registrar per domain; handlers reach the daemon's
//! collaborators through `HandlerContext` and return `ServiceError` for a typed
//! failure.

mod accounts;
mod backup;
mod cache;
mod config;
mod content;
mod download;
mod guards;
mod instance;
mod java;
mod lifecycle;
mod process;
mod profile;
mod server;
mod skins;
mod sync;

use crate::runtime::{Channels, Router};

pub fn make_router() -> Router {
    let mut router = Router::default();
    let mut on = Channels::new(&mut router);

    lifecycle::register(&mut on);
    config::register(&mut on);
    cache::register(&mut on);
    java::register(&mut on);
    download::register(&mut on);
    accounts::register(&mut on);
    skins::register(&mut on);
    process::register(&mut on);
    server::register(&mut on);
    instance::register(&mut on);
    backup::register(&mut on);
    content::register(&mut on);
    profile::register(&mut on);
    sync::register(&mut on);

    router
}
