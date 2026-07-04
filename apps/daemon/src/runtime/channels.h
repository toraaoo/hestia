#pragma once

#include <exception>
#include <utility>

#include <nlohmann/json.hpp>

#include <hestia/ipc/errors.h>

#include "runtime/router.h"

namespace hestia::daemon {
    // The registrar handed to every service: binds a typed contract handler onto
    // the router. The channel name and the payload shapes come from the contract
    // in hestia::proto, so a service physically cannot register under a
    // different name — or a different wire format — than the client SDK calls.
    class Channels {
    public:
        explicit Channels(Router &router) : router_(router) {}

        // Register a handler for contract C: decode C::Params (a malformed
        // payload answers bad_request), invoke `fn(params, ctx)`, and encode the
        // returned C::Result. Handlers throw ServiceError to answer with a
        // specific error code.
        template <typename C, typename F>
        void handle(F fn) {
            router_.on(C::kChannel, [fn = std::move(fn)](const ipc::Request &req, HandlerContext &ctx) {
                typename C::Params params;
                try {
                    params = req.payload.get<typename C::Params>();
                } catch (const std::exception &e) {
                    throw ServiceError(ipc::errors::kBadRequest, e.what());
                }
                return ipc::Response::success(nlohmann::json(fn(params, ctx)));
            });
        }

    private:
        Router &router_;
    };
} // namespace hestia::daemon
