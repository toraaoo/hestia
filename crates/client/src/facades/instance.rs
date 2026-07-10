use std::time::Duration;

use ipc::errors::IpcError;
use proto::backup::{
    BackupInfo, InstanceBackupCreate, InstanceBackupCreateParams, InstanceBackupList,
    InstanceBackupRef, InstanceBackupRemove, InstanceBackupRestore, InstanceBackupRestoreParams,
};
use proto::content::{
    ContentAddSpec, ContentKind, InstalledContent, InstanceContentAdd, InstanceContentAddParams,
    InstanceContentList, InstanceContentListParams, InstanceContentRemove,
    InstanceContentRemoveParams, InstanceContentUpdate, InstanceContentUpdateParams,
};
use proto::instance::{
    InstanceConfigGet, InstanceConfigGetParams, InstanceConfigList, InstanceConfigSet,
    InstanceConfigSetParams, InstanceCreate, InstanceCreateParams, InstanceFlavors, InstanceInfo,
    InstanceLaunch, InstanceLaunchParams, InstanceList, InstanceLogs, InstanceLogsParams,
    InstanceRef, InstanceRemove, InstanceResolve, InstanceStop, InstanceUpdate,
    InstanceUpdateParams, InstanceVersions,
};
use proto::minecraft::{
    ConfigEntry, Flavor, GameVersion, InstanceProfile, ProvisionProgress, ResolveParams,
    VersionsParams,
};
use proto::process::ProcessLogLine;
use serde_json::Value;

use crate::facades::jobs::{forward, run_backup_job, run_content_job};
use crate::session::{job_id, Session};

pub struct Instance<'a> {
    pub(crate) session: &'a Session,
}

