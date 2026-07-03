#include "java/install_manager.h"

#include <chrono>
#include <exception>
#include <utility>

#include <hestia/engine/java.h>
#include <hestia/ipc/java_codec.h>
#include <hestia/ipc/topics.h>

namespace hestia::daemon {
    JavaInstallManager::JavaInstallManager(engine::Engine &engine, EventSink sink)
        : engine_(engine), sink_(std::move(sink)) {}

    JavaInstallManager::~JavaInstallManager() {
        std::vector<Worker> workers;
        {
            std::scoped_lock const lk(mu_);
            workers = std::move(workers_);
        }
        for (auto &worker: workers) {
            if (worker.thread.joinable()) worker.thread.join();
        }
    }

    std::optional<std::string> JavaInstallManager::start(int major, std::string id) {
        if (id.empty()) id = "java-" + std::to_string(next_id_++);
        auto done = std::make_shared<std::atomic<bool>>(false);
        std::scoped_lock const lk(mu_);
        if (!active_majors_.insert(major).second) return std::nullopt;
        prune_finished();
        std::thread thread([this, id, major, done] {
            run(id, major);
            done->store(true);
        });
        workers_.push_back(Worker{.thread = std::move(thread), .done = std::move(done)});
        return id;
    }

    void JavaInstallManager::run(const std::string &id, int major) {
        using clock = std::chrono::steady_clock;
        auto last_emit = clock::time_point{};
        auto last_phase = std::optional<ipc::JavaInstallPhase>{};
        const auto on_progress = [&](const ipc::JavaInstallProgress &progress) {
            const auto now = clock::now();
            const bool phase_change = last_phase != progress.phase;
            const bool final_report = progress.total > 0 && progress.current >= progress.total;
            if (!phase_change && !final_report && now - last_emit < std::chrono::milliseconds(100)) return;
            last_emit = now;
            last_phase = progress.phase;
            auto payload = ipc::to_json(progress);
            payload["id"] = id;
            sink_(ipc::Event{.topic = ipc::topics::kJavaInstallProgress, .payload = std::move(payload)});
        };
        try {
            const auto runtime = engine_.java().install(major, on_progress);
            sink_(ipc::Event{.topic = ipc::topics::kJavaInstallDone,
                             .payload = {{"id", id}, {"runtime", ipc::to_json(runtime)}}});
        } catch (const std::exception &e) {
            sink_(ipc::Event{.topic = ipc::topics::kJavaInstallError, .payload = {{"id", id}, {"message", e.what()}}});
        }
        std::scoped_lock const lk(mu_);
        active_majors_.erase(major);
    }

    // Callers hold mu_. Finished workers join instantly, so a long install
    // never blocks starting a new one.
    void JavaInstallManager::prune_finished() {
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
