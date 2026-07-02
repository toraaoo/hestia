#include "services/services.h"

#include "download_manager.h"
#include "handler_context.h"
#include "router.h"

#include <cctype>
#include <filesystem>
#include <optional>
#include <string>
#include <utility>

#include <hestia/engine/checksum.h>
#include <hestia/engine/downloader.h>
#include <hestia/ipc/errors.h>

namespace hestia::daemon {
    namespace {
        bool is_hex(const std::string &s) {
            for (const char c : s) {
                if (!std::isxdigit(static_cast<unsigned char>(c))) return false;
            }
            return true;
        }
    }

    void register_downloads_service(Router &router) {
        router.on("download.start", [](const ipc::Request &req, HandlerContext &ctx) {
            const auto url = req.payload.value("url", std::string{});
            if (url.empty()) {
                return ipc::Response::failure(ipc::errors::kBadRequest, "url is required");
            }

            const std::filesystem::path destination(req.payload.value("dest", std::string{}));
            if (destination.empty()) {
                return ipc::Response::failure(ipc::errors::kBadRequest, "dest is required");
            }
            if (!destination.is_absolute()) {
                return ipc::Response::failure(ipc::errors::kBadRequest,
                                              "dest must be an absolute path");
            }

            std::optional<engine::Checksum> checksum;
            if (req.payload.contains("checksum")) {
                const auto &c = req.payload["checksum"];
                const auto name = c.value("algorithm", std::string{});
                const auto algorithm = engine::parse_hash_algorithm(name);
                if (!algorithm) {
                    return ipc::Response::failure(ipc::errors::kBadRequest,
                                                  "unknown checksum algorithm: " + name);
                }
                auto hex = c.value("hex", std::string{});
                const auto expected_len = engine::hex_digest_length(*algorithm);
                if (hex.size() != expected_len || !is_hex(hex)) {
                    return ipc::Response::failure(
                        ipc::errors::kBadRequest,
                        name + " checksum must be " + std::to_string(expected_len) +
                            " hex characters");
                }
                checksum = engine::Checksum{.algorithm = *algorithm, .hex = std::move(hex)};
            }

            const auto id = ctx.downloads.start(url, destination, std::move(checksum),
                                                req.payload.value("id", std::string{}));
            return ipc::Response::success({{"id", id}});
        });
    }
}
