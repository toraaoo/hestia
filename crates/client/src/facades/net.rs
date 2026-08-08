use ipc::errors::IpcError;

use crate::session::Session;

pub struct Net<'a> {
    pub(crate) session: &'a Session,
}

impl Net<'_> {
    /// Whether the daemon can currently reach upstream. Front-ends read this
    /// once and follow the `net.state` topic rather than inferring it from a
    /// failed call.
    pub async fn status(&self) -> Result<proto::net::NetworkStatus, IpcError> {
        self.session
            .call::<proto::net::NetStatus>(&proto::Empty {})
            .await
    }
}
