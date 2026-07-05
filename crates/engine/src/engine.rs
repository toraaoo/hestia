//! The daemon-internal aggregate root; front-ends reach it only over IPC. Adding
//! a domain = a module, a member, and a getter here.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::accounts::Accounts;
use crate::cache::Cache;
use crate::config::Config;
use crate::java::Java;
use crate::minecraft::Minecraft;

pub struct Engine {
    data_home: Mutex<PathBuf>,
    config: Config,
    cache: Cache,
    java: Java,
    accounts: Accounts,
    minecraft: Minecraft,
}

impl Engine {
    pub fn new(override_home: Option<&Path>) -> Self {
        let data_home = common::paths::data_home(override_home);
        tracing::info!(home = %data_home.display(), "engine data home");
        let config = Config::new(common::paths::config_path(Some(&data_home)));
        let cache = Cache::new(data_home.join("cache"));
        let java = Java::new(data_home.join("java"));
        let accounts = Accounts::new(data_home.join("accounts.json"));
        Engine {
            data_home: Mutex::new(data_home),
            config,
            cache,
            java,
            accounts,
            minecraft: Minecraft::new(),
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
}
