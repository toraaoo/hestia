#pragma once

#include <filesystem>
#include <string>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct DaemonStatus {
        static constexpr const char *kChannel = "daemon.status";
        using Params = Empty;
        struct Result {
            long long pid = 0;
            std::string version;
            long long uptime_seconds = 0;
            std::filesystem::path home;
            std::filesystem::path log;

            static constexpr auto kFields =
                fields(field("pid", &Result::pid), field("version", &Result::version),
                       field("uptime_seconds", &Result::uptime_seconds), field("home", &Result::home),
                       field("log", &Result::log));
        };
    };

    struct DaemonStop {
        static constexpr const char *kChannel = "daemon.stop";
        using Params = Empty;
        struct Result {
            bool stopping = false;

            static constexpr auto kFields = fields(field("stopping", &Result::stopping));
        };
    };
} // namespace hestia::proto
