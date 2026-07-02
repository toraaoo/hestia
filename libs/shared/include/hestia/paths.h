#pragma once

#include <filesystem>

// Per-user data-directory resolution — the single source of truth for "where
// Hestia's data lives", linked by the daemon (via the engine) and every client.
namespace hestia::paths {
    // The fixed anchor directory: ~/.hestia (unix/macOS) or %APPDATA%\Hestia
    // (windows). Never redirected — it holds the persisted-home pointer and is the
    // default data directory when nothing else is configured.
    std::filesystem::path anchor_dir();

    // Resolve the data directory. Precedence: `override_dir` → $HESTIA_HOME →
    // the persisted-home pointer → the platform default (anchor_dir()).
    std::filesystem::path data_home(const std::filesystem::path &override_dir = {});

    // Persist `dir` as the default data directory for future runs (empty removes
    // the pointer, reverting to the platform default).
    void set_persisted_home(const std::filesystem::path &dir);

    // The config file within the resolved data directory.
    std::filesystem::path config_path(const std::filesystem::path &override_dir = {});
} // namespace hestia::paths
