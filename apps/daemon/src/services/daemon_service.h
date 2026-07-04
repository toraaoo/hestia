#pragma once

#include "services/service.h"

namespace hestia::daemon {
    class DaemonService : public Service {
    public:
        void register_channels(Channels &on) override;
    };
} // namespace hestia::daemon
