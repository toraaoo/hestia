//! The raw supervisor surface: start, stop, and inspect any supervised process.

use proto::process::{
    ProcessList, ProcessListResult, ProcessLogs, ProcessLogsResult, ProcessStart,
    ProcessStartResult, ProcessStatus, ProcessStop,
};
use proto::Empty;

use crate::runtime::{Channels, ServiceError, StartError};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<ProcessStart, _, _>(|spec, ctx| async move {
        match ctx.runtime.processes().start(spec).await {
            Ok(info) => Ok(ProcessStartResult {
                id: info.id,
                pid: info.pid,
            }),
            Err(StartError::EmptyProgram) => Err(ServiceError::bad_request("program is empty")),
            Err(StartError::InvalidId) => Err(ServiceError::bad_request(
                "process id may only contain letters, digits, '-', '_' and '.'",
            )),
            Err(StartError::Spawn(e)) => Err(ServiceError::handler_error(format!(
                "cannot spawn process: {e}"
            ))),
        }
    });

    on.handle::<ProcessStop, _, _>(|p, ctx| async move {
        if ctx.runtime.processes().stop(&p.id) {
            Ok(Empty {})
        } else {
            Err(ServiceError::not_found(format!("no process '{}'", p.id)))
        }
    });

    on.handle::<ProcessList, _, _>(|_: Empty, ctx| async move {
        Ok(ProcessListResult {
            processes: ctx.runtime.processes().list(),
        })
    });

    on.handle::<ProcessStatus, _, _>(|p, ctx| async move {
        ctx.runtime
            .processes()
            .status(&p.id)
            .ok_or_else(|| ServiceError::not_found(format!("no process '{}'", p.id)))
    });

    on.handle::<ProcessLogs, _, _>(|p, ctx| async move {
        match ctx.runtime.processes().logs(&p.id, p.tail) {
            Some(lines) => Ok(ProcessLogsResult { lines }),
            None => Err(ServiceError::not_found(format!("no process '{}'", p.id))),
        }
    });
}
