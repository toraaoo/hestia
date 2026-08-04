use ipc::errors::IpcError;

use crate::session::Session;

pub struct Sync<'a> {
    pub(crate) session: &'a Session,
}

impl Sync<'_> {
    pub async fn get(&self) -> Result<proto::sync::SyncConfig, IpcError> {
        self.session
            .call::<proto::sync::SyncGet>(&proto::Empty {})
            .await
    }

    pub async fn set(
        &self,
        targets: proto::sync::SyncTargets,
    ) -> Result<proto::sync::SyncConfig, IpcError> {
        let params = proto::sync::SyncSetParams { targets };
        self.session.call::<proto::sync::SyncSet>(&params).await
    }

    /// Every instance's per-folder-target link state.
    pub async fn status(&self) -> Result<Vec<proto::sync::InstanceSyncStatus>, IpcError> {
        Ok(self
            .session
            .call::<proto::sync::SyncStatus>(&proto::Empty {})
            .await?
            .instances)
    }

    /// Adopt a stopped instance's folder contents into the shared store
    /// (every folder target when `targets` is empty). Returns the targets
    /// linked after the call.
    pub async fn adopt(
        &self,
        instance: &str,
        targets: Vec<String>,
    ) -> Result<Vec<String>, IpcError> {
        let params = proto::sync::SyncAdoptParams {
            instance: instance.to_string(),
            targets,
        };
        Ok(self
            .session
            .call::<proto::sync::SyncAdopt>(&params)
            .await?
            .adopted)
    }

    /// Put a stopped instance in or out of shared settings. Leaving copies the
    /// folders it shares out of the store; rejoining lets the store win
    /// whatever the two both have. The warnings say what that cost.
    pub async fn share(
        &self,
        instance: &str,
        enabled: bool,
    ) -> Result<proto::sync::SyncShareResult, IpcError> {
        let params = proto::sync::SyncShareParams {
            instance: instance.to_string(),
            enabled,
        };
        self.session.call::<proto::sync::SyncShare>(&params).await
    }
}
