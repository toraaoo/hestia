#include <hestia/engine/config_store.h>

namespace hestia::engine {
    ConfigStore::ConfigStore(std::filesystem::path path)
        : path_(std::move(path)), cfg_(config::Config::load(path_)) {}

    std::optional<std::string> ConfigStore::get(const std::string &key) const {
        std::lock_guard<std::mutex> lk(mu_);
        return cfg_.get(key);
    }

    void ConfigStore::set(const std::string &key, const std::string &value) {
        std::lock_guard<std::mutex> lk(mu_);
        cfg_.set(key, value);
        cfg_.save(path_);
    }

    void ConfigStore::reload(std::filesystem::path path) {
        std::lock_guard<std::mutex> lk(mu_);
        path_ = std::move(path);
        cfg_ = config::Config::load(path_);
    }
}
