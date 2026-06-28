#pragma once

#include "command.h"

namespace hestia::cli {
    // `hestia tui` — launch the interactive terminal UI.
    class TuiCommand : public Command {
    public:
        void register_command(CLI::App &parent, AppContext &ctx) override;
    };
}
