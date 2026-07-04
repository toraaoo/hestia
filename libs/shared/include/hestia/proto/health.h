#pragma once

#include <string>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct Ping {
        static constexpr const char *kChannel = "health.ping";
        using Params = Empty;
        struct Result {
            std::string status;
            int pid = 0;

            static constexpr auto kFields = fields(field("status", &Result::status), field("pid", &Result::pid));
        };
    };
} // namespace hestia::proto
