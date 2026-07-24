//! Per-instance content profiles: named selections over the installed pool,
//! stored as `profiles.json` beside `content.json` in the entry root. A profile
//! is a selection, not a copy — members are pool filenames, and the launch-time
//! reconcile decides which managed files are mirrored into `data/`. An absent
//! or empty file is valid and means "no profiles, mirror everything".

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use proto::content::ContentKind;
use proto::instance::Profile;
use serde::{Deserialize, Serialize};

use crate::registry;

const FILE: &str = "profiles.json";

/// The name reserved by the launch override (`--profile none` = no profile).
const RESERVED: &str = "none";

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Stored {
    active: String,
    profiles: BTreeMap<String, Members>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Members {
    members: Vec<String>,
}

/// Whether a pool item of this kind can be a profile member. Datapacks are
/// world-of-record (never pool content); modpacks are not single-file installs.
pub(crate) fn selectable(kind: ContentKind) -> bool {
    matches!(
        kind,
        ContentKind::Mod | ContentKind::ResourcePack | ContentKind::Shader
    )
}

fn load_stored(entry_dir: &Path) -> Stored {
    registry::read_record(entry_dir, FILE).unwrap_or_default()
}

fn save_stored(entry_dir: &Path, stored: &Stored) -> Result<()> {
    registry::write_record(entry_dir, FILE, stored)
}

/// A profile's captured settings store (`<instance>/profiles/<name>/`). Its
/// existence *is* the captured flag — the disk is the registry.
pub(crate) fn store_dir(entry_dir: &Path, name: &str) -> std::path::PathBuf {
    entry_dir.join("profiles").join(name)
}

fn profile(entry_dir: &Path, name: &str, members: &Members) -> Profile {
    Profile {
        name: name.to_string(),
        members: members.members.clone(),
        captured: store_dir(entry_dir, name).is_dir(),
    }
}

/// The active profile name (empty = none) and every profile, sorted by name.
pub(crate) fn list(entry_dir: &Path) -> (String, Vec<Profile>) {
    let stored = load_stored(entry_dir);
    let profiles = stored
        .profiles
        .iter()
        .map(|(name, members)| profile(entry_dir, name, members))
        .collect();
    (stored.active, profiles)
}

fn validate_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        bail!(proto::error::ErrorInfo::FieldRequired {
            field: proto::error::Field::Name
        });
    }
    if name.eq_ignore_ascii_case(RESERVED) {
        bail!(proto::error::ErrorInfo::ReservedName {
            name: RESERVED.to_string()
        });
    }
    Ok(name)
}

fn find<'a>(stored: &'a Stored, name: &str) -> Option<(&'a str, &'a Members)> {
    stored
        .profiles
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(key, members)| (key.as_str(), members))
}

pub(crate) fn create(entry_dir: &Path, name: &str, members: Vec<String>) -> Result<Profile> {
    let name = validate_name(name)?;
    let mut stored = load_stored(entry_dir);
    if find(&stored, name).is_some() {
        bail!(proto::error::ErrorInfo::AlreadyExists {
            entry: proto::error::Nameable::Profile,
            name: name.to_string()
        });
    }
    let entry = Members { members };
    let created = profile(entry_dir, name, &entry);
    stored.profiles.insert(name.to_string(), entry);
    save_stored(entry_dir, &stored)?;
    Ok(created)
}

/// Removing the active profile clears the active selection.
pub(crate) fn remove(entry_dir: &Path, name: &str) -> Result<()> {
    let mut stored = load_stored(entry_dir);
    let Some((key, _)) = find(&stored, name) else {
        bail!(proto::error::ErrorInfo::ProfileNotFound {
            scope: proto::error::ProfileScope::Instance,
            name: name.to_string()
        });
    };
    let key = key.to_string();
    stored.profiles.remove(&key);
    if stored.active == key {
        stored.active = String::new();
    }
    let store = store_dir(entry_dir, &key);
    if store.is_dir() {
        std::fs::remove_dir_all(&store)
            .with_context(|| format!("cannot remove {}", store.display()))?;
    }
    save_stored(entry_dir, &stored)
}

