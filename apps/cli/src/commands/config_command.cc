#include "commands/config_command.h"

#include <iostream>
#include <memory>
#include <string>

#include <hestia/client.h>

namespace hestia::cli {
    namespace {
        // `hestia config get <key>`
        class ConfigGetCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("get", "Print the value of a config key");
                cmd->add_option("key", key_, "Config key")->required();
                cmd->callback([this, &ctx] {
                    ctx.with_client([this, &ctx](client::Client &client) {
                        if (const auto value = client.config().get(key_)) {
                            std::cout << *value << '\n';
                        } else {
                            std::cerr << "key not found: " << key_ << '\n';
                            ctx.exit_code = 1;
                        }
                    });
                });
            }

        private:
            std::string key_;
        };

        // `hestia config home` — print the resolved data directory.
        class ConfigHomeCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("home", "Print the resolved data directory");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) { std::cout << client.config().home().string() << '\n'; });
                });
            }
        };

        // `hestia config set-home <dir>` — persist the data directory for future
        // runs. With no argument, reverts to the default.
        class ConfigSetHomeCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("set-home", "Persist the data directory used by future runs");
                cmd->add_option("dir", dir_, "Directory to use (omit to revert to the default)");
                cmd->callback([this, &ctx] {
                    ctx.with_client([this](client::Client &client) {
                        const auto home = client.config().set_home(dir_);
                        if (dir_.empty()) {
                            std::cout << "reverted to default: " << home.string() << '\n';
                        } else {
                            std::cout << "data directory set to: " << home.string() << '\n';
                        }
                    });
                });
            }

        private:
            std::string dir_;
        };

        // `hestia config set <key> <value>`
        class ConfigSetCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("set", "Set the value of a config key");
                cmd->add_option("key", key_, "Config key")->required();
                cmd->add_option("value", value_, "Config value")->required();
                cmd->callback([this, &ctx] {
                    ctx.with_client([this](client::Client &client) { client.config().set(key_, value_); });
                });
            }

        private:
            std::string key_;
            std::string value_;
        };
    } // namespace

    ConfigCommand::ConfigCommand() : CommandGroup("config", "Read and write configuration") {
        add(std::make_unique<ConfigGetCommand>());
        add(std::make_unique<ConfigSetCommand>());
        add(std::make_unique<ConfigHomeCommand>());
        add(std::make_unique<ConfigSetHomeCommand>());
    }
} // namespace hestia::cli
