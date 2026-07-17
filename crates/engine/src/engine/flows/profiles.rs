//! Per-instance content profile management: CRUD over the `profiles.json`
//! store, with members validated against the instance's installed pool.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use proto::instance::Profile;

use crate::content::{install, profiles};
use crate::engine::Engine;

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

    fn profile_dir(&self, reference: &str) -> Result<PathBuf> {
        let record = self
            .instances()
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        Ok(self.instances().instance_dir(&record.id))
    }
}
