#include "services/config_service.h"

#include "platform/autostart.h"
#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <algorithm>
#include <cctype>
#include <optional>
#include <string>

#include <hestia/engine/engine.h>
#include <hestia/proto/config.h>

namespace hestia::daemon {
    namespace {
        constexpr const char *kHome = proto::config_key<&proto::Config::home>();
        constexpr const char *kAutostart = proto::config_key<&proto::Config::autostart>();

        std::optional<bool> parse_bool(std::string value) {
            std::ranges::transform(value, value.begin(),
                                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
            if (value == "true" || value == "1" || value == "on" || value == "yes" || value == "enabled") return true;
            if (value == "false" || value == "0" || value == "off" || value == "no" || value == "disabled") return false;
            return std::nullopt;
        }
    } // namespace

    void ConfigService::register_channels(Channels &on) {
        on.handle<proto::ConfigGet>([](const proto::ConfigGet::Params &p, HandlerContext &ctx) {
            if (p.key == kHome) {
                return proto::ConfigGet::Result{.value = ctx.runtime.engine().data_home().string()};
            }
            if (p.key == kAutostart) {
                return proto::ConfigGet::Result{.value = make_autostart()->is_enabled() ? "true" : "false"};
            }
            if (const auto value = ctx.runtime.engine().config().get(p.key)) {
                return proto::ConfigGet::Result{.value = *value};
            }
            throw ServiceError(ipc::errors::kNotFound, "key not found: " + p.key);
        });

        on.handle<proto::ConfigSet>([](const proto::ConfigSet::Params &p, HandlerContext &ctx) {
            if (p.key == kHome) {
                ctx.runtime.engine().set_data_home(p.value);
                return proto::Empty{};
            }
            if (p.key == kAutostart) {
                const auto enabled = parse_bool(p.value);
                if (!enabled) {
                    throw ServiceError(ipc::errors::kBadRequest, "autostart expects a boolean, got: " + p.value);
                }
                if (*enabled) {
                    make_autostart()->enable();
                } else {
                    make_autostart()->disable();
                }
                return proto::Empty{};
            }
            ctx.runtime.engine().config().set(p.key, p.value);
            return proto::Empty{};
        });

        on.handle<proto::ConfigList>([](const proto::Empty &, HandlerContext &ctx) {
            return proto::ConfigList::Result{.entries = ctx.runtime.engine().config().all()};
        });
    }
} // namespace hestia::daemon
