#include <hestia/engine/engine.h>

#include <spdlog/spdlog.h>

#include <hestia/paths.h>

namespace hestia::engine {
    Engine::Engine(const std::filesystem::path &override_home)
        : data_home_(paths::data_home(override_home)), config_(paths::config_path(data_home_)),
          cache_(data_home_ / "cache"), java_(data_home_ / "java", &cache_),
          accounts_(data_home_ / "accounts.json") {
        spdlog::info("engine data home: {}", data_home_.string());
    }

    std::filesystem::path Engine::set_data_home(const std::string &dir) {
        paths::set_persisted_home(dir);
        data_home_ = paths::data_home();
        config_.reload(paths::config_path(data_home_));
        cache_.reload(data_home_ / "cache");
        java_.reload(data_home_ / "java");
        accounts_.reload(data_home_ / "accounts.json");
        spdlog::info("engine data home changed to {}", data_home_.string());
        return data_home_;
    }
} // namespace hestia::engine
