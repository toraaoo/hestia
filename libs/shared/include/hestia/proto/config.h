#pragma once

#include <filesystem>
#include <string>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct ConfigGet {
        static constexpr const char *kChannel = "config.get";
        struct Params {
            std::string key;

            static constexpr auto kFields = fields(field("key", &Params::key, kRequired));
        };
        struct Result {
            std::string value;

            static constexpr auto kFields = fields(field("value", &Result::value));
        };
    };

    struct ConfigSet {
        static constexpr const char *kChannel = "config.set";
        struct Params {
            std::string key;
            std::string value;

            static constexpr auto kFields =
                fields(field("key", &Params::key, kRequired), field("value", &Params::value, kRequired));
        };
        using Result = Empty;
    };

    struct ConfigHome {
        static constexpr const char *kChannel = "config.home";
        using Params = Empty;
        struct Result {
            std::filesystem::path path;

            static constexpr auto kFields = fields(field("path", &Result::path, kRequired));
        };
    };

    struct ConfigSetHome {
        static constexpr const char *kChannel = "config.set-home";
        struct Params {
            std::string dir; // empty reverts to the platform default

            static constexpr auto kFields = fields(field("dir", &Params::dir, kOmitIfEmpty));
        };
        using Result = ConfigHome::Result;
    };
} // namespace hestia::proto
