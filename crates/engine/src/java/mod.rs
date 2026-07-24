//! Installs and tracks Java runtimes: each install lives at
//! `<dir>/<vendor>-<major>/` beside a `runtime.json` record, and listing scans
//! that directory — the disk is the registry.

mod adoptium;
mod extract;
mod platform;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::java::{JavaInstallPhase, JavaInstallProgress, JavaRelease, JavaRuntime};
use serde_json::{json, Value};

use crate::cache::Cache;
use crate::download::Downloader;

const RUNTIME_RECORD: &str = "runtime.json";

pub struct JavaInstallOutcome {
    pub runtime: JavaRuntime,
    pub already_installed: bool,
}

pub struct Java {
    dir: Mutex<PathBuf>,
}

impl Java {
    pub fn new(dir: PathBuf) -> Self {
        Java {
            dir: Mutex::new(dir),
        }
    }

    pub fn dir(&self) -> PathBuf {
        self.dir.lock().unwrap().clone()
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.dir.lock().unwrap() = dir;
    }

    pub async fn releases(&self) -> Result<Vec<JavaRelease>> {
        adoptium::releases().await
    }

    /// Scan the install directory; the disk is the registry.
    pub fn installed(&self) -> Vec<JavaRuntime> {
        let mut runtimes = Vec::new();
        if let Ok(entries) = std::fs::read_dir(self.dir()) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(runtime) = read_runtime(&entry.path()) {
                        runtimes.push(runtime);
                    }
                }
            }
        }
        runtimes.sort_by_key(|r| r.major);
        runtimes
    }

    pub fn uninstall(&self, major: i32) -> bool {
        let mut removed = false;
        if let Ok(entries) = std::fs::read_dir(self.dir()) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                if read_runtime(&entry.path()).map(|r| r.major) == Some(major) {
                    let _ = std::fs::remove_dir_all(entry.path());
                    removed = true;
                }
            }
        }
        if removed {
            tracing::info!(major, "uninstalled java runtime");
        } else {
            tracing::debug!(major, "no java runtime to uninstall");
        }
        removed
    }

    /// Blocking resolve → download → extract → register; a failed install leaves
    /// nothing behind. An already-installed line is returned as-is unless `force`.
    pub async fn install(
        &self,
        major: i32,
        force: bool,
        cache: Option<&Cache>,
        on_progress: impl Fn(&JavaInstallProgress) + Send + Sync,
    ) -> Result<JavaInstallOutcome> {
        if major <= 0 {
            bail!(proto::error::ErrorInfo::InvalidValue {
                field: proto::error::Field::JavaVersion,
                reason: proto::error::Reason::JavaMajor
            });
        }
        tracing::info!(major, force, "java install requested");
        if !force {
            if let Some(runtime) = self.installed().into_iter().find(|r| r.major == major) {
                tracing::info!(major, release = %runtime.release_name, "java already installed");
                return Ok(JavaInstallOutcome {
                    runtime,
                    already_installed: true,
                });
            }
        }
        let phase = |p: JavaInstallPhase| JavaInstallProgress {
            phase: p,
            current: 0,
            total: 0,
        };

        on_progress(&phase(JavaInstallPhase::Resolving));
        let package = adoptium::resolve(major, &platform::host_target()?).await?;
        validate_archive_name(&package.archive_name)?;
        tracing::info!(
            major,
            release = %package.release_name,
            archive = %package.archive_name,
            "resolved java package"
        );

        let base = self.dir();
        let install_dir = base.join(format!("{}-{}", package.vendor, package.major));
        let archive = base.join("tmp").join(&package.archive_name);
        let staging = with_suffix(&install_dir, ".staging");

        let _ = std::fs::remove_dir_all(&staging);
        let result = self
            .run_install(
                &package,
                &archive,
                &staging,
                &install_dir,
                cache,
                &on_progress,
            )
            .await;
        if let Err(e) = &result {
            tracing::error!(major, "java install failed: {e:#}");
            let _ = std::fs::remove_dir_all(&staging);
            let _ = std::fs::remove_file(&archive);
        }
        result?;
        let _ = std::fs::remove_file(&archive);

        let outcome = read_runtime(&install_dir)
            .map(|runtime| JavaInstallOutcome {
                runtime,
                already_installed: false,
            })
            .with_context(|| {
                format!(
                    "install of {} did not produce a usable runtime",
                    package.release_name
                )
            })?;
        tracing::info!(
            major,
            home = %outcome.runtime.home.display(),
            "installed java runtime"
        );
        Ok(outcome)
    }

    async fn run_install(
        &self,
        package: &adoptium::JavaPackage,
        archive: &Path,
        staging: &Path,
        install_dir: &Path,
        cache: Option<&Cache>,
        on_progress: &(impl Fn(&JavaInstallProgress) + Send + Sync),
    ) -> Result<()> {
        Downloader::new(cache)
            .fetch(&package.url, archive, Some(&package.checksum), &|dp| {
                on_progress(&JavaInstallProgress {
                    phase: JavaInstallPhase::Downloading,
                    current: dp.downloaded,
                    total: dp.total,
                });
            })
            .await?;

        on_progress(&JavaInstallProgress {
            phase: JavaInstallPhase::Extracting,
            current: 0,
            total: 0,
        });
        tracing::debug!(archive = %archive.display(), "extracting java archive");
        let archive_owned = archive.to_path_buf();
        let staging_owned = staging.to_path_buf();
        tokio::task::spawn_blocking(move || {
            extract::extract_archive(&archive_owned, &staging_owned, |_, _| {})
        })
        .await
        .context("extraction task panicked")??;

        let executable = platform::find_java_executable(staging).with_context(|| {
            format!(
                "archive {} contained no java executable",
                package.archive_name
            )
        })?;
        let relative = executable.strip_prefix(staging).unwrap_or(&executable);
        write_runtime_record(staging, package, relative)?;

        let _ = std::fs::remove_dir_all(install_dir);
        std::fs::rename(staging, install_dir).context("moving staged install into place")?;
        Ok(())
    }
}

