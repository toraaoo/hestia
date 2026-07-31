//! The global content-profile store: data-home-level project reference lists
//! under `<data_home>/profiles/<name>.json`, each file an object holding
//! `{source, project_id, slug}` entries — the disk is the registry, as with
//! `java` and `backups`. A profile stores references, never jars: applying one
//! resolves each reference against the target instance's game version and
//! loader.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::profile::{GlobalProfile, ProfileEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::{self, Document, Step};

/// One profile file. The name is the file's, not the document's, so it is not a
/// field here — two sources of truth for a name is one too many.
#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Stored {
    entries: Vec<ProfileEntry>,
}

impl Document for Stored {
    const NAME: &'static str = "a global profile";
    const MIGRATIONS: &'static [Step] = &[lift_bare_array];
}

/// v1 → v2: the file was a bare JSON array, which has nowhere to carry a schema
/// stamp. Wrap it in an object so it can say what it is from here on.
fn lift_bare_array(value: &mut Value) -> anyhow::Result<()> {
    if let Some(entries) = value.as_array() {
        *value = serde_json::json!({ "entries": entries });
    }
    Ok(())
}

pub struct Profiles {
    dir: Mutex<PathBuf>,
}

impl Profiles {
    pub fn new(dir: PathBuf) -> Self {
        Profiles {
            dir: Mutex::new(dir),
        }
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.dir.lock().unwrap() = dir;
    }

    fn dir(&self) -> PathBuf {
        self.dir.lock().unwrap().clone()
    }

    pub fn list(&self) -> Vec<GlobalProfile> {
        let mut profiles: Vec<GlobalProfile> = std::fs::read_dir(self.dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let name = path.file_stem()?.to_str()?.to_string();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    return None;
                }
                let stored: Stored = schema::load(&path)?;
                Some(GlobalProfile {
                    name,
                    entries: stored.entries,
                })
            })
            .collect();
        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        profiles
    }

    pub fn get(&self, name: &str) -> Result<GlobalProfile> {
        let name = canonical_name(name)?;
        self.list()
            .into_iter()
            .find(|p| p.name == name)
            .with_context(|| format!("no global profile named '{name}'"))
    }

    pub fn create(&self, name: &str) -> Result<GlobalProfile> {
        let name = canonical_name(name)?;
        let dir = self.dir();
        let path = dir.join(format!("{name}.json"));
        if path.exists() {
            bail!(proto::error::ErrorInfo::AlreadyExists {
                entry: proto::error::Nameable::GlobalProfile,
                name: name.to_string()
            });
        }
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create {}", dir.display()))?;
        write_entries(&path, &[])?;
        tracing::info!(profile = %name, "global profile created");
        Ok(GlobalProfile {
            name,
            entries: Vec::new(),
        })
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let name = canonical_name(name)?;
        let path = self.dir().join(format!("{name}.json"));
        if !path.is_file() {
            bail!(proto::error::ErrorInfo::ProfileNotFound {
                scope: proto::error::ProfileScope::Global,
                name: name.to_string()
            });
        }
        std::fs::remove_file(&path).with_context(|| format!("cannot remove {}", path.display()))?;
        tracing::info!(profile = %name, "global profile removed");
        Ok(())
    }

    pub fn save(&self, name: &str, entries: &[ProfileEntry]) -> Result<GlobalProfile> {
        let name = canonical_name(name)?;
        let path = self.dir().join(format!("{name}.json"));
        if !path.is_file() {
            bail!(proto::error::ErrorInfo::ProfileNotFound {
                scope: proto::error::ProfileScope::Global,
                name: name.to_string()
            });
        }
        write_entries(&path, entries)?;
        Ok(GlobalProfile {
            name,
            entries: entries.to_vec(),
        })
    }
}

/// A profile's name doubles as its filename, so it is slugged like an entry
/// name — `My QoL`, `my-qol`, and `MY QOL` all reach the one profile `my-qol`.
fn canonical_name(name: &str) -> Result<String> {
    proto::naming::slugify(name)
        .with_context(|| format!("profile name '{name}' has no usable characters"))
}

fn write_entries(path: &Path, entries: &[ProfileEntry]) -> Result<()> {
    schema::save(
        path,
        &Stored {
            entries: entries.to_vec(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(tag: &str) -> (Profiles, tempfile::TempDir) {
        let dir = tempfile::Builder::new()
            .prefix(&format!("hestia-global-profiles-{tag}-"))
            .tempdir()
            .expect("temp dir");
        (Profiles::new(dir.path().join("profiles")), dir)
    }

    #[test]
    fn create_edit_and_remove_round_trip() {
        let (store, _dir) = store("crud");
        assert!(store.list().is_empty());
        store.create("My QoL").unwrap();
        assert!(
            store.create("my-qol").is_err(),
            "names collapse to the slug"
        );

        let entries = vec![ProfileEntry {
            source: "modrinth".to_string(),
            project_id: "AANobbMI".to_string(),
            slug: "sodium".to_string(),
        }];
        let saved = store.save("MY QOL", &entries).unwrap();
        assert_eq!(saved.name, "my-qol");
        assert_eq!(store.get("my-qol").unwrap().entries.len(), 1);

        store.remove("my-qol").unwrap();
        assert!(store.get("my-qol").is_err());
    }

    #[test]
    fn a_profile_written_as_a_bare_array_is_lifted_into_an_object() {
        let (store, _dir) = store("lift");
        let dir = store.dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("legacy.json"),
            r#"[{"source":"modrinth","projectId":"AANobbMI","slug":"sodium"}]"#,
        )
        .unwrap();

        let profile = store.get("legacy").unwrap();

        assert_eq!(profile.entries.len(), 1);
        assert_eq!(profile.entries[0].slug, "sodium");
        let raw: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("legacy.json")).unwrap())
                .unwrap();
        assert_eq!(
            raw[schema::FIELD],
            Value::from(2),
            "the lift was written back, so it happens once"
        );
        assert_eq!(raw["entries"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn unknown_profiles_error() {
        let (store, _dir) = store("unknown");
        assert!(store.get("ghost").is_err());
        assert!(store.remove("ghost").is_err());
        assert!(store.save("ghost", &[]).is_err());
        assert!(store.create("!!!").is_err());
    }
}
