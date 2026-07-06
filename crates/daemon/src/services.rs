//! Every channel the daemon serves, wired onto the router. One `handle::<C>` per
//! channel; handlers reach the daemon's collaborators through `HandlerContext`
//! and return `ServiceError` for a typed failure.

use engine::ConfigError;
use proto::accounts::{
    AccountList, AccountListResult, AccountLoginBegin, AccountLoginBeginResult,
    AccountLoginComplete, AccountLoginCompleteResult, AccountRemove, AccountSwitch,
    AccountSwitchResult,
};
use proto::app::{AppInfo, AppInfoResult};
use proto::cache::{
    CacheClear, CacheEntry, CacheInfo, CacheInfoResult, CacheList, CacheListResult, CacheUsage,
};
use proto::config::{
    ConfigGet, ConfigGetResult, ConfigList, ConfigListResult, ConfigSet, AUTOSTART_KEY, HOME_KEY,
};
use proto::daemon::{DaemonStatus, DaemonStatusResult, DaemonStop, DaemonStopResult};
use proto::download::DownloadStart;
use proto::events::{EventsSubscribe, EventsSubscribeResult};
use proto::health::{Ping, PingResult};
use proto::instance::{
    InstanceCreate, InstanceCreateResult, InstanceFlavors, InstanceLaunch, InstanceLaunchResult,
    InstanceList, InstanceListResult, InstanceRemove, InstanceResolve, InstanceStop,
    InstanceVersions,
};
use proto::java::{
    JavaInstall, JavaInstallResult, JavaList, JavaListResult, JavaReleases, JavaReleasesResult,
    JavaUninstall,
};
use proto::minecraft::{FlavorsResult, VersionsResult};
use proto::process::LogSource;
use proto::process::{
    ProcessList, ProcessListResult, ProcessLogs, ProcessLogsResult, ProcessSpec, ProcessStart,
    ProcessStartResult, ProcessState, ProcessStatus, ProcessStop, RestartPolicy,
};
use proto::server::{
    ServerCreate, ServerCreateResult, ServerFlavors, ServerList, ServerListResult, ServerLogs,
    ServerRemove, ServerResolve, ServerStart, ServerStartResult, ServerStatus, ServerStop,
    ServerVersions,
};
use proto::Empty;
use serde_json::{json, Value};

use crate::autostart;
use crate::runtime::{
    instance_process_id, server_process_id, Channels, Router, ServiceError, StartError,
};

