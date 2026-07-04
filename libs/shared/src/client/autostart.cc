#include "hestia/client/autostart.h"

#include "session.h"

namespace hestia::client {
    void Autostart::enable() {
        session_->call<proto::AutostartEnable>();
    }

    void Autostart::disable() {
        session_->call<proto::AutostartDisable>();
    }

    bool Autostart::status() {
        return session_->call<proto::AutostartStatus>().enabled;
    }
} // namespace hestia::client
