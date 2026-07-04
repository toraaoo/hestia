#include "services/config_service.h"

#include "platform/autostart.h"
#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <stdexcept>

#include <hestia/engine/engine.h>
#include <hestia/proto/config.h>

namespace hestia::daemon {
    void ConfigService::register_channels(Channels &on) {
        on.handle<proto::ConfigGet>([](const proto::ConfigGet::Params &p, HandlerContext &ctx) {
            if (p.key == proto::kHomeKey) {
                return proto::ConfigGet::Result{.value = ctx.runtime.engine().data_home().string()};
            }
            if (p.key == proto::kAutostartKey) {
                return proto::ConfigGet::Result{.value = make_autostart()->is_enabled()};
            }
            try {
                return proto::ConfigGet::Result{.value = ctx.runtime.engine().config().get(p.key)};
            } catch (const std::invalid_argument &e) {
                throw ServiceError(ipc::errors::kNotFound, e.what());
            }
        });

        on.handle<proto::ConfigSet>([](const proto::ConfigSet::Params &p, HandlerContext &ctx) {
            if (p.key == proto::kHomeKey) {
                if (!p.value.is_string()) {
                    throw ServiceError(ipc::errors::kBadRequest, "home expects a string");
                }
                ctx.runtime.engine().set_data_home(p.value.get<std::string>());
                return proto::Empty{};
            }
            if (p.key == proto::kAutostartKey) {
                if (!p.value.is_boolean()) {
                    throw ServiceError(ipc::errors::kBadRequest, "autostart expects a boolean");
                }
                if (p.value.get<bool>()) {
                    make_autostart()->enable();
                } else {
                    make_autostart()->disable();
                }
                return proto::Empty{};
            }
            try {
                ctx.runtime.engine().config().set(p.key, p.value);
            } catch (const std::invalid_argument &e) {
                throw ServiceError(ipc::errors::kBadRequest, e.what());
            }
            return proto::Empty{};
        });

        on.handle<proto::ConfigList>([](const proto::Empty &, HandlerContext &ctx) {
            auto entries = ctx.runtime.engine().config().all();
            entries[proto::kHomeKey] = ctx.runtime.engine().data_home().string();
            entries[proto::kAutostartKey] = make_autostart()->is_enabled();
            return proto::ConfigList::Result{.entries = std::move(entries)};
        });
    }
} // namespace hestia::daemon
