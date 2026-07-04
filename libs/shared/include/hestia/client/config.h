#pragma once

#include <map>
#include <optional>
#include <string>
#include <string_view>

#include <hestia/client/facade.h>
#include <hestia/proto/config.h>

namespace hestia::client {
    class Config : public Facade {
    public:
        using Facade::Facade;

        // nullopt when the key is not set.
        std::optional<std::string> get(std::string_view key);
        std::map<std::string, std::string> list();
        void set(std::string_view key, std::string_view value);
    };
} // namespace hestia::client
