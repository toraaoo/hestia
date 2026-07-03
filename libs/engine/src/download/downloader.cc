#include <hestia/engine/downloader.h>

#include <cctype>
#include <fstream>
#include <stdexcept>
#include <system_error>

#include <cpr/cpr.h>
#include <fmt/format.h>

#include "download/checksum.h"

namespace hestia::engine {
    namespace fs = std::filesystem;

    namespace {
        std::string to_lower(std::string s) {
            for (auto &c: s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
            return s;
        }

        void validate(const ipc::Checksum &checksum) {
            if (!ipc::is_valid_checksum(checksum)) {
                throw std::runtime_error(fmt::format("invalid checksum '{}': expected {} hex characters", checksum.hex,
                                                     ipc::hex_digest_length(checksum.algorithm)));
            }
        }

        void remove_quietly(const fs::path &path) {
            std::error_code ec;
            fs::remove(path, ec);
        }
    } // namespace

    void Downloader::fetch(const std::string &url, const fs::path &destination,
                           const std::optional<ipc::Checksum> &checksum,
                           const DownloadProgressCallback &on_progress) const {
        if (url.empty()) throw std::runtime_error("download url is empty");
        if (checksum) validate(*checksum);

        if (const auto parent = destination.parent_path(); !parent.empty()) {
            fs::create_directories(parent);
        }

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
                on_progress(ipc::DownloadProgress{
                    .downloaded = download_now > 0 ? static_cast<std::uint64_t>(download_now) : 0,
                    .total = download_total > 0 ? static_cast<std::uint64_t>(download_total) : 0,
                });
            }
            return true;
        });

        const cpr::Response response = cpr::Get(cpr::Url{url}, write_cb, progress_cb);
        out.close();

        if (response.error) {
            remove_quietly(part);
            throw std::runtime_error(fmt::format("download of {} failed: {}", url, response.error.message));
        }
        if (response.status_code < 200 || response.status_code >= 300) {
            remove_quietly(part);
            throw std::runtime_error(fmt::format("download of {} failed: HTTP {}", url, response.status_code));
        }

        if (hasher) {
            if (const std::string actual = hasher->hex_digest(); actual != expected) {
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
    }
} // namespace hestia::engine
