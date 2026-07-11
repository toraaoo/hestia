//! Third-party content: the source catalogue (search, project, versions, modpack
//! resolution) and the per-entry install surface for servers and instances.

use proto::content::{
    ContentJobResult, ContentKind, ContentListResult, ContentProjectGet, ContentSearch,
    ContentSources, ContentVersions, InstanceContentAdd, InstanceContentList,
    InstanceContentRemove, InstanceContentUpdate, ModpackResolve, ServerContentAdd,
    ServerContentList, ServerContentRemove, ServerContentUpdate, SourcesResult,
    VersionsResult as ContentVersionsResult,
};
use proto::Empty;

use super::guards::{
    ensure_no_backup, ensure_no_content, ensure_no_update, ensure_stopped, find_instance,
    find_server, require_content_items,
};
use crate::runtime::{instance_process_id, server_process_id, Channels, ContentJob, ServiceError};

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
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))
    });

    on.handle::<ContentProjectGet, _, _>(|p, ctx| async move {
        if p.project.is_empty() {
            return Err(ServiceError::bad_request("project is required"));
        }
        ctx.runtime
            .engine()
            .content()
            .project(&p.source, &p.project)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))
    });

    on.handle::<ContentVersions, _, _>(|q, ctx| async move {
        if q.project.is_empty() {
            return Err(ServiceError::bad_request("project is required"));
        }
        let versions = ctx
            .runtime
            .engine()
            .content()
            .versions(&q)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(ContentVersionsResult { versions })
    });

    on.handle::<ModpackResolve, _, _>(|p, ctx| async move {
        if p.version_id.is_empty() {
            return Err(ServiceError::bad_request("version_id is required"));
        }
        ctx.runtime
            .engine()
            .content()
            .resolve_modpack(&p.source, &p.version_id)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))
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
            None => Err(ServiceError::bad_request(
                "a content change is already running for that server",
            )),
        }
    });

    on.handle::<ServerContentList, _, _>(|p, ctx| async move {
        let record = find_server(&ctx, &p.server)?;
        let (items, untracked) = ctx
            .runtime
            .engine()
            .server_content(&record.id, p.kind)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(ContentListResult { items, untracked })
    });

    on.handle::<ServerContentRemove, _, _>(|p, ctx| async move {
        if p.item.is_empty() {
            return Err(ServiceError::bad_request("item is required"));
        }
        if !p.worlds.is_empty() && p.kind != ContentKind::DataPack {
            return Err(ServiceError::bad_request(
                "only datapacks are installed per world",
            ));
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
            Ok(false) => Err(ServiceError::not_found(format!(
                "nothing installed matches '{}'",
                p.item
            ))),
            Err(e) => Err(ServiceError::handler_error(format!("{e:#}"))),
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
            None => Err(ServiceError::bad_request(
                "a content change is already running for that server",
            )),
        }
    });
}

fn register_instance(on: &mut Channels<'_>) {
    on.handle::<InstanceContentAdd, _, _>(|p, ctx| async move {
        require_content_items(&p.spec)?;
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::InstanceAdd {
                instance_id: record.id,
                spec: p.spec,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ServiceError::bad_request(
                "a content change is already running for that instance",
            )),
        }
    });

    on.handle::<InstanceContentList, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let (items, untracked) = ctx
            .runtime
            .engine()
            .instance_content(&record.id, p.kind)
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(ContentListResult { items, untracked })
    });

    on.handle::<InstanceContentRemove, _, _>(|p, ctx| async move {
        if p.item.is_empty() {
            return Err(ServiceError::bad_request("item is required"));
        }
        if !p.worlds.is_empty() && p.kind != ContentKind::DataPack {
            return Err(ServiceError::bad_request(
                "only datapacks are installed per world",
            ));
        }
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_content(&ctx, &process_id, &record.name)?;
        match ctx
            .runtime
            .engine()
            .remove_instance_content(&record.id, p.kind, &p.item, &p.worlds)
        {
            Ok(true) => Ok(Empty {}),
            Ok(false) => Err(ServiceError::not_found(format!(
                "nothing installed matches '{}'",
                p.item
            ))),
            Err(e) => Err(ServiceError::handler_error(format!("{e:#}"))),
        }
    });

    on.handle::<InstanceContentUpdate, _, _>(|p, ctx| async move {
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::InstanceUpdate {
                instance_id: record.id,
                kind: p.kind,
                item: p.item,
            },
            p.id,
        ) {
            Some(id) => Ok(ContentJobResult { id }),
            None => Err(ServiceError::bad_request(
                "a content change is already running for that instance",
            )),
        }
    });
}
