#include "hestia/client/bridge.h"

#include <exception>
#include <mutex>
#include <optional>
#include <string>

#include <nlohmann/json.hpp>

#include "hestia/client/client.h"
#include "hestia/ipc/protocol.h"

namespace hestia::client {
    namespace {
        Client &shared_client() {
            static std::mutex mu;
            static std::optional<Client> client;
            std::scoped_lock const lock(mu);
            if (!client) client = Client::connect();
            return *client;
        }
    } // namespace

    BridgeReply call_daemon(std::string_view channel, std::string_view payload_json) noexcept {
        try {
            nlohmann::json payload = nlohmann::json::object();
            if (!payload_json.empty()) {
                auto parsed = nlohmann::json::parse(payload_json);
                if (!parsed.is_null()) payload = std::move(parsed);
            }
            const ipc::Response res = shared_client().call(std::string(channel), payload);
            if (res.ok) return {.ok = true, .json = res.payload.dump(), .error = {}};
            return {.ok = false, .json = {}, .error = res.error ? res.error->message : "daemon error"};
        } catch (const std::exception &e) {
            return {.ok = false, .json = {}, .error = e.what()};
        } catch (...) {
            return {.ok = false, .json = {}, .error = "unknown error"};
        }
    }
} // namespace hestia::client
