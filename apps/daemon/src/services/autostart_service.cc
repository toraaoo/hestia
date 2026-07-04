#include "services/autostart_service.h"

#include "platform/autostart.h"
#include "runtime/channels.h"

#include <hestia/proto/autostart.h>

namespace hestia::daemon {
    void AutostartService::register_channels(Channels &on) {
        // Autostart is constructed per call so an unsupported platform fails the
        // one request rather than the whole daemon, and so the registration always
        // resolves the daemon's current executable path.
        on.handle<proto::AutostartEnable>([](const proto::Empty &, HandlerContext &) {
            make_autostart()->enable();
            return proto::AutostartState{.enabled = true};
        });

        on.handle<proto::AutostartDisable>([](const proto::Empty &, HandlerContext &) {
            make_autostart()->disable();
            return proto::AutostartState{.enabled = false};
        });

        on.handle<proto::AutostartStatus>([](const proto::Empty &, HandlerContext &) {
            return proto::AutostartState{.enabled = make_autostart()->is_enabled()};
        });
    }
} // namespace hestia::daemon
