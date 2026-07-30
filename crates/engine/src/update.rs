//! Self-update over the published release manifest (`latest.json`): a version
//! check and the signed installer download. Network reads are stateless; the
//! staging directory only holds the downloaded installer.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{anyhow, Context, Result};
use proto::update::{UpdateCheckResult, UpdateInfo};

use crate::download::{http_client, Downloader, ProgressFn};
use crate::signature::verify_file;
use crate::version::is_newer;

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
        if let Err(e) = verify_file(&dest, &entry.signature, common::app::update_pubkeys()) {
            let _ = std::fs::remove_file(&dest);
            return Err(e.context("update signature verification failed"));
        }
        Ok((dest, manifest.version))
    }
}

async fn fetch_manifest() -> Result<Manifest> {
    http_client()
        .get(common::app::UPDATE_ENDPOINT)
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
