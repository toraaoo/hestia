#include <gtest/gtest.h>

#include <filesystem>
#include <stdexcept>

#include <hestia/engine/engine.h>

namespace fs = std::filesystem;
using hestia::engine::Engine;

TEST(Engine, ResolvesOverrideHome) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_engine";
    fs::remove_all(dir);
    const Engine engine{dir};
    EXPECT_EQ(engine.data_home(), dir);
    fs::remove_all(dir);
}

TEST(Engine, ExposesConfigSubsystem) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_engine_cfg";
    fs::remove_all(dir);
    Engine engine{dir};
    EXPECT_TRUE(engine.config().all().is_object());
    EXPECT_THROW(engine.config().set("unknown", "v"), std::invalid_argument);
    fs::remove_all(dir);
}
