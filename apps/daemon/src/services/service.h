#pragma once

// A unit of daemon functionality: one channel-prefix worth of typed handlers,
// registered onto the router through the Channels registrar. Mirrors the CLI's
// Command and the desktop's Feature; make_services() (registry.cc) is the one
// place a service is wired in.
namespace hestia::daemon {
    class Channels;

    class Service {
    public:
        virtual ~Service() = default;

        virtual void register_channels(Channels &on) = 0;
    };
} // namespace hestia::daemon
