//! Third-party content: the source catalogue (search, project, versions, modpack
//! resolution) and the per-entry install surface for servers and instances.

use proto::content::{
    ContentJobResult, ContentKind, ContentListResult, ContentProjectGet, ContentSearch,
    ContentSources, ContentUpdatesResult, ContentVersions, InstanceContentAdd,
    InstanceContentCheckUpdates, InstanceContentEnable, InstanceContentList, InstanceContentRemove,
    InstanceContentSetVersion, InstanceContentUpdate, ModpackResolve, ServerContentAdd,
    ServerContentCheckUpdates, ServerContentEnable, ServerContentList, ServerContentRemove,
    ServerContentSetVersion, ServerContentUpdate, SourcesResult,
    VersionsResult as ContentVersionsResult,
};
use proto::error::{ErrorInfo, Field, Unsupported};
use proto::Empty;

use super::guards::{
    ensure_no_backup, ensure_no_content, ensure_no_update, ensure_stopped, find_instance,
    find_server, require_content_items,
};
use crate::runtime::{instance_process_id, server_process_id, Channels, ContentJob};

/// A datapack toggle may narrow by world; any other kind rejects `worlds`.
fn check_worlds(kind: ContentKind, worlds: &[String]) -> Result<(), ErrorInfo> {
    if !worlds.is_empty() && kind != ContentKind::DataPack {
        return Err(ErrorInfo::UnsupportedOperation {
            reason: Unsupported::DatapacksPerWorld,
        });
    }
    Ok(())
}

pub(super) fn register(on: &mut Channels<'_>) {
    register_sources(on);
    register_server(on);
    register_instance(on);
}

fn register_sources(on: &mut Channels<'_>) {
    on.handle::<ContentSources, _, _>(|_: Empty, ctx| async move {
        Ok(SourcesResult {
            sources: ctx.runtime.engine().content().sources(),
        })
    });

    on.handle::<ContentSearch, _, _>(|q, ctx| async move {
        ctx.runtime
            .engine()
            .content()
            .search(&q)
            .await
            .map_err(crate::runtime::internal)
    });

    on.handle::<ContentProjectGet, _, _>(|p, ctx| async move {
        if p.project.is_empty() {
            return Err(ErrorInfo::FieldRequired {
                field: Field::Project,
            });
        }
        ctx.runtime
            .engine()
            .content()
            .project(&p.source, &p.project)
            .await
            .map_err(crate::runtime::internal)
    });

    on.handle::<ContentVersions, _, _>(|q, ctx| async move {
        if q.project.is_empty() {
            return Err(ErrorInfo::FieldRequired {
                field: Field::Project,
            });
        }
        let versions = ctx
            .runtime
            .engine()
            .content()
            .versions(&q)
            .await
            .map_err(crate::runtime::internal)?;
        Ok(ContentVersionsResult { versions })
    });

    on.handle::<ModpackResolve, _, _>(|p, ctx| async move {
        if p.version_id.is_empty() {
            return Err(ErrorInfo::FieldRequired {
                field: Field::Version,
            });
        }
        ctx.runtime
            .engine()
            .content()
            .resolve_modpack(&p.source, &p.version_id)
            .await
            .map_err(crate::runtime::internal)
    });
}

