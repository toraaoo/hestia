use std::time::Duration;

use ipc::errors::IpcError;
use proto::content::{
    ContentAddSpec, ContentFailure, ContentKind, ContentUpdate, InstalledContent,
    InstanceContentAdd, InstanceContentAddParams, InstanceContentCheckUpdates,
    InstanceContentCheckUpdatesParams, InstanceContentEnable, InstanceContentEnableParams,
    InstanceContentList, InstanceContentListParams, InstanceContentRemove,
    InstanceContentRemoveParams, InstanceContentSetVersion, InstanceContentSetVersionParams,
    InstanceContentUpdate, InstanceContentUpdateParams,
};
use proto::instance::{
    InstanceConfigGet, InstanceConfigGetParams, InstanceConfigList, InstanceConfigSet,
    InstanceConfigSetParams, InstanceCreate, InstanceCreateParams, InstanceDetails,
    InstanceFlavors, InstanceInfo, InstanceInfoQuery, InstanceLaunch, InstanceLaunchParams,
    InstanceList, InstanceLoaders, InstanceLogs, InstanceLogsParams, InstanceProfileCapture,
    InstanceProfileCreate, InstanceProfileCreateParams, InstanceProfileEdit,
    InstanceProfileEditParams, InstanceProfileList, InstanceProfileRef, InstanceProfileRelease,
    InstanceProfileRemove, InstanceProfileRename, InstanceProfileRenameParams, InstanceProfileUse,
    InstanceRef, InstanceRemove, InstanceRename, InstanceRenameParams, InstanceResolve,
    InstanceStop, InstanceStopParams, InstanceUpdate, InstanceUpdateParams, InstanceVersions,
    InstanceWorlds, Profile,
};
use proto::minecraft::{
    ConfigEntry, Flavor, GameVersion, InstanceProfile, LoadersParams, ProvisionProgress,
    ResolveParams, VersionsParams,
};
use proto::process::ProcessLogLine;
use serde_json::Value;

