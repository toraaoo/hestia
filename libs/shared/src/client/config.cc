#include "hestia/client/config.h"

#include "session.h"

namespace hestia::client {
    std::optional<std::string> Config::get(std::string_view key) {
        const auto res = session_->try_call<proto::ConfigGet>({.key = std::string(key)});
        if (!res) return std::nullopt;
        return res->value;
    }

    void Config::set(std::string_view key, std::string_view value) {
        session_->call<proto::ConfigSet>({.key = std::string(key), .value = std::string(value)});
    }

    std::filesystem::path Config::home() {
        return session_->call<proto::ConfigHome>().path;
    }

    std::filesystem::path Config::set_home(std::string_view dir) {
        return session_->call<proto::ConfigSetHome>({.dir = std::string(dir)}).path;
    }
} // namespace hestia::client
