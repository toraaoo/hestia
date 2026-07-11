//! Materialises resolved profile pieces onto disk: single artifacts (jars),
//! Maven-layout libraries, and the shared asset store. Every ensure is
//! idempotent — a file already present at the expected size is skipped — so a
//! launch only pays for what is missing.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use proto::download::{Checksum, HashAlgorithm};
use proto::minecraft::{Artifact, AssetIndex, Library, ProvisionPhase, ProvisionProgress};
use serde_json::Value;

use crate::cache::Cache;
use crate::download::Downloader;

const ASSET_HOST: &str = "https://resources.download.minecraft.net";
const CONCURRENT_FETCHES: usize = 16;

pub type OnProgress<'a> = &'a (dyn Fn(&ProvisionProgress) + Send + Sync);

pub fn validate_filename(name: &str) -> Result<()> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        bail!("artifact has an unsafe filename: '{name}'");
    }
    Ok(())
}

/// Join an upstream-supplied relative path under `root`, rejecting anything
/// absolute or traversing.
pub fn safe_join(root: &Path, relative: &str) -> Result<PathBuf> {
    let rel = Path::new(relative);
    if relative.is_empty()
        || rel.is_absolute()
        || rel.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        bail!("unsafe artifact path: '{relative}'");
    }
    Ok(root.join(rel))
}

fn present(path: &Path, size: u64) -> bool {
    match std::fs::metadata(path) {
        Ok(m) => m.is_file() && (size == 0 || m.len() == size),
        Err(_) => false,
    }
}

/// Ensure one artifact at `destination`, reporting byte progress under `phase`.
pub async fn ensure_artifact(
    cache: Option<&Cache>,
    artifact: &Artifact,
    destination: &Path,
    phase: ProvisionPhase,
    on_progress: OnProgress<'_>,
) -> Result<()> {
    if present(destination, artifact.size) {
        return Ok(());
    }
    Downloader::new(cache)
        .fetch(
            &artifact.url,
            destination,
            artifact.checksum.as_ref(),
            &|dp| {
                on_progress(&ProvisionProgress {
                    phase,
                    current: dp.downloaded,
                    total: dp.total,
                    detail: artifact.filename.clone(),
                    ..ProvisionProgress::default()
                });
            },
        )
        .await
}

/// Ensure every library under `root` (Maven layout), reporting completed/total
/// counts. Duplicate paths (a modloader profile layered over the base game) are
/// fetched once.
pub async fn ensure_libraries(
    cache: Option<&Cache>,
    libraries: &[Library],
    root: &Path,
    on_progress: OnProgress<'_>,
) -> Result<()> {
    let mut seen = HashSet::new();
    let mut targets: Vec<Library> = Vec::new();
    for library in libraries {
        if seen.insert(library.path.as_str()) {
            targets.push(library.clone());
        }
    }
    let total = targets.len() as u64;
    let done = AtomicU64::new(0);
    report_count(on_progress, ProvisionPhase::Libraries, 0, total, "");

    // Owned items: a closure taking a reference and returning an async block
    // trips rustc's higher-ranked lifetime inference (rust-lang/rust#89976).
    let fetches = targets.into_iter().map(|library| {
        let done = &done;
        async move {
            let destination = safe_join(root, &library.path)?;
            if !present(&destination, library.artifact.size) {
                Downloader::new(cache)
                    .fetch(
                        &library.artifact.url,
                        &destination,
                        library.artifact.checksum.as_ref(),
                        &|_| {},
                    )
                    .await
                    .with_context(|| format!("library {}", library.name))?;
            }
            let current = done.fetch_add(1, Ordering::Relaxed) + 1;
            report_count(
                on_progress,
                ProvisionPhase::Libraries,
                current,
                total,
                &library.name,
            );
            Ok::<(), anyhow::Error>(())
        }
    });
    drain(fetches).await
}

