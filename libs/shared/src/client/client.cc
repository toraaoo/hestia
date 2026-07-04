#include "hestia/client.h"

#include <stdexcept>
#include <string>
#include <utility>

#include <hestia/ipc/endpoint.h>
#include <hestia/proto/health.h>

#include "session.h"
#include "spawn.h"

namespace hestia::client {
    Client::Client(std::shared_ptr<Session> session)
        : session_(std::move(session)), app_(*session_), daemon_(*session_), config_(*session_), process_(*session_),
          download_(*session_), java_(*session_), cache_(*session_), accounts_(*session_) {}

    Client::Client(Client &&) noexcept = default;
    Client &Client::operator=(Client &&) noexcept = default;
    Client::~Client() = default;

    Client Client::connect(bool auto_spawn) {
        const auto endpoint = ipc::default_endpoint();
        std::shared_ptr<ipc::Connection> conn;
        try {
            conn = ipc::connect(endpoint);
        } catch (const std::exception &) {
            if (!auto_spawn) throw std::runtime_error("hestiad is not running");
            spawn_daemon();
            conn = connect_with_retry(endpoint);
            if (!conn) throw std::runtime_error("started hestiad but it did not become reachable");
        }
        Client client(std::make_shared<Session>(std::move(conn)));
        // Single skew check at connect: a daemon speaking an incompatible major
        // fails fast and clearly rather than mis-parsing later messages.
        const auto health = client.session_->call_raw(proto::Ping::kChannel, nlohmann::json::object());
        if (!ipc::compatible(health.version)) {
            throw std::runtime_error("hestiad speaks protocol v" + std::to_string(health.version) +
                                     ", this client speaks v" + std::to_string(ipc::kProtocolVersion));
        }
        return client;
    }

    ipc::Response Client::call(const std::string &channel, const nlohmann::json &payload) {
        return session_->call_raw(channel, payload);
    }
} // namespace hestia::client
