#pragma once

#include <memory>
#include <string>
#include <utility>
#include <vector>

#include <CLI/CLI.hpp>

#include <hestia/logging.h>

namespace hestia::cli {
    // Cross-cutting options bound on the root application and made available to
    // every command through AppContext.
    struct GlobalOptions {
        bool verbose = false;
        bool quiet = false;

        // Override for Hestia's data directory (empty = use HESTIA_HOME or the
        // platform default). Bound to the --home flag.
        std::string home;

        // Minimum log level implied by the verbose/quiet flags.
        LogLevel log_level() const {
            if (verbose) {
                return LogLevel::debug;
            }
            if (quiet) {
                return LogLevel::warn;
            }
            return LogLevel::info;
        }
    };

    // State threaded through command registration and execution: the parsed
    // global options and the collected process exit code. Passed by reference to
    // every command; commands must not retain the reference beyond the lifetime
    // of the run.
    struct AppContext {
        GlobalOptions global;
        int exit_code = 0;
    };

    // A unit of CLI functionality. Implementations register themselves onto a
    // parent CLI::App, which may be the root application or another command's
    // app — letting commands nest to any depth.
    class Command {
    public:
        virtual ~Command() = default;

        // Add this command (its options, flags, and callback) to `parent`.
        virtual void register_command(CLI::App &parent, AppContext &ctx) = 0;
    };

    // Base for a command that groups child commands (e.g. `hestia config set`).
    // A bare group prints its help; children register onto the group's own app,
    // so the same mechanism nests recursively.
    class CommandGroup : public Command {
    public:
        CommandGroup(std::string name, std::string description)
            : name_(std::move(name)), description_(std::move(description)) {}

        void register_command(CLI::App &parent, AppContext &ctx) override {
            auto *group = parent.add_subcommand(name_, description_);
            group->require_subcommand();
            for (auto &child : children_) {
                child->register_command(*group, ctx);
            }
        }

    protected:
        void add(std::unique_ptr<Command> child) {
            children_.push_back(std::move(child));
        }

    private:
        std::string name_;
        std::string description_;
        std::vector<std::unique_ptr<Command>> children_;
    };
}
