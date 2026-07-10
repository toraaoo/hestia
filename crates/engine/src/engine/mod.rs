//! The daemon-internal aggregate root; front-ends reach it only over IPC. Adding
//! a domain = a module, a member, and a getter here.
//!
//! The aggregate owns the subsystems and nothing else. The cross-subsystem flows
//! composed over them — provisioning, launching, backups, content — live in
//! `flows`, one module apiece, each an `impl Engine` block.

mod flows;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use proto::minecraft::ConfigEntry;

use crate::accounts::Accounts;
use crate::cache::Cache;
use crate::config::Config;
use crate::content::Content;
use crate::instances::Instances;
use crate::java::Java;
use crate::minecraft::Minecraft;
use crate::servers::Servers;

/// Everything a server create needs from the caller — the engine-side input to
/// `provision_server` (EULA assertion and job ids are daemon concerns).
#[derive(Debug, Clone, Default)]
pub struct ServerCreateSpec {
    pub name: String,
    pub flavor: String,
    pub version: String,
    pub loader_version: Option<String>,
    pub port: Option<u16>,
    pub config: Vec<ConfigEntry>,
}

/// Everything a server update needs from the caller — the engine-side input to
/// `update_server` (the downgrade confirmation is obtained by the front-end).
#[derive(Debug, Clone, Default)]
pub struct ServerUpdateSpec {
    pub server: String,
    pub version: String,
    pub loader_version: Option<String>,
    pub allow_downgrade: bool,
}

pub struct Engine {
    data_home: Mutex<PathBuf>,
    config: Config,
    cache: Cache,
    java: Java,
    accounts: Accounts,
    minecraft: Minecraft,
    content: Content,
    servers: Servers,
    instances: Instances,
    // One backup or restore per entry at a time: two archives of the same
    // data would interleave the rcon save-off/save-on dance.
    backups_active: Mutex<HashSet<String>>,
}

impl Engine {
    pub fn new(override_home: Option<&Path>) -> Self {
        let data_home = common::paths::data_home(override_home);
        tracing::info!(home = %data_home.display(), "engine data home");
        let config = Config::new(common::paths::config_path(Some(&data_home)));
        let cache = Cache::new(data_home.join("cache"));
        let java = Java::new(data_home.join("java"));
        let accounts = Accounts::new(data_home.join("accounts.json"));
        let servers = Servers::new(data_home.join("servers"));
        let instances = Instances::new(data_home.join("instances"));
        Engine {
            data_home: Mutex::new(data_home),
            config,
            cache,
            java,
            accounts,
            minecraft: Minecraft::new(),
            content: Content::new(),
            servers,
            instances,
            backups_active: Mutex::new(HashSet::new()),
        }
    }

    pub fn data_home(&self) -> PathBuf {
        self.data_home.lock().unwrap().clone()
    }

    /// Persist `dir` (empty reverts to the default), re-resolve, and repoint every
    /// subsystem on the running daemon.
    pub fn set_data_home(&self, dir: &str) -> std::io::Result<PathBuf> {
        common::paths::set_persisted_home(Path::new(dir))?;
        let resolved = common::paths::data_home(None);
        self.config
            .reload(common::paths::config_path(Some(&resolved)));
        self.cache.reload(resolved.join("cache"));
        self.java.reload(resolved.join("java"));
        self.accounts.reload(resolved.join("accounts.json"));
        self.servers.reload(resolved.join("servers"));
        self.instances.reload(resolved.join("instances"));
        *self.data_home.lock().unwrap() = resolved.clone();
        tracing::info!(home = %resolved.display(), "engine data home changed");
        Ok(resolved)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    pub fn java(&self) -> &Java {
        &self.java
    }

    pub fn accounts(&self) -> &Accounts {
        &self.accounts
    }

    pub fn minecraft(&self) -> &Minecraft {
        &self.minecraft
    }

    pub fn content(&self) -> &Content {
        &self.content
    }

    pub fn servers(&self) -> &Servers {
        &self.servers
    }

    pub fn instances(&self) -> &Instances {
        &self.instances
    }
}
