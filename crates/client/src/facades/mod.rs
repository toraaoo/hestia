//! One typed facade per domain, reached through `Client` accessors. Facade
//! methods are thin wrappers over `Session::call`, returning `proto` types
//! directly — mirroring the engine's domain modules on the other side of the
//! socket.

mod accounts;
mod app;
mod cache;
mod config;
mod content;
mod daemon;
mod instance;
mod java;
mod jobs;
mod process;
mod profiles;
mod server;
mod skins;
mod sync;

pub use accounts::Accounts;
pub use app::App;
pub use cache::Cache;
pub use config::Config;
pub use content::Content;
pub use daemon::Daemon;
pub use instance::Instance;
pub use java::Java;
pub use process::{Process, ProcessEvent};
pub use profiles::Profiles;
pub use server::Server;
pub use skins::Skins;
pub use sync::Sync;
