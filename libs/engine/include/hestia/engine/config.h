#pragma once

#include <filesystem>
#include <functional>
#include <mutex>
#include <string>

#include <nlohmann/json.hpp>

#include <hestia/proto/contract.h>

namespace hestia::engine {
    using proto::from_json;
    using proto::to_json;

    // The config schema: a setting is a typed field with its default plus a
    // kFields entry; a nested struct with its own kFields becomes a sub-object.
    // The reserved keys (home, autostart) are routed by the daemon's config
    // service, not stored here.
    struct Settings {
        static constexpr auto kFields = proto::fields();
    };

    // Thread-safe owner of the persisted Settings: every write saves
    // immediately, reload() repoints it when the data directory changes.
    // Internal code reads settings() and writes through update(); the
    // dotted-path get/set serve the wire and reject unknown keys and
    // type-mismatched values.
    class Config {
    public:
        explicit Config(std::filesystem::path path);

        Settings settings() const;
        void update(const std::function<void(Settings &)> &mutate);

        nlohmann::json get(const std::string &key) const;
        void set(const std::string &key, const nlohmann::json &value);
        nlohmann::json all() const;

        void reload(std::filesystem::path path);

    private:
        mutable std::mutex mu_;
        std::filesystem::path path_;
        Settings settings_;
    };
} // namespace hestia::engine
