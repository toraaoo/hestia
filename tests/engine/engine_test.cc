#include <gtest/gtest.h>

#include <filesystem>

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
    engine.config().set("theme", "dark");
    EXPECT_EQ(engine.config().get("theme").value_or(""), "dark");
    fs::remove_all(dir);
}
