#pragma once

namespace hestia::daemon {
    // Bind the IPC endpoint, build the Runtime, register every service, and serve
    // client connections until the process is signalled. Returns a process exit
    // code (0 on a clean stop, non-zero if the endpoint cannot be bound).
    int run_daemon();
}
