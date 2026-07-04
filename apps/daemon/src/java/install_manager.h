#pragma once

#include <atomic>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <thread>
#include <vector>

#include <hestia/engine/engine.h>
#include <hestia/ipc/protocol.h>

// Runs Java installs on background worker threads so `java.install` answers
// immediately; progress and the terminal outcome are published as
// java.install.* events through the sink (the DownloadManager pattern).
namespace hestia::daemon {
    class JavaInstallManager {
    public:
        using EventSink = std::function<void(const ipc::Event &)>;

        JavaInstallManager(engine::Engine &engine, EventSink sink);
        ~JavaInstallManager();

        // The outcome arrives as a java.install.done or java.install.error event
        // carrying the returned id; nullopt when `major` is already installing.
        std::optional<std::string> start(int major, std::string id, bool force);

    private:
        struct Worker {
            std::thread thread;
            std::shared_ptr<std::atomic<bool>> done;
        };

        void run(const std::string &id, int major, bool force);
        void prune_finished();

        engine::Engine &engine_;
        EventSink sink_;
        std::mutex mu_;
        std::vector<Worker> workers_;
        std::set<int> active_majors_;
        std::atomic<int> next_id_{1};
    };
} // namespace hestia::daemon
