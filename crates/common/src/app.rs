//! Shared application identity — the single source of truth every front-end and
//! the daemon read, mirroring the generated `app_info.h` in the C++ tree.

pub const NAME: &str = "Hestia";
pub const ID: &str = "tech.lawrenceallen.hestia";
pub const VENDOR: &str = "toraaoo";
pub const CHANNEL: &str = "dev";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
