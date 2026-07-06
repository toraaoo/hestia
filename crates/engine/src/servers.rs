//! Persistent Minecraft server store: each server lives at `<dir>/<id>/`
//! (its jar, `eula.txt`, and the world the game writes) beside a `server.json`
//! record; listing scans the directory — the disk is the registry.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::minecraft::{ProvisionPhase, ServerProfile};
use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::minecraft::launch::{self, LaunchPlan};
use crate::minecraft::materialize::{self, OnProgress};
use crate::registry;

const RECORD: &str = "server.json";
const PROPERTIES: &str = "server.properties";
const GAME_PORT_BASE: u16 = 25565;
const RCON_PORT_BASE: u16 = 25575;
const PORT_SPAN: u16 = 100;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RconConfig {
    pub port: u16,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub created_unix: i64,
    /// False until the create job has finished provisioning files.
    #[serde(default)]
    pub ready: bool,
    /// Claimed at create and never moved — players connect to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_port: Option<u16>,
    /// Claimed at first start; internal, so it may be reallocated freely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rcon: Option<RconConfig>,
    pub profile: ServerProfile,
}

pub struct Servers {
    dir: Mutex<PathBuf>,
    // Serializes scan-pick-persist port claims so two concurrent creates or
    // starts cannot claim the same port.
    claims: Mutex<()>,
}

impl Servers {
    pub fn new(dir: PathBuf) -> Self {
        Servers {
            dir: Mutex::new(dir),
            claims: Mutex::new(()),
        }
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.dir.lock().unwrap() = dir;
    }

    pub fn dir(&self) -> PathBuf {
        self.dir.lock().unwrap().clone()
    }

    pub fn server_dir(&self, id: &str) -> PathBuf {
        self.dir().join(id)
    }

    pub fn list(&self) -> Vec<ServerRecord> {
        let mut records: Vec<ServerRecord> = registry::scan(&self.dir(), RECORD);
        records.sort_by(|a, b| a.name.cmp(&b.name));
        records
    }

    /// Find one server by id or name.
    pub fn get(&self, reference: &str) -> Option<ServerRecord> {
        self.list()
            .into_iter()
            .find(|r| r.id == reference || r.name == reference)
    }

    /// Register a new server: allocate its id from the name, claim its game
    /// port, create its directory, and write the (not yet ready) record.
    pub fn create(
        &self,
        name: &str,
        profile: ServerProfile,
        port: Option<u16>,
    ) -> Result<ServerRecord> {
        let id = registry::slugify(name)?;
        if self.get(&id).is_some() || self.get(name).is_some() {
            bail!("a server named '{name}' already exists");
        }
        let _claims = self.claims.lock().unwrap();
        let game_port = self.claim_game_port(port)?;
        let record = ServerRecord {
            id: id.clone(),
            name: name.to_string(),
            created_unix: registry::now_unix(),
            ready: false,
            game_port: Some(game_port),
            rcon: None,
            profile,
        };
        let dir = self.server_dir(&id);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create {}", dir.display()))?;
        registry::write_record(&dir, RECORD, &record)?;
        tracing::info!(id, name, game_port, "server registered");
        Ok(record)
    }

    fn claim_game_port(&self, requested: Option<u16>) -> Result<u16> {
        let claimed = self.claimed_ports(None);
        match requested {
            Some(port) => {
                if claimed.contains(&port) {
                    bail!("port {port} is already claimed by another server");
                }
                if !can_bind(port) {
                    bail!("port {port} is already in use");
                }
                Ok(port)
            }
            None => allocate_port(GAME_PORT_BASE, &claimed),
        }
    }

    /// Every port any server's record claims (game and rcon), except
    /// `exclude`'s own.
    fn claimed_ports(&self, exclude: Option<&str>) -> HashSet<u16> {
        self.list()
            .iter()
            .filter(|r| Some(r.id.as_str()) != exclude)
            .flat_map(|r| {
                r.game_port
                    .into_iter()
                    .chain(r.rcon.as_ref().map(|c| c.port))
            })
            .collect()
    }

