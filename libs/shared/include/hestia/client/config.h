#pragma once

#include <optional>
#include <string_view>

#include <nlohmann/json.hpp>

#include <hestia/client/facade.h>
#include <hestia/proto/config.h>

namespace hestia::client {
    class Config : public Facade {
    public:
        using Facade::Facade;

        // nullopt when the key is unknown.
        std::optional<nlohmann::json> get(std::string_view key);
        nlohmann::json list();
        void set(std::string_view key, nlohmann::json value);
    };
} // namespace hestia::client
