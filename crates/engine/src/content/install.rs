//! The per-entry installed-content store: files live in the entry root's
//! managed kind directories (`<entry>/mods/`, `resourcepacks/`, `shaderpacks/`,
//! `datapacks/`) beside a `content.json` index recording each item's
//! provenance, and are mirrored (hardlink, else copy) into the entry's `data/`
//! game directory — one flat load dir per kind, or, for a datapack, each save
//! world it targets. The managed tree is hestia's registry, `data/` is what the
//! game loads; a sync pass at every start/launch re-mirrors anything missing,
//! so a backup restore (which swaps `data/`) heals itself.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::content::{ContentFile, ContentKind, ContentVersion, InstalledContent, ReleaseChannel};
use proto::download::HashAlgorithm;
use serde::{Deserialize, Serialize};

use crate::checksum::Hasher;
use crate::content::profiles;
use crate::registry;

const INDEX: &str = "content.json";

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
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
        ContentKind::Modpack => bail!(proto::error::ErrorInfo::UnsupportedOperation {
            reason: proto::error::Unsupported::ModpackNotSingleFile
        }),
    }
}

/// Where a datapack loads from: `data/<world>/datapacks/<file>`. A pack is
/// mirrored into one such dir per target world, from the same managed copy the
/// other kinds keep.
pub(crate) fn datapack_path(data_dir: &Path, world: &str, filename: &str) -> PathBuf {
    data_dir.join(world).join("datapacks").join(filename)
}

/// A world's folder name, from the data-relative path the index stores
/// (`saves/<name>` for an instance, the bare `level-name` for a server).
pub(crate) fn world_name(world: &str) -> &str {
    world.rsplit('/').next().unwrap_or(world)
}

/// The worlds an item's mirror covers: its own selection, or — empty, and for
/// datapacks only — every world the entry has.
pub(crate) fn target_worlds<'a>(item: &'a InstalledContent, worlds: &'a [String]) -> &'a [String] {
    match item.kind == ContentKind::DataPack && item.worlds.is_empty() {
        true => worlds,
        false => &item.worlds,
    }
}

/// Bring one item's mirrors in line with the index immediately (the entry is
/// stopped when this runs) — the launch-time [`sync`] re-asserts the same, so
/// this only makes a change visible before the next start.
pub(crate) fn apply_files(
    entry_dir: &Path,
    data_dir: &Path,
    item: &InstalledContent,
    worlds: &[String],
) -> Result<()> {
    place(entry_dir, data_dir, item, worlds, !item.enabled).map(|_| ())
}

/// Mirror an item's managed file into the game's load dirs, or — `excluded` —
/// take it out of them; either way, copies where it does not belong go. Returns
/// `(mirrored, removed)`.
fn place(
    entry_dir: &Path,
    data_dir: &Path,
    item: &InstalledContent,
    worlds: &[String],
    excluded: bool,
) -> Result<(u32, u32)> {
    let managed = managed_path(entry_dir, item)?;
    let wanted: HashSet<PathBuf> = match excluded {
        true => HashSet::new(),
        false => mirror_paths(data_dir, item, worlds)?.into_iter().collect(),
    };
    let (mut mirrored, mut removed) = (0u32, 0u32);
    for path in candidate_paths(data_dir, item, worlds)? {
        if wanted.contains(&path) {
            if managed.is_file() && !path.exists() {
                mirror(&managed, &path)?;
                mirrored += 1;
            }
        } else if path.is_file() {
            std::fs::remove_file(&path)
                .with_context(|| format!("cannot remove {}", path.display()))?;
            removed += 1;
        }
    }
    Ok((mirrored, removed))
}

/// Every `data/` path an item's managed file is mirrored to: one per target
/// world for a datapack, the single flat load dir for the other kinds.
fn mirror_paths(
    data_dir: &Path,
    item: &InstalledContent,
    worlds: &[String],
) -> Result<Vec<PathBuf>> {
    if item.kind == ContentKind::DataPack {
        return Ok(target_worlds(item, worlds)
            .iter()
            .filter(|world| !item.disabled_worlds.contains(world))
            .map(|world| datapack_path(data_dir, world, &item.filename))
            .collect());
    }
    Ok(vec![data_path(data_dir, item)?])
}

