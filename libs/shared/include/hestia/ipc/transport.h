#pragma once

#include <filesystem>
#include <functional>
#include <memory>
#include <string>
#include <string_view>

// The IpcTransport seam: moves length-prefixed message frames over a
// single-machine, per-user channel (Unix domain socket on POSIX, named pipe on
// Windows). Payload bytes are OPAQUE here — framing is the transport's only job;
// message semantics (the channel/JSON envelope) live one layer up. See
// docs/daemon-protocol.md.
namespace hestia::ipc {
    // Receives one raw request frame, returns the response frame. Invoked by the
    // Listener for each incoming request.
    using RequestHandler = std::function<std::string(std::string_view request)>;

    // Server side, owned by the daemon. One instance per endpoint.
    class Listener {
    public:
        virtual ~Listener() = default;

        // Block, accepting connections and dispatching frames to `handler`,
        // until stop() is called.
        virtual void serve(const RequestHandler &handler) = 0;

        // Unblock serve() and release the endpoint. Async-signal-safe, so it can
        // be called directly from a SIGINT/SIGTERM handler.
        virtual void stop() = 0;
    };

    // Client side, used by every frontend (wrapped by the client SDK in Phase 2).
    class Channel {
    public:
        virtual ~Channel() = default;

        // Send one request frame and block for the response frame.
        virtual std::string send(std::string_view request) = 0;
    };

    // Bind a listener to `endpoint`, failing fast if another daemon already owns
    // it (single-instance guard — a stale socket from a crashed daemon is
    // reclaimed; a live one is refused). Throws std::system_error on failure.
    std::unique_ptr<Listener> bind_listener(const std::filesystem::path &endpoint);

    // Connect to a daemon listening on `endpoint`. Throws std::system_error if no
    // daemon is reachable.
    std::unique_ptr<Channel> connect(const std::filesystem::path &endpoint);
}
