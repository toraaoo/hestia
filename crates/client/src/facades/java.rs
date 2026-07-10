use ipc::errors::IpcError;
use ipc::protocol::Event;
use serde_json::Value;

use crate::session::{job_id, Session};

pub struct Java<'a> {
    pub(crate) session: &'a Session,
}

impl Java<'_> {
    pub async fn releases(&self) -> Result<Vec<proto::java::JavaRelease>, IpcError> {
        Ok(self
            .session
            .call::<proto::java::JavaReleases>(&proto::Empty {})
            .await?
            .releases)
    }

    pub async fn list(&self) -> Result<Vec<proto::java::JavaRuntime>, IpcError> {
        Ok(self
            .session
            .call::<proto::java::JavaList>(&proto::Empty {})
            .await?
            .runtimes)
    }

    pub async fn uninstall(&self, major: i32) -> Result<(), IpcError> {
        let params = proto::java::JavaUninstallParams { major };
        self.session
            .call::<proto::java::JavaUninstall>(&params)
            .await?;
        Ok(())
    }

    /// Install a runtime, blocking until the daemon reports done or error and
    /// forwarding each progress event to `on_progress`. Returns the registered
    /// runtime and whether it was already installed.
    pub async fn install(
        &self,
        major: i32,
        force: bool,
        on_progress: impl Fn(&proto::java::JavaInstallProgress) + Send + Sync + 'static,
    ) -> Result<(proto::java::JavaRuntime, bool), IpcError> {
        use proto::java::{JavaInstall, JavaInstallParams};

        let id = job_id("java-install");
        let on_event = move |event: &Event| {
            if let Ok(progress) =
                serde_json::from_value::<proto::java::JavaInstallProgress>(event.payload.clone())
            {
                on_progress(&progress);
            }
        };

        let session = self.session;
        let install_id = id.clone();
        let payload = self
            .session
            .run_job(
                &id,
                "java.install.done",
                "java.install.error",
                on_event,
                move || async move {
                    let params = JavaInstallParams {
                        major,
                        id: install_id,
                        force,
                    };
                    session.call::<JavaInstall>(&params).await.map(|_| ())
                },
            )
            .await?;

        let runtime =
            serde_json::from_value(payload.get("runtime").cloned().unwrap_or(Value::Null))
                .map_err(|e| IpcError::Malformed(e.to_string()))?;
        let already = payload
            .get("already_installed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok((runtime, already))
    }
}
