#pragma once

#include <optional>
#include <string>
#include <string_view>

#include <nlohmann/json.hpp>

// The daemon protocol envelope, layered on top of the raw frame transport. Both
// sides — the daemon's router and the client SDK — encode/decode through here, so
// the wire format lives in exactly one place. See docs/daemon-protocol.md.
namespace hestia::ipc {
    // A request: a channel name, a JSON payload, and an optional correlation id.
    struct Request {
        std::string channel;
        nlohmann::json payload = nlohmann::json::object();
        std::optional<long long> id;
    };

    struct Error {
        std::string code;
        std::string message;
    };

    // A response: success carries a payload; failure carries an error. The id
    // echoes the request's id when present.
    struct Response {
        bool ok = false;
        nlohmann::json payload = nlohmann::json::object();
        std::optional<Error> error;
        std::optional<long long> id;

        static Response success(nlohmann::json payload = nlohmann::json::object());
        static Response failure(std::string code, std::string message);
    };

    // Encode to / decode from a frame's bytes. The decoders throw std::exception
    // (nlohmann parse_error or out-of-range) on a malformed frame; callers map
    // that to a protocol-level error.
    std::string encode(const Request &request);
    std::string encode(const Response &response);
    Request decode_request(std::string_view frame);
    Response decode_response(std::string_view frame);
}
