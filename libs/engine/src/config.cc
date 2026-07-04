#include <hestia/engine/config.h>

#include <fstream>
#include <stdexcept>
#include <string_view>

#include <nlohmann/json.hpp>

namespace hestia::engine {
    namespace {
        std::map<std::string, std::string> load_entries(const std::filesystem::path &path) {
            std::ifstream in(path);
            if (!in) {
                return {};
            }
            const auto doc = nlohmann::json::parse(in, nullptr, false);
            if (!doc.is_object()) {
                return {};
            }
            std::map<std::string, std::string> entries;
            for (const auto &[key, value]: doc.items()) {
                if (value.is_string()) {
                    entries.insert_or_assign(key, value.get<std::string>());
                }
            }
            return entries;
        }

        void save_entries(const std::filesystem::path &path, const std::map<std::string, std::string> &entries) {
            if (path.has_parent_path()) {
                std::filesystem::create_directories(path.parent_path());
            }
            std::ofstream out(path, std::ios::trunc);
            if (!out) {
                throw std::runtime_error("failed to open config file for writing: " + path.string());
            }
            out << nlohmann::json(entries).dump(2) << '\n';
        }

        void validate_key(std::string_view key) {
            if (key.empty()) {
                throw std::invalid_argument("config key must not be empty");
            }
        }
    } // namespace

    Config::Config(std::filesystem::path path) : path_(std::move(path)), entries_(load_entries(path_)) {}

    std::optional<std::string> Config::get(const std::string &key) const {
        std::scoped_lock const lk(mu_);
        if (const auto it = entries_.find(key); it != entries_.end()) {
            return it->second;
        }
        return std::nullopt;
    }

    std::map<std::string, std::string> Config::all() const {
        std::scoped_lock const lk(mu_);
        return entries_;
    }

    void Config::set(const std::string &key, const std::string &value) {
        std::scoped_lock const lk(mu_);
        validate_key(key);
        entries_.insert_or_assign(key, value);
        save_entries(path_, entries_);
    }

    void Config::reload(std::filesystem::path path) {
        std::scoped_lock const lk(mu_);
        path_ = std::move(path);
        entries_ = load_entries(path_);
    }
} // namespace hestia::engine
