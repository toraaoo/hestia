#pragma once

#include <atomic>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <thread>

#include <hestia/engine/engine.h>
#include <hestia/ipc/protocol.h>

// Runs the blocking Microsoft device-code login on a worker thread so
// `account.login` answers immediately; the code prompt and the terminal outcome
// are published as account.login.* events through the sink (the
// JavaInstallManager pattern). One login at a time: the flow is interactive.
namespace hestia::daemon {
    class LoginManager {
    public:
        using EventSink = std::function<void(const ipc::Event &)>;

        LoginManager(engine::Engine &engine, EventSink sink);
        ~LoginManager();

        // The outcome arrives as an account.login.done or account.login.error
        // event carrying the returned id; nullopt when a login is already
        // running.
        std::optional<std::string> start(std::string id);

    private:
        void run(const std::string &id);

        engine::Engine &engine_;
        EventSink sink_;
        std::mutex mu_;
        std::thread worker_;
        bool active_ = false;
        std::atomic<bool> cancel_{false};
        std::atomic<int> next_id_{1};
    };
} // namespace hestia::daemon
