#include "commands/autostart_command.h"

#include <iostream>
#include <memory>

#include <hestia/client/client.h>

namespace hestia::cli {
    namespace {
        // `hestia autostart enable` — register the daemon to start at login.
        class AutostartEnableCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("enable", "Start the daemon at login");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        client.autostart_enable();
                        std::cout << "autostart enabled\n";
                    });
                });
            }
        };

        // `hestia autostart disable` — remove the login registration.
        class AutostartDisableCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("disable", "Do not start the daemon at login");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        client.autostart_disable();
                        std::cout << "autostart disabled\n";
                    });
                });
            }
        };

        // `hestia autostart status` — report whether login start is registered.
        class AutostartStatusCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("status", "Show whether the daemon starts at login");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::cout << (client.autostart_status() ? "enabled" : "disabled") << '\n';
                    });
                });
            }
        };
    } // namespace

    AutostartCommand::AutostartCommand() : CommandGroup("autostart", "Manage starting the daemon at login") {
        add(std::make_unique<AutostartEnableCommand>());
        add(std::make_unique<AutostartDisableCommand>());
        add(std::make_unique<AutostartStatusCommand>());
    }
} // namespace hestia::cli
