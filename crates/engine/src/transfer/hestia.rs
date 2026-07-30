//! hestia's own archive format — the full-fidelity one.
//!
//! It is the entry directory itself: the content pool, the profiles, the
//! modpack record, the game directory, everything but what regenerates. The
//! record travels beside them in `hestia.instance.json`, which is both the
//! manifest and the marker — and carries a **resolved** profile, so importing
//! one needs no network and cannot be broken by a version falling out of a
//! catalogue years later.
//!
//! The marker is not called `instance.json` deliberately. A marker file is a
//! claim about what an archive is, so it has to be specific enough that finding
//! it inside someone else's zip means something.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use proto::error::ErrorInfo;
use proto::transfer::{ExportFormat, ImportFormat};
use serde::{Deserialize, Serialize};

use super::archive::{self, Reader, Writer, Written};
use super::exclude::Rules;
use super::{Blueprint, Descriptor, Format, Landed, Recipe, Source, Target};
use crate::cancel::Job;
use crate::instances::InstanceRecord;
use crate::registry;

pub(crate) const MANIFEST: &str = "hestia.instance.json";

/// The manifest schema. Bumped only when a reader could no longer make sense of
/// an older archive — additive fields do not need it, since every field decodes
/// through `#[serde(default)]`.
pub(crate) const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Manifest {
    pub(crate) format_version: u32,
    pub(crate) exported_unix: i64,
    /// Which build wrote it — the first thing worth knowing about an archive
    /// that will not read back.
    pub(crate) exported_by: String,
    /// The exported instance's record. Its `id` is **not** honoured on import:
    /// an imported instance is a new entry and gets a fresh one. It is carried
    /// so an archive can be traced back to what produced it.
    pub(crate) instance: InstanceRecord,
}

pub(crate) struct Hestia;

impl Format for Hestia {
    fn id(&self) -> ImportFormat {
        ImportFormat::Hestia
    }

    fn marker(&self) -> &'static str {
        MANIFEST
    }

    fn read(&self, reader: &mut Reader, prefix: &str) -> Result<Blueprint> {
        let manifest = manifest(reader, prefix)?;
        let profile = &manifest.instance.profile;
        let descriptor = Descriptor {
            name: manifest.instance.name.clone(),
            game_version: profile.game_version.clone(),
            loader: super::loader_of(profile),
            loader_version: profile.loader_version.clone().unwrap_or_default(),
        };
        Ok(Blueprint {
            descriptor,
            recipe: Recipe::Record(Box::new(manifest.instance)),
        })
    }

    fn land(
        &self,
        reader: &mut Reader,
        prefix: &str,
        target: &Target<'_>,
        job: &Job<'_>,
    ) -> Result<Landed> {
        // Everything but the manifest, whose content is already the record the
        // instance was registered with.
        let files = reader.extract_under(prefix, target.entry_dir, job, &|relative| {
            relative != MANIFEST
        })?;
        Ok(Landed {
            files,
            warnings: Vec::new(),
        })
    }
}

fn manifest(reader: &mut Reader, prefix: &str) -> Result<Manifest> {
    let text = reader
        .read_text(&format!("{prefix}{MANIFEST}"))
        .map_err(|e| invalid(e.to_string()))?;
    let manifest: Manifest = serde_json::from_str(&text)
        .map_err(|e| invalid(format!("{MANIFEST} is malformed: {e}")))?;
    if manifest.format_version > FORMAT_VERSION {
        return Err(ErrorInfo::ArchiveUnsupported {
            format: "hestia".to_string(),
            component: format!("archive format {}", manifest.format_version),
        }
        .into());
    }
    if manifest.instance.profile.game_version.is_empty() {
        return Err(invalid(format!("{MANIFEST} pins no Minecraft version")).into());
    }
    Ok(manifest)
}

fn invalid(detail: String) -> ErrorInfo {
    ErrorInfo::ArchiveInvalid {
        format: "hestia".to_string(),
        detail,
    }
}

/// The path an export writes to when the caller named none: the instance's
/// slug and the time it was taken, so repeated exports never collide.
pub(crate) fn default_destination(
    dir: &Path,
    record: &InstanceRecord,
    format: ExportFormat,
) -> PathBuf {
    let slug = registry::dir_name(&record.id, &record.name);
    let stamp = registry::utc_stamp(registry::now_unix());
    dir.join(format!("{slug}-{stamp}.{}", format.extension()))
}

