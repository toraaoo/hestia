#pragma once

#include <string>

#include <hestia/proto/contract.h>

namespace hestia::proto {
    struct EventsSubscribe {
        static constexpr const char *kChannel = "events.subscribe";
        struct Params {
            std::string id; // empty subscribes to every event

            static constexpr auto kFields = fields(field("id", &Params::id, kOmitIfEmpty));
        };
        struct Result {
            bool subscribed = false;

            static constexpr auto kFields = fields(field("subscribed", &Result::subscribed));
        };
    };
} // namespace hestia::proto
