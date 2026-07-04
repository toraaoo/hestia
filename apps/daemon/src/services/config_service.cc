#include "services/config_service.h"

#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <hestia/engine/engine.h>
#include <hestia/proto/config.h>

namespace hestia::daemon {
    void ConfigService::register_channels(Channels &on) {
        on.handle<proto::ConfigGet>([](const proto::ConfigGet::Params &p, HandlerContext &ctx) {
            if (const auto value = ctx.runtime.engine().config().get(p.key)) {
                return proto::ConfigGet::Result{.value = *value};
            }
            throw ServiceError(ipc::errors::kNotFound, "key not found: " + p.key);
        });

        on.handle<proto::ConfigSet>([](const proto::ConfigSet::Params &p, HandlerContext &ctx) {
            ctx.runtime.engine().config().set(p.key, p.value);
            return proto::Empty{};
        });

        on.handle<proto::ConfigHome>([](const proto::Empty &, HandlerContext &ctx) {
            return proto::ConfigHome::Result{.path = ctx.runtime.engine().data_home()};
        });

        on.handle<proto::ConfigSetHome>([](const proto::ConfigSetHome::Params &p, HandlerContext &ctx) {
            return proto::ConfigHome::Result{.path = ctx.runtime.engine().set_data_home(p.dir)};
        });
    }
} // namespace hestia::daemon
