#include <hestia/engine/config.h>

#include <fstream>
#include <stdexcept>
#include <string>

namespace hestia::engine {
    namespace {
        Settings load_settings(const std::filesystem::path &path) {
            std::ifstream in(path);
            if (!in) {
                return {};
            }
            const auto doc = nlohmann::json::parse(in, nullptr, false);
            if (!doc.is_object()) {
                return {};
            }
            try {
                return doc.get<Settings>();
            } catch (const std::exception &) {
                return {};
            }
        }

        void save_settings(const std::filesystem::path &path, const Settings &settings) {
            if (path.has_parent_path()) {
                std::filesystem::create_directories(path.parent_path());
            }
            std::ofstream out(path, std::ios::trunc);
            if (!out) {
                throw std::runtime_error("failed to open config file for writing: " + path.string());
            }
            out << nlohmann::json(settings).dump(2) << '\n';
        }

        const nlohmann::json *find_node(const nlohmann::json &root, const std::string &key) {
            const nlohmann::json *node = &root;
            std::size_t begin = 0;
            while (true) {
                const auto end = key.find('.', begin);
                const auto segment = key.substr(begin, end - begin);
                if (segment.empty() || !node->is_object()) {
                    return nullptr;
                }
                const auto it = node->find(segment);
                if (it == node->end()) {
                    return nullptr;
                }
                node = &*it;
                if (end == std::string::npos) {
                    return node;
                }
                begin = end + 1;
            }
        }

        bool same_json_kind(const nlohmann::json &a, const nlohmann::json &b) {
            if (a.is_number() && b.is_number()) {
                return true;
            }
            return a.type() == b.type();
        }
    } // namespace

    Config::Config(std::filesystem::path path) : path_(std::move(path)), settings_(load_settings(path_)) {}

    Settings Config::settings() const {
        std::scoped_lock const lk(mu_);
        return settings_;
    }

    void Config::update(const std::function<void(Settings &)> &mutate) {
        std::scoped_lock const lk(mu_);
        mutate(settings_);
        save_settings(path_, settings_);
    }

    nlohmann::json Config::get(const std::string &key) const {
        std::scoped_lock const lk(mu_);
        const nlohmann::json doc(settings_);
        const auto *node = find_node(doc, key);
        if (node == nullptr) {
            throw std::invalid_argument("unknown config key: " + key);
        }
        return *node;
    }

    void Config::set(const std::string &key, const nlohmann::json &value) {
        std::scoped_lock const lk(mu_);
        nlohmann::json doc(settings_);
        const auto *node = find_node(doc, key);
        if (node == nullptr) {
            throw std::invalid_argument("unknown config key: " + key);
        }
        if (!same_json_kind(*node, value)) {
            throw std::invalid_argument(key + " expects a " + node->type_name());
        }
        *const_cast<nlohmann::json *>(node) = value;
        try {
            settings_ = doc.get<Settings>();
        } catch (const std::exception &e) {
            throw std::invalid_argument("invalid value for " + key + ": " + e.what());
        }
        save_settings(path_, settings_);
    }

    nlohmann::json Config::all() const {
        std::scoped_lock const lk(mu_);
        nlohmann::json doc(settings_);
        return doc;
    }

    void Config::reload(std::filesystem::path path) {
        std::scoped_lock const lk(mu_);
        path_ = std::move(path);
        settings_ = load_settings(path_);
    }
} // namespace hestia::engine
