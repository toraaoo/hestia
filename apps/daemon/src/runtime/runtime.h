#pragma once

#include <filesystem>
#include <memory>

#include <hestia/engine/engine.h>
#include <hestia/ipc/protocol.h>

#include "downloads/download_manager.h"
#include "process/process_supervisor.h"
#include "runtime/event_hub.h"

// The daemon's long-lived collaborators in one place — the anti-churn seam a new
// subsystem hangs off, mirroring how hestia::engine::Engine aggregates the domain
// modules. Adding one is a member here plus an accessor, with no change to the
// serve loop or the handler context.
namespace hestia::daemon {
    class Runtime {
    public:
        // Construction order is load-bearing. The event hub is constructed before
        // anything that publishes into it (the download manager, the supervisor).
        // Because members are destroyed in reverse declaration order, the
        // supervisor and download manager — declared after the hub — are torn down
        // first, so their worker threads never publish into a dead hub during
        // shutdown.
        explicit Runtime(const std::filesystem::path &override_home = {})
            : engine_(override_home),
              downloads_([this](const ipc::Event &e) { hub_.publish(e); }),
              supervisor_(make_process_supervisor(engine_.data_home())) {
            supervisor_->set_event_sink([this](const ipc::Event &e) { hub_.publish(e); });
            supervisor_->reconcile();         // re-adopt processes that survived a previous daemon
            supervisor_->start_supervision(); // poll liveness, stream logs, enforce restarts
        }

        engine::Engine &engine() { return engine_; }
        EventHub &hub() { return hub_; }
        DownloadManager &downloads() { return downloads_; }
        ProcessSupervisor &supervisor() { return *supervisor_; }

    private:
        engine::Engine engine_;
        EventHub hub_;
        DownloadManager downloads_;
        std::unique_ptr<ProcessSupervisor> supervisor_;
    };
}
