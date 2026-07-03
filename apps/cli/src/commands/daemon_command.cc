#include "commands/daemon_command.h"

#include <chrono>
#include <exception>
#include <iostream>
#include <memory>
#include <optional>
#include <string>
#include <thread>

#include <hestia/client/client.h>
#include <hestia/ipc/endpoint.h>
#include <hestia/ipc/transport.h>

#include "output.h"

namespace hestia::cli {
    namespace {
        // These commands connect without auto-spawn, so they cannot use
        // AppContext::with_client; this replicates its error contract.
        void guarded(AppContext &ctx, const std::function<void()> &body) {
            try {
                body();
            } catch (const std::exception &e) {
                std::cerr << "hestia: " << e.what() << '\n';
                ctx.exit_code = 1;
            }
        }

        std::optional<client::Client> connect_running() {
            try {
                return client::Client::connect(/*auto_spawn=*/false);
            } catch (const std::exception &) {
                return std::nullopt;
            }
        }

        // The daemon answers daemon.stop before exiting; the endpoint vanishing
        // is what proves it is gone.
        bool wait_stopped() {
            for (int attempt = 0; attempt < 50; ++attempt) {
                try {
                    ipc::connect(ipc::default_endpoint());
                } catch (const std::exception &) {
                    return true;
                }
                std::this_thread::sleep_for(std::chrono::milliseconds(100));
            }
            return false;
        }

        std::string human_duration(long long seconds) {
            std::string out;
            if (seconds >= 3600) out += std::to_string(seconds / 3600) + "h ";
            if (seconds >= 60) out += std::to_string(seconds % 3600 / 60) + "m ";
            out += std::to_string(seconds % 60) + "s";
            return out;
        }

        void print_status(const client::DaemonStatus &status) {
            std::cout << "State:    running\n"
                      << "PID:      " << status.pid << '\n'
                      << "Version:  " << status.version << '\n'
                      << "Uptime:   " << human_duration(status.uptime_seconds) << '\n'
                      << "Home:     " << status.home << '\n'
                      << "Log:      " << status.log << '\n';
        }

        bool stop_running_daemon(AppContext &ctx) {
            auto client = connect_running();
            if (!client) {
                std::cout << "State:    stopped\n";
                return false;
            }
            Spinner spinner("Stopping daemon");
            client->daemon_stop();
            if (!wait_stopped()) {
                spinner.stop();
                std::cerr << "hestia: daemon did not stop within 5 seconds\n";
                ctx.exit_code = 1;
                return false;
            }
            return true;
        }

        void start_daemon(AppContext &ctx, const char *done_verb) {
            try {
                std::optional<client::DaemonStatus> status;
                {
                    Spinner const spinner("Starting daemon");
                    auto client = client::Client::connect(/*auto_spawn=*/true);
                    status = client.daemon_status();
                }
                std::cout << done_verb << " (version " << status->version << ", pid " << status->pid << ")\n";
            } catch (const std::exception &e) {
                std::cerr << "hestia: " << e.what() << '\n';
                ctx.exit_code = 1;
            }
        }

        class DaemonStatusCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("status", "Show whether the daemon is running");
                cmd->callback([&ctx] {
                    guarded(ctx, [] {
                        if (auto client = connect_running()) {
                            print_status(client->daemon_status());
                        } else {
                            std::cout << "State:    stopped\n";
                        }
                    });
                });
            }
        };

        class DaemonStartCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("start", "Start the daemon if it is not running");
                cmd->callback([&ctx] {
                    guarded(ctx, [&ctx] {
                        if (auto client = connect_running()) {
                            const auto status = client->daemon_status();
                            std::cout << "Already running (version " << status.version << ", pid " << status.pid
                                      << ")\n";
                            return;
                        }
                        start_daemon(ctx, "Started");
                    });
                });
            }
        };

        class DaemonStopCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("stop", "Stop the running daemon");
                cmd->callback([&ctx] {
                    guarded(ctx, [&ctx] {
                        if (stop_running_daemon(ctx)) std::cout << "Stopped\n";
                    });
                });
            }
        };

        class DaemonRestartCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("restart", "Restart the daemon (picks up a new binary)");
                cmd->callback([&ctx] {
                    guarded(ctx, [&ctx] {
                        if (connect_running() && !stop_running_daemon(ctx)) return;
                        start_daemon(ctx, "Restarted");
                    });
                });
            }
        };
    } // namespace

    DaemonCommand::DaemonCommand() : CommandGroup("daemon", "Manage the background daemon") {
        add(std::make_unique<DaemonStatusCommand>());
        add(std::make_unique<DaemonStartCommand>());
        add(std::make_unique<DaemonStopCommand>());
        add(std::make_unique<DaemonRestartCommand>());
    }
} // namespace hestia::cli
