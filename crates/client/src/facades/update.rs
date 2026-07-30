use std::path::PathBuf;

use ipc::errors::IpcError;
use ipc::protocol::Event;
use serde_json::Value;

use crate::session::{job_id, Session};

pub struct Update<'a> {
    pub(crate) session: &'a Session,
}

impl Update<'_> {
    pub async fn check(&self) -> Result<proto::update::UpdateCheckResult, IpcError> {
        self.session
            .call::<proto::update::UpdateCheck>(&proto::Empty {})
            .await
    }

    /// Download the latest installer through the daemon, forwarding byte
    /// progress. Resolves to the staged path and the version it carries.
    pub async fn download(
        &self,
        on_progress: impl Fn(&proto::download::DownloadProgress) + Send + Sync + 'static,
    ) -> Result<(PathBuf, String), IpcError> {
        use proto::update::{UpdateDownload, UpdateDownloadParams};

        let id = job_id("update");
        let on_event = move |event: &Event| {
            if let Ok(progress) =
                serde_json::from_value::<proto::download::DownloadProgress>(event.payload.clone())
            {
                on_progress(&progress);
            }
        };

        let session = self.session;
        let download_id = id.clone();
        let payload = self
            .session
            .run_job(
                &id,
                "update.done",
                "update.error",
                on_event,
                move || async move {
                    let params = UpdateDownloadParams { id: download_id };
                    session.call::<UpdateDownload>(&params).await.map(|_| ())
                },
            )
            .await?;

        let path = payload
            .get("path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .ok_or_else(|| IpcError::Malformed("update.done carried no path".into()))?;
        let version = payload
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok((path, version))
    }
}
