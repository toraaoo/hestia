#include "services/process_service.h"

#include "process/process_supervisor.h"
#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <hestia/proto/process.h>

namespace hestia::daemon {
    void ProcessService::register_channels(Channels &on) {
        on.handle<proto::ProcessStart>([](const proto::LaunchSpec &spec, HandlerContext &ctx) {
            return ctx.runtime.supervisor().start(spec);
        });

        on.handle<proto::ProcessStop>([](const proto::ProcessId &p, HandlerContext &ctx) {
            ctx.runtime.supervisor().stop(p.id);
            return proto::Empty{};
        });

        on.handle<proto::ProcessList>([](const proto::Empty &, HandlerContext &ctx) {
            return proto::ProcessList::Result{.processes = ctx.runtime.supervisor().list()};
        });

        on.handle<proto::ProcessStatus>([](const proto::ProcessId &p, HandlerContext &ctx) {
            if (const auto record = ctx.runtime.supervisor().status(p.id)) return *record;
            throw ServiceError(ipc::errors::kNotFound, "no such process: " + p.id);
        });

        on.handle<proto::ProcessLogs>([](const proto::ProcessLogs::Params &p, HandlerContext &ctx) {
            if (const auto text = ctx.runtime.supervisor().tail_log(p.id, p.lines)) {
                return proto::ProcessLogs::Result{.text = *text};
            }
            throw ServiceError(ipc::errors::kNotFound, "no such process: " + p.id);
        });
    }
} // namespace hestia::daemon
