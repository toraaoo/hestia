#include "hestia/ipc/java_codec.h"

namespace hestia::ipc {
    using nlohmann::json;

    std::optional<JavaInstallPhase> parse_java_install_phase(std::string_view name) {
        if (name == "resolving") return JavaInstallPhase::resolving;
        if (name == "downloading") return JavaInstallPhase::downloading;
        if (name == "extracting") return JavaInstallPhase::extracting;
        return std::nullopt;
    }

    const char *to_string(JavaInstallPhase phase) {
        switch (phase) {
            case JavaInstallPhase::resolving: return "resolving";
            case JavaInstallPhase::downloading: return "downloading";
            case JavaInstallPhase::extracting: return "extracting";
        }
        return "resolving";
    }

    json to_json(const JavaRelease &release) {
        return json{
            {"major", release.major},
            {"lts", release.lts},
        };
    }

    JavaRelease java_release_from_json(const json &j) {
        return JavaRelease{
            .major = j.value("major", 0),
            .lts = j.value("lts", false),
        };
    }

    json to_json(const JavaRuntime &runtime) {
        return json{
            {"vendor", runtime.vendor},
            {"major", runtime.major},
            {"release_name", runtime.release_name},
            {"home", runtime.home.string()},
            {"executable", runtime.executable.string()},
        };
    }

    JavaRuntime java_runtime_from_json(const json &j) {
        JavaRuntime runtime;
        runtime.vendor = j.value("vendor", std::string{});
        runtime.major = j.value("major", 0);
        runtime.release_name = j.value("release_name", std::string{});
        runtime.home = j.value("home", std::string{});
        runtime.executable = j.value("executable", std::string{});
        return runtime;
    }

    json to_json(const JavaInstallProgress &progress) {
        return json{
            {"phase", to_string(progress.phase)},
            {"current", progress.current},
            {"total", progress.total},
        };
    }

    JavaInstallProgress java_install_progress_from_json(const json &j) {
        JavaInstallProgress progress;
        progress.phase = parse_java_install_phase(j.value("phase", std::string{})).value_or(JavaInstallPhase::resolving);
        progress.current = j.value("current", std::uint64_t{0});
        progress.total = j.value("total", std::uint64_t{0});
        return progress;
    }
} // namespace hestia::ipc
