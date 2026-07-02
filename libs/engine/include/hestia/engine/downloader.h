#pragma once

#include <cstdint>
#include <filesystem>
#include <functional>
#include <optional>
#include <string>

#include <hestia/engine/checksum.h>

namespace hestia::engine {
    struct Checksum {
        HashAlgorithm algorithm;
        std::string hex;
    };

    struct DownloadProgress {
        std::uint64_t downloaded_bytes = 0;
        std::uint64_t total_bytes = 0; // 0 = unknown
    };

    using DownloadProgressCallback = std::function<void(const DownloadProgress &)>;

    // Streams a URL to disk through a `.part` temp file, hashing incrementally
    // when a checksum is given, and renames into place only on success — the
    // destination never holds a partial or corrupt download. Stateless, like
    // greeting.
    class Downloader {
    public:
        // Throws std::runtime_error on a network error, a non-2xx status, or a
        // checksum mismatch; the `.part` file is removed on every failure.
        void fetch(const std::string &url, const std::filesystem::path &destination,
                   const std::optional<Checksum> &checksum = std::nullopt,
                   const DownloadProgressCallback &on_progress = {}) const;
    };
}