    /// Reconcile `server.properties` with the record's claimed ports before a
    /// spawn. The game port never moves (players depend on it) — an outside
    /// squatter is an error. The rcon port is internal and reallocates freely;
    /// a record from before ports existed gains both here.
    pub fn ensure_start_config(&self, id: &str) -> Result<ServerRecord> {
        let _claims = self.claims.lock().unwrap();
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        let claimed = self.claimed_ports(Some(&record.id));

        let game_port = match record.game_port {
            Some(port) => port,
            None => allocate_port(GAME_PORT_BASE, &claimed)?,
        };
        if claimed.contains(&game_port) || !can_bind(game_port) {
            bail!("game port {game_port} is in use by another process");
        }

        let rcon = match record.rcon.take() {
            Some(cfg) if !claimed.contains(&cfg.port) && can_bind(cfg.port) => cfg,
            prior => RconConfig {
                port: allocate_port(RCON_PORT_BASE, &claimed)?,
                password: prior.map(|c| c.password).unwrap_or_else(generate_password),
            },
        };

        merge_properties(
            &self.server_dir(&record.id).join(PROPERTIES),
            &[
                ("server-port", game_port.to_string()),
                ("enable-rcon", "true".to_string()),
                ("rcon.port", rcon.port.to_string()),
                ("rcon.password", rcon.password.clone()),
            ],
        )?;
        record.game_port = Some(game_port);
        record.rcon = Some(rcon);
        registry::write_record(&self.server_dir(&record.id), RECORD, &record)?;
        Ok(record)
    }

    /// Download the server's files into its directory and record the EULA
    /// acceptance the caller obtained from the user.
    pub async fn provision(
        &self,
        record: &ServerRecord,
        cache: Option<&Cache>,
        on_progress: OnProgress<'_>,
    ) -> Result<()> {
        let dir = self.server_dir(&record.id);
        materialize::validate_filename(&record.profile.primary.filename)?;
        if !record.profile.libraries.is_empty() {
            materialize::ensure_libraries(
                cache,
                &record.profile.libraries,
                &dir.join("libraries"),
                on_progress,
            )
            .await?;
        }
        materialize::ensure_artifact(
            cache,
            &record.profile.primary,
            &dir.join(&record.profile.primary.filename),
            ProvisionPhase::Server,
            on_progress,
        )
        .await?;
        std::fs::write(dir.join("eula.txt"), "eula=true\n").context("cannot write eula.txt")?;
        tracing::info!(id = %record.id, "server provisioned");
        Ok(())
    }

    pub fn mark_ready(&self, id: &str) -> Result<ServerRecord> {
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        record.ready = true;
        registry::write_record(&self.server_dir(id), RECORD, &record)?;
        Ok(record)
    }

    /// Delete a server's directory (jar, world and all). Returns false when no
    /// server matches.
    pub fn remove(&self, reference: &str) -> Result<bool> {
        let Some(record) = self.get(reference) else {
            return Ok(false);
        };
        let dir = self.server_dir(&record.id);
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("cannot remove {}", dir.display()))?;
        tracing::info!(id = %record.id, "server removed");
        Ok(true)
    }

    pub fn launch_plan(&self, record: &ServerRecord, java: &Path) -> LaunchPlan {
        launch::server_plan(&record.profile, java, &self.server_dir(&record.id))
    }
}

// Both the game and rcon listeners bind all interfaces, so probe the same way.
fn can_bind(port: u16) -> bool {
    std::net::TcpListener::bind(("0.0.0.0", port)).is_ok()
}

fn allocate_port(base: u16, claimed: &HashSet<u16>) -> Result<u16> {
    (base..base.saturating_add(PORT_SPAN))
        .find(|port| !claimed.contains(port) && can_bind(*port))
        .with_context(|| format!("no free port in {base}..{}", base.saturating_add(PORT_SPAN)))
}

// Vanilla has no rcon bind-address setting — the listener is reachable from
// the network, so the password is the only barrier. Never log it.
fn generate_password() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).expect("system RNG must be available for the rcon password");
    bytes
        .iter()
        .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
        .collect()
}

/// Rewrite `entries` into the properties file, preserving every other line
/// (user edits included) and appending keys not yet present.
fn merge_properties(path: &Path, entries: &[(&str, String)]) -> Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(str::to_string).collect();
    for (key, value) in entries {
        let prefix = format!("{key}=");
        let entry = format!("{key}={value}");
        match lines
            .iter_mut()
            .find(|l| l.trim_start().starts_with(&prefix))
        {
            Some(line) => *line = entry,
            None => lines.push(entry),
        }
    }
    std::fs::write(path, lines.join("\n") + "\n")
        .with_context(|| format!("cannot write {}", path.display()))
}