pub(crate) fn export(source: &Source<'_>, destination: &Path, job: &Job<'_>) -> Result<Written> {
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        exported_unix: registry::now_unix(),
        exported_by: format!("{} {}", common::app::NAME, common::app::VERSION),
        instance: source.record.clone(),
    };
    let encoded = serde_json::to_vec_pretty(&manifest).context("the manifest serializes")?;
    let rules = Rules::new(source.entry_dir, source.data_dir, source.exclude);

    let mut writer = Writer::create(destination)?;
    writer.add_bytes(MANIFEST, &encoded)?;
    writer.add_all(
        &archive::plan(source.entry_dir, "", &|relative| rules.keeps(relative)),
        job,
    )?;
    writer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::minecraft::InstanceProfile;

    fn temp(tag: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("hestia-native-{tag}-"))
            .tempdir()
            .expect("temp dir")
    }

    fn write(root: &Path, relative: &str, body: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    fn record() -> InstanceRecord {
        InstanceRecord {
            id: "0192abc".to_string(),
            name: "Cozy".to_string(),
            created_unix: 1_700_000_000,
            last_played_unix: Some(1_800_000_000),
            playtime_seconds: 3600,
            jvm: Default::default(),
            profile: InstanceProfile {
                flavor: "fabric".to_string(),
                game_version: "1.21.1".to_string(),
                loader_version: Some("0.16.9".to_string()),
                java_major: 21,
                ..Default::default()
            },
        }
    }

    fn job_for(cancel: &crate::cancel::Cancel) -> Job<'_> {
        static NOOP: fn(&proto::minecraft::ProvisionProgress) = |_| {};
        Job::new(&NOOP, cancel)
    }

    /// Export an instance tree, then read it back the way an import does.
    #[test]
    fn an_export_round_trips_through_the_manifest_and_the_tree() {
        let dir = temp("roundtrip");
        let entry_dir = dir.path().join("instances/cozy");
        let data_dir = entry_dir.join("data");
        write(&entry_dir, "content.json", r#"{"items":[]}"#);
        write(&entry_dir, "mods/sodium.jar", "jar bytes");
        write(&data_dir, "options.txt", "lang:en");
        write(&data_dir, "saves/world/level.dat", "nbt");
        write(&entry_dir, "logs/session-1.log", "noise");
        write(&data_dir, "crash-reports/crash.txt", "noise");

        let record = record();
        let archive_path = dir.path().join("cozy.hestia");
        let cancel = crate::cancel::Cancel::new();
        let job = job_for(&cancel);
        export(
            &Source {
                record: &record,
                entry_dir: &entry_dir,
                data_dir: &data_dir,
                exclude: &[],
            },
            &archive_path,
            &job,
        )
        .unwrap();

        let mut reader = Reader::open(&archive_path).unwrap();
        let blueprint = Hestia.read(&mut reader, "").unwrap();
        assert_eq!(blueprint.descriptor.name, "Cozy");
        assert_eq!(blueprint.descriptor.game_version, "1.21.1");
        assert_eq!(blueprint.descriptor.loader, "fabric");
        assert!(matches!(blueprint.recipe, Recipe::Record(_)));

        let restored = temp("restored");
        let target = Target {
            entry_dir: restored.path(),
            data_dir: &restored.path().join("data"),
        };
        Hestia.land(&mut reader, "", &target, &job).unwrap();

        assert!(restored.path().join("mods/sodium.jar").is_file());
        assert_eq!(
            std::fs::read_to_string(restored.path().join("data/options.txt")).unwrap(),
            "lang:en"
        );
        assert!(restored.path().join("data/saves/world/level.dat").is_file());
        assert!(
            !restored.path().join("logs").exists(),
            "session logs regenerate"
        );
        assert!(!restored.path().join("data/crash-reports").exists());
        assert!(
            !restored.path().join(MANIFEST).exists(),
            "the manifest is the record, not a file the instance keeps"
        );
        assert!(
            !restored.path().join("instance.json").exists(),
            "the record is written by the store when the instance is registered"
        );
    }

    #[test]
    fn a_newer_archive_format_is_refused_rather_than_guessed_at() {
        let dir = temp("newer");
        let archive_path = dir.path().join("future.hestia");
        let mut manifest = Manifest {
            format_version: FORMAT_VERSION + 1,
            instance: record(),
            ..Default::default()
        };
        {
            let mut writer = Writer::create(&archive_path).unwrap();
            writer
                .add_bytes(MANIFEST, &serde_json::to_vec(&manifest).unwrap())
                .unwrap();
            writer.finish().unwrap();
        }
        let mut reader = Reader::open(&archive_path).unwrap();
        assert!(Hestia.read(&mut reader, "").is_err());

        manifest.format_version = FORMAT_VERSION;
        manifest.instance.profile.game_version = String::new();
        let broken = dir.path().join("broken.hestia");
        {
            let mut writer = Writer::create(&broken).unwrap();
            writer
                .add_bytes(MANIFEST, &serde_json::to_vec(&manifest).unwrap())
                .unwrap();
            writer.finish().unwrap();
        }
        assert!(Hestia
            .read(&mut Reader::open(&broken).unwrap(), "")
            .is_err());
    }

    #[test]
    fn the_generated_name_carries_the_slug_and_the_format() {
        let path = default_destination(Path::new("/exports"), &record(), ExportFormat::Hestia);
        assert!(path.starts_with("/exports"));
        assert!(path.to_string_lossy().contains("cozy"));
        assert_eq!(path.extension().unwrap(), "hestia");
    }
}
