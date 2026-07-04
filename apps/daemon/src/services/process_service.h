#pragma once

#include "services/service.h"

namespace hestia::daemon {
    class ProcessService : public Service {
    public:
        void register_channels(Channels &on) override;
    };
} // namespace hestia::daemon