/// Where an item may have a mirrored copy at all — its targets plus, for a
/// datapack, every other world it could have been mirrored into before.
fn candidate_paths(
    data_dir: &Path,
    item: &InstalledContent,
    worlds: &[String],
) -> Result<Vec<PathBuf>> {
    let mut paths = mirror_paths(data_dir, item, worlds)?;
    if item.kind == ContentKind::DataPack {
        for world in worlds.iter().chain(item.worlds.iter()) {
            let path = datapack_path(data_dir, world, &item.filename);
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    Ok(paths)
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

/// Reconcile the `data/` mirror with the index. With no `selection`, heal only:
/// re-mirror every enabled indexed file whose `data/` copy is missing. With a
/// selection (a profile's member filenames), members are mirrored and tracked
/// non-members have their `data/` copy removed (the managed copy stays) —
/// untracked files are never touched. A disabled item is treated like a
/// non-member: kept out of `data/` regardless of selection, so this pass is the
/// single enforcement point for the enabled flag. A datapack mirrors into each
/// of `worlds` it targets (profiles never select one, since a world is shared
/// across them) and is dropped from the worlds it no longer targets.
pub(crate) fn sync(
    entry_dir: &Path,
    data_dir: &Path,
    selection: Option<&HashSet<String>>,
    worlds: &[String],
) -> Result<()> {
    let mut healed = 0u32;
    let mut removed = 0u32;
    for item in load(entry_dir) {
        let excluded = !item.enabled
            || (profiles::selectable(item.kind)
                && selection.is_some_and(|members| !members.contains(&item.filename)));
        let (mirrored, dropped) = place(entry_dir, data_dir, &item, worlds, excluded)?;
        healed += mirrored;
        removed += dropped;
    }
    if healed > 0 || removed > 0 {
        tracing::info!(
            entry = %entry_dir.display(),
            healed,
            removed,
            "content mirror reconciled with data dir"
        );
    }
    Ok(())
}

/// Delete an item's managed file and every `data/` mirror of it (any of which
/// may already be gone).
pub(crate) fn remove_files(
    entry_dir: &Path,
    data_dir: &Path,
    item: &InstalledContent,
    worlds: &[String],
) {
    let paths = std::iter::once(managed_path(entry_dir, item).ok())
        .chain(
            candidate_paths(data_dir, item, worlds)
                .unwrap_or_default()
                .into_iter()
                .map(Some),
        )
        .collect::<Vec<_>>();
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
            .ok_or_else(|| {
                anyhow::Error::from(proto::error::ErrorInfo::VersionNotFound {
                    reference: pin.to_string(),
                })
            });
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

/// Filenames present in the game's load dirs that no index entry accounts for —
/// files the user dropped in by hand. A datapack is reported as `<world>/<file>`,
/// since each world has its own load dir.
pub(crate) fn untracked(
    data_dir: &Path,
    kind: ContentKind,
    items: &[InstalledContent],
    worlds: &[String],
) -> Vec<String> {
    let known: HashSet<&str> = items.iter().map(|i| i.filename.as_str()).collect();
    if kind == ContentKind::DataPack {
        let mut found = Vec::new();
        for world in worlds {
            let dir = data_dir.join(world).join("datapacks");
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if entry.path().is_file() && !known.contains(name.as_str()) {
                    found.push(format!("{}/{name}", world_name(world)));
                }
            }
        }
        found.sort();
        return found;
    }
    let Ok(dir) = kind_dir(kind) else {
        return Vec::new();
    };
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

    fn temp_entry(tag: &str) -> (PathBuf, PathBuf) {
        let entry = std::env::temp_dir().join(format!(
            "hestia-install-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&entry);
        let data = entry.join("data");
        std::fs::create_dir_all(&data).unwrap();
        (entry, data)
    }

    fn tracked(kind: ContentKind, filename: &str) -> InstalledContent {
        InstalledContent {
            kind,
            filename: filename.to_string(),
            enabled: true,
            ..InstalledContent::default()
        }
    }

    fn install_tracked(entry: &Path, data: &Path, item: &InstalledContent) {
        let managed = managed_path(entry, item).unwrap();
        std::fs::create_dir_all(managed.parent().unwrap()).unwrap();
        std::fs::write(&managed, item.filename.as_bytes()).unwrap();
        if item.kind != ContentKind::DataPack {
            mirror(&managed, &data_path(data, item).unwrap()).unwrap();
        }
    }

    #[test]
    fn sync_without_selection_heals_all_tracked_items() {
        let (entry, data) = temp_entry("healall");
        let items = vec![
            tracked(ContentKind::Mod, "sodium.jar"),
            tracked(ContentKind::ResourcePack, "cozy.zip"),
        ];
        for item in &items {
            install_tracked(&entry, &data, item);
        }
        save(&entry, items.clone()).unwrap();
        std::fs::remove_file(data.join("mods/sodium.jar")).unwrap();

        sync(&entry, &data, None, &[]).unwrap();

        assert!(data.join("mods/sodium.jar").is_file());
        assert!(data.join("resourcepacks/cozy.zip").is_file());
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn sync_with_selection_mirrors_members_and_removes_non_members() {
        let (entry, data) = temp_entry("selection");
        let items = vec![
            tracked(ContentKind::Mod, "sodium.jar"),
            tracked(ContentKind::Mod, "lithium.jar"),
            tracked(ContentKind::ResourcePack, "cozy.zip"),
        ];
        for item in &items {
            install_tracked(&entry, &data, item);
        }
        save(&entry, items).unwrap();
        std::fs::remove_file(data.join("mods/sodium.jar")).unwrap();
        std::fs::write(data.join("mods").join("hand-dropped.jar"), "mine").unwrap();

        let members: HashSet<String> = ["sodium.jar".to_string()].into_iter().collect();
        sync(&entry, &data, Some(&members), &[]).unwrap();

        assert!(data.join("mods/sodium.jar").is_file(), "member mirrored");
        assert!(
            !data.join("mods/lithium.jar").exists(),
            "non-member removed"
        );
        assert!(
            !data.join("resourcepacks/cozy.zip").exists(),
            "non-member resourcepack removed"
        );
        assert!(
            data.join("mods/hand-dropped.jar").is_file(),
            "untracked file untouched"
        );
        assert!(
            entry.join("mods/lithium.jar").is_file(),
            "managed copy stays"
        );

        // Clearing the selection mirrors everything back.
        sync(&entry, &data, None, &[]).unwrap();
        assert!(data.join("mods/lithium.jar").is_file());
        assert!(data.join("resourcepacks/cozy.zip").is_file());
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn datapacks_mirror_into_every_world_and_ignore_profile_selection() {
        let (entry, data) = temp_entry("datapack");
        let mut pack = tracked(ContentKind::DataPack, "terralith.zip");
        pack.worlds = Vec::new();
        install_tracked(&entry, &data, &pack);
        save(&entry, vec![pack.clone()]).unwrap();
        let worlds = vec!["saves/one".to_string(), "saves/two".to_string()];

        let empty: HashSet<String> = HashSet::new();
        sync(&entry, &data, Some(&empty), &worlds).unwrap();

        for world in &worlds {
            assert!(
                datapack_path(&data, world, &pack.filename).is_file(),
                "mirrored into {world} despite the profile selection"
            );
        }
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn a_datapack_selection_bounds_which_worlds_get_it() {
        let (entry, data) = temp_entry("dp-selection");
        let mut pack = tracked(ContentKind::DataPack, "terralith.zip");
        pack.worlds = vec!["saves/one".to_string()];
        install_tracked(&entry, &data, &pack);
        let worlds = vec!["saves/one".to_string(), "saves/two".to_string()];
        // A copy the selection no longer covers is dropped.
        mirror(
            &managed_path(&entry, &pack).unwrap(),
            &datapack_path(&data, "saves/two", &pack.filename),
        )
        .unwrap();
        save(&entry, vec![pack.clone()]).unwrap();

        sync(&entry, &data, None, &worlds).unwrap();

        assert!(datapack_path(&data, "saves/one", &pack.filename).is_file());
        assert!(!datapack_path(&data, "saves/two", &pack.filename).exists());
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn sync_keeps_disabled_items_out_of_data() {
        let (entry, data) = temp_entry("disabled");
        let mut mod_item = tracked(ContentKind::Mod, "sodium.jar");
        install_tracked(&entry, &data, &mod_item);
        mod_item.enabled = false;
        save(&entry, vec![mod_item]).unwrap();

        sync(&entry, &data, None, &[]).unwrap();

        assert!(
            !data.join("mods/sodium.jar").exists(),
            "disabled item is not mirrored"
        );
        assert!(
            entry.join("mods/sodium.jar").is_file(),
            "managed copy stays"
        );
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn disabling_one_world_leaves_the_others_loaded() {
        let (entry, data) = temp_entry("dp-disable");
        let mut pack = tracked(ContentKind::DataPack, "terralith.zip");
        pack.disabled_worlds = vec!["saves/two".to_string()];
        install_tracked(&entry, &data, &pack);
        let worlds = vec!["saves/one".to_string(), "saves/two".to_string()];
        save(&entry, vec![pack.clone()]).unwrap();

        sync(&entry, &data, None, &worlds).unwrap();

        assert!(datapack_path(&data, "saves/one", &pack.filename).is_file());
        assert!(!datapack_path(&data, "saves/two", &pack.filename).exists());
        assert!(
            managed_path(&entry, &pack).unwrap().is_file(),
            "managed copy stays"
        );
        std::fs::remove_dir_all(&entry).ok();
    }

    #[test]
    fn datapack_path_lives_inside_the_world() {
        assert_eq!(
            datapack_path(Path::new("/data"), "saves/my-world", "terralith.zip"),
            Path::new("/data/saves/my-world/datapacks/terralith.zip")
        );
    }

    #[test]
    fn untracked_datapacks_are_reported_per_world() {
        let (entry, data) = temp_entry("dp-untracked");
        let pack = tracked(ContentKind::DataPack, "terralith.zip");
        install_tracked(&entry, &data, &pack);
        let stray = datapack_path(&data, "saves/one", "handmade.zip");
        std::fs::create_dir_all(stray.parent().unwrap()).unwrap();
        std::fs::write(&stray, "mine").unwrap();

        let found = untracked(
            &data,
            ContentKind::DataPack,
            std::slice::from_ref(&pack),
            &["saves/one".to_string()],
        );

        assert_eq!(found, vec!["one/handmade.zip".to_string()]);
        std::fs::remove_dir_all(&entry).ok();
    }
}
