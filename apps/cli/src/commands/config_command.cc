#include "commands/config_command.h"

#include <iostream>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

#include <hestia/client.h>

#include "output.h"

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
                            if (value->is_string()) {
                                std::cout << value->get<std::string>() << '\n';
                            } else {
                                std::cout << value->dump(2) << '\n';
                            }
                        } else {
                            std::cerr << "unknown config key: " << key_ << '\n';
                            ctx.exit_code = 1;
                        }
                    });
                });
            }

        private:
            std::string key_;
        };

        // Flatten a settings tree into KEY/VALUE rows, sub-objects as dotted paths.
        void flatten(const nlohmann::json &node, const std::string &prefix,
                     std::vector<std::vector<std::string>> &rows) {
            if (node.is_object()) {
                for (const auto &[key, value]: node.items()) {
                    std::string path = prefix;
                    if (!path.empty()) path += '.';
                    path += key;
                    flatten(value, path, rows);
                }
                return;
            }
            rows.push_back({prefix, node.is_string() ? node.get<std::string>() : node.dump()});
        }

        // `hestia config list` — print the effective settings as a table.
        class ConfigListCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("list", "List all config entries");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::vector<std::vector<std::string>> rows;
                        flatten(client.config().list(), "", rows);
                        print_table({"KEY", "VALUE"}, rows);
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
                    ctx.with_client([this](client::Client &client) {
                        auto value = nlohmann::json::parse(value_, nullptr, false);
                        if (value.is_discarded()) value = value_;
                        client.config().set(key_, std::move(value));
                    });
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
