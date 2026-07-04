#pragma once

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct AutostartState {
        bool enabled = false;

        static constexpr auto kFields = fields(field("enabled", &AutostartState::enabled));
    };

    struct AutostartEnable {
        static constexpr const char *kChannel = "autostart.enable";
        using Params = Empty;
        using Result = AutostartState;
    };

    struct AutostartDisable {
        static constexpr const char *kChannel = "autostart.disable";
        using Params = Empty;
        using Result = AutostartState;
    };

    struct AutostartStatus {
        static constexpr const char *kChannel = "autostart.status";
        using Params = Empty;
        using Result = AutostartState;
    };
} // namespace hestia::proto
