#include <hestia/engine/config.h>

#include <fstream>
#include <stdexcept>
#include <string_view>

namespace hestia::engine {
    namespace {
        std::map<std::string, std::string> load_entries(const std::filesystem::path &path) {
            std::map<std::string, std::string> entries;
            std::ifstream in(path);
            if (!in) {
                return entries;
            }
            std::string line;
            while (std::getline(in, line)) {
                if (line.empty() || line.front() == '#') {
                    continue;
                }
                const auto eq = line.find('=');
                if (eq == std::string::npos) {
                    continue;
                }
                entries.insert_or_assign(line.substr(0, eq), line.substr(eq + 1));
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
            for (const auto &[key, value]: entries) {
                out << key << '=' << value << '\n';
            }
        }

        void validate_entry(std::string_view key, std::string_view value) {
            if (key.empty()) {
                throw std::invalid_argument("config key must not be empty");
            }
            if (key.find_first_of("=\n\r") != std::string_view::npos) {
                throw std::invalid_argument("config key must not contain '=', newline, or CR");
            }
            if (value.find_first_of("\n\r") != std::string_view::npos) {
                throw std::invalid_argument("config value must not contain newline or CR");
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

    void Config::set(const std::string &key, const std::string &value) {
        std::scoped_lock const lk(mu_);
        validate_entry(key, value);
        entries_.insert_or_assign(key, value);
        save_entries(path_, entries_);
    }

    void Config::reload(std::filesystem::path path) {
        std::scoped_lock const lk(mu_);
        path_ = std::move(path);
        entries_ = load_entries(path_);
    }
} // namespace hestia::engine
