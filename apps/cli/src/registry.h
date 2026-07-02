#pragma once

#include <memory>
#include <vector>

#include "command.h"

namespace hestia::cli {
    // Construct the set of top-level commands. This is the single place a new
    // command is wired into the application.
    std::vector<std::unique_ptr<Command>> make_commands();
} // namespace hestia::cli