pub(crate) fn rename(entry_dir: &Path, name: &str, new_name: &str) -> Result<Profile> {
    let new_name = validate_name(new_name)?;
    let mut stored = load_stored(entry_dir);
    let Some((key, _)) = find(&stored, name) else {
        bail!(proto::error::ErrorInfo::ProfileNotFound {
            scope: proto::error::ProfileScope::Instance,
            name: name.to_string()
        });
    };
    let key = key.to_string();
    if let Some((other, _)) = find(&stored, new_name) {
        if other != key {
            bail!(proto::error::ErrorInfo::AlreadyExists {
                entry: proto::error::Nameable::Profile,
                name: new_name.to_string()
            });
        }
    }
    let members = stored.profiles.remove(&key).expect("found above");
    let old_store = store_dir(entry_dir, &key);
    if old_store.is_dir() {
        std::fs::rename(&old_store, store_dir(entry_dir, new_name))
            .with_context(|| format!("cannot move {}", old_store.display()))?;
    }
    let renamed = profile(entry_dir, new_name, &members);
    stored.profiles.insert(new_name.to_string(), members);
    if stored.active == key {
        stored.active = new_name.to_string();
    }
    save_stored(entry_dir, &stored)?;
    Ok(renamed)
}

/// Set the active profile; an empty `name` clears it.
pub(crate) fn set_active(entry_dir: &Path, name: &str) -> Result<()> {
    let mut stored = load_stored(entry_dir);
    if name.is_empty() {
        stored.active = String::new();
        return save_stored(entry_dir, &stored);
    }
    let Some((key, _)) = find(&stored, name) else {
        bail!(proto::error::ErrorInfo::ProfileNotFound {
            scope: proto::error::ProfileScope::Instance,
            name: name.to_string()
        });
    };
    stored.active = key.to_string();
    save_stored(entry_dir, &stored)
}

/// Apply resolved member edits: `add`/`remove` are pool filenames the caller
/// already validated against the index.
pub(crate) fn edit(
    entry_dir: &Path,
    name: &str,
    add: &[String],
    remove: &[String],
) -> Result<Profile> {
    let mut stored = load_stored(entry_dir);
    let Some((key, _)) = find(&stored, name) else {
        bail!(proto::error::ErrorInfo::ProfileNotFound {
            scope: proto::error::ProfileScope::Instance,
            name: name.to_string()
        });
    };
    let key = key.to_string();
    let members = stored.profiles.get_mut(&key).expect("found above");
    for filename in add {
        if !members.members.contains(filename) {
            members.members.push(filename.clone());
        }
    }
    members
        .members
        .retain(|filename| !remove.contains(filename));
    let edited = profile(entry_dir, &key, members);
    save_stored(entry_dir, &stored)?;
    Ok(edited)
}

/// The profile a launch runs under: `requested` is the launch override
/// (`none` = no profile, empty = the active profile, else a profile name that
/// must exist). `None` means "no profile — mirror everything, global store".
pub(crate) fn resolve(entry_dir: &Path, requested: &str) -> Result<Option<Profile>> {
    let stored = load_stored(entry_dir);
    let name = match requested {
        RESERVED => return Ok(None),
        "" if stored.active.is_empty() => return Ok(None),
        "" => stored.active.clone(),
        other => other.to_string(),
    };
    let Some((key, members)) = find(&stored, &name) else {
        bail!(proto::error::ErrorInfo::ProfileNotFound {
            scope: proto::error::ProfileScope::Instance,
            name: name.to_string()
        });
    };
    Ok(Some(profile(entry_dir, key, members)))
}

/// The member set a launch reconciles against (see [`resolve`]).
#[cfg(test)]
pub(crate) fn selection(
    entry_dir: &Path,
    requested: &str,
) -> Result<Option<std::collections::HashSet<String>>> {
    Ok(resolve(entry_dir, requested)?.map(|p| p.members.into_iter().collect()))
}

