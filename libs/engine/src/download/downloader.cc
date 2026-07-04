#include <hestia/engine/downloader.h>

#include <cctype>
#include <fstream>
#include <stdexcept>
#include <system_error>

#include <cpr/cpr.h>
#include <fmt/format.h>
#include <spdlog/spdlog.h>

#include <hestia/engine/cache.h>

#include "download/checksum.h"

namespace hestia::engine {
    namespace fs = std::filesystem;

    namespace {
        std::string to_lower(std::string s) {
            for (auto &c: s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
            return s;
        }

        void validate(const proto::Checksum &checksum) {
            if (!proto::is_valid_checksum(checksum)) {
                throw std::runtime_error(fmt::format("invalid checksum '{}': expected {} hex characters", checksum.hex,
                                                     proto::hex_digest_length(checksum.algorithm)));
            }
        }

        void remove_quietly(const fs::path &path) {
            std::error_code ec;
            fs::remove(path, ec);
        }

        // Copies a cached blob to `destination`, re-hashing on the way out.
        // A blob that no longer matches its key is evicted, and the caller
        // falls back to the network.
        bool serve_from_cache(Cache &cache, const fs::path &destination, const proto::Checksum &checksum,
                              const DownloadProgressCallback &on_progress) {
            const auto blob = cache.lookup(checksum);
            if (!blob) return false;
            std::error_code ec;
            const auto total = fs::file_size(*blob, ec);
            if (ec) return false;
            std::ifstream in(*blob, std::ios::binary);
            if (!in) return false;

            const fs::path part = destination.string() + ".part";
            std::ofstream out(part, std::ios::binary | std::ios::trunc);
            if (!out) return false;

            Hasher hasher(checksum.algorithm);
            std::uint64_t copied = 0;
            char buf[64 * 1024];
            while (in.read(buf, sizeof buf) || in.gcount() > 0) {
                const auto n = in.gcount();
                out.write(buf, n);
                hasher.update(buf, static_cast<std::size_t>(n));
                copied += static_cast<std::uint64_t>(n);
                if (on_progress) on_progress(proto::DownloadProgress{.downloaded = copied, .total = total});
                if (in.eof()) break;
            }
            out.close();

            if (!out || hasher.hex_digest() != to_lower(checksum.hex)) {
                spdlog::warn("cache blob {} is corrupt; evicting and refetching", checksum.hex);
                cache.evict(checksum);
                remove_quietly(part);
                return false;
            }
            fs::rename(part, destination, ec);
            if (ec) {
                remove_quietly(part);
                return false;
            }
            return true;
        }
    } // namespace

    Downloader::Downloader(Cache *cache) : cache_(cache) {}

    void Downloader::fetch(const std::string &url, const fs::path &destination,
                           const std::optional<proto::Checksum> &checksum,
                           const DownloadProgressCallback &on_progress) const {
        if (url.empty()) throw std::runtime_error("download url is empty");
        if (checksum) validate(*checksum);

        if (const auto parent = destination.parent_path(); !parent.empty()) {
            fs::create_directories(parent);
        }

        if (checksum && cache_ && serve_from_cache(*cache_, destination, *checksum, on_progress)) {
            spdlog::debug("cache hit for {}, served to {}", checksum->hex, destination.string());
            return;
        }

        spdlog::debug("downloading {} -> {}", url, destination.string());
        const fs::path part = destination.string() + ".part";
        std::ofstream out(part, std::ios::binary | std::ios::trunc);
        if (!out) {
            throw std::runtime_error(fmt::format("cannot open {} for writing", part.string()));
        }

        std::optional<Hasher> hasher;
        std::string expected;
        if (checksum) {
            hasher.emplace(checksum->algorithm);
            expected = to_lower(checksum->hex);
        }

        const cpr::WriteCallback write_cb([&](std::string_view data, intptr_t) {
            out.write(data.data(), static_cast<std::streamsize>(data.size()));
            if (hasher) hasher->update(data.data(), data.size());
            return out.good();
        });
        const cpr::ProgressCallback progress_cb([&](cpr::cpr_pf_arg_t download_total, cpr::cpr_pf_arg_t download_now,
                                                    cpr::cpr_pf_arg_t, cpr::cpr_pf_arg_t, intptr_t) {
            if (on_progress) {
                on_progress(proto::DownloadProgress{
                    .downloaded = download_now > 0 ? static_cast<std::uint64_t>(download_now) : 0,
                    .total = download_total > 0 ? static_cast<std::uint64_t>(download_total) : 0,
                });
            }
            return true;
        });

        const cpr::Response response = cpr::Get(cpr::Url{url}, write_cb, progress_cb);
        out.close();

        if (response.error) {
            spdlog::warn("download of {} failed: {}", url, response.error.message);
            remove_quietly(part);
            throw std::runtime_error(fmt::format("download of {} failed: {}", url, response.error.message));
        }
        if (response.status_code < 200 || response.status_code >= 300) {
            spdlog::warn("download of {} failed: HTTP {}", url, response.status_code);
            remove_quietly(part);
            throw std::runtime_error(fmt::format("download of {} failed: HTTP {}", url, response.status_code));
        }

        if (hasher) {
            if (const std::string actual = hasher->hex_digest(); actual != expected) {
                spdlog::warn("checksum mismatch for {}: expected {}, got {}", url, expected, actual);
                remove_quietly(part);
                throw std::runtime_error(
                    fmt::format("checksum mismatch for {}: expected {}, got {}", url, expected, actual));
            }
        }

        std::error_code ec;
        fs::rename(part, destination, ec);
        if (ec) {
            remove_quietly(part);
            throw std::runtime_error(
                fmt::format("cannot move {} to {}: {}", part.string(), destination.string(), ec.message()));
        }

        spdlog::debug("downloaded {} ({} bytes)", destination.string(),
                      static_cast<std::uint64_t>(response.downloaded_bytes));
        if (checksum && cache_) cache_->store(destination, *checksum);
    }
} // namespace hestia::engine
