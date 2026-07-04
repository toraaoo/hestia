#pragma once

#include <filesystem>
#include <string>

#include <hestia/engine/accounts.h>
#include <hestia/engine/cache.h>
#include <hestia/engine/config.h>
#include <hestia/engine/java.h>

namespace hestia::engine {
    // The daemon-internal aggregate root; frontends reach it only over IPC.
    // Adding a domain = a public header, a src/<domain>/ folder, and a member +
    // getter here. See docs/contributing.md.
    class Engine {
    public:
        explicit Engine(const std::filesystem::path &override_home = {});

        const std::filesystem::path &data_home() const { return data_home_; }

        // Persists `dir` (empty reverts to the default), re-resolves, and
        // repoints every subsystem on the running daemon.
        std::filesystem::path set_data_home(const std::string &dir);

        Config &config() { return config_; }
        const Config &config() const { return config_; }

        Cache &cache() { return cache_; }
        const Cache &cache() const { return cache_; }

        Java &java() { return java_; }
        const Java &java() const { return java_; }

        Accounts &accounts() { return accounts_; }
        const Accounts &accounts() const { return accounts_; }

    private:
        std::filesystem::path data_home_;
        Config config_;
        Cache cache_;
        Java java_;
        Accounts accounts_;
    };
} // namespace hestia::engine
