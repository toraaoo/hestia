#pragma once

#include <filesystem>
#include <memory>

#include <hestia/ipc/transport.h>

// Daemon discovery and auto-spawn for Client::connect: find hestiad next to the
// current binary (or via PATH), start it detached, and poll for its endpoint.
namespace hestia::client {
    void spawn_daemon();

    std::shared_ptr<ipc::Connection> connect_with_retry(const std::filesystem::path &endpoint);
} // namespace hestia::client
