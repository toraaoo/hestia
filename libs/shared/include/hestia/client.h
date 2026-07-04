#pragma once

#include <memory>
#include <string>

#include <nlohmann/json.hpp>

#include <hestia/client/app.h>
#include <hestia/client/autostart.h>
#include <hestia/client/cache.h>
#include <hestia/client/config.h>
#include <hestia/client/daemon.h>
#include <hestia/client/download.h>
#include <hestia/client/java.h>
#include <hestia/client/process.h>
#include <hestia/ipc/protocol.h>

// The thin client SDK every frontend (CLI/TUI/desktop/tray) uses to drive the
// daemon — the single boundary they code against. One persistent, multiplexed
// connection; each domain is a facade reached through an accessor
// (client.java().install(21)), mirroring the engine's engine.java() on the
// daemon side of the socket.
namespace hestia::client {
    class Client {
    public:
        // Connect to the running daemon. If none is running and `auto_spawn` is
        // true, start one and wait for it to come up. Throws std::runtime_error
        // if the daemon is unreachable (and could not be spawned).
        static Client connect(bool auto_spawn = true);

        Client(Client &&) noexcept;
        Client &operator=(Client &&) noexcept;
        ~Client();

        // Raw request; throws only on transport failure (a daemon-side error is a
        // Response with ok == false). The facades below are built on this.
        ipc::Response call(const std::string &channel, const nlohmann::json &payload);

        App &app() { return app_; }
        Daemon &daemon() { return daemon_; }
        Config &config() { return config_; }
        Autostart &autostart() { return autostart_; }
        Process &process() { return process_; }
        Download &download() { return download_; }
        Java &java() { return java_; }
        Cache &cache() { return cache_; }

    private:
        explicit Client(std::shared_ptr<Session> session);

        std::shared_ptr<Session> session_;
        App app_;
        Daemon daemon_;
        Config config_;
        Autostart autostart_;
        Process process_;
        Download download_;
        Java java_;
        Cache cache_;
    };
} // namespace hestia::client
