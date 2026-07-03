#pragma once

#include <filesystem>
#include <functional>
#include <optional>
#include <string>

#include <hestia/ipc/download.h>

namespace hestia::engine {
    using DownloadProgressCallback = std::function<void(const ipc::DownloadProgress &)>;

    // Streams a URL to disk through a `.part` temp file, hashing incrementally
    // when a checksum is given; renames into place only on success.
    class Downloader {
    public:
        // Throws std::runtime_error on a network error, a non-2xx status, or a
        // checksum mismatch; the `.part` file is removed on every failure.
        void fetch(const std::string &url, const std::filesystem::path &destination,
                   const std::optional<ipc::Checksum> &checksum = std::nullopt,
                   const DownloadProgressCallback &on_progress = {}) const;
    };
} // namespace hestia::engine
