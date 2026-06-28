#pragma once

#include <filesystem>
#include <map>
#include <optional>
#include <string>
#include <string_view>

namespace hestia::config {
    // The fixed anchor directory: ~/.hestia (unix/macOS) or %APPDATA%\Hestia
    // (windows). Never redirected — it is where the persisted-home pointer lives,
    // and the default data directory when nothing else is configured.
    std::filesystem::path anchor_dir();

    // Resolve Hestia's per-user data directory. Precedence:
    //   1. `override_dir`, when non-empty (e.g. a --home CLI flag)
    //   2. the HESTIA_HOME environment variable
    //   3. the persisted-home pointer written by set_persisted_home()
    //   4. the platform default (anchor_dir())
    std::filesystem::path data_home(const std::filesystem::path &override_dir = {});

    // Persist `dir` as the default data directory so future runs use it without
    // --home or HESTIA_HOME. Writes a pointer file under anchor_dir(). An empty
    // `dir` removes the pointer, reverting to the platform default.
    void set_persisted_home(const std::filesystem::path &dir);

    // Path to the config file within the resolved data directory.
    std::filesystem::path config_path(const std::filesystem::path &override_dir = {});

    // A flat string key/value store persisted as `key=value` lines.
    class Config {
    public:
        // Load from `path`. A missing file yields an empty config (not an error).
        static Config load(const std::filesystem::path &path);

        std::optional<std::string> get(std::string_view key) const;
        void set(std::string_view key, std::string_view value);

        const std::map<std::string, std::string> &entries() const { return entries_; }

        // Persist to `path`, creating parent directories as needed.
        void save(const std::filesystem::path &path) const;

    private:
        std::map<std::string, std::string> entries_;
    };
}
