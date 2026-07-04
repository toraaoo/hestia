#pragma once

#include <functional>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include <hestia/client/facade.h>
#include <hestia/proto/process.h>

namespace hestia::client {
    // An event pushed by the daemon to a subscriber. A "process.state" event
    // carries `record`; a "process.log" event carries `log` (a chunk of new
    // output). `id` is the process the event concerns.
    struct ProcessEvent {
        std::string topic;
        std::string id;
        std::optional<proto::ProcessRecord> record;
        std::optional<std::string> log;
    };

    using ProcessEventCallback = std::function<void(const ProcessEvent &)>;

    // Process supervision. start/stop/list/status/logs round-trip to the daemon,
    // which owns the processes so they outlive this client.
    class Process : public Facade {
    public:
        using Facade::Facade;

        proto::ProcessRecord start(const proto::LaunchSpec &spec);
        void stop(std::string_view id);
        std::vector<proto::ProcessRecord> list();
        std::optional<proto::ProcessRecord> status(std::string_view id);
        std::string logs(std::string_view id, int lines = 200);

        // Stream live process events. `cb` is invoked on the connection's reader
        // thread for each matching event; pass an `id_filter` to scope it to one
        // process, or empty for all. Uses the session's single event-callback
        // slot, so it replaces any previously installed callback.
        void subscribe(ProcessEventCallback cb, const std::string &id_filter = {});
    };
} // namespace hestia::client
