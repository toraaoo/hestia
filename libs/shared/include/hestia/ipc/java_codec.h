#pragma once

#include <nlohmann/json.hpp>

#include <hestia/ipc/java.h>

// The single home for java domain types ⇄ JSON. Daemon and client SDK both
// (de)serialize through here, so the wire format is defined once and cannot drift.
namespace hestia::ipc {
    nlohmann::json to_json(const JavaRelease &release);
    JavaRelease java_release_from_json(const nlohmann::json &j);

    nlohmann::json to_json(const JavaRuntime &runtime);
    JavaRuntime java_runtime_from_json(const nlohmann::json &j);

    nlohmann::json to_json(const JavaInstallProgress &progress);
    JavaInstallProgress java_install_progress_from_json(const nlohmann::json &j);
} // namespace hestia::ipc
