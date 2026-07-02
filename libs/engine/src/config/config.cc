#include <hestia/engine/config/config.h>

#include <fstream>
#include <stdexcept>

namespace hestia::config {
    Config Config::load(const std::filesystem::path &path) {
        Config config;
        std::ifstream in(path);
        if (!in) {
            return config;
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
            config.entries_.insert_or_assign(line.substr(0, eq), line.substr(eq + 1));
        }
        return config;
    }

    std::optional<std::string> Config::get(std::string_view key) const {
        if (const auto it = entries_.find(std::string(key)); it != entries_.end()) {
            return it->second;
        }
        return std::nullopt;
    }

    void Config::set(std::string_view key, std::string_view value) {
        // The on-disk format is one `key=value` line each, so a key/value
        // carrying a newline (or a key carrying '=') would corrupt the file and
        // mis-parse on load. Reject rather than silently mangle.
        if (key.empty()) {
            throw std::invalid_argument("config key must not be empty");
        }
        if (key.find_first_of("=\n\r") != std::string_view::npos) {
            throw std::invalid_argument("config key must not contain '=', newline, or CR");
        }
        if (value.find_first_of("\n\r") != std::string_view::npos) {
            throw std::invalid_argument("config value must not contain newline or CR");
        }
        entries_.insert_or_assign(std::string(key), std::string(value));
    }

    void Config::save(const std::filesystem::path &path) const {
        if (path.has_parent_path()) {
            std::filesystem::create_directories(path.parent_path());
        }
        std::ofstream out(path, std::ios::trunc);
        if (!out) {
            throw std::runtime_error("failed to open config file for writing: " + path.string());
        }
        for (const auto &[key, value]: entries_) {
            out << key << '=' << value << '\n';
        }
    }
} // namespace hestia::config
