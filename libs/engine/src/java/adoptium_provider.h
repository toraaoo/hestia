#pragma once

#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#include <hestia/engine/java.h>

namespace hestia::engine {
    // Eclipse Temurin builds from the Adoptium API (https://api.adoptium.net).
    class AdoptiumProvider : public JavaProvider {
    public:
        [[nodiscard]] std::string vendor() const override { return "temurin"; }
        [[nodiscard]] std::vector<proto::JavaRelease> releases() const override;
        [[nodiscard]] JavaPackage resolve(int major, const JavaTarget &target) const override;
    };

    // Separate from the HTTP fetch so they are unit-testable.
    std::vector<proto::JavaRelease> adoptium_releases_from_json(const nlohmann::json &j);
    JavaPackage adoptium_package_from_json(const nlohmann::json &assets, int major, const JavaTarget &target);
} // namespace hestia::engine
