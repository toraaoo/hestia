//! The per-entry installed-content store: files live in the entry root's
//! managed kind directories (`<entry>/mods/`, `resourcepacks/`, `shaderpacks/`)
//! beside a `content.json` index recording each item's provenance, and are
//! mirrored (hardlink, else copy) into the entry's `data/` game directory —
//! the managed tree is hestia's registry, `data/` is what the game loads. A
//! sync pass at every start/launch re-mirrors anything missing, so a backup
//! restore (which swaps `data/`) heals itself.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::content::{ContentFile, ContentKind, ContentVersion, InstalledContent, ReleaseChannel};
use proto::download::HashAlgorithm;
use serde::{Deserialize, Serialize};

use crate::checksum::Hasher;
use crate::registry;

const INDEX: &str = "content.json";

#[derive(Serialize, Deserialize, Default)]
struct Index {
    items: Vec<InstalledContent>,
}

/// The directory name for an installable kind — the game's own load-dir name
/// (`shaderpacks`, as Iris/OptiFine read), so the managed dir and its `data/`
/// mirror stay symmetric. Mods/resourcepacks/shaders have a flat managed dir
/// mirrored into `data/`; a datapack's `datapacks` dir lives inside a world
/// instead (see [`datapack_path`]). Modpacks are not single-file installs.
pub(crate) fn kind_dir(kind: ContentKind) -> Result<&'static str> {
    match kind {
        ContentKind::Mod => Ok("mods"),
        ContentKind::ResourcePack => Ok("resourcepacks"),
        ContentKind::Shader => Ok("shaderpacks"),
        ContentKind::DataPack => Ok("datapacks"),
        ContentKind::Modpack => bail!("modpack content cannot be installed as a single file"),
    }
}

/// A datapack's file path: `data/<world>/datapacks/<file>`, where `world` is
/// the world dir relative to `data/` (`world` for a server's `level-name`,
/// `saves/<name>` for an instance's save). Datapacks load from inside a world,
/// so — unlike other kinds — the file lives under `data/` with no separate
/// managed copy, and is therefore already covered by the world's backups.
pub(crate) fn datapack_path(data_dir: &Path, item: &InstalledContent) -> PathBuf {
    data_dir
        .join(&item.world)
        .join("datapacks")
        .join(&item.filename)
}

pub(crate) fn load(entry_dir: &Path) -> Vec<InstalledContent> {
    registry::read_record::<Index>(entry_dir, INDEX)
        .map(|i| i.items)
        .unwrap_or_default()
}

pub(crate) fn save(entry_dir: &Path, items: Vec<InstalledContent>) -> Result<()> {
    registry::write_record(entry_dir, INDEX, &Index { items })
}

pub(crate) fn managed_path(entry_dir: &Path, item: &InstalledContent) -> Result<PathBuf> {
    Ok(entry_dir.join(kind_dir(item.kind)?).join(&item.filename))
}

pub(crate) fn data_path(data_dir: &Path, item: &InstalledContent) -> Result<PathBuf> {
    Ok(data_dir.join(kind_dir(item.kind)?).join(&item.filename))
}

/// Place `source` at `dest`: hardlink when the filesystem allows (free, same
/// volume), else copy. An existing `dest` is replaced.
pub(crate) fn mirror(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    if dest.exists() {
        std::fs::remove_file(dest).with_context(|| format!("cannot replace {}", dest.display()))?;
    }
    if std::fs::hard_link(source, dest).is_err() {
        std::fs::copy(source, dest)
            .with_context(|| format!("cannot copy {} to {}", source.display(), dest.display()))?;
    }
    Ok(())
}

/// Re-mirror every indexed file whose `data/` copy is missing. Datapacks are
/// skipped: they live inside the world (under `data/`), so a backup restore
/// brings them back with the world — there is no managed copy to heal from.
pub(crate) fn sync(entry_dir: &Path, data_dir: &Path) -> Result<()> {
    let mut healed = 0u32;
    for item in load(entry_dir) {
        if item.kind == ContentKind::DataPack {
            continue;
        }
        let managed = managed_path(entry_dir, &item)?;
        let dest = data_path(data_dir, &item)?;
        if managed.is_file() && !dest.exists() {
            mirror(&managed, &dest)?;
            healed += 1;
        }
    }
    if healed > 0 {
        tracing::info!(entry = %entry_dir.display(), healed, "content mirrored into data dir");
    }
    Ok(())
}

/// Delete an item's managed file and its `data/` mirror (either may already be
/// gone). A datapack has no managed copy — only its in-world file.
pub(crate) fn remove_files(entry_dir: &Path, data_dir: &Path, item: &InstalledContent) {
    let paths = if item.kind == ContentKind::DataPack {
        vec![Some(datapack_path(data_dir, item))]
    } else {
        vec![
            managed_path(entry_dir, item).ok(),
            data_path(data_dir, item).ok(),
        ]
    };
    for path in paths.into_iter().flatten() {
        if let Err(e) = std::fs::remove_file(&path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %path.display(), error = %e, "cannot remove content file");
            }
        }
    }
}

/// Whether `reference` names this item: its project id, slug, filename, or
/// title (the human-facing ones case-insensitively).
pub(crate) fn matches(item: &InstalledContent, reference: &str) -> bool {
    item.project_id == reference
        || item.filename == reference
        || item.slug.eq_ignore_ascii_case(reference)
        || item.title.eq_ignore_ascii_case(reference)
}

