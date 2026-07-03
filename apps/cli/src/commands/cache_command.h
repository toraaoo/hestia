#pragma once

#include "command.h"

namespace hestia::cli {
    // `hestia cache info|list|clear`
    class CacheCommand : public CommandGroup {
    public:
        CacheCommand();
    };
} // namespace hestia::cli
