//! One typed facade per domain, reached through `Client` accessors. Facade
//! methods are thin wrappers over `Session::call`, returning `proto` types
//! directly — mirroring the engine's domain modules on the other side of the
//! socket.

use std::time::Duration;

use ipc::errors::IpcError;
use ipc::protocol::Event;
use proto::accounts::{
    Account, AccountList, AccountLoginBegin, AccountLoginBeginParams, AccountLoginBeginResult,
    AccountLoginComplete, AccountLoginCompleteParams, AccountRemove, AccountRemoveParams,
    LoginMethod,
};
use proto::instance::{
    InstanceCreate, InstanceCreateParams, InstanceFlavors, InstanceInfo, InstanceLaunch,
    InstanceLaunchParams, InstanceList, InstanceRef, InstanceRemove, InstanceResolve, InstanceStop,
    InstanceVersions,
};
use proto::minecraft::{
    Flavor, GameVersion, InstanceProfile, ProvisionProgress, ResolveParams, ServerProfile,
    VersionsParams,
};
use proto::process::{
    ProcessExitEvent, ProcessInfo, ProcessList, ProcessLogLine, ProcessLogs, ProcessLogsParams,
    ProcessRef, ProcessSpec, ProcessStart, ProcessStartResult, ProcessStatus, ProcessStop,
};
use proto::server::{
    ServerCreate, ServerCreateParams, ServerFlavors, ServerInfo, ServerList, ServerLogs,
    ServerLogsParams, ServerRef, ServerRemove, ServerResolve, ServerStart, ServerStartResult,
    ServerStatus, ServerStop, ServerVersions,
};
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

pub struct Process<'a> {
    pub(crate) session: &'a Session,
}

impl Process<'_> {
    pub async fn start(&self, spec: ProcessSpec) -> Result<ProcessStartResult, IpcError> {
        self.session.call::<ProcessStart>(&spec).await
    }

    pub async fn stop(&self, id: &str) -> Result<(), IpcError> {
        self.session
            .call::<ProcessStop>(&ProcessRef { id: id.to_string() })
            .await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<ProcessInfo>, IpcError> {
        Ok(self
            .session
            .call::<ProcessList>(&proto::Empty {})
            .await?
            .processes)
    }

    pub async fn status(&self, id: &str) -> Result<ProcessInfo, IpcError> {
        self.session
            .call::<ProcessStatus>(&ProcessRef { id: id.to_string() })
            .await
    }

    pub async fn logs(
        &self,
        id: &str,
        tail: Option<usize>,
    ) -> Result<Vec<ProcessLogLine>, IpcError> {
        let params = ProcessLogsParams {
            id: id.to_string(),
            tail,
        };
        Ok(self.session.call::<ProcessLogs>(&params).await?.lines)
    }

    /// Launch a process and block until it exits, forwarding each output line to
    /// `on_output`. Returns the terminal exit event (state + code). The spec's id
    /// is filled in when empty so events can be matched before the process starts.
    pub async fn run(
        &self,
        mut spec: ProcessSpec,
        on_output: impl Fn(&ProcessLogLine) + Send + Sync + 'static,
    ) -> Result<ProcessExitEvent, IpcError> {
        if spec.id.is_empty() {
            spec.id = job_id("process");
        }
        let id = spec.id.clone();

        let on_event = move |event: &Event| {
            if let Ok(out) =
                serde_json::from_value::<proto::process::ProcessOutputEvent>(event.payload.clone())
            {
                on_output(&out.line);
            }
        };

        let session = self.session;
        let start_spec = spec.clone();
        // process.exit is the sole terminal topic; pass an unused error topic so
        // run_job's failure branch never fires (a non-zero exit is still "done").
        let payload = self
            .session
            .run_job(&id, "process.exit", "", on_event, move || async move {
                session.call::<ProcessStart>(&start_spec).await.map(|_| ())
            })
            .await?;

        serde_json::from_value(payload).map_err(|e| IpcError::Malformed(e.to_string()))
    }
}

