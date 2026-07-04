#include "downloads/download_manager.h"

#include <chrono>
#include <exception>
#include <utility>

#include <spdlog/spdlog.h>

#include <hestia/engine/downloader.h>

namespace hestia::daemon {
    DownloadManager::DownloadManager(engine::Engine &engine, EventSink sink)
        : engine_(engine), sink_(std::move(sink)) {}

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
                                       std::optional<proto::Checksum> checksum, std::string id) {
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
                              const std::optional<proto::Checksum> &checksum) const {
        using clock = std::chrono::steady_clock;
        auto last_emit = clock::time_point{};
        const auto on_progress = [&](const proto::DownloadProgress &progress) {
            const auto now = clock::now();
            const bool final_report = progress.total > 0 && progress.downloaded >= progress.total;
            if (!final_report && now - last_emit < std::chrono::milliseconds(100)) return;
            last_emit = now;
            sink_(proto::make_event(proto::DownloadProgressEvent{.id = id, .progress = progress}));
        };
        spdlog::info("download {} started: {}", id, url);
        try {
            engine::Downloader{&engine_.cache()}.fetch(url, destination, checksum, on_progress);
            spdlog::info("download {} done: {}", id, destination.string());
            sink_(proto::make_event(proto::DownloadDoneEvent{.id = id, .path = destination}));
        } catch (const std::exception &e) {
            spdlog::warn("download {} failed: {}", id, e.what());
            sink_(proto::make_event(proto::DownloadErrorEvent{.id = id, .message = e.what()}));
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
