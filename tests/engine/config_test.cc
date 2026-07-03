#include <gtest/gtest.h>

#include <filesystem>
#include <fstream>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

#include <hestia/engine/config.h>

namespace fs = std::filesystem;
using hestia::engine::Config;

TEST(Config, PersistsEachSet) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store";
    const fs::path path = dir / "config";
    fs::remove_all(dir);

    {
        Config store(path);
        EXPECT_FALSE(store.get("theme").has_value());
        store.set("theme", "dark");
        EXPECT_EQ(store.get("theme").value_or(""), "dark");
    }
    // A fresh store over the same file sees the persisted value.
    Config reopened(path);
    EXPECT_EQ(reopened.get("theme").value_or(""), "dark");

    fs::remove_all(dir);
}

TEST(Config, ReloadRepointsAtNewFile) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_reload";
    const fs::path first = dir / "a" / "config";
    const fs::path second = dir / "b" / "config";
    fs::remove_all(dir);

    Config store(first);
    store.set("k", "first");

    store.reload(second);
    EXPECT_FALSE(store.get("k").has_value()); // the new file is empty
    store.set("k", "second");

    Config reopened(second);
    EXPECT_EQ(reopened.get("k").value_or(""), "second");

    fs::remove_all(dir);
}

TEST(Config, ConcurrentSetsAreSerializedAndPersisted) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_concurrent";
    const fs::path path = dir / "config";
    fs::remove_all(dir);

    constexpr int kThreads = 8;
    constexpr int kPerThread = 50;
    {
        Config store(path);
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
        for (auto &w: workers) w.join();

        for (int t = 0; t < kThreads; ++t) {
            for (int i = 0; i < kPerThread; ++i) {
                const std::string key = "k" + std::to_string(t) + "_" + std::to_string(i);
                EXPECT_EQ(store.get(key).value_or(""), std::to_string(t * 1000 + i));
            }
        }
    }
    // Every concurrent set() persisted, so a fresh store sees them all — no
    // interleaved save() corrupted the file.
    Config reopened(path);
    EXPECT_EQ(reopened.get("k3_7").value_or(""), std::to_string(3 * 1000 + 7));
    EXPECT_EQ(reopened.get("k0_0").value_or(""), "0");
    EXPECT_EQ(reopened.get("k7_49").value_or(""), std::to_string(7 * 1000 + 49));

    fs::remove_all(dir);
}

TEST(Config, RejectsEntriesThatWouldCorruptTheFile) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_validate";
    fs::remove_all(dir);

    Config store(dir / "config");
    EXPECT_THROW(store.set("", "v"), std::invalid_argument);
    EXPECT_THROW(store.set("a=b", "v"), std::invalid_argument);
    EXPECT_THROW(store.set("a\nb", "v"), std::invalid_argument);
    EXPECT_THROW(store.set("k", "line1\nline2"), std::invalid_argument);
    EXPECT_THROW(store.set("k", "has\rcr"), std::invalid_argument);
    // A value may contain '=' — only the key splits on it.
    EXPECT_NO_THROW(store.set("url", "http://x?a=b"));
    EXPECT_EQ(store.get("url").value_or(""), "http://x?a=b");

    fs::remove_all(dir);
}

TEST(Config, LoadSkipsCommentsBlanksAndMalformedLines) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_store_parse";
    const fs::path path = dir / "config";
    fs::remove_all(dir);
    fs::create_directories(dir);
    {
        std::ofstream out(path);
        out << "# a comment\n"
            << "\n"
            << "no_equals_sign_here\n"
            << "good=value\n";
    }
    const Config store(path);
    EXPECT_EQ(store.get("good").value_or(""), "value");
    EXPECT_FALSE(store.get("# a comment").has_value());
    EXPECT_FALSE(store.get("no_equals_sign_here").has_value());

    fs::remove_all(dir);
}