fn register_server(on: &mut Channels<'_>) {
    on.handle::<ServerContentAdd, _, _>(|p, ctx| async move {
        require_content_items(&p.spec)?;
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "server", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_update(&ctx, &record.id, &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::ServerAdd {
                server_id: record.id,
                spec: p.spec,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ErrorInfo::ContentInProgress {
                name: record.name.clone(),
            }),
        }
    });

    on.handle::<ServerContentList, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let (items, untracked) = ctx
            .runtime
            .engine()
            .server_content(&record.id, p.kind)
            .map_err(crate::runtime::internal)?;
        Ok(ContentListResult { items, untracked })
    });

    on.handle::<ServerContentRemove, _, _>(|p, ctx| async move {
        if p.item.is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Item });
        }
        if !p.worlds.is_empty() && p.kind != ContentKind::DataPack {
            return Err(ErrorInfo::UnsupportedOperation {
                reason: Unsupported::DatapacksPerWorld,
            });
        }
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "server", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_content(&ctx, &process_id, &record.name)?;
        match ctx
            .runtime
            .engine()
            .remove_server_content(&record.id, p.kind, &p.item, &p.worlds)
        {
            Ok(true) => Ok(Empty {}),
            Ok(false) => Err(ErrorInfo::ContentNotFound {
                reference: p.item.clone(),
            }),
            Err(e) => Err(crate::runtime::internal(e)),
        }
    });

    on.handle::<ServerContentUpdate, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "server", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_update(&ctx, &record.id, &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::ServerUpdate {
                server_id: record.id,
                kind: p.kind,
                item: p.item,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ErrorInfo::ContentInProgress {
                name: record.name.clone(),
            }),
        }
    });

    on.handle::<ServerContentEnable, _, _>(|p, ctx| async move {
        if p.item.is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Item });
        }
        check_worlds(p.kind, &p.worlds)?;
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "server", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_content(&ctx, &process_id, &record.name)?;
        ensure_no_update(&ctx, &record.id, &record.name)?;
        match ctx
            .runtime
            .engine()
            .enable_server_content(&record.id, p.kind, &p.item, p.enabled, &p.worlds)
        {
            Ok(0) => Err(ErrorInfo::ContentNotFound {
                reference: p.item.clone(),
            }),
            Ok(_) => Ok(Empty {}),
            Err(e) => Err(crate::runtime::internal(e)),
        }
    });

    on.handle::<ServerContentCheckUpdates, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let updates = ctx
            .runtime
            .engine()
            .check_server_updates(&record.id, p.kind)
            .await
            .map_err(crate::runtime::internal)?;
        Ok(ContentUpdatesResult { updates })
    });

    on.handle::<ServerContentSetVersion, _, _>(|p, ctx| async move {
        if p.item.is_empty() || p.version.is_empty() {
            return Err(ErrorInfo::FieldsRequired {
                fields: vec![Field::Item, Field::Version],
            });
        }
        let record = find_server(&ctx, &p.server)?;
        let process_id = server_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "server", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_update(&ctx, &record.id, &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::ServerSetVersion {
                server_id: record.id,
                kind: p.kind,
                item: p.item,
                version: p.version,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ErrorInfo::ContentInProgress {
                name: record.name.clone(),
            }),
        }
    });
}

fn register_instance(on: &mut Channels<'_>) {
    on.handle::<InstanceContentAdd, _, _>(|p, ctx| async move {
        require_content_items(&p.spec)?;
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::InstanceAdd {
                instance_id: record.id,
                spec: p.spec,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ErrorInfo::ContentInProgress {
                name: record.name.clone(),
            }),
        }
    });

    on.handle::<InstanceContentList, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let (items, untracked) = ctx
            .runtime
            .engine()
            .instance_content(&record.id, p.kind)
            .map_err(crate::runtime::internal)?;
        Ok(ContentListResult { items, untracked })
    });

    on.handle::<InstanceContentRemove, _, _>(|p, ctx| async move {
        if p.item.is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Item });
        }
        if !p.worlds.is_empty() && p.kind != ContentKind::DataPack {
            return Err(ErrorInfo::UnsupportedOperation {
                reason: Unsupported::DatapacksPerWorld,
            });
        }
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        ensure_no_content(&ctx, &process_id, &record.name)?;
        match ctx
            .runtime
            .engine()
            .remove_instance_content(&record.id, p.kind, &p.item, &p.worlds)
        {
            Ok(true) => Ok(Empty {}),
            Ok(false) => Err(ErrorInfo::ContentNotFound {
                reference: p.item.clone(),
            }),
            Err(e) => Err(crate::runtime::internal(e)),
        }
    });

    on.handle::<InstanceContentUpdate, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::InstanceUpdate {
                instance_id: record.id,
                kind: p.kind,
                item: p.item,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ErrorInfo::ContentInProgress {
                name: record.name.clone(),
            }),
        }
    });

    on.handle::<InstanceContentEnable, _, _>(|p, ctx| async move {
        if p.item.is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Item });
        }
        check_worlds(p.kind, &p.worlds)?;
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        ensure_no_content(&ctx, &process_id, &record.name)?;
        match ctx
            .runtime
            .engine()
            .enable_instance_content(&record.id, p.kind, &p.item, p.enabled, &p.worlds)
        {
            Ok(0) => Err(ErrorInfo::ContentNotFound {
                reference: p.item.clone(),
            }),
            Ok(_) => Ok(Empty {}),
            Err(e) => Err(crate::runtime::internal(e)),
        }
    });

    on.handle::<InstanceContentCheckUpdates, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let updates = ctx
            .runtime
            .engine()
            .check_instance_updates(&record.id, p.kind)
            .await
            .map_err(crate::runtime::internal)?;
        Ok(ContentUpdatesResult { updates })
    });

    on.handle::<InstanceContentSetVersion, _, _>(|p, ctx| async move {
        if p.item.is_empty() || p.version.is_empty() {
            return Err(ErrorInfo::FieldsRequired {
                fields: vec![Field::Item, Field::Version],
            });
        }
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::InstanceSetVersion {
                instance_id: record.id,
                kind: p.kind,
                item: p.item,
                version: p.version,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ErrorInfo::ContentInProgress {
                name: record.name.clone(),
            }),
        }
    });
}