/// Drop filenames that left the pool from every profile (content removal).
pub(crate) fn prune(entry_dir: &Path, filenames: &[String]) -> Result<()> {
    if filenames.is_empty() {
        return Ok(());
    }
    let mut stored = load_stored(entry_dir);
    let mut changed = false;
    for members in stored.profiles.values_mut() {
        let before = members.members.len();
        members
            .members
            .retain(|filename| !filenames.contains(filename));
        changed |= members.members.len() != before;
    }
    if changed {
        save_stored(entry_dir, &stored)?;
    }
    Ok(())
}

/// Follow a pool item's filename change (a content update) in every profile.
pub(crate) fn remap(entry_dir: &Path, old: &str, new: &str) -> Result<()> {
    if old == new {
        return Ok(());
    }
    let mut stored = load_stored(entry_dir);
    let mut changed = false;
    for members in stored.profiles.values_mut() {
        for filename in members.members.iter_mut() {
            if filename == old {
                *filename = new.to_string();
                changed = true;
            }
        }
    }
    if changed {
        save_stored(entry_dir, &stored)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "hestia-profiles-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn absent_file_means_no_profiles() {
        let dir = temp_dir("absent");
        let (active, profiles) = list(&dir);
        assert!(active.is_empty());
        assert!(profiles.is_empty());
        assert_eq!(selection(&dir, "").unwrap(), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_use_and_remove_round_trip() {
        let dir = temp_dir("crud");
        create(&dir, "perf", vec!["sodium.jar".to_string()]).unwrap();
        set_active(&dir, "perf").unwrap();
        let (active, profiles) = list(&dir);
        assert_eq!(active, "perf");
        assert_eq!(profiles.len(), 1);

        remove(&dir, "perf").unwrap();
        let (active, profiles) = list(&dir);
        assert!(active.is_empty(), "removing the active profile clears it");
        assert!(profiles.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn names_are_unique_case_insensitively_and_none_is_reserved() {
        let dir = temp_dir("names");
        create(&dir, "Perf", vec![]).unwrap();
        assert!(create(&dir, "perf", vec![]).is_err());
        assert!(create(&dir, "none", vec![]).is_err());
        assert!(create(&dir, "NONE", vec![]).is_err());
        assert!(create(&dir, "  ", vec![]).is_err());
        assert!(rename(&dir, "Perf", "none").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rename_follows_the_active_pointer() {
        let dir = temp_dir("rename");
        create(&dir, "perf", vec!["a.jar".to_string()]).unwrap();
        set_active(&dir, "perf").unwrap();
        let renamed = rename(&dir, "perf", "speed").unwrap();
        assert_eq!(renamed.name, "speed");
        assert_eq!(renamed.members, vec!["a.jar".to_string()]);
        let (active, _) = list(&dir);
        assert_eq!(active, "speed");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn selection_resolves_override_active_and_none() {
        let dir = temp_dir("selection");
        create(&dir, "perf", vec!["sodium.jar".to_string()]).unwrap();
        create(&dir, "vanilla-ish", vec![]).unwrap();
        set_active(&dir, "perf").unwrap();

        let active = selection(&dir, "").unwrap().unwrap();
        assert!(active.contains("sodium.jar"));
        let named = selection(&dir, "vanilla-ish").unwrap().unwrap();
        assert!(named.is_empty());
        assert_eq!(selection(&dir, "none").unwrap(), None);
        assert!(selection(&dir, "ghost").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn prune_and_remap_touch_every_profile() {
        let dir = temp_dir("prune");
        create(&dir, "a", vec!["x.jar".to_string(), "y.jar".to_string()]).unwrap();
        create(&dir, "b", vec!["x.jar".to_string()]).unwrap();

        remap(&dir, "x.jar", "x2.jar").unwrap();
        let (_, profiles) = list(&dir);
        assert!(profiles
            .iter()
            .all(|p| p.members.contains(&"x2.jar".to_string())));

        prune(&dir, &["x2.jar".to_string()]).unwrap();
        let (_, profiles) = list(&dir);
        assert!(profiles
            .iter()
            .all(|p| !p.members.contains(&"x2.jar".to_string())));
        assert!(profiles[0].members.contains(&"y.jar".to_string()));
        std::fs::remove_dir_all(&dir).ok();
    }
}
