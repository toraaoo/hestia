//! Modpacks: installing one into a new or existing entry, moving it to another
//! published version, reading which pack an entry runs, and taking it back out.
//!
//! An install that *creates* its entry has nothing to guard — the entry does not
//! exist yet. One aimed at an existing entry is guarded exactly as a content
//! install is: stopped, and racing no backup, update or other content job.

use proto::error::{ErrorInfo, Field};
use proto::modpack::{
    InstanceModpackInstall, InstanceModpackRemove, InstanceModpackStatus, InstanceModpackUpdate,
    ModpackJobResult, ModpackRef, ModpackStatusResult, ModpackTarget, ServerModpackInstall,
    ServerModpackRemove, ServerModpackStatus, ServerModpackUpdate,
};

use super::guards::{instance_for, server_for, Intent};
use crate::runtime::{Channels, ModpackJob};

pub(super) fn register(on: &mut Channels<'_>) {
    register_instance(on);
    register_server(on);
}

fn register_instance(on: &mut Channels<'_>) {
    on.handle::<InstanceModpackInstall, _, _>(|p, ctx| async move {
        require_pack_ref(&p.pack)?;
        let target = match &p.target {
            ModpackTarget::Create { name } => ModpackTarget::Create { name: name.clone() },
            ModpackTarget::Existing { entry } => {
                let record = instance_for(&ctx, entry, Intent::Mutate)?;
                ModpackTarget::Existing {
                    entry: record.id.clone(),
                }
            }
        };
        let busy = target_name(&target);
        start(
            ctx.runtime.modpack_jobs().start(
                ModpackJob::InstallInstance {
                    pack: p.pack,
                    target,
                },
                p.id,
            ),
            busy,
        )
    });

    on.handle::<InstanceModpackUpdate, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Mutate)?;
        start(
            ctx.runtime.modpack_jobs().start(
                ModpackJob::UpdateInstance {
                    instance_id: record.id,
                    version: p.version,
                    allow_downgrade: p.allow_downgrade,
                },
                p.id,
            ),
            record.name,
        )
    });

    on.handle::<InstanceModpackStatus, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Read)?;
        let pack = ctx
            .runtime
            .engine()
            .instance_modpack(&record.id)
            .map_err(crate::runtime::engine_error)?;
        Ok(ModpackStatusResult { pack })
    });

    on.handle::<InstanceModpackRemove, _, _>(|p, ctx| async move {
        let record = instance_for(&ctx, &p.instance, Intent::Mutate)?;
        ctx.runtime
            .engine()
            .remove_instance_modpack(&record.id)
            .map_err(crate::runtime::engine_error)
    });
}

fn register_server(on: &mut Channels<'_>) {
    on.handle::<ServerModpackInstall, _, _>(|p, ctx| async move {
        require_pack_ref(&p.pack)?;
        let target = match &p.target {
            ModpackTarget::Create { name } => {
                if !p.eula {
                    return Err(ErrorInfo::EulaRequired);
                }
                ModpackTarget::Create { name: name.clone() }
            }
            ModpackTarget::Existing { entry } => {
                let record = server_for(&ctx, entry, Intent::Mutate)?;
                ModpackTarget::Existing {
                    entry: record.id.clone(),
                }
            }
        };
        let busy = target_name(&target);
        start(
            ctx.runtime.modpack_jobs().start(
                ModpackJob::InstallServer {
                    pack: p.pack,
                    target,
                    eula: p.eula,
                    port: p.port,
                },
                p.id,
            ),
            busy,
        )
    });

    on.handle::<ServerModpackUpdate, _, _>(|p, ctx| async move {
        let record = server_for(&ctx, &p.server, Intent::Mutate)?;
        start(
            ctx.runtime.modpack_jobs().start(
                ModpackJob::UpdateServer {
                    server_id: record.id,
                    version: p.version,
                    allow_downgrade: p.allow_downgrade,
                },
                p.id,
            ),
            record.name,
        )
    });

    on.handle::<ServerModpackStatus, _, _>(|p, ctx| async move {
        let record = server_for(&ctx, &p.server, Intent::Read)?;
        let pack = ctx
            .runtime
            .engine()
            .server_modpack(&record.id)
            .map_err(crate::runtime::engine_error)?;
        Ok(ModpackStatusResult { pack })
    });

    on.handle::<ServerModpackRemove, _, _>(|p, ctx| async move {
        let record = server_for(&ctx, &p.server, Intent::Mutate)?;
        ctx.runtime
            .engine()
            .remove_server_modpack(&record.id)
            .map_err(crate::runtime::engine_error)
    });
}

/// Exactly one selector, checked at the edge so a malformed reference never
/// becomes a job that fails a second later.
fn require_pack_ref(pack: &ModpackRef) -> Result<(), ErrorInfo> {
    let picked = [&pack.project, &pack.url, &pack.path]
        .iter()
        .filter(|s| !s.is_empty())
        .count();
    match picked {
        1 => Ok(()),
        0 => Err(ErrorInfo::FieldRequired {
            field: Field::Project,
        }),
        _ => Err(ErrorInfo::MutuallyExclusive {
            options: vec!["a project".into(), "a url".into(), "a file".into()],
        }),
    }
}

/// What a busy refusal names — the entry for an install into one, the pack
/// itself when the job would have created its own entry.
fn target_name(target: &ModpackTarget) -> String {
    match target {
        ModpackTarget::Create { name } => name.clone(),
        ModpackTarget::Existing { entry } => entry.clone(),
    }
}

fn start(started: Option<String>, name: String) -> Result<ModpackJobResult, ErrorInfo> {
    match started {
        Some(id) => Ok(ModpackJobResult { id }),
        None => Err(ErrorInfo::ContentInProgress { name }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_names_exactly_one_thing() {
        let project = ModpackRef {
            project: "fabulously-optimized".to_string(),
            ..ModpackRef::default()
        };
        assert!(require_pack_ref(&project).is_ok());
        assert!(require_pack_ref(&ModpackRef::default()).is_err());

        let both = ModpackRef {
            project: "fabulously-optimized".to_string(),
            path: "/tmp/pack.mrpack".to_string(),
            ..ModpackRef::default()
        };
        assert!(require_pack_ref(&both).is_err());
    }
}
