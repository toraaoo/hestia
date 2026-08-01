use std::path::{Path, PathBuf};

use ipc::errors::IpcError;
use ipc::protocol::Event;

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

    /// Resolves to the staged path and the version it carries.
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

        let done: proto::update::UpdateDoneEvent =
            serde_json::from_value(payload).map_err(|e| IpcError::Malformed(e.to_string()))?;
        Ok((done.path, done.version))
    }

    pub async fn apply(&self, path: &Path) -> Result<proto::update::UpdateApplyResult, IpcError> {
        self.session
            .call_with_timeout::<proto::update::UpdateApply>(
                &proto::update::UpdateApplyParams {
                    path: path.to_path_buf(),
                },
                APPLY_TIMEOUT,
            )
            .await
    }
}

/// A package install waits on an interactive elevation prompt.
const APPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
