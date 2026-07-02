#include <hestia/engine/engine.h>

#include <hestia/paths.h>

namespace hestia::engine {
    Engine::Engine(const std::filesystem::path &override_home)
        : data_home_(paths::data_home(override_home)), config_(paths::config_path(data_home_)) {}

    std::filesystem::path Engine::set_data_home(const std::string &dir) {
        paths::set_persisted_home(dir);
        data_home_ = paths::data_home();
        config_.reload(paths::config_path(data_home_));
        // Future subsystems repoint here too.
        return data_home_;
    }
} // namespace hestia::engine
