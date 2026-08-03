//! Minecraft instances (clients): the provider catalogue, the record lifecycle,
//! launch over the supervisor, and the per-instance JVM settings. Backups live
//! in `backup`, content installs in `content`.

use proto::error::{EntryKind, ErrorInfo, Field, ProfileScope};
use proto::instance::{
    AddressPing, InstanceConfigGet, InstanceConfigGetResult, InstanceConfigList,
    InstanceConfigListResult, InstanceConfigSet, InstanceCreate, InstanceCreateResult,
    InstanceFlavors, InstanceInfoQuery, InstanceLaunch, InstanceLaunchResult, InstanceList,
    InstanceListResult, InstanceLoaders, InstanceLogs, InstanceProfileCapture,
    InstanceProfileCreate, InstanceProfileEdit, InstanceProfileList, InstanceProfileListResult,
    InstanceProfileRelease, InstanceProfileRemove, InstanceProfileRename, InstanceProfileUse,
    InstanceRemove, InstanceRename, InstanceResolve, InstanceServerEdit, InstanceServerMove,
    InstanceServerRemove, InstanceServers, InstanceServersResult, InstanceServersWriteResult,
    InstanceStop, InstanceUpdate, InstanceUpdateResult, InstanceVersions, InstanceWorlds,
    InstanceWorldsResult, ServerEntry,
};
use proto::minecraft::{ConfigEntry, FlavorsResult, LoadersResult, VersionsResult};
use proto::process::ProcessLogsResult;
use proto::Empty;

use super::guards::{instance_for, Intent};
use crate::runtime::{Channels, LaunchOrder};

