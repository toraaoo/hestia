//! Global content profiles: the data-home-level project reference lists and
//! the one-shot apply that installs one into an instance's pool (a job over
//! the `ContentManager`, publishing the `content.*` topics).

use proto::content::ContentJobResult;
use proto::error::{ErrorInfo, Field, ProfileScope, Task};
use proto::profile::{
    InstanceProfileApply, ProfileCreate, ProfileEdit, ProfileList, ProfileListResult, ProfileRemove,
};
use proto::Empty;

use super::guards::{ensure_no_backup, ensure_no_content, ensure_stopped, find_instance};
use crate::runtime::{instance_process_id, Channels, ContentJob};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<ProfileList, _, _>(|_: Empty, ctx| async move {
        Ok(ProfileListResult {
            profiles: ctx.runtime.engine().profiles().list(),
        })
    });

    on.handle::<ProfileCreate, _, _>(|p, ctx| async move {
        if p.name.trim().is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Name });
        }
        ctx.runtime
            .engine()
            .profiles()
            .create(&p.name)
            .map_err(crate::runtime::internal)
    });

    on.handle::<ProfileRemove, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .profiles()
            .remove(&p.name)
            .map_err(|_| ErrorInfo::ProfileNotFound {
                scope: ProfileScope::Global,
                name: p.name.clone(),
            })?;
        Ok(Empty {})
    });

    on.handle::<ProfileEdit, _, _>(|p, ctx| async move {
        if p.add.is_empty() && p.remove.is_empty() {
            return Err(ErrorInfo::NothingToDo { what: Task::Modify });
        }
        ctx.runtime
            .engine()
            .edit_global_profile(&p.name, &p.source, &p.add, &p.remove)
            .await
            .map_err(crate::runtime::internal)
    });

    on.handle::<InstanceProfileApply, _, _>(|p, ctx| async move {
        if p.profile.is_empty() {
            return Err(ErrorInfo::FieldRequired { field: Field::Name });
        }
        // The profile must exist before the job is accepted, so a typo answers
        // here rather than as an async job error.
        ctx.runtime
            .engine()
            .profiles()
            .get(&p.profile)
            .map_err(|_| ErrorInfo::ProfileNotFound {
                scope: ProfileScope::Global,
                name: p.profile.clone(),
            })?;
        let record = find_instance(&ctx, &p.instance)?;
        let process_id = instance_process_id(&record.id);
        ensure_stopped(&ctx, &process_id, "instance", &record.name)?;
        ensure_no_backup(&ctx, &process_id, &record.name)?;
        ensure_no_content(&ctx, &process_id, &record.name)?;
        match ctx.runtime.content_jobs().start(
            ContentJob::ProfileApply {
                instance_id: record.id,
                profile: p.profile,
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
