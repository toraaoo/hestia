#include "services/app_service.h"

#include "runtime/channels.h"

#include <hestia/app_info.h>
#include <hestia/proto/app.h>

namespace hestia::daemon {
    void AppService::register_channels(Channels &on) {
        on.handle<proto::AppInfo>([](const proto::Empty &, HandlerContext &) {
            return proto::AppInfo::Result{
                .name = APP_NAME,
                .version = APP_VERSION,
                .id = APP_ID,
                .vendor = APP_VENDOR,
                .channel = APP_CHANNEL,
            };
        });
    }
} // namespace hestia::daemon
