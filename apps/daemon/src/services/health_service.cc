#include "services/health_service.h"

#include "runtime/channels.h"

#include <hestia/proto/health.h>

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

    void HealthService::register_channels(Channels &on) {
        on.handle<proto::Ping>([](const proto::Empty &, HandlerContext &) {
            return proto::Ping::Result{.status = "alive", .pid = current_pid()};
        });
    }
} // namespace hestia::daemon
