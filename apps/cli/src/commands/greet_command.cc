#include "commands/greet_command.h"

#include <iostream>

#include <spdlog/spdlog.h>

#include <hestia/client/client.h>

namespace hestia::cli {
    void GreetCommand::register_command(CLI::App &parent, AppContext &ctx) {
        auto *cmd = parent.add_subcommand("greet", "Print a friendly greeting");
        cmd->add_option("-n,--name", name_, "Name to greet");
        cmd->callback([this, &ctx] {
            spdlog::debug("greet: name='{}'", name_);
            ctx.with_client([this](client::Client &client) { std::cout << client.greet(name_) << '\n'; });
        });
    }
} // namespace hestia::cli
