#include "services/services.h"

#include "downloads/download_manager.h"
#include "runtime/handler_context.h"
#include "runtime/router.h"
#include "runtime/runtime.h"

#include <exception>
#include <string>
#include <utility>

#include <hestia/ipc/download.h>
#include <hestia/ipc/download_codec.h>
#include <hestia/ipc/errors.h>

namespace hestia::daemon {
    void register_downloads_service(Router &router) {
        router.on("download.start", [](const ipc::Request &req, HandlerContext &ctx) {
            ipc::DownloadSpec spec;
            try {
                spec = ipc::download_spec_from_json(req.payload);
            } catch (const std::exception &e) {
                return ipc::Response::failure(ipc::errors::kBadRequest, e.what());
            }

            if (spec.url.empty()) {
                return ipc::Response::failure(ipc::errors::kBadRequest, "url is required");
            }
            if (spec.destination.empty()) {
                return ipc::Response::failure(ipc::errors::kBadRequest, "dest is required");
            }
            if (!spec.destination.is_absolute()) {
                return ipc::Response::failure(ipc::errors::kBadRequest, "dest must be an absolute path");
            }
            if (spec.checksum && !ipc::is_valid_checksum(*spec.checksum)) {
                return ipc::Response::failure(
                    ipc::errors::kBadRequest,
                    std::string(ipc::to_string(spec.checksum->algorithm)) + " checksum must be " +
                        std::to_string(ipc::hex_digest_length(spec.checksum->algorithm)) + " hex characters");
            }

            const auto id =
                ctx.runtime.downloads().start(spec.url, spec.destination, std::move(spec.checksum), spec.id);
            return ipc::Response::success({{"id", id}});
        });
    }
} // namespace hestia::daemon
