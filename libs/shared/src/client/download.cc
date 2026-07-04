#include "hestia/client/download.h"

#include <stdexcept>
#include <string>

#include "session.h"

namespace hestia::client {
    void Download::fetch(proto::DownloadSpec spec, const DownloadProgressCallback &on_progress) {
        // Validate the checksum here so a malformed one fails clearly without a
        // round-trip.
        if (spec.checksum && !proto::is_valid_checksum(*spec.checksum)) {
            throw std::runtime_error(std::string(proto::to_string(spec.checksum->algorithm)) + " checksum must be " +
                                     std::to_string(proto::hex_digest_length(spec.checksum->algorithm)) +
                                     " hex characters");
        }
        if (spec.id.empty()) spec.id = job_id("dl");

        session_->run_job(
            spec.id, proto::DownloadDoneEvent::kTopic, proto::DownloadErrorEvent::kTopic,
            [&on_progress](const ipc::Event &event) {
                if (event.topic != proto::DownloadProgressEvent::kTopic || !on_progress) return;
                on_progress(event.payload.get<proto::DownloadProgressEvent>().progress);
            },
            [&] { session_->call<proto::DownloadStart>(spec); });
    }
} // namespace hestia::client
