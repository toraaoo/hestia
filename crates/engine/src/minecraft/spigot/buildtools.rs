//! Compiling a Bukkit or Spigot server with SpigotMC's BuildTools.
//!
//! BuildTools clones the four upstream repositories, decompiles the vanilla
//! server, applies the CraftBukkit and Spigot patch sets and compiles the
//! result — the only way either jar legally comes into existence, and why these
//! flavors use the provider `install` hook at all.
//!
//! Two consequences shape everything here. It is *slow* (a first build is
//! minutes of decompilation and maven, and clones a few hundred megabytes), so
//! the work tree is shared by every version and both flavors and a build is
//! skipped whenever its jar is already there. And it drives external tools —
//! git and a POSIX shell — which it installs for itself only on Windows, so a
//! missing toolchain is checked before the create commits to anything.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use proto::minecraft::{ProvisionPhase, ProvisionProgress};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc::UnboundedSender;

use super::super::materialize::{self, OnProgress};
use super::super::meta::spigot;
use super::super::provider::InstallRequest;
use crate::download::Downloader;

const TOOLS_JAR: &str = "BuildTools.jar";
const POLL: Duration = Duration::from_millis(250);
const REPORT_EVERY: Duration = Duration::from_secs(1);
/// Longer than this is a maven stack trace or a path dump, not a step name.
const MAX_DETAIL: usize = 160;

/// Which of the two jars one build produces. Both come out of the same run, so
/// a flavor names the one it launches and inherits the other for free.
#[derive(Clone, Copy)]
pub enum Product {
    Bukkit,
    Spigot,
}

impl Product {
    /// What BuildTools calls the jar it copies out, and therefore the name the
    /// launch profile carries.
    pub fn jar_name(self, version: &str) -> String {
        format!("{}-{version}.jar", self.prefix())
    }

    fn prefix(self) -> &'static str {
        match self {
            Product::Bukkit => "craftbukkit",
            Product::Spigot => "spigot",
        }
    }
}

/// Put this product's jar in the entry's directory, building it first if no
/// build has produced it yet. Idempotent on the jar's presence — the install
/// hook runs on every create and update, and a build is minutes of work.
pub async fn ensure(
    product: Product,
    request: &InstallRequest<'_>,
    on_progress: OnProgress<'_>,
) -> Result<()> {
    materialize::validate_filename(request.game_version)?;
    let target = request.root.join(product.jar_name(request.game_version));
    if target.is_file() {
        return Ok(());
    }

    let jars = jars_dir(request.meta, request.game_version);
    if built(&jars, product).is_none() {
        build(request, on_progress).await?;
    }
    let built = built(&jars, product).with_context(|| {
        format!(
            "BuildTools produced no {} jar for Minecraft {}",
            product.prefix(),
            request.game_version
        )
    })?;
    std::fs::copy(&built, &target).with_context(|| format!("cannot place {}", target.display()))?;
    Ok(())
}

/// Where BuildTools works: one tree shared by every version and both flavors.
/// It keeps the upstream clones and its own maven repository there and reuses
/// them across builds, so only the first build pays for the download.
fn work_dir(meta: &Path) -> PathBuf {
    meta.join("spigot")
}

fn jars_dir(meta: &Path, version: &str) -> PathBuf {
    work_dir(meta).join("jars").join(version)
}

/// The jar a build left for this product, found by prefix rather than by the
/// exact name: BuildTools names its output after the version *its* metadata
/// reports, which is not always the string that was asked for.
fn built(jars: &Path, product: Product) -> Option<PathBuf> {
    let prefix = format!("{}-", product.prefix());
    std::fs::read_dir(jars).ok()?.flatten().find_map(|entry| {
        let name = entry.file_name().to_string_lossy().into_owned();
        (name.starts_with(&prefix) && name.ends_with(".jar")).then(|| entry.path())
    })
}

async fn build(request: &InstallRequest<'_>, on_progress: OnProgress<'_>) -> Result<()> {
    ensure_toolchain().await?;
    let work = work_dir(request.meta);
    let jars = jars_dir(request.meta, request.game_version);
    std::fs::create_dir_all(&jars).with_context(|| format!("cannot create {}", jars.display()))?;

    let tools = work.join(TOOLS_JAR);
    if !tools.is_file() {
        // Jenkins publishes no checksum for the artifact, so the fetch is
        // plain; a truncated jar fails the run below rather than being trusted.
        Downloader::new(request.cache)
            .fetch(spigot::BUILDTOOLS_URL, &tools, None, &|_| Ok(()))
            .await
            .context("cannot fetch BuildTools")?;
    }

    on_progress.check()?;
    run(request, &tools, &work, &jars, on_progress).await
}

