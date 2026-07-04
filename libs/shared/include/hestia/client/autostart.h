#pragma once

#include <hestia/client/facade.h>
#include <hestia/proto/autostart.h>

namespace hestia::client {
    // Register/unregister the daemon to start with the user session, backed by
    // the platform's native mechanism (systemd user unit / LaunchAgent / logon
    // Scheduled Task).
    class Autostart : public Facade {
    public:
        using Facade::Facade;

        void enable();
        void disable();
        bool status();
    };
} // namespace hestia::client
