//! Minecraft instances (clients): the provider catalogue, the record lifecycle,
//! launch over the supervisor, and the per-instance JVM settings. Backups live
//! in `backup`, content installs in `content`.

use proto::error::{EntryKind, ErrorInfo, Field, ProfileScope};
use proto::instance::{
    InstanceConfigGet, InstanceConfigGetResult, InstanceConfigList, InstanceConfigListResult,
    InstanceConfigSet, InstanceCreate, InstanceCreateResult, InstanceFlavors, InstanceInfoQuery,
    InstanceLaunch, InstanceLaunchResult, InstanceList, InstanceListResult, InstanceLoaders,
    InstanceLogs, InstanceProfileCapture, InstanceProfileCreate, InstanceProfileEdit,
    InstanceProfileList, InstanceProfileListResult, InstanceProfileRelease, InstanceProfileRemove,
    InstanceProfileRename, InstanceProfileUse, InstanceRemove, InstanceRename, InstanceResolve,
    InstanceStop, InstanceUpdate, InstanceUpdateResult, InstanceVersions, InstanceWorlds,
    InstanceWorldsResult,
};
use proto::minecraft::{ConfigEntry, FlavorsResult, LoadersResult, VersionsResult};
use proto::process::ProcessLogsResult;
use proto::Empty;