pub struct Server<'a> {
    pub(crate) session: &'a Session,
}

impl Server<'_> {
    pub async fn flavors(&self) -> Result<Vec<Flavor>, IpcError> {
        Ok(self
            .session
            .call::<ServerFlavors>(&proto::Empty {})
            .await?
            .flavors)
    }

    pub async fn versions(&self, flavor: &str) -> Result<Vec<GameVersion>, IpcError> {
        let params = VersionsParams {
            flavor: flavor.to_string(),
        };
        Ok(self.session.call::<ServerVersions>(&params).await?.versions)
    }

    pub async fn resolve(
        &self,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
    ) -> Result<ServerProfile, IpcError> {
        let params = ResolveParams {
            flavor: flavor.to_string(),
            version: version.to_string(),
            loader_version,
        };
        self.session.call::<ServerResolve>(&params).await
    }

    /// Create a fully provisioned server, blocking until the daemon reports
    /// done or error and forwarding each progress event to `on_progress`.
    /// `eula` asserts the user accepted the Minecraft EULA.
    pub async fn create(
        &self,
        name: &str,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
        eula: bool,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<ServerInfo, IpcError> {
        let id = job_id("server-create");
        let on_event = move |event: &Event| {
            if let Ok(progress) = serde_json::from_value::<ProvisionProgress>(event.payload.clone())
            {
                on_progress(&progress);
            }
        };

        let session = self.session;
        let create_id = id.clone();
        let params = ServerCreateParams {
            name: name.to_string(),
            flavor: flavor.to_string(),
            version: version.to_string(),
            loader_version,
            eula,
            id: create_id.clone(),
        };
        let payload = self
            .session
            .run_job(
                &id,
                "server.create.done",
                "server.create.error",
                on_event,
                move || async move { session.call::<ServerCreate>(&params).await.map(|_| ()) },
            )
            .await?;

        serde_json::from_value(payload.get("server").cloned().unwrap_or(Value::Null))
            .map_err(|e| IpcError::Malformed(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<ServerInfo>, IpcError> {
        Ok(self
            .session
            .call::<ServerList>(&proto::Empty {})
            .await?
            .servers)
    }

    pub async fn status(&self, server: &str) -> Result<ServerInfo, IpcError> {
        self.session.call::<ServerStatus>(&server_ref(server)).await
    }

    pub async fn remove(&self, server: &str) -> Result<(), IpcError> {
        self.session
            .call::<ServerRemove>(&server_ref(server))
            .await?;
        Ok(())
    }

    pub async fn start(&self, server: &str) -> Result<ServerStartResult, IpcError> {
        self.session.call::<ServerStart>(&server_ref(server)).await
    }

    pub async fn stop(&self, server: &str) -> Result<(), IpcError> {
        self.session.call::<ServerStop>(&server_ref(server)).await?;
        Ok(())
    }

    pub async fn logs(
        &self,
        server: &str,
        tail: Option<usize>,
    ) -> Result<Vec<ProcessLogLine>, IpcError> {
        let params = ServerLogsParams {
            server: server.to_string(),
            tail,
        };
        Ok(self.session.call::<ServerLogs>(&params).await?.lines)
    }
}

fn server_ref(server: &str) -> ServerRef {
    ServerRef {
        server: server.to_string(),
    }
}

pub struct Instance<'a> {
    pub(crate) session: &'a Session,
}

impl Instance<'_> {
    pub async fn flavors(&self) -> Result<Vec<Flavor>, IpcError> {
        Ok(self
            .session
            .call::<InstanceFlavors>(&proto::Empty {})
            .await?
            .flavors)
    }

    pub async fn versions(&self, flavor: &str) -> Result<Vec<GameVersion>, IpcError> {
        let params = VersionsParams {
            flavor: flavor.to_string(),
        };
        Ok(self
            .session
            .call::<InstanceVersions>(&params)
            .await?
            .versions)
    }

    pub async fn resolve(
        &self,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
    ) -> Result<InstanceProfile, IpcError> {
        let params = ResolveParams {
            flavor: flavor.to_string(),
            version: version.to_string(),
            loader_version,
        };
        self.session.call::<InstanceResolve>(&params).await
    }

    /// Create an instance record (the profile is resolved upstream, so this can
    /// take a little while; files are materialised at launch).
    pub async fn create(
        &self,
        name: &str,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
    ) -> Result<InstanceInfo, IpcError> {
        let params = InstanceCreateParams {
            name: name.to_string(),
            flavor: flavor.to_string(),
            version: version.to_string(),
            loader_version,
        };
        Ok(self
            .session
            .call_with_timeout::<InstanceCreate>(&params, Duration::from_secs(60))
            .await?
            .instance)
    }

    pub async fn list(&self) -> Result<Vec<InstanceInfo>, IpcError> {
        Ok(self
            .session
            .call::<InstanceList>(&proto::Empty {})
            .await?
            .instances)
    }

    pub async fn remove(&self, instance: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceRemove>(&instance_ref(instance))
            .await?;
        Ok(())
    }

    pub async fn stop(&self, instance: &str) -> Result<(), IpcError> {
        self.session
            .call::<InstanceStop>(&instance_ref(instance))
            .await?;
        Ok(())
    }

    /// Launch an instance, blocking until the game process has spawned (or the
    /// preparation failed) and forwarding each progress event to `on_progress`.
    /// Returns the supervised process id and pid.
    pub async fn launch(
        &self,
        instance: &str,
        account: &str,
        on_progress: impl Fn(&ProvisionProgress) + Send + Sync + 'static,
    ) -> Result<(String, u32), IpcError> {
        let id = job_id("instance-launch");
        let on_event = move |event: &Event| {
            if let Ok(progress) = serde_json::from_value::<ProvisionProgress>(event.payload.clone())
            {
                on_progress(&progress);
            }
        };

        let session = self.session;
        let launch_id = id.clone();
        let params = InstanceLaunchParams {
            instance: instance.to_string(),
            account: account.to_string(),
            id: launch_id.clone(),
        };
        let payload = self
            .session
            .run_job(
                &id,
                "instance.launch.done",
                "instance.launch.error",
                on_event,
                move || async move { session.call::<InstanceLaunch>(&params).await.map(|_| ()) },
            )
            .await?;

        let process_id = payload
            .get("process_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let pid = payload.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32;
        Ok((process_id, pid))
    }
}

fn instance_ref(instance: &str) -> InstanceRef {
    InstanceRef {
        instance: instance.to_string(),
    }
}

pub struct Accounts<'a> {
    pub(crate) session: &'a Session,
}

impl Accounts<'_> {
    /// Begin a sign-in; returns what the user must act on (a device code or a
    /// browser URL). The daemon holds per-login state keyed by the returned id.
    pub async fn begin_login(
        &self,
        method: LoginMethod,
    ) -> Result<AccountLoginBeginResult, IpcError> {
        self.session
            .call_with_timeout::<AccountLoginBegin>(
                &AccountLoginBeginParams { method },
                Duration::from_secs(60),
            )
            .await
    }

    /// Drive a begun sign-in to a stored account. Long-running (the device-code
    /// flow polls until the user approves), so it carries a generous timeout.
    pub async fn complete_login(&self, id: &str, code: &str) -> Result<Account, IpcError> {
        let params = AccountLoginCompleteParams {
            id: id.to_string(),
            code: code.to_string(),
        };
        Ok(self
            .session
            .call_with_timeout::<AccountLoginComplete>(&params, Duration::from_secs(16 * 60))
            .await?
            .account)
    }

    pub async fn list(&self) -> Result<Vec<Account>, IpcError> {
        Ok(self
            .session
            .call::<AccountList>(&proto::Empty {})
            .await?
            .accounts)
    }

    pub async fn remove(&self, reference: &str) -> Result<(), IpcError> {
        let params = AccountRemoveParams {
            account: reference.to_string(),
        };
        self.session.call::<AccountRemove>(&params).await?;
        Ok(())
    }
}
