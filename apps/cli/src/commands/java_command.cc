#include "commands/java_command.h"

#include <iostream>
#include <memory>
#include <optional>
#include <string>

#include <hestia/client.h>

#include "output.h"

namespace hestia::cli {
    namespace {
        class JavaAvailableCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("available", "List the Java release lines available to install");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::vector<std::vector<std::string>> rows;
                        {
                            Spinner const spinner("Fetching available releases");
                            for (const auto &release: client.java().releases()) {
                                rows.push_back({std::to_string(release.major), release.lts ? "yes" : ""});
                            }
                        }
                        print_table({"VERSION", "LTS"}, rows);
                    });
                });
            }
        };

        class JavaInstallCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("install", "Install a Java runtime via the daemon");
                cmd->add_option("major", major_, "Major version to install (e.g. 21); omit for the latest release");
                cmd->callback([this, &ctx] {
                    ctx.with_client([this](client::Client &client) {
                        std::optional<Spinner> spinner;
                        spinner.emplace(major_ > 0 ? "Resolving temurin " + std::to_string(major_)
                                                   : "Resolving the latest temurin");
                        ProgressBar download_bar("Downloading");
                        ProgressBar extract_bar("Extracting", false);
                        const auto on_progress = [&](const proto::JavaInstallProgress &p) {
                            if (p.phase == proto::JavaInstallPhase::downloading) {
                                spinner.reset();
                                download_bar.update(p.current, p.total);
                            } else if (p.phase == proto::JavaInstallPhase::extracting) {
                                download_bar.finish();
                                if (p.total > 0) {
                                    spinner.reset();
                                    extract_bar.update(p.current, p.total);
                                } else if (!spinner) {
                                    spinner.emplace("Extracting");
                                }
                            }
                        };
                        proto::JavaRuntime runtime;
                        try {
                            runtime = client.java().install(major_, on_progress);
                        } catch (...) {
                            spinner.reset();
                            download_bar.finish();
                            extract_bar.finish();
                            throw;
                        }
                        spinner.reset();
                        download_bar.finish();
                        extract_bar.finish();
                        std::cout << "Installed " << runtime.vendor << ' ' << runtime.release_name << " at "
                                  << runtime.home.string() << '\n';
                    });
                });
            }

        private:
            int major_ = 0;
        };

        class JavaListCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("list", "List installed Java runtimes");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::vector<std::vector<std::string>> rows;
                        {
                            Spinner const spinner("Fetching installed runtimes");
                            for (const auto &runtime: client.java().list()) {
                                rows.push_back({runtime.vendor, std::to_string(runtime.major), runtime.release_name,
                                                runtime.home.string()});
                            }
                        }
                        print_table({"VENDOR", "VERSION", "RELEASE", "PATH"}, rows);
                    });
                });
            }
        };

        class JavaUninstallCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("uninstall", "Remove an installed Java runtime");
                cmd->add_option("major", major_, "Major version to remove")->required();
                cmd->callback([this, &ctx] {
                    ctx.with_client([this](client::Client &client) {
                        {
                            Spinner const spinner("Uninstalling java " + std::to_string(major_));
                            client.java().uninstall(major_);
                        }
                        std::cout << "Uninstalled java " << major_ << '\n';
                    });
                });
            }

        private:
            int major_ = 0;
        };
    } // namespace

    JavaCommand::JavaCommand() : CommandGroup("java", "Manage Java runtimes") {
        add(std::make_unique<JavaAvailableCommand>());
        add(std::make_unique<JavaInstallCommand>());
        add(std::make_unique<JavaListCommand>());
        add(std::make_unique<JavaUninstallCommand>());
    }
} // namespace hestia::cli
