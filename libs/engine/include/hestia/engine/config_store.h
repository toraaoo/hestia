#pragma once

#include <filesystem>
#include <mutex>
#include <optional>
#include <string>

#include <hestia/config.h>

namespace hestia::engine {
    // A thread-safe live view of the flat key/value config file, owned by the
    // Engine. Reads and writes are serialized so concurrent client requests are
    // safe, and every set() is persisted immediately. reload() repoints it at a
    // new file when the data directory changes under it.
    class ConfigStore {
    public:
        explicit ConfigStore(std::filesystem::path path);

        std::optional<std::string> get(const std::string &key) const;
        void set(const std::string &key, const std::string &value);

        void reload(std::filesystem::path path);

    private:
        mutable std::mutex mu_;
        std::filesystem::path path_;
        config::Config cfg_;
    };
}