/// The shared shape of a multiplayer-list write: the list as it now stands,
/// carrying whatever the write could not guarantee.
fn server_list_result(written: engine::ServerListWrite) -> InstanceServersWriteResult {
    InstanceServersWriteResult {
        servers: written.servers,
        warnings: written.warnings,
    }
}

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<InstanceFlavors, _, _>(|_: Empty, ctx| async move {
        Ok(FlavorsResult {
            flavors: ctx.runtime.engine().instance_flavors().await,
        })
    });

    on.handle::<InstanceVersions, _, _>(|p, ctx| async move {
        let versions = ctx
            .runtime
            .engine()
            .minecraft()
            .instance_versions(&p.flavor)
            .await
            .map_err(crate::runtime::engine_error)?;
        Ok(VersionsResult { versions })
    });

    on.handle::<InstanceResolve, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .minecraft()
            .resolve_instance(&p.flavor, &p.version, p.loader_version)
            .await
            .map_err(crate::runtime::engine_error)
    });

    on.handle::<InstanceLoaders, _, _>(|p, ctx| async move {
        let loaders = ctx
            .runtime
            .engine()
            .minecraft()
            .instance_loader_versions(&p.flavor, &p.version)
            .await
            .map_err(crate::runtime::engine_error)?;
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
            .map_err(crate::runtime::engine_error)?;
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
        let record = instance_for(&ctx, &p.instance, Intent::Lifecycle)?;
        let record = ctx
            .runtime
            .engine()
            .update_instance(&record.id, &p.version, p.loader_version, p.allow_downgrade)
            .await
            .map_err(crate::runtime::engine_error)?;
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
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        ctx.runtime
            .engine()
            .instance_detail(&record.id)
            .map_err(crate::runtime::engine_error)
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

    on.handle::<InstanceServers, _, _>(|p, ctx| async move {
        let servers = ctx
            .runtime
            .engine()
            .instance_servers(&p.instance)
            .map_err(crate::runtime::engine_error)?;
        Ok(InstanceServersResult { servers })
    });

    on.handle::<InstanceServerEdit, _, _>(|p, ctx| async move {
        let written = ctx
            .runtime
            .engine()
            .edit_instance_server(
                &p.instance,
                &p.server,
                ServerEntry {
                    name: p.name,
                    address: p.address,
                    accept_textures: p.accept_textures,
                    ..ServerEntry::default()
                },
            )
            .map_err(crate::runtime::engine_error)?;
        Ok(server_list_result(written))
    });

    on.handle::<InstanceServerRemove, _, _>(|p, ctx| async move {
        let written = ctx
            .runtime
            .engine()
            .remove_instance_server(&p.instance, &p.server)
            .map_err(crate::runtime::engine_error)?;
        Ok(server_list_result(written))
    });

    on.handle::<InstanceServerMove, _, _>(|p, ctx| async move {
        let written = ctx
            .runtime
            .engine()
            .move_instance_server(&p.instance, &p.server, p.position)
            .map_err(crate::runtime::engine_error)?;
        Ok(server_list_result(written))
    });

    on.handle::<AddressPing, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .ping_address(&p.address)
            .await
            .map_err(crate::runtime::engine_error)
    });

    on.handle::<InstanceRemove, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Lifecycle)?;
        ctx.runtime
            .engine()
            .instances()
            .remove(&record.id)
            .map_err(crate::runtime::engine_error)?;
        ctx.runtime.discard_instance_sessions(&record.id);
        tracing::info!(instance = %record.id, name = %record.name, "instance removed");
        Ok(Empty {})
    });

    on.handle::<InstanceRename, _, _>(|p, ctx| async move {
        if p.name.trim().is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Name });
        }
        let record = instance_for(&ctx, &p.instance, Intent::Lifecycle)?;
        let renamed = ctx
            .runtime
            .engine()
            .instances()
            .rename(&record.id, &p.name)
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(id = %renamed.id, name = %renamed.name, "instance renamed");
        Ok(ctx.runtime.instance_view(renamed))
    });

    on.handle::<InstanceLaunch, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Start)?;
        // The account's tokens can no longer be refreshed: block up front so a
        // dead sign-in prompts re-login instead of failing mid-launch.
        if ctx.runtime.engine().accounts().needs_reauth(&p.account) {
            return Err(ErrorInfo::SessionExpired {
                reference: p.account.clone(),
            });
        }
        // Concurrent sessions are gated twice: `instance.multi-session` has to
        // allow them at all, and the launch opts in with `new_session`.
        let running = ctx.runtime.instance_running(&record.id);
        if running {
            if !p.new_session {
                return Err(ErrorInfo::EntryRunning {
                    entry: EntryKind::Instance,
                    name: record.name.clone(),
                });
            }
            if !ctx
                .runtime
                .engine()
                .config()
                .settings()
                .instance
                .multi_session
            {
                return Err(ErrorInfo::MultiSessionDisabled {
                    name: record.name.clone(),
                });
            }
        }
        // A concurrent session runs against the mirror the live sessions use
        // (the reconcile is skipped), so a profile override that differs from
        // the active one cannot be honoured.
        if running && !p.profile.is_empty() {
            let (active, _) = ctx
                .runtime
                .engine()
                .instance_profiles(&record.id)
                .map_err(crate::runtime::engine_error)?;
            let requested = if p.profile == "none" { "" } else { &p.profile };
            if !requested.eq_ignore_ascii_case(&active) {
                return Err(ErrorInfo::EntryRunning {
                    entry: EntryKind::Instance,
                    name: record.name.clone(),
                });
            }
        }
        match ctx.runtime.instance_launches().start(LaunchOrder {
            instance_id: record.id,
            account: p.account,
            profile: p.profile,
            reconcile: !running,
            quick_play: p.quick_play,
            id: p.id,
        }) {
            Some(id) => Ok(InstanceLaunchResult { id }),
            None => Err(ErrorInfo::Internal {
                detail: "that instance could not be launched".into(),
            }),
        }
    });

    on.handle::<InstanceStop, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
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
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
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
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        match ctx
            .runtime
            .engine()
            .instances()
            .config_get(&record.id, &p.key)
        {
            Ok(Some(value)) => Ok(InstanceConfigGetResult { value }),
            Ok(None) => Err(ErrorInfo::ConfigKeyUnset { key: p.key.clone() }),
            Err(e) => Err(crate::runtime::engine_error(e)),
        }
    });

    on.handle::<InstanceConfigSet, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        ctx.runtime
            .engine()
            .instances()
            .config_set(&record.id, &p.key, &p.value)
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(instance = %record.id, key = %p.key, "instance config updated");
        Ok(Empty {})
    });

    on.handle::<InstanceConfigList, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        let entries = ctx
            .runtime
            .engine()
            .instances()
            .config_list(&record.id)
            .map_err(crate::runtime::engine_error)?
            .into_iter()
            .map(|(key, value)| ConfigEntry { key, value })
            .collect();
        Ok(InstanceConfigListResult { entries })
    });

    // Profile CRUD is metadata-safe while the instance runs (a change applies
    // at the next launch); only seeding reads the pool, so only create guards
    // against an in-flight content job.
    on.handle::<InstanceProfileList, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        let (active, profiles) = ctx
            .runtime
            .engine()
            .instance_profiles(&record.id)
            .map_err(crate::runtime::engine_error)?;
        Ok(InstanceProfileListResult { active, profiles })
    });

    on.handle::<InstanceProfileCreate, _, _>(|p, ctx| async move {
        // Seeding copies the pool, so it waits on whatever is writing it; a
        // metadata-only create waits on nothing.
        let intent = match p.seed_from_pool {
            true => Intent::Backup,
            false => Intent::Read,
        };
        let record = instance_for(&ctx, &p.instance, intent)?;
        let profile = ctx
            .runtime
            .engine()
            .create_instance_profile(&record.id, &p.name, p.seed_from_pool)
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(instance = %record.id, profile = %profile.name, "profile created");
        Ok(profile)
    });

    on.handle::<InstanceProfileRemove, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
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
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        ctx.runtime
            .engine()
            .rename_instance_profile(&record.id, &p.name, &p.new_name)
            .map_err(crate::runtime::engine_error)
    });

    on.handle::<InstanceProfileUse, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        ctx.runtime
            .engine()
            .use_instance_profile(&record.id, &p.name)
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(instance = %record.id, profile = %p.name, "active profile changed");
        Ok(Empty {})
    });

    on.handle::<InstanceProfileEdit, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        ctx.runtime
            .engine()
            .edit_instance_profile(&record.id, &p.name, &p.add, &p.remove)
            .map_err(crate::runtime::engine_error)
    });

    // Capture/release move real settings trees (and a released store may be
    // what a live session's `config` link writes through), so both require a
    // stopped instance — unlike the metadata-only CRUD above.
    on.handle::<InstanceProfileCapture, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Mutate)?;
        ctx.runtime
            .engine()
            .capture_instance_profile(&record.id, &p.name)
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(instance = %record.id, profile = %p.name, "profile settings captured");
        Ok(Empty {})
    });

    on.handle::<InstanceProfileRelease, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Mutate)?;
        ctx.runtime
            .engine()
            .release_instance_profile(&record.id, &p.name)
            .map_err(crate::runtime::engine_error)?;
        tracing::info!(instance = %record.id, profile = %p.name, "profile settings released");
        Ok(Empty {})
    });
}
