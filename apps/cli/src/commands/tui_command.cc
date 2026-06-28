#include "commands/tui_command.h"

#include "hestia/tui/run.h"

namespace hestia::cli {
    void TuiCommand::register_command(CLI::App &parent, AppContext &ctx) {
        auto *cmd = parent.add_subcommand("tui", "Launch the interactive terminal UI");
        cmd->callback([&ctx] {
            ctx.exit_code = hestia::tui::run();
        });
    }
}
