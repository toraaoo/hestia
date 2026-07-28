//! The `.mrpack` archive format: a zip carrying `modrinth.index.json` (the
//! manifest of files to fetch) beside the `overrides/`, `client-overrides/` and
//! `server-overrides/` trees the pack writes straight into the game directory.
//!
//! Deliberately platform-agnostic. A pack picked off disk has no source, and the
//! provider that serves packs over HTTP parses the same bytes — so the format
//! lives here rather than inside `modrinth.rs`, which keeps only the API calls
//! that fetch it.

use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::content::{ModpackFile, ResolvedModpack, SideSupport};
use proto::download::{Checksum, HashAlgorithm};
use proto::minecraft::Artifact;
use serde_json::{Map, Value};

use crate::cancel::Job;
use crate::checksum::Hasher;

const INDEX: &str = "modrinth.index.json";

/// The dependency keys that name a modloader, newest-preferred order. The loader
/// name is the key with any `-loader` suffix stripped.
const LOADER_KEYS: [&str; 4] = ["fabric-loader", "quilt-loader", "neoforge", "forge"];

/// Which side an install is for — which of the two side-specific override trees
/// applies, and which `env` field decides whether a file is wanted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Client,
    Server,
}

impl Side {
    /// The override tree that belongs to this side alone. The shared
    /// `overrides/` tree applies to both and is written first, so a side tree
    /// wins where the two name the same path.
    fn overrides_dir(self) -> &'static str {
        match self {
            Side::Client => "client-overrides",
            Side::Server => "server-overrides",
        }
    }

    /// Whether the pack wants this file on this side. A file the pack marks
    /// unsupported for the side is not an error — a client-only shader in a
    /// pack installed on a server is simply not that server's file.
    pub fn wants(self, file: &ModpackFile) -> bool {
        let support = match self {
            Side::Client => file.client,
            Side::Server => file.server,
        };
        support != SideSupport::Unsupported
    }
}

/// One file written out of an override tree.
pub struct WrittenOverride {
    /// Path relative to the game directory.
    pub path: String,
    pub sha1: String,
}

/// An opened `.mrpack`, read from memory. Pack archives are indexes and configs
/// — references rather than embedded jars — so holding one in memory is cheap;
/// each override is streamed to disk individually rather than all at once.
pub struct Archive {
    zip: zip::ZipArchive<Cursor<Vec<u8>>>,
}

impl Archive {
    pub fn open(bytes: Vec<u8>) -> Result<Self> {
        let zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| {
            anyhow::Error::from(proto::error::ErrorInfo::ModpackInvalid {
                detail: format!("not a valid archive: {e}"),
            })
        })?;
        Ok(Archive { zip })
    }

    pub fn read(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("cannot read modpack {}", path.display()))?;
        Archive::open(bytes)
    }

    /// The pack's manifest. Ids are left empty — only the platform that served
    /// the archive knows them, and a pack read off disk has none.
    pub fn index(&mut self) -> Result<ResolvedModpack> {
        let entry = self.zip.by_name(INDEX).map_err(|_| {
            anyhow::Error::from(proto::error::ErrorInfo::ModpackInvalid {
                detail: format!("{INDEX} is missing from the archive"),
            })
        })?;
        let index: Value = serde_json::from_reader(entry).map_err(|e| {
            anyhow::Error::from(proto::error::ErrorInfo::ModpackInvalid {
                detail: format!("{INDEX} is malformed: {e}"),
            })
        })?;
        parse_index(&index)
    }

    /// Write the pack's own game-directory files for `side` into `dest`,
    /// returning each with the hash it was written with. The shared `overrides/`
    /// tree goes first so the side-specific tree wins where both name a path —
    /// which is what having two trees means.
    ///
    /// `keep` is asked before each file is written; answering false leaves what
    /// is on disk untouched and omits it from the result.
    pub fn extract_overrides(
        &mut self,
        side: Side,
        dest: &Path,
        job: &Job,
        mut keep: impl FnMut(&str) -> bool,
    ) -> Result<Vec<WrittenOverride>> {
        let mut planned: Vec<(usize, String)> = Vec::new();
        for prefix in ["overrides", side.overrides_dir()] {
            for i in 0..self.zip.len() {
                let entry = self
                    .zip
                    .by_index(i)
                    .context("cannot read a modpack archive entry")?;
                if entry.is_dir() {
                    continue;
                }
                let Some(relative) = entry.name().strip_prefix(prefix) else {
                    continue;
                };
                let relative = relative.trim_start_matches('/');
                if relative.is_empty() {
                    continue;
                }
                if !is_safe_path(relative) {
                    bail!(proto::error::ErrorInfo::ModpackInvalid {
                        detail: format!("unsafe override path: {}", entry.name())
                    });
                }
                planned.push((i, relative.to_string()));
            }
        }

        let total = planned.len() as u64;
        let mut written = Vec::new();
        for (done, (index, relative)) in planned.into_iter().enumerate() {
            job.check()?;
            job.report(&proto::minecraft::ProvisionProgress {
                phase: proto::minecraft::ProvisionPhase::Overrides,
                current: done as u64,
                total,
                detail: relative.clone(),
                ..Default::default()
            });
            if !keep(&relative) {
                continue;
            }
            let target = dest.join(&relative);
            let sha1 = self.write_entry(index, &target)?;
            written.push(WrittenOverride {
                path: relative,
                sha1,
            });
        }
        Ok(written)
    }

    /// Stream one archive entry to `target`, hashing as it goes. Writes through
    /// a temp file and renames, so a cancelled or failed extraction never leaves
    /// a half-written config the game would then read.
    fn write_entry(&mut self, index: usize, target: &Path) -> Result<String> {
        let mut entry = self
            .zip
            .by_index(index)
            .context("cannot read a modpack archive entry")?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        let temp = temp_path(target);
        let mut hasher = Hasher::new(HashAlgorithm::Sha1);
        {
            let mut out = std::fs::File::create(&temp)
                .with_context(|| format!("cannot write {}", temp.display()))?;
            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = entry.read(&mut buf).context("cannot read modpack entry")?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                std::io::Write::write_all(&mut out, &buf[..n])
                    .with_context(|| format!("cannot write {}", temp.display()))?;
            }
        }
        std::fs::rename(&temp, target)
            .with_context(|| format!("cannot place {}", target.display()))?;
        Ok(hasher.hex_digest())
    }
}