fn validate_archive_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('.')
        || name.contains('/')
        || name.contains('\\')
        || name.contains('"')
    {
        bail!("provider returned an unsafe archive name: '{name}'");
    }
    Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn write_runtime_record(
    install_dir: &Path,
    package: &adoptium::JavaPackage,
    relative_exe: &Path,
) -> Result<()> {
    let record = json!({
        "vendor": package.vendor,
        "major": package.major,
        "release_name": package.release_name,
        "executable": to_forward_slashes(relative_exe),
    });
    let text = serde_json::to_string_pretty(&record).expect("record serializes");
    std::fs::write(install_dir.join(RUNTIME_RECORD), format!("{text}\n"))
        .with_context(|| format!("cannot write {RUNTIME_RECORD}"))?;
    Ok(())
}

fn read_runtime(install_dir: &Path) -> Option<JavaRuntime> {
    let text = std::fs::read_to_string(install_dir.join(RUNTIME_RECORD)).ok()?;
    let record: Value = serde_json::from_str(&text).ok()?;
    let major = record.get("major").and_then(Value::as_i64).unwrap_or(0) as i32;
    let executable_rel = record
        .get("executable")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let executable = install_dir.join(executable_rel);
    if major <= 0 || !executable.is_file() {
        return None;
    }
    let home = executable
        .parent()
        .and_then(Path::parent)
        .unwrap_or(install_dir)
        .to_path_buf();
    Some(JavaRuntime {
        vendor: record
            .get("vendor")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        major,
        release_name: record
            .get("release_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        home,
        executable,
        // Usage is a cross-store question; the daemon's list handler fills it.
        in_use: false,
    })
}

fn to_forward_slashes(path: &Path) -> String {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}
