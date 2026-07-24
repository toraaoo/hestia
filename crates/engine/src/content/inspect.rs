//! Classify a local file for import by reading its zip central directory.

use std::fs::File;
use std::path::Path;

use anyhow::{bail, Context, Result};
use proto::content::ContentKind;
use zip::ZipArchive;

// Loader-agnostic: a new flavor adds its manifest name here.
const MOD_MANIFESTS: [&str; 5] = [
    "fabric.mod.json",
    "quilt.mod.json",
    "META-INF/mods.toml",
    "META-INF/neoforge.mods.toml",
    "mcmod.info",
];

pub(crate) enum Detected {
    Kind(ContentKind),
    Unknown,
}

/// Errors only when the file is not a readable archive or is a modpack — both
/// un-installable as single-file content. An unrecognised but valid zip is
/// [`Detected::Unknown`].
pub(crate) fn classify(path: &Path) -> Result<Detected> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let archive = ZipArchive::new(file).with_context(|| {
        format!(
            "'{}' is not a readable .jar or .zip archive",
            file_name(path)
        )
    })?;
    let names: Vec<&str> = archive.file_names().collect();
    let has = |entry: &str| names.contains(&entry);
    let under = |dir: &str| names.iter().any(|n| n.starts_with(dir));

    if has("modrinth.index.json") {
        bail!(
            "'{}' is a modpack; install it when creating an instance, not as a single file",
            file_name(path)
        );
    }
    if MOD_MANIFESTS.iter().any(|m| has(m)) {
        return Ok(Detected::Kind(ContentKind::Mod));
    }
    if has("pack.mcmeta") {
        // A datapack carries `data/`, a resourcepack `assets/`; both have pack.mcmeta.
        return Ok(Detected::Kind(if under("data/") && !under("assets/") {
            ContentKind::DataPack
        } else {
            ContentKind::ResourcePack
        }));
    }
    if under("shaders/") {
        return Ok(Detected::Kind(ContentKind::Shader));
    }
    Ok(Detected::Unknown)
}

fn file_name(path: &Path) -> std::borrow::Cow<'_, str> {
    path.file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;

    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use super::*;

    fn archive(tag: &str, entries: &[&str]) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("hestia-inspect-{}-{}.zip", tag, std::process::id()));
        let file = File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        for entry in entries {
            if entry.ends_with('/') {
                zip.add_directory(*entry, SimpleFileOptions::default())
                    .unwrap();
            } else {
                zip.start_file(*entry, SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(b"x").unwrap();
            }
        }
        zip.finish().unwrap();
        path
    }

    fn kind(tag: &str, entries: &[&str]) -> Option<ContentKind> {
        let path = archive(tag, entries);
        let out = classify(&path).ok();
        std::fs::remove_file(&path).ok();
        match out {
            Some(Detected::Kind(k)) => Some(k),
            _ => None,
        }
    }

    #[test]
    fn detects_kinds_loader_agnostically() {
        assert_eq!(kind("fabric", &["fabric.mod.json"]), Some(ContentKind::Mod));
        assert_eq!(
            kind("forge", &["META-INF/mods.toml"]),
            Some(ContentKind::Mod)
        );
        assert_eq!(
            kind("neoforge", &["META-INF/neoforge.mods.toml"]),
            Some(ContentKind::Mod)
        );
        assert_eq!(
            kind("resource", &["pack.mcmeta", "assets/minecraft/"]),
            Some(ContentKind::ResourcePack)
        );
        assert_eq!(
            kind("datapack", &["pack.mcmeta", "data/minecraft/"]),
            Some(ContentKind::DataPack)
        );
        assert_eq!(
            kind("shader", &["shaders/world0/gbuffers_basic.vsh"]),
            Some(ContentKind::Shader)
        );
    }

    #[test]
    fn unknown_archive_is_valid_but_unclassified() {
        let path = archive("unknown", &["random.txt"]);
        assert!(matches!(classify(&path), Ok(Detected::Unknown)));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn modpack_and_non_archive_are_rejected() {
        let pack = archive("modpack", &["modrinth.index.json"]);
        assert!(classify(&pack).is_err());
        std::fs::remove_file(&pack).ok();

        let junk =
            std::env::temp_dir().join(format!("hestia-inspect-junk-{}.jar", std::process::id()));
        std::fs::write(&junk, b"not a zip").unwrap();
        assert!(classify(&junk).is_err());
        std::fs::remove_file(&junk).ok();
    }
}
