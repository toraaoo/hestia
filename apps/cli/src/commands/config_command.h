#pragma once

#include "command.h"

namespace hestia::cli {
    // `hestia config` — a command group nesting `get` and `set` leaf commands.
    class ConfigCommand : public CommandGroup {
    public:
        ConfigCommand();
    };
}
