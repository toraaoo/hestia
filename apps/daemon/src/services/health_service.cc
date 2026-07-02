#include "services/services.h"

#include "runtime/handler_context.h"
#include "runtime/router.h"

#if !defined(_WIN32)
#include <unistd.h>
#else
#include <windows.h>
#endif

namespace hestia::daemon {
    namespace {
        int current_pid() {
#if !defined(_WIN32)
            return static_cast<int>(::getpid());
#else
            return static_cast<int>(::GetCurrentProcessId());
#endif
        }
    } // namespace

    void register_health_service(Router &router) {
        router.on("health.ping", [](const ipc::Request &, HandlerContext &) {
            return ipc::Response::success({{"status", "alive"}, {"pid", current_pid()}});
        });
    }
} // namespace hestia::daemon
