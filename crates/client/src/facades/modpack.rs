//! Modpacks: install one into a new or existing entry, move it to another
//! published version, read which pack an entry runs, and take it back out.
//!
//! The install and update verbs are jobs — a pack is a hundred downloads and a
//! game directory rewrite — so they publish the `modpack.*` topics and settle on
//! the done event, which carries the entry the pack landed in. That id is the
//! only way a caller learns which entry a *creating* install just made.

use ipc::errors::IpcError;
use proto::minecraft::ProvisionProgress;
use proto::modpack::{
    InstalledModpack, InstanceModpackInstall, InstanceModpackInstallParams, InstanceModpackRef,
    InstanceModpackRemove, InstanceModpackStatus, InstanceModpackUpdate,
    InstanceModpackUpdateParams, ModpackDoneEvent, ModpackRef, ModpackRemoveResult, ModpackTarget,
    ServerModpackInstall, ServerModpackInstallParams, ServerModpackRef, ServerModpackRemove,
    ServerModpackStatus, ServerModpackUpdate, ServerModpackUpdateParams,
};

use crate::facades::jobs::forward;
use crate::session::{job_id, Session};

pub struct Modpack<'a> {
    pub(crate) session: &'a Session,
}

impl Modpack<'_> {
    /// Install a pack into a new or existing instance.
    pub async fn install_instance(
        &self,
        pack: ModpackRef,
        target: ModpackTarget,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<ModpackDoneEvent, IpcError> {
        let id = job_id("instance-modpack-install");
        let session = self.session;
        let params = InstanceModpackInstallParams {
            pack,
            target,
            id: id.clone(),
        };
        self.drive(&id, on_progress, move || async move {
            session
                .call::<InstanceModpackInstall>(&params)
                .await
                .map(|_| ())
        })
        .await
    }

    /// Install a pack into a new or existing server. `eula` is required when the
    /// target creates one.
    pub async fn install_server(
        &self,
        pack: ModpackRef,
        target: ModpackTarget,
        eula: bool,
        port: Option<u16>,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<ModpackDoneEvent, IpcError> {
        let id = job_id("server-modpack-install");
        let session = self.session;
        let params = ServerModpackInstallParams {
            pack,
            target,
            eula,
            port,
            id: id.clone(),
        };
        self.drive(&id, on_progress, move || async move {
            session
                .call::<ServerModpackInstall>(&params)
                .await
                .map(|_| ())
        })
        .await
    }

    /// Move an instance's pack to `version` (empty takes the newest). A pack
    /// update carries the game version with it, so an older one needs
    /// `allow_downgrade`.
    pub async fn update_instance(
        &self,
        instance: &str,
        version: &str,
        allow_downgrade: bool,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<ModpackDoneEvent, IpcError> {
        let id = job_id("instance-modpack-update");
        let session = self.session;
        let params = InstanceModpackUpdateParams {
            instance: instance.to_string(),
            version: version.to_string(),
            allow_downgrade,
            id: id.clone(),
        };
        self.drive(&id, on_progress, move || async move {
            session
                .call::<InstanceModpackUpdate>(&params)
                .await
                .map(|_| ())
        })
        .await
    }

    pub async fn update_server(
        &self,
        server: &str,
        version: &str,
        allow_downgrade: bool,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<ModpackDoneEvent, IpcError> {
        let id = job_id("server-modpack-update");
        let session = self.session;
        let params = ServerModpackUpdateParams {
            server: server.to_string(),
            version: version.to_string(),
            allow_downgrade,
            id: id.clone(),
        };
        self.drive(&id, on_progress, move || async move {
            session
                .call::<ServerModpackUpdate>(&params)
                .await
                .map(|_| ())
        })
        .await
    }

    /// The pack an instance runs, or `None` when it was not built from one.
    pub async fn instance_status(
        &self,
        instance: &str,
    ) -> Result<Option<InstalledModpack>, IpcError> {
        let params = InstanceModpackRef {
            instance: instance.to_string(),
        };
        Ok(self
            .session
            .call::<InstanceModpackStatus>(&params)
            .await?
            .pack)
    }

    pub async fn server_status(&self, server: &str) -> Result<Option<InstalledModpack>, IpcError> {
        let params = ServerModpackRef {
            server: server.to_string(),
        };
        Ok(self
            .session
            .call::<ServerModpackStatus>(&params)
            .await?
            .pack)
    }

    pub async fn remove_instance(&self, instance: &str) -> Result<ModpackRemoveResult, IpcError> {
        let params = InstanceModpackRef {
            instance: instance.to_string(),
        };
        self.session.call::<InstanceModpackRemove>(&params).await
    }

    pub async fn remove_server(&self, server: &str) -> Result<ModpackRemoveResult, IpcError> {
        let params = ServerModpackRef {
            server: server.to_string(),
        };
        self.session.call::<ServerModpackRemove>(&params).await
    }

    async fn drive<F, Fut>(
        &self,
        id: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
        start: F,
    ) -> Result<ModpackDoneEvent, IpcError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), IpcError>>,
    {
        let payload = self
            .session
            .run_job(
                id,
                "modpack.done",
                "modpack.error",
                forward(on_progress),
                start,
            )
            .await?;
        serde_json::from_value(payload).map_err(|e| IpcError::Malformed(e.to_string()))
    }
}
