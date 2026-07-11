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
        kind: proto::sync::SyncKind,
        targets: proto::sync::SyncTargets,
    ) -> Result<proto::sync::SyncConfig, IpcError> {
        let params = proto::sync::SyncSetParams { kind, targets };
        self.session.call::<proto::sync::SyncSet>(&params).await
    }
}
