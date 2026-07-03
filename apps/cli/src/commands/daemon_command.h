#pragma once

#include "command.h"

namespace hestia::cli {
    // `hestia daemon status|start|stop|restart`
    class DaemonCommand : public CommandGroup {
    public:
        DaemonCommand();
    };
} // namespace hestia::cli