/// Ensure the asset index and every object it names under `root`
/// (`indexes/<id>.json` + `objects/<hh>/<hash>`), reporting completed/total
/// counts. Objects are content-addressed, so the store is shared by every
/// version and never fetched twice.
pub async fn ensure_assets(
    cache: Option<&Cache>,
    index: &AssetIndex,
    root: &Path,
    on_progress: OnProgress<'_>,
) -> Result<()> {
    validate_filename(&index.id)?;
    let index_path = root.join("indexes").join(format!("{}.json", index.id));
    if !present(&index_path, index.artifact.size) {
        Downloader::new(cache)
            .fetch(
                &index.artifact.url,
                &index_path,
                index.artifact.checksum.as_ref(),
                &|_| {},
            )
            .await
            .context("asset index")?;
    }

    let text = std::fs::read_to_string(&index_path)
        .with_context(|| format!("cannot read {}", index_path.display()))?;
    let parsed: Value = serde_json::from_str(&text).context("asset index is malformed JSON")?;
    if parsed.get("virtual").and_then(Value::as_bool) == Some(true)
        || parsed.get("map_to_resources").and_then(Value::as_bool) == Some(true)
    {
        tracing::warn!(index = %index.id, "legacy (virtual) asset layout is not supported");
    }
    let objects = parsed
        .get("objects")
        .and_then(Value::as_object)
        .context("asset index has no objects map")?;

    let mut todo: Vec<(String, u64)> = Vec::new();
    let mut seen = HashSet::new();
    for object in objects.values() {
        let Some(hash) = object.get("hash").and_then(Value::as_str) else {
            continue;
        };
        if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("asset index names an invalid object hash: '{hash}'");
        }
        if seen.insert(hash) {
            todo.push((
                hash.to_string(),
                object.get("size").and_then(Value::as_u64).unwrap_or(0),
            ));
        }
    }

    let total = todo.len() as u64;
    let done = AtomicU64::new(0);
    report_count(on_progress, ProvisionPhase::Assets, 0, total, &index.id);

    let objects_root = root.join("objects");
    let fetches = todo.into_iter().map(|(hash, size)| {
        let done = &done;
        let objects_root = &objects_root;
        let index_id = &index.id;
        async move {
            let prefix = hash[..2].to_string();
            let destination = objects_root.join(&prefix).join(&hash);
            if !present(&destination, size) {
                let checksum = Checksum {
                    algorithm: HashAlgorithm::Sha1,
                    hex: hash.clone(),
                };
                // The objects tree is itself content-addressed; going through
                // the download cache would only store every asset twice.
                Downloader::new(None)
                    .fetch(
                        &format!("{ASSET_HOST}/{prefix}/{hash}"),
                        &destination,
                        Some(&checksum),
                        &|_| {},
                    )
                    .await
                    .with_context(|| format!("asset object {hash}"))?;
            }
            let current = done.fetch_add(1, Ordering::Relaxed) + 1;
            report_count(
                on_progress,
                ProvisionPhase::Assets,
                current,
                total,
                index_id,
            );
            Ok::<(), anyhow::Error>(())
        }
    });
    drain(fetches).await
}

fn report_count(
    on_progress: OnProgress<'_>,
    phase: ProvisionPhase,
    current: u64,
    total: u64,
    detail: &str,
) {
    on_progress(&ProvisionProgress {
        phase,
        current,
        total,
        detail: detail.to_string(),
        ..ProvisionProgress::default()
    });
}

/// Run the fetch futures a few at a time, failing fast on the first error.
async fn drain<I, F>(fetches: I) -> Result<()>
where
    I: Iterator<Item = F>,
    F: std::future::Future<Output = Result<()>>,
{
    let mut stream = futures_util::stream::iter(fetches).buffer_unordered(CONCURRENT_FETCHES);
    while let Some(result) = stream.next().await {
        result?;
    }
    Ok(())
}