pub fn make_router() -> Router {
    let mut router = Router::default();
    let mut on = Channels::new(&mut router);

    on.handle::<Ping, _, _>(|_: Empty, _ctx| async move {
        Ok(PingResult {
            status: "alive".into(),
            pid: std::process::id() as i32,
        })
    });

    on.handle::<AppInfo, _, _>(|_: Empty, _ctx| async move {
        Ok(AppInfoResult {
            name: common::app::NAME.into(),
            version: common::app::VERSION.into(),
            id: common::app::ID.into(),
            vendor: common::app::VENDOR.into(),
            channel: common::app::CHANNEL.into(),
        })
    });

    on.handle::<DaemonStatus, _, _>(|_: Empty, ctx| async move {
        Ok(DaemonStatusResult {
            pid: std::process::id() as i64,
            version: common::app::VERSION.into(),
            uptime_seconds: ctx.runtime.uptime_seconds(),
            home: ctx.runtime.engine().data_home(),
            log: ctx.runtime.log_path().clone(),
        })
    });

    on.handle::<DaemonStop, _, _>(|p, ctx| async move {
        // Stop on a short delay so this response reaches the client before the
        // serve loop shuts down.
        let runtime = ctx.runtime.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            runtime.request_stop(p.stop_processes);
        });
        Ok(DaemonStopResult { stopping: true })
    });

    on.handle::<ConfigGet, _, _>(|p, ctx| async move {
        if p.key == HOME_KEY {
            return Ok(ConfigGetResult {
                value: json!(ctx.runtime.engine().data_home().display().to_string()),
            });
        }
        if p.key == AUTOSTART_KEY {
            return Ok(ConfigGetResult {
                value: json!(autostart::is_enabled()),
            });
        }
        match ctx.runtime.engine().config().get(&p.key) {
            Ok(value) => Ok(ConfigGetResult { value }),
            Err(ConfigError::UnknownKey(m)) => {
                Err(ServiceError::not_found(format!("unknown config key: {m}")))
            }
            Err(e) => Err(ServiceError::handler_error(e.to_string())),
        }
    });

    on.handle::<ConfigSet, _, _>(|p, ctx| async move {
        if p.key == HOME_KEY {
            let Value::String(dir) = p.value else {
                return Err(ServiceError::bad_request("home expects a string"));
            };
            ctx.runtime
                .engine()
                .set_data_home(&dir)
                .map_err(|e| ServiceError::handler_error(e.to_string()))?;
            return Ok(Empty {});
        }
        if p.key == AUTOSTART_KEY {
            let Value::Bool(enabled) = p.value else {
                return Err(ServiceError::bad_request("autostart expects a boolean"));
            };
            autostart::set(enabled).map_err(|e| ServiceError::handler_error(e.to_string()))?;
            return Ok(Empty {});
        }
        ctx.runtime
            .engine()
            .config()
            .set(&p.key, p.value)
            .map_err(|e| ServiceError::bad_request(e.to_string()))?;
        Ok(Empty {})
    });

    on.handle::<ConfigList, _, _>(|_: Empty, ctx| async move {
        let mut entries = ctx.runtime.engine().config().all();
        if let Value::Object(map) = &mut entries {
            map.insert(
                HOME_KEY.into(),
                json!(ctx.runtime.engine().data_home().display().to_string()),
            );
            map.insert(AUTOSTART_KEY.into(), json!(autostart::is_enabled()));
        }
        Ok(ConfigListResult { entries })
    });

    on.handle::<CacheInfo, _, _>(|_: Empty, ctx| async move {
        let cache = ctx.runtime.engine().cache();
        let usage = cache.usage();
        Ok(CacheInfoResult {
            path: cache.dir(),
            usage: CacheUsage {
                entries: usage.entries,
                bytes: usage.bytes,
            },
        })
    });

    on.handle::<CacheList, _, _>(|_: Empty, ctx| async move {
        let entries = ctx
            .runtime
            .engine()
            .cache()
            .entries()
            .into_iter()
            .map(|e| CacheEntry {
                checksum: e.checksum,
                size: e.size,
            })
            .collect();
        Ok(CacheListResult { entries })
    });

    on.handle::<CacheClear, _, _>(|_: Empty, ctx| async move {
        let freed = ctx.runtime.engine().cache().clear();
        Ok(CacheUsage {
            entries: freed.entries,
            bytes: freed.bytes,
        })
    });

    on.handle::<JavaReleases, _, _>(|_: Empty, ctx| async move {
        let releases = ctx
            .runtime
            .engine()
            .java()
            .releases()
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        Ok(JavaReleasesResult { releases })
    });

    on.handle::<JavaList, _, _>(|_: Empty, ctx| async move {
        Ok(JavaListResult {
            runtimes: ctx.runtime.engine().java().installed(),
        })
    });

    on.handle::<JavaInstall, _, _>(|p, ctx| async move {
        if p.major <= 0 {
            return Err(ServiceError::bad_request(
                "major must be a positive integer",
            ));
        }
        match ctx.runtime.java_installs().start(p.major, p.id, p.force) {
            Some(id) => Ok(JavaInstallResult { id }),
            None => Err(ServiceError::bad_request(format!(
                "java {} is already being installed",
                p.major
            ))),
        }
    });

    on.handle::<JavaUninstall, _, _>(|p, ctx| async move {
        if p.major <= 0 {
            return Err(ServiceError::bad_request(
                "major must be a positive integer",
            ));
        }
        if ctx.runtime.engine().java().uninstall(p.major) {
            Ok(Empty {})
        } else {
            Err(ServiceError::not_found(format!(
                "no installed java runtime for major {}",
                p.major
            )))
        }
    });

    on.handle::<DownloadStart, _, _>(|spec, ctx| async move {
        if spec.url.is_empty() {
            return Err(ServiceError::bad_request("download url is empty"));
        }
        let id = ctx.runtime.downloads().start(spec);
        Ok(proto::download::DownloadStartResult { id })
    });

    on.handle::<AccountLoginBegin, _, _>(|p, ctx| async move {
        let challenge = ctx
            .runtime
            .engine()
            .accounts()
            .begin_login(p.method)
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        Ok(AccountLoginBeginResult {
            id: challenge.id,
            method: challenge.method,
            url: challenge.url,
            user_code: challenge.user_code,
            verification_uri: challenge.verification_uri,
        })
    });

    on.handle::<AccountLoginComplete, _, _>(|p, ctx| async move {
        let account = ctx
            .runtime
            .engine()
            .accounts()
            .complete_login(&p.id, &p.code)
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        Ok(AccountLoginCompleteResult { account })
    });

    on.handle::<AccountList, _, _>(|_: Empty, ctx| async move {
        let accounts = ctx.runtime.engine().accounts();
        Ok(AccountListResult {
            accounts: accounts.list(),
            default_uuid: accounts
                .default_account()
                .map(|a| a.uuid)
                .unwrap_or_default(),
        })
    });

    on.handle::<AccountSwitch, _, _>(|p, ctx| async move {
        match ctx.runtime.engine().accounts().switch(&p.account) {
            Ok(Some(account)) => Ok(AccountSwitchResult { account }),
            Ok(None) => Err(ServiceError::not_found(format!(
                "no account matches '{}'",
                p.account
            ))),
            Err(e) => Err(ServiceError::handler_error(e.to_string())),
        }
    });

    on.handle::<AccountRemove, _, _>(|p, ctx| async move {
        match ctx.runtime.engine().accounts().remove(&p.account) {
            Ok(true) => Ok(Empty {}),
            Ok(false) => Err(ServiceError::not_found(format!(
                "no account matches '{}'",
                p.account
            ))),
            Err(e) => Err(ServiceError::handler_error(e.to_string())),
        }
    });

    on.handle::<ProcessStart, _, _>(|spec, ctx| async move {
        match ctx.runtime.processes().start(spec).await {
            Ok(info) => Ok(ProcessStartResult {
                id: info.id,
                pid: info.pid,
            }),
            Err(StartError::EmptyProgram) => Err(ServiceError::bad_request("program is empty")),
            Err(StartError::InvalidId) => Err(ServiceError::bad_request(
                "process id may only contain letters, digits, '-', '_' and '.'",
            )),
            Err(StartError::Spawn(e)) => Err(ServiceError::handler_error(format!(
                "cannot spawn process: {e}"
            ))),
        }
    });

    on.handle::<ProcessStop, _, _>(|p, ctx| async move {
        if ctx.runtime.processes().stop(&p.id) {
            Ok(Empty {})
        } else {
            Err(ServiceError::not_found(format!("no process '{}'", p.id)))
        }
    });

    on.handle::<ProcessList, _, _>(|_: Empty, ctx| async move {
        Ok(ProcessListResult {
            processes: ctx.runtime.processes().list(),
        })
    });

    on.handle::<ProcessStatus, _, _>(|p, ctx| async move {
        ctx.runtime
            .processes()
            .status(&p.id)
            .ok_or_else(|| ServiceError::not_found(format!("no process '{}'", p.id)))
    });

    on.handle::<ProcessLogs, _, _>(|p, ctx| async move {
        match ctx.runtime.processes().logs(&p.id, p.tail) {
            Some(lines) => Ok(ProcessLogsResult { lines }),
            None => Err(ServiceError::not_found(format!("no process '{}'", p.id))),
        }
    });

    on.handle::<EventsSubscribe, _, _>(|p, ctx| async move {
        let filter = if p.id.is_empty() { None } else { Some(p.id) };
        ctx.runtime
            .hub()
            .subscribe(ctx.conn_id, ctx.out.clone(), filter);
        Ok(EventsSubscribeResult { subscribed: true })
    });

    on.handle::<ServerFlavors, _, _>(|_: Empty, ctx| async move {
        Ok(FlavorsResult {
            flavors: ctx.runtime.engine().minecraft().server_flavors(),
        })
    });

    on.handle::<ServerVersions, _, _>(|p, ctx| async move {
        let versions = ctx
            .runtime
            .engine()
            .minecraft()
            .server_versions(&p.flavor)
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        Ok(VersionsResult { versions })
    });

    on.handle::<ServerResolve, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .minecraft()
            .resolve_server(&p.flavor, &p.version, p.loader_version)
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))
    });

    on.handle::<InstanceFlavors, _, _>(|_: Empty, ctx| async move {
        Ok(FlavorsResult {
            flavors: ctx.runtime.engine().minecraft().instance_flavors(),
        })
    });

    on.handle::<InstanceVersions, _, _>(|p, ctx| async move {
        let versions = ctx
            .runtime
            .engine()
            .minecraft()
            .instance_versions(&p.flavor)
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        Ok(VersionsResult { versions })
    });

    on.handle::<InstanceResolve, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .minecraft()
            .resolve_instance(&p.flavor, &p.version, p.loader_version)
            .await
            .map_err(|e| ServiceError::handler_error(e.to_string()))
    });

    on.handle::<ServerCreate, _, _>(|p, ctx| async move {
        if p.flavor.is_empty() || p.version.is_empty() {
            return Err(ServiceError::bad_request("flavor and version are required"));
        }
        if !p.eula {
            return Err(ServiceError::bad_request(
                "creating a server requires accepting the Minecraft EULA",
            ));
        }
        match ctx.runtime.server_creates().start(p) {
            Some(id) => Ok(ServerCreateResult { id }),
            None => Err(ServiceError::bad_request(
                "that server is already being created",
            )),
        }
    });

    on.handle::<ServerList, _, _>(|_: Empty, ctx| async move {
        let servers = ctx
            .runtime
            .engine()
            .servers()
            .list()
            .into_iter()
            .map(|r| ctx.runtime.server_view(r))
            .collect();
        Ok(ServerListResult { servers })
    });

    on.handle::<ServerStatus, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        Ok(ctx.runtime.server_view(record))
    });

    on.handle::<ServerRemove, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        if is_running(&ctx, &server_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is running; stop it first",
                record.name
            )));
        }
        ctx.runtime
            .engine()
            .servers()
            .remove(&record.id)
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        ctx.runtime
            .processes()
            .discard(&server_process_id(&record.id));
        Ok(Empty {})
    });

    on.handle::<ServerStart, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        if is_running(&ctx, &process_id) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is already running",
                record.name
            )));
        }
        let (_, plan) = ctx
            .runtime
            .engine()
            .server_launch_plan(&record.id)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        let spec = ProcessSpec {
            id: process_id,
            program: plan.program.to_string_lossy().into_owned(),
            args: plan.args,
            log: LogSource::File(plan.cwd.join("logs").join("latest.log")),
            cwd: Some(plan.cwd),
            env: Default::default(),
            restart: RestartPolicy::Never,
        };
        match ctx.runtime.processes().start(spec).await {
            Ok(info) => Ok(ServerStartResult {
                process_id: info.id,
                pid: info.pid,
            }),
            Err(StartError::EmptyProgram | StartError::InvalidId) => {
                Err(ServiceError::bad_request("invalid launch plan"))
            }
            Err(StartError::Spawn(e)) => Err(ServiceError::handler_error(format!(
                "cannot spawn the server: {e}"
            ))),
        }
    });

    on.handle::<ServerStop, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        if !is_running(&ctx, &process_id) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is not running",
                record.name
            )));
        }
        ctx.runtime.processes().stop(&process_id);
        Ok(Empty {})
    });

    on.handle::<ServerLogs, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let lines = ctx
            .runtime
            .processes()
            .logs(&server_process_id(&record.id), p.tail)
            .unwrap_or_default();
        Ok(ProcessLogsResult { lines })
    });

    on.handle::<InstanceCreate, _, _>(|p, ctx| async move {
        if p.flavor.is_empty() || p.version.is_empty() {
            return Err(ServiceError::bad_request("flavor and version are required"));
        }
        let record = ctx
            .runtime
            .engine()
            .create_instance(&p.name, &p.flavor, &p.version, p.loader_version)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(InstanceCreateResult {
            instance: ctx.runtime.instance_view(record),
        })
    });

    on.handle::<InstanceList, _, _>(|_: Empty, ctx| async move {
        let instances = ctx
            .runtime
            .engine()
            .instances()
            .list()
            .into_iter()
            .map(|r| ctx.runtime.instance_view(r))
            .collect();
        Ok(InstanceListResult { instances })
    });

    on.handle::<InstanceRemove, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        if is_running(&ctx, &instance_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "instance '{}' is running; stop it first",
                record.name
            )));
        }
        ctx.runtime
            .engine()
            .instances()
            .remove(&record.id)
            .map_err(|e| ServiceError::handler_error(e.to_string()))?;
        ctx.runtime
            .processes()
            .discard(&instance_process_id(&record.id));
        Ok(Empty {})
    });

    on.handle::<InstanceLaunch, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        if is_running(&ctx, &instance_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "instance '{}' is already running",
                record.name
            )));
        }
        match ctx
            .runtime
            .instance_launches()
            .start(record.id, p.account, p.id)
        {
            Some(id) => Ok(InstanceLaunchResult { id }),
            None => Err(ServiceError::bad_request(
                "that instance is already launching",
            )),
        }
    });

    on.handle::<InstanceStop, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        if !is_running(&ctx, &process_id) {
            return Err(ServiceError::bad_request(format!(
                "instance '{}' is not running",
                record.name
            )));
        }
        ctx.runtime.processes().stop(&process_id);
        Ok(Empty {})
    });

    router
}

fn find_server(
    ctx: &crate::runtime::HandlerContext,
    reference: &str,
) -> Result<engine::ServerRecord, ServiceError> {
    ctx.runtime
        .engine()
        .servers()
        .get(reference)
        .ok_or_else(|| ServiceError::not_found(format!("no server matches '{reference}'")))
}

fn find_instance(
    ctx: &crate::runtime::HandlerContext,
    reference: &str,
) -> Result<engine::InstanceRecord, ServiceError> {
    ctx.runtime
        .engine()
        .instances()
        .get(reference)
        .ok_or_else(|| ServiceError::not_found(format!("no instance matches '{reference}'")))
}

fn is_running(ctx: &crate::runtime::HandlerContext, process_id: &str) -> bool {
    ctx.runtime
        .processes()
        .status(process_id)
        .is_some_and(|info| info.state == ProcessState::Running)
}
