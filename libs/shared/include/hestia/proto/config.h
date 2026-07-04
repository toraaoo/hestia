#pragma once

#include <string>

#include <nlohmann/json.hpp>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    // Reserved keys the daemon routes to their own subsystem instead of the
    // settings store: the data-directory pointer and the login registration.
    inline constexpr const char *kHomeKey = "home";
    inline constexpr const char *kAutostartKey = "autostart";

    struct ConfigGet {
        static constexpr const char *kChannel = "config.get";
        struct Params {
            std::string key;

            static constexpr auto kFields = fields(field("key", &Params::key, kRequired));
        };
        struct Result {
            nlohmann::json value;

            static constexpr auto kFields = fields(field("value", &Result::value));
        };
    };

    struct ConfigSet {
        static constexpr const char *kChannel = "config.set";
        struct Params {
            std::string key;
            nlohmann::json value;

            static constexpr auto kFields =
                fields(field("key", &Params::key, kRequired), field("value", &Params::value, kRequired));
        };
        using Result = Empty;
    };

    struct ConfigList {
        static constexpr const char *kChannel = "config.list";
        using Params = Empty;
        struct Result {
            nlohmann::json entries;

            static constexpr auto kFields = fields(field("entries", &Result::entries));
        };
    };
} // namespace hestia::proto
