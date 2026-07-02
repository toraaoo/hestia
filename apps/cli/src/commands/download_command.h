#pragma once

#include <string>

#include "command.h"

namespace hestia::cli {
    // `hestia download` — a temporary tester for the daemon's download channel.
    class DownloadCommand : public Command {
    public:
        void register_command(CLI::App &parent, AppContext &ctx) override;

    private:
        std::string url_;
        std::string dest_;
        std::string checksum_;
    };
} // namespace hestia::cli