/// Refuse before the long work starts when the tools BuildTools shells out to
/// are missing. It bootstraps its own PortableGit on Windows and nowhere else,
/// so only the other platforms are checked — and the message names the fix,
/// since nothing in the launcher can install git for the user.
#[cfg(not(windows))]
async fn ensure_toolchain() -> Result<()> {
    for (program, args) in [("git", ["--version"].as_slice()), ("sh", &["-c", "exit"])] {
        let ok = tokio::process::Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .is_ok_and(|status| status.success());
        if !ok {
            bail!(
                "building a Bukkit or Spigot server needs '{program}' on PATH \
                 (SpigotMC's BuildTools drives it); install it and try again"
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
async fn ensure_toolchain() -> Result<()> {
    Ok(())
}

/// Run BuildTools to completion, relaying its narration as progress.
///
/// The whole build is one process, so it is a single cancellation checkpoint
/// polled while it runs: dropping the child kills it, leaving a partial work
/// tree that the next build repairs the same way a failed one does. Its own
/// grandchildren (maven, git) are not in the same process group and finish on
/// their own.
async fn run(
    request: &InstallRequest<'_>,
    tools: &Path,
    work: &Path,
    jars: &Path,
    on_progress: OnProgress<'_>,
) -> Result<()> {
    let version = request.game_version;
    tracing::info!(version, "compiling bukkit and spigot with BuildTools");
    let mut child = tokio::process::Command::new(request.java)
        .arg("-jar")
        .arg(tools)
        .arg("--rev")
        .arg(version)
        .arg("--output-dir")
        .arg(jars)
        // Both, always: the run costs the same and it leaves the other flavor
        // on this version free.
        .args(["--compile", "CRAFTBUKKIT", "--compile", "SPIGOT", "--nogui"])
        .current_dir(work)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("cannot run BuildTools")?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    if let Some(stdout) = child.stdout.take() {
        relay(stdout, tx.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        relay(stderr, tx);
    }

    let mut reported = Instant::now() - REPORT_EVERY;
    loop {
        while let Ok(line) = rx.try_recv() {
            tracing::debug!(target: "buildtools", "{line}");
            if is_step(&line) && reported.elapsed() >= REPORT_EVERY {
                reported = Instant::now();
                on_progress.report(&ProvisionProgress {
                    phase: ProvisionPhase::Server,
                    detail: line,
                    ..ProvisionProgress::default()
                });
            }
        }
        // Dropping `child` on the way out kills it (`kill_on_drop`).
        on_progress.check()?;
        if let Some(status) = child.try_wait().context("cannot await BuildTools")? {
            while let Ok(line) = rx.try_recv() {
                tracing::debug!(target: "buildtools", "{line}");
            }
            if !status.success() {
                bail!("BuildTools failed to build Minecraft {version} ({status})");
            }
            tracing::info!(version, "bukkit and spigot built");
            return Ok(());
        }
        tokio::time::sleep(POLL).await;
    }
}

fn relay<R: AsyncRead + Unpin + Send + 'static>(reader: R, tx: UnboundedSender<String>) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if tx.send(line).is_err() {
                return;
            }
        }
    });
}

/// Whether a line is BuildTools narrating its own progress rather than passing
/// maven's, git's or the decompiler's through. A build emits tens of thousands
/// of lines, most of them bracketed levels or one per patched file; reporting
/// only its own — and at most one a second — keeps a long build visible without
/// flooding the event hub.
fn is_step(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty()
        && line.len() <= MAX_DETAIL
        && !line.starts_with('[')
        && !line.starts_with("Patching ")
        && !line.starts_with("Extracted:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_product_names_the_jar_buildtools_writes() {
        assert_eq!(Product::Spigot.jar_name("1.21.4"), "spigot-1.21.4.jar");
        assert_eq!(Product::Bukkit.jar_name("1.21.4"), "craftbukkit-1.21.4.jar");
    }

    #[test]
    fn only_buildtools_own_narration_is_reported() {
        assert!(is_step("Compiling Spigot & Spigot-API"));
        assert!(is_step("Applying CraftBukkit Patches"));
        assert!(is_step("Success! Everything completed successfully."));
        assert!(!is_step("[INFO] Building Spigot-API 1.21.4-R0.1-SNAPSHOT"));
        assert!(!is_step(
            "Patching net/minecraft/server/MinecraftServer.java"
        ));
        assert!(!is_step(""));
        assert!(!is_step(&"x".repeat(MAX_DETAIL + 1)));
    }
}
