#include "commands/auth_command.h"

#include <cctype>
#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <string>

#include <hestia/client.h>

#include "output.h"

#if defined(_WIN32)
#include <windows.h>
#endif

namespace hestia::cli {
    namespace {
        void open_browser(const std::string &url) {
            if (!url.starts_with("https://") || url.find_first_of("\"'") != std::string::npos) return;
#if defined(_WIN32)
            ::ShellExecuteA(nullptr, "open", url.c_str(), nullptr, nullptr, SW_SHOWNORMAL);
#elif defined(__APPLE__)
            std::system(("open '" + url + "' >/dev/null 2>&1").c_str());
#else
            std::system(("xdg-open '" + url + "' >/dev/null 2>&1 &").c_str());
#endif
        }

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

        std::string extract_code(const std::string &pasted) {
            auto input = trim(pasted);
            const auto marker = input.find("code=");
            if (marker == std::string::npos) return input;
            const auto begin = marker + 5;
            const auto end = input.find('&', begin);
            return url_decode(input.substr(begin, end == std::string::npos ? std::string::npos : end - begin));
        }

        void wait_for_enter(const std::string &prompt) {
            std::cout << prompt << std::flush;
            std::string discard;
            std::getline(std::cin, discard);
        }

        proto::Account device_code_login(client::Client &client) {
            proto::AccountLoginBegin::Result flow;
            {
                Spinner const spinner("Requesting a sign-in code");
                flow = client.accounts().begin_login(proto::LoginMethod::device_code);
            }
            std::cout << "\nTo sign in, open\n\n  " << flow.verification_uri << "\n\nand enter the code\n\n  "
                      << flow.user_code << "\n\n";
            wait_for_enter("Press Enter to open your browser... ");
            open_browser(flow.verification_uri);

            Spinner const spinner("Waiting for you to finish in the browser");
            return client.accounts().complete_login(flow.id);
        }

        proto::Account sisu_login(client::Client &client) {
            proto::AccountLoginBegin::Result flow;
            {
                Spinner const spinner("Preparing the Microsoft sign-in");
                flow = client.accounts().begin_login(proto::LoginMethod::sisu);
            }
            std::cout << "Open this URL in your browser and sign in:\n\n  " << flow.url << "\n\n";
            wait_for_enter("Press Enter to open your browser... ");
            open_browser(flow.url);
            std::cout << "You'll land on a blank page — paste its full address (or just the\n"
                         "code) here, then press Enter:\n> "
                      << std::flush;

            std::string input;
            std::getline(std::cin, input);
            const auto code = extract_code(input);
            if (code.empty()) {
                throw std::runtime_error("no authorization code was pasted");
            }

            Spinner const spinner("Completing the sign-in");
            return client.accounts().complete_login(flow.id, code);
        }

        class AuthLoginCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("login", "Sign in to a Microsoft account");
                cmd->add_flag("--sisu", sisu_,
                              "Sign in through the browser-redirect (sisu) flow instead of a device code");
                cmd->callback([this, &ctx] {
                    ctx.with_client([this](client::Client &client) {
                        const auto account = sisu_ ? sisu_login(client) : device_code_login(client);
                        std::cout << "Signed in as " << account.name << " (" << account.uuid << ")\n";
                    });
                });
            }

        private:
            bool sisu_ = false;
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
