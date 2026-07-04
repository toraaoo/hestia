#pragma once

#include <functional>
#include <map>
#include <stdexcept>
#include <string>

#include <hestia/ipc/protocol.h>

#include "runtime/handler_context.h"

namespace hestia::daemon {
    // Thrown by a handler to answer with a specific protocol error code
    // (ipc::errors::*); any other exception escaping a handler becomes a
    // handler_error response.
    class ServiceError : public std::runtime_error {
    public:
        ServiceError(std::string code, const std::string &message)
            : std::runtime_error(message), code_(std::move(code)) {}

        [[nodiscard]] const std::string &code() const { return code_; }

    private:
        std::string code_;
    };

    // Maps a channel name to a handler and routes a decoded request to it. An
    // unknown channel or a handler exception becomes a protocol-level error
    // response, so the caller always gets a well-formed Response back.
    class Router {
    public:
        using Handler = std::function<ipc::Response(const ipc::Request &, HandlerContext &)>;

        void on(std::string channel, Handler handler);

        ipc::Response route(const ipc::Request &request, HandlerContext &ctx) const;

    private:
        std::map<std::string, Handler> handlers_;
    };
} // namespace hestia::daemon
