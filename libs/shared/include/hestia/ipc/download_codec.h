#pragma once

#include <nlohmann/json.hpp>

#include <hestia/ipc/download.h>

// The single home for download domain types ⇄ JSON. Daemon and client SDK both
// (de)serialize through here, so the wire format is defined once and cannot drift.
namespace hestia::ipc {
    nlohmann::json to_json(const DownloadSpec &spec);
    DownloadSpec download_spec_from_json(const nlohmann::json &payload);

    nlohmann::json to_json(const DownloadProgress &progress);
    DownloadProgress progress_from_json(const nlohmann::json &j);
} // namespace hestia::ipc
