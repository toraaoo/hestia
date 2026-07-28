//! The sync flows that compose the `sync` subsystem with the instance store:
//! the reconcile pass each launch runs, per-instance link status, and the adopt
//! migration.

use std::path::Path;

use anyhow::{Context, Result};
use proto::sync::InstanceSyncStatus;
use proto::warning::WarningInfo;

use crate::content::modpack;
use crate::engine::Engine;
use crate::sync::Settings;

impl Engine {
    /// Whether instances share their targets at all (`sync.enabled`).
    pub fn sync_enabled(&self) -> bool {
        self.config.settings().sync.enabled
    }

    /// Reconcile one instance's `data/` against the store. Switched off, this
    /// does nothing at all — no copy, no link, no warning.
    ///
    /// `profile_store` is a captured profile's own store, which scopes the
    /// settings-class targets to it. Without one, an entry running a **modpack**
    /// keeps its config tree: the pack ships it, so hestia does not fold it into
    /// what every other instance reads. That is the automatic pass only — a
    /// `sync adopt` the user asks for still opts the folder in, and the link it
    /// leaves is reconciled from then on.
    pub(crate) fn apply_instance_sync(
        &self,
        name: &str,
        entry_dir: &Path,
        data_dir: &Path,
        profile_store: Option<&Path>,
    ) -> Vec<WarningInfo> {
        if !self.sync_enabled() {
            return Vec::new();
        }
        let settings = match profile_store {
            Some(store) => Settings::Profile(store),
            None if modpack::load(entry_dir).is_some() => Settings::Local,
            None => Settings::Shared,
        };
        self.sync.apply(name, data_dir, settings)
    }

    /// Link a brand-new instance's folder targets, so nothing can fill them
    /// before the first launch — a modpack install writes its `overrides/`
    /// straight into the game directory. The settings folders are deliberately
    /// left for the launch: whether a pack owns them is not knowable yet.
    /// Best-effort, like every reconcile; nothing here fails a create.
    pub(crate) fn link_new_instance(&self, name: &str, data_dir: &Path) {
        if !self.sync_enabled() {
            return;
        }
        for warning in self.sync.apply(name, data_dir, Settings::Local) {
            tracing::debug!(instance = name, warning = %warning, "sync at create");
        }
    }

    /// Every instance's per-folder-target link state.
    pub fn sync_status(&self) -> Vec<InstanceSyncStatus> {
        self.instances
            .list()
            .into_iter()
            .map(|record| InstanceSyncStatus {
                targets: self.sync.status(&self.instances.data_dir(&record)),
                id: record.id,
                name: record.name,
            })
            .collect()
    }

    /// Adopt a stopped instance's existing folder contents into the shared
    /// store (all folder targets when `targets` is empty). All-or-nothing per
    /// target; a store collision refuses that target with the names.
    pub fn adopt_instance_sync(&self, reference: &str, targets: &[String]) -> Result<Vec<String>> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        self.sync.adopt(&self.instances.data_dir(&record), targets)
    }
}
