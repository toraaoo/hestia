#pragma once

#include <filesystem>
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
        void set(std::string_view key, std::string_view value);
        std::filesystem::path home();
        // Persist the data directory for future runs; empty reverts to the
        // platform default. Returns the newly resolved home.
        std::filesystem::path set_home(std::string_view dir);
    };
} // namespace hestia::client
