#include "services/java_service.h"

#include "java/install_manager.h"
#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <string>

#include <hestia/engine/engine.h>
#include <hestia/proto/java.h>

namespace hestia::daemon {
    void JavaService::register_channels(Channels &on) {
        on.handle<proto::JavaReleases>([](const proto::Empty &, HandlerContext &ctx) {
            return proto::JavaReleases::Result{.releases = ctx.runtime.engine().java().releases()};
        });

        on.handle<proto::JavaList>([](const proto::Empty &, HandlerContext &ctx) {
            return proto::JavaList::Result{.runtimes = ctx.runtime.engine().java().installed()};
        });

        on.handle<proto::JavaInstall>([](const proto::JavaInstall::Params &p, HandlerContext &ctx) {
            if (p.major <= 0) {
                throw ServiceError(ipc::errors::kBadRequest, "major must be a positive integer");
            }
            const auto id = ctx.runtime.java_installs().start(p.major, p.id, p.force);
            if (!id) {
                throw ServiceError(ipc::errors::kBadRequest,
                                   "java " + std::to_string(p.major) + " is already being installed");
            }
            return proto::JavaInstall::Result{.id = *id};
        });

        on.handle<proto::JavaUninstall>([](const proto::JavaUninstall::Params &p, HandlerContext &ctx) {
            if (p.major <= 0) {
                throw ServiceError(ipc::errors::kBadRequest, "major must be a positive integer");
            }
            if (!ctx.runtime.engine().java().uninstall(p.major)) {
                throw ServiceError(ipc::errors::kNotFound,
                                   "no installed java runtime for major " + std::to_string(p.major));
            }
            return proto::Empty{};
        });
    }
} // namespace hestia::daemon
