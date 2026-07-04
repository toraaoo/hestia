#pragma once

#include <string>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct AppInfo {
        static constexpr const char *kChannel = "app.info";
        using Params = Empty;
        struct Result {
            std::string name;
            std::string version;
            std::string id;
            std::string vendor;
            std::string channel;

            static constexpr auto kFields =
                fields(field("name", &Result::name), field("version", &Result::version), field("id", &Result::id),
                       field("vendor", &Result::vendor), field("channel", &Result::channel));
        };
    };
} // namespace hestia::proto
