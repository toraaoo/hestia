#include <gtest/gtest.h>

#include <filesystem>
#include <string>
#include <thread>
#include <vector>

#include <hestia/engine/config_store.h>

namespace fs = std::filesystem;
using hestia::engine::ConfigStore;

TEST(ConfigStore, PersistsEachSet) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store";
    const fs::path path = dir / "config";
    fs::remove_all(dir);

    {
        ConfigStore store(path);
        EXPECT_FALSE(store.get("theme").has_value());
        store.set("theme", "dark");
        EXPECT_EQ(store.get("theme").value_or(""), "dark");
    }
    // A fresh store over the same file sees the persisted value.
    ConfigStore reopened(path);
    EXPECT_EQ(reopened.get("theme").value_or(""), "dark");

    fs::remove_all(dir);
}

TEST(ConfigStore, ReloadRepointsAtNewFile) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_reload";
    const fs::path first = dir / "a" / "config";
    const fs::path second = dir / "b" / "config";
    fs::remove_all(dir);

    ConfigStore store(first);
    store.set("k", "first");

    store.reload(second);
    EXPECT_FALSE(store.get("k").has_value()); // the new file is empty
    store.set("k", "second");

    ConfigStore reopened(second);
    EXPECT_EQ(reopened.get("k").value_or(""), "second");

    fs::remove_all(dir);
}

TEST(ConfigStore, ConcurrentSetsAreSerializedAndPersisted) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_concurrent";
    const fs::path path = dir / "config";
    fs::remove_all(dir);

    constexpr int kThreads = 8;
    constexpr int kPerThread = 50;
    {
        ConfigStore store(path);
        std::vector<std::thread> workers;
        for (int t = 0; t < kThreads; ++t) {
            workers.emplace_back([&store, t] {
                for (int i = 0; i < kPerThread; ++i) {
                    const std::string key = "k" + std::to_string(t) + "_" + std::to_string(i);
                    store.set(key, std::to_string(t * 1000 + i));
                    // Interleave reads so the shared mutex is exercised both ways.
                    (void)store.get(key);
                }
            });
        }
        for (auto &w : workers) w.join();

        for (int t = 0; t < kThreads; ++t) {
            for (int i = 0; i < kPerThread; ++i) {
                const std::string key = "k" + std::to_string(t) + "_" + std::to_string(i);
                EXPECT_EQ(store.get(key).value_or(""), std::to_string(t * 1000 + i));
            }
        }
    }
    // Every concurrent set() persisted, so a fresh store sees them all — no
    // interleaved save() corrupted the file.
    ConfigStore reopened(path);
    EXPECT_EQ(reopened.get("k3_7").value_or(""), std::to_string(3 * 1000 + 7));
    EXPECT_EQ(reopened.get("k0_0").value_or(""), "0");
    EXPECT_EQ(reopened.get("k7_49").value_or(""), std::to_string(7 * 1000 + 49));

    fs::remove_all(dir);
}
