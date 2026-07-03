#pragma once

#include <filesystem>
#include <map>
#include <mutex>
#include <optional>
#include <string>

namespace hestia::engine {
    // Thread-safe live view of the key=value config file: every set() persists
    // immediately, reload() repoints it when the data directory changes.
    class Config {
    public:
        explicit Config(std::filesystem::path path);

        std::optional<std::string> get(const std::string &key) const;
        void set(const std::string &key, const std::string &value);

        void reload(std::filesystem::path path);

    private:
        mutable std::mutex mu_;
        std::filesystem::path path_;
        std::map<std::string, std::string> entries_;
    };
} // namespace hestia::engine
