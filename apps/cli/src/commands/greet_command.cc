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
            try {
                auto client = client::Client::connect();
                std::cout << client.greet(name_) << '\n';
                ctx.exit_code = 0;
            } catch (const std::exception &e) {
                std::cerr << "hestia: " << e.what() << '\n';
                ctx.exit_code = 1;
            }
        });
    }
}
