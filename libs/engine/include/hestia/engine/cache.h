#pragma once

#include <cstdint>
#include <filesystem>
#include <mutex>
#include <optional>
#include <vector>

#include <hestia/proto/download.h>

namespace hestia::engine {
    struct CacheEntry {
        proto::Checksum checksum;
        std::uint64_t size = 0;
    };

    struct CacheUsage {
        std::uint64_t entries = 0;
        std::uint64_t bytes = 0;
    };

    // Content-addressed store of verified downloads, keyed by checksum
    // (<dir>/<algorithm>/<hex[0:2]>/<hex>). Blobs are immutable; consumers
    // re-verify on the way out, so a damaged blob is evicted, never served.
    class Cache {
    public:
        explicit Cache(std::filesystem::path dir);

        [[nodiscard]] std::filesystem::path dir() const;

        [[nodiscard]] std::optional<std::filesystem::path> lookup(const proto::Checksum &checksum) const;

        // Best effort: a failure to cache never fails the download that fed it.
        void store(const std::filesystem::path &file, const proto::Checksum &checksum);

        void evict(const proto::Checksum &checksum);

        [[nodiscard]] std::vector<CacheEntry> entries() const;
        [[nodiscard]] CacheUsage usage() const;

        CacheUsage clear();

        void reload(std::filesystem::path dir);

    private:
        mutable std::mutex mu_;
        std::filesystem::path dir_;
    };
} // namespace hestia::engine
