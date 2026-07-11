//! Persistent Minecraft server store: each server lives at `<dir>/<id>/` —
//! the `server.json` record beside `data/`, the working directory the game
//! itself runs in (jar, `eula.txt`, `server.properties`, the world). The root
//! is reserved for managed content directories (`mods/`, `plugins/`,
//! `configs/`, `backups/`); every directory appears on demand. Listing scans
//! the parent — the disk is the registry.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::minecraft::{ProvisionPhase, ProvisionProgress, ServerProfile};
use serde::{Deserialize, Serialize};

use crate::backup::BackupSettings;
use crate::cache::Cache;
use crate::minecraft::launch::{self, JavaSettings, LaunchPlan};
use crate::minecraft::materialize::{self, OnProgress};
use crate::registry;

const RECORD: &str = "server.json";
const PROPERTIES: &str = "server.properties";
const DATA: &str = "data";
const GAME_PORT_BASE: u16 = 25565;
const RCON_PORT_BASE: u16 = 25575;
const PORT_SPAN: u16 = 100;

// Pre-EULA servers (before 1.7.10) have no gate and would boot for real; the
// generation run is killed after this long and the file check decides.
const GENERATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// `server.properties` keys hestia owns; a `config set` to any of them is
/// rejected (the game port is fixed at create, rcon is configured at start).
const MANAGED_PROPERTIES: &[&str] = &["server-port", "enable-rcon", "rcon.port", "rcon.password"];

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
    /// Per-entry JVM tuning (memory, extra flags) injected at each start.
    #[serde(default)]
    pub jvm: JavaSettings,
    /// Scheduled-backup tuning (interval, retention); unset disables the
    /// schedule.
    #[serde(default)]
    pub backup: BackupSettings,
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

    /// The server's working directory — everything the game itself reads and
    /// writes (jar, libraries, `eula.txt`, `server.properties`, the world).
    pub fn data_dir(&self, id: &str) -> PathBuf {
        self.server_dir(id).join(DATA)
    }

    pub fn list(&self) -> Vec<ServerRecord> {
        let mut records: Vec<ServerRecord> = registry::scan(&self.dir(), RECORD);
        records.sort_by(|a, b| a.name.cmp(&b.name));
        records
    }

    /// Find one server by id or name (any spelling that slugs the same).
    pub fn get(&self, reference: &str) -> Option<ServerRecord> {
        self.list()
            .into_iter()
            .find(|r| proto::naming::reference_matches(reference, &r.id, &r.name))
    }

    /// Register a new server: allocate its id from the name, claim its game
    /// port, create its directory, and write the (not yet ready) record.
    pub fn create(
        &self,
        name: &str,
        profile: ServerProfile,
        port: Option<u16>,
    ) -> Result<ServerRecord> {
        if registry::name_taken(name, self.list().iter().map(|r| r.name.as_str())) {
            bail!("a server named '{name}' already exists");
        }
        let id = registry::allocate_id(name, |id| self.get(id).is_some())?;
        let _claims = self.claims.lock().unwrap();
        let game_port = self.claim_game_port(port)?;
        let record = ServerRecord {
            id: id.clone(),
            name: name.to_string(),
            created_unix: registry::now_unix(),
            ready: false,
            game_port: Some(game_port),
            rcon: None,
            jvm: JavaSettings::default(),
            backup: BackupSettings::default(),
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
            &self.data_dir(&record.id).join(PROPERTIES),
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

    /// Download the server's files into its directory, generate its
    /// `server.properties` schema, and record the EULA acceptance the caller
    /// obtained from the user.
    pub async fn provision(
        &self,
        record: &ServerRecord,
        cache: Option<&Cache>,
        java: &Path,
        on_progress: OnProgress<'_>,
    ) -> Result<()> {
        let data = self.data_dir(&record.id);
        std::fs::create_dir_all(&data)
            .with_context(|| format!("cannot create {}", data.display()))?;
        materialize::validate_filename(&record.profile.primary.filename)?;
        if !record.profile.libraries.is_empty() {
            materialize::ensure_libraries(
                cache,
                &record.profile.libraries,
                &data.join("libraries"),
                on_progress,
            )
            .await?;
        }
        materialize::ensure_artifact(
            cache,
            &record.profile.primary,
            &data.join(&record.profile.primary.filename),
            ProvisionPhase::Server,
            on_progress,
        )
        .await?;

        on_progress(&ProvisionProgress {
            phase: ProvisionPhase::Server,
            current: 0,
            total: 0,
            detail: "generating server.properties".into(),
            ..ProvisionProgress::default()
        });
        if let Err(e) = self.generate_properties(record, java).await {
            tracing::warn!(id = %record.id, error = format!("{e:#}"), "server.properties generation failed");
        } else if !data.join(PROPERTIES).exists() {
            tracing::warn!(id = %record.id, "the server did not write server.properties");
        }

        std::fs::write(data.join("eula.txt"), "eula=true\n").context("cannot write eula.txt")?;
        tracing::info!(id = %record.id, "server provisioned");
        Ok(())
    }

    /// Run the server once, before `eula.txt` exists, to make it write the
    /// complete `server.properties` for exactly its version: the EULA gate
    /// stops it right after, before it binds ports or generates a world. The
    /// file is the ground truth `config_set` validates against.
    async fn generate_properties(&self, record: &ServerRecord, java: &Path) -> Result<()> {
        let plan = self.launch_plan(record, java);
        let mut child = tokio::process::Command::new(&plan.program)
            .args(&plan.args)
            .current_dir(&plan.cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("cannot run {}", plan.program.display()))?;
        match tokio::time::timeout(GENERATE_TIMEOUT, child.wait()).await {
            Ok(status) => {
                let status = status.context("waiting for the generation run")?;
                tracing::debug!(id = %record.id, %status, "server.properties generation run exited");
            }
            Err(_) => {
                let _ = child.kill().await;
                tracing::debug!(id = %record.id, "server.properties generation run timed out (no EULA gate?)");
            }
        }
        Ok(())
    }

    /// Move a server onto a freshly resolved profile. The record swaps under
    /// the `ready` gate, so a half-updated server cannot start and a failed
    /// update is recovered by updating again. Name, ports, JVM settings, and
    /// the world on disk are untouched.
    pub async fn update(
        &self,
        id: &str,
        profile: ServerProfile,
        cache: Option<&Cache>,
        java: &Path,
        on_progress: OnProgress<'_>,
    ) -> Result<ServerRecord> {
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        let previous_primary = record.profile.primary.filename.clone();
        record.profile = profile;
        record.ready = false;
        let data = self.data_dir(&record.id);
        materialize::validate_filename(&record.profile.primary.filename)?;
        registry::write_record(&self.server_dir(&record.id), RECORD, &record)?;

        if !record.profile.libraries.is_empty() {
            materialize::ensure_libraries(
                cache,
                &record.profile.libraries,
                &data.join("libraries"),
                on_progress,
            )
            .await?;
        }
        materialize::ensure_artifact(
            cache,
            &record.profile.primary,
            &data.join(&record.profile.primary.filename),
            ProvisionPhase::Server,
            on_progress,
        )
        .await?;

        on_progress(&ProvisionProgress {
            phase: ProvisionPhase::Server,
            current: 0,
            total: 0,
            detail: "regenerating server.properties".into(),
            ..ProvisionProgress::default()
        });
        if let Err(e) = self.regenerate_properties(&record, java).await {
            tracing::warn!(id = %record.id, error = format!("{e:#}"), "server.properties regeneration failed");
        }

        if previous_primary != record.profile.primary.filename {
            let _ = std::fs::remove_file(data.join(&previous_primary));
        }
        tracing::info!(
            id = %record.id,
            version = %record.profile.game_version,
            loader = ?record.profile.loader_version,
            "server updated"
        );
        self.mark_ready(&record.id)
    }

    /// Rerun the schema-generation trick for the record's new version: with
    /// `eula.txt` suspended the gate stops the run right after it rewrites
    /// `server.properties` — set values survive, keys the version does not
    /// know are dropped. The acceptance recorded at create is rewritten even
    /// when the run fails.
    async fn regenerate_properties(&self, record: &ServerRecord, java: &Path) -> Result<()> {
        let eula = self.data_dir(&record.id).join("eula.txt");
        if eula.exists() {
            std::fs::remove_file(&eula).context("cannot suspend eula.txt for the schema run")?;
        }
        let generated = self.generate_properties(record, java).await;
        std::fs::write(&eula, "eula=true\n").context("cannot restore eula.txt")?;
        generated
    }

    pub fn mark_ready(&self, id: &str) -> Result<ServerRecord> {
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        record.ready = true;
        registry::write_record(&self.server_dir(id), RECORD, &record)?;
        Ok(record)
    }

    /// Rename a server: rewrite the record's display name. The id is stable, so
    /// the directory, ports, rcon, and JVM/backup settings stay put — only the
    /// name field changes. The caller guarantees the server is stopped and not
    /// busy.
    pub fn rename(&self, reference: &str, new_name: &str) -> Result<ServerRecord> {
        let _claims = self.claims.lock().unwrap();
        let mut record = self
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        if registry::name_taken(
            new_name,
            self.list()
                .iter()
                .filter(|r| r.id != record.id)
                .map(|r| r.name.as_str()),
        ) {
            bail!("a server named '{new_name}' already exists");
        }
        record.name = new_name.to_string();
        registry::write_record(&self.server_dir(&record.id), RECORD, &record)?;
        tracing::info!(id = %record.id, name = %new_name, "server renamed");
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
        launch::server_plan(
            &record.profile,
            java,
            &self.data_dir(&record.id),
            &record.jvm,
        )
    }

    /// Read one setting: a reserved JVM or backup key from the record, or any
    /// other key from `server.properties`. `Ok(None)` means the key is not set.
    pub fn config_get(&self, id: &str, key: &str) -> Result<Option<String>> {
        let record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        if let Some(value) = record.jvm.get(key).or_else(|| record.backup.get(key)) {
            return Ok(value);
        }
        Ok(read_property(
            &self.data_dir(&record.id).join(PROPERTIES),
            key,
        ))
    }

    /// Write one setting: a reserved JVM or backup key onto the record, or a
    /// `server.properties` key through to the file. A property key must exist
    /// in the file the server itself generated at create — the ground truth
    /// for exactly its version — so a typo cannot silently drift the file
    /// (without a file to check against, any key is accepted). The
    /// hestia-managed keys are rejected. An empty value clears a JVM key.
    /// Settings take effect on the next start.
    pub fn config_set(&self, id: &str, key: &str, value: &str) -> Result<()> {
        let _claims = self.claims.lock().unwrap();
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        if record.jvm.set(key, value)? || record.backup.set(key, value)? {
            registry::write_record(&self.server_dir(&record.id), RECORD, &record)?;
            return Ok(());
        }
        if MANAGED_PROPERTIES.contains(&key) {
            bail!(
                "'{key}' is managed by hestia (the game port is fixed at create with -p; \
                 rcon is configured automatically)"
            );
        }
        let properties = self.data_dir(&record.id).join(PROPERTIES);
        if properties.exists() {
            if read_property(&properties, key).is_none() {
                bail!("'{key}' is not a server.properties key this server's version knows");
            }
        } else {
            tracing::debug!(
                id = %record.id,
                key,
                "no server.properties to validate against; accepting the key"
            );
        }
        merge_properties(&properties, &[(key, value.to_string())])
    }

    /// The reserved JVM and backup settings (always shown) followed by every
    /// current `server.properties` entry.
    pub fn config_list(&self, id: &str) -> Result<Vec<(String, String)>> {
        let record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        let mut entries = record.jvm.entries();
        entries.extend(record.backup.entries());
        entries.extend(read_properties(&self.data_dir(&record.id).join(PROPERTIES)));
        Ok(entries)
    }
}

/// Parse `server.properties` into key/value pairs, skipping blank and comment
/// lines. Values are kept verbatim; the split is on the first `=`.
fn read_properties(path: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let (key, value) = trimmed.split_once('=')?;
            Some((key.trim().to_string(), value.to_string()))
        })
        .collect()
}

fn read_property(path: &Path, key: &str) -> Option<String> {
    read_properties(path)
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

/// The server's world directory name (`level-name`, default `world`), read from
/// `server.properties` in `data_dir`. This is where the server keeps its world,
/// and so where datapacks install.
pub(crate) fn level_name(data_dir: &Path) -> String {
    read_property(&data_dir.join(PROPERTIES), "level-name")
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "world".to_string())
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
/// (user edits included) and appending keys not yet present. The data
/// directory appears on demand with the file.
fn merge_properties(path: &Path, entries: &[(&str, String)]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
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
