#pragma once

#include <functional>

#include <hestia/client/facade.h>
#include <hestia/proto/download.h>

namespace hestia::client {
    using DownloadProgressCallback = std::function<void(const proto::DownloadProgress &)>;

    class Download : public Facade {
    public:
        using Facade::Facade;

        // Download a file via the daemon, blocking until it completes;
        // `on_progress` is invoked on the reader thread as bytes arrive. Throws
        // std::runtime_error on failure (bad request, network error, checksum
        // mismatch). Uses the session's single event-callback slot, so it
        // replaces any callback installed by Process::subscribe().
        void fetch(proto::DownloadSpec spec, const DownloadProgressCallback &on_progress = {});
    };
} // namespace hestia::client
