//! Content profile management: per-instance profile CRUD over the
//! `profiles.json` store (members validated against the instance's installed
//! pool), and the global-profile flows — reference edits resolved through the
//! content registry, and the one-shot apply that installs a global profile's
//! references into an instance's pool as ordinary tagged content.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use proto::content::{
    ContentAddItem, ContentAddSpec, ContentFailure, ContentKind, InstalledContent,
};
use proto::instance::Profile;
use proto::profile::{GlobalProfile, ProfileEntry};

use crate::content::{install, profiles};
use crate::engine::Engine;
use crate::minecraft::materialize::OnProgress;

impl Engine {
    /// The instance's active profile name (empty = none) and every profile.
    pub fn instance_profiles(&self, reference: &str) -> Result<(String, Vec<Profile>)> {
        Ok(profiles::list(&self.profile_dir(reference)?))
    }

    /// Create a profile — seeded with every selectable pool item, or empty.
    pub fn create_instance_profile(
        &self,
        reference: &str,
        name: &str,
        seed_from_pool: bool,
    ) -> Result<Profile> {
        let dir = self.profile_dir(reference)?;
        let members = if seed_from_pool {
            install::load(&dir)
                .into_iter()
                .filter(|item| profiles::selectable(item.kind))
                .map(|item| item.filename)
                .collect()
        } else {
            Vec::new()
        };
        let created = profiles::create(&dir, name, members)?;
        tracing::info!(instance = %reference, profile = %created.name, "profile created");
        Ok(created)
    }

    pub fn remove_instance_profile(&self, reference: &str, name: &str) -> Result<()> {
        profiles::remove(&self.profile_dir(reference)?, name)?;
        tracing::info!(instance = %reference, profile = %name, "profile removed");
        Ok(())
    }

    pub fn rename_instance_profile(
        &self,
        reference: &str,
        name: &str,
        new_name: &str,
    ) -> Result<Profile> {
        let renamed = profiles::rename(&self.profile_dir(reference)?, name, new_name)?;
        tracing::info!(instance = %reference, from = %name, to = %renamed.name, "profile renamed");
        Ok(renamed)
    }

    /// Set the active profile (empty clears it); applied at the next launch.
    pub fn use_instance_profile(&self, reference: &str, name: &str) -> Result<()> {
        profiles::set_active(&self.profile_dir(reference)?, name)?;
        tracing::info!(instance = %reference, profile = %name, "active profile changed");
        Ok(())
    }

    /// Add/remove members by pool reference (project id, slug, filename, or
    /// title). A reference that matches nothing — or only a datapack — errors.
    pub fn edit_instance_profile(
        &self,
        reference: &str,
        name: &str,
        add: &[String],
        remove: &[String],
    ) -> Result<Profile> {
        let dir = self.profile_dir(reference)?;
        let pool = install::load(&dir);
        let resolve = |reference: &String| -> Result<Vec<String>> {
            let matched: Vec<String> = pool
                .iter()
                .filter(|item| profiles::selectable(item.kind) && install::matches(item, reference))
                .map(|item| item.filename.clone())
                .collect();
            if matched.is_empty() {
                if pool.iter().any(|item| {
                    !profiles::selectable(item.kind) && install::matches(item, reference)
                }) {
                    bail!(
                        "'{reference}' is not selectable content (datapacks live in their world)"
                    );
                }
                bail!("no installed content matches '{reference}'");
            }
            Ok(matched)
        };
        let mut adds = Vec::new();
        for reference in add {
            adds.extend(resolve(reference)?);
        }
        let mut removes = Vec::new();
        for reference in remove {
            removes.extend(resolve(reference)?);
        }
        profiles::edit(&dir, name, &adds, &removes)
    }

    /// Capture the profile's own settings store, seeded from the global
    /// `shared/` store as it currently stands. Launches under the profile
    /// then sync settings against it; divergence after capture is by design.
    pub fn capture_instance_profile(&self, reference: &str, name: &str) -> Result<Profile> {
        let (dir, profile) = self.find_profile(reference, name)?;
        if profile.captured {
            bail!("profile '{}' is already captured", profile.name);
        }
        self.sync()
            .capture(&profiles::store_dir(&dir, &profile.name))?;
        tracing::info!(instance = %reference, profile = %profile.name, "profile settings captured");
        Ok(Profile {
            captured: true,
            ..profile
        })
    }

    /// Delete the profile's captured store; it inherits the global store again
    /// from the next launch (the stale link relinks itself).
    pub fn release_instance_profile(&self, reference: &str, name: &str) -> Result<Profile> {
        let (dir, profile) = self.find_profile(reference, name)?;
        if !profile.captured {
            bail!("profile '{}' has no captured settings", profile.name);
        }
        self.sync()
            .release(&profiles::store_dir(&dir, &profile.name))?;
        tracing::info!(instance = %reference, profile = %profile.name, "profile settings released");
        Ok(Profile {
            captured: false,
            ..profile
        })
    }

