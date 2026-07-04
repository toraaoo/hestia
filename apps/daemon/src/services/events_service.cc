#include "services/events_service.h"

#include "runtime/channels.h"
#include "runtime/event_hub.h"
#include "runtime/runtime.h"

#include <optional>
#include <string>

#include <hestia/proto/events.h>

namespace hestia::daemon {
    void EventsService::register_channels(Channels &on) {
        // A streaming channel: it needs the calling connection to push to, which it
        // gets from the context — so it is an ordinary handler, not a special case
        // in the serve loop. Closing the connection prunes the subscription.
        on.handle<proto::EventsSubscribe>([](const proto::EventsSubscribe::Params &p, HandlerContext &ctx) {
            std::optional<std::string> filter;
            if (!p.id.empty()) filter = p.id;
            ctx.runtime.hub().subscribe(ctx.connection, std::move(filter));
            return proto::EventsSubscribe::Result{.subscribed = true};
        });
    }
} // namespace hestia::daemon
