#include <gtest/gtest.h>

#include <filesystem>
#include <fstream>
#include <stdexcept>

#include <hestia/engine/config/config.h>

namespace fs = std::filesystem;
using hestia::config::Config;

TEST(Config, MissingFileIsEmpty) {
    const fs::path path = fs::temp_directory_path() / "hestia_test_absent" / "config";
    fs::remove_all(path.parent_path());
    const Config cfg = Config::load(path);
    EXPECT_FALSE(cfg.get("anything").has_value());
}

TEST(Config, SetSaveLoadRoundTrip) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_cfg";
    const fs::path path = dir / "config";
    fs::remove_all(dir);

    Config cfg = Config::load(path);
    cfg.set("theme", "dark");
    cfg.set("name", "Ada");
    cfg.save(path);

    const Config reloaded = Config::load(path);
    EXPECT_EQ(reloaded.get("theme").value_or(""), "dark");
    EXPECT_EQ(reloaded.get("name").value_or(""), "Ada");
    EXPECT_FALSE(reloaded.get("missing").has_value());

    fs::remove_all(dir);
}

TEST(Config, SetOverwrites) {
    Config cfg;
    cfg.set("k", "one");
    cfg.set("k", "two");
    EXPECT_EQ(cfg.get("k").value_or(""), "two");
}

TEST(Config, SetRejectsCorruptingCharacters) {
    Config cfg;
    EXPECT_THROW(cfg.set("", "v"), std::invalid_argument);
    EXPECT_THROW(cfg.set("a=b", "v"), std::invalid_argument);
    EXPECT_THROW(cfg.set("a\nb", "v"), std::invalid_argument);
    EXPECT_THROW(cfg.set("k", "line1\nline2"), std::invalid_argument);
    EXPECT_THROW(cfg.set("k", "has\rcr"), std::invalid_argument);
    // A value may contain '=' — only the key splits on it.
    EXPECT_NO_THROW(cfg.set("url", "http://x?a=b"));
    EXPECT_EQ(cfg.get("url").value_or(""), "http://x?a=b");
}

TEST(Config, LoadSkipsCommentsBlanksAndMalformedLines) {
    const fs::path dir = fs::temp_directory_path() / "hestia_test_cfg_parse";
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
    const Config cfg = Config::load(path);
    EXPECT_EQ(cfg.get("good").value_or(""), "value");
    EXPECT_FALSE(cfg.get("# a comment").has_value());
    EXPECT_FALSE(cfg.get("no_equals_sign_here").has_value());
    EXPECT_EQ(cfg.entries().size(), 1u);

    fs::remove_all(dir);
}
