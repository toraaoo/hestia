#include "registry.h"

#include "commands/cache_command.h"
#include "commands/config_command.h"
#include "commands/daemon_command.h"
#include "commands/java_command.h"

namespace hestia::cli {
    std::vector<std::unique_ptr<Command>> make_commands() {
        std::vector<std::unique_ptr<Command>> commands;
        commands.push_back(std::make_unique<JavaCommand>());
        commands.push_back(std::make_unique<CacheCommand>());
        commands.push_back(std::make_unique<ConfigCommand>());
        commands.push_back(std::make_unique<DaemonCommand>());
        return commands;
    }
} // namespace hestia::cli
