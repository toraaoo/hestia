//! Self-update over the published release manifest (`latest.json`): a version
//! check and the signed installer download. Network reads are stateless; the
//! staging directory only holds the downloaded installer.
//!
//! The endpoint and the minisign public key are read from the desktop app's
//! `tauri.conf.json` (`plugins.updater`), embedded at compile time — one
//! source of truth for every front-end, so a regenerated key or a moved feed
//! is a single-file change.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use proto::update::{UpdateCheckResult, UpdateInfo};

use crate::download::{http_client, Downloader, ProgressFn};

const DESKTOP_CONF: &str = include_str!("../../desktop/tauri.conf.json");

struct UpdaterConfig {
    endpoint: String,
    pubkey: String,
}

fn updater_config() -> Result<&'static UpdaterConfig> {
    static CONFIG: OnceLock<Option<UpdaterConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let conf: serde_json::Value = serde_json::from_str(DESKTOP_CONF).ok()?;
            let updater = conf.get("plugins")?.get("updater")?;
            Some(UpdaterConfig {
                endpoint: updater.get("endpoints")?.get(0)?.as_str()?.to_string(),
                pubkey: updater.get("pubkey")?.as_str()?.to_string(),
            })
        })
        .as_ref()
        .context("no updater endpoint/pubkey configured in tauri.conf.json")
}

pub struct Update {
    dir: Mutex<PathBuf>,
}

#[derive(serde::Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    platforms: HashMap<String, PlatformEntry>,
}

#[derive(serde::Deserialize, Clone)]
struct PlatformEntry {
    url: String,
    signature: String,
}

impl Update {
    pub fn new(dir: PathBuf) -> Self {
        Update {
            dir: Mutex::new(dir),
        }
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.dir.lock().unwrap() = dir;
    }

    pub async fn check(&self) -> Result<UpdateCheckResult> {
        let manifest = fetch_manifest().await?;
        Ok(UpdateCheckResult {
            current: common::app::VERSION.to_string(),
            available: available(&manifest).map(|entry| UpdateInfo {
                version: manifest.version.clone(),
                notes: manifest.notes.clone(),
                url: entry.url.clone(),
            }),
        })
    }

    /// Download this platform's latest installer, verifying its minisign
    /// signature before handing back the path — a file that fails to verify
    /// is discarded. Returns the path and the version it carries.
    pub async fn download(&self, on_progress: &ProgressFn<'_>) -> Result<(PathBuf, String)> {
        let manifest = fetch_manifest().await?;
        let entry = available(&manifest)
            .ok_or_else(|| anyhow!("{} is already the latest version", common::app::VERSION))?
            .clone();
        let name = entry
            .url
            .rsplit('/')
            .next()
            .filter(|n| !n.is_empty())
            .context("update url has no file name")?
            .to_string();
        let dir = self.dir.lock().unwrap().clone();
        std::fs::create_dir_all(&dir).context("cannot create the update staging directory")?;
        let dest = dir.join(name);
        Downloader::new(None)
            .fetch(&entry.url, &dest, None, on_progress)
            .await?;
        if let Err(e) = verify_signature(&dest, &entry.signature) {
            let _ = std::fs::remove_file(&dest);
            return Err(e.context("update signature verification failed"));
        }
        Ok((dest, manifest.version))
    }
}

async fn fetch_manifest() -> Result<Manifest> {
    http_client()
        .get(&updater_config()?.endpoint)
        .send()
        .await
        .context("cannot reach the update endpoint")?
        .error_for_status()
        .context("update endpoint answered an error")?
        .json()
        .await
        .context("malformed update manifest")
}

/// The manifest's entry for this platform, when its version is newer than
/// this build.
fn available(manifest: &Manifest) -> Option<&PlatformEntry> {
    if !is_newer(&manifest.version, common::app::VERSION) {
        return None;
    }
    manifest.platforms.get(&format!(
        "{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ))
}

/// Strictly newer on the numeric `x.y.z` triple; anything unparsable is never
/// newer, so a malformed manifest cannot trigger an update.
fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches('v');
    let v = v.split(['-', '+']).next()?;
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

fn verify_signature(path: &Path, signature: &str) -> Result<()> {
    // Wire contract with tauri's signer: the public key and the signature are
    // both base64-wrapped minisign documents.
    let pubkey = base64_text(&updater_config()?.pubkey).context("bad update public key")?;
    let pubkey = minisign_verify::PublicKey::decode(&pubkey).map_err(|e| anyhow!("{e}"))?;
    let signature = base64_text(signature).context("bad update signature")?;
    let signature = minisign_verify::Signature::decode(&signature).map_err(|e| anyhow!("{e}"))?;
    let data = std::fs::read(path).context("cannot read the downloaded installer")?;
    pubkey
        .verify(&data, &signature, true)
        .map_err(|e| anyhow!("{e}"))
}

fn base64_text(value: &str) -> Result<String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(value.trim())?;
    Ok(String::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_version};

    #[test]
    fn versions_parse_with_prefixes_and_prereleases() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("v0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("not-a-version"), None);
    }

    #[test]
    fn newer_is_strict_and_rejects_garbage() {
        assert!(is_newer("0.0.2", "0.0.1"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.0.1", "0.0.1"));
        assert!(!is_newer("0.0.1", "0.0.2"));
        assert!(!is_newer("garbage", "0.0.1"));
        assert!(!is_newer("0.0.2", "garbage"));
    }
}
