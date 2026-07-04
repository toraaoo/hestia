#pragma once

#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

#include <hestia/proto/contract.h>

// The java domain: types shared by the daemon (which installs runtimes) and the
// client SDK (which requests them), plus the contracts for the java.* channels
// and events.
namespace hestia::proto {
    struct JavaRelease {
        int major = 0;
        bool lts = false;

        static constexpr auto kFields = fields(field("major", &JavaRelease::major), field("lts", &JavaRelease::lts));
    };

    struct JavaRuntime {
        std::string vendor;
        int major = 0;
        std::string release_name;
        std::filesystem::path home; // the JAVA_HOME root (Contents/Home on macOS)
        std::filesystem::path executable;

        static constexpr auto kFields =
            fields(field("vendor", &JavaRuntime::vendor), field("major", &JavaRuntime::major),
                   field("release_name", &JavaRuntime::release_name), field("home", &JavaRuntime::home),
                   field("executable", &JavaRuntime::executable));
    };

    enum class JavaInstallPhase : std::uint8_t { resolving, downloading, extracting };

    std::optional<JavaInstallPhase> parse_java_install_phase(std::string_view name);

    const char *to_string(JavaInstallPhase phase);

    void to_json(nlohmann::json &j, JavaInstallPhase phase);
    void from_json(const nlohmann::json &j, JavaInstallPhase &phase);

    struct JavaInstallProgress {
        JavaInstallPhase phase = JavaInstallPhase::resolving;
        std::uint64_t current = 0; // bytes while downloading, entries while extracting
        std::uint64_t total = 0;   // 0 = unknown

        static constexpr auto kFields =
            fields(field("phase", &JavaInstallProgress::phase), field("current", &JavaInstallProgress::current),
                   field("total", &JavaInstallProgress::total));
    };

    struct JavaReleases {
        static constexpr const char *kChannel = "java.releases";
        using Params = Empty;
        struct Result {
            std::vector<JavaRelease> releases;

            static constexpr auto kFields = fields(field("releases", &Result::releases));
        };
    };

    struct JavaList {
        static constexpr const char *kChannel = "java.list";
        using Params = Empty;
        struct Result {
            std::vector<JavaRuntime> runtimes;

            static constexpr auto kFields = fields(field("runtimes", &Result::runtimes));
        };
    };

    struct JavaInstall {
        static constexpr const char *kChannel = "java.install";
        struct Params {
            int major = 0;
            std::string id; // caller-assigned job id; empty lets the daemon generate one
            bool force = false;

            static constexpr auto kFields = fields(field("major", &Params::major), field("id", &Params::id),
                                                   field("force", &Params::force));
        };
        struct Result {
            std::string id;

            static constexpr auto kFields = fields(field("id", &Result::id));
        };
    };

    struct JavaUninstall {
        static constexpr const char *kChannel = "java.uninstall";
        struct Params {
            int major = 0;

            static constexpr auto kFields = fields(field("major", &Params::major));
        };
        using Result = Empty;
    };

    struct JavaInstallProgressEvent {
        static constexpr const char *kTopic = "java.install.progress";
        std::string id;
        JavaInstallProgress progress;

        static constexpr auto kFields = fields(field("id", &JavaInstallProgressEvent::id),
                                               field("", &JavaInstallProgressEvent::progress, kFlatten));
    };

    struct JavaInstallDoneEvent {
        static constexpr const char *kTopic = "java.install.done";
        std::string id;
        JavaRuntime runtime;
        bool already_installed = false;

        static constexpr auto kFields =
            fields(field("id", &JavaInstallDoneEvent::id), field("runtime", &JavaInstallDoneEvent::runtime),
                   field("already_installed", &JavaInstallDoneEvent::already_installed));
    };

    struct JavaInstallErrorEvent {
        static constexpr const char *kTopic = "java.install.error";
        std::string id;
        std::string message;

        static constexpr auto kFields =
            fields(field("id", &JavaInstallErrorEvent::id), field("message", &JavaInstallErrorEvent::message));
    };
} // namespace hestia::proto
