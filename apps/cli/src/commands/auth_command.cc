#include "commands/auth_command.h"

#include <iostream>
#include <memory>
#include <optional>
#include <string>

#include <hestia/client.h>

#include "output.h"

namespace hestia::cli {
    namespace {
        class AuthLoginCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("login", "Sign in to a Microsoft account");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::optional<Spinner> spinner;
                        spinner.emplace("Requesting a sign-in code");
                        const auto account = client.accounts().login([&](const proto::AccountLoginCode &code) {
                            spinner.reset();
                            std::cout << "Open " << code.verification_uri << " and enter the code "
                                      << code.user_code << " (valid for " << code.expires_in / 60
                                      << " minutes)\n";
                            spinner.emplace("Waiting for the sign-in to complete");
                        });
                        spinner.reset();
                        std::cout << "Signed in as " << account.name << " (" << account.uuid << ")\n";
                    });
                });
            }
        };

        class AuthListCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("list", "List signed-in accounts");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::vector<std::vector<std::string>> rows;
                        {
                            Spinner const spinner("Fetching accounts");
                            for (const auto &account: client.accounts().list()) {
                                rows.push_back({account.name, account.uuid});
                            }
                        }
                        print_table({"NAME", "UUID"}, rows);
                    });
                });
            }
        };

        class AuthLogoutCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("logout", "Sign out of an account and forget its tokens");
                cmd->add_option("account", account_, "The account's name or uuid")->required();
                cmd->callback([this, &ctx] {
                    ctx.with_client([this](client::Client &client) {
                        client.accounts().remove(account_);
                        std::cout << "Signed out " << account_ << '\n';
                    });
                });
            }

        private:
            std::string account_;
        };
    } // namespace

    AuthCommand::AuthCommand() : CommandGroup("auth", "Manage Minecraft accounts (Microsoft sign-in)") {
        add(std::make_unique<AuthLoginCommand>());
        add(std::make_unique<AuthListCommand>());
        add(std::make_unique<AuthLogoutCommand>());
    }
} // namespace hestia::cli
