#include "commands/auth_command.h"

#include <cctype>
#include <iostream>
#include <stdexcept>
#include <string>

#include <hestia/client.h>

#include "output.h"

namespace hestia::cli {
    namespace {
        std::string url_decode(const std::string &value) {
            std::string out;
            out.reserve(value.size());
            for (std::size_t i = 0; i < value.size(); ++i) {
                if (value[i] == '%' && i + 2 < value.size() && std::isxdigit(static_cast<unsigned char>(value[i + 1])) &&
                    std::isxdigit(static_cast<unsigned char>(value[i + 2]))) {
                    out.push_back(static_cast<char>(std::stoul(value.substr(i + 1, 2), nullptr, 16)));
                    i += 2;
                } else if (value[i] == '+') {
                    out.push_back(' ');
                } else {
                    out.push_back(value[i]);
                }
            }
            return out;
        }

        std::string trim(const std::string &value) {
            const auto begin = value.find_first_not_of(" \t\r\n");
            if (begin == std::string::npos) return {};
            const auto end = value.find_last_not_of(" \t\r\n");
            return value.substr(begin, end - begin + 1);
        }

        // Accepts either the full redirect URL the browser lands on or a bare code.
        std::string extract_code(const std::string &pasted) {
            auto input = trim(pasted);
            const auto marker = input.find("code=");
            if (marker == std::string::npos) return input;
            const auto begin = marker + 5;
            const auto end = input.find('&', begin);
            return url_decode(input.substr(begin, end == std::string::npos ? std::string::npos : end - begin));
        }

        class AuthLoginCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("login", "Sign in to a Microsoft account");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        proto::AccountLoginBegin::Result flow;
                        {
                            Spinner const spinner("Preparing the Microsoft sign-in");
                            flow = client.accounts().begin_login();
                        }
                        std::cout << "Open this URL in your browser and sign in:\n\n  " << flow.url << "\n\n"
                                  << "You'll land on a blank page — paste its full address (or just the\n"
                                     "code) here, then press Enter:\n> "
                                  << std::flush;

                        std::string input;
                        std::getline(std::cin, input);
                        const auto code = extract_code(input);
                        if (code.empty()) {
                            throw std::runtime_error("no authorization code was pasted");
                        }

                        proto::Account account;
                        {
                            Spinner const spinner("Completing the sign-in");
                            account = client.accounts().complete_login(flow.id, code);
                        }
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
