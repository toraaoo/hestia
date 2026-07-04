#pragma once

#include <vector>

#include <hestia/client/facade.h>
#include <hestia/proto/cache.h>

namespace hestia::client {
    // The daemon's content-addressed download cache.
    class Cache : public Facade {
    public:
        using Facade::Facade;

        proto::CacheInfo::Result info();
        std::vector<proto::CacheEntry> list();
        // Reports what was removed.
        proto::CacheUsage clear();
    };
} // namespace hestia::client
