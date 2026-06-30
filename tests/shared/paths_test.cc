#include <gtest/gtest.h>

#include <cstdlib>
#include <filesystem>

#include <hestia/paths.h>

namespace fs = std::filesystem;

namespace {
    void set_env(const char *key, const char *value) {
#if defined(_WIN32)
        _putenv_s(key, value ? value : "");
#else
        if (value)
            ::setenv(key, value, 1);
        else
            ::unsetenv(key);
#endif
    }
}

TEST(Paths, DataHomePrecedence) {
    const char *saved_home = std::getenv("HESTIA_HOME");
    const fs::path override_dir = fs::temp_directory_path() / "hestia_override";
    const fs::path env_dir = fs::temp_directory_path() / "hestia_env";

    set_env("HESTIA_HOME", env_dir.string().c_str());
    EXPECT_EQ(hestia::paths::data_home(override_dir), override_dir);  // override wins
    EXPECT_EQ(hestia::paths::data_home(), env_dir);                   // then $HESTIA_HOME
    EXPECT_EQ(hestia::paths::config_path(override_dir), override_dir / "config");

    set_env("HESTIA_HOME", saved_home);
}
