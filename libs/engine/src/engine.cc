#include <hestia/engine/engine.h>

namespace hestia::engine {
    Engine::Engine(const std::filesystem::path &override_home)
        : data_home_(config::data_home(override_home)),
          config_(config::config_path(data_home_)) {}

    std::filesystem::path Engine::set_data_home(const std::string &dir) {
        config::set_persisted_home(dir);
        data_home_ = config::data_home();
        config_.reload(config::config_path(data_home_));
        // Future subsystems repoint here too.
        return data_home_;
    }
}
