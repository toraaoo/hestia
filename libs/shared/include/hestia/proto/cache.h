#pragma once

#include <cstdint>
#include <filesystem>
#include <vector>

#include <hestia/proto/contract.h>
#include <hestia/proto/download.h>

namespace hestia::proto {
    struct CacheEntry {
        Checksum checksum;
        std::uint64_t size = 0;

        static constexpr auto kFields =
            fields(field("", &CacheEntry::checksum, kFlatten), field("size", &CacheEntry::size));
    };

    struct CacheUsage {
        std::uint64_t entries = 0;
        std::uint64_t bytes = 0;

        static constexpr auto kFields = fields(field("entries", &CacheUsage::entries), field("bytes", &CacheUsage::bytes));
    };

    struct CacheInfo {
        static constexpr const char *kChannel = "cache.info";
        using Params = Empty;
        struct Result {
            std::filesystem::path path;
            CacheUsage usage;

            static constexpr auto kFields =
                fields(field("path", &Result::path), field("", &Result::usage, kFlatten));
        };
    };

    struct CacheList {
        static constexpr const char *kChannel = "cache.list";
        using Params = Empty;
        struct Result {
            std::vector<CacheEntry> entries;

            static constexpr auto kFields = fields(field("entries", &Result::entries));
        };
    };

    struct CacheClear {
        static constexpr const char *kChannel = "cache.clear";
        using Params = Empty;
        using Result = CacheUsage;
    };
} // namespace hestia::proto
