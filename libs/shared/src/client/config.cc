#include "hestia/client/config.h"

#include <string>
#include <utility>

#include "session.h"

namespace hestia::client {
    std::optional<nlohmann::json> Config::get(std::string_view key) {
        const auto res = session_->try_call<proto::ConfigGet>({.key = std::string(key)});
        if (!res) return std::nullopt;
        return res->value;
    }

    nlohmann::json Config::list() {
        return session_->call<proto::ConfigList>().entries;
    }

    void Config::set(std::string_view key, nlohmann::json value) {
        session_->call<proto::ConfigSet>({.key = std::string(key), .value = std::move(value)});
    }
} // namespace hestia::client
