//! Wire contracts + domain types shared by both sides of the socket. Pure data:
//! no I/O, no async. `serde` derive is the marshalling layer.

pub mod accounts;
pub mod app;
pub mod backup;
pub mod cache;
pub mod config;
pub mod content;
pub mod contract;
pub mod daemon;
pub mod download;
pub mod events;
pub mod health;
pub mod instance;
pub mod java;
pub mod minecraft;
pub mod process;
pub mod server;
pub mod sync;

pub use contract::{Contract, Empty, Topic};
