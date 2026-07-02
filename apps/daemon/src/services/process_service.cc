#include "services/services.h"

#include "runtime/handler_context.h"
#include "process/process_supervisor.h"
#include "runtime/router.h"
#include "runtime/runtime.h"

#include <hestia/ipc/errors.h>
#include <hestia/ipc/process_codec.h>

#include <string>

namespace hestia::daemon {
    void register_process_service(Router &router) {
        router.on("process.start", [](const ipc::Request &req, HandlerContext &ctx) {
            const auto record = ctx.runtime.supervisor().start(ipc::launch_spec_from_json(req.payload));
            return ipc::Response::success(ipc::to_json(record));
        });

        router.on("process.stop", [](const ipc::Request &req, HandlerContext &ctx) {
            ctx.runtime.supervisor().stop(req.payload.at("id").get<std::string>());
            return ipc::Response::success();
        });

        router.on("process.list", [](const ipc::Request &, HandlerContext &ctx) {
            nlohmann::json processes = nlohmann::json::array();
            for (const auto &record: ctx.runtime.supervisor().list()) processes.push_back(ipc::to_json(record));
            return ipc::Response::success({{"processes", processes}});
        });

        router.on("process.status", [](const ipc::Request &req, HandlerContext &ctx) {
            const auto id = req.payload.at("id").get<std::string>();
            if (const auto record = ctx.runtime.supervisor().status(id)) {
                return ipc::Response::success(ipc::to_json(*record));
            }
            return ipc::Response::failure(ipc::errors::kNotFound, "no such process: " + id);
        });

        router.on("process.logs", [](const ipc::Request &req, HandlerContext &ctx) {
            const auto id = req.payload.at("id").get<std::string>();
            const int lines = req.payload.value("lines", 200);
            if (const auto text = ctx.runtime.supervisor().tail_log(id, lines)) {
                return ipc::Response::success({{"text", *text}});
            }
            return ipc::Response::failure(ipc::errors::kNotFound, "no such process: " + id);
        });
    }
}
