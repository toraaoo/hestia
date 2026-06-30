#pragma once

#include <filesystem>
#include <string>

#include <hestia/engine/config_store.h>

namespace hestia::engine {
    // The launcher engine: the daemon-internal aggregate root that owns every
    // domain subsystem — Hestia's equivalent of Tailscale's LocalBackend. The
    // daemon constructs exactly one and threads it through request handlers;
    // frontends never link it, reaching it only over IPC.
    //
    // Adding a domain (instances, accounts, versions, …) is mechanical: give it a
    // module class under hestia/engine/, construct it in the initializer list
    // against the resolved data directory, and expose it with a getter. Daemon
    // services then call through engine.<module>(). See docs/contributing.md.
    class Engine {
    public:
        // Resolve the data directory once ($HESTIA_HOME → persisted pointer →
        // platform default) and construct the subsystems against it. A non-empty
        // `override_home` wins over resolution (a --home flag, or tests).
        explicit Engine(const std::filesystem::path &override_home = {});

        // The resolved data directory every subsystem persists under.
        const std::filesystem::path &data_home() const { return data_home_; }

        // Persist `dir` as the data directory, re-resolve, and repoint every
        // subsystem so the change takes effect for this running daemon — not just
        // the next start. An empty `dir` reverts to the resolved default. Returns
        // the newly resolved directory.
        std::filesystem::path set_data_home(const std::string &dir);

        // Domain subsystems.
        ConfigStore &config() { return config_; }
        const ConfigStore &config() const { return config_; }

    private:
        std::filesystem::path data_home_;
        ConfigStore config_;
    };
}
