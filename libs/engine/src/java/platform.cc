#include "java/platform.h"

#include <stdexcept>

#include <hestia/engine/java.h>

namespace hestia::engine {
    namespace fs = std::filesystem;

    JavaTarget host_target() {
#if defined(_WIN32)
        const char *os = "windows";
#elif defined(__APPLE__)
        const char *os = "mac";
#else
        const char *os = "linux";
#endif
#if defined(__aarch64__) || defined(_M_ARM64)
        const char *arch = "aarch64";
#elif defined(__x86_64__) || defined(_M_X64)
        const char *arch = "x64";
#else
        const char *arch = nullptr;
#endif
        if (arch == nullptr) {
            throw std::runtime_error("no Java builds are published for this CPU architecture");
        }
        return JavaTarget{.os = os, .arch = arch};
    }

    namespace {
#if defined(_WIN32)
        constexpr const char *kJavaExe = "java.exe";
#else
        constexpr const char *kJavaExe = "java";
#endif

        std::optional<fs::path> java_under(const fs::path &home) {
            std::error_code ec;
            if (fs::path exe = home / "bin" / kJavaExe; fs::is_regular_file(exe, ec)) return exe;
            if (fs::path exe = home / "Contents" / "Home" / "bin" / kJavaExe; fs::is_regular_file(exe, ec)) {
                return exe;
            }
            return std::nullopt;
        }
    } // namespace

    std::optional<fs::path> find_java_executable(const fs::path &root) {
        if (auto exe = java_under(root)) return exe;
        std::error_code ec;
        for (const auto &entry: fs::directory_iterator(root, ec)) {
            if (!entry.is_directory(ec)) continue;
            if (auto exe = java_under(entry.path())) return exe;
        }
        return std::nullopt;
    }
} // namespace hestia::engine
