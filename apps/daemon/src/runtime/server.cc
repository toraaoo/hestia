#include "runtime/server.h"

#include "runtime/handler_context.h"
#include "runtime/router.h"
#include "runtime/runtime.h"
#include "services/services.h"

#include <hestia/ipc/endpoint.h>
#include <hestia/ipc/errors.h>
#include <hestia/ipc/protocol.h>
#include <hestia/ipc/transport.h>

#include <spdlog/spdlog.h>

#include <atomic>
#include <csignal>
#include <exception>
#include <memory>

namespace hestia::daemon {
    namespace {
        // The serving listener, so the signal handler can unblock serve(). Only
        // stop() (async-signal-safe) is ever called from the handler.
        std::atomic<ipc::Listener *> g_listener{nullptr};

        void handle_signal(int) {
            if (auto *l = g_listener.load()) l->stop();
        }

        // Serve one client connection: loop reading request frames, dispatch each
        // through the router with a per-request context, and write the correlated
        // response. The context carries the connection, so streaming channels
        // (events.subscribe) are ordinary handlers.
        void serve_connection(const std::shared_ptr<ipc::Connection> &conn,
                              const ipc::Peer &peer, const Router &router,
                              Runtime &runtime) {
            while (auto frame = conn->recv()) {
                ipc::Request req;
                try {
                    req = ipc::decode_request(*frame);
                } catch (const std::exception &e) {
                    spdlog::warn("dropping malformed frame: {}", e.what());
                    conn->send(ipc::encode(
                        ipc::Response::failure(ipc::errors::kBadRequest, e.what())));
                    continue;
                }
                HandlerContext ctx{runtime, conn, peer};
                auto res = router.route(req, ctx);
                res.id = req.id;
                conn->send(ipc::encode(res));
            }
            runtime.hub().unsubscribe(conn.get());
        }
    }

    int run_daemon() {
        const auto endpoint = ipc::default_endpoint();
        std::unique_ptr<ipc::Listener> listener;
        try {
            listener = ipc::bind_listener(endpoint);
        } catch (const std::exception &e) {
            spdlog::error("cannot start: {}", e.what());
            return 1;
        }

        Runtime runtime;

        Router router;
        register_all_services(router);

        g_listener.store(listener.get());
        std::signal(SIGINT, handle_signal);
        std::signal(SIGTERM, handle_signal);
#if !defined(_WIN32)
        std::signal(SIGPIPE, SIG_IGN); // a client vanishing mid-write must not kill us
#endif

        spdlog::info("hestiad listening on {}", endpoint.string());
        listener->serve([&](std::shared_ptr<ipc::Connection> conn,
                            const ipc::Peer &peer) {
            spdlog::debug("client connected (uid {})", peer.uid);
            serve_connection(conn, peer, router, runtime);
            spdlog::debug("client disconnected");
        });
        g_listener.store(nullptr);
        spdlog::info("hestiad stopped");
        return 0;
    }
}
