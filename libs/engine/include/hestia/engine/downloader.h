#pragma once

#include <filesystem>
#include <functional>
#include <optional>
#include <string>

#include <hestia/proto/download.h>

namespace hestia::engine {
    class Cache;

    using DownloadProgressCallback = std::function<void(const proto::DownloadProgress &)>;

    // Streams a URL to disk through a `.part` temp file, hashing incrementally
    // when a checksum is given; renames into place only on success. With a
    // cache, a checksummed fetch is served from it when possible (re-verified
    // on the way out) and feeds it after a successful download.
    class Downloader {
    public:
        explicit Downloader(Cache *cache = nullptr);

        // Throws std::runtime_error on a network error, a non-2xx status, or a
        // checksum mismatch; the `.part` file is removed on every failure.
        void fetch(const std::string &url, const std::filesystem::path &destination,
                   const std::optional<proto::Checksum> &checksum = std::nullopt,
                   const DownloadProgressCallback &on_progress = {}) const;

    private:
        Cache *cache_;
    };
} // namespace hestia::engine
