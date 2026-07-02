#include "downloads/download_manager.h"

#include <chrono>
#include <exception>
#include <utility>

#include <hestia/engine/download/downloader.h>
#include <hestia/ipc/download_codec.h>
#include <hestia/ipc/topics.h>

namespace hestia::daemon {
    DownloadManager::DownloadManager(EventSink sink) : sink_(std::move(sink)) {}

    DownloadManager::~DownloadManager() {
        std::vector<Worker> workers;
        {
            std::scoped_lock const lk(mu_);
            workers = std::move(workers_);
        }
        for (auto &worker: workers) {
            if (worker.thread.joinable()) worker.thread.join();
        }
    }

    std::string DownloadManager::start(std::string url, std::filesystem::path destination,
                                       std::optional<ipc::Checksum> checksum, std::string id) {
        if (id.empty()) id = "dl-" + std::to_string(next_id_++);
        auto done = std::make_shared<std::atomic<bool>>(false);
        std::thread thread([this, id, url = std::move(url), destination = std::move(destination),
                            checksum = std::move(checksum), done] {
            run(id, url, destination, checksum);
            done->store(true);
        });
        std::scoped_lock const lk(mu_);
        prune_finished();
        workers_.push_back(Worker{.thread = std::move(thread), .done = std::move(done)});
        return id;
    }

    void DownloadManager::run(const std::string &id, const std::string &url, const std::filesystem::path &destination,
                              const std::optional<ipc::Checksum> &checksum) const {
        using clock = std::chrono::steady_clock;
        auto last_emit = clock::time_point{};
        const auto on_progress = [&](const ipc::DownloadProgress &progress) {
            const auto now = clock::now();
            const bool final_report = progress.total > 0 && progress.downloaded >= progress.total;
            if (!final_report && now - last_emit < std::chrono::milliseconds(100)) return;
            last_emit = now;
            auto payload = ipc::to_json(progress);
            payload["id"] = id;
            sink_(ipc::Event{.topic = ipc::topics::kDownloadProgress, .payload = std::move(payload)});
        };
        try {
            engine::Downloader{}.fetch(url, destination, checksum, on_progress);
            sink_(ipc::Event{.topic = ipc::topics::kDownloadDone,
                             .payload = {{"id", id}, {"path", destination.string()}}});
        } catch (const std::exception &e) {
            sink_(ipc::Event{.topic = ipc::topics::kDownloadError, .payload = {{"id", id}, {"message", e.what()}}});
        }
    }

    // Callers hold mu_. Finished workers join instantly, so a long download
    // never blocks starting a new one.
    void DownloadManager::prune_finished() {
        for (auto it = workers_.begin(); it != workers_.end();) {
            if (it->done->load()) {
                if (it->thread.joinable()) it->thread.join();
                it = workers_.erase(it);
            } else {
                ++it;
            }
        }
    }
} // namespace hestia::daemon
