#include "commands/config_command.h"

#include <iostream>
#include <memory>
#include <string>

#include <nlohmann/json.hpp>

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

        // `hestia config list` — print the whole store as formatted JSON.
        class ConfigListCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("list", "Print all config entries as JSON");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::cout << nlohmann::json(client.config().list()).dump(2) << '\n';
                    });
                });
            }
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
        add(std::make_unique<ConfigListCommand>());
    }
} // namespace hestia::cli
