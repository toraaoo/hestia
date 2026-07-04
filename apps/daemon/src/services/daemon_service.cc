#include "services/daemon_service.h"

#include "runtime/channels.h"
#include "runtime/runtime.h"

#include <chrono>
#include <thread>

#include <hestia/app_info.h>
#include <hestia/engine/engine.h>
#include <hestia/proto/daemon.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <unistd.h>
#endif

namespace hestia::daemon {
    namespace {
        long long current_pid() {
#if defined(_WIN32)
            return static_cast<long long>(::GetCurrentProcessId());
#else
            return static_cast<long long>(::getpid());
#endif
        }
    } // namespace

    void DaemonService::register_channels(Channels &on) {
        on.handle<proto::DaemonStatus>([](const proto::Empty &, HandlerContext &ctx) {
            return proto::DaemonStatus::Result{
                .pid = current_pid(),
                .version = APP_VERSION,
                .uptime_seconds = ctx.runtime.uptime_seconds(),
                .home = ctx.runtime.engine().data_home(),
                .log = ctx.runtime.log_path(),
            };
        });

        on.handle<proto::DaemonStop>([](const proto::Empty &, HandlerContext &ctx) {
            // Shut down on a short delay so this response reaches the client
            // before the serve loop closes its connection.
            std::thread([&runtime = ctx.runtime] {
                std::this_thread::sleep_for(std::chrono::milliseconds(200));
                runtime.request_stop();
            }).detach();
            return proto::DaemonStop::Result{.stopping = true};
        });
    }
} // namespace hestia::daemon
