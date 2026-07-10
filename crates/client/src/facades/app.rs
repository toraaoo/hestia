use ipc::errors::IpcError;

use crate::session::Session;

pub struct App<'a> {
    pub(crate) session: &'a Session,
}

impl App<'_> {
    pub async fn info(&self) -> Result<proto::app::AppInfoResult, IpcError> {
        self.session
            .call::<proto::app::AppInfo>(&proto::Empty {})
            .await
    }

    pub async fn ping(&self) -> Result<proto::health::PingResult, IpcError> {
        self.session
            .call::<proto::health::Ping>(&proto::Empty {})
            .await
    }
}
