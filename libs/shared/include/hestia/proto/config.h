#pragma once

#include <filesystem>
#include <map>
#include <string>
#include <tuple>
#include <type_traits>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct Config {
        std::filesystem::path home;
        bool autostart = false;

        static constexpr auto kFields =
            fields(field("home", &Config::home), field("autostart", &Config::autostart));
    };

    template <auto Member>
    constexpr const char *config_key() {
        const char *key = nullptr;
        std::apply(
            [&](auto... f) {
                (
                    [&] {
                        if constexpr (std::is_same_v<decltype(f.member), decltype(Member)>) {
                            if (f.member == Member) key = f.key;
                        }
                    }(),
                    ...);
            },
            Config::kFields);
        return key;
    }

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

    struct ConfigList {
        static constexpr const char *kChannel = "config.list";
        using Params = Empty;
        struct Result {
            std::map<std::string, std::string> entries;

            static constexpr auto kFields = fields(field("entries", &Result::entries));
        };
    };
} // namespace hestia::proto
