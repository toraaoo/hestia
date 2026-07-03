#pragma once

#include "command.h"

namespace hestia::cli {
    // `hestia java available|install|list|uninstall`
    class JavaCommand : public CommandGroup {
    public:
        JavaCommand();
    };
} // namespace hestia::cli