    fn find_profile(&self, reference: &str, name: &str) -> Result<(PathBuf, Profile)> {
        let dir = self.profile_dir(reference)?;
        let (_, profiles) = profiles::list(&dir);
        let profile = profiles
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .with_context(|| format!("no profile named '{name}'"))?;
        Ok((dir, profile))
    }

    fn profile_dir(&self, reference: &str) -> Result<PathBuf> {
        let record = self
            .instances()
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        Ok(self.instances().instance_dir(&record.id))
    }

    /// Edit a global profile's references: adds are project slugs/ids resolved
    /// through the content registry on `source` (empty = the default source),
    /// removes match by project id or slug.
    pub async fn edit_global_profile(
        &self,
        name: &str,
        source: &str,
        add: &[String],
        remove: &[String],
    ) -> Result<GlobalProfile> {
        let profile = self.profiles().get(name)?;
        let mut entries = profile.entries;
        for reference in add {
            let project = self
                .content()
                .project(source, reference)
                .await
                .with_context(|| format!("cannot resolve '{reference}'"))?;
            if !profiles::selectable(project.kind) {
                bail!(
                    "'{}' is {:?} content; a global profile takes mods, resourcepacks, and shaders",
                    project.title,
                    project.kind
                );
            }
            let known = entries
                .iter()
                .any(|e| e.source == project.source && e.project_id == project.id);
            if !known {
                entries.push(ProfileEntry {
                    source: project.source,
                    project_id: project.id,
                    slug: project.slug,
                });
            }
        }
        entries.retain(|e| {
            !remove
                .iter()
                .any(|r| e.project_id == *r || e.slug.eq_ignore_ascii_case(r))
        });
        self.profiles().save(&profile.name, &entries)
    }

    /// Apply a global profile into an instance's pool: every reference not
    /// already present (any origin — a local copy wins) is resolved against the
    /// instance's game version and loader and installed through the ordinary
    /// add-content path, then tagged `profile:<name>` in the index. A reference
    /// with no compatible version is a per-item failure; the batch continues.
    /// Applying never removes de-listed content.
    pub async fn apply_global_profile(
        &self,
        reference: &str,
        profile_name: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<(Vec<InstalledContent>, Vec<ContentFailure>)> {
        let record = self
            .instances()
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let profile = self.profiles().get(profile_name)?;
        let entry_dir = self.instances().instance_dir(&record.id);
        let pool: HashSet<String> = install::load(&entry_dir)
            .into_iter()
            .map(|i| i.project_id)
            .filter(|p| !p.is_empty())
            .collect();

        let mut failures = Vec::new();
        let mut groups: Vec<(ContentKind, String, Vec<String>)> = Vec::new();
        for entry in &profile.entries {
            if pool.contains(&entry.project_id) {
                tracing::info!(
                    instance = %record.id,
                    project = %entry.slug,
                    "already in the pool; skipping"
                );
                continue;
            }
            let project = match self
                .content()
                .project(&entry.source, &entry.project_id)
                .await
            {
                Ok(project) => project,
                Err(e) => {
                    failures.push(ContentFailure {
                        item: entry.slug.clone(),
                        title: String::new(),
                        message: format!("{e:#}"),
                    });
                    continue;
                }
            };
            let group = groups
                .iter_mut()
                .find(|(kind, source, _)| *kind == project.kind && *source == entry.source);
            match group {
                Some((_, _, items)) => items.push(entry.project_id.clone()),
                None => groups.push((
                    project.kind,
                    entry.source.clone(),
                    vec![entry.project_id.clone()],
                )),
            }
        }

        let mut installed = Vec::new();
        for (kind, source, items) in groups {
            let spec = ContentAddSpec {
                kind,
                source,
                items: items
                    .into_iter()
                    .map(|project| ContentAddItem {
                        project,
                        ..ContentAddItem::default()
                    })
                    .collect(),
                worlds: Vec::new(),
            };
            let (mut items, mut group_failures) = self
                .add_instance_content(&record.id, &spec, on_progress)
                .await?;
            installed.append(&mut items);
            failures.append(&mut group_failures);
        }

        if !installed.is_empty() {
            let origin = format!("profile:{}", profile.name);
            let mut index = install::load(&entry_dir);
            for item in index.iter_mut() {
                let applied = installed
                    .iter()
                    .any(|i| i.kind == item.kind && i.filename == item.filename);
                if applied && item.origin.is_empty() {
                    item.origin = origin.clone();
                }
            }
            install::save(&entry_dir, index)?;
            for item in installed.iter_mut() {
                if item.origin.is_empty() {
                    item.origin = origin.clone();
                }
            }
        }
        tracing::info!(
            instance = %record.id,
            profile = %profile.name,
            installed = installed.len(),
            failures = failures.len(),
            "global profile applied"
        );
        Ok((installed, failures))
    }
}
