#include "hestia/client/daemon.h"

#include "session.h"

namespace hestia::client {
    proto::DaemonStatus::Result Daemon::status() {
        return session_->call<proto::DaemonStatus>();
    }

    void Daemon::stop() {
        session_->call<proto::DaemonStop>();
    }
} // namespace hestia::client
