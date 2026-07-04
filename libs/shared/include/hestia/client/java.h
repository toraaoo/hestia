#pragma once

#include <functional>
#include <vector>

#include <hestia/client/facade.h>
#include <hestia/proto/java.h>

namespace hestia::client {
    using JavaInstallProgressCallback = std::function<void(const proto::JavaInstallProgress &)>;

    // Java runtimes, managed by the daemon.
    class Java : public Facade {
    public:
        using Facade::Facade;

        std::vector<proto::JavaRelease> releases();
        std::vector<proto::JavaRuntime> list();
        // Blocks until the runtime is registered, reporting progress on the
        // reader thread; like Download::fetch(), it uses the session's single
        // event-callback slot.
        proto::JavaRuntime install(int major, const JavaInstallProgressCallback &on_progress = {});
        void uninstall(int major);
    };
} // namespace hestia::client
