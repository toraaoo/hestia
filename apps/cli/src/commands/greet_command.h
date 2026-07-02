#pragma once

#include <string>

#include "command.h"

namespace hestia::cli {
    // `hestia greet` — a leaf command; thin wrapper over hestia::greeting.
    class GreetCommand : public Command {
    public:
        void register_command(CLI::App &parent, AppContext &ctx) override;

    private:
        std::string name_;
    };
} // namespace hestia::cli
