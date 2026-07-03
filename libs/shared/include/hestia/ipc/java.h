#pragma once

#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>

// The java domain types, shared by the daemon (which installs runtimes) and the
// client SDK (which requests them). Wire codec: java_codec.h.
namespace hestia::ipc {
    struct JavaRelease {
        int major = 0;
        bool lts = false;
    };

    struct JavaRuntime {
        std::string vendor;
        int major = 0;
        std::string release_name;
        std::filesystem::path home; // the JAVA_HOME root (Contents/Home on macOS)
        std::filesystem::path executable;
    };

    enum class JavaInstallPhase : std::uint8_t { resolving, downloading, extracting };

    std::optional<JavaInstallPhase> parse_java_install_phase(std::string_view name);

    const char *to_string(JavaInstallPhase phase);

    struct JavaInstallProgress {
        JavaInstallPhase phase = JavaInstallPhase::resolving;
        std::uint64_t current = 0; // bytes while downloading, entries while extracting
        std::uint64_t total = 0;   // 0 = unknown
    };
} // namespace hestia::ipc