use super::guards::{ensure_no_content, ensure_stopped, find_instance};
use crate::runtime::{instance_process_id, Channels};

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
            .map_err(crate::runtime::internal)?;
        Ok(VersionsResult { versions })
    });

    on.handle::<InstanceResolve, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .minecraft()
            .resolve_instance(&p.flavor, &p.version, p.loader_version)
            .await
            .map_err(crate::runtime::internal)
    });

    on.handle::<InstanceLoaders, _, _>(|p, ctx| async move {
        let loaders = ctx
            .runtime
            .engine()
            .minecraft()
            .instance_loader_versions(&p.flavor, &p.version)
            .await
            .map_err(crate::runtime::internal)?;
        Ok(LoadersResult { loaders })
    });

    on.handle::<InstanceCreate, _, _>(|p, ctx| async move {
        if p.flavor.is_empty() || p.version.is_empty() {
            return Err(ErrorInfo::FieldsRequired {
                fields: vec![Field::Flavor, Field::Version],
            });
        }
        let record = ctx
            .runtime
            .engine()
            .create_instance(&p.name, &p.flavor, &p.version, p.loader_version, &p.config)
            .await
            .map_err(crate::runtime::internal)?;
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
            return Err(ErrorInfo::FieldRequired {
                field: Field::Version,
            });
        }
        let record = find_instance(&ctx, &p.instance)?;
        if ctx.runtime.instance_running(&record.id) {
            return Err(ErrorInfo::EntryRunning {
                entry: EntryKind::Instance,
                name: record.name.clone(),
            });
        }
        let record = ctx
            .runtime
            .engine()
            .update_instance(&record.id, &p.version, p.loader_version, p.allow_downgrade)
            .await
            .map_err(crate::runtime::internal)?;
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

    on.handle::<InstanceInfoQuery, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ctx.runtime
            .engine()
            .instance_detail(&record.id)
            .map_err(crate::runtime::internal)
    });

    on.handle::<InstanceWorlds, _, _>(|p, ctx| async move {
        let worlds = ctx
            .runtime
            .engine()
            .instance_worlds(&p.instance)
            .map_err(|_| ErrorInfo::EntryNotFound {
                entry: EntryKind::Instance,
                reference: p.instance.clone(),
            })?;
        Ok(InstanceWorldsResult { worlds })
    });

    on.handle::<InstanceRemove, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        if ctx.runtime.instance_running(&record.id) {
            return Err(ErrorInfo::EntryRunning {
                entry: EntryKind::Instance,
                name: record.name.clone(),
            });
        }
        ctx.runtime
            .engine()
            .instances()
            .remove(&record.id)
            .map_err(crate::runtime::internal)?;
        ctx.runtime.discard_instance_sessions(&record.id);
        tracing::info!(instance = %record.id, name = %record.name, "instance removed");
        Ok(Empty {})
    });

    on.handle::<InstanceRename, _, _>(|p, ctx| async move {
        if p.name.trim().is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Name });
        }
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        if ctx.runtime.instance_running(&record.id) {
            return Err(ErrorInfo::EntryRunning {
                entry: EntryKind::Instance,
                name: record.name.clone(),
            });
        }
        ensure_no_content(&ctx, &process_id, &record.name)?;
        let renamed = ctx
            .runtime
            .engine()
            .instances()
            .rename(&record.id, &p.name)
            .map_err(crate::runtime::internal)?;
        tracing::info!(id = %renamed.id, name = %renamed.name, "instance renamed");
        Ok(ctx.runtime.instance_view(renamed))
    });

    on.handle::<InstanceLaunch, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        // The account's tokens can no longer be refreshed: block up front so a
        // dead sign-in prompts re-login instead of failing mid-launch.
        if ctx.runtime.engine().accounts().needs_reauth(&p.account) {
            return Err(ErrorInfo::SessionExpired {
                reference: p.account.clone(),
            });
        }
        // Concurrent sessions are opt-in: by default a running instance is
        // refused, and `new_session` unlocks a second (or third) launch.
        let running = ctx.runtime.instance_running(&record.id);
        if !p.new_session && running {
            return Err(ErrorInfo::EntryRunning {
                entry: EntryKind::Instance,
                name: record.name.clone(),
            });
        }
        // A concurrent session runs against the mirror the live sessions use
        // (the reconcile is skipped), so a profile override that differs from
        // the active one cannot be honoured.
        if running && !p.profile.is_empty() {
            let (active, _) = ctx
                .runtime
                .engine()
                .instance_profiles(&record.id)
                .map_err(crate::runtime::internal)?;
            let requested = if p.profile == "none" { "" } else { &p.profile };
            if !requested.eq_ignore_ascii_case(&active) {
                return Err(ErrorInfo::EntryRunning {
                    entry: EntryKind::Instance,
                    name: record.name.clone(),
                });
            }
        }
        match ctx
            .runtime
            .instance_launches()
            .start(record.id, p.account, p.profile, !running, p.id)
        {
            Some(id) => Ok(InstanceLaunchResult { id }),
            None => Err(ErrorInfo::Internal {
                detail: "that instance could not be launched".into(),
            }),
        }
    });

    on.handle::<InstanceStop, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let sessions = ctx.runtime.instance_sessions(&record.id);
        match p.session {
            // Stop one named session, refusing an id that is not this instance's.
            Some(session) => {
                if !sessions.iter().any(|s| s.id == session) {
                    return Err(ErrorInfo::ProcessNotFound {
                        id: session.clone(),
                    });
                }
                ctx.runtime.processes().stop(&session);
            }
            None => {
                let stopped = ctx.runtime.stop_instance_sessions(&record.id);
                if stopped == 0 {
                    return Err(ErrorInfo::NotRunning {
                        entry: EntryKind::Instance,
                        name: record.name.clone(),
                    });
                }
            }
        }
        Ok(Empty {})
    });

    on.handle::<InstanceLogs, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        // A specific session, else the newest running one, else the newest.
        let sessions = ctx.runtime.instance_sessions(&record.id);
        let target = match &p.session {
            Some(session) => sessions
                .iter()
                .find(|s| &s.id == session)
                .map(|s| s.id.clone()),
            None => sessions
                .iter()
                .find(|s| s.state == proto::process::ProcessState::Running)
                .or_else(|| sessions.first())
                .map(|s| s.id.clone()),
        };
        let lines = target
            .and_then(|id| ctx.runtime.processes().logs(&id, p.tail))
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
            Ok(None) => Err(ErrorInfo::ConfigKeyUnset { key: p.key.clone() }),
            Err(e) => Err(crate::runtime::internal(e)),
        }
    });

    on.handle::<InstanceConfigSet, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ctx.runtime
            .engine()
            .instances()
            .config_set(&record.id, &p.key, &p.value)
            .map_err(crate::runtime::internal)?;
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
            .map_err(crate::runtime::internal)?
            .into_iter()
            .map(|(key, value)| ConfigEntry { key, value })
            .collect();
        Ok(InstanceConfigListResult { entries })
    });

    // Profile CRUD is metadata-safe while the instance runs (a change applies
    // at the next launch); only seeding reads the pool, so only create guards
    // against an in-flight content job.
    on.handle::<InstanceProfileList, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let (active, profiles) = ctx
            .runtime
            .engine()
            .instance_profiles(&record.id)
            .map_err(crate::runtime::internal)?;
        Ok(InstanceProfileListResult { active, profiles })
    });

    on.handle::<InstanceProfileCreate, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        if p.seed_from_pool {
            ensure_no_content(&ctx, &instance_process_id(&record.id), &record.name)?;
        }
        let profile = ctx
            .runtime
            .engine()
            .create_instance_profile(&record.id, &p.name, p.seed_from_pool)
            .map_err(crate::runtime::internal)?;
        tracing::info!(instance = %record.id, profile = %profile.name, "profile created");
        Ok(profile)
    });

    on.handle::<InstanceProfileRemove, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ctx.runtime
            .engine()
            .remove_instance_profile(&record.id, &p.name)
            .map_err(|_| ErrorInfo::ProfileNotFound {
                scope: ProfileScope::Instance,
                name: p.name.clone(),
            })?;
        tracing::info!(instance = %record.id, profile = %p.name, "profile removed");
        Ok(Empty {})
    });

    on.handle::<InstanceProfileRename, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ctx.runtime
            .engine()
            .rename_instance_profile(&record.id, &p.name, &p.new_name)
            .map_err(crate::runtime::internal)
    });

    on.handle::<InstanceProfileUse, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ctx.runtime
            .engine()
            .use_instance_profile(&record.id, &p.name)
            .map_err(crate::runtime::internal)?;
        tracing::info!(instance = %record.id, profile = %p.name, "active profile changed");
        Ok(Empty {})
    });

    on.handle::<InstanceProfileEdit, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ctx.runtime
            .engine()
            .edit_instance_profile(&record.id, &p.name, &p.add, &p.remove)
            .map_err(crate::runtime::internal)
    });

    // Capture/release move real settings trees (and a released store may be
    // what a live session's `config` link writes through), so both require a
    // stopped instance — unlike the metadata-only CRUD above.
    on.handle::<InstanceProfileCapture, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ensure_stopped(
            &ctx,
            &instance_process_id(&record.id),
            "instance",
            &record.name,
        )?;
        ctx.runtime
            .engine()
            .capture_instance_profile(&record.id, &p.name)
            .map_err(crate::runtime::internal)?;
        tracing::info!(instance = %record.id, profile = %p.name, "profile settings captured");
        Ok(Empty {})
    });

    on.handle::<InstanceProfileRelease, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        ensure_stopped(
            &ctx,
            &instance_process_id(&record.id),
            "instance",
            &record.name,
        )?;
        ctx.runtime
            .engine()
            .release_instance_profile(&record.id, &p.name)
            .map_err(crate::runtime::internal)?;
        tracing::info!(instance = %record.id, profile = %p.name, "profile settings released");
        Ok(Empty {})
    });
}
