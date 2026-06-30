#pragma once

#include <filesystem>
#include <map>
#include <optional>
#include <string>
#include <string_view>

namespace hestia::config {
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
