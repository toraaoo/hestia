#include "commands/cache_command.h"

#include <iostream>
#include <memory>
#include <string>

#include <hestia/client/client.h>

#include "output.h"

namespace hestia::cli {
    namespace {
        class CacheInfoCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("info", "Show download-cache location and usage");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        client::CacheStats stats;
                        {
                            Spinner const spinner("Fetching cache usage");
                            stats = client.cache_info();
                        }
                        std::cout << "Location: " << stats.path << '\n'
                                  << "Entries:  " << stats.entries << '\n'
                                  << "Size:     " << human_size(stats.bytes) << '\n';
                    });
                });
            }
        };

        class CacheListCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("list", "List cached downloads");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        std::vector<std::vector<std::string>> rows;
                        {
                            Spinner const spinner("Fetching cache entries");
                            for (const auto &entry: client.cache_list()) {
                                rows.push_back({entry.algorithm, entry.hex.substr(0, 12), human_size(entry.size)});
                            }
                        }
                        print_table({"ALGORITHM", "CHECKSUM", "SIZE"}, rows);
                    });
                });
            }
        };

        class CacheClearCommand : public Command {
        public:
            void register_command(CLI::App &parent, AppContext &ctx) override {
                auto *cmd = parent.add_subcommand("clear", "Remove every cached download");
                cmd->callback([&ctx] {
                    ctx.with_client([](client::Client &client) {
                        client::CacheStats freed;
                        {
                            Spinner const spinner("Clearing cache");
                            freed = client.cache_clear();
                        }
                        std::cout << "Freed " << human_size(freed.bytes) << " (" << freed.entries << " entries)\n";
                    });
                });
            }
        };
    } // namespace

    CacheCommand::CacheCommand() : CommandGroup("cache", "Manage the download cache") {
        add(std::make_unique<CacheInfoCommand>());
        add(std::make_unique<CacheListCommand>());
        add(std::make_unique<CacheClearCommand>());
    }
} // namespace hestia::cli
