//! The sync flows that compose the `sync` subsystem with the instance store:
//! per-instance link status and the adopt migration.

use anyhow::{Context, Result};
use proto::sync::InstanceSyncStatus;

use crate::engine::Engine;

impl Engine {
    /// Every instance's per-folder-target link state.
    pub fn sync_status(&self) -> Vec<InstanceSyncStatus> {
        self.instances
            .list()
            .into_iter()
            .map(|record| InstanceSyncStatus {
                targets: self.sync.status(&self.instances.data_dir(&record.id)),
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
        self.sync
            .adopt(&self.instances.data_dir(&record.id), targets)
    }
}
