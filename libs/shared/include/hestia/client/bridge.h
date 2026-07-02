#pragma once

#include <string>
#include <string_view>

// Noexcept boundary over the client SDK for exception-disabled callers (the CEF
// desktop is built -fno-exceptions; a throw unwinding into it would terminate).
namespace hestia::client {
    struct BridgeReply {
        bool ok = false;
        std::string json;  // the daemon's response payload as JSON, when ok
        std::string error; // a human-readable message, when !ok
    };

    BridgeReply call_daemon(std::string_view channel, std::string_view payload_json) noexcept;
} // namespace hestia::client
