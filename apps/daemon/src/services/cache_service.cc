#include "services/cache_service.h"

#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <hestia/engine/engine.h>
#include <hestia/proto/cache.h>

namespace hestia::daemon {
    void CacheService::register_channels(Channels &on) {
        on.handle<proto::CacheInfo>([](const proto::Empty &, HandlerContext &ctx) {
            auto &cache = ctx.runtime.engine().cache();
            const auto usage = cache.usage();
            return proto::CacheInfo::Result{
                .path = cache.dir(),
                .usage = {.entries = usage.entries, .bytes = usage.bytes},
            };
        });

        on.handle<proto::CacheList>([](const proto::Empty &, HandlerContext &ctx) {
            proto::CacheList::Result out;
            for (const auto &entry: ctx.runtime.engine().cache().entries()) {
                out.entries.push_back(proto::CacheEntry{.checksum = entry.checksum, .size = entry.size});
            }
            return out;
        });

        on.handle<proto::CacheClear>([](const proto::Empty &, HandlerContext &ctx) {
            const auto freed = ctx.runtime.engine().cache().clear();
            return proto::CacheUsage{.entries = freed.entries, .bytes = freed.bytes};
        });
    }
} // namespace hestia::daemon
