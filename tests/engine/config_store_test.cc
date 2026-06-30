#include <gtest/gtest.h>

#include <filesystem>

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