fn temp_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    target.with_file_name(name)
}

/// Parse a `modrinth.index.json` into a resolved pack. Pure — the source and the
/// project/version ids come from whoever served the archive, not the index.
/// Rejects an unsupported format version, a file with an unsafe (absolute or
/// parent-escaping) path, and a manifest that pins no Minecraft version.
pub fn parse_index(index: &Value) -> Result<ResolvedModpack> {
    let format = index
        .get("formatVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if format != 1 {
        bail!(proto::error::ErrorInfo::ModpackInvalid {
            detail: format!("unsupported format version: {format} (expected 1)")
        });
    }

    let mut files = Vec::new();
    if let Some(arr) = index.get("files").and_then(Value::as_array) {
        for f in arr {
            let path = f.get("path").and_then(Value::as_str).unwrap_or_default();
            if !is_safe_path(path) {
                bail!(proto::error::ErrorInfo::ModpackInvalid {
                    detail: format!("unsafe file path: {path}")
                });
            }
            let url = f
                .get("downloads")
                .and_then(Value::as_array)
                .and_then(|d| d.first())
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let size = f.get("fileSize").and_then(Value::as_u64).unwrap_or(0);
            let sha1 = f
                .get("hashes")
                .and_then(|h| h.get("sha1"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let env = f.get("env");
            // A missing `env` means the file applies to both sides (the spec).
            let client = env
                .and_then(|e| e.get("client"))
                .and_then(Value::as_str)
                .map(parse_side)
                .unwrap_or(SideSupport::Required);
            let server = env
                .and_then(|e| e.get("server"))
                .and_then(Value::as_str)
                .map(parse_side)
                .unwrap_or(SideSupport::Required);
            files.push(ModpackFile {
                path: path.to_string(),
                artifact: Artifact {
                    url,
                    filename: filename_of(path),
                    size,
                    checksum: (!sha1.is_empty()).then_some(Checksum {
                        algorithm: HashAlgorithm::Sha1,
                        hex: sha1,
                    }),
                },
                client,
                server,
            });
        }
    }

    let deps = index.get("dependencies").and_then(Value::as_object);
    let game_version = deps
        .and_then(|d| d.get("minecraft"))
        .and_then(Value::as_str)
        .ok_or_else(|| proto::error::ErrorInfo::ModpackInvalid {
            detail: "it pins no Minecraft version".to_string(),
        })?
        .to_string();
    let (loader, loader_version) = deps
        .and_then(find_loader)
        .map(|(l, v)| (Some(l), Some(v)))
        .unwrap_or((None, None));

    Ok(ResolvedModpack {
        source: String::new(),
        project_id: String::new(),
        version_id: String::new(),
        version_number: str_field(index, "versionId"),
        name: str_field(index, "name"),
        summary: str_field(index, "summary"),
        game_version,
        loader,
        loader_version,
        files,
    })
}

fn find_loader(deps: &Map<String, Value>) -> Option<(String, String)> {
    for key in LOADER_KEYS {
        if let Some(version) = deps.get(key).and_then(Value::as_str) {
            let name = key.strip_suffix("-loader").unwrap_or(key).to_string();
            return Some((name, version.to_string()));
        }
    }
    None
}

fn parse_side(s: &str) -> SideSupport {
    match s {
        "required" => SideSupport::Required,
        "optional" => SideSupport::Optional,
        "unsupported" => SideSupport::Unsupported,
        _ => SideSupport::Unknown,
    }
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn filename_of(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// A relative path that stays inside the game directory: not empty, not
/// absolute, and with no parent (`..`), root, or drive-prefix components.
pub fn is_safe_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let p = Path::new(path);
    if p.is_absolute() {
        return false;
    }
    p.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn index_maps_files_loader_and_game_version() {
        let pack = parse_index(&json!({
            "formatVersion": 1,
            "name": "Cozy Pack",
            "versionId": "1.4.0",
            "summary": "warm",
            "files": [{
                "path": "mods/sodium.jar",
                "downloads": ["https://cdn.modrinth.com/data/AAA/versions/BBB/sodium.jar"],
                "fileSize": 12,
                "hashes": {"sha1": "abc"},
            }],
            "dependencies": {"minecraft": "1.21.1", "fabric-loader": "0.16.9"},
        }))
        .unwrap();

        assert_eq!(pack.name, "Cozy Pack");
        assert_eq!(pack.version_number, "1.4.0");
        assert_eq!(pack.game_version, "1.21.1");
        assert_eq!(pack.loader.as_deref(), Some("fabric"));
        assert_eq!(pack.loader_version.as_deref(), Some("0.16.9"));
        assert_eq!(pack.files[0].artifact.filename, "sodium.jar");
        assert_eq!(pack.files[0].client, SideSupport::Required);
    }

    #[test]
    fn a_missing_env_means_both_sides_want_the_file() {
        let pack = parse_index(&json!({
            "formatVersion": 1,
            "files": [
                {"path": "mods/shared.jar", "downloads": ["https://x/y.jar"]},
                {"path": "mods/client.jar", "downloads": ["https://x/z.jar"],
                 "env": {"client": "required", "server": "unsupported"}},
            ],
            "dependencies": {"minecraft": "1.21.1"},
        }))
        .unwrap();

        assert!(Side::Server.wants(&pack.files[0]));
        assert!(Side::Client.wants(&pack.files[0]));
        assert!(Side::Client.wants(&pack.files[1]));
        assert!(
            !Side::Server.wants(&pack.files[1]),
            "a client-unsupported file is not the server's"
        );
    }

    #[test]
    fn a_pack_with_no_loader_is_vanilla() {
        let pack = parse_index(&json!({
            "formatVersion": 1,
            "dependencies": {"minecraft": "1.21.1"},
        }))
        .unwrap();
        assert_eq!(pack.loader, None);
    }

    #[test]
    fn unsupported_format_and_unsafe_paths_are_rejected() {
        assert!(parse_index(&json!({"formatVersion": 2})).is_err());
        assert!(parse_index(&json!({
            "formatVersion": 1,
            "files": [{"path": "../escape.jar", "downloads": ["https://x/y"]}],
            "dependencies": {"minecraft": "1.21.1"},
        }))
        .is_err());
        assert!(parse_index(&json!({"formatVersion": 1, "dependencies": {}})).is_err());
    }

    /// Build a `.mrpack` in memory from `(entry name, body)` pairs.
    fn archive(entries: &[(&str, &str)]) -> Archive {
        use std::io::Write;
        let mut buffer = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut buffer));
            for (name, body) in entries {
                zip.start_file::<_, ()>(*name, Default::default()).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        Archive::open(buffer).unwrap()
    }

    fn temp(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hestia-mrpack-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn extract(archive: &mut Archive, side: Side, dest: &Path) -> Vec<WrittenOverride> {
        let cancel = crate::cancel::Cancel::new();
        let noop = |_: &proto::minecraft::ProvisionProgress| {};
        let job = Job::new(&noop, &cancel);
        archive
            .extract_overrides(side, dest, &job, |_| true)
            .unwrap()
    }

    #[test]
    fn a_side_takes_the_shared_tree_and_its_own() {
        let mut pack = archive(&[
            ("modrinth.index.json", "{}"),
            ("overrides/config/shared.toml", "shared"),
            ("client-overrides/options.txt", "client"),
            ("server-overrides/server-icon.png", "server"),
        ]);
        let dest = temp("sides");

        let written = extract(&mut pack, Side::Client, &dest);

        assert!(dest.join("config/shared.toml").is_file(), "shared tree");
        assert!(dest.join("options.txt").is_file(), "the client's own tree");
        assert!(
            !dest.join("server-icon.png").exists(),
            "the other side's tree is not this side's"
        );
        assert_eq!(written.len(), 2);
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn the_server_tree_is_the_servers_own() {
        let mut pack = archive(&[
            ("overrides/config/shared.toml", "shared"),
            ("client-overrides/options.txt", "client"),
            ("server-overrides/server-icon.png", "server"),
        ]);
        let dest = temp("server-side");

        extract(&mut pack, Side::Server, &dest);

        assert!(dest.join("config/shared.toml").is_file());
        assert!(dest.join("server-icon.png").is_file());
        assert!(!dest.join("options.txt").exists());
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn a_side_tree_wins_where_both_name_the_same_path() {
        let mut pack = archive(&[
            ("overrides/config/cozy.toml", "shared value"),
            ("client-overrides/config/cozy.toml", "client value"),
        ]);
        let dest = temp("precedence");

        extract(&mut pack, Side::Client, &dest);

        assert_eq!(
            std::fs::read_to_string(dest.join("config/cozy.toml")).unwrap(),
            "client value",
            "the side tree is written after the shared one, so it wins"
        );
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn nested_directories_are_created_and_hashed() {
        let mut pack = archive(&[("overrides/config/deep/nested/file.json", "{}")]);
        let dest = temp("nested");

        let written = extract(&mut pack, Side::Client, &dest);

        assert!(dest.join("config/deep/nested/file.json").is_file());
        assert_eq!(written[0].path, "config/deep/nested/file.json");
        assert_eq!(
            written[0].sha1,
            crate::content::install::sha1_file(&dest.join("config/deep/nested/file.json")).unwrap(),
            "the recorded hash is what was written"
        );
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn refusing_a_file_leaves_what_is_there() {
        let mut pack = archive(&[
            ("overrides/config/mine.toml", "the pack's"),
            ("overrides/config/theirs.toml", "the pack's"),
        ]);
        let dest = temp("keep");
        std::fs::create_dir_all(dest.join("config")).unwrap();
        std::fs::write(dest.join("config/mine.toml"), "my edit").unwrap();

        let cancel = crate::cancel::Cancel::new();
        let noop = |_: &proto::minecraft::ProvisionProgress| {};
        let job = Job::new(&noop, &cancel);
        let written = pack
            .extract_overrides(Side::Client, &dest, &job, |path| path != "config/mine.toml")
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(dest.join("config/mine.toml")).unwrap(),
            "my edit",
            "a refused file is untouched"
        );
        assert_eq!(written.len(), 1, "and is not reported as written");
        assert_eq!(written[0].path, "config/theirs.toml");
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn an_override_escaping_the_game_directory_is_refused() {
        let mut pack = archive(&[("overrides/../../escape.txt", "no")]);
        let dest = temp("escape");
        let cancel = crate::cancel::Cancel::new();
        let noop = |_: &proto::minecraft::ProvisionProgress| {};
        let job = Job::new(&noop, &cancel);

        assert!(pack
            .extract_overrides(Side::Client, &dest, &job, |_| true)
            .is_err());
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn a_pack_with_no_override_trees_writes_nothing() {
        let mut pack = archive(&[("modrinth.index.json", "{}")]);
        let dest = temp("empty");
        assert!(extract(&mut pack, Side::Client, &dest).is_empty());
        std::fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn safe_paths_stay_inside_the_game_directory() {
        assert!(is_safe_path("mods/sodium.jar"));
        assert!(is_safe_path("config/nested/file.toml"));
        assert!(!is_safe_path(""));
        assert!(!is_safe_path("/etc/passwd"));
        assert!(!is_safe_path("../../escape"));
        assert!(!is_safe_path("mods/../../escape"));
    }
}
