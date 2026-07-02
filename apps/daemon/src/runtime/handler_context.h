#pragma once

#include <memory>

#include <hestia/ipc/transport.h>

// The collaborators a request handler may need, bundled into one object passed to
// every handler. `runtime` carries the daemon's long-lived collaborators (engine,
// event hub, download manager, process supervisor); `connection`/`peer` vary per
// request, which lets streaming channels (e.g. events.subscribe) be ordinary
// handlers instead of serve-loop special cases.
namespace hestia::daemon {
    class Runtime;

    struct HandlerContext {
        Runtime &runtime;
        std::shared_ptr<ipc::Connection> connection;
        ipc::Peer peer;
    };
}
