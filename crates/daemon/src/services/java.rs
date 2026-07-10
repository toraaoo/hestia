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
