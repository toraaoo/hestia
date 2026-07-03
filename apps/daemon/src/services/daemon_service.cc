#include "services/services.h"

#include "runtime/handler_context.h"
#include "runtime/router.h"
#include "runtime/runtime.h"

#include <chrono>
#include <thread>

#include <hestia/app_info.h>
#include <hestia/engine/engine.h>
#include <hestia/ipc/endpoint.h>

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

    void register_daemon_service(Router &router) {
        router.on("daemon.status", [](const ipc::Request &, HandlerContext &ctx) {
            return ipc::Response::success({
                {"pid", current_pid()},
                {"version", APP_VERSION},
                {"uptime_seconds", ctx.runtime.uptime_seconds()},
                {"home", ctx.runtime.engine().data_home().string()},
                {"log", (ipc::runtime_dir() / "hestiad.log").string()},
            });
        });

        router.on("daemon.stop", [](const ipc::Request &, HandlerContext &ctx) {
            // Shut down on a short delay so this response reaches the client
            // before the serve loop closes its connection.
            std::thread([&runtime = ctx.runtime] {
                std::this_thread::sleep_for(std::chrono::milliseconds(200));
                runtime.request_stop();
            }).detach();
            return ipc::Response::success({{"stopping", true}});
        });
    }
} // namespace hestia::daemon
