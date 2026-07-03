#include "services/services.h"

#include "runtime/handler_context.h"
#include "runtime/router.h"
#include "runtime/runtime.h"

#include <hestia/engine/engine.h>

namespace hestia::daemon {
    void register_cache_service(Router &router) {
        router.on("cache.info", [](const ipc::Request &, HandlerContext &ctx) {
            auto &cache = ctx.runtime.engine().cache();
            const auto usage = cache.usage();
            return ipc::Response::success({
                {"path", cache.dir().string()},
                {"entries", usage.entries},
                {"bytes", usage.bytes},
            });
        });

        router.on("cache.list", [](const ipc::Request &, HandlerContext &ctx) {
            auto entries = nlohmann::json::array();
            for (const auto &entry: ctx.runtime.engine().cache().entries()) {
                entries.push_back({
                    {"algorithm", ipc::to_string(entry.checksum.algorithm)},
                    {"hex", entry.checksum.hex},
                    {"size", entry.size},
                });
            }
            return ipc::Response::success({{"entries", std::move(entries)}});
        });

        router.on("cache.clear", [](const ipc::Request &, HandlerContext &ctx) {
            const auto freed = ctx.runtime.engine().cache().clear();
            return ipc::Response::success({{"entries", freed.entries}, {"bytes", freed.bytes}});
        });
    }
} // namespace hestia::daemon
