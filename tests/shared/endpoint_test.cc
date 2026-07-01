#include <gtest/gtest.h>

#include <cstdlib>
#include <filesystem>

#include <hestia/ipc/endpoint.h>

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

TEST(Endpoint, DefaultEndpointLivesUnderRuntimeDir) {
    const fs::path endpoint = hestia::ipc::default_endpoint();
    EXPECT_FALSE(endpoint.empty());
#if !defined(_WIN32)
    // On POSIX the socket sits directly in the runtime dir, so both agree on one
    // location (daemon binds, clients connect to the same path).
    EXPECT_EQ(endpoint.parent_path(), hestia::ipc::runtime_dir());
    EXPECT_EQ(endpoint.filename(), "hestiad.sock");
#endif
}

#if !defined(_WIN32)
TEST(Endpoint, PrefersXdgRuntimeDir) {
    const char *saved = std::getenv("XDG_RUNTIME_DIR");
    const fs::path xdg = fs::temp_directory_path() / "hestia_xdg";

    set_env("XDG_RUNTIME_DIR", xdg.string().c_str());
    EXPECT_EQ(hestia::ipc::runtime_dir(), xdg / "hestia");

    set_env("XDG_RUNTIME_DIR", saved);
}

TEST(Endpoint, FallsBackToUidScopedTmpWhenXdgUnset) {
    const char *saved = std::getenv("XDG_RUNTIME_DIR");

    set_env("XDG_RUNTIME_DIR", nullptr);
    const fs::path dir = hestia::ipc::runtime_dir();
    // Falls back to a uid-scoped /tmp dir so two users never collide on one path.
    EXPECT_EQ(dir.parent_path(), "/tmp");
    EXPECT_EQ(dir.filename().string().rfind("hestia-", 0), 0u);

    set_env("XDG_RUNTIME_DIR", saved);
}
#endif
