#include "services/services.h"

#include "runtime/handler_context.h"
#include "runtime/router.h"
#include "runtime/runtime.h"

#include <string>

#include <hestia/engine/engine.h>
#include <hestia/ipc/errors.h>
#include <hestia/ipc/java_codec.h>

namespace hestia::daemon {
    void register_java_service(Router &router) {
        router.on("java.releases", [](const ipc::Request &, HandlerContext &ctx) {
            auto releases = nlohmann::json::array();
            for (const auto &release: ctx.runtime.engine().java().releases()) {
                releases.push_back(ipc::to_json(release));
            }
            return ipc::Response::success({{"releases", std::move(releases)}});
        });

        router.on("java.list", [](const ipc::Request &, HandlerContext &ctx) {
            auto runtimes = nlohmann::json::array();
            for (const auto &runtime: ctx.runtime.engine().java().installed()) {
                runtimes.push_back(ipc::to_json(runtime));
            }
            return ipc::Response::success({{"runtimes", std::move(runtimes)}});
        });

        router.on("java.install", [](const ipc::Request &req, HandlerContext &ctx) {
            const int major = req.payload.value("major", 0);
            if (major <= 0) {
                return ipc::Response::failure(ipc::errors::kBadRequest, "major must be a positive integer");
            }
            const auto id = ctx.runtime.java_installs().start(major, req.payload.value("id", std::string{}));
            if (!id) {
                return ipc::Response::failure(ipc::errors::kBadRequest,
                                              "java " + std::to_string(major) + " is already being installed");
            }
            return ipc::Response::success({{"id", *id}});
        });

        router.on("java.uninstall", [](const ipc::Request &req, HandlerContext &ctx) {
            const int major = req.payload.value("major", 0);
            if (major <= 0) {
                return ipc::Response::failure(ipc::errors::kBadRequest, "major must be a positive integer");
            }
            if (!ctx.runtime.engine().java().uninstall(major)) {
                return ipc::Response::failure(ipc::errors::kNotFound,
                                              "no installed java runtime for major " + std::to_string(major));
            }
            return ipc::Response::success();
        });
    }
} // namespace hestia::daemon
