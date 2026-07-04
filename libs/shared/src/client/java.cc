#include "hestia/client/java.h"

#include <stdexcept>

#include "session.h"

namespace hestia::client {
    std::vector<proto::JavaRelease> Java::releases() {
        return session_->call<proto::JavaReleases>().releases;
    }

    std::vector<proto::JavaRuntime> Java::list() {
        return session_->call<proto::JavaList>().runtimes;
    }

    JavaInstallResult Java::install(int major, bool force, const JavaInstallProgressCallback &on_progress) {
        if (major <= 0) {
            const auto releases = session_->call<proto::JavaReleases>().releases;
            if (releases.empty()) {
                throw std::runtime_error("no java releases are available to install");
            }
            major = releases.back().major; // releases arrive sorted ascending
        }
        const auto id = job_id("java");
        const auto done = session_->run_job(
            id, proto::JavaInstallDoneEvent::kTopic, proto::JavaInstallErrorEvent::kTopic,
            [&on_progress](const ipc::Event &event) {
                if (event.topic != proto::JavaInstallProgressEvent::kTopic || !on_progress) return;
                on_progress(event.payload.get<proto::JavaInstallProgressEvent>().progress);
            },
            [&] { session_->call<proto::JavaInstall>({.major = major, .id = id, .force = force}); });
        const auto event = done.get<proto::JavaInstallDoneEvent>();
        return {.runtime = event.runtime, .already_installed = event.already_installed};
    }

    void Java::uninstall(int major) {
        session_->call<proto::JavaUninstall>({.major = major});
    }
} // namespace hestia::client
