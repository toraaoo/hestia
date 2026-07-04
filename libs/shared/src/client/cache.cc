#include "hestia/client/cache.h"

#include "session.h"

namespace hestia::client {
    proto::CacheInfo::Result Cache::info() {
        return session_->call<proto::CacheInfo>();
    }

    std::vector<proto::CacheEntry> Cache::list() {
        return session_->call<proto::CacheList>().entries;
    }

    proto::CacheUsage Cache::clear() {
        return session_->call<proto::CacheClear>();
    }
} // namespace hestia::client
