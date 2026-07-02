#include "services/services.h"

#include "runtime/handler_context.h"
#include "runtime/router.h"
#include "runtime/runtime.h"

#include <hestia/engine/engine.h>
#include <hestia/ipc/errors.h>

#include <string>

namespace hestia::daemon {
    void register_config_service(Router &router) {
        router.on("config.get", [](const ipc::Request &req, HandlerContext &ctx) {
            const auto key = req.payload.at("key").get<std::string>();
            if (const auto value = ctx.runtime.engine().config().get(key)) {
                return ipc::Response::success({{"value", *value}});
            }
            return ipc::Response::failure(ipc::errors::kNotFound, "key not found: " + key);
        });

        router.on("config.set", [](const ipc::Request &req, HandlerContext &ctx) {
            ctx.runtime.engine().config().set(req.payload.at("key").get<std::string>(),
                                    req.payload.at("value").get<std::string>());
            return ipc::Response::success();
        });

        router.on("config.home", [](const ipc::Request &, HandlerContext &ctx) {
            return ipc::Response::success({{"path", ctx.runtime.engine().data_home().string()}});
        });

        router.on("config.set-home", [](const ipc::Request &req, HandlerContext &ctx) {
            const auto dir = req.payload.value("dir", std::string{});
            return ipc::Response::success({{"path", ctx.runtime.engine().set_data_home(dir).string()}});
        });
    }
}
