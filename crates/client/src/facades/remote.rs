use ipc::errors::IpcError;
use proto::remote::{
    RemoteKey, RemoteKeyCreate, RemoteKeyCreateParams, RemoteKeyCreateResult, RemoteKeyList,
    RemoteKeyRef, RemoteKeyRevoke, RemoteStatus, RemoteStatusResult, Scope,
};

use crate::session::Session;

pub struct Remote<'a> {
    pub(crate) session: &'a Session,
}

impl Remote<'_> {
    /// Whether this node's HTTP door is open, and where.
    pub async fn status(&self) -> Result<RemoteStatusResult, IpcError> {
        self.session.call::<RemoteStatus>(&proto::Empty {}).await
    }

    /// Mint a key. The token comes back once and is not recoverable — a caller
    /// that does not show it to the operator has thrown it away.
    pub async fn create_key(
        &self,
        name: &str,
        scopes: Vec<Scope>,
        servers: Vec<String>,
    ) -> Result<RemoteKeyCreateResult, IpcError> {
        self.session
            .call::<RemoteKeyCreate>(&RemoteKeyCreateParams {
                name: name.to_string(),
                scopes,
                servers,
            })
            .await
    }

    pub async fn keys(&self) -> Result<Vec<RemoteKey>, IpcError> {
        Ok(self
            .session
            .call::<RemoteKeyList>(&proto::Empty {})
            .await?
            .keys)
    }

    /// Revoke by id or by the prefix a listing shows.
    pub async fn revoke_key(&self, key: &str) -> Result<(), IpcError> {
        self.session
            .call::<RemoteKeyRevoke>(&RemoteKeyRef {
                key: key.to_string(),
            })
            .await?;
        Ok(())
    }
}
