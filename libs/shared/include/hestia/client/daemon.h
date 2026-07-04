#pragma once

#include <hestia/client/facade.h>
#include <hestia/proto/daemon.h>

namespace hestia::client {
    // Daemon lifecycle. stop() asks the daemon to shut itself down; it answers
    // before exiting, so poll the endpoint to observe the exit.
    class Daemon : public Facade {
    public:
        using Facade::Facade;

        proto::DaemonStatus::Result status();
        void stop();
    };
} // namespace hestia::client