/// Pick the version to install from a project's (newest-first) catalogue. A
/// non-empty `pin` matches by version id or number; otherwise the newest
/// compatible release, else the newest compatible of any channel.
pub(crate) fn pick_version<'a>(
    versions: &'a [ContentVersion],
    game_version: &str,
    loader: Option<&str>,
    pin: &str,
) -> Result<&'a ContentVersion> {
    if !pin.is_empty() {
        return versions
            .iter()
            .find(|v| v.id == pin || v.version_number == pin)
            .with_context(|| format!("no version matches '{pin}'"));
    }
    let compatible = |v: &&ContentVersion| {
        v.game_versions.iter().any(|g| g == game_version)
            && loader.is_none_or(|l| v.loaders.iter().any(|x| x == l))
    };
    versions
        .iter()
        .filter(compatible)
        .find(|v| v.channel == ReleaseChannel::Release)
        .or_else(|| versions.iter().find(compatible))
        .with_context(|| {
            format!(
                "no compatible version for {game_version}{}",
                loader.map(|l| format!(" ({l})")).unwrap_or_default()
            )
        })
}

pub(crate) fn primary_file(version: &ContentVersion) -> Result<&ContentFile> {
    version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .with_context(|| format!("version '{}' has no files", version.version_number))
}

pub(crate) fn sha1_file(path: &Path) -> Result<String> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut hasher = Hasher::new(HashAlgorithm::Sha1);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.hex_digest())
}

/// Filenames present in `data/<kind dir>` that no index entry accounts for —
/// files the user dropped in by hand.
pub(crate) fn untracked(
    data_dir: &Path,
    kind: ContentKind,
    items: &[InstalledContent],
) -> Vec<String> {
    // Datapacks live under per-world dirs, not one flat dir; there is no single
    // place to scan for strays, so none are reported.
    if kind == ContentKind::DataPack {
        return Vec::new();
    }
    let Ok(dir) = kind_dir(kind) else {
        return Vec::new();
    };
    let known: HashSet<&str> = items.iter().map(|i| i.filename.as_str()).collect();
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(data_dir.join(dir)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_file() && !known.contains(name.as_str()) {
                found.push(name);
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, number: &str, channel: ReleaseChannel, games: &[&str]) -> ContentVersion {
        ContentVersion {
            id: id.to_string(),
            version_number: number.to_string(),
            channel,
            game_versions: games.iter().map(|s| s.to_string()).collect(),
            loaders: vec!["fabric".to_string()],
            ..ContentVersion::default()
        }
    }

    #[test]
    fn pick_prefers_newest_compatible_release() {
        let versions = vec![
            version("v3", "3.0-beta", ReleaseChannel::Beta, &["1.21.1"]),
            version("v2", "2.0", ReleaseChannel::Release, &["1.21.1"]),
            version("v1", "1.0", ReleaseChannel::Release, &["1.20.4"]),
        ];
        let picked = pick_version(&versions, "1.21.1", Some("fabric"), "").unwrap();
        assert_eq!(picked.id, "v2");
    }

    #[test]
    fn pick_falls_back_to_prerelease_when_no_release_fits() {
        let versions = vec![
            version("v2", "2.0-beta", ReleaseChannel::Beta, &["1.21.1"]),
            version("v1", "1.0", ReleaseChannel::Release, &["1.20.4"]),
        ];
        let picked = pick_version(&versions, "1.21.1", Some("fabric"), "").unwrap();
        assert_eq!(picked.id, "v2");
    }

    #[test]
    fn pick_filters_by_loader() {
        let mut other = version("v2", "2.0", ReleaseChannel::Release, &["1.21.1"]);
        other.loaders = vec!["forge".to_string()];
        let versions = vec![other];
        assert!(pick_version(&versions, "1.21.1", Some("fabric"), "").is_err());
        assert!(pick_version(&versions, "1.21.1", None, "").is_ok());
    }

    #[test]
    fn pick_pin_matches_id_or_number() {
        let versions = vec![
            version("v2", "2.0", ReleaseChannel::Release, &["1.21.1"]),
            version("v1", "1.0", ReleaseChannel::Release, &["1.20.4"]),
        ];
        assert_eq!(
            pick_version(&versions, "1.21.1", None, "v1").unwrap().id,
            "v1"
        );
        assert_eq!(
            pick_version(&versions, "1.21.1", None, "2.0").unwrap().id,
            "v2"
        );
        assert!(pick_version(&versions, "1.21.1", None, "nope").is_err());
    }

    #[test]
    fn reference_matching() {
        let item = InstalledContent {
            project_id: "AANobbMI".to_string(),
            slug: "sodium".to_string(),
            title: "Sodium".to_string(),
            filename: "sodium-fabric.jar".to_string(),
            ..InstalledContent::default()
        };
        for reference in [
            "AANobbMI",
            "sodium",
            "Sodium",
            "SODIUM",
            "sodium-fabric.jar",
        ] {
            assert!(matches(&item, reference), "should match {reference}");
        }
        assert!(!matches(&item, "lithium"));
    }

    #[test]
    fn installable_kind_dirs() {
        assert_eq!(kind_dir(ContentKind::Mod).unwrap(), "mods");
        assert_eq!(
            kind_dir(ContentKind::ResourcePack).unwrap(),
            "resourcepacks"
        );
        assert_eq!(kind_dir(ContentKind::Shader).unwrap(), "shaderpacks");
        assert_eq!(kind_dir(ContentKind::DataPack).unwrap(), "datapacks");
        assert!(kind_dir(ContentKind::Modpack).is_err());
    }

    #[test]
    fn datapack_path_lives_inside_the_world() {
        let item = InstalledContent {
            kind: ContentKind::DataPack,
            world: "saves/my-world".to_string(),
            filename: "terralith.zip".to_string(),
            ..InstalledContent::default()
        };
        assert_eq!(
            datapack_path(Path::new("/data"), &item),
            Path::new("/data/saves/my-world/datapacks/terralith.zip")
        );
    }
}
