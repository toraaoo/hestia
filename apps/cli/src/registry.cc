#include "registry.h"

#include "commands/autostart_command.h"
#include "commands/config_command.h"
#include "commands/download_command.h"
#include "commands/greet_command.h"

namespace hestia::cli {
    std::vector<std::unique_ptr<Command>> make_commands() {
        std::vector<std::unique_ptr<Command>> commands;
        commands.push_back(std::make_unique<GreetCommand>());
        commands.push_back(std::make_unique<ConfigCommand>());
        commands.push_back(std::make_unique<AutostartCommand>());
        commands.push_back(std::make_unique<DownloadCommand>());
        return commands;
    }
} // namespace hestia::cli
