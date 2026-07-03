#include "services/services.h"

#include "runtime/handler_context.h"
#include "runtime/router.h"

#include <hestia/app_info.h>

namespace hestia::daemon {
    void register_app_service(Router &router) {
        router.on("app.info", [](const ipc::Request &, HandlerContext &) {
            return ipc::Response::success({
                {"name", APP_NAME},
                {"version", APP_VERSION},
                {"id", APP_ID},
                {"vendor", APP_VENDOR},
                {"channel", APP_CHANNEL},
            });
        });
    }
} // namespace hestia::daemon
