#include "services/downloads_service.h"

#include "downloads/download_manager.h"
#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <string>

#include <hestia/proto/download.h>

namespace hestia::daemon {
    void DownloadsService::register_channels(Channels &on) {
        on.handle<proto::DownloadStart>([](const proto::DownloadSpec &spec, HandlerContext &ctx) {
            if (spec.url.empty()) {
                throw ServiceError(ipc::errors::kBadRequest, "url is required");
            }
            if (spec.destination.empty()) {
                throw ServiceError(ipc::errors::kBadRequest, "dest is required");
            }
            if (!spec.destination.is_absolute()) {
                throw ServiceError(ipc::errors::kBadRequest, "dest must be an absolute path");
            }
            if (spec.checksum && !proto::is_valid_checksum(*spec.checksum)) {
                throw ServiceError(ipc::errors::kBadRequest,
                                   std::string(proto::to_string(spec.checksum->algorithm)) + " checksum must be " +
                                       std::to_string(proto::hex_digest_length(spec.checksum->algorithm)) +
                                       " hex characters");
            }
            return proto::DownloadStart::Result{
                .id = ctx.runtime.downloads().start(spec.url, spec.destination, spec.checksum, spec.id)};
        });
    }
} // namespace hestia::daemon
