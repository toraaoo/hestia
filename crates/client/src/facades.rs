//! One typed facade per domain, reached through `Client` accessors. Facade
//! methods are thin wrappers over `Session::call`, returning `proto` types
//! directly — mirroring the engine's domain modules on the other side of the
//! socket.

use ipc::errors::IpcError;
use ipc::protocol::Event;
use serde_json::Value;

use crate::session::{job_id, Session};

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

pub struct Daemon<'a> {
    pub(crate) session: &'a Session,
}

impl Daemon<'_> {
    pub async fn status(&self) -> Result<proto::daemon::DaemonStatusResult, IpcError> {
        self.session
            .call::<proto::daemon::DaemonStatus>(&proto::Empty {})
            .await
    }

    pub async fn stop(&self) -> Result<proto::daemon::DaemonStopResult, IpcError> {
        self.session
            .call::<proto::daemon::DaemonStop>(&proto::Empty {})
            .await
    }
}

pub struct Config<'a> {
    pub(crate) session: &'a Session,
}

impl Config<'_> {
    /// Returns `None` when the key is unknown (a `not_found` from the daemon).
    pub async fn get(&self, key: &str) -> Result<Option<Value>, IpcError> {
        let params = proto::config::ConfigGetParams {
            key: key.to_string(),
        };
        Ok(self
            .session
            .try_call::<proto::config::ConfigGet>(&params)
            .await?
            .map(|r| r.value))
    }

    pub async fn set(&self, key: &str, value: Value) -> Result<(), IpcError> {
        let params = proto::config::ConfigSetParams {
            key: key.to_string(),
            value,
        };
        self.session
            .call::<proto::config::ConfigSet>(&params)
            .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Value, IpcError> {
        Ok(self
            .session
            .call::<proto::config::ConfigList>(&proto::Empty {})
            .await?
            .entries)
    }
}

pub struct Cache<'a> {
    pub(crate) session: &'a Session,
}

impl Cache<'_> {
    pub async fn info(&self) -> Result<proto::cache::CacheInfoResult, IpcError> {
        self.session
            .call::<proto::cache::CacheInfo>(&proto::Empty {})
            .await
    }

    pub async fn list(&self) -> Result<Vec<proto::cache::CacheEntry>, IpcError> {
        Ok(self
            .session
            .call::<proto::cache::CacheList>(&proto::Empty {})
            .await?
            .entries)
    }

    pub async fn clear(&self) -> Result<proto::cache::CacheUsage, IpcError> {
        self.session
            .call::<proto::cache::CacheClear>(&proto::Empty {})
            .await
    }
}

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
