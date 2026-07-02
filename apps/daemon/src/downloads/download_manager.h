#pragma once

#include <atomic>
#include <filesystem>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

#include <hestia/ipc/download.h>
#include <hestia/ipc/protocol.h>

// Runs downloads on background worker threads — one per active download — so
// `download.start` answers immediately; progress and the terminal outcome are
// published as download.* events through the sink (the ProcessSupervisor
// pattern). Progress events are throttled so the hub's bounded queue isn't
// flooded.
namespace hestia::daemon {
    class DownloadManager {
    public:
        using EventSink = std::function<void(const ipc::Event &)>;

        explicit DownloadManager(EventSink sink);
        ~DownloadManager();

        // Start a download and return its id — the caller-supplied `id` when
        // non-empty, else a generated one. The outcome arrives as a
        // download.done or download.error event carrying that id.
        std::string start(std::string url, std::filesystem::path destination, std::optional<ipc::Checksum> checksum,
                          std::string id);

    private:
        struct Worker {
            std::thread thread;
            std::shared_ptr<std::atomic<bool>> done;
        };

        void run(const std::string &id, const std::string &url, const std::filesystem::path &destination,
                 const std::optional<ipc::Checksum> &checksum) const;
        void prune_finished();

        EventSink sink_;
        std::mutex mu_;
        std::vector<Worker> workers_;
        std::atomic<int> next_id_{1};
    };
} // namespace hestia::daemon
