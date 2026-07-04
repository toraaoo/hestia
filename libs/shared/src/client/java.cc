#include "hestia/client/java.h"

#include "session.h"

namespace hestia::client {
    std::vector<proto::JavaRelease> Java::releases() {
        return session_->call<proto::JavaReleases>().releases;
    }

    std::vector<proto::JavaRuntime> Java::list() {
        return session_->call<proto::JavaList>().runtimes;
    }

    proto::JavaRuntime Java::install(int major, const JavaInstallProgressCallback &on_progress) {
        const auto id = job_id("java");
        const auto done = session_->run_job(
            id, proto::JavaInstallDoneEvent::kTopic, proto::JavaInstallErrorEvent::kTopic,
            [&on_progress](const ipc::Event &event) {
                if (event.topic != proto::JavaInstallProgressEvent::kTopic || !on_progress) return;
                on_progress(event.payload.get<proto::JavaInstallProgressEvent>().progress);
            },
            [&] { session_->call<proto::JavaInstall>({.major = major, .id = id}); });
        return done.get<proto::JavaInstallDoneEvent>().runtime;
    }

    void Java::uninstall(int major) {
        session_->call<proto::JavaUninstall>({.major = major});
    }
} // namespace hestia::client
