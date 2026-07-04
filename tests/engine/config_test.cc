#include <gtest/gtest.h>

#include <filesystem>
#include <fstream>
#include <stdexcept>
#include <thread>
#include <vector>

#include <nlohmann/json.hpp>

#include <hestia/engine/config.h>

namespace fs = std::filesystem;
using hestia::engine::Config;
using hestia::engine::Settings;

TEST(Config, StartsFromDefaultsAndPersistsUpdates) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store";
    const fs::path path = dir / "config";
    fs::remove_all(dir);

    {
        Config store(path);
        EXPECT_TRUE(store.all().is_object());
        store.update([](Settings &) {});
    }
    ASSERT_TRUE(fs::exists(path));
    std::ifstream in(path);
    const auto doc = nlohmann::json::parse(in, nullptr, false);
    EXPECT_TRUE(doc.is_object());

    const Config reopened(path);
    EXPECT_EQ(reopened.all(), doc);

    fs::remove_all(dir);
}

TEST(Config, RejectsUnknownKeys) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_unknown";
    fs::remove_all(dir);

    Config store(dir / "config");
    EXPECT_THROW((void)store.get("nope"), std::invalid_argument);
    EXPECT_THROW((void)store.get(""), std::invalid_argument);
    EXPECT_THROW((void)store.get("a.b"), std::invalid_argument);
    EXPECT_THROW(store.set("nope", "v"), std::invalid_argument);
    EXPECT_THROW(store.set("", "v"), std::invalid_argument);
    EXPECT_THROW(store.set("a.b", 1), std::invalid_argument);

    fs::remove_all(dir);
}

TEST(Config, MalformedFileLoadsDefaults) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_malformed";
    const fs::path path = dir / "config";
    fs::remove_all(dir);
    fs::create_directories(dir);
    {
        std::ofstream out(path);
        out << "not json at all {{\n";
    }
    const Config store(path);
    EXPECT_EQ(store.all(), nlohmann::json::object());

    fs::remove_all(dir);
}

TEST(Config, ReloadRepointsAtNewFile) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_reload";
    const fs::path first = dir / "a" / "config";
    const fs::path second = dir / "b" / "config";
    fs::remove_all(dir);

    Config store(first);
    store.update([](Settings &) {});
    EXPECT_TRUE(fs::exists(first));

    store.reload(second);
    EXPECT_FALSE(fs::exists(second));
    store.update([](Settings &) {});
    EXPECT_TRUE(fs::exists(second));

    fs::remove_all(dir);
}

TEST(Config, ConcurrentAccessIsSerialized) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_concurrent";
    const fs::path path = dir / "config";
    fs::remove_all(dir);

    constexpr int kThreads = 8;
    constexpr int kPerThread = 50;
    {
        Config store(path);
        std::vector<std::thread> workers;
        workers.reserve(kThreads);
        for (int t = 0; t < kThreads; ++t) {
            workers.emplace_back([&store] {
                for (int i = 0; i < kPerThread; ++i) {
                    store.update([](Settings &) {});
                    (void)store.settings();
                    (void)store.all();
                }
            });
        }
        for (auto &w: workers) w.join();
    }
    // Every interleaved save left the file a valid settings document.
    std::ifstream in(path);
    const auto doc = nlohmann::json::parse(in, nullptr, false);
    EXPECT_TRUE(doc.is_object());

    fs::remove_all(dir);
}
