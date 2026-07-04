#include "runtime/router.h"

#include <chrono>

#include <spdlog/spdlog.h>

#include <hestia/ipc/errors.h>

namespace hestia::daemon {
    void Router::on(std::string channel, Handler handler) {
        handlers_[std::move(channel)] = std::move(handler);
    }

    ipc::Response Router::route(const ipc::Request &request, HandlerContext &ctx) const {
        const auto it = handlers_.find(request.channel);
        if (it == handlers_.end()) {
            spdlog::warn("no handler for channel '{}'", request.channel);
            return ipc::Response::failure(ipc::errors::kUnknownChannel, "unknown channel: " + request.channel);
        }
        spdlog::debug("dispatch {}", request.channel);
        const auto started = std::chrono::steady_clock::now();
        const auto elapsed_ms = [&] {
            return std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - started)
                .count();
        };
        try {
            auto response = it->second(request, ctx);
            spdlog::debug("{} -> {} ({} ms)", request.channel, response.ok ? "ok" : "error", elapsed_ms());
            return response;
        } catch (const ServiceError &e) {
            spdlog::warn("{} failed [{}]: {} ({} ms)", request.channel, e.code(), e.what(), elapsed_ms());
            return ipc::Response::failure(e.code(), e.what());
        } catch (const std::exception &e) {
            // A handler that throws (e.g. a missing payload field) becomes a clean
            // error rather than taking down the daemon.
            spdlog::error("{} handler threw: {} ({} ms)", request.channel, e.what(), elapsed_ms());
            return ipc::Response::failure(ipc::errors::kHandlerError, e.what());
        }
    }
} // namespace hestia::daemon
