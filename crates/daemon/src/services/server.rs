//! Minecraft servers: the provider catalogue, the lifecycle over the supervisor,
//! the rcon console, and the per-server settings. Backups live in `backup`,
//! content installs in `content`.

use proto::minecraft::{ConfigEntry, FlavorsResult, LoadersResult, VersionsResult};
use proto::process::{LogSource, ProcessLogsResult, ProcessSpec, RestartPolicy};
use proto::server::{
    ServerCommand, ServerCommandResult, ServerConfigGet, ServerConfigGetResult, ServerConfigList,
    ServerConfigListResult, ServerConfigSet, ServerCreate, ServerCreateResult, ServerFlavors,
    ServerList, ServerListResult, ServerLoaders, ServerLogs, ServerPing, ServerRemove,
    ServerRename, ServerResolve, ServerStart, ServerStartResult, ServerStatus, ServerStop,
    ServerUpdate, ServerUpdateResult, ServerVersions,
};
use proto::Empty;

use super::guards::{
    ensure_no_backup, ensure_no_content, ensure_no_update, find_server, is_running,
};
use crate::runtime::{server_process_id, Channels, ServiceError, StartError};

pub(super) fn register(on: &mut Channels<'_>) {
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
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(VersionsResult { versions })
    });

    on.handle::<ServerResolve, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .minecraft()
            .resolve_server(&p.flavor, &p.version, p.loader_version)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))
    });

    on.handle::<ServerLoaders, _, _>(|p, ctx| async move {
        let loaders = ctx
            .runtime
            .engine()
            .minecraft()
            .server_loader_versions(&p.flavor, &p.version)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(LoadersResult { loaders })
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

    on.handle::<ServerUpdate, _, _>(|p, ctx| async move {
        if p.version.is_empty() {
            return Err(ServiceError::bad_request("version is required"));
        }
        let record = find_server(&ctx, &p.server)?;
        if is_running(&ctx, &server_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is running; stop it first",
                record.name
            )));
        }
        if ctx.runtime.server_creates().in_flight(&record.name) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is still being created",
                record.name
            )));
        }
        ensure_no_backup(&ctx, &server_process_id(&record.id), &record.name)?;
        match ctx.runtime.server_updates().start(record.id, p) {
            Some(id) => Ok(ServerUpdateResult { id }),
            None => Err(ServiceError::bad_request(
                "that server is already being updated",
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
        let mut view = ctx.runtime.server_view(record);
        if p.with_usage {
            view.disk_bytes = ctx.runtime.engine().server_disk_usage(&view.id).ok();
        }
        Ok(view)
    });

    on.handle::<ServerPing, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        ctx.runtime
            .engine()
            .server_ping(&record.id)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))
    });

    on.handle::<ServerRemove, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        if is_running(&ctx, &server_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is running; stop it first",
                record.name
            )));
        }
        ensure_no_backup(&ctx, &server_process_id(&record.id), &record.name)?;
        ctx.runtime
            .engine()
            .servers()
            .remove(&record.id)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        ctx.runtime
            .processes()
            .discard(&server_process_id(&record.id));
        tracing::info!(server = %record.id, name = %record.name, "server removed");
        Ok(Empty {})
    });

    on.handle::<ServerRename, _, _>(|p, ctx| async move {
        if p.name.trim().is_empty() {
            return Err(ServiceError::bad_request("a new name is required"));
        }
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        if is_running(&ctx, &process_id) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is running; stop it first",
                record.name
            )));
        }
        if ctx.runtime.server_creates().in_flight(&record.name) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is still being created",
                record.name
            )));
        }
        ensure_no_update(&ctx, &record.id, &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_content(&ctx, &process_id, &record.name)?;
        let renamed = ctx
            .runtime
            .engine()
            .servers()
            .rename(&record.id, &p.name)
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        tracing::info!(id = %renamed.id, name = %renamed.name, "server renamed");
        Ok(ctx.runtime.server_view(renamed))
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
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        tracing::info!(server = %record.id, name = %record.name, "starting server");
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

    on.handle::<ServerCommand, _, _>(|p, ctx| async move {
        if p.command.trim().is_empty() {
            return Err(ServiceError::bad_request("command is empty"));
        }
        let record = find_server(&ctx, &p.server)?;
        if !is_running(&ctx, &server_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "server '{}' is not running",
                record.name
            )));
        }
        let response = ctx
            .runtime
            .engine()
            .server_command(&record.id, &p.command)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(ServerCommandResult { response })
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

    on.handle::<ServerConfigGet, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        match ctx
            .runtime
            .engine()
            .servers()
            .config_get(&record.id, &p.key)
        {
            Ok(Some(value)) => Ok(ServerConfigGetResult { value }),
            Ok(None) => Err(ServiceError::not_found(format!("'{}' is not set", p.key))),
            Err(e) => Err(ServiceError::bad_request(format!("{e:#}"))),
        }
    });

    on.handle::<ServerConfigSet, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        ctx.runtime
            .engine()
            .servers()
            .config_set(&record.id, &p.key, &p.value)
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        tracing::info!(server = %record.id, key = %p.key, "server config updated");
        Ok(Empty {})
    });

    on.handle::<ServerConfigList, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let entries = ctx
            .runtime
            .engine()
            .servers()
            .config_list(&record.id)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?
            .into_iter()
            .map(|(key, value)| ConfigEntry { key, value })
            .collect();
        Ok(ServerConfigListResult { entries })
    });
}
