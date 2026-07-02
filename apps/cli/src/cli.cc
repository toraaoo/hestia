#include "cli.h"

#include <cstdlib>
#include <iostream>

#include <CLI/CLI.hpp>

#include <hestia/app_info.h>
#include <hestia/logging.h>

#include "command.h"
#include "registry.h"

namespace hestia::cli {
    int run(int argc, char **argv) {
        CLI::App app{"Hestia command-line interface", "hestia"};
        app.set_version_flag("--version", APP_VERSION);

        // Allow a single optional subcommand, and let global flags appear at any
        // position (e.g. `hestia -v greet` or `hestia greet -v`).
        app.require_subcommand(0, 1);
        app.fallthrough();

        AppContext ctx;
        auto *verbose = app.add_flag("-v,--verbose", ctx.global.verbose, "Enable verbose (debug) logging");
        app.add_flag("-q,--quiet", ctx.global.quiet, "Only show warnings and errors")->excludes(verbose);
        app.add_option("--home", ctx.global.home,
                       "Override Hestia's data directory "
                       "(else $HESTIA_HOME, else the platform default)");

        // Configure logging once global flags are parsed, before any command
        // callback runs. In the daemon model the data directory is daemon-global,
        // so --home is exported as $HESTIA_HOME and only takes effect when this
        // invocation auto-spawns the daemon; a daemon already running keeps its
        // own data directory.
        app.parse_complete_callback([&ctx] {
            init_logging(ctx.global.log_level());
#if !defined(_WIN32)
            if (!ctx.global.home.empty()) {
                ::setenv("HESTIA_HOME", ctx.global.home.c_str(), 1);
            }
#endif
        });

        const auto commands = make_commands();
        for (const auto &command: commands) {
            command->register_command(app, ctx);
        }

        try {
            app.parse(argc, argv);
        } catch (const CLI::ParseError &e) {
            return app.exit(e);
        }

        // No subcommand given: show usage.
        if (app.get_subcommands().empty()) {
            std::cout << app.help();
            return 0;
        }

        return ctx.exit_code;
    }
} // namespace hestia::cli
