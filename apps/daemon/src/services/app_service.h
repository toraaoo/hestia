#pragma once

#include "services/service.h"

namespace hestia::daemon {
    class AppService : public Service {
    public:
        void register_channels(Channels &on) override;
    };
} // namespace hestia::daemon