impl Instance<'_> {
    pub async fn flavors(&self) -> Result<Vec<Flavor>, IpcError> {
        Ok(self
            .session
            .call::<InstanceFlavors>(&proto::Empty {})
            .await?
            .flavors)
    }

    pub async fn versions(&self, flavor: &str) -> Result<Vec<GameVersion>, IpcError> {
        let params = VersionsParams {
            flavor: flavor.to_string(),
        };
        Ok(self
            .session
            .call::<InstanceVersions>(&params)
            .await?
            .versions)
    }

    pub async fn resolve(
        &self,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
    ) -> Result<InstanceProfile, IpcError> {
        let params = ResolveParams {
            flavor: flavor.to_string(),
            version: version.to_string(),
            loader_version,
        };
        self.session.call::<InstanceResolve>(&params).await
    }

    /// Create an instance record (the profile is resolved upstream, so this can
    /// take a little while; files are materialised at launch).
    pub async fn create(
        &self,
        name: &str,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
        config: Vec<ConfigEntry>,
    ) -> Result<InstanceInfo, IpcError> {
        let params = InstanceCreateParams {
            name: name.to_string(),
            flavor: flavor.to_string(),
            version: version.to_string(),
            loader_version,
            config,
        };
        Ok(self
            .session
            .call_with_timeout::<InstanceCreate>(&params, Duration::from_secs(60))
            .await?
            .instance)
    }

    /// Move a stopped instance to another version (the profile is resolved
    /// upstream and the game directory is backed up first, so this can take a
    /// while; the new files materialise at the next launch). `allow_downgrade`
    /// asserts the user confirmed a downgrade.
    pub async fn update(
        &self,
        instance: &str,
        version: &str,
        loader_version: Option<String>,
        allow_downgrade: bool,
    ) -> Result<InstanceInfo, IpcError> {
        let params = InstanceUpdateParams {
            instance: instance.to_string(),
            version: version.to_string(),
            loader_version,
            allow_downgrade,
        };
        Ok(self
            .session
            .call_with_timeout::<InstanceUpdate>(&params, Duration::from_secs(10 * 60))
            .await?
            .instance)
    }

    pub async fn list(&self) -> Result<Vec<InstanceInfo>, IpcError> {
        Ok(self
            .session
            .call::<InstanceList>(&proto::Empty {})
            .await?
            .instances)
    }

    pub async fn remove(&self, instance: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceRemove>(&instance_ref(instance))
            .await?;
        Ok(())
    }

    pub async fn stop(&self, instance: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceStop>(&instance_ref(instance))
            .await?;
        Ok(())
    }

    pub async fn logs(
        &self,
        instance: &str,
        tail: Option<usize>,
    ) -> Result<Vec<ProcessLogLine>, IpcError> {
        let params = InstanceLogsParams {
            instance: instance.to_string(),
            tail,
        };
        Ok(self.session.call::<InstanceLogs>(&params).await?.lines)
    }

    /// Archive a stopped instance's data directory, blocking until the daemon
    /// reports done or error and forwarding each progress event to
    /// `on_progress`.
    pub async fn backup_create(
        &self,
        instance: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<BackupInfo, IpcError> {
        let id = job_id("instance-backup");
        let params = InstanceBackupCreateParams {
            instance: instance.to_string(),
            id: id.clone(),
        };
        let session = self.session;
        run_backup_job(session, &id, on_progress, move || async move {
            session
                .call::<InstanceBackupCreate>(&params)
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn backup_list(&self, instance: &str) -> Result<Vec<BackupInfo>, IpcError> {
        Ok(self
            .session
            .call::<InstanceBackupList>(&instance_ref(instance))
            .await?
            .backups)
    }

    /// Replace a stopped instance's data directory with a backup's content,
    /// blocking until the daemon reports done or error and forwarding each
    /// progress event to `on_progress`.
    pub async fn backup_restore(
        &self,
        instance: &str,
        backup: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<BackupInfo, IpcError> {
        let id = job_id("instance-restore");
        let params = InstanceBackupRestoreParams {
            instance: instance.to_string(),
            backup: backup.to_string(),
            id: id.clone(),
        };
        let session = self.session;
        run_backup_job(session, &id, on_progress, move || async move {
            session
                .call::<InstanceBackupRestore>(&params)
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn backup_remove(&self, instance: &str, backup: &str) -> Result<(), IpcError> {
        let params = InstanceBackupRef {
            instance: instance.to_string(),
            backup: backup.to_string(),
        };
        self.session.call::<InstanceBackupRemove>(&params).await?;
        Ok(())
    }

    /// Read one JVM setting; `None` when it is not set (a `not_found` from the
    /// daemon).
    pub async fn config_get(&self, instance: &str, key: &str) -> Result<Option<String>, IpcError> {
        let params = InstanceConfigGetParams {
            instance: instance.to_string(),
            key: key.to_string(),
        };
        Ok(self
            .session
            .try_call::<InstanceConfigGet>(&params)
            .await?
            .map(|r| r.value))
    }

    pub async fn config_set(&self, instance: &str, key: &str, value: &str) -> Result<(), IpcError> {
        let params = InstanceConfigSetParams {
            instance: instance.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        };
        self.session.call::<InstanceConfigSet>(&params).await?;
        Ok(())
    }

    pub async fn config_list(&self, instance: &str) -> Result<Vec<ConfigEntry>, IpcError> {
        Ok(self
            .session
            .call::<InstanceConfigList>(&instance_ref(instance))
            .await?
            .entries)
    }

    /// Launch an instance, blocking until the game process has spawned (or the
    /// preparation failed) and forwarding each progress event to `on_progress`.
    /// Returns the supervised process id and pid.
    pub async fn launch(
        &self,
        instance: &str,
        account: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<(String, u32), IpcError> {
        let id = job_id("instance-launch");
        let session = self.session;
        let params = InstanceLaunchParams {
            instance: instance.to_string(),
            account: account.to_string(),
            id: id.clone(),
        };
        let payload = self
            .session
            .run_job(
                &id,
                "instance.launch.done",
                "instance.launch.error",
                forward(on_progress),
                move || async move { session.call::<InstanceLaunch>(&params).await.map(|_| ()) },
            )
            .await?;

        let process_id = payload
            .get("process_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let pid = payload.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32;
        Ok((process_id, pid))
    }

    /// Install content into an instance, blocking until the daemon reports done
    /// or error and forwarding each progress event to `on_progress`. Returns
    /// everything installed (the item plus any required dependencies).
    pub async fn content_add(
        &self,
        instance: &str,
        spec: ContentAddSpec,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<Vec<InstalledContent>, IpcError> {
        let id = job_id("instance-content-add");
        let params = InstanceContentAddParams {
            instance: instance.to_string(),
            spec,
            id: id.clone(),
        };
        let session = self.session;
        run_content_job(session, &id, on_progress, move || async move {
            session
                .call::<InstanceContentAdd>(&params)
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn content_list(
        &self,
        instance: &str,
        kind: ContentKind,
    ) -> Result<(Vec<InstalledContent>, Vec<String>), IpcError> {
        let params = InstanceContentListParams {
            instance: instance.to_string(),
            kind,
        };
        let result = self.session.call::<InstanceContentList>(&params).await?;
        Ok((result.items, result.untracked))
    }

    pub async fn content_remove(
        &self,
        instance: &str,
        kind: ContentKind,
        item: &str,
    ) -> Result<(), IpcError> {
        let params = InstanceContentRemoveParams {
            instance: instance.to_string(),
            kind,
            item: item.to_string(),
        };
        self.session.call::<InstanceContentRemove>(&params).await?;
        Ok(())
    }

    /// Update platform-sourced content to its newest compatible version — one
    /// named item, or every item of the kind when `item` is empty.
    pub async fn content_update(
        &self,
        instance: &str,
        kind: ContentKind,
        item: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<Vec<InstalledContent>, IpcError> {
        let id = job_id("instance-content-update");
        let params = InstanceContentUpdateParams {
            instance: instance.to_string(),
            kind,
            item: item.to_string(),
            id: id.clone(),
        };
        let session = self.session;
        run_content_job(session, &id, on_progress, move || async move {
            session
                .call::<InstanceContentUpdate>(&params)
                .await
                .map(|_| ())
        })
        .await
    }
}

fn instance_ref(instance: &str) -> InstanceRef {
    InstanceRef {
        instance: instance.to_string(),
    }
}