use crate::facades::jobs::{forward, run_content_job};
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

    pub async fn loaders(&self, flavor: &str, version: &str) -> Result<Vec<String>, IpcError> {
        let params = LoadersParams {
            flavor: flavor.to_string(),
            version: version.to_string(),
        };
        Ok(self.session.call::<InstanceLoaders>(&params).await?.loaders)
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
    /// upstream; the new files materialise at the next launch). Nothing is
    /// backed up first — instances have no backups. `allow_downgrade` asserts
    /// the user confirmed a downgrade.
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

    /// The instance's static, informational view (locations + disk footprint).
    pub async fn info(&self, instance: &str) -> Result<InstanceDetails, IpcError> {
        self.session
            .call::<InstanceInfoQuery>(&instance_ref(instance))
            .await
    }

    /// The instance's save-world folder names — the worlds a datapack can
    /// install into.
    pub async fn worlds(&self, instance: &str) -> Result<Vec<String>, IpcError> {
        Ok(self
            .session
            .call::<InstanceWorlds>(&instance_ref(instance))
            .await?
            .worlds)
    }

    pub async fn remove(&self, instance: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceRemove>(&instance_ref(instance))
            .await?;
        Ok(())
    }

    /// Rename a stopped instance; the id (directory slug) is re-derived from
    /// the new name. Returns the updated record, whose `id` and `name` reflect
    /// the rename.
    pub async fn rename(&self, instance: &str, name: &str) -> Result<InstanceInfo, IpcError> {
        let params = InstanceRenameParams {
            instance: instance.to_string(),
            name: name.to_string(),
        };
        self.session.call::<InstanceRename>(&params).await
    }

    /// Stop a session (when `session` is set) or all of the instance's sessions.
    pub async fn stop(&self, instance: &str, session: Option<String>) -> Result<(), IpcError> {
        let params = InstanceStopParams {
            instance: instance.to_string(),
            session,
        };
        self.session.call::<InstanceStop>(&params).await?;
        Ok(())
    }

    pub async fn logs(
        &self,
        instance: &str,
        session: Option<String>,
        tail: Option<usize>,
    ) -> Result<Vec<ProcessLogLine>, IpcError> {
        let params = InstanceLogsParams {
            instance: instance.to_string(),
            session,
            tail,
        };
        Ok(self.session.call::<InstanceLogs>(&params).await?.lines)
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
    /// `profile` overrides the active content profile for this launch (empty
    /// keeps it; `none` launches with no profile). Returns the supervised
    /// process id and pid.
    pub async fn launch(
        &self,
        instance: &str,
        account: &str,
        new_session: bool,
        profile: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<(String, u32), IpcError> {
        let id = job_id("instance-launch");
        let session = self.session;
        let params = InstanceLaunchParams {
            instance: instance.to_string(),
            account: account.to_string(),
            id: id.clone(),
            new_session,
            profile: profile.to_string(),
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

    /// Install a batch of content into an instance, blocking until the daemon
    /// reports done or error and forwarding each progress event to
    /// `on_progress`. Returns everything installed (items plus required
    /// dependencies) and, per item that could not be installed, a failure.
    pub async fn content_add(
        &self,
        instance: &str,
        spec: ContentAddSpec,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<(Vec<InstalledContent>, Vec<ContentFailure>), IpcError> {
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

    /// The instance's active profile name (empty = none) and every profile.
    pub async fn profiles(&self, instance: &str) -> Result<(String, Vec<Profile>), IpcError> {
        let result = self
            .session
            .call::<InstanceProfileList>(&instance_ref(instance))
            .await?;
        Ok((result.active, result.profiles))
    }

    /// Create a profile — seeded from the pool by default, or empty.
    pub async fn create_profile(
        &self,
        instance: &str,
        name: &str,
        seed_from_pool: bool,
    ) -> Result<Profile, IpcError> {
        let params = InstanceProfileCreateParams {
            instance: instance.to_string(),
            name: name.to_string(),
            seed_from_pool,
        };
        self.session.call::<InstanceProfileCreate>(&params).await
    }

    /// Removing the active profile clears the active selection.
    pub async fn remove_profile(&self, instance: &str, name: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceProfileRemove>(&profile_ref(instance, name))
            .await?;
        Ok(())
    }

    pub async fn rename_profile(
        &self,
        instance: &str,
        name: &str,
        new_name: &str,
    ) -> Result<Profile, IpcError> {
        let params = InstanceProfileRenameParams {
            instance: instance.to_string(),
            name: name.to_string(),
            new_name: new_name.to_string(),
        };
        self.session.call::<InstanceProfileRename>(&params).await
    }

    /// Set the active profile (empty clears it); applied at the next launch.
    pub async fn use_profile(&self, instance: &str, name: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceProfileUse>(&profile_ref(instance, name))
            .await?;
        Ok(())
    }

    /// Add/remove members by pool reference (project id, slug, filename, or
    /// title).
    pub async fn edit_profile(
        &self,
        instance: &str,
        name: &str,
        add: Vec<String>,
        remove: Vec<String>,
    ) -> Result<Profile, IpcError> {
        let params = InstanceProfileEditParams {
            instance: instance.to_string(),
            name: name.to_string(),
            add,
            remove,
        };
        self.session.call::<InstanceProfileEdit>(&params).await
    }

    /// Capture the profile's own settings store (snapshotted from the global
    /// one); launches under it then sync settings against the captured store.
    /// The instance must be stopped.
    pub async fn capture_profile(&self, instance: &str, name: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceProfileCapture>(&profile_ref(instance, name))
            .await?;
        Ok(())
    }

    /// Delete the profile's captured store; it inherits the global store
    /// again. The instance must be stopped.
    pub async fn release_profile(&self, instance: &str, name: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceProfileRelease>(&profile_ref(instance, name))
            .await?;
        Ok(())
    }

    /// Apply a global profile into the instance's pool — a content job:
    /// references not already present install at their newest compatible
    /// version, tagged with the profile; incompatible ones come back as
    /// failures. Applying never removes de-listed content.
    pub async fn apply_profile(
        &self,
        instance: &str,
        profile: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<(Vec<InstalledContent>, Vec<ContentFailure>), IpcError> {
        let id = job_id("profile-apply");
        let params = proto::profile::InstanceProfileApplyParams {
            instance: instance.to_string(),
            profile: profile.to_string(),
            id: id.clone(),
        };
        let session = self.session;
        run_content_job(session, &id, on_progress, move || async move {
            session
                .call::<proto::profile::InstanceProfileApply>(&params)
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

    /// Uninstall one item. A non-empty `worlds` narrows a datapack removal
    /// to those save worlds; empty clears every copy.
    pub async fn content_remove(
        &self,
        instance: &str,
        kind: ContentKind,
        item: &str,
        worlds: &[String],
    ) -> Result<(), IpcError> {
        let params = InstanceContentRemoveParams {
            instance: instance.to_string(),
            kind,
            item: item.to_string(),
            worlds: worlds.to_vec(),
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
        .map(|(items, _)| items)
    }

    /// Enable or disable one installed item. A non-empty `worlds` narrows a
    /// datapack toggle to those save worlds.
    pub async fn content_enable(
        &self,
        instance: &str,
        kind: ContentKind,
        item: &str,
        enabled: bool,
        worlds: &[String],
    ) -> Result<(), IpcError> {
        let params = InstanceContentEnableParams {
            instance: instance.to_string(),
            kind,
            item: item.to_string(),
            enabled,
            worlds: worlds.to_vec(),
        };
        self.session.call::<InstanceContentEnable>(&params).await?;
        Ok(())
    }

    /// Which platform-sourced items of the kind have a newer compatible version.
    pub async fn content_check_updates(
        &self,
        instance: &str,
        kind: ContentKind,
    ) -> Result<Vec<ContentUpdate>, IpcError> {
        let params = InstanceContentCheckUpdatesParams {
            instance: instance.to_string(),
            kind,
        };
        let result = self
            .session
            .call_with_timeout::<InstanceContentCheckUpdates>(&params, Duration::from_secs(120))
            .await?;
        Ok(result.updates)
    }

    /// Re-pin one item to a specific published `version` (id or number).
    pub async fn content_set_version(
        &self,
        instance: &str,
        kind: ContentKind,
        item: &str,
        version: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<Vec<InstalledContent>, IpcError> {
        let id = job_id("instance-content-set-version");
        let params = InstanceContentSetVersionParams {
            instance: instance.to_string(),
            kind,
            item: item.to_string(),
            version: version.to_string(),
            id: id.clone(),
        };
        let session = self.session;
        run_content_job(session, &id, on_progress, move || async move {
            session
                .call::<InstanceContentSetVersion>(&params)
                .await
                .map(|_| ())
        })
        .await
        .map(|(items, _)| items)
    }
}

fn instance_ref(instance: &str) -> InstanceRef {
    InstanceRef {
        instance: instance.to_string(),
    }
}

fn profile_ref(instance: &str, name: &str) -> InstanceProfileRef {
    InstanceProfileRef {
        instance: instance.to_string(),
        name: name.to_string(),
    }
}
