//! Java runtime management: the provider catalogue plus install/uninstall.

use proto::java::{
    JavaInstall, JavaInstallResult, JavaList, JavaListResult, JavaReleases, JavaReleasesResult,
    JavaUninstall,
};
use proto::Empty;

use crate::runtime::{Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<JavaReleases, _, _>(|_: Empty, ctx| async move {
        let releases = ctx
            .runtime
            .engine()
            .java()
            .releases()
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        // The launcher only ever launches Minecraft, so offering every
        // vendor-supported major is noise.
        let releases = releases
            .into_iter()
            .filter(|r| engine::REQUIRED_JAVA_MAJORS.contains(&r.major))
            .collect();
        Ok(JavaReleasesResult { releases })
    });

    on.handle::<JavaList, _, _>(|_: Empty, ctx| async move {
        let engine = ctx.runtime.engine();
        let required: std::collections::HashSet<i32> = engine
            .servers()
            .list()
            .iter()
            .map(|r| r.profile.java_major)
            .chain(
                engine
                    .instances()
                    .list()
                    .iter()
                    .map(|r| r.profile.java_major),
            )
            .collect();
        let runtimes = engine
            .java()
            .installed()
            .into_iter()
            .map(|mut runtime| {
                runtime.in_use = required.contains(&runtime.major);
                runtime
            })
            .collect();
        Ok(JavaListResult { runtimes })
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
            tracing::info!(major = p.major, "java runtime uninstalled");
            Ok(Empty {})
        } else {
            Err(ServiceError::not_found(format!(
                "no installed java runtime for major {}",
                p.major
            )))
        }
    });
}
