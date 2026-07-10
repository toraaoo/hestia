//! Minecraft instances (clients): the provider catalogue, the record lifecycle,
//! launch over the supervisor, and the per-instance JVM settings. Backups live
//! in `backup`, content installs in `content`.

use proto::instance::{
    InstanceConfigGet, InstanceConfigGetResult, InstanceConfigList, InstanceConfigListResult,
    InstanceConfigSet, InstanceCreate, InstanceCreateResult, InstanceFlavors, InstanceLaunch,
    InstanceLaunchResult, InstanceList, InstanceListResult, InstanceLogs, InstanceRemove,
    InstanceResolve, InstanceStop, InstanceUpdate, InstanceUpdateResult, InstanceVersions,
    InstanceWorlds, InstanceWorldsResult,
};
use proto::minecraft::{ConfigEntry, FlavorsResult, VersionsResult};
use proto::process::ProcessLogsResult;
use proto::Empty;

use super::guards::{ensure_no_backup, find_instance, is_running};
use crate::runtime::{instance_process_id, Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
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
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(VersionsResult { versions })
    });

    on.handle::<InstanceResolve, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .minecraft()
            .resolve_instance(&p.flavor, &p.version, p.loader_version)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))
    });

    on.handle::<InstanceCreate, _, _>(|p, ctx| async move {
        if p.flavor.is_empty() || p.version.is_empty() {
            return Err(ServiceError::bad_request("flavor and version are required"));
        }
        let record = ctx
            .runtime
            .engine()
            .create_instance(&p.name, &p.flavor, &p.version, p.loader_version, &p.config)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        tracing::info!(
            instance = %record.id,
            name = %record.name,
            flavor = %record.profile.flavor,
            version = %record.profile.game_version,
            "instance created"
        );
        Ok(InstanceCreateResult {
            instance: ctx.runtime.instance_view(record),
        })
    });

    on.handle::<InstanceUpdate, _, _>(|p, ctx| async move {
        if p.version.is_empty() {
            return Err(ServiceError::bad_request("version is required"));
        }
        let record = find_instance(&ctx, &p.instance)?;
        if is_running(&ctx, &instance_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "instance '{}' is running; stop it first",
                record.name
            )));
        }
        ensure_no_backup(&ctx, &instance_process_id(&record.id), &record.name)?;
        let record = ctx
            .runtime
            .engine()
            .update_instance(&record.id, &p.version, p.loader_version, p.allow_downgrade)
            .await
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        tracing::info!(
            instance = %record.id,
            version = %record.profile.game_version,
            "instance updated"
        );
        Ok(InstanceUpdateResult {
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

    on.handle::<InstanceWorlds, _, _>(|p, ctx| async move {
        let worlds = ctx
            .runtime
            .engine()
            .instance_worlds(&p.instance)
            .map_err(|e| ServiceError::not_found(format!("{e:#}")))?;
        Ok(InstanceWorldsResult { worlds })
    });

    on.handle::<InstanceRemove, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        if is_running(&ctx, &instance_process_id(&record.id)) {
            return Err(ServiceError::bad_request(format!(
                "instance '{}' is running; stop it first",
                record.name
            )));
        }
        ensure_no_backup(&ctx, &instance_process_id(&record.id), &record.name)?;
        ctx.runtime
            .engine()
            .instances()
            .remove(&record.id)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        ctx.runtime
            .processes()
            .discard(&instance_process_id(&record.id));
        tracing::info!(instance = %record.id, name = %record.name, "instance removed");
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
        ensure_no_backup(&ctx, &instance_process_id(&record.id), &record.name)?;
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

    on.handle::<InstanceLogs, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let lines = ctx
            .runtime
            .processes()
            .logs(&instance_process_id(&record.id), p.tail)
            .unwrap_or_default();
        Ok(ProcessLogsResult { lines })
    });

    on.handle::<InstanceConfigGet, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        match ctx
            .runtime
            .engine()
            .instances()
            .config_get(&record.id, &p.key)
        {
            Ok(Some(value)) => Ok(InstanceConfigGetResult { value }),
            Ok(None) => Err(ServiceError::not_found(format!("'{}' is not set", p.key))),
            Err(e) => Err(ServiceError::bad_request(format!("{e:#}"))),
        }
    });

    on.handle::<InstanceConfigSet, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ctx.runtime
            .engine()
            .instances()
            .config_set(&record.id, &p.key, &p.value)
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        tracing::info!(instance = %record.id, key = %p.key, "instance config updated");
        Ok(Empty {})
    });

    on.handle::<InstanceConfigList, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let entries = ctx
            .runtime
            .engine()
            .instances()
            .config_list(&record.id)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?
            .into_iter()
            .map(|(key, value)| ConfigEntry { key, value })
            .collect();
        Ok(InstanceConfigListResult { entries })
    });
}
