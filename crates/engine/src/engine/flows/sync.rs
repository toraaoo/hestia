//! The sync flows that compose the `sync` subsystem with the instance store:
//! the reconcile pass a launch runs, the one its exit replays, per-instance
//! link status, and the adopt migration.

use std::path::Path;

use anyhow::{Context, Result};
use proto::sync::InstanceSyncStatus;
use proto::warning::WarningInfo;

use crate::content::modpack;
use crate::engine::Engine;
use crate::instances::InstanceRecord;
use crate::sync::{Pass, Scope};

impl Engine {
    /// Whether instances share their targets at all (`sync.enabled`).
    pub fn sync_enabled(&self) -> bool {
        self.config.settings().sync.enabled
    }

    /// The reconcile an instance would run, or `None` when it shares nothing —
    /// sharing switched off launcher-wide, or this instance opted out.
    ///
    /// `profile_store` is a captured profile's own store, which scopes the
    /// settings-class targets to it. Without one, an entry running a **modpack**
    /// keeps its config tree: the pack ships it, so hestia does not fold it into
    /// what every other instance reads. That is the automatic pass only — a
    /// `sync adopt` the user asks for still opts the folder in, and the link it
    /// leaves is reconciled from then on.
    pub(crate) fn instance_pass(
        &self,
        record: &InstanceRecord,
        entry_dir: &Path,
        data_dir: &Path,
        profile_store: Option<&Path>,
    ) -> Option<Pass> {
        if !self.shares_settings(record) {
            return None;
        }
        let scope = match profile_store {
            Some(store) => Scope::Profile(store.to_path_buf()),
            None if modpack::load(entry_dir).is_some() => Scope::Local,
            None => Scope::Shared,
        };
        Some(pass(record, data_dir, scope))
    }

    fn shares_settings(&self, record: &InstanceRecord) -> bool {
        self.sync_enabled() && record.shares_settings()
    }

    /// Reconcile for a launching session and record what it reconciled, so the
    /// pass that runs when the session exits uses the same scope.
    pub(crate) fn begin_instance_sync(&self, session: &str, pass: Pass) -> Vec<WarningInfo> {
        let warnings = self.sync.apply(&pass);
        self.sync.remember(session, pass);
        warnings
    }

    /// Reconcile once more for a session that has exited, so what the player
    /// changed in game reaches the store now rather than at their next launch.
    ///
    /// Skipped while the instance's other sessions run: they hold the same
    /// files open, and a pass that copied over a live game's `options.txt`
    /// would lose whatever it writes at its own exit.
    pub fn finish_instance_sync(&self, session: &str) {
        let Some(pass) = self.sync.recall(session) else {
            return;
        };
        let running = self.running_sessions(&pass.id);
        if running > 0 {
            tracing::debug!(
                instance = %pass.name,
                running,
                "leaving the sync pass to the last session out"
            );
            return;
        }
        for warning in self.sync.apply(&pass) {
            tracing::debug!(instance = %pass.name, warning = %warning, "sync at exit");
        }
    }

    /// Link a brand-new instance's folder targets, so nothing can fill them
    /// before the first launch — a modpack install writes its `overrides/`
    /// straight into the game directory. The settings folders are deliberately
    /// left for the launch: whether a pack owns them is not knowable yet.
    /// Best-effort, like every reconcile; nothing here fails a create.
    pub(crate) fn link_new_instance(&self, record: &InstanceRecord, data_dir: &Path) {
        if !self.shares_settings(record) {
            return;
        }
        for warning in self.sync.apply(&pass(record, data_dir, Scope::Local)) {
            tracing::debug!(instance = %record.name, warning = %warning, "sync at create");
        }
    }

    /// Every instance's per-folder-target link state, and whether it shares at
    /// all — an opted-out instance reports no targets, since none of them
    /// describe it.
    pub fn sync_status(&self) -> Vec<InstanceSyncStatus> {
        self.instances
            .list()
            .into_iter()
            .map(|record| InstanceSyncStatus {
                enabled: record.shares_settings(),
                targets: match record.shares_settings() {
                    true => self.sync.status(&self.instances.data_dir(&record)),
                    false => Vec::new(),
                },
                id: record.id,
                name: record.name,
            })
            .collect()
    }

    /// Put one instance in or out of shared settings. Leaving hands it its own
    /// copy of every folder it shared; rejoining folds it back in with the
    /// store winning whatever the two both have. What was duplicated or
    /// discarded rides back as warnings. The caller guarantees it is stopped.
    pub fn set_instance_sharing(
        &self,
        reference: &str,
        on: bool,
    ) -> Result<(bool, Vec<WarningInfo>)> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        if record.shares_settings() == on {
            return Ok((on, Vec::new()));
        }
        let warnings = if self.sync_enabled() {
            let data_dir = self.instances.data_dir(&record);
            let pass = pass(&record, &data_dir, self.settings_scope(&record));
            match on {
                true => self.sync.attach(&pass)?,
                false => self.sync.detach(&pass)?,
            }
        } else {
            Vec::new()
        };
        self.instances.set_sharing(&record.id, on)?;
        tracing::info!(instance = %record.name, sharing = on, "instance sharing changed");
        Ok((on, warnings))
    }

    /// Which store the settings-class targets belong to outside a launch, where
    /// no profile is in play.
    fn settings_scope(&self, record: &InstanceRecord) -> Scope {
        match modpack::load(&self.instances.instance_dir(record)).is_some() {
            true => Scope::Local,
            false => Scope::Shared,
        }
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

fn pass(record: &InstanceRecord, data_dir: &Path, scope: Scope) -> Pass {
    Pass {
        id: record.id.clone(),
        name: record.name.clone(),
        data_dir: data_dir.to_path_buf(),
        scope,
    }
}
